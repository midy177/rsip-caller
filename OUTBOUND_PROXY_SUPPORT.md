# Outbound Proxy 支持文档

## 概述

SIP Caller 现在支持通过outbound proxy进行SIP通信，这对于需要通过代理服务器或NAT穿透的场景非常有用。

## 命令行参数

新增了`--outbound-proxy`参数：

```bash
# 通过代理服务器连接
sip-caller --server sip:server.com:5060 --user user@example.com --password pass --outbound-proxy sip:proxy.com:5060;transport=udp;lr

# 也可以使用环境变量
export SIP_OUTBOUND_PROXY="sip:proxy.com:5060;transport=udp;lr"
sip-caller --server sip:server.com:5060 --user user@example.com --password pass
```

## 代理URI格式

代理URI支持完整的SIP URI格式：

```
sip:proxy.example.com:5060;transport=udp;lr
```

参数说明：
- `transport`: 传输协议 (udp/tcp/ws/wss)
- `lr`: 添加loose routing标识（重要！）
- `;`: 参数分隔符

## API 使用

### 便捷函数

```rust
// 不使用代理
let client = create_sip_client("server.com:5060", "user", "pass").await?;

// 使用代理
let client = create_sip_client_with_proxy(
    "server.com:5060", 
    "user", 
    "pass", 
    Some("sip:proxy.com:5060;transport=udp;lr")
).await?;
```

### 环境变量支持

应用程序支持以下环境变量：

- `SIP_SERVER`: SIP服务器地址
- `SIP_USER`: SIP用户名
- `SIP_PASSWORD`: SIP密码
- `SIP_OUTBOUND_PROXY`: 代理服务器地址

## 工作原理

1. **代理检测**: 如果配置了outbound proxy，所有SIP请求将发送到代理服务器
2. **路由处理**: 代理服务器处理请求路由到目标服务器
3. **响应路径**: 响应通过相同的代理路径返回
4. **NAT穿透**: 代理可以帮助穿透复杂的NAT环境

## 使用场景

### 1. 企业网络
```
sip-caller --server sip:internal.company.com \
           --user alice@company.com \
           --password secret \
           --outbound-proxy sip:proxy.company.com:5060;lr
```

### 2. 云代理服务
```
sip-caller --server sip:provider.com \
           --user alice@provider.com \
           --password secret \
           --outbound-proxy sip:cloud-proxy.com:5080;transport=tcp
```

### 3. 本地开发测试
```
# 创建简单的代理服务器（使用sipp或类似工具）
sip-caller --server sip:target.local \
           --user test@test.local \
           --password test \
           --outbound-proxy sip:proxy.local:5060
```

## 注意事项

1. **必需参数**: `lr`标识对于大多数代理服务器是必需的
2. **传输匹配**: 确保代理和目标使用兼容的传输协议
3. **DNS解析**: 代理主机名必须可解析
4. **防火墙**: 确保代理端口可访问
5. **TLS支持**: 对于SIPS连接，代理也必须支持TLS

## 故障排除

### 代理连接失败
```
# 检查代理服务器状态
telnet proxy.example.com 5060

# 验证URI格式
sip:sip:proxy.example.com:5060;transport=udp;lr
```

### NAT穿透问题
1. 使用UDP传输（默认）
2. 配置STUN/TURN服务器（如果支持）
3. 检查代理服务器的NAT配置

### 调试日志
```bash
RUST_LOG=debug sip-caller --server ... --outbound-proxy ...
```

## 实现细节

outbound proxy支持在以下文件中实现：

- `src/main.rs`: 命令行参数解析和API调用
- `src/lib.rs`: create_sip_client_with_proxy() 函数
- `src/sip_client.rs`: 代理处理逻辑
- `src/sip_transport.rs`: 网络传输层处理

代理功能完全集成到现有的SIP客户端架构中，不会影响不使用代理的场景。