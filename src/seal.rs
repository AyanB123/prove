use anyhow::{anyhow, bail, Context, Result};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const HMAC_KEY_FILE: &str = "hmac.key";
const ED25519_SK_FILE: &str = "ed25519.sk";
const ED25519_PK_FILE: &str = "ed25519.pub";
const KEY_ID_FILE: &str = "key_id";
const ALG_FILE: &str = "alg";
const TRUSTED_DIR: &str = "trusted";

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
pub struct Cosignature {
    pub key_id: String,
    pub signature: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReceiptSeal {
    pub alg: String,
    pub key_id: String,
    pub signature: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    /// Additional ed25519 cosignatures for multi-party quorum.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cosignatures: Vec<Cosignature>,
}

impl ReceiptSeal {
    pub fn all_signers(&self) -> Vec<(&str, &str, Option<&str>)> {
        let mut out = vec![(
            self.key_id.as_str(),
            self.signature.as_str(),
            self.public_key.as_deref(),
        )];
        for c in &self.cosignatures {
            out.push((
                c.key_id.as_str(),
                c.signature.as_str(),
                c.public_key.as_deref(),
            ));
        }
        out
    }
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

    pub fn trusted_dir(prove_dir: &Path) -> PathBuf {
        Self::keys_dir(prove_dir).join(TRUSTED_DIR)
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

        if ed_sk.exists() || alg_hint.as_deref() == Some("ed25519") {
            if !ed_sk.exists() {
                return Ok(None);
            }
            let bytes =
                std::fs::read(&ed_sk).with_context(|| format!("read {}", ed_sk.display()))?;
            if bytes.len() != 32 {
                bail!("ed25519.sk must be 32 bytes, got {}", bytes.len());
            }
            let mut seed = [0u8; 32];
            seed.copy_from_slice(&bytes);
            let signing = SigningKey::from_bytes(&seed);
            let verifying = signing.verifying_key();
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
            let secret =
                std::fs::read(&hmac).with_context(|| format!("read {}", hmac.display()))?;
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
            return Self::load(prove_dir)?.ok_or_else(|| anyhow!("key missing after exists check"));
        }
        let key_id = format!("k_{}", &uuid::Uuid::new_v4().simple().to_string()[..12]);
        std::fs::write(dir.join(KEY_ID_FILE), &key_id)?;
        std::fs::write(dir.join(ALG_FILE), alg.as_str())?;
        std::fs::create_dir_all(Self::trusted_dir(prove_dir))?;

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
                verify_ed25519_hex(verifying, payload, sig_hex)
            }
        }
    }

    pub fn make_seal(&self, payload: &[u8]) -> ReceiptSeal {
        ReceiptSeal {
            alg: self.alg().as_str().into(),
            key_id: self.key_id().into(),
            signature: self.sign_hex(payload),
            public_key: self.public_key_hex(),
            cosignatures: vec![],
        }
    }

    /// Add this key's cosignature to an existing seal (ed25519 only).
    pub fn cosign(&self, seal: &mut ReceiptSeal, payload: &[u8]) -> Result<()> {
        if self.alg() != SealAlg::Ed25519 || seal.alg != "ed25519" {
            bail!("cosign requires ed25519 local key and seal");
        }
        let kid = self.key_id();
        if seal.key_id == kid || seal.cosignatures.iter().any(|c| c.key_id == kid) {
            bail!("key_id {kid} already signed this receipt");
        }
        seal.cosignatures.push(Cosignature {
            key_id: kid.into(),
            signature: self.sign_hex(payload),
            public_key: self.public_key_hex(),
        });
        Ok(())
    }
}

/// Trusted ed25519 public keys (key_id -> 32-byte pubkey).
pub fn load_trusted_keys(prove_dir: &Path) -> Result<BTreeMap<String, VerifyingKey>> {
    let dir = LocalKey::trusted_dir(prove_dir);
    let mut map = BTreeMap::new();
    if !dir.exists() {
        return Ok(map);
    }
    for ent in std::fs::read_dir(&dir)? {
        let ent = ent?;
        let path = ent.path();
        if path.extension().and_then(|s| s.to_str()) != Some("pub") {
            continue;
        }
        let key_id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if key_id.is_empty() {
            continue;
        }
        let raw = std::fs::read_to_string(&path)?;
        let vk = parse_verifying_key(raw.trim())?;
        map.insert(key_id, vk);
    }
    Ok(map)
}

