# Team mission bus

Share missions and sealed receipts across machines without a cloud account.

## Bundle format
`.provebundle.json` includes mission, receipts (with seals/cosignatures), events tail, git head/tree, policy hash.

## Commands
```bash
# export local mission
prove bus export
prove bus export -o share/mission.provebundle.json --include-policy

# import on another clone
prove bus import share/mission.provebundle.json
prove bus import share/mission.provebundle.json --force

# shared folder workflow (Dropbox / network drive / git-annex)
prove bus push --dir //team/share/prove-bus
prove bus list --dir //team/share/prove-bus
prove bus pull --dir //team/share/prove-bus
prove bus pull --dir //team/share/prove-bus --mission mis_abc --force
```

## Cost ledger
```bash
prove cost
```
Sums gate command durations and backend receipt counts (not LLM token $).

## Trust
Import preserves receipt seals. Admit still enforces policy, sandbox, and seal quorum locally.
