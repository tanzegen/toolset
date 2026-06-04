//! 哈希与 UUID：文本 MD5 / SHA1 / SHA256 / SHA512，UUID v4 生成。

use md5::Md5;
use serde::Serialize;
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha512};
use uuid::Uuid;

fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

#[derive(Serialize)]
pub struct HashResult {
    pub md5: String,
    pub sha1: String,
    pub sha256: String,
    pub sha512: String,
}

pub fn hash_text(input: String) -> HashResult {
    let data = input.as_bytes();
    HashResult {
        md5: to_hex(&Md5::digest(data)),
        sha1: to_hex(&Sha1::digest(data)),
        sha256: to_hex(&Sha256::digest(data)),
        sha512: to_hex(&Sha512::digest(data)),
    }
}

pub fn uuid_v4(count: u32) -> Vec<String> {
    let n = count.clamp(1, 100);
    (0..n).map(|_| Uuid::new_v4().to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vectors_abc() {
        let r = hash_text("abc".into());
        assert_eq!(r.md5, "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(r.sha1, "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(
            r.sha256,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn empty_string_md5() {
        let r = hash_text("".into());
        assert_eq!(r.md5, "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn uuid_count_and_format() {
        let ids = uuid_v4(5);
        assert_eq!(ids.len(), 5);
        assert_eq!(ids[0].len(), 36);
        assert_eq!(ids[0].chars().filter(|c| *c == '-').count(), 4);
        assert_eq!(ids[0].as_bytes()[14], b'4');
    }

    #[test]
    fn uuid_count_clamped() {
        assert_eq!(uuid_v4(0).len(), 1);
        assert_eq!(uuid_v4(999).len(), 100);
    }
}