pub fn trust_key(prove_dir: &Path, key_id: &str, pubkey_hex: &str) -> Result<PathBuf> {
    if key_id.is_empty()
        || key_id.contains('/')
        || key_id.contains('\\')
        || key_id.contains("..")
    {
        bail!("invalid key_id");
    }
    let _ = parse_verifying_key(pubkey_hex)?;
    let dir = LocalKey::trusted_dir(prove_dir);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{key_id}.pub"));
    std::fs::write(&path, pubkey_hex.trim())?;
    Ok(path)
}

pub fn untrust_key(prove_dir: &Path, key_id: &str) -> Result<()> {
    let path = LocalKey::trusted_dir(prove_dir).join(format!("{key_id}.pub"));
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

pub fn list_trusted(prove_dir: &Path) -> Result<Vec<(String, String)>> {
    let map = load_trusted_keys(prove_dir)?;
    Ok(map
        .into_iter()
        .map(|(id, vk)| (id, hex::encode(vk.as_bytes())))
        .collect())
}

fn parse_verifying_key(pubkey_hex: &str) -> Result<VerifyingKey> {
    let bytes = hex::decode(pubkey_hex.trim()).map_err(|_| anyhow!("invalid public key hex"))?;
    let arr: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow!("ed25519 public key must be 32 bytes"))?;
    VerifyingKey::from_bytes(&arr).map_err(|e| anyhow!("invalid ed25519 public key: {e}"))
}

fn verify_ed25519_hex(vk: &VerifyingKey, payload: &[u8], sig_hex: &str) -> bool {
    let Ok(bytes) = hex::decode(sig_hex) else {
        return false;
    };
    let Ok(sig_arr) = <[u8; 64]>::try_from(bytes.as_slice()) else {
        return false;
    };
    let sig = Signature::from_bytes(&sig_arr);
    vk.verify(payload, &sig).is_ok()
}

/// Verify seal against local key and/or trusted public keys.
/// Returns number of unique valid signers.
pub fn count_valid_signers(
    prove_dir: &Path,
    seal: &ReceiptSeal,
    payload: &[u8],
) -> Result<usize> {
    let local = LocalKey::load(prove_dir)?;
    let trusted = load_trusted_keys(prove_dir)?;
    let mut valid: BTreeSet<String> = BTreeSet::new();

    for (kid, sig, pk_hex) in seal.all_signers() {
        let mut ok = false;
        if let Some(ref lk) = local {
            if lk.key_id() == kid && lk.verify_hex(payload, sig) {
                ok = true;
            }
        }
        if !ok {
            if let Some(vk) = trusted.get(kid) {
                if seal.alg == "ed25519" && verify_ed25519_hex(vk, payload, sig) {
                    ok = true;
                }
            }
        }
        // Also accept embedded public_key if it matches a trusted key or local
        if !ok && seal.alg == "ed25519" {
            if let Some(pk) = pk_hex {
                if let Ok(vk) = parse_verifying_key(pk) {
                    if verify_ed25519_hex(&vk, payload, sig) {
                        // only count if this pk is local or trusted
                        let pk_h = hex::encode(vk.as_bytes());
                        let local_pk = local.as_ref().and_then(|l| l.public_key_hex());
                        let trusted_hit = trusted.values().any(|t| hex::encode(t.as_bytes()) == pk_h);
                        let local_hit = local_pk.as_deref() == Some(pk_h.as_str());
                        if trusted_hit || local_hit {
                            ok = true;
                        }
                    }
                }
            }
        }
        if ok {
            valid.insert(kid.to_string());
        }
    }
    Ok(valid.len())
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
        if s.len() % 2 != 0 {
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
        assert!(key.verify_hex(&payload, &seal.signature));
    }

    #[test]
    fn cosign_and_quorum() {
        let dir = tempdir().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        std::fs::create_dir_all(a.join(".prove")).unwrap();
        std::fs::create_dir_all(b.join(".prove")).unwrap();
        let ka = LocalKey::init(&a.join(".prove"), SealAlg::Ed25519).unwrap();
        let kb = LocalKey::init(&b.join(".prove"), SealAlg::Ed25519).unwrap();
        // trust B's pubkey in A
        trust_key(
            &a.join(".prove"),
            kb.key_id(),
            &kb.public_key_hex().unwrap(),
        )
        .unwrap();
        let payload = sealing_payload(b"{\"q\":1}");
        let mut seal = ka.make_seal(&payload);
        kb.cosign(&mut seal, &payload).unwrap();
        let n = count_valid_signers(&a.join(".prove"), &seal, &payload).unwrap();
        assert_eq!(n, 2);
    }
}
