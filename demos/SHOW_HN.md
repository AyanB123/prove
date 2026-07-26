# Show HN draft

## Title

**Show HN: Prove – evidence-gated control plane for coding agents (false-done 10/10 → 0/10)**

Alternate titles (pick one):

1. **Show HN: Prove – don't trust the agent, trust the evidence**
2. **Show HN: Prove – multi-CLI agent control plane that blocks false-done**
3. **Show HN: Prove – coding agents can claim; only receipts advance state**

**Chosen for launch:** the first title (metric in the title). Metrics convert on Show HN when they are reproducible from the repo.

---

## First comment (post immediately after submission)

Hi HN,

I built **Prove**, an open-source CLI control plane for coding agents.

**One-liner:** agents can claim; only evidence can advance.

### Why

I kept watching multi-CLI setups (Claude Code, Codex, Aider, local loops) produce confident "all tests passed" narratives that were not machine-checkable. Visible greens, hidden contracts still red. Stale logs after the tree moved. Reviewers stuck reading chat instead of proof.

Generation quality is mostly commoditized. **Claim admissibility** is not.

### What Prove does

Prove sits above your agent CLIs as a local control plane:

1. Shared mission memory across backends
2. Lifecycle FSM where claim ≠ state (`planned → patching → testing → reviewing → done`)
3. Proof gates that only admit **git-state-bound receipts** (head/tree + policy + command digests)
4. `prove pr` that refuses to export an evidence bundle unless phase is done and receipts still admit on HEAD

Backends may propose transitions. Self-reports never commit them.

### Reproducible eval

Public trap suite in the repo:

```text
false-done rate: naive 10/10 → prove 0/10
```

```bash
cargo build --release
./target/release/prove eval traps
```

Traps include visible-pass-hidden-fail, stale receipt after drift, self-report skip attempts, command-set tamper, and PR export without done.

### What it is not

- Not an IDE
- Not a foundation model
- Not a hosted cloud agent
- Not formal verification or a full OS sandbox (those are roadmap)

v0.1 certifies that **your policy commands ran clean on this tree**. That is a deliberately narrow trust story.

### Stack

Rust CLI. Adapters: `local-loop`, `naive`, `claude-code`, `aider`, `codex`. Apache-2.0.

### Ask

Try the trap eval and tell me which false-done mode I am still missing. If you run multi-agent coding day to day, I want the cases that burned you.

Repo: https://github.com/AyanB123/prove  
Tagline: Don't trust the agent. Trust the evidence.

---

## Submission checklist

- [ ] Public GitHub repo with punched README
- [ ] `prove eval traps` green on a clean clone
- [ ] 45s demo video unlisted → public at submit time
- [ ] Replace `\<you\>` URL placeholders
- [ ] Post during US morning weekday if possible
- [ ] First comment within 2 minutes of submission
- [ ] Stay in thread for replies (adapter questions, trust model, Windows paths)

## Reply bank (short)

**"How is this different from CI?"**  
CI runs after the fact. Prove gates the agent runtime so lifecycle cannot reach done on a self-report. Receipts bind to the live tree hash, not a pasted log.

**"Does it need Claude/Codex installed?"**  
No. `local-loop` and the trap eval run without external agent CLIs. Real adapters are optional.

**"Can the agent edit receipts?"**  
Receipt admission re-checks head/tree, policy hash, and command-set hash. Tamper and drift fail closed. Full signed multi-party receipts are roadmap, not v0.1.

**"Why Rust?"**  
Local CLI, predictable packaging, pure lifecycle/receipt logic easy to test without a model in the loop.



