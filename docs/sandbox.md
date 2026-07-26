# Gate command sandbox (v0.4 foundation)

Prove never trusts agent claims. Gate commands still need isolation so a hostile or buggy suite cannot freely use secrets / network / the whole machine.

## Modes (`safety.sandbox` in policy.yml)

| Mode | Behavior |
|------|----------|
| `off` | Inherit full environment (legacy) |
| `standard` (default) | Scrub secrets, allowlist env, no shell, cwd = repo, timeout |
| `strict` | standard + Linux `bwrap` when installed (`--unshare-net` unless allow_network) |

## Policy knobs

```yaml
safety:
  sandbox: standard          # off | standard | strict
  allow_network: false
  command_timeout_secs: 900
  deny_command_regex:
    - "rm\\s+-rf\\s+/"
```

## What standard scrub drops
- Tokens/secrets (`*TOKEN*`, `*SECRET*`, `AWS_*`, `OPENAI_*`, `GITHUB_TOKEN`, …)
- Non-allowlisted env vars

## What remains allowed
PATH, basic locale/home/temp, common toolchain homes (`CARGO_HOME`, `JAVA_HOME`, …).

## Marker
Sandboxed runs set `PROVE_SANDBOX=1`.

## Doctor
`prove doctor` prints the active sandbox description.

## Limits (honest)
- Not a full VM/container
- Windows has no bwrap; standard mode is env/cwd/timeout isolation
- Network block is hard only under Linux strict+bwrap; standard sets offline hints for pip/npm
