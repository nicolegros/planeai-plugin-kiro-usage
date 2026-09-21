use serde::Serialize;
use serde_json::{json, Value};
use std::{
    io::{self, BufRead, Read, Write},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const PLUGIN_ID: &str = "kiro-usage";
const PLUGIN_NAME: &str = "Kiro Usage";
const PLUGIN_VERSION: &str = env!("CARGO_PKG_VERSION");
const HOST_API_VERSION: &str = "planeai.plugin-host.v1";
const CLI_TIMEOUT: Duration = Duration::from_secs(20);
const POLL_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Clone, Debug, PartialEq, Serialize)]
struct Usage {
    plan: String,
    reset_date: String,
    covered_credits: f64,
    total_credits: f64,
    covered_percent: f64,
    note: Option<String>,
    fetched_at_ms: u128,
}

#[derive(Clone, Debug, Default)]
struct UsageState {
    usage: Option<Usage>,
    error: Option<String>,
    refreshing: bool,
}

#[derive(Clone)]
struct RefreshCoordinator {
    state: Arc<Mutex<UsageState>>,
    cancelled: Arc<AtomicBool>,
    worker: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
}

impl RefreshCoordinator {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(UsageState::default())),
            cancelled: Arc::new(AtomicBool::new(false)),
            worker: Arc::new(Mutex::new(None)),
        }
    }

    fn snapshot(&self) -> Value {
        let state = self
            .state
            .lock()
            .expect("usage state mutex poisoned")
            .clone();
        let stale = state.usage.is_some() && state.error.is_some();
        json!({
            "status": if state.usage.is_some() { "available" } else if state.refreshing { "loading" } else { "unavailable" },
            "usage": state.usage,
            "error": state.error,
            "refreshing": state.refreshing,
            "stale": stale,
        })
    }

    fn refresh(&self) -> Value {
        let mut state = self.state.lock().expect("usage state mutex poisoned");
        if state.refreshing {
            return snapshot_value(&state);
        }
        state.refreshing = true;
        state.error = None;
        drop(state);

        let state = Arc::clone(&self.state);
        let cancelled = Arc::clone(&self.cancelled);
        let worker = thread::spawn(move || {
            let result = fetch_usage(&cancelled);
            let mut state = state.lock().expect("usage state mutex poisoned");
            state.refreshing = false;
            match result {
                Ok(usage) => {
                    state.usage = Some(usage);
                    state.error = None;
                }
                Err(message) if message == "refresh cancelled" => {}
                Err(message) => state.error = Some(message),
            }
        });
        let mut worker_slot = self.worker.lock().expect("refresh worker mutex poisoned");
        if worker_slot
            .as_ref()
            .is_some_and(thread::JoinHandle::is_finished)
        {
            if let Some(completed) = worker_slot.take() {
                let _ = completed.join();
            }
        }
        *worker_slot = Some(worker);

        self.snapshot()
    }

    fn shutdown(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
        if let Some(worker) = self
            .worker
            .lock()
            .expect("refresh worker mutex poisoned")
            .take()
        {
            let _ = worker.join();
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("{PLUGIN_ID} starting");
    let coordinator = RefreshCoordinator::new();
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        let request: Value = match serde_json::from_str(&line) {
            Ok(request) => request,
            Err(error) => {
                eprintln!("{PLUGIN_ID} ignored malformed JSON-RPC frame: {error}");
                continue;
            }
        };
        let (response, shutdown) = dispatch(&request, &coordinator);
        if request.get("id").is_some() {
            write_frame(&mut stdout, &response)?;
        }
        if shutdown {
            coordinator.shutdown();
            eprintln!("{PLUGIN_ID} shutting down");
            break;
        }
    }
    Ok(())
}

fn dispatch(request: &Value, coordinator: &RefreshCoordinator) -> (Value, bool) {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();

    let response = match method {
        "plugin.handshake" => {
            if request
                .get("params")
                .and_then(|params| params.get("host_api_version"))
                .and_then(Value::as_str)
                != Some(HOST_API_VERSION)
            {
                error(id, -32001, "unsupported plugin host API version")
            } else {
                success(
                    id,
                    json!({
                        "plugin_id": PLUGIN_ID,
                        "plugin_name": PLUGIN_NAME,
                        "plugin_version": PLUGIN_VERSION,
                        "host_api_version": HOST_API_VERSION,
                    }),
                )
            }
        }
        "usage.status" => success(id, coordinator.snapshot()),
        "usage.refresh" => success(id, coordinator.refresh()),
        "plugin.shutdown" => success(id, json!({ "stopping": true })),
        _ => error(id, -32601, "method not found"),
    };
    (response, method == "plugin.shutdown")
}

