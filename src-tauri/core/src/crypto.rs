//! 加密工具：对称 AES-256-GCM / AES-256-CBC / ChaCha20-Poly1305 + 非对称 RSA。
//! 对称密钥支持「口令派生(SHA-256)」与「原始 Hex/Base64(32 字节)」。
//! 对称输出 = Base64(nonce/iv ‖ 密文)；RSA 输出 = Base64(密文)。

use crate::error::{AppError, AppResult};
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use serde::Serialize;
use sha2::Sha256;

use aes_gcm::aead::{Aead, AeadCore, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce as GcmNonce};
use chacha20poly1305::{ChaCha20Poly1305, Nonce as ChaNonce};

use aes::Aes256;
use cbc::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};

use rand::rngs::OsRng;
use rand::RngCore;

use rsa::pkcs8::{
    DecodePrivateKey, DecodePublicKey, EncodePrivateKey, EncodePublicKey, LineEnding,
};
use rsa::{Oaep, RsaPrivateKey, RsaPublicKey};

type Aes256CbcEnc = cbc::Encryptor<Aes256>;
type Aes256CbcDec = cbc::Decryptor<Aes256>;

#[derive(Serialize)]
pub struct RsaKeypair {
    pub public_pem: String,
    pub private_pem: String,
}

// ---------------------------------------------------------------------------
// 对称密钥派生
// ---------------------------------------------------------------------------

fn derive_key(mode: &str, key: &str) -> AppResult<[u8; 32]> {
    let mut out = [0u8; 32];
    match mode {
        "passphrase" => {
            if key.is_empty() {
                return Err(AppError::Invalid("口令不能为空".to_string()));
            }
            // 口令原始字节作密钥：不足 32 字节用 \0 补齐、超过则截断（out 已零初始化）。
            let bytes = key.as_bytes();
            let n = bytes.len().min(32);
            out[..n].copy_from_slice(&bytes[..n]);
            Ok(out)
        }
        "hex" => {
            let b = hex::decode(key.trim())
                .map_err(|_| AppError::Invalid("Hex 密钥解析失败".to_string()))?;
            if b.len() != 32 {
                return Err(AppError::Invalid(format!(
                    "密钥需 32 字节（256 位），当前 {} 字节",
                    b.len()
                )));
            }
            out.copy_from_slice(&b);
            Ok(out)
        }
        "base64" => {
            let b = STANDARD
                .decode(key.trim())
                .map_err(|_| AppError::Invalid("Base64 密钥解析失败".to_string()))?;
            if b.len() != 32 {
                return Err(AppError::Invalid(format!(
                    "密钥需 32 字节（256 位），当前 {} 字节",
                    b.len()
                )));
            }
            out.copy_from_slice(&b);
            Ok(out)
        }
        other => Err(AppError::Invalid(format!("未知密钥模式: {other}"))),
    }
}

// ---------------------------------------------------------------------------
// 对称：AES-GCM / ChaCha20-Poly1305（AEAD，12 字节 nonce）
// ---------------------------------------------------------------------------

fn gcm_encrypt(key: &[u8; 32], pt: &[u8]) -> AppResult<String> {
    let cipher = Aes256Gcm::new_from_slice(key).expect("32B");
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ct = cipher
        .encrypt(&nonce, pt)
        .map_err(|_| AppError::Invalid("AES-GCM 加密失败".to_string()))?;
    let mut out = nonce.to_vec();
    out.extend_from_slice(&ct);
    Ok(STANDARD.encode(out))
}

fn gcm_decrypt(key: &[u8; 32], data: &[u8]) -> AppResult<Vec<u8>> {
    if data.len() < 12 {
        return Err(AppError::Invalid("密文过短".to_string()));
    }
    let (nonce, ct) = data.split_at(12);
    let cipher = Aes256Gcm::new_from_slice(key).expect("32B");
    cipher
        .decrypt(GcmNonce::from_slice(nonce), ct)
        .map_err(|_| AppError::Invalid("解密失败：密钥或密文不匹配".to_string()))
}

fn chacha_encrypt(key: &[u8; 32], pt: &[u8]) -> AppResult<String> {
    let cipher = ChaCha20Poly1305::new_from_slice(key).expect("32B");
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
    let ct = cipher
        .encrypt(&nonce, pt)
        .map_err(|_| AppError::Invalid("ChaCha20 加密失败".to_string()))?;
    let mut out = nonce.to_vec();
    out.extend_from_slice(&ct);
    Ok(STANDARD.encode(out))
}

