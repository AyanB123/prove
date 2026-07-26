# Receipt seals

## Algorithms
| Alg | CLI | Files | Notes |
|-----|-----|-------|-------|
| **ed25519** (default) | `prove keys init` | `ed25519.sk`, `ed25519.pub` | Public key portable; multi-party ready |
| hmac-sha256 | `prove keys init --alg hmac-sha256` | `hmac.key` | Symmetric local integrity |

## Commands
```bash
prove keys init                 # ed25519
prove keys init --alg hmac-sha256
prove keys status
prove keys pubkey               # ed25519 public hex
```

## Required seals
```yaml
safety:
  require_sealed_receipts: true
```

## Seal payload
HMAC/Ed25519 over `sha256("prove-receipt-v1" || canonical_unsigned_receipt_json)`.

Ed25519 seals may include `public_key` on the receipt for handoff verification.
