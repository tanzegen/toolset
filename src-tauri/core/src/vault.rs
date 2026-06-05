//! 凭据保险箱（vault）：只加密「密码类字段」（登录密码 / 私钥 PEM / 私钥口令）。
//!
//! 设计要点（详见 docs/ssh-terminal/spec.md §6）：
//! - 主密码经 Argon2id 派生 32 字节密钥；
//! - 每个密码类字段用 AES-256-GCM 独立加密，随机 12B nonce，
//!   AAD 绑定 `连接id:字段名`，防止把 A 连接的密文块挪用到 B；
//! - verifier 块加密一段固定明文，用于「主密码是否正确」的校验；
//! - 纯逻辑、无 I/O，可独立单测。忘记主密码不可找回（上层据此做「重置 vault」）。

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

/// verifier 固定明文：解密得到它即认为主密码正确。
const VERIFIER_PLAINTEXT: &[u8] = b"toolset-vault-v1";
/// verifier 块固定的 AAD。
const VERIFIER_AAD: &[u8] = b"verifier";

/// KDF 参数（随配置持久化，使导出文件可在别的机器用同一主密码解开）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KdfParams {
    pub algo: String, // 目前固定 "argon2id"
    pub salt: String, // base64
    #[serde(rename = "mCost")]
    pub m_cost: u32, // 内存开销（KiB）
    #[serde(rename = "tCost")]
    pub t_cost: u32, // 迭代次数
    #[serde(rename = "pCost")]
    pub p_cost: u32, // 并行度
}

impl KdfParams {
    /// 生成一份默认参数 + 随机 salt：m=64MiB、t=3、p=1（解锁一次性开销）。
    pub fn generate() -> Self {
        let mut salt = [0u8; 16];
        rand::rngs::OsRng.fill_bytes(&mut salt);
        KdfParams {
            algo: "argon2id".to_string(),
            salt: B64.encode(salt),
            m_cost: 65536,
            t_cost: 3,
            p_cost: 1,
        }
    }
}

/// 单个密文字段。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretBlob {
    pub v: u8,        // 结构版本
    pub nonce: String, // base64(12B)
    pub ct: String,    // base64(密文||tag)
}

/// 主密码校验块。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verifier {
    pub nonce: String,
    pub ct: String,
}

/// Argon2id 派生 32 字节密钥。
pub fn derive_key(master: &str, kdf: &KdfParams) -> AppResult<[u8; 32]> {
    if master.is_empty() {
        return Err(AppError::Invalid("主密码不能为空".to_string()));
    }
    let salt = B64
        .decode(kdf.salt.as_bytes())
        .map_err(|_| AppError::Invalid("KDF salt 非法".to_string()))?;
    let params = Params::new(kdf.m_cost, kdf.t_cost, kdf.p_cost, Some(32))
        .map_err(|e| AppError::Invalid(format!("KDF 参数非法：{e}")))?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; 32];
    argon
        .hash_password_into(master.as_bytes(), &salt, &mut key)
        .map_err(|e| AppError::Invalid(format!("派生密钥失败：{e}")))?;
    Ok(key)
}

fn cipher(key: &[u8; 32]) -> AppResult<Aes256Gcm> {
    Aes256Gcm::new_from_slice(key).map_err(|_| AppError::Invalid("密钥长度非法".to_string()))
}

fn random_nonce() -> [u8; 12] {
    let mut n = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut n);
    n
}

/// 加密一个密码类字段。`conn_id`/`field` 进 AAD，绑定密文归属。
pub fn encrypt_field(
    key: &[u8; 32],
    conn_id: &str,
    field: &str,
    plaintext: &str,
) -> AppResult<SecretBlob> {
    let nonce = random_nonce();
    let aad = format!("{conn_id}:{field}");
    let ct = cipher(key)?
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: plaintext.as_bytes(),
                aad: aad.as_bytes(),
            },
        )
        .map_err(|_| AppError::Invalid("加密失败".to_string()))?;
    Ok(SecretBlob {
        v: 1,
        nonce: B64.encode(nonce),
        ct: B64.encode(ct),
    })
}