fn chacha_decrypt(key: &[u8; 32], data: &[u8]) -> AppResult<Vec<u8>> {
    if data.len() < 12 {
        return Err(AppError::Invalid("密文过短".to_string()));
    }
    let (nonce, ct) = data.split_at(12);
    let cipher = ChaCha20Poly1305::new_from_slice(key).expect("32B");
    cipher
        .decrypt(ChaNonce::from_slice(nonce), ct)
        .map_err(|_| AppError::Invalid("解密失败：密钥或密文不匹配".to_string()))
}

// ---------------------------------------------------------------------------
// 对称：AES-256-CBC（PKCS7 填充，16 字节 IV，无认证）
// ---------------------------------------------------------------------------

fn cbc_encrypt(key: &[u8; 32], pt: &[u8]) -> AppResult<String> {
    let mut iv = [0u8; 16];
    let mut rng = OsRng;
    rng.fill_bytes(&mut iv);
    let enc =
        Aes256CbcEnc::new_from_slices(key, &iv).map_err(|_| AppError::Invalid("初始化失败".into()))?;
    let ct = enc.encrypt_padded_vec_mut::<Pkcs7>(pt);
    let mut out = iv.to_vec();
    out.extend_from_slice(&ct);
    Ok(STANDARD.encode(out))
}

fn cbc_decrypt(key: &[u8; 32], data: &[u8]) -> AppResult<Vec<u8>> {
    if data.len() < 16 {
        return Err(AppError::Invalid("密文过短".to_string()));
    }
    let (iv, ct) = data.split_at(16);
    let dec =
        Aes256CbcDec::new_from_slices(key, iv).map_err(|_| AppError::Invalid("初始化失败".into()))?;
    dec.decrypt_padded_vec_mut::<Pkcs7>(ct)
        .map_err(|_| AppError::Invalid("解密失败：密钥或密文不匹配".to_string()))
}

// ---------------------------------------------------------------------------
// 非对称：RSA（OAEP-SHA256）
// ---------------------------------------------------------------------------

pub fn rsa_generate(bits: u32) -> AppResult<RsaKeypair> {
    let bits = match bits {
        2048 | 3072 | 4096 => bits as usize,
        _ => 2048,
    };
    let mut rng = OsRng;
    let priv_key = RsaPrivateKey::new(&mut rng, bits)
        .map_err(|e| AppError::Invalid(format!("生成密钥对失败: {e}")))?;
    let pub_key = RsaPublicKey::from(&priv_key);
    let private_pem = priv_key
        .to_pkcs8_pem(LineEnding::LF)
        .map_err(|e| AppError::Invalid(e.to_string()))?
        .to_string();
    let public_pem = pub_key
        .to_public_key_pem(LineEnding::LF)
        .map_err(|e| AppError::Invalid(e.to_string()))?;
    Ok(RsaKeypair {
        public_pem,
        private_pem,
    })
}

fn rsa_encrypt(public_pem: &str, pt: &[u8]) -> AppResult<String> {
    let pub_key = RsaPublicKey::from_public_key_pem(public_pem.trim())
        .map_err(|e| AppError::Invalid(format!("公钥解析失败: {e}")))?;
    let mut rng = OsRng;
    let ct = pub_key
        .encrypt(&mut rng, Oaep::new::<Sha256>(), pt)
        .map_err(|e| AppError::Invalid(format!("RSA 加密失败: {e}（明文不能超过密钥容量）")))?;
    Ok(STANDARD.encode(ct))
}

fn rsa_decrypt(private_pem: &str, data: &[u8]) -> AppResult<Vec<u8>> {
    let priv_key = RsaPrivateKey::from_pkcs8_pem(private_pem.trim())
        .map_err(|e| AppError::Invalid(format!("私钥解析失败: {e}")))?;
    priv_key
        .decrypt(Oaep::new::<Sha256>(), data)
        .map_err(|e| AppError::Invalid(format!("RSA 解密失败: {e}")))
}

// ---------------------------------------------------------------------------
// 统一入口
// ---------------------------------------------------------------------------

