//! 数值工具：Hex↔Float（IEEE 754 位模式）与进制转换。

use crate::error::{AppError, AppResult};
use serde::Serialize;
use std::num::FpCategory;

// ---------------------------------------------------------------------------
// Hex <-> Float
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct FloatResult {
    pub width: u32,
    pub float_value: String,
    pub hex: String,
    pub binary: String,
    pub sign: u8,
    pub exponent_raw: u32,
    pub exponent_unbiased: i32,
    pub mantissa_hex: String,
    pub category: String,
    pub int_unsigned: String,
    pub int_signed: String,
}

fn clean_hex(s: &str) -> AppResult<String> {
    let t = s.trim();
    let t = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")).unwrap_or(t);
    let cleaned: String = t
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '_')
        .collect();
    if cleaned.is_empty() {
        return Err(AppError::Empty);
    }
    if !cleaned.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(AppError::Invalid("包含非十六进制字符".to_string()));
    }
    Ok(cleaned)
}

/// 把二进制串按 `sizes` 分组，用空格分隔（如 [1,8,23] -> "s eeeeeeee mmm..."）。
fn group_bits(bits: &str, sizes: &[usize]) -> String {
    let mut out = Vec::new();
    let mut i = 0;
    for &n in sizes {
        out.push(&bits[i..i + n]);
        i += n;
    }
    out.join(" ")
}

fn category_label(c: FpCategory) -> &'static str {
    match c {
        FpCategory::Nan => "NaN",
        FpCategory::Infinite => "无穷",
        FpCategory::Zero => "零",
        FpCategory::Subnormal => "非正规",
        FpCategory::Normal => "正规",
    }
}

fn f32_result(bits: u32) -> FloatResult {
    let v = f32::from_bits(bits);
    let sign = (bits >> 31) & 1;
    let exp_raw = (bits >> 23) & 0xFF;
    let mantissa = bits & 0x7F_FFFF;
    FloatResult {
        width: 32,
        float_value: format!("{v}"),
        hex: format!("0x{bits:08X}"),
        binary: group_bits(&format!("{bits:032b}"), &[1, 8, 23]),
        sign: sign as u8,
        exponent_raw: exp_raw,
        exponent_unbiased: exp_raw as i32 - 127,
        mantissa_hex: format!("0x{mantissa:06X}"),
        category: category_label(v.classify()).to_string(),
        int_unsigned: bits.to_string(),
        int_signed: (bits as i32).to_string(),
    }
}

fn f64_result(bits: u64) -> FloatResult {
    let v = f64::from_bits(bits);
    let sign = (bits >> 63) & 1;
    let exp_raw = ((bits >> 52) & 0x7FF) as u32;
    let mantissa = bits & 0x000F_FFFF_FFFF_FFFF;
    FloatResult {
        width: 64,
        float_value: format!("{v}"),
        hex: format!("0x{bits:016X}"),
        binary: group_bits(&format!("{bits:064b}"), &[1, 11, 52]),
        sign: sign as u8,
        exponent_raw: exp_raw,
        exponent_unbiased: exp_raw as i32 - 1023,
        mantissa_hex: format!("0x{mantissa:013X}"),
        category: category_label(v.classify()).to_string(),
        int_unsigned: bits.to_string(),
        int_signed: (bits as i64).to_string(),
    }
}

pub fn hex_to_float(hex: String, width: String) -> AppResult<FloatResult> {
    let cleaned = clean_hex(&hex)?;
    let w = match width.as_str() {
        "32" => 32,
        "64" => 64,
        _ => {
            if cleaned.len() <= 8 {
                32
            } else {
                64
            }
        }
    };
    if w == 32 {
        if cleaned.len() > 8 {
            return Err(AppError::Invalid("32 位最多 8 个十六进制位".to_string()));
        }
        let bits = u32::from_str_radix(&cleaned, 16)
            .map_err(|_| AppError::Invalid("十六进制解析失败".to_string()))?;
        Ok(f32_result(bits))
    } else {
        if cleaned.len() > 16 {
            return Err(AppError::Invalid("64 位最多 16 个十六进制位".to_string()));
        }
        let bits = u64::from_str_radix(&cleaned, 16)
            .map_err(|_| AppError::Invalid("十六进制解析失败".to_string()))?;
        Ok(f64_result(bits))
    }
}

pub fn float_to_hex(value: String, width: String) -> AppResult<FloatResult> {
    let t = value.trim();
    if t.is_empty() {
        return Err(AppError::Empty);
    }
    if width.as_str() == "32" {
        let v: f32 = t
            .parse()
            .map_err(|_| AppError::Invalid("不是合法的浮点数".to_string()))?;
        Ok(f32_result(v.to_bits()))
    } else {
        let v: f64 = t
            .parse()
            .map_err(|_| AppError::Invalid("不是合法的浮点数".to_string()))?;
        Ok(f64_result(v.to_bits()))
    }
}

// ---------------------------------------------------------------------------
// 进制转换
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct BaseResult {
    pub bin: String,
    pub oct: String,
    pub dec_unsigned: String,
    pub dec_signed: String,
    pub hex: String,
    pub bit_width: u32,
    pub bits_grouped: String,
}

