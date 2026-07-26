# Contributing

## Dev
```bash
cargo test
cargo build --release
./target/release/prove eval traps
```

## Invariants (do not break)
1. Backend self-reports never advance lifecycle.
2. Test admission requires Prove-spawned command results bound to tree/policy hashes.
3. `prove pr` refuses unless phase is done and test receipt still admits.
4. Trap suite must stay `prove false-done = 0`.

## Layout
- `src/lifecycle.rs` — pure FSM
- `src/receipts.rs` — evidence
- `src/orchestrator.rs` — mission loop
- `src/adapters/` — backends
- `src/eval.rs` — trap suite
