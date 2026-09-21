# PlaneAI Kiro Usage Plugin

A manually installed PlaneAI local plugin that displays the Kiro CLI’s current estimated usage. Version 1 supports macOS on Apple Silicon.

It runs `kiro-cli chat --no-interactive /usage` locally on startup and every five minutes. The titlebar shows the bundled purple Kiro ghost icon and the percentage of credits **covered by the plan**, opens a details modal for the active session, and provides a manual refresh. No account data is persisted. See [THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md) for the bundled Kiro icon notice.

## Install

1. Download and unpack the `planeai-plugin-kiro-usage-*-macos-arm64.tar.gz` asset from a release.
2. In PlaneAI, open **Plugin Manager** and choose **Install Local Plugin**.
3. Select the unpacked `planeai-plugin-kiro-usage` directory.

PlaneAI imports an immutable copy. To update, install the directory from a newer release.

## Requirements

- PlaneAI with local-plugin support
- macOS Apple Silicon
- `kiro-cli` on `PATH`, authenticated to Kiro

If Kiro CLI is unavailable, unauthenticated, takes longer than 20 seconds, exits unsuccessfully, or returns unrecognised usage output, the titlebar shows `Kiro —`. A last successful value remains visible in the details pane as stale for the current PlaneAI run only.

## Development

```bash
pnpm install --frozen-lockfile
make test
make verify-package
```

`make verify-package` stages `dist/planeai-plugin-kiro-usage` and verifies that the packaged executable handshake matches the manifest. Install that directory in PlaneAI for local testing.

## Release

The release process matches the PlaneAI GitHub plugin. On each `main` push, GitHub Actions validates the package, uses Conventional Commits to determine a version bump, creates a draft tag and release, builds the tagged package, verifies its handshake, uploads the archive, and publishes the release only after every build passes.

Version 1 builds the `macos-arm64` artifact only. The release archive contains `planeai-plugin.json`, `ui/entry.js`, and the executable under `bin/macos-arm64/`.
