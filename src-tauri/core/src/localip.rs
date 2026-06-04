//! 内网 IP：枚举本机网卡地址（纯本地，不联网）。
//! 主用出口地址用「UDP connect 不发包」技巧获取——connect 只让 OS 按路由选源地址，不发数据。

use crate::error::AppResult;
use serde::Serialize;
use std::net::UdpSocket;

#[derive(Serialize)]
pub struct IfaceAddr {
    pub name: String,
    pub ip: String,
    pub version: u8,
    pub is_loopback: bool,
}

#[derive(Serialize)]
pub struct LocalIpResult {
    /// 主用出口内网 IPv4（OS 默认路由选中的源地址）
    pub primary: Option<String>,
    pub interfaces: Vec<IfaceAddr>,
}

fn primary_ip() -> Option<String> {
    let sock = UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.connect("8.8.8.8:80").ok()?; // 不会真正发包
    sock.local_addr().ok().map(|a| a.ip().to_string())
}

pub fn local_ips() -> AppResult<LocalIpResult> {
    let mut interfaces = Vec::new();
    if let Ok(ifaces) = if_addrs::get_if_addrs() {
        for i in ifaces {
            let ip = i.ip();
            interfaces.push(IfaceAddr {
                name: i.name.clone(),
                ip: ip.to_string(),
                version: if ip.is_ipv4() { 4 } else { 6 },
                is_loopback: i.is_loopback(),
            });
        }
    }
    // 非环回在前、IPv4 在前
    interfaces.sort_by_key(|x| (x.is_loopback, x.version));
    Ok(LocalIpResult {
        primary: primary_ip(),
        interfaces,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_some_interface() {
        let r = local_ips().unwrap();
        // 任何机器至少有环回地址
        assert!(!r.interfaces.is_empty());
    }
}