fn snapshot_value(state: &UsageState) -> Value {
    let stale = state.usage.is_some() && state.error.is_some();
    json!({
        "status": if state.usage.is_some() { "available" } else if state.refreshing { "loading" } else { "unavailable" },
        "usage": state.usage,
        "error": state.error,
        "refreshing": state.refreshing,
        "stale": stale,
    })
}

fn fetch_usage(cancelled: &AtomicBool) -> Result<Usage, String> {
    let mut child = Command::new("kiro-cli")
        .args(["chat", "--no-interactive", "/usage"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Unable to start kiro-cli: {error}"))?;
    let deadline = Instant::now() + CLI_TIMEOUT;

    loop {
        if cancelled.load(Ordering::Relaxed) {
            kill_child(&mut child);
            return Err("refresh cancelled".to_string());
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = read_pipe(child.stdout.take())?;
                let stderr = read_pipe(child.stderr.take())?;
                if !status.success() {
                    return Err(process_error(status.code(), &stderr));
                }
                return parse_usage(&stdout).map(|mut usage| {
                    usage.fetched_at_ms = now_ms();
                    usage
                });
            }
            Ok(None) if Instant::now() >= deadline => {
                kill_child(&mut child);
                return Err("Kiro CLI usage request timed out after 20 seconds.".to_string());
            }
            Ok(None) => thread::sleep(POLL_INTERVAL),
            Err(error) => {
                kill_child(&mut child);
                return Err(format!("Unable to check Kiro CLI status: {error}"));
            }
        }
    }
}

fn kill_child(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn read_pipe(pipe: Option<impl Read>) -> Result<String, String> {
    let mut bytes = Vec::new();
    if let Some(mut pipe) = pipe {
        pipe.read_to_end(&mut bytes)
            .map_err(|error| format!("Unable to read Kiro CLI output: {error}"))?;
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn process_error(code: Option<i32>, stderr: &str) -> String {
    let detail = stderr.trim();
    let suffix = code
        .map(|code| format!(" (exit code {code})"))
        .unwrap_or_default();
    if detail.is_empty() {
        format!("Kiro CLI usage request failed{suffix}.")
    } else {
        format!(
            "Kiro CLI usage request failed{suffix}: {}",
            truncate(detail, 240)
        )
    }
}

fn parse_usage(output: &str) -> Result<Usage, String> {
    let lines: Vec<&str> = output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let header = lines
        .iter()
        .find(|line| line.starts_with("Estimated Usage |"))
        .ok_or("Kiro CLI returned unrecognised usage output.")?;
    let credits = lines
        .iter()
        .find(|line| line.starts_with("Credits ("))
        .ok_or("Kiro CLI usage output did not include credits.")?;

    let mut header_parts = header.split('|').map(str::trim);
    let title = header_parts.next().unwrap_or_default();
    if title != "Estimated Usage" {
        return Err("Kiro CLI returned unrecognised usage output.".to_string());
    }
    let reset = header_parts
        .next()
        .and_then(|part| part.strip_prefix("resets on "))
        .filter(|value| !value.is_empty())
        .ok_or("Kiro CLI usage output did not include a reset date.")?;
    let plan = header_parts
        .next()
        .filter(|value| !value.is_empty())
        .ok_or("Kiro CLI usage output did not include a plan.")?;

    let credit_body = credits
        .strip_prefix("Credits (")
        .and_then(|value| value.strip_suffix('%'))
        .ok_or("Kiro CLI usage output had invalid credit information.")?;
    let (covered, remainder) = credit_body
        .split_once(" of ")
        .ok_or("Kiro CLI usage output had invalid covered credits.")?;
    let (total, percent) = remainder
        .split_once(" covered in plan), ")
        .ok_or("Kiro CLI usage output had invalid total credits.")?;
    let covered_credits = parse_number(covered, "covered credits")?;
    let total_credits = parse_number(total, "total credits")?;
    let covered_percent = parse_number(percent, "covered percentage")?;

    let note = lines
        .iter()
        .filter(|line| **line != *header && **line != *credits)
        .map(|line| (*line).to_string())
        .collect::<Vec<_>>()
        .join(" ");

    Ok(Usage {
        plan: plan.to_string(),
        reset_date: reset.to_string(),
        covered_credits,
        total_credits,
        covered_percent,
        note: (!note.is_empty()).then_some(note),
        fetched_at_ms: 0,
    })
}

fn parse_number(value: &str, label: &str) -> Result<f64, String> {
    value
        .trim()
        .replace(',', "")
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite() && *number >= 0.0)
        .ok_or_else(|| format!("Kiro CLI usage output had invalid {label}."))
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn truncate(value: &str, limit: usize) -> String {
    let mut characters = value.chars();
    let prefix: String = characters.by_ref().take(limit).collect();
    if characters.next().is_some() {
        format!("{prefix}…")
    } else {
        prefix
    }
}

fn write_frame(output: &mut impl Write, frame: &Value) -> io::Result<()> {
    serde_json::to_writer(&mut *output, frame)?;
    output.write_all(b"\n")?;
    output.flush()
}

fn success(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

#[cfg(test)]
mod tests {
    use super::*;

    const USAGE_OUTPUT: &str = "Estimated Usage | resets on 2026-10-01 | KIRO PRO\nCredits (779.46 of 1000 covered in plan), 77.9%\nYour plan is managed by your organization's administrator.\n";

    #[test]
    fn parses_observed_kiro_usage_output() {
        let usage = parse_usage(USAGE_OUTPUT).unwrap();
        assert_eq!(usage.plan, "KIRO PRO");
        assert_eq!(usage.reset_date, "2026-10-01");
        assert_eq!(usage.covered_credits, 779.46);
        assert_eq!(usage.total_credits, 1000.0);
        assert_eq!(usage.covered_percent, 77.9);
        assert_eq!(
            usage.note.as_deref(),
            Some("Your plan is managed by your organization's administrator.")
        );
    }

    #[test]
    fn accepts_extra_lines_and_grouped_credit_numbers() {
        let usage = parse_usage(
            "Other notice\nEstimated Usage | resets on 2026-11-01 | KIRO ULTRA\nCredits (1,234.50 of 2,000 covered in plan), 61.725%\nSecond notice\n",
        )
        .unwrap();
        assert_eq!(usage.covered_credits, 1234.5);
        assert_eq!(usage.total_credits, 2000.0);
        assert_eq!(usage.covered_percent, 61.725);
        assert_eq!(usage.note.as_deref(), Some("Other notice Second notice"));
    }

    #[test]
    fn rejects_unrecognised_usage_output() {
        assert!(parse_usage("Estimated Usage | KIRO PRO\nCredits (oops)\n").is_err());
    }

    #[test]
    fn handshake_advertises_plugin_identity() {
        let coordinator = RefreshCoordinator::new();
        let (response, shutdown) = dispatch(
            &json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "plugin.handshake",
                "params": { "host_api_version": HOST_API_VERSION },
            }),
            &coordinator,
        );
        assert!(!shutdown);
        assert_eq!(response["result"]["plugin_id"], PLUGIN_ID);
        assert_eq!(response["result"]["host_api_version"], HOST_API_VERSION);
    }

    #[test]
    fn stale_snapshot_retains_last_success_after_failure() {
        let coordinator = RefreshCoordinator::new();
        {
            let mut state = coordinator.state.lock().unwrap();
            state.usage = Some(Usage {
                plan: "KIRO PRO".to_string(),
                reset_date: "2026-10-01".to_string(),
                covered_credits: 10.0,
                total_credits: 1000.0,
                covered_percent: 1.0,
                note: None,
                fetched_at_ms: 1,
            });
            state.error = Some("Kiro CLI usage request failed.".to_string());
        }
        let snapshot = coordinator.snapshot();
        assert_eq!(snapshot["status"], "available");
        assert_eq!(snapshot["stale"], true);
        assert_eq!(snapshot["usage"]["covered_percent"], 1.0);
    }

    #[test]
    fn shutdown_is_acknowledged() {
        let coordinator = RefreshCoordinator::new();
        let (response, shutdown) = dispatch(
            &json!({ "jsonrpc": "2.0", "id": 2, "method": "plugin.shutdown" }),
            &coordinator,
        );
        assert!(shutdown);
        assert_eq!(response["result"]["stopping"], true);
    }
}
