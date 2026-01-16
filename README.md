# SIP Caller

基于 Rust 和 rsipstack 的 SIP 客户端库，支持 SIP 服务器注册、INVITE 呼叫、200 OK 响应处理和 RTP 音频流建立。

## 特性

- ✅ SIP 服务器注册
- ✅ INVITE 呼叫发起
- ✅ 200 OK 响应处理
- ✅ 基本 RTP 音频流建立
- ✅ 多协议传输支持（UDP、TCP、WebSocket、WebSocket Secure）
- ✅ IPv4 和 IPv6 双栈支持（自动回退）
- ✅ Outbound 代理支持
- ✅ 异步处理（基于 tokio）
- ✅ 线程安全（使用 Arc/Mutex）
- ✅ 完整的错误处理
- ✅ 完整的单元测试和文档测试

## 项目结构

```
sip-caller/
├── src/
│   ├── main.rs        # 主程序示例，包含命令行参数解析
│   ├── config.rs      # 协议配置定义，包括 Protocol 枚举
│   ├── sip_client.rs  # SIP 客户端核心模块，处理注册和呼叫
│   ├── sip_dialog.rs  # SIP 对话层处理
│   ├── sip_transport.rs # 传输层辅助函数，包括 UDP 连接创建
│   ├── rtp.rs         # RTP 处理模块
│   └── utils.rs       # 工具函数，包括日志初始化和网络接口获取
├── Cargo.toml         # 项目依赖配置
└── README.md          # 项目文档
```

## 依赖

- `tokio` - 异步运行时，处理并发网络 I/O
- `rsipstack` - SIP 协议栈实现（支持 UDP/TCP/WebSocket）
- `rsip` - SIP 消息解析
- `tracing` - 结构化日志
- `uuid` - 呼叫 ID 生成和管理
- `clap` - 命令行参数解析（支持环境变量）
- `rand` - 随机数生成
- `tokio-util` - Tokio 工具库
- `get_if_addrs` - 网络接口获取
- `rtp-rs` - RTP 处理库

## 快速开始

### 安装依赖

确保已安装 Rust 1.80+：

```bash
rustup update
```

### 编译项目

```bash
cargo build
```

### 运行示例

#### 使用命令行参数

```bash
# 查看帮助信息
cargo run -- --help

# 使用默认配置（持续通话，按 Ctrl+C 终止）
cargo run

# 使用自定义参数
cargo run -- --server 192.168.1.100:5060 --user alice@example.com --target bob@example.com --duration 30

# 持续通话直到手动按 Ctrl+C 终止
cargo run -- --server 192.168.1.100:5060 --user alice --target bob

# 启用详细日志
cargo run -- --verbose

# 设置特定日志级别
cargo run -- --log-level debug

# 简写形式（30秒后自动挂断）
cargo run -- -s 192.168.1.100:5060 -u alice@example.com -p mypassword -t bob@example.com -d 30 -v
```

#### 使用环境变量

```bash
# 设置环境变量
export SIP_SERVER="192.168.1.100:5060"
export SIP_USER="alice@example.com"
export SIP_PASSWORD="mypassword"
export SIP_TARGET="bob@example.com"

# 运行程序
cargo run

# 或一次性设置
SIP_SERVER="192.168.1.100:5060" \
SIP_USER="alice@example.com" \
SIP_PASSWORD="mypassword" \
SIP_TARGET="bob@example.com" \
cargo run
```

#### 命令行参数说明

| 参数 | 短参数 | 环境变量 | 默认值 | 说明 |
|------|--------|----------|--------|------|
| `--server` | `-s` | `SIP_SERVER` | `127.0.0.1:5060` | SIP 服务器地址 |
| `--outbound-proxy` | - | - | 无 | Outbound 代理服务器地址（可选） |
| `--user` | `-u` | `SIP_USER` | `alice@example.com` | SIP 用户 ID |
| `--password` | `-p` | `SIP_PASSWORD` | `password` | SIP 密码 |
| `--target` | `-t` | `SIP_TARGET` | `bob@example.com` | 呼叫目标 |
| `--local-port` | - | - | `0` | 本地 SIP 端口（0 表示自动分配） |
| `--ipv6` | - | - | `false` | 优先使用 IPv6（找不到时自动回退到 IPv4） |
| `--rtp-start-port` | - | - | `20000` | RTP 起始端口 |
| `--user-agent` | - | - | `RSipCaller/0.2.0` | User-Agent 标识 |
| `--log-level` | `-l` | - | `info` | 日志级别（trace/debug/info/warn/error） |

