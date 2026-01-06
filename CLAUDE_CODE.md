# SIP Caller 实现总结

## 项目概述
基于 `rsipstack` 库实现的完整 SIP 客户端，参考 https://github.com/restsend/rsipstack/blob/main/examples/client 的官方示例代码。

## 已实现的功能

### ✅ 核心 SIP 功能
1. **SIP 注册 (REGISTER)**
   - 支持 Digest 认证
   - 自动处理 401 Unauthorized 响应
   - 可配置过期时间

2. **呼叫发起 (INVITE)**
   - 发起 INVITE 请求
   - 携带 SDP Offer
   - 处理 SDP Answer
   - 支持认证重试

3. **挂断处理 (BYE)**
   - 主动发送 BYE 请求
   - 清理 RTP 会话资源

### ✅ RTP 媒体流
1. **RTP 连接管理**
   - 自动端口绑定（支持端口范围扫描）
   - 生成标准 SDP 描述
   - 支持 PCMU (G.711μ) 和 PCMA (G.711a) 编解码器

2. **回声模式**
   - 将接收到的 RTP 数据原样发回
   - 用于测试通话双向连通性

3. **音频文件播放**（预留功能）
   - 支持播放 PCMU/PCMA 格式音频文件
   - 20ms 固定时间片发送

### ✅ 对话状态管理
- 实时监控对话状态变化
  - `Early`: 振铃状态
  - `Confirmed`: 通话建立
  - `Terminated`: 通话结束
- 状态变更时自动执行相应操作

## 项目结构

```
sip-caller/
├── src/
│   ├── main.rs          # 主程序，包含 SIP 注册和呼叫逻辑
│   └── rtp.rs           # RTP 媒体流处理模块
├── Cargo.toml           # 项目依赖配置
├── README.md            # 用户使用文档
└── CLAUDE_CODE.md       # 开发实现总结（本文件）
```

### 模块划分

#### main.rs
- **Args**: 命令行参数解析（使用 clap）
- **get_first_non_loopback_interface()**: 获取本地非回环 IP
- **process_dialog()**: 异步处理对话状态变化
- **extract_peer_rtp_addr()**: 从 SDP 中提取对端 RTP 地址
- **main()**: 主流程协调器

#### rtp.rs
- **MediaSessionOption**: RTP 会话配置结构体
- **build_rtp_conn()**: 建立 RTP 连接并生成 SDP
- **play_echo()**: 回声播放功能
- **play_audio_file()**: 音频文件播放功能

## 核心依赖

```toml
tokio = { version = "1.49.0", features = ["full"] }  # 异步运行时
rsipstack = "0.3.4"                                   # SIP 协议栈
rsip = "0.4"                                          # SIP 消息解析
rtp-rs = "0.6"                                        # RTP 数据包处理
clap = { version = "4.5", features = ["derive"] }    # 命令行解析
tracing = "0.1"                                       # 结构化日志
tokio-util = "0.7"                                    # Tokio 工具集
get_if_addrs = "0.5"                                  # 网络接口查询
```

## 关键实现细节

### 1. 异步架构
- 使用 Tokio 异步运行时
- 多个并发任务：
  - 端点服务任务 (endpoint.serve())
  - 传入请求处理任务
  - 对话状态监控任务
  - RTP 数据处理任务

### 2. 错误处理
- 所有关键操作返回 `Result<T, Box<dyn std::error::Error>>`
- 使用 `rsipstack::Error` 和 `rsipstack::Result`
- 错误信息清晰，便于调试

### 3. 资源管理
- 使用 `CancellationToken` 优雅关闭
- `Arc` 共享所有权
- 自动资源清理（RAII 模式）

### 4. SDP 协商
```
Client                    Server
  |                         |
  |--- INVITE (SDP Offer)-->|
  |                         |
  |<-- 200 OK (SDP Answer)--|
  |                         |
  |--- ACK ---------------->|
  |                         |
  |<== RTP Stream =========>|
  |                         |
  |--- BYE ---------------->|
  |<-- 200 OK --------------|
```

### 5. 端口管理
RTP 端口绑定策略：
- 从 `rtp_start_port` 开始（默认 20000）
- 每次尝试递增 2（为 RTCP 预留奇数端口）
- 最多尝试 100 个端口
- 失败则返回错误

## 命令行参数

```bash
Usage: sip-caller [OPTIONS]

Options:
  -s, --server <SERVER>              # SIP 服务器地址 [default: pbx.ras.yeastar.com:5060]
  -u, --user <USER>                  # SIP 用户 ID [default: 6634]
  -p, --password <PASSWORD>          # SIP 密码 [default: B5ULy6h6J9]
  -t, --target <TARGET>              # 呼叫目标 [default: 6737]
  -d, --duration <DURATION>          # 通话时长（秒）
      --local-port <LOCAL_PORT>      # 本地 SIP 端口 [default: 0]
      --rtp-start-port <RTP_START>   # RTP 起始端口 [default: 20000]
      --echo-mode <ECHO_MODE>        # 回声模式 [default: true]
  -l, --log-level <LOG_LEVEL>        # 日志级别 [default: info]
  -h, --help                         # 显示帮助信息
  -V, --version                      # 显示版本信息
```

