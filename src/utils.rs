/// SIP 工具函数模块
///
/// 提供自定义的 SIP 相关辅助函数，用于覆盖 rsipstack 的默认行为

use std::net::IpAddr;
use uuid::Uuid;

/// 获取第一个非回环的网络接口 IP 地址
///
/// 遍历系统所有网络接口，返回第一个非回环的 IPv4 地址
///
/// # 返回
/// - `Ok(IpAddr)` - 成功找到的 IPv4 地址
/// - `Err` - 未找到可用的 IPv4 接口
///
/// # 示例
/// ```rust,no_run
/// use sip_caller::utils::get_first_non_loopback_interface;
///
/// let local_ip = get_first_non_loopback_interface().unwrap();
/// println!("本地IP: {}", local_ip);
/// ```
pub fn get_first_non_loopback_interface() -> Result<IpAddr, Box<dyn std::error::Error>> {
    for interface in get_if_addrs::get_if_addrs()? {
        if !interface.is_loopback() {
            match interface.addr {
                get_if_addrs::IfAddr::V4(ref addr) => return Ok(IpAddr::V4(addr.ip)),
                _ => continue,
            }
        }
    }
    Err("未找到 IPv4 接口".into())
}

/// 生成基于 UUID 的 Call-ID
///
/// 这个函数替代了 rsipstack 默认的 `make_call_id` 函数，
/// 使用 UUID v4 代替随机文本，确保全局唯一性
///
/// # 参数
/// * `domain` - 可选的域名后缀
///
/// # 示例
/// ```rust
/// let call_id = make_call_id(Some("example.com"));
/// // 生成类似: "550e8400-e29b-41d4-a716-446655440000@example.com"
///
/// let call_id = make_call_id(None);
/// // 生成类似: "550e8400-e29b-41d4-a716-446655440000"
/// ```
pub fn make_call_id(domain: Option<&str>) -> rsip::headers::CallId {
    let uuid = Uuid::new_v4();

    match domain {
        Some(d) => format!("{}@{}", uuid, d).into(),
        None => uuid.to_string().into(),
    }
}

/// 生成带主机名的 Call-ID
///
/// # 参数
/// * `hostname` - 主机名或域名
#[allow(dead_code)]
pub fn make_call_id_with_host(hostname: &str) -> rsip::headers::CallId {
    make_call_id(Some(hostname))
}

/// 生成纯 UUID Call-ID（无域名后缀）
#[allow(dead_code)]
pub fn make_uuid_call_id() -> rsip::headers::CallId {
    Uuid::new_v4().to_string().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_call_id_with_domain() {
        let call_id = make_call_id(Some("example.com"));
        let call_id_str = call_id.to_string();

        assert!(call_id_str.contains("@example.com"));
        assert!(call_id_str.len() > 36); // UUID 长度 + @ + domain
    }

    #[test]
    fn test_make_call_id_without_domain() {
        let call_id = make_call_id(None);
        let call_id_str = call_id.to_string();

        // UUID v4 格式: 8-4-4-4-12
        assert_eq!(call_id_str.len(), 36);
        assert!(!call_id_str.contains("@"));
    }

    #[test]
    fn test_make_uuid_call_id() {
        let call_id1 = make_uuid_call_id();
        let call_id2 = make_uuid_call_id();

        // 两次生成的 Call-ID 应该不同
        assert_ne!(call_id1.to_string(), call_id2.to_string());
    }

    #[test]
    fn test_make_call_id_uniqueness() {
        let mut call_ids = std::collections::HashSet::new();

        for _ in 0..1000 {
            let call_id = make_call_id(Some("test.com"));
            call_ids.insert(call_id.to_string());
        }

        // 1000 个 Call-ID 应该都是唯一的
        assert_eq!(call_ids.len(), 1000);
    }
}
