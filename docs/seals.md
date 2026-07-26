# Receipt seals

## Algorithms
| Alg | Command | Notes |
|-----|---------|-------|
| ed25519 (default) | `prove keys init` | Public key portable; multi-party |
| hmac-sha256 | `prove keys init --alg hmac-sha256` | Local symmetric |

## Multi-party quorum
```yaml
safety:
  require_sealed_receipts: true
  seal_quorum: 2
```

```bash
# machine A
prove keys init
prove keys pubkey   # share public hex + key_id

# machine B
prove keys init
prove keys trust --key-id <A_id> --pubkey <A_hex>
prove keys pubkey

# A trusts B
prove keys trust --key-id <B_id> --pubkey <B_hex>

# after A seals a receipt:
prove keys cosign .prove/receipts/rec_xxx.json   # on B
# admit requires seal_quorum distinct valid signers
```

Trusted pubs live in `.prove/keys/trusted/<key_id>.pub`.
