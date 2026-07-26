use anyhow::{anyhow, bail, Context, Result};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

const HMAC_KEY_FILE: &str = "hmac.key";
const ED25519_SK_FILE: &str = "ed25519.sk";
const ED25519_PK_FILE: &str = "ed25519.pub";
const KEY_ID_FILE: &str = "key_id";
const ALG_FILE: &str = "alg";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SealAlg {
    HmacSha256,
    Ed25519,
}

impl SealAlg {
    pub fn as_str(self) -> &'static str {
        match self {
            SealAlg::HmacSha256 => "hmac-sha256",
            SealAlg::Ed25519 => "ed25519",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "hmac" | "hmac-sha256" => Ok(SealAlg::HmacSha256),
            "ed25519" | "eddsa" => Ok(SealAlg::Ed25519),
            other => bail!("unknown seal alg '{other}' (use hmac-sha256 or ed25519)"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReceiptSeal {
    pub alg: String,
    pub key_id: String,
    pub signature: String,
    /// Optional public key hex (ed25519) for portable verification / multi-party handoff.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
}

pub enum LocalKey {
    Hmac {
        key_id: String,
        secret: Vec<u8>,
        path: PathBuf,
    },
    Ed25519 {
        key_id: String,
        signing: SigningKey,
        verifying: VerifyingKey,
        path: PathBuf,
        public_path: PathBuf,
    },
}

impl LocalKey {
    pub fn keys_dir(prove_dir: &Path) -> PathBuf {
        prove_dir.join("keys")
    }

    pub fn key_id(&self) -> &str {
        match self {
            LocalKey::Hmac { key_id, .. } | LocalKey::Ed25519 { key_id, .. } => key_id,
        }
    }

    pub fn path(&self) -> &Path {
        match self {
            LocalKey::Hmac { path, .. } | LocalKey::Ed25519 { path, .. } => path,
        }
    }

    pub fn alg(&self) -> SealAlg {
        match self {
            LocalKey::Hmac { .. } => SealAlg::HmacSha256,
            LocalKey::Ed25519 { .. } => SealAlg::Ed25519,
        }
    }

    pub fn public_key_hex(&self) -> Option<String> {
        match self {
            LocalKey::Hmac { .. } => None,
            LocalKey::Ed25519 { verifying, .. } => Some(hex::encode(verifying.as_bytes())),
        }
    }

    pub fn load(prove_dir: &Path) -> Result<Option<Self>> {
        let dir = Self::keys_dir(prove_dir);
        if !dir.exists() {
            return Ok(None);
        }
        let key_id = std::fs::read_to_string(dir.join(KEY_ID_FILE))
            .unwrap_or_else(|_| "local".into())
            .trim()
            .to_string();
        let alg_hint = std::fs::read_to_string(dir.join(ALG_FILE))
            .ok()
            .map(|s| s.trim().to_string());

        let ed_sk = dir.join(ED25519_SK_FILE);
        let hmac = dir.join(HMAC_KEY_FILE);

        if ed_sk.exists()
            || alg_hint.as_deref() == Some("ed25519")
            || (!hmac.exists() && dir.join(ED25519_PK_FILE).exists())
        {
            if !ed_sk.exists() {
                return Ok(None);
            }
            let bytes = std::fs::read(&ed_sk)
                .with_context(|| format!("read {}", ed_sk.display()))?;
            if bytes.len() != 32 {
                bail!("ed25519.sk must be 32 bytes, got {}", bytes.len());
            }
            let mut seed = [0u8; 32];
            seed.copy_from_slice(&bytes);
            let signing = SigningKey::from_bytes(&seed);
            let verifying = signing.verifying_key();
            // refresh public file if missing
            let pk_path = dir.join(ED25519_PK_FILE);
            if !pk_path.exists() {
                let _ = std::fs::write(&pk_path, verifying.as_bytes());
            }
            return Ok(Some(LocalKey::Ed25519 {
                key_id,
                signing,
                verifying,
                path: ed_sk,
                public_path: pk_path,
            }));
        }

        if hmac.exists() {
            let secret = std::fs::read(&hmac)
                .with_context(|| format!("read {}", hmac.display()))?;
            if secret.len() < 16 {
                bail!("hmac key too short at {}", hmac.display());
            }
            return Ok(Some(LocalKey::Hmac {
                key_id,
                secret,
                path: hmac,
            }));
        }
        Ok(None)
    }

    pub fn init(prove_dir: &Path, alg: SealAlg) -> Result<Self> {
        let dir = Self::keys_dir(prove_dir);
        std::fs::create_dir_all(&dir)?;
        if Self::load(prove_dir)?.is_some() {
            return Self::load(prove_dir)?
                .ok_or_else(|| anyhow!("key missing after exists check"));
        }
        let key_id = format!("k_{}", &uuid::Uuid::new_v4().simple().to_string()[..12]);
        std::fs::write(dir.join(KEY_ID_FILE), &key_id)?;
        std::fs::write(dir.join(ALG_FILE), alg.as_str())?;

        let gi = prove_dir.join(".gitignore");
        if !gi.exists() {
            std::fs::write(
                gi,
                "keys/\nreceipts/\nmission.json\nevents.jsonl\nlocks/\nartifacts/\nmemory/\n",
            )?;
        }

        match alg {
            SealAlg::HmacSha256 => {
                let key_path = dir.join(HMAC_KEY_FILE);
                let mut secret = Vec::with_capacity(32);
                for _ in 0..2 {
                    secret.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
                }
                std::fs::write(&key_path, &secret)?;
                chmod600(&key_path)?;
                Ok(LocalKey::Hmac {
                    key_id,
                    secret,
                    path: key_path,
                })
            }
            SealAlg::Ed25519 => {
                let mut csprng = OsRng;
                let signing = SigningKey::generate(&mut csprng);
                let verifying = signing.verifying_key();
                let sk_path = dir.join(ED25519_SK_FILE);
                let pk_path = dir.join(ED25519_PK_FILE);
                std::fs::write(&sk_path, signing.to_bytes())?;
                std::fs::write(&pk_path, verifying.as_bytes())?;
                chmod600(&sk_path)?;
                Ok(LocalKey::Ed25519 {
                    key_id,
                    signing,
                    verifying,
                    path: sk_path,
                    public_path: pk_path,
                })
            }
        }
    }

    pub fn sign_hex(&self, payload: &[u8]) -> String {
        match self {
            LocalKey::Hmac { secret, .. } => hmac_sha256_hex(secret, payload),
            LocalKey::Ed25519 { signing, .. } => {
                let sig: Signature = signing.sign(payload);
                hex::encode(sig.to_bytes())
            }
        }
    }

    pub fn verify_hex(&self, payload: &[u8], sig_hex: &str) -> bool {
        match self {
            LocalKey::Hmac { secret, .. } => {
                let expected = hmac_sha256_hex(secret, payload);
                ct_eq_hex(&expected, sig_hex)
            }
            LocalKey::Ed25519 { verifying, .. } => {
                let Ok(bytes) = hex::decode(sig_hex) else {
                    return false;
                };
                let Ok(sig_arr) = <[u8; 64]>::try_from(bytes.as_slice()) else {
                    return false;
                };
                let sig = Signature::from_bytes(&sig_arr);
                verifying.verify(payload, &sig).is_ok()
            }
        }
    }

    pub fn make_seal(&self, payload: &[u8]) -> ReceiptSeal {
        ReceiptSeal {
            alg: self.alg().as_str().into(),
            key_id: self.key_id().into(),
            signature: self.sign_hex(payload),
            public_key: self.public_key_hex(),
        }
    }
}

fn chmod600(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)?.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(path, perms)?;
    }
    let _ = path;
    Ok(())
}

fn hmac_sha256_hex(secret: &[u8], payload: &[u8]) -> String {
    const BLK: usize = 64;
    let mut key = secret.to_vec();
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

fn ct_eq_hex(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        diff |= x ^ y;
    }
    diff == 0
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

    pub fn decode(s: &str) -> Result<Vec<u8>, ()> {
        if !s.len().is_multiple_of(2) {
            return Err(());
        }
        let mut out = Vec::with_capacity(s.len() / 2);
        let bytes = s.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let hi = from_hex(bytes[i])?;
            let lo = from_hex(bytes[i + 1])?;
            out.push((hi << 4) | lo);
            i += 2;
        }
        Ok(out)
    }