fn modulo_of(bw: u32) -> u128 {
    if bw >= 64 {
        1u128 << 64
    } else {
        1u128 << bw
    }
}

fn parse_to_raw(input: &str, from_base: u32, bw: u32) -> AppResult<u64> {
    let s: String = input
        .trim()
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '_' && *c != ',')
        .collect();
    if s.is_empty() {
        return Err(AppError::Empty);
    }
    let modulo = modulo_of(bw);

    if from_base == 10 {
        if let Some(mag) = s.strip_prefix('-') {
            let m = u128::from_str_radix(mag, 10)
                .map_err(|_| AppError::Invalid("十进制解析失败".to_string()))?;
            let signed_max = if bw >= 64 { 1u128 << 63 } else { 1u128 << (bw - 1) };
            if m > signed_max {
                return Err(AppError::Invalid("超出该位宽有符号范围".to_string()));
            }
            return Ok(((modulo - m) % modulo) as u64);
        }
    }

    let body = match from_base {
        16 => s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(&s),
        2 => s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")).unwrap_or(&s),
        8 => s.strip_prefix("0o").or_else(|| s.strip_prefix("0O")).unwrap_or(&s),
        _ => s.as_str(),
    };
    let v = u128::from_str_radix(body, from_base)
        .map_err(|_| AppError::Invalid(format!("{from_base} 进制解析失败")))?;
    if v >= modulo {
        return Err(AppError::Invalid("超出该位宽范围".to_string()));
    }
    Ok(v as u64)
}

fn to_signed(masked: u64, bw: u32) -> i64 {
    if bw >= 64 {
        masked as i64
    } else {
        let sign_bit = 1u64 << (bw - 1);
        if masked & sign_bit != 0 {
            (masked as i128 - (1i128 << bw)) as i64
        } else {
            masked as i64
        }
    }
}

fn group_nibbles(bits: &str) -> String {
    let rev: Vec<char> = bits.chars().rev().collect();
    let mut groups: Vec<String> = rev
        .chunks(4)
        .map(|c| c.iter().rev().collect::<String>())
        .collect();
    groups.reverse();
    groups.join(" ")
}

pub fn base_convert(input: String, from_base: u32, bit_width: u32) -> AppResult<BaseResult> {
    let bw = match bit_width {
        8 | 16 | 32 | 64 => bit_width,
        _ => 64,
    };
    let raw = parse_to_raw(&input, from_base, bw)?;
    let masked = if bw >= 64 { raw } else { raw & ((1u64 << bw) - 1) };
    Ok(BaseResult {
        bin: format!("{masked:b}"),
        oct: format!("{masked:o}"),
        dec_unsigned: masked.to_string(),
        dec_signed: to_signed(masked, bw).to_string(),
        hex: format!("{masked:X}"),
        bit_width: bw,
        bits_grouped: group_nibbles(&format!("{masked:0width$b}", width = bw as usize)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_to_f32_pi() {
        let r = hex_to_float("0x40490FDB".into(), "auto".into()).unwrap();
        assert_eq!(r.width, 32);
        assert!(r.float_value.starts_with("3.14159"), "got {}", r.float_value);
        assert_eq!(r.sign, 0);
        assert_eq!(r.exponent_unbiased, 1);
    }

    #[test]
    fn hex_to_f64_pi() {
        let r = hex_to_float("400921FB54442D18".into(), "auto".into()).unwrap();
        assert_eq!(r.width, 64);
        assert!(r.float_value.starts_with("3.14159265358979"), "got {}", r.float_value);
    }

    #[test]
    fn float_to_hex_roundtrip_f32() {
        let r = float_to_hex("3.1415927".into(), "32".into()).unwrap();
        assert_eq!(r.hex, "0x40490FDB");
    }

    #[test]
    fn float_to_hex_roundtrip_f64() {
        let r = float_to_hex("3.141592653589793".into(), "64".into()).unwrap();
        assert_eq!(r.hex, "0x400921FB54442D18");
    }

    #[test]
    fn base_dec_to_all() {
        let r = base_convert("255".into(), 10, 8).unwrap();
        assert_eq!(r.bin, "11111111");
        assert_eq!(r.oct, "377");
        assert_eq!(r.hex, "FF");
        assert_eq!(r.dec_unsigned, "255");
        assert_eq!(r.dec_signed, "-1"); // 8 位有符号
    }

    #[test]
    fn base_negative_twos_complement() {
        let r = base_convert("-1".into(), 10, 8).unwrap();
        assert_eq!(r.hex, "FF");
        assert_eq!(r.dec_unsigned, "255");
        assert_eq!(r.dec_signed, "-1");
    }

    #[test]
    fn base_hex_input() {
        let r = base_convert("0xFF".into(), 16, 16).unwrap();
        assert_eq!(r.dec_unsigned, "255");
        assert_eq!(r.dec_signed, "255");
        assert_eq!(r.bin, "11111111");
    }

    #[test]
    fn base_overflow_errors() {
        assert!(base_convert("256".into(), 10, 8).is_err());
    }
}