## 使用示例

### 基本呼叫
```bash
# 默认配置呼叫（回声模式）
cargo run

# 指定呼叫时长
cargo run -- --duration 30

# 自定义配置
cargo run -- \
  --server sip.example.com:5060 \
  --user alice \
  --password secret123 \
  --target bob \
  --duration 60
```

### 调试模式
```bash
# 启用详细日志
cargo run -- --log-level debug

# 启用跟踪级别日志
cargo run -- --log-level trace
```

## 测试方式

### 1. 回声测试
```bash
cargo run -- --echo-mode true --duration 30
```
预期：对端应该能听到自己的声音（延迟很小）

### 2. 自动挂断测试
```bash
cargo run -- --duration 10
```
预期：10 秒后自动发送 BYE 并退出

### 3. 手动终止测试
```bash
cargo run
# 按 Ctrl+C 终止
```
预期：程序捕获信号，发送 BYE，清理资源后退出

### 4. 编译检查
```bash
# 检查代码
cargo check

# 构建发布版本
cargo build --release

# 运行 Clippy 检查
cargo clippy
```

## 性能优化

1. **发布版本编译优化**
```toml
[profile.release]
opt-level = 3        # 最高优化级别
lto = true           # 链接时优化
codegen-units = 1    # 单个代码生成单元
```

2. **异步并发**
- 所有 I/O 操作异步执行
- 多个独立任务并发运行
- 避免阻塞操作

3. **零拷贝**
- RTP 数据尽可能使用引用
- 减少不必要的内存分配

## 安全性考虑

1. **内存安全**
   - 无 unsafe 代码
   - Rust 所有权系统保证

2. **错误处理**
   - 所有可能失败的操作都返回 Result
   - 避免 panic（除非遇到无法恢复的错误）

3. **资源清理**
   - 使用 RAII 模式自动清理
   - CancellationToken 确保优雅关闭

## 代码规范

### 命名约定
- 模块名: 小写蛇形命名 (`rtp`, `main`)
- 结构体: 大驼峰命名 (`MediaSessionOption`, `Args`)
- 函数: 小写蛇形命名 (`build_rtp_conn`, `play_echo`)
- 常量: 大写蛇形命名（项目中未使用）

### 文档注释
- 所有公共 API 都有文档注释
- 包含参数说明、返回值说明
- 提供使用示例

### 代码组织
- 功能模块化（RTP 逻辑独立到 rtp.rs）
- 单一职责原则
- 低耦合高内聚

## 已知限制

1. **仅支持主动呼叫**
   - 当前不处理传入的 INVITE 请求
   - 无法接听来电

2. **编解码器**
   - 仅支持 PCMU 和 PCMA
   - 不支持高清编解码器（如 Opus）

3. **NAT 穿透**
   - 需要手动配置外部 IP
   - 不支持 STUN/TURN

4. **媒体功能**
   - 音频文件播放功能未在主流程中使用
   - 无音频录制功能
   - 无 DTMF 支持

## 后续改进方向

### 短期目标
- [ ] 实现接收来电功能
- [ ] 添加 DTMF 发送支持
- [ ] 实现音频录制功能
- [ ] 支持更多音频编解码器

### 中期目标
- [ ] 实现呼叫保持/恢复
- [ ] 实现呼叫转移
- [ ] 支持 STUN/TURN NAT 穿透
- [ ] 添加集成测试

### 长期目标
- [ ] 支持视频通话
- [ ] 实现会议功能
- [ ] 支持加密传输（SRTP）
- [ ] Web 界面控制

## 参考文档

- **rsipstack 官方仓库**: https://github.com/restsend/rsipstack
- **rsipstack 客户端示例**: https://github.com/restsend/rsipstack/blob/main/examples/client
- **RFC 3261 (SIP)**: https://tools.ietf.org/html/rfc3261
- **RFC 3550 (RTP)**: https://tools.ietf.org/html/rfc3550
- **RFC 4566 (SDP)**: https://tools.ietf.org/html/rfc4566
- **Tokio 文档**: https://tokio.rs/
- **Rust 异步编程**: https://rust-lang.github.io/async-book/

## 开发环境

- **Rust 版本**: 1.80+
- **操作系统**: Linux / macOS / Windows
- **依赖管理**: Cargo
- **IDE 推荐**: VS Code + rust-analyzer

## 总结

本项目成功实现了基于 `rsipstack` 的 SIP 客户端，具备以下特点：

1. **完整的 SIP 协议支持**: 注册、呼叫、挂断
2. **RTP 媒体流处理**: 回声模式、音频播放预留
3. **良好的代码组织**: 模块化、文档完善、命名规范
4. **稳定可靠**: 完善的错误处理、资源管理
5. **易于使用**: 丰富的命令行参数、清晰的日志输出

代码符合 Rust 最佳实践，遵循人体工程学设计原则，功能划分清晰明确。
