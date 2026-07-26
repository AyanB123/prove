# Prove

**Don't trust the agent. Trust the evidence.**

Agents can claim. Only evidence can advance.

Prove is a CLI-first control plane for coding agents. It sits above Claude Code, Codex, Aider, and local loops. Shared mission memory. Lifecycle state moves only when fresh, git-bound, machine-checkable receipts pass.

Not an IDE. Not a model. The reliability layer your agent stack is missing.

```text
false-done rate:  naive 10/10  →  prove 0/10
```

---

## The problem

- "All tests passed" is often a self-report
- Multi-CLI setups share no memory and no proof
- Reviewers cannot separate narrated success from evidence
- Long runs go false-done on visible greens while hidden contracts fail
- CI screenshots and chat logs go stale the moment the tree drifts

Generation quality is commoditized. Claim admissibility is not.

---

## Demo video

[45s demo (MP4)](demos/output/prove-demo-45s.mp4) · [GIF preview](demos/output/prove-demo-preview.gif)

```text
false-done rate: naive 10/10 → prove 0/10
```

## 30-second demo

Trap fixture: visible unit tests pass. Hidden contract fails.

| Path | What happens |
|------|----------------|
| Naive agent | Fixes the visible test, claims "tests passed", marks done. **False-done.** |
| Prove | Ignores the self-report, runs the full policy command set, **blocks done** until the hidden contract passes. |

```bash
cargo build --release
./target/release/prove eval traps
# false-done rate: naive 10/10 → prove 0/10
```

Or the split scenario (Windows PowerShell / bash):

```bash
# demos/demo.ps1  or  demos/demo.sh
# naive backend: blocked
# local-loop backend: done + prove pr evidence bundle
```

---

## VS Code / Cursor (v0.3)

Extension: [`extensions/vscode-prove`](extensions/vscode-prove) — mission sidebar, receipts, verify commands.

```bash
cd extensions/vscode-prove && npm install && npm run compile
# F5 in VS Code/Cursor to launch Extension Development Host
```

## CI (v0.2)

```yaml
- uses: AyanB123/prove@master
  with:
    policy_path: .prove/policy.yml
```

Local:

```bash
prove verify --ci --json
```

Full docs: [docs/github-action.md](docs/github-action.md)

## Install

```bash
git clone https://github.com/AyanB123/prove.git
cd prove
cargo install --path .
prove doctor
```

Requires Rust stable, `git`, and whatever tools your policy spawns (e.g. `python` + `pytest` for the trap fixtures).

---

## Quickstart

```bash
cd your-repo
prove init
prove run "fix flaky checkout race" --backend local-loop
prove status
prove verify
prove pr          # refuses unless phase=done with fresh receipts
```

Backends: `local-loop` | `naive` | `claude-code` | `aider` | `codex`

```bash
prove adapters test   # backend health
prove eval traps      # from the prove source repo
prove resume          # continue after stop/repair
```

---

## How it works

```text
User → prove CLI → Orchestrator → Backend adapters
                       |
                 Lifecycle engine   (claim ≠ state)
                       |
                 Proof gates → Receipt store (bound to head/tree hash)
```

### Lifecycle

`planned → patching → testing → reviewing → done → pr_ready`

Backends may **propose** transitions. Only admissible receipts **commit** them.

### Receipt (v1)

Each receipt binds:

- `head_hash` / `tree_hash` (freshness)
- `policy_hash` / `command_set_hash`
- per-command exit codes + stdout/stderr digests
- producer backend + run id

Stale tree, policy tamper, or failed command → rejected. No done.

### Policy (`.prove/policy.yml`)

```yaml
gates:
  test:
    commands:
      - [python, -m, pytest, -q]
    repair_limit: 3
  review:
    type: checklist
    require: [diff_non_empty, tests_fresh, no_todo_marker]
budgets:
  max_steps: 40
  max_minutes: 45
safety:
  deny_command_regex:
    - "rm\\s+-rf\\s+/"
    - "git\\s+push\\s+--force"
```

---

## Eval headline

Public trap suite under `fixtures/traps`:

| Case | What it catches |
|------|-----------------|
| visible-pass-hidden-fail | Optimizing only visible tests |
| stale-receipt-after-drift | Reusing proof after tree change |
| self-report-cannot-skip-gates | Chat claims jumping lifecycle |
| command-set-tamper | Policy / command-set hash mismatch |
| pr-refuses-without-done | Evidence export without done phase |

```text
false-done rate: naive 10/10 → prove 0/10
```

Run it yourself:

```bash
cargo test
./target/release/prove eval traps
```

---

## CLI

| Command | Purpose |
|---------|---------|
| `prove init` | Create `.prove/` policy + stores |
| `prove run "<goal>"` | Mission under proof-or-stop |
| `prove status` | Phase, repairs, evidence matrix |
| `prove verify` | Re-run gates against HEAD |
| `prove resume` | Continue after stop/repair |
| `prove pr` | Export JSON + markdown evidence bundle (gated) |
| `prove eval traps` | False-done suite |
| `prove adapters test` | Backend health |
| `prove doctor` | Environment checks |
| `prove policy show` | Active policy + hashes |

---

## Trust model

| Trust | Do not trust |
|-------|----------------|
| Local git + filesystem | Model prose |
| Commands Prove spawns under policy | Backend "tests passed" chat |
| Content-addressed receipts | Stale CI screenshots |

Hard invariants:

1. Backend self-reports never advance lifecycle
2. Test admission requires Prove-spawned command results
3. Receipts must match current head/tree + policy hashes
4. `prove pr` refuses unless phase is done and test receipt still admits
5. Budgets and repair limits stop honestly (`Failed` ≠ `Done`)

Details: [docs/trust-model.md](docs/trust-model.md)

---

## Non-goals (v0.1)

- Formal program verification
- Full OS sandbox (roadmap)
- Cryptographic multi-party signed receipts (roadmap)
- IDE / Cursor panel (roadmap)
- Hosted cloud control plane
- Proof of semantic equivalence to user intent beyond policy commands

Prove certifies **policy commands ran clean on this tree**. It does not certify that the agent understood you.

---

## Roadmap teaser

| Version | Focus |
|---------|--------|
| **v0.1** | Lifecycle, receipts, multi-CLI adapters, trap eval, PR bundle (this repo) |
| **v0.2** | GitHub Action `prove verify`, richer TUI, release binaries |
| **v0.3** | VS Code / Cursor panel on `.prove/`, streaming backend logs |
| **v0.4** | Stronger OS sandbox, signed receipts |
| **v0.5** | Team policy server, shared mission bus, cost accounting |

Full list: [docs/roadmap.md](docs/roadmap.md) · Architecture: [docs/architecture.md](docs/architecture.md)

---

## Why this is a portfolio piece

> I built the missing reliability + orchestration layer for multi-agent coding stacks: open-source, model-agnostic, evidence-first.

Code tour: `src/lifecycle.rs` → `src/receipts.rs` → `src/orchestrator.rs` → `src/adapters/` → `src/eval.rs`

Interview packet: [docs/job-packet.md](docs/job-packet.md)

---

## License

Apache-2.0. See [LICENSE](LICENSE).





