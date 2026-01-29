use thiserror::Error;

#[derive(Error, Debug)]
pub enum SipError {
    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),

    #[error("SIP protocol error: {0}")]
    Protocol(String),

    #[error("Invalid URI: {0}")]
    InvalidUri(String),

    #[error("Registration failed: {0}")]
    RegistrationFailed(String),

    #[error("Call failed: {0}")]
    CallFailed(String),

    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Config error: {0}")]
    Config(#[from] ConfigError),

    #[error("Media error: {0}")]
    Media(String),

    #[error("Unknown error: {0}")]
    Other(String),
}

impl From<Box<dyn std::error::Error>> for SipError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        SipError::Other(err.to_string())
    }
}

#[derive(Error, Debug)]
pub enum RtpError {
    #[error("RTP error: {0}")]
    Rtp(String),

    #[error("Media error: {0}")]
    Media(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Invalid configuration: {0}")]
    Invalid(String),

    #[error("Missing required field: {0}")]
    Missing(String),

    #[error("Parse error: {0}")]
    Parse(String),
}

impl From<&str> for ConfigError {
    fn from(s: &str) -> Self {
        ConfigError::Parse(s.to_string())
    }
}

/// SIP呼叫操作的Result类型别名
pub type CallResult<T> = Result<T, CallError>;

/// SIP呼叫相关错误类型
#[derive(Error, Debug)]
pub enum CallError {
    /// SIP协议相关错误
    #[error("SIP协议错误")]
    SipProtocol(#[from] rsipstack::Error),

    /// URI解析错误
    #[error("URI解析错误")]
    UriParse(#[from] rsip::Error),

    /// 网络相关错误
    #[error("网络连接失败: {host}:{port}")]
    NetworkConnection { host: String, port: u16 },

    #[error("网络超时: {duration}ms")]
    NetworkTimeout { duration: u64 },

    /// 配置相关错误
    #[error("无效的SIP配置: {field}")]
    InvalidConfig { field: String },

    #[error("认证失败: {reason}")]
    AuthenticationFailed { reason: String },

    /// 呼叫相关错误
    #[error("呼叫目标无效: {target}")]
    InvalidTarget { target: String },

    #[error("SDP内容无效: {reason}")]
    InvalidSdp { reason: String },

    #[error("呼叫被拒绝: {code} {phrase}")]
    CallRejected { code: u16, phrase: String },

    /// 状态相关错误
    #[error("SIP客户端未初始化")]
    NotInitialized,

    #[error("SIP客户端未连接")]
    NotConnected,

    #[error("呼叫正在进行中")]
    CallInProgress,

    /// 系统错误
    #[error("系统错误: {0}")]
    System(#[from] std::io::Error),

    #[error("序列化错误: {0}")]
    Serialization(String),

    #[error("其他错误: {0}")]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl CallError {
    /// 判断错误是否可恢复（可用于重试逻辑）
    pub fn is_recoverable(&self) -> bool {
        match self {
            CallError::NetworkTimeout { .. } => true,
            CallError::NetworkConnection { .. } => true,
            CallError::SipProtocol(_) => false,
            CallError::UriParse(_) => false,
            CallError::CallRejected { .. } => false,
            CallError::AuthenticationFailed { .. } => true,
            CallError::NotInitialized => false,
            CallError::NotConnected => true,
            CallError::InvalidTarget { .. } => false,
            CallError::InvalidSdp { .. } => false,
            CallError::InvalidConfig { .. } => false,
            CallError::CallInProgress => false,
            CallError::System(_) => true,
            CallError::Serialization(_) => false,
            CallError::Other(_) => false,
        }
    }

    /// 获取标准错误代码，用于日志分析和监控
    pub fn error_code(&self) -> &'static str {
        match self {
            CallError::SipProtocol(_) => "SIP_PROTOCOL_ERROR",
            CallError::NetworkConnection { .. } => "NETWORK_CONNECTION_ERROR",
            CallError::NetworkTimeout { .. } => "NETWORK_TIMEOUT",
            CallError::InvalidTarget { .. } => "INVALID_TARGET",
            CallError::InvalidSdp { .. } => "INVALID_SDP",
            CallError::CallRejected { .. } => "CALL_REJECTED",
            CallError::NotInitialized => "NOT_INITIALIZED",
            CallError::NotConnected => "NOT_CONNECTED",
            CallError::CallInProgress => "CALL_IN_PROGRESS",
            CallError::AuthenticationFailed { .. } => "AUTHENTICATION_FAILED",
            CallError::InvalidConfig { .. } => "INVALID_CONFIG",
            CallError::System(_) => "SYSTEM_ERROR",
            CallError::Serialization(_) => "SERIALIZATION_ERROR",
            CallError::Other(_) => "UNKNOWN_ERROR",
            CallError::UriParse(_) => "URI_PARSE_ERROR",
        }
    }

    /// 获取SIP状态码（如果有）
    pub fn sip_status_code(&self) -> Option<u16> {
        match self {
            CallError::CallRejected { code, .. } => Some(*code),
            _ => None,
        }
    }

    /// 创建网络连接错误
    pub fn network_connection(host: impl Into<String>, port: u16) -> Self {
        CallError::NetworkConnection {
            host: host.into(),
            port,
        }
    }

    /// 创建网络超时错误
    pub fn network_timeout(duration_ms: u64) -> Self {
        CallError::NetworkTimeout {
            duration: duration_ms,
        }
    }

    /// 创建无效目标错误
    pub fn invalid_target(target: impl Into<String>) -> Self {
        CallError::InvalidTarget {
            target: target.into(),
        }
    }

    /// 创建SDP错误
    pub fn invalid_sdp(reason: impl Into<String>) -> Self {
        CallError::InvalidSdp {
            reason: reason.into(),
        }
    }

    /// 创建认证失败错误
    pub fn authentication_failed(reason: impl Into<String>) -> Self {
        CallError::AuthenticationFailed {
            reason: reason.into(),
        }
    }

    /// 创建配置错误
    pub fn invalid_config(field: impl Into<String>) -> Self {
        CallError::InvalidConfig {
            field: field.into(),
        }
    }

    /// 创建序列化错误
    pub fn serialization_error(msg: impl Into<String>) -> Self {
        CallError::Serialization(msg.into())
    }
}

// 为了方便转换，实现从常见错误类型的转换
impl From<tokio::time::error::Elapsed> for CallError {
    fn from(_: tokio::time::error::Elapsed) -> Self {
        CallError::NetworkTimeout {
            duration: 30000, // 默认30秒超时
        }
    }
}

impl From<uuid::Error> for CallError {
    fn from(err: uuid::Error) -> Self {
        CallError::Other(Box::new(err))
    }
}