    fn from_hex(b: u8) -> Result<u8, ()> {
        match b {
            b'0'..=b'9' => Ok(b - b'0'),
            b'a'..=b'f' => Ok(b - b'a' + 10),
            b'A'..=b'F' => Ok(b - b'A' + 10),
            _ => Err(()),
        }
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
        let key = LocalKey::init(&prove, SealAlg::HmacSha256).unwrap();
        let payload = sealing_payload(b"{\"hello\":1}");
        let sig = key.sign_hex(&payload);
        assert!(key.verify_hex(&payload, &sig));
        assert!(!key.verify_hex(&payload, "00"));
        assert_eq!(key.alg(), SealAlg::HmacSha256);
    }

    #[test]
    fn roundtrip_ed25519_seal() {
        let dir = tempdir().unwrap();
        let prove = dir.path().join(".prove");
        std::fs::create_dir_all(&prove).unwrap();
        let key = LocalKey::init(&prove, SealAlg::Ed25519).unwrap();
        let payload = sealing_payload(b"{\"hello\":2}");
        let seal = key.make_seal(&payload);
        assert_eq!(seal.alg, "ed25519");
        assert!(seal.public_key.is_some());
        assert!(key.verify_hex(&payload, &seal.signature));
        // reload from disk
        let loaded = LocalKey::load(&prove).unwrap().unwrap();
        assert_eq!(loaded.alg(), SealAlg::Ed25519);
        assert!(loaded.verify_hex(&payload, &seal.signature));
    }
}
