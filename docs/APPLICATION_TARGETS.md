# Application targets — Prove

12–20 startups and teams where Prove is a natural portfolio signal. Roles skew to AI coding agents, agent infra, and developer tools.

**Always attach:** public repo, 45s demo (when live), `docs/job-packet.md` pitch, `docs/eval-report.md` metric (`false-done naive 10/10 → prove 0/10`), code tour (`lifecycle` → `receipts` → `orchestrator` → `adapters` → `eval`).

Customize one sentence per company: name their product surface, then map Prove to reliability / admissibility / multi-agent control.

---

## P0 — highest map

| # | Company | Role angles | Why Prove maps | What to attach / say |
|---|---------|-------------|----------------|----------------------|
| 1 | **Cursor** | SWE, Agents, Infra, Product Eng | Agent-native IDE; trust and run reliability are product-critical | Repo + video; "control plane beside the agent, not another chat panel"; discuss false-done in long agent runs |
| 2 | **Anthropic** | SWE (Claude Code / API), Applied, Safety-adjacent eng | Claude Code is a first-class backend; evidence over self-report matches careful product culture | Eval report + trust model; "backends propose, receipts admit"; interest in harness design |
| 3 | **OpenAI** | SWE (Codex / agents), Applied Eng, Platform | Codex-class coding agents need runtime gates and eval beyond pass@k | Trap suite design; ignore `claimed_tests_passed` pattern; PR evidence bundle idea |
| 4 | **Cognition (Devin)** | SWE, Agent Runtime, Eval | Autonomous coding agents fail loudly on false completion | Lifecycle FSM + repair limits as honest stop; multi-step mission memory |
| 5 | **Sourcegraph (Amp / Cody)** | SWE, Cody/Amp, Platform | Code intelligence + agents; reviewers need evidence bound to HEAD | `prove pr` gated export; git head/tree binding story |
| 6 | **Factory** | SWE, Agent Infra, Product | Droids / agent coding workflows; orchestration and proof fit the thesis | Multi-CLI conductor + shared mission memory; portfolio shock metric |

---

## P1 — strong map

| # | Company | Role angles | Why Prove maps | What to attach / say |
|---|---------|-------------|----------------|----------------------|
| 7 | **Sweep** | SWE, Founding/early | PR-oriented agents; evidence bundles are the review surface | `prove pr` JSON/markdown; refuse export without done |
| 8 | **Continue** | SWE, Open source | Open hub for custom agents/models; model-agnostic control plane | Apache-2.0 CLI; adapters as pluggable backends |
| 9 | **Aider** (team / adjacent) | OSS eng, collaborator | Terminal coding agent; Prove sits above as reliability layer | local-loop vs external adapter contract; doctor + policy.yml |
| 10 | **Cline / similar agent-extension teams** | SWE, VS Code ext | IDE-embedded agents need non-chat proof | Roadmap: Cursor/VS Code panel on `.prove/`; status evidence matrix |
| 11 | **Replit (Agent)** | SWE, Agent platform | Hosted agent runs; budgets and stop conditions matter | Budgets, repair_limit, Failed ≠ Done |
| 12 | **GitHub (Copilot workspace / agents)** | SWE, Copilot | PR and CI-adjacent agent UX | Contrast "CI after the fact" vs runtime admissibility; Action `prove verify` roadmap |
| 13 | **Poolside** | SWE, Infra | Code models + products; eval culture | Why HumanEval is wrong headline for control planes; false-done metric |
| 14 | **Magic** | SWE, Research eng | Code gen lab; still needs claim discipline in tooling | Pure FSM + receipt hashes as testable systems design |

---

## P2 — agent infra / devtools adjacency

| # | Company | Role angles | Why Prove maps | What to attach / say |
|---|---------|-------------|----------------|----------------------|
| 15 | **LangChain / LangGraph** | SWE, Framework | Graph/runtime for agents; gates and state machines | Lifecycle as typed state; adapters as tools with untrusted outputs |
| 16 | **LlamaIndex** | SWE | Agent/workflow products over data+code | Mission memory + structured event log |
| 17 | **Modal** or **Fly.io** (AI/workload eng) | Platform SWE | People run agent workloads here; isolation roadmap | Local TCB honesty + sandbox roadmap; CLI packaging in Rust |
| 18 | **Temporal** (AI/workflow customers eng) | SWE | Durable workflows parallel "mission" semantics | Resume after stop/repair; fail closed; idempotent verify |
| 19 | **Exa / Parallel / other agent-native startups** | Founding eng, SWE | Thin product over agents; reliability is differentiation | 2-sentence pitch + trap demo; ship-small, measure hard |
| 20 | **Any seed/Series A "AI software engineer" product** | Founding eng | Same false-done pain | Lead with metric and code tour; offer to add their failure mode as a trap |

---

## Role titles to search

- Software Engineer, Agents / Agent Runtime  
- Software Engineer, Developer Tools / DevEx  
- Applied AI Engineer (coding agents)  
- Platform Engineer (AI products)  
- Evaluation / Harness Engineer  
- Research Engineer (tools, not only pretraining)  
- Founding Engineer (AI coding startup)

---

## Outreach sequence (per target)

1. Skim their agent product for one concrete failure mode (self-report, stale proof, multi-tool amnesia, PR without evidence).
2. Open with the 2-sentence pitch from `docs/job-packet.md`.
3. One mapping sentence: "For [product], Prove is the layer that [specific refusal/admission]."
4. Link repo + video + eval headline.
5. Close with one question from job-packet (false-done measurement, where admissibility lives, hardest production failure).
6. Attach or link code tour paths, not a generic resume wall of text.

---

## What not to do

- Do not lead with HumanEval/SWE-bench scores you do not have; lead with false-done.
- Do not claim OS sandbox or signed multi-party receipts as shipped (roadmap only).
- Do not imply live vendor CLI trap matrices without numbers; local-loop + naive suite is the honest headline.
- Do not spray identical mail merge; change the mapping sentence every time.

---

## Tracking template

| Company | Role | Priority | Applied | Link | Notes |
|---------|------|----------|---------|------|-------|
| Cursor | | P0 | | | |
| Anthropic | | P0 | | | |
| OpenAI | | P0 | | | |
| Cognition | | P0 | | | |
| Sourcegraph | | P0 | | | |
| Factory | | P0 | | | |
| Sweep | | P1 | | | |
| Continue | | P1 | | | |
| … | | | | | |
