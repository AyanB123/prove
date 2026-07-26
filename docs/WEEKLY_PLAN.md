# Prove — ordered execution plan

## Locked decisions
- Dual core: Proof-or-Stop + Multi-CLI conductor
- CLI-first, Apache-2.0
- Target: startup eng teams 2–20
- Portfolio shock in weeks 1–4

## Week 1 — DONE
- [x] Lifecycle FSM (claim ≠ state)
- [x] Receipt mint/admit + policy hashes
- [x] Local verify runner
- [x] local-loop + naive adapters
- [x] Trap fixture: visible-pass-hidden-fail
- [x] `prove eval traps` green

## Week 2 — DONE
- [x] External adapters with pre/post file diff (claude-code, aider, codex)
- [x] Prove-aware prompts + repair hints + memory pack
- [x] Rule router preferring installed real CLIs
- [x] Status evidence matrix
- [x] `prove pr` JSON + markdown evidence bundle
- [x] `prove doctor`
- [x] Demo scripts (naive block vs local-loop done)
- [ ] Optional smoke: one real `claude`/`aider` turn on trap (manual; needs network/auth)

## Week 3 — DONE (code + docs)
- [x] Expand trap suite to 10 cases
- [x] `docs/eval-report.md` with methodology + numbers
- [x] Launch README punch-up
- [x] 45s demo storyboard (`demos/STORYBOARD-45s.md`)
- [x] Show HN title + first comment draft (`demos/SHOW_HN.md`)
- [x] Record demo video (`demos/output/prove-demo-45s.mp4`)
- [x] Replace GitHub URL placeholders → https://github.com/AyanB123/prove

## Week 4 — DONE (code + docs)
- [x] Hardening (Windows paths, clearer errors, status truncation, policy schema errors)
- [x] Job application packet polish (`docs/job-packet.md`)
- [x] Launch checklist (`docs/LAUNCH_CHECKLIST.md`)
- [x] Application targets (`docs/APPLICATION_TARGETS.md`)
- [x] Final verify: cargo test 12/12, eval naive 10/10 → prove 0/10
- [ ] Public launch posts (HUMAN — Show HN / LinkedIn / Twitter; drafts ready)
- [ ] Send applications (HUMAN — use job-packet + APPLICATION_TARGETS)

## Command sequence (daily driver)
```bash
prove doctor
prove init
prove run "..." --backend local-loop   # or claude-code|aider|codex
prove status
prove verify
prove pr
prove eval traps
```

## Trap suite (10)
1. visible-pass-hidden-fail
2. stale-receipt-after-drift
3. self-report-cannot-skip-gates
4. command-set-tamper
5. pr-refuses-without-done
6. deny-dangerous-command
7. todo-marker-blocks-review
8. lifecycle-cannot-skip-to-done
9. missing-test-blocks-review
10. repair-limit-stops-without-done

## Launch metrics (current)
```text
false-done rate: naive 10/10 → prove 0/10
prove blocked correctly: 10/10
unit tests: 12 passed
```

## What only you can do next
1. Create GitHub repo and push (see `docs/LAUNCH_CHECKLIST.md`)
2. Replace `github.com/AyanB123/prove` placeholders
3. Record 45s video from `demos/STORYBOARD-45s.md`
4. Post Show HN / LinkedIn / Twitter
5. Apply using `docs/job-packet.md` + `docs/APPLICATION_TARGETS.md`



## v0.2 product (post-launch) — CI surface
- [x] `prove verify --ci --json --require-done`
- [x] GitHub Action (`action.yml`)
- [x] `.github/workflows/ci.yml` + `release.yml`
- [x] Example policies + `docs/github-action.md`
- [ ] VS Code/Cursor panel (v0.3)
- [ ] Sandbox + signed receipts (v0.4)
