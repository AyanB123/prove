# Trust model

## Actor assumptions

Coding agents can be helpful and still:

- over-claim success
- optimize for visible tests
- emit stale logs after tree drift
- narrate “done” without running the real suite

Prove assumes agents are **useful proposers** and **untrusted certifiers**.

## Trusted computing base (v0.1)

- Local filesystem of the repo
- Local `git` state fingerprints
- Commands spawned by Prove under `policy.yml`
- Receipt JSON written under `.prove/receipts/`

## Explicitly untrusted

- Natural language from any model/backend
- Backend exit narration
- Chat transcripts as proof of tests
- Old receipts after working tree changes
- Edited pytest output files not produced in-gate

## Gate predicates

A `tests_passed` receipt admits iff:

1. `head_hash` and `tree_hash` match current capture
2. `policy_hash` matches active policy
3. `command_set_hash` matches active test command set
4. Every command exit code is 0
5. Commands were not deny-listed

## Failure behavior

On failed admission:

- lifecycle moves to repair (patching) or stop (Failed)
- never to Done
- mission memory records rejection reason

## Non-goals (v0.1)

- Formal program verification
- Cryptographic multi-party signatures (roadmap)
- Full OS sandbox (roadmap)
- Proof of semantic equivalence to user intent beyond policy commands
