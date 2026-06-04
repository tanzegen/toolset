//! 随机密码生成：字符集 / 长度 / 数量 / 必含 / 排除 / 默认排除易混淆。
//! 使用 getrandom（OS CSPRNG）+ 拒绝采样，保证无偏。

use crate::error::{AppError, AppResult};
use serde::Serialize;
use std::collections::HashSet;

const DIGITS: &str = "0123456789";
const LOWER: &str = "abcdefghijklmnopqrstuvwxyz";
const UPPER: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const SYMBOLS: &str = "!@#$%^&*()-_=+[]{};:,.?/";
/// 易混淆字符（默认排除）：0 O o 1 l L i I
const CONFUSABLE: &str = "0Oo1lLiI";

#[derive(Serialize)]
pub struct PasswordResult {
    pub passwords: Vec<String>,
    pub pool_size: usize,
}

/// 无偏地返回 [0, n) 内的随机数（拒绝采样）。
fn rand_below(n: usize) -> usize {
    if n <= 1 {
        return 0;
    }
    let n64 = n as u64;
    let zone = (u64::MAX / n64) * n64;
    loop {
        let mut buf = [0u8; 8];
        getrandom::getrandom(&mut buf).expect("OS RNG 失败");
        let v = u64::from_le_bytes(buf);
        if v < zone {
            return (v % n64) as usize;
        }
    }
}

fn shuffle(v: &mut [char]) {
    for i in (1..v.len()).rev() {
        let j = rand_below(i + 1);
        v.swap(i, j);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn generate_password(
    digits: bool,
    lower: bool,
    upper: bool,
    symbols: bool,
    length: usize,
    count: usize,
    must_include: String,
    exclude: String,
    exclude_confusable: bool,
) -> AppResult<PasswordResult> {
    let length = length.clamp(1, 256);
    let count = count.clamp(1, 100);

    // 排除集
    let mut excl: HashSet<char> = exclude.chars().filter(|c| !c.is_whitespace()).collect();
    if exclude_confusable {
        excl.extend(CONFUSABLE.chars());
    }

    // 必含字符（去重、去空白）；这些字符始终允许（从排除集移除）
    let mut seen = HashSet::new();
    let must: Vec<char> = must_include
        .chars()
        .filter(|c| !c.is_whitespace() && seen.insert(*c))
        .collect();
    for c in &must {
        excl.remove(c);
    }

    // 各选中类别，去除排除集
    let build = |s: &str| -> Vec<char> { s.chars().filter(|c| !excl.contains(c)).collect() };
    let mut classes: Vec<Vec<char>> = Vec::new();
    if digits {
        classes.push(build(DIGITS));
    }
    if lower {
        classes.push(build(LOWER));
    }
    if upper {
        classes.push(build(UPPER));
    }
    if symbols {
        classes.push(build(SYMBOLS));
    }

    // 总字符池 = 各类别 ∪ 必含，去重
    let mut pool: Vec<char> = Vec::new();
    for cls in &classes {
        pool.extend(cls.iter().copied());
    }
    pool.extend(must.iter().copied());
    let mut seen2 = HashSet::new();
    pool.retain(|c| seen2.insert(*c));

    if pool.is_empty() {
        return Err(AppError::Invalid(
            "可用字符集为空：请至少选择一个字符集，或减少排除".to_string(),
        ));
    }

    // 必需槽位：每个非空类别 ≥1 + 每个必含字符 ≥1
    let non_empty: Vec<&Vec<char>> = classes.iter().filter(|c| !c.is_empty()).collect();
    let required = non_empty.len() + must.len();
    if required > length {
        return Err(AppError::Invalid(format!(
            "长度 {length} 不足以容纳必需字符（至少需要 {required} 位）"
        )));
    }

    let pool_size = pool.len();
    let mut passwords = Vec::with_capacity(count);
    for _ in 0..count {
        let mut chars: Vec<char> = Vec::with_capacity(length);
        for cls in &non_empty {
            chars.push(cls[rand_below(cls.len())]);
        }
        for c in &must {
            chars.push(*c);
        }
        while chars.len() < length {
            chars.push(pool[rand_below(pool_size)]);
        }
        shuffle(&mut chars);
        passwords.push(chars.into_iter().collect());
    }

    Ok(PasswordResult {
        passwords,
        pool_size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gen(d: bool, lo: bool, up: bool, sy: bool, len: usize) -> PasswordResult {
        generate_password(d, lo, up, sy, len, 5, String::new(), String::new(), true).unwrap()
    }

    #[test]
    fn length_and_count() {
        let r = gen(true, true, true, false, 20);
        assert_eq!(r.passwords.len(), 5);
        assert!(r.passwords.iter().all(|p| p.chars().count() == 20));
    }

    #[test]
    fn digits_only() {
        let r = gen(true, false, false, false, 12);
        assert!(r.passwords.iter().all(|p| p.chars().all(|c| c.is_ascii_digit())));
    }

    #[test]
    fn excludes_confusable_by_default() {
        let r = gen(true, true, true, false, 30);
        assert!(r
            .passwords
            .iter()
            .all(|p| p.chars().all(|c| !CONFUSABLE.contains(c))));
    }

    #[test]
    fn must_include_present() {
        let r = generate_password(
            true, true, true, true, 16, 8, "@#".into(), String::new(), true,
        )
        .unwrap();
        assert!(r.passwords.iter().all(|p| p.contains('@') && p.contains('#')));
    }

    #[test]
    fn exclude_chars_absent() {
        let r = generate_password(
            true, false, false, false, 16, 5, String::new(), "357".into(), true,
        )
        .unwrap();
        assert!(r
            .passwords
            .iter()
            .all(|p| p.chars().all(|c| c != '3' && c != '5' && c != '7')));
    }

    #[test]
    fn empty_pool_errors() {
        // 只选数字，却把所有数字都排除
        let r = generate_password(
            true, false, false, false, 8, 1, String::new(), "0123456789".into(), false,
        );
        assert!(r.is_err());
    }

    #[test]
    fn length_too_short_errors() {
        // 4 个类别 → 至少 4 位，长度 3 不够
        let r = generate_password(true, true, true, true, 3, 1, String::new(), String::new(), true);
        assert!(r.is_err());
    }
}
