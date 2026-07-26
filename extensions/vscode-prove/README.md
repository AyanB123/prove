# Prove — VS Code / Cursor extension

**Don't trust the agent. Trust the evidence.**

Sidebar + commands for the [Prove](https://github.com/AyanB123/prove) control plane.

## Features
- **Mission & Evidence** view — phase, backend, receipts, policy, artifacts
- **Receipt webview** — click a receipt for commands, exit codes, hashes, seal
- Status bar phase indicator
- Commands: Status, Verify, Verify (CI), Doctor, Init, Open Policy/Mission
- Watches `.prove/` for live updates

## Requirements
Install the Prove CLI:

```bash
# from release binary: https://github.com/AyanB123/prove/releases
# or from source:
cargo install --git https://github.com/AyanB123/prove
```

Optional: set `prove.cliPath` if `prove` is not on PATH.

## Install

### From VSIX (recommended until Marketplace listing)
1. Download `prove-0.3.2.vsix` from [Releases](https://github.com/AyanB123/prove/releases)
2. VS Code / Cursor → Extensions → `...` → **Install from VSIX...**

### Development
```bash
cd extensions/vscode-prove
npm install
npm run compile
# F5 → Extension Development Host
```

## Settings
| Setting | Default | Meaning |
|---------|---------|---------|
| `prove.cliPath` | `prove` | CLI binary path |
| `prove.autoRefreshMs` | `3000` | Poll interval (0 = watcher only) |

## Trust model
The extension **reads** `.prove/` and shells out to the CLI for verify/doctor.  
It never treats agent chat as evidence.

## License
Apache-2.0
