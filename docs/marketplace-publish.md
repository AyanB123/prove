# Publish Prove extension to VS Code Marketplace / Open VSX

## Prerequisites
1. [Azure DevOps publisher](https://marketplace.visualstudio.com/manage) account `ayanb123` (or change `publisher` in package.json)
2. Personal Access Token with **Marketplace → Manage**
3. Prove CLI documented for users

## Package
```bash
cd extensions/vscode-prove
npm install
npm run compile
node ../../scripts/package-vsix.js
# → prove-0.3.2.vsix
```

## Publish (when ready)
```bash
npm i -g @vscode/vsce
# or: npx @vscode/vsce
cd extensions/vscode-prove
vsce login ayanb123
vsce publish -p %VSCE_PAT%
```

Open VSX:
```bash
npx ovsx publish prove-0.3.2.vsix -p %OVSX_PAT%
```

## Until publish
Ship VSIX on GitHub Releases and install via **Install from VSIX**.

## Checklist before marketplace submit
- [ ] Icon 128x128 PNG present
- [ ] README has install + requirements
- [ ] CHANGELOG current
- [ ] LICENSE Apache-2.0
- [ ] No secrets in package
- [ ] Test on clean VS Code / Cursor with only VSIX + CLI binary
