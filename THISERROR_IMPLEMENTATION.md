# SIP Caller Library - 完整的thiserror实现

## 概述

SIP Caller现在已完全重构为使用thiserror的库，不再进行二次封装。这提供了更直接、更高效的错误处理机制。

## 错误类型

### 1. SipError
主要的SIP相关错误：
```rust
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
```

### 2. RtpError
RTP流媒体相关错误：
```rust
#[derive(Error, Debug)]
pub enum RtpError {
    #[error("RTP error: {0}")]
    Rtp(String),
    
    #[error("Media error: {0}")]
    Media(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

### 3. ConfigError
配置相关错误：
```rust
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Invalid configuration: {0}")]
    Invalid(String),
    
    #[error("Missing required field: {0}")]
    Missing(String),
    
    #[error("Parse error: {0}")]
    Parse(String),
}
```

### 4. MediaPlayError
媒体播放相关错误：
```rust
#[derive(Error, Debug)]
pub enum MediaPlayError {
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
    
    #[error("Media error: {0}")]
    Media(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("RTP error: {0}")]
    Rtp(String),
    
    #[error("SDP error: {0}")]
    Sdp(String),
}
```

## 库API

### 核心功能

1. **SIP客户端创建**
```rust
pub async fn create_sip_client(server: &str, user: &str, password: &str) -> Result<SipClient, SipError>
```

2. **媒体播放器创建**
```rust
pub async fn create_audio_player(file_path: &str) -> Result<Box<dyn MediaPlayer>, MediaPlayError>
pub async fn create_video_player(file_path: &str) -> Result<Box<dyn MediaPlayer>, MediaPlayError>
```

3. **RTP会话创建**
```rust
pub async fn create_rtp_session(media_type: MediaKind) -> Result<(RtpPlayer, String), MediaPlayError>
```

### 使用示例

```rust
use sip_caller::*;

// 创建SIP客户端
let client = create_sip_client("127.0.0.1:5060", "user@example.com", "password").await?;

// 创建音频播放器
let audio_player = create_audio_player("audio.wav").await?;

// 创建视频播放器
let video_player = create_video_player("video.ivf").await?;

// 创建RTP会话
let (rtp_player, local_sdp) = create_rtp_session(MediaKind::Audio).await?;
```

## 优势

1. **直接使用thiserror**：不再进行二次封装，减少开销
2. **类型安全**：每个错误类型都有明确的语义
3. **自动转换**：实现了From trait，错误转换自动化
4. **详细的错误信息**：使用error!宏提供格式化的错误消息
5. **良好的IDE支持**：IDE可以准确解析和显示错误类型

## 项目结构

```
sip-caller/
├── src/
│   ├── lib.rs           # 库入口，定义公共API
│   ├── main.rs          # CLI应用程序
│   ├── error.rs         # 错误定义（使用thiserror）
│   ├── config.rs        # 配置结构
│   ├── rtp.rs          # RTP传输层
│   ├── rtp_play.rs      # RTP播放器（使用MediaPlayError）
│   ├── sip_client.rs    # SIP客户端
│   ├── sip_dialog.rs    # SIP对话框
│   ├── sip_transport.rs # SIP传输
│   └── utils.rs         # 工具函数
├── examples/
│   ├── rtp_play_example.rs
│   └── README_rtp_play.md
├── Cargo.toml           # 库和二进制配置
└── README.md
```

## 编译和测试

```bash
# 检查库
cargo check --lib

# 检查二进制
cargo check --bin sip-caller

# 运行测试
cargo test

# 构建发布版本
cargo build --release
```

## 外部使用

在另一个项目的Cargo.toml中添加：

```toml
[dependencies]
sip-caller = "0.1.0"
```

然后使用：

```rust
use sip_caller::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = create_sip_client("127.0.0.1:5060", "user", "pass").await?;
    // ...
    Ok(())
}
```

## 总结

通过直接使用thiserror，SIP Caller现在提供了：
- 更清晰的错误类型
- 更少的运行时开销
- 更好的IDE支持
- 更简单的错误处理代码