# Roadmap

## v0.1 — shock MVP — DONE
- [x] Lifecycle engine + receipts
- [x] Local verify runner + policy
- [x] local-loop + naive + external adapters
- [x] Trap suite (false-done 10/10 → 0/10)
- [x] PR evidence bundle
- [x] Demo video + GitHub release

## v0.2 — team CI surface — IN PROGRESS
- [x] `prove verify --ci --json --require-done`
- [x] GitHub Action `AyanB123/prove` (`action.yml`)
- [x] In-repo CI workflow (test + self-verify)
- [x] Release workflow (linux/mac/win binaries on tags)
- [x] Example CI policies (`examples/`)
- [ ] Richer TUI status (optional)
- [ ] Consume prebuilt release binary in Action (faster than build-from-source)

## v0.3
- [ ] VS Code / Cursor panel reading `.prove/`
- [ ] Streaming backend logs

## v0.4
- [ ] Stronger sandbox (OS-level)
- [ ] Signed receipts

## v0.5
- [ ] Team policy server / shared mission bus
- [ ] Cost accounting across backends

## Later
- Optional computer-use backend
- Richer code graph
- Hosted eval leaderboard
