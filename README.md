# PlaneAI Kiro Usage Plugin

A manually installed PlaneAI local plugin that displays the Kiro CLI’s current estimated usage. Version 1 supports macOS on Apple Silicon.

It runs `kiro-cli chat --no-interactive /usage` locally on startup and every five minutes. The indicator shows the percentage of credits **covered by the plan**, opens a details modal for the active session, and provides a manual refresh. No account data is persisted.

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
cargo test
./scripts/package.sh
```

The release archive is written to `dist/`. Its unpacked directory is directly installable in PlaneAI.

## Release

Push a `v*` tag. GitHub Actions builds the macOS-arm64 tarball and attaches it to a GitHub Release.
