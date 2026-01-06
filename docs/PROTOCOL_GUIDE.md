# 传输协议配置功能

## 功能概述

SIP Caller 现在支持多种传输协议：

- **UDP** - 无连接传输（默认）
- **TCP** - 面向连接传输
- **WebSocket (WS)** - HTTP WebSocket 传输
- **WebSocket Secure (WSS)** - HTTPS WebSocket 传输

## 快速使用

### 1. 使用 UDP（默认）

```bash
cargo run -- --server 192.168.1.100:5060 --user alice --password secret --target bob
```

### 2. 使用 TCP

```bash
cargo run -- --server 192.168.1.100:5060 --protocol tcp --user alice --password secret --target bob
```

### 3. 使用 WebSocket

```bash
cargo run -- --server 192.168.1.100:8080 --protocol ws --user alice --password secret --target bob
```

### 4. 使用 WebSocket Secure

```bash
cargo run -- --server 192.168.1.100:443 --protocol wss --user alice --password secret --target bob
```

## 代码架构

### 新增文件

1. **src/config.rs** - 协议配置模块
   - `Protocol` 枚举：定义支持的协议类型
   - 协议解析和验证
   - 单元测试

2. **src/transport.rs** - 传输层辅助模块
   - `create_transport_connection()`: 根据协议类型创建连接
   - `extract_peer_rtp_addr()`: 从 SDP 提取 RTP 地址
   - 单元测试

### 修改文件

1. **src/main.rs**
   - 添加 `--protocol` 命令行参数
   - 使用新的传输创建函数
   - 重构代码结构

2. **README.md**
   - 更新特性列表
   - 添加协议配置说明
   - 更新使用示例

## Protocol 枚举 API

```rust
pub enum Protocol {
    Udp,   // UDP 传输
    Tcp,   // TCP 传输
    Ws,    // WebSocket 传输
    Wss,   // WebSocket Secure 传输
}

// 主要方法
impl Protocol {
    pub fn as_str(&self) -> &'static str;
    pub fn default_port(&self) -> u16;
    pub fn is_secure(&self) -> bool;
    pub fn is_websocket(&self) -> bool;
    pub fn to_rsip_transport(&self) -> rsip::transport::Transport;
}

// 从字符串解析
impl FromStr for Protocol {
    fn from_str(s: &str) -> Result<Self, Self::Err>;
}
```

## 测试

运行所有测试：

```bash
cargo test
```

测试覆盖：
- ✅ 协议类型解析测试
- ✅ 协议默认端口测试
- ✅ 协议安全性测试
- ✅ SDP 解析测试
- ✅ 传输连接创建（编译时验证）

## 技术细节

### 协议转换

每个协议都会被转换为 rsipstack 的相应类型：

- `Protocol::Udp` → `UdpConnection`
- `Protocol::Tcp` → `TcpConnection`
- `Protocol::Ws` → `WebSocketConnection` (WS 模式)
- `Protocol::Wss` → `WebSocketConnection` (WSS 模式)

### 连接创建流程

1. 解析命令行参数中的 `--protocol` 选项
2. 检测本地网络接口 IP
3. 调用 `create_transport_connection()` 创建对应连接
4. 将连接添加到传输层
5. 建立 SIP 会话

## 注意事项

1. **端口选择**
   - UDP/TCP 默认使用 5060 端口
   - WS 默认使用 80 端口
   - WSS 默认使用 443 端口

2. **服务器要求**
   - 服务器必须支持相应的传输协议
   - WebSocket 需要服务器配置 WebSocket 升级
   - WSS 需要有效的 TLS 证书

3. **防火墙配置**
   - 确保防火墙允许相应协议和端口
   - TCP/WS/WSS 需要建立连接，可能需要额外配置

## 故障排除

### 连接失败

如果连接失败，请检查：

1. 服务器地址是否正确
2. 服务器是否支持该协议
3. 防火墙设置
4. 网络连接

### 调试模式

启用详细日志查看更多信息：

```bash
cargo run -- --log-level debug --protocol tcp --server your-server:5060
```

或使用 trace 级别：

```bash
cargo run -- --log-level trace --protocol ws --server your-server:8080
```