### 协议类型说明

SIP Caller 支持以下传输协议：

- **UDP** (`udp`): 默认协议，无连接传输，适合大多数 SIP 场景
- **TCP** (`tcp`): 面向连接传输，更可靠但开销稍大
- **WebSocket** (`ws`): 基于 HTTP 的 WebSocket 传输，适合 Web 应用
- **WebSocket Secure** (`wss`): 基于 HTTPS 的加密 WebSocket 传输，安全性最高

使用示例：

```bash
# 使用 UDP（默认）
cargo run -- --server 192.168.1.100:5060

# 使用 TCP
cargo run -- --server 192.168.1.100:5060 --protocol tcp

# 使用 WebSocket
cargo run -- --server 192.168.1.100:8080 --protocol ws

# 使用 WebSocket Secure
cargo run -- --server 192.168.1.100:443 --protocol wss
```

### IPv6 支持说明

SIP Caller 支持 IPv4 和 IPv6 双栈网络：

- **默认行为（IPv4）**: 优先使用 IPv4 地址，如果找不到 IPv4 接口则自动回退到 IPv6
- **启用 IPv6**: 使用 `--ipv6` 参数优先使用 IPv6 地址，找不到时自动回退到 IPv4

智能回退机制确保在各种网络环境下都能正常工作。

使用示例：

```bash
# 默认使用 IPv4
cargo run -- --server 192.168.1.100:5060

# 优先使用 IPv6（找不到时自动回退到 IPv4）
cargo run -- --server 192.168.1.100:5060 --ipv6

# 在纯 IPv6 环境中
cargo run -- --server [2001:db8::1]:5060 --ipv6

# 组合使用：TCP + IPv6
cargo run -- --server 192.168.1.100:5060 --protocol tcp --ipv6
```

### Outbound 代理支持

Outbound 代理（Outbound Proxy）允许所有 SIP 请求通过指定的代理服务器转发，这在以下场景中非常有用：

- **NAT 穿越**：通过代理服务器解决 NAT 问题
- **企业网络**：符合企业网络架构要求
- **负载均衡**：通过代理实现负载均衡
- **安全控制**：集中管理和监控 SIP 流量

#### 工作原理

当指定 Outbound 代理时：
1. 客户端连接到代理服务器而不是目标 SIP 服务器
2. SIP 消息中的目标 URI 仍然是原始服务器
3. 代理负责将请求转发到实际的 SIP 服务器

#### 使用示例

```bash
# 基本用法：通过代理连接到 SIP 服务器
cargo run -- --server sip.example.com:5060 --outbound-proxy proxy.example.com:5060

# 使用 TCP 协议和代理
cargo run -- --server sip.example.com:5060 --protocol tcp --outbound-proxy proxy.example.com:5060

# 组合使用：代理 + IPv6 + TCP
cargo run -- --server sip.example.com:5060 --protocol tcp --ipv6 --outbound-proxy proxy.example.com:5060

# 企业环境示例
cargo run -- \
  --server internal-sip.company.com:5060 \
  --outbound-proxy corporate-proxy.company.com:5060 \
  --user alice@company.com \
  --password secret \
  --target bob@company.com
```

#### 注意事项

- 代理地址格式：`hostname:port` 或 `ip:port`
- 代理必须支持相应的传输协议（UDP/TCP/WS/WSS）
- 确保防火墙允许连接到代理服务器
- 代理和目标服务器可以使用不同的端口

#### 技术实现

SIP Caller 使用 rsipstack 的 TransportLayer 内置的 outbound 代理支持，直接通过设置 `TransportLayer.outbound` 字段来配置代理，确保：
- 连接逻辑更清晰
- 代码更符合 rsipstack 设计
- 减少不必要的中间步骤

代理 URI 支持完整格式，如 `sip:proxy.example.com:5060;transport=tcp;lr`，系统会自动：
- 提取 transport 协议
- 添加 lr 参数（如果缺失）
- 正确设置连接目标

### 运行测试

```bash
# 运行所有单元测试
cargo test

# 运行单元测试并显示输出
cargo test -- --show-output

# 运行特定测试
cargo test test_sip_config_creation
```

## API 使用示例

### 基本用法

