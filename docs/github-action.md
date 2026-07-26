# GitHub Action — Prove Verify

Drop-in PR gate: run policy-bound commands and **fail the job** if evidence does not admit.

> Agents can claim. Only evidence can advance.

## Consumer workflow (other repos)

```yaml
# .github/workflows/prove.yml
name: Prove
on:
  pull_request:
  push:
    branches: [main, master]

jobs:
  verify:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      # optional: install language tooling your policy needs
      - uses: dtolnay/rust-toolchain@stable   # if policy runs cargo test
      # - run: pip install pytest            # if policy runs pytest

      - name: Prove verify
        uses: AyanB123/prove@master
        with:
          policy_path: .prove/policy.yml   # or examples path you commit
          # require_done: "true"           # also require mission Done + review receipt
```

## Inputs

| Input | Default | Meaning |
|-------|---------|---------|
| `policy_path` | `.prove/policy.yml` | Gate command set |
| `require_done` | `false` | Also require mission Done/PrReady |
| `toolchain` | `stable` | Rust used to build Prove |
| `working_directory` | `.` | Subdir to verify |
| `extra_args` | `` | Extra CLI args |
| `version` | latest release | Prebuilt tag e.g. `v0.2.0` |
| `build_from_source` | `false` | Force cargo build |

## Local equivalent

```bash
prove init   # or commit your policy.yml
prove verify --ci --json
prove verify --ci --require-done   # after prove run ... completed
```

Exit codes:
- `0` — admitted
- `1` — rejected (`--ci`)

## Example policies

- `examples/ci-policy.cargo-test.yml`
- `examples/ci-policy.pytest.yml`

## Trust model in CI

Prove spawns the policy commands itself. Job logs saying "tests passed" are not enough — the Action fails unless command exit codes are 0 under the bound policy hash.

See `docs/trust-model.md` and `docs/eval-report.md`.

