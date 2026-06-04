//! 子网计算器：解析 CIDR / IP+掩码，给出网络、广播、可用主机范围、掩码等。IPv4 完整，IPv6 基本。

use crate::error::{AppError, AppResult};
use serde::Serialize;
use std::net::{Ipv4Addr, Ipv6Addr};

#[derive(Serialize)]
pub struct SubnetResult {
    pub version: u8,
    pub cidr: String,
    pub address: String,
    pub network: String,
    pub broadcast: String,
    pub netmask: String,
    pub wildcard: String,
    pub prefix: u32,
    pub first_host: String,
    pub last_host: String,
    pub total: String,
    pub usable: String,
    pub network_int: String,
    pub ip_class: String,
    pub is_private: bool,
}

/// 把点分掩码（255.255.255.0）转成前缀长度，要求是连续的 1。
fn mask_to_prefix(mask: Ipv4Addr) -> AppResult<u32> {
    let bits = u32::from(mask);
    let ones = bits.count_ones();
    // 连续性校验：前 ones 位为 1，其余为 0
    let expected = if ones == 0 { 0 } else { u32::MAX << (32 - ones) };
    if bits != expected {
        return Err(AppError::Invalid("子网掩码不连续".to_string()));
    }
    Ok(ones)
}

fn ipv4_class(first_octet: u8) -> &'static str {
    match first_octet {
        0..=127 => "A",
        128..=191 => "B",
        192..=223 => "C",
        224..=239 => "D（组播）",
        _ => "E（保留）",
    }
}

fn calc_v4(ip: Ipv4Addr, prefix: u32) -> AppResult<SubnetResult> {
    if prefix > 32 {
        return Err(AppError::Invalid("IPv4 前缀须在 0–32".to_string()));
    }
    let ip_u = u32::from(ip);
    let mask: u32 = if prefix == 0 { 0 } else { u32::MAX << (32 - prefix) };
    let network = ip_u & mask;
    let broadcast = network | !mask;
    let total: u64 = 1u64 << (32 - prefix);

    let (first, last, usable) = if prefix <= 30 {
        (network + 1, broadcast - 1, total - 2)
    } else if prefix == 31 {
        (network, broadcast, 2) // RFC 3021 点对点
    } else {
        (network, network, 1) // /32 单地址
    };

    Ok(SubnetResult {
        version: 4,
        cidr: format!("{}/{}", Ipv4Addr::from(network), prefix),
        address: ip.to_string(),
        network: Ipv4Addr::from(network).to_string(),
        broadcast: Ipv4Addr::from(broadcast).to_string(),
        netmask: Ipv4Addr::from(mask).to_string(),
        wildcard: Ipv4Addr::from(!mask).to_string(),
        prefix,
        first_host: Ipv4Addr::from(first).to_string(),
        last_host: Ipv4Addr::from(last).to_string(),
        total: total.to_string(),
        usable: usable.to_string(),
        network_int: network.to_string(),
        ip_class: ipv4_class(ip.octets()[0]).to_string(),
        is_private: ip.is_private(),
    })
}

fn calc_v6(ip: Ipv6Addr, prefix: u32) -> AppResult<SubnetResult> {
    if prefix > 128 {
        return Err(AppError::Invalid("IPv6 前缀须在 0–128".to_string()));
    }
    let ip_u = u128::from(ip);
    let mask: u128 = if prefix == 0 {
        0
    } else {
        u128::MAX << (128 - prefix)
    };
    let network = ip_u & mask;
    let last = network | !mask;
    let shift = 128 - prefix;
    let total = if shift < 128 {
        (1u128 << shift).to_string()
    } else {
        "2^128".to_string()
    };
    let is_private = (ip.segments()[0] & 0xfe00) == 0xfc00; // fc00::/7 ULA

    Ok(SubnetResult {
        version: 6,
        cidr: format!("{}/{}", Ipv6Addr::from(network), prefix),
        address: ip.to_string(),
        network: Ipv6Addr::from(network).to_string(),
        broadcast: Ipv6Addr::from(last).to_string(), // v6 无广播，这里给最后地址
        netmask: Ipv6Addr::from(mask).to_string(),
        wildcard: "—".to_string(),
        prefix,
        first_host: Ipv6Addr::from(network).to_string(),
        last_host: Ipv6Addr::from(last).to_string(),
        total: total.clone(),
        usable: total,
        network_int: network.to_string(),
        ip_class: "—".to_string(),
        is_private,
    })
}