```rust
use sip_caller::{SipClient, SipConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 创建 SIP 配置
    let config = SipConfig::new("127.0.0.1:5060", "alice@example.com")?;

    // 2. 创建 SIP 客户端
    let client = SipClient::new(config).await?;

    // 3. 注册到 SIP 服务器
    client.register("password").await?;

    // 4. 发起呼叫
    let call_id = client.call("bob@example.com").await?;
    println!("呼叫 ID: {}", call_id);

    // 5. 挂断呼叫
    client.hangup(&call_id).await?;

    Ok(())
}
```

### 自定义配置

```rust
use sip_caller::SipConfig;
use std::net::SocketAddr;

// 创建自定义配置
let config = SipConfig::new("192.168.1.100:5060", "alice@example.com")?
    .with_media_addr("192.168.1.100:20000".parse::<SocketAddr>()?)
    .with_user_agent("MyApp/2.0".to_string())
    .with_codecs(vec!["PCMU".to_string(), "G729".to_string()]);
```

### 错误处理

```rust
use sip_caller::{SipClient, SipConfig};
use sip_caller::error::SipCallerError;

#[tokio::main]
async fn main() {
    let config = SipConfig::new("127.0.0.1:5060", "alice@example.com")
        .expect("无效的配置");

    let client = SipClient::new(config).await
        .expect("创建客户端失败");

    match client.register("password").await {
        Ok(_) => println!("注册成功"),
        Err(SipCallerError::RegistrationFailed(msg)) => {
            eprintln!("注册失败: {}", msg);
        }
        Err(e) => eprintln!("其他错误: {}", e),
    }
}
```

## 设计说明

### 架构选择

1. **异步处理**: 使用 tokio 异步框架处理网络 I/O，提高并发性能
2. **模块化设计**: 将错误处理、配置、客户端逻辑分离到不同模块
3. **线程安全**: 使用 `Arc<Mutex<>>` 管理共享状态，确保多线程安全
4. **错误处理**: 使用 `Result<T, SipCallerError>` 明确处理错误，提供有意义的错误信息
5. **依赖最小化**: 仅使用必要的外部 crate，优先使用标准库

### 错误处理策略

- 所有公共 API 返回 `Result<T, SipCallerError>`
- 使用 `thiserror` 创建自定义错误类型
- 错误信息清晰，便于调试
- 错误传播使用 `?` 运算符

### 并发处理

- 使用 `tokio::sync::Mutex` 保护共享状态
- 客户端管理器使用 `Arc` 包装，支持多线程访问
- 所有网络操作都是异步的，避免阻塞

### 文档

```rust
/// Doc comment for public items
///
/// This provides detailed documentation about the function, struct, or module.
/// Include examples when helpful.
///
/// # Examples
/// ```
/// let config = SipConfig::new("127.0.0.1:5060", "user@example.com")?;
/// ```
pub fn create_client(config: SipConfig) -> Result<SipClient, SipCallerError> {
    // Implementation
}
```

### 测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sip_config_creation() {
        let config = SipConfig::new("127.0.0.1:5060", "user@example.com").unwrap();
        assert_eq!(config.server, "127.0.0.1:5060");
    }

    #[tokio::test]
    async fn test_async_operation() {
        let result = perform_async_operation().await;
        assert!(result.is_ok());
    }
}
```

## 环境变量

主程序支持以下环境变量（也可以通过命令行参数设置）：

- `SIP_SERVER`: SIP 服务器地址（默认: `127.0.0.1:5060`）
- `SIP_USER`: SIP 用户 ID（默认: `alice@example.com`）
- `SIP_PASSWORD`: SIP 密码（默认: `password`）
- `SIP_TARGET`: 呼叫目标（默认: `bob@example.com`）

**注意**: 命令行参数的优先级高于环境变量。

## 性能和安全

### 性能优化

- 使用异步 I/O 减少线程开销
- 零拷贝网络传输（rvoip 内部优化）
- 高效的消息解析

### 安全性

- 避免使用 `unsafe` 代码
- 使用 Rust 所有权系统防止内存错误
- 输入验证（SIP URIs、地址解析）
- 错误边界清晰，防止 panic
- Sanitize log output to prevent information leakage

## 编译检查

```bash
# 检查代码编译
cargo check

# 检查代码格式
cargo fmt --check

# 运行 Clippy 代码检查
cargo clippy -- -D warnings
```

## 许可证

本项目仅用于学习和演示目的。

## 贡献

欢迎提交 Issue 和 Pull Request！

## 联系方式

如有问题，请提交 Issue。