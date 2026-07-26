# Receipt seals (v0.2+)

Local integrity seals stop silent tampering of receipt JSON on disk.

## Usage
```bash
prove keys init     # writes .prove/keys/hmac.key (keep private)
prove keys status
prove verify        # new receipts include seal{alg,key_id,signature}
```

## Algorithm
- HMAC-SHA256 over `sha256("prove-receipt-v1" || canonical_unsigned_receipt_json)`
- Verified on admit when `.prove/` exists and receipt carries a seal
- Missing seal is allowed (backward compatible); invalid seal rejects admit

## Not yet
- Ed25519 multi-party signatures
- Required-seal policy flag
- Key rotation / remote KMS

This is the foundation for v0.4 signed receipts.


## Required seals

```yaml
safety:
  require_sealed_receipts: true
```

When true, admit fails unless the receipt carries a valid seal for the local key.
Workflow:

```bash
prove keys init
prove verify   # writes sealed receipts
```
