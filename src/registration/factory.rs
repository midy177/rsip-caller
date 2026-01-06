/// 注册器工厂
///
/// 根据配置自动选择合适的注册器实现
use super::{
    outbound::OutboundProxyRegistrar,
    standard::StandardRegistrar,
    traits::*,
};
use rsipstack::transaction::endpoint::EndpointInnerRef;
use tracing::info;

/// 注册器类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrarType {
    /// 标准注册器
    Standard,

    /// Outbound 代理注册器（使用 realm 识别租户）
    OutboundProxy,
}

/// 注册器工厂配置
#[derive(Debug, Clone)]
pub struct RegistrarFactoryConfig {
    /// 注册器类型
    pub registrar_type: RegistrarType,

    /// 租户域名（仅 OutboundProxy 需要）
    pub tenant_domain: Option<String>,

    /// 代理服务器地址（仅 OutboundProxy 需要）
    pub proxy_addr: Option<String>,

    /// 基础配置
    pub base_config: RegistrationConfig,
}

impl RegistrarFactoryConfig {
    /// 创建标准注册器配置
    pub fn standard(base_config: RegistrationConfig) -> Self {
        Self {
            registrar_type: RegistrarType::Standard,
            tenant_domain: None,
            proxy_addr: None,
            base_config,
        }
    }

    /// 创建 Outbound 代理注册器配置
    ///
    /// # 参数
    /// - `base_config`: 基础注册配置
    /// - `tenant_domain`: 租户域名（用于From/To和realm）
    /// - `proxy_addr`: 代理服务器地址（物理连接地址）
    pub fn outbound_proxy(
        base_config: RegistrationConfig,
        tenant_domain: String,
        proxy_addr: String,
    ) -> Self {
        Self {
            registrar_type: RegistrarType::OutboundProxy,
            tenant_domain: Some(tenant_domain),
            proxy_addr: Some(proxy_addr),
            base_config,
        }
    }
}

/// 注册器工厂
pub struct RegistrarFactory;

impl RegistrarFactory {
    /// 创建注册器
    ///
    /// # 参数
    /// - `endpoint`: SIP Endpoint Inner
    /// - `config`: 工厂配置
    ///
    /// # 返回
    /// - 实现了 Registrar trait 的注册器实例
    pub fn create(
        endpoint: EndpointInnerRef,
        config: RegistrarFactoryConfig,
    ) -> Box<dyn Registrar> {
        info!("创建注册器: 类型={:?}", config.registrar_type);

        match config.registrar_type {
            RegistrarType::Standard => {
                info!("使用标准注册器");
                Box::new(StandardRegistrar::new(endpoint, config.base_config))
            }
            RegistrarType::OutboundProxy => {
                let tenant_domain = config
                    .tenant_domain
                    .expect("Outbound proxy registrar requires tenant_domain");

                let proxy_addr = config
                    .proxy_addr
                    .expect("Outbound proxy registrar requires proxy_addr");

                info!("使用 Outbound 代理注册器，租户: {}, 代理: {}", tenant_domain, proxy_addr);

                Box::new(OutboundProxyRegistrar::new(
                    endpoint,
                    config.base_config,
                    tenant_domain,
                    proxy_addr,
                ))
            }
        }
    }

    /// 根据服务器地址自动选择注册器类型
    ///
    /// # 逻辑
    /// - 如果有 outbound_proxy 且 server 不是有效域名/IP -> OutboundProxy
    /// - 否则 -> Standard
    pub fn auto_detect(
        server_host: &str,
        has_outbound_proxy: bool,
    ) -> RegistrarType {
        if has_outbound_proxy {
            // 检查 server_host 是否是租户ID（不是域名也不是IP）
            let is_tenant_id = !server_host.contains('.')
                && server_host.parse::<std::net::IpAddr>().is_err();

            if is_tenant_id {
                info!(
                    "自动检测: {} 是租户ID，使用 Outbound 代理注册器",
                    server_host
                );
                return RegistrarType::OutboundProxy;
            }
        }

        info!("自动检测: 使用标准注册器");
        RegistrarType::Standard
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_detect_tenant_id() {
        // 租户ID场景 - 使用 OutboundProxy
        assert_eq!(
            RegistrarFactory::auto_detect("xfc", true),
            RegistrarType::OutboundProxy
        );

        assert_eq!(
            RegistrarFactory::auto_detect("tenant123", true),
            RegistrarType::OutboundProxy
        );

        // 标准场景
        assert_eq!(
            RegistrarFactory::auto_detect("sip.example.com", true),
            RegistrarType::Standard
        );

        assert_eq!(
            RegistrarFactory::auto_detect("192.168.1.100", true),
            RegistrarType::Standard
        );

        // 无代理场景
        assert_eq!(
            RegistrarFactory::auto_detect("xfc", false),
            RegistrarType::Standard
        );
    }
}
