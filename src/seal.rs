use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

const KEY_FILE: &str = "hmac.key";
const KEY_ID_FILE: &str = "key_id";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReceiptSeal {
    pub alg: String,
    pub key_id: String,
    pub signature: String,
}

pub struct LocalKey {
    pub key_id: String,
    pub secret: Vec<u8>,
    pub path: PathBuf,
}

impl LocalKey {
    pub fn keys_dir(prove_dir: &Path) -> PathBuf {
        prove_dir.join("keys")
    }

    pub fn load(prove_dir: &Path) -> Result<Option<Self>> {
        let dir = Self::keys_dir(prove_dir);
        let key_path = dir.join(KEY_FILE);
        if !key_path.exists() {
            return Ok(None);
        }
        let secret = std::fs::read(&key_path)
            .with_context(|| format!("read key {}", key_path.display()))?;
        if secret.len() < 16 {
            bail!("prove key too short at {}", key_path.display());
        }
        let key_id = std::fs::read_to_string(dir.join(KEY_ID_FILE))
            .unwrap_or_else(|_| "local".into())
            .trim()
            .to_string();
        Ok(Some(Self {
            key_id,
            secret,
            path: key_path,
        }))
    }

    pub fn init(prove_dir: &Path) -> Result<Self> {
        let dir = Self::keys_dir(prove_dir);
        std::fs::create_dir_all(&dir)?;
        let key_path = dir.join(KEY_FILE);
        if key_path.exists() {
            return Self::load(prove_dir)?.ok_or_else(|| anyhow!("key missing after exists check"));
        }
        // 32 random bytes via uuid entropy (no extra deps)
        let mut secret = Vec::with_capacity(32);
        for _ in 0..2 {
            secret.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
        }
        std::fs::write(&key_path, &secret)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&key_path)?.permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(&key_path, perms)?;
        }
        let key_id = format!("k_{}", &uuid::Uuid::new_v4().simple().to_string()[..12]);
        std::fs::write(dir.join(KEY_ID_FILE), &key_id)?;
        // gitignore keys if a local gitignore exists under .prove
        let gi = prove_dir.join(".gitignore");
        if !gi.exists() {
            std::fs::write(gi, "keys/\nreceipts/\nmission.json\nevents.jsonl\nlocks/\nartifacts/\nmemory/\n")?;
        }
        Ok(Self {
            key_id,
            secret,
            path: key_path,
        })
    }

    pub fn sign_hex(&self, payload: &[u8]) -> String {
        // HMAC-SHA256 (RFC 2104) minimal impl
        const BLK: usize = 64;
        let mut key = self.secret.clone();
        if key.len() > BLK {
            let mut h = Sha256::new();
            h.update(&key);
            key = h.finalize().to_vec();
        }
        if key.len() < BLK {
            key.resize(BLK, 0);
        }
        let mut o_key = vec![0u8; BLK];
        let mut i_key = vec![0u8; BLK];
        for i in 0..BLK {
            o_key[i] = key[i] ^ 0x5c;
            i_key[i] = key[i] ^ 0x36;
        }
        let mut inner = Sha256::new();
        inner.update(&i_key);
        inner.update(payload);
        let mut outer = Sha256::new();
        outer.update(&o_key);
        outer.update(inner.finalize());
        hex::encode(outer.finalize())
    }

    pub fn verify_hex(&self, payload: &[u8], sig_hex: &str) -> bool {
        let expected = self.sign_hex(payload);
        // constant-time-ish compare
        if expected.len() != sig_hex.len() {
            return false;
        }
        let mut diff = 0u8;
        for (a, b) in expected.bytes().zip(sig_hex.bytes()) {
            diff |= a ^ b;
        }
        diff == 0
    }
}

pub fn sealing_payload(receipt_json_without_seal: &[u8]) -> Vec<u8> {
    let mut h = Sha256::new();
    h.update(b"prove-receipt-v1\0");
    h.update(receipt_json_without_seal);
    h.finalize().to_vec()
}

mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes
            .as_ref()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn roundtrip_hmac_seal() {
        let dir = tempdir().unwrap();
        let prove = dir.path().join(".prove");
        std::fs::create_dir_all(&prove).unwrap();
        let key = LocalKey::init(&prove).unwrap();
        let payload = sealing_payload(b"{\"hello\":1}");
        let sig = key.sign_hex(&payload);
        assert!(key.verify_hex(&payload, &sig));
        assert!(!key.verify_hex(&payload, "00"));
    }
}
