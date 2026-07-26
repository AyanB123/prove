# Job packet — Prove

Application-ready notes for AI coding / agent infra / devtools roles.

---

## 2-sentence pitch

I built Prove, an open-source evidence-gated control plane for coding agents: Claude Code, Codex, Aider, and local loops run as backends, but lifecycle only advances on git-state-bound receipts. Generation is commoditized; claim admissibility is not, and Prove is the reliability layer that makes "done" mean something machine-checkable.

---

## What I built

- CLI-first control plane (`prove`) in Rust, Apache-2.0
- Pure lifecycle FSM: `planned → patching → testing → reviewing → done → pr_ready` (claim ≠ state)
- Receipt mint/admit bound to head/tree + policy + command-set hashes
- Multi-CLI conductor with shared mission memory; backends propose, Prove admits
- Adapters: `local-loop`, `naive`, `claude-code`, `aider`, `codex`
- Policy-driven proof gates (`.prove/policy.yml`): test commands, review checklist, budgets, deny lists
- `prove pr` exports JSON + markdown evidence bundles only when phase is done and receipts still admit
- Public trap suite (`prove eval traps`) measuring false-done, not vanity pass@k
- Doctor, status evidence matrix, resume, adapters health check

---

## Technical depth (systems points interviewers love)

1. **Admissibility as the product surface.** Separated generation (untrusted proposers) from certification (Prove-spawned gates + pure FSM). Backend `claimed_tests_passed` is logged and ignored for lifecycle admission.
2. **Content-addressed evidence.** Receipts bind `head_hash` / `tree_hash` / `policy_hash` / `command_set_hash` plus per-command exit codes and stdout/stderr digests. Drift, tamper, or failed command → reject. Fail closed.
3. **Illegal transitions cannot skip to done.** Lifecycle is pure and unit-tested; self-report cannot jump Patching → Done without ReviewReceiptAdmitted-class events.
4. **Honest stop ≠ Done.** Repair limits and budgets surface `Failed` without minting false success. `prove pr` refuses export unless phase is done and the test receipt still admits on HEAD.
5. **Multi-backend conductor without shared trust.** Same mission memory and policy across Claude Code / Codex / Aider / local-loop; none of them certify their own success.
6. **Eval that would break if the trust model broke.** Trap suite targets self-report, visible-only greens, stale receipts, command-set tamper, PR-without-done, deny lists, TODO markers, missing tests, repair-limit honesty. Headline metric is false-done rate.

---

## Metrics

```text
false-done rate: naive 10/10 → prove 0/10
blocked correctly: 10/10
```

Reproduce from a clean clone:

```bash
cargo build --release
./target/release/prove eval traps
```

Full methodology: `docs/eval-report.md`.

---

## 60s demo commands

```bash
# build + headline eval
cargo build --release
./target/release/prove eval traps
# expect: false-done rate: naive 10/10 → prove 0/10

# split scenario (Windows / bash)
# demos/demo.ps1   or   demos/demo.sh
# naive backend: blocked
# local-loop backend: done + prove pr evidence bundle

# live walkthrough on any git repo
prove doctor
prove init
prove run "fix flaky checkout race" --backend local-loop
prove status
prove verify
prove pr
```

---

## Code tour order

1. `src/lifecycle.rs` — illegal transitions cannot skip to done
2. `src/receipts.rs` — mint/admit evidence; hash binding
3. `src/orchestrator.rs` — ignores `claimed_tests_passed`; proof gates
4. `src/adapters/mod.rs` — naive vs local-loop vs external CLIs
5. `src/eval.rs` — trap suite and false-done metrics
6. `src/policy.rs` + `.prove/policy.yml` shape — gates, budgets, deny lists
7. `docs/trust-model.md` + `docs/architecture.md` — invariants and module map

---

## Target companies / roles

**Companies (examples):** Cursor, Anthropic, OpenAI, Cognition, Sourcegraph, Factory, Sweep, Continue, Aider-adjacent teams, agent infra startups, developer-experience / platform eng orgs shipping coding agents.

**Roles:** Software Engineer (Agents / Infra), Applied AI Engineer, Developer Tools Engineer, Platform / Reliability Engineer for agent stacks, Research Engineer (evaluation / harnesses), Founding / early eng on multi-agent products.

**Why Prove maps:** You already ship generation. Prove shows I design the layer that refuses unearned success: lifecycle, receipts, multi-CLI orchestration, and a measurable false-done eval.

See also: `docs/APPLICATION_TARGETS.md`.

---

## Email / LinkedIn blurb (short)

Built Prove, an open-source evidence-gated control plane for coding agents (Rust, Apache-2.0). Claude Code / Codex / Aider / local loops run as backends; lifecycle only advances on git-bound receipts. Public trap suite: false-done naive 10/10 → prove 0/10. Looking for eng roles on AI coding agents, agent infra, or devtools. Repo + 45s demo on request.

---

## Questions I'd love to discuss

1. How do you define and measure false-done (or unearned "task complete") in your agent product today, and where does trust still collapse to chat or CI screenshots?
2. If claim admissibility were a first-class subsystem, would you put it in the IDE, the harness, CI, or a local control plane beside the model CLIs?
3. What is the hardest production failure mode you still see: visible-only greens, stale proof after drift, multi-agent memory gaps, or PR export without machine-checkable evidence on HEAD?

---

## Attach with every application

| Artifact | Path / note |
|----------|-------------|
| Repo | public GitHub once launched |
| Pitch + tour | this file |
| Eval numbers | `docs/eval-report.md` |
| Architecture / trust | `docs/architecture.md`, `docs/trust-model.md` |
| 45s demo | record from `demos/STORYBOARD-45s.md` |
| Launch narrative | `demos/SHOW_HN.md` (first comment is reusable for blogs) |

---

## Next 30 days (if hired / if continuing)

- GitHub Action `prove verify`
- Cursor / VS Code panel on `.prove/`
- Stronger OS sandbox + signed receipts
- Team policy server and shared mission bus