/// algo: aes-gcm | aes-cbc | chacha20 | rsa
/// direction: encrypt | decrypt
/// 对称：key_mode = passphrase|hex|base64，key 为口令/密钥；RSA：key 为 PEM（加密用公钥、解密用私钥）。
pub fn crypto_process(
    algo: String,
    direction: String,
    key_mode: String,
    key: String,
    input: String,
) -> AppResult<String> {
    if input.is_empty() {
        return Err(AppError::Empty);
    }
    let encrypt = match direction.as_str() {
        "encrypt" => true,
        "decrypt" => false,
        _ => return Err(AppError::Invalid("未知方向".to_string())),
    };

    if algo == "rsa" {
        if encrypt {
            return rsa_encrypt(&key, input.as_bytes());
        }
        let data = STANDARD
            .decode(input.trim())
            .map_err(|_| AppError::Invalid("密文 Base64 解析失败".to_string()))?;
        let pt = rsa_decrypt(&key, &data)?;
        return String::from_utf8(pt)
            .map_err(|_| AppError::Invalid("解密结果不是合法 UTF-8 文本".to_string()));
    }

    // 对称
    let k = derive_key(&key_mode, &key)?;
    if encrypt {
        let pt = input.as_bytes();
        match algo.as_str() {
            "aes-gcm" => gcm_encrypt(&k, pt),
            "chacha20" => chacha_encrypt(&k, pt),
            "aes-cbc" => cbc_encrypt(&k, pt),
            other => Err(AppError::Invalid(format!("未知算法: {other}"))),
        }
    } else {
        let data = STANDARD
            .decode(input.trim())
            .map_err(|_| AppError::Invalid("密文 Base64 解析失败".to_string()))?;
        let pt = match algo.as_str() {
            "aes-gcm" => gcm_decrypt(&k, &data)?,
            "chacha20" => chacha_decrypt(&k, &data)?,
            "aes-cbc" => cbc_decrypt(&k, &data)?,
            other => return Err(AppError::Invalid(format!("未知算法: {other}"))),
        };
        String::from_utf8(pt).map_err(|_| AppError::Invalid("解密结果不是合法 UTF-8 文本".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(algo: &str, mode: &str, key: &str) {
        let ct = crypto_process(
            algo.into(),
            "encrypt".into(),
            mode.into(),
            key.into(),
            "Hello, 世界 🔐".into(),
        )
        .unwrap();
        let pt = crypto_process(algo.into(), "decrypt".into(), mode.into(), key.into(), ct).unwrap();
        assert_eq!(pt, "Hello, 世界 🔐");
    }

    #[test]
    fn aes_gcm_roundtrip_passphrase() {
        roundtrip("aes-gcm", "passphrase", "my secret pass");
    }

    #[test]
    fn chacha_roundtrip_passphrase() {
        roundtrip("chacha20", "passphrase", "my secret pass");
    }

    #[test]
    fn aes_cbc_roundtrip_passphrase() {
        roundtrip("aes-cbc", "passphrase", "my secret pass");
    }

    #[test]
    fn aes_gcm_roundtrip_hex_key() {
        let key = "0".repeat(64); // 32 字节 hex
        roundtrip("aes-gcm", "hex", &key);
    }

    #[test]
    fn wrong_passphrase_fails() {
        let ct = crypto_process(
            "aes-gcm".into(),
            "encrypt".into(),
            "passphrase".into(),
            "right".into(),
            "secret".into(),
        )
        .unwrap();
        let r = crypto_process(
            "aes-gcm".into(),
            "decrypt".into(),
            "passphrase".into(),
            "wrong".into(),
            ct,
        );
        assert!(r.is_err());
    }

    #[test]
    fn bad_hex_key_len_errors() {
        assert!(derive_key("hex", "abcd").is_err());
    }

    #[test]
    fn passphrase_key_zero_padded() {
        // 16 字节口令 → 原样 16 字节 + 16 个 \0；超过 32 截断
        let k = derive_key("passphrase", "5qEBzrSccgOyWCSk").unwrap();
        let mut expected = [0u8; 32];
        expected[..16].copy_from_slice(b"5qEBzrSccgOyWCSk");
        assert_eq!(k, expected);
        // 超过 32 字节截断
        assert_eq!(derive_key("passphrase", &"x".repeat(40)).unwrap(), [b'x'; 32]);
    }

    #[test]
    fn rsa_roundtrip() {
        let kp = rsa_generate(2048).unwrap();
        assert!(kp.public_pem.contains("BEGIN PUBLIC KEY"));
        assert!(kp.private_pem.contains("BEGIN PRIVATE KEY"));
        let ct = crypto_process(
            "rsa".into(),
            "encrypt".into(),
            String::new(),
            kp.public_pem.clone(),
            "rsa secret".into(),
        )
        .unwrap();
        let pt = crypto_process(
            "rsa".into(),
            "decrypt".into(),
            String::new(),
            kp.private_pem.clone(),
            ct,
        )
        .unwrap();
        assert_eq!(pt, "rsa secret");
    }
}
