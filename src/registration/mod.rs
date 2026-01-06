/// Registration 模块
///
/// 提供灵活的 SIP 注册实现
///
/// ## 设计模式
///
/// 使用 **策略模式 (Strategy Pattern)** 和 **工厂模式 (Factory Pattern)**：
///
/// - `Registrar` trait: 定义注册行为接口
/// - `StandardRegistrar`: 标准注册实现
/// - `OutboundProxyRegistrar`: Outbound 代理注册实现
/// - `RegistrarFactory`: 工厂创建注册器
///
/// ## 使用示例
///
/// ### 标准注册
///
/// ```rust,no_run
/// use sip_caller::registration::*;
///
/// let config = RegistrationConfig::new("alice".into(), "password".into());
/// let factory_config = RegistrarFactoryConfig::standard(config);
/// let mut registrar = RegistrarFactory::create(endpoint, factory_config);
///
/// registrar.register(server_uri, Some(3600)).await?;
/// ```
///
/// ### Outbound 代理注册
///
/// ```rust,no_run
/// use sip_caller::registration::*;
///
/// let config = RegistrationConfig::new("1001".into(), "password".into())
///     .with_realm("xfc".into());
///
/// let factory_config = RegistrarFactoryConfig::outbound_proxy(
///     config,
///     "xfc".into(),  // 租户域名
/// );
///
/// let mut registrar = RegistrarFactory::create(endpoint, factory_config);
/// registrar.register(proxy_uri, Some(3600)).await?;
/// ```
///
/// ### 自动检测
///
/// ```rust,no_run
/// use sip_caller::registration::*;
///
/// let registrar_type = RegistrarFactory::auto_detect("xfc", true);
/// // 返回 RegistrarType::OutboundProxy
/// ```

mod factory;
mod outbound;
mod standard;
mod traits;

// 导出公共接口
pub use factory::{RegistrarFactory, RegistrarFactoryConfig, RegistrarType};
pub use outbound::OutboundProxyRegistrar;
pub use standard::StandardRegistrar;
pub use traits::{RegistrationConfig, RegistrationResult, Registrar};
