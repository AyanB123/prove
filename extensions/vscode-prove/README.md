# Prove — VS Code / Cursor extension

Sidebar + commands for the [Prove](https://github.com/AyanB123/prove) control plane.

## Features
- **Mission & Evidence** view: phase, backend, receipts, touched files
- Status bar: current phase (click → status)
- Commands: Status, Verify, Verify (CI), Doctor, Open Policy/Mission, Init, Refresh
- Auto-refresh when `.prove/` changes

## Setup
1. Install the `prove` CLI (`cargo install --path .` from repo root)
2. Open this folder in VS Code/Cursor **or** package the extension:
   ```bash
   cd extensions/vscode-prove
   npm install
   npm run compile
   # F5 to launch Extension Development Host
   # or: npx @vscode/vsce package
   ```
3. Open a workspace with `.prove/` (or run **Prove: Init**)

## Trust model
The extension **reads** `.prove/` and shells out to the CLI for verify/doctor. It never treats agent chat as evidence.

## License
Apache-2.0