/// 解密一个密码类字段。AAD 必须与加密时一致，否则失败（防挪用 / 错密码）。
pub fn decrypt_field(
    key: &[u8; 32],
    conn_id: &str,
    field: &str,
    blob: &SecretBlob,
) -> AppResult<String> {
    let nonce = B64
        .decode(blob.nonce.as_bytes())
        .map_err(|_| AppError::Invalid("nonce 非法".to_string()))?;
    let ct = B64
        .decode(blob.ct.as_bytes())
        .map_err(|_| AppError::Invalid("密文非法".to_string()))?;
    let aad = format!("{conn_id}:{field}");
    let pt = cipher(key)?
        .decrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &ct,
                aad: aad.as_bytes(),
            },
        )
        .map_err(|_| AppError::Invalid("解密失败（主密码错误或数据被篡改）".to_string()))?;
    String::from_utf8(pt).map_err(|_| AppError::Invalid("明文非合法 UTF-8".to_string()))
}

/// 生成主密码校验块。
pub fn make_verifier(key: &[u8; 32]) -> AppResult<Verifier> {
    let nonce = random_nonce();
    let ct = cipher(key)?
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: VERIFIER_PLAINTEXT,
                aad: VERIFIER_AAD,
            },
        )
        .map_err(|_| AppError::Invalid("生成校验块失败".to_string()))?;
    Ok(Verifier {
        nonce: B64.encode(nonce),
        ct: B64.encode(ct),
    })
}

/// 校验主密码：解密 verifier 并比对固定明文。
pub fn check_verifier(key: &[u8; 32], v: &Verifier) -> bool {
    let (Ok(nonce), Ok(ct)) = (B64.decode(v.nonce.as_bytes()), B64.decode(v.ct.as_bytes())) else {
        return false;
    };
    let Ok(c) = cipher(key) else { return false };
    match c.decrypt(
        Nonce::from_slice(&nonce),
        Payload {
            msg: &ct,
            aad: VERIFIER_AAD,
        },
    ) {
        Ok(pt) => pt == VERIFIER_PLAINTEXT,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> [u8; 32] {
        let kdf = KdfParams {
            algo: "argon2id".into(),
            // 固定 salt 让测试确定；实际使用为随机
            salt: B64.encode([7u8; 16]),
            m_cost: 19456,
            t_cost: 2,
            p_cost: 1,
        };
        derive_key("correct horse battery staple", &kdf).unwrap()
    }

    #[test]
    fn field_roundtrip() {
        let k = key();
        let blob = encrypt_field(&k, "conn-1", "password", "s3cr3t").unwrap();
        let got = decrypt_field(&k, "conn-1", "password", &blob).unwrap();
        assert_eq!(got, "s3cr3t");
    }

    #[test]
    fn aad_mismatch_fails() {
        let k = key();
        let blob = encrypt_field(&k, "conn-1", "password", "s3cr3t").unwrap();
        // 换连接 id（AAD 不符）必须解密失败，防止把 A 的密文挪到 B
        assert!(decrypt_field(&k, "conn-2", "password", &blob).is_err());
        // 换字段名同理
        assert!(decrypt_field(&k, "conn-1", "keyPass", &blob).is_err());
    }

    #[test]
    fn wrong_master_fails() {
        let k = key();
        let blob = encrypt_field(&k, "conn-1", "password", "s3cr3t").unwrap();
        let kdf = KdfParams {
            algo: "argon2id".into(),
            salt: B64.encode([7u8; 16]),
            m_cost: 19456,
            t_cost: 2,
            p_cost: 1,
        };
        let wrong = derive_key("wrong password", &kdf).unwrap();
        assert!(decrypt_field(&wrong, "conn-1", "password", &blob).is_err());
    }

    #[test]
    fn verifier_distinguishes_master() {
        let k = key();
        let v = make_verifier(&k).unwrap();
        assert!(check_verifier(&k, &v));
        let kdf = KdfParams {
            algo: "argon2id".into(),
            salt: B64.encode([7u8; 16]),
            m_cost: 19456,
            t_cost: 2,
            p_cost: 1,
        };
        let wrong = derive_key("nope", &kdf).unwrap();
        assert!(!check_verifier(&wrong, &v));
    }

    #[test]
    fn derive_key_is_deterministic() {
        // 同主密码 + 同参数 → 同密钥（导出文件跨机解密的基础）
        assert_eq!(key(), key());
    }
}
