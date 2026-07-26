# Prove — 45s demo storyboard

Recording target: **45 seconds**. Split-screen naive vs Prove. Terminal-first. Minimal captions.

**Assets ready:** `demos/demo.ps1`, `demos/demo.sh`, `prove eval traps`, trap fixture `fixtures/traps/visible-pass-hidden-fail`.

**Pre-roll (not filmed):**
```powershell
cargo build --release
# left pane: dark terminal titled NAIVE
# right pane: dark terminal titled PROVE
# font large enough for 1080p crop
```

---

## Shot list

| t | Visual | Terminal / action | On-screen text |
|---|--------|-------------------|----------------|
| **0:00–0:03** | Title card, solid dark bg | none | **Prove** · Don't trust the agent. Trust the evidence. |
| **0:03–0:06** | Split appears: LEFT = NAIVE, RIGHT = PROVE. Both in same trap repo dir | both show `pwd` / fixture path briefly | Trap: visible tests pass · hidden contract fails |
| **0:06–0:12** | LEFT focus (dim right) | `prove init` then `prove run "fix discount validation" --backend naive` | Naive agent claims success from visible greens |
| **0:12–0:16** | LEFT: status output freezes on false-done / claim language | `prove status` (phase stuck or claim without admissible done; naive self-report path) | **FALSE-DONE** (red badge, left only) |
| **0:16–0:22** | RIGHT focus (dim left) | `prove init` then `prove run "fix discount validation" --backend local-loop` | Prove runs full policy command set |
| **0:22–0:28** | RIGHT: gate lines scroll | proof gate test → hidden contract fails or repair loop → correct fix → test receipt admitted | Claim ignored · Receipt required |
| **0:28–0:33** | RIGHT: clean status | `prove status` showing `phase: done` + evidence matrix all checkmarks | **DONE** only with fresh receipts |
| **0:33–0:38** | RIGHT: PR export | `prove pr` → JSON/markdown evidence bundle snippet | `prove pr` gated on done + fresh tree |
| **0:38–0:42** | Full-width cut to eval | `./target/release/prove eval traps` final line | `false-done rate: naive 10/10 → prove 0/10` |
| **0:42–0:45** | End card | none | **Prove** · Agents can claim. Only evidence can advance. · `false-done 10/10 → 0/10` · github.com/\<you\>/prove |

---

## Caption style

- Max 6–8 words per line
- High contrast white on near-black
- No emoji
- Red only for FALSE-DONE; green only for admitted done / 0/5
- Avoid em dashes in lower-thirds

---

## Audio (optional VO, ~45s)

> Coding agents lie politely. They fix the tests you can see and call it done.
>
> Prove sits above Claude Code, Codex, Aider, and local loops.
> Backends can claim. Lifecycle only moves on git-bound receipts.
>
> Same trap. Naive false-dones. Prove blocks until the hidden contract passes.
>
> Eval: naive five of five false-done. Prove: zero.
>
> Don't trust the agent. Trust the evidence.

If silent: keep keyboard click soft, emphasize the eval line with a short hold.

---

## Exact commands to film

### Left (naive)
```bash
cd /tmp/prove_naive_trap   # copy of fixtures/traps/visible-pass-hidden-fail
prove init
# write policy with visible + hidden pytest commands (see demo.ps1)
prove run "fix discount validation" --backend naive
prove status
```

### Right (prove / local-loop)
```bash
cd /tmp/prove_loop_trap
prove init
# same policy
prove run "fix discount validation" --backend local-loop
prove status
prove pr
```

### Closer
```bash
cd /path/to/prove
./target/release/prove eval traps
```

Windows: run `demos/demo.ps1` once off-camera to confirm timings, then re-run panes separately for clean split recording.

---

## Edit notes

1. Speed-ramp long adapter waits; never rush the eval headline.
2. Hold FALSE-DONE badge ≥1.5s and DONE evidence matrix ≥2s.
3. Crop terminal scrollback so `phase` and receipt ids stay readable.
4. End card: leave 1s of silence after URL for pause-and-copy.
5. Export 1080p 30fps; also 9:16 center-crop with stacked panes if posting short-form.

---

## Checklist before record

- [ ] `cargo build --release` green
- [ ] `prove eval traps` prints `naive 10/10 → prove 0/10`
- [ ] demo.ps1 / demo.sh green once
- [ ] URL placeholder replaced
- [ ] No secrets in terminal scrollback


