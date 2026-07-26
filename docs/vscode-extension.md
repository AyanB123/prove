# Prove VS Code / Cursor extension

Location: `extensions/vscode-prove`

## What it does
- Activity bar **Prove** view: mission phase, receipts, policy, artifacts
- Status bar phase indicator
- Commands shell out to the `prove` CLI (verify never trusts chat)

## Dev
```bash
cd extensions/vscode-prove
npm install
npm run compile
# VS Code: F5 → Extension Development Host
```

## Package
```bash
cd extensions/vscode-prove
npx @vscode/vsce package --no-dependencies
```

## Settings
- `prove.cliPath` — path to CLI (default `prove`)
- `prove.autoRefreshMs` — poll interval (default 3000)

## Requires
Prove CLI on PATH or `prove.cliPath` set to your release binary.

## Receipt webview
Click a receipt in the sidebar to open a formatted evidence view (commands, exit codes, seal, hashes).
