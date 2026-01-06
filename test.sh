#!/bin/bash
# SIP Caller 测试脚本

set -e

echo "========================================="
echo "SIP Caller - Outbound 代理测试"
echo "========================================="
echo ""

# 默认配置
SERVER="${SERVER:-xfc:5060}"
OUTBOUND_PROXY="${OUTBOUND_PROXY:-sip.tst.novo-one.com:5060}"
USER="${USER:-1001}"
PASSWORD="${PASSWORD:-admin}"
TARGET="${TARGET:-1000}"
PROTOCOL="${PROTOCOL:-udp}"
LOG_LEVEL="${LOG_LEVEL:-info}"

echo "配置信息:"
echo "  服务器 (租户ID): $SERVER"
echo "  Outbound 代理: $OUTBOUND_PROXY"
echo "  用户: $USER"
echo "  密码: ****"
echo "  目标: $TARGET"
echo "  协议: $PROTOCOL"
echo "  日志级别: $LOG_LEVEL"
echo ""

# 编译项目
echo "📦 编译项目..."
cargo build --release

echo ""
echo "🚀 启动 SIP Caller..."
echo "========================================="
echo ""

# 运行程序
cargo run --release -- \
  --server "$SERVER" \
  --outbound-proxy "$OUTBOUND_PROXY" \
  --user "$USER" \
  --password "$PASSWORD" \
  --target "$TARGET" \
  --protocol "$PROTOCOL" \
  --log-level "$LOG_LEVEL"

echo ""
echo "========================================="
echo "测试完成"
echo "========================================="