pub fn subnet_calc(input: String) -> AppResult<SubnetResult> {
    let s = input.trim();
    if s.is_empty() {
        return Err(AppError::Empty);
    }

    // 拆出 ip 部分与前缀/掩码部分
    let (ip_part, suffix): (&str, Option<&str>) = if let Some((a, b)) = s.split_once('/') {
        (a.trim(), Some(b.trim()))
    } else if let Some((a, b)) = s.split_once(char::is_whitespace) {
        (a.trim(), Some(b.trim()))
    } else {
        (s, None)
    };

    let is_v6 = ip_part.contains(':');

    if is_v6 {
        let ip: Ipv6Addr = ip_part
            .parse()
            .map_err(|_| AppError::Invalid("IPv6 地址无效".to_string()))?;
        let prefix = match suffix {
            Some(p) => p
                .parse::<u32>()
                .map_err(|_| AppError::Invalid("IPv6 前缀无效".to_string()))?,
            None => 128,
        };
        calc_v6(ip, prefix)
    } else {
        let ip: Ipv4Addr = ip_part
            .parse()
            .map_err(|_| AppError::Invalid("IPv4 地址无效".to_string()))?;
        let prefix = match suffix {
            Some(p) if p.contains('.') => {
                let mask: Ipv4Addr = p
                    .parse()
                    .map_err(|_| AppError::Invalid("子网掩码无效".to_string()))?;
                mask_to_prefix(mask)?
            }
            Some(p) => p
                .parse::<u32>()
                .map_err(|_| AppError::Invalid("IPv4 前缀无效".to_string()))?,
            None => 32,
        };
        calc_v4(ip, prefix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v4_slash24() {
        let r = subnet_calc("192.168.1.10/24".into()).unwrap();
        assert_eq!(r.network, "192.168.1.0");
        assert_eq!(r.broadcast, "192.168.1.255");
        assert_eq!(r.netmask, "255.255.255.0");
        assert_eq!(r.wildcard, "0.0.0.255");
        assert_eq!(r.first_host, "192.168.1.1");
        assert_eq!(r.last_host, "192.168.1.254");
        assert_eq!(r.usable, "254");
        assert_eq!(r.total, "256");
        assert_eq!(r.ip_class, "C");
        assert!(r.is_private);
    }

    #[test]
    fn v4_dotted_mask() {
        let r = subnet_calc("10.0.0.5 255.255.255.0".into()).unwrap();
        assert_eq!(r.prefix, 24);
        assert_eq!(r.network, "10.0.0.0");
    }

    #[test]
    fn v4_slash31_p2p() {
        let r = subnet_calc("192.168.0.0/31".into()).unwrap();
        assert_eq!(r.usable, "2");
        assert_eq!(r.first_host, "192.168.0.0");
        assert_eq!(r.last_host, "192.168.0.1");
    }

    #[test]
    fn noncontiguous_mask_errors() {
        assert!(subnet_calc("10.0.0.1 255.0.255.0".into()).is_err());
    }

    #[test]
    fn v6_slash64() {
        let r = subnet_calc("2001:db8::1/64".into()).unwrap();
        assert_eq!(r.version, 6);
        assert_eq!(r.network, "2001:db8::");
    }

    #[test]
    fn bad_ip_errors() {
        assert!(subnet_calc("999.1.1.1/24".into()).is_err());
    }
}
