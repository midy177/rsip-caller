.PHONY: all build test clean run help release cross cross-linux cross-windows cross-all

# 项目配置
PROJECT_NAME = sip-caller
CARGO = cargo

# 默认目标
all: build

# 显示帮助信息
help:
	@echo "SIP Caller - Makefile"
	@echo "===================="
	@echo ""
	@echo "基础命令:"
	@echo "  make build           - 编译项目（调试版本）"
	@echo "  make release         - 编译项目（发布版本）"
	@echo "  make test            - 运行测试"
	@echo "  make run             - 运行程序"
	@echo "  make clean           - 清理编译产物"
	@echo ""
	@echo "Cross 跨平台编译:"
	@echo "  make linux           - 编译 Linux x86_64 版本"
	@echo "  make windows         - 编译 Windows x86_64 版本"
	@echo "  make all-platforms   - 编译所有平台版本"
	@echo "  make install-targets - 安装交叉编译目标"
	@echo ""
	@echo "代码质量:"
	@echo "  make lint            - 运行代码检查（clippy）"
	@echo "  make fmt             - 格式化代码"
	@echo "  make fmt-check       - 检查代码格式"
	@echo "  make doc             - 生成文档"
	@echo ""
	@echo "注意: 需要先安装 cross: cargo install cross"
	@echo ""

# 编译项目（调试版本）
build:
	@echo "编译项目（调试版本）..."
	$(CARGO) build

# 编译项目（发布版本）
release:
	@echo "编译项目（发布版本）..."
	$(CARGO) build --release

# 运行测试
test:
	@echo "运行测试..."
	$(CARGO) test -- --show-output

# 运行测试（详细输出）
test-verbose:
	@echo "运行测试（详细输出）..."
	$(CARGO) test -- --nocapture --test-threads=1

# 运行程序
run:
	@echo "运行程序..."
	$(CARGO) run

# 运行程序（发布版本）
run-release:
	@echo "运行程序（发布版本）..."
	$(CARGO) run --release

# 清理编译产物
clean:
	@echo "清理编译产物..."
	$(CARGO) clean

# 检查代码（不编译）
check:
	@echo "检查代码..."
	$(CARGO) check

# 运行代码检查（clippy）
lint:
	@echo "运行代码检查..."
	$(CARGO) clippy -- -D warnings

# 格式化代码
fmt:
	@echo "格式化代码..."
	$(CARGO) fmt

# 检查代码格式
fmt-check:
	@echo "检查代码格式..."
	$(CARGO) fmt --check

# 生成文档
doc:
	@echo "生成文档..."
	$(CARGO) doc

# Cross 编译命令
# 检查 cross 是否安装
check-cross:
	@which cross > /dev/null 2>&1 || \
		(echo "错误: cross 未安装"; \
		echo "请先安装 cross:"; \
		echo "  cargo install cross"; \
		exit 1)

# 编译 Linux 版本
linux: check-cross
	@echo "使用 cross 编译 Linux x86_64 版本..."
	@cross build --release --target x86_64-unknown-linux-gnu

# 编译 Windows 版本
windows: check-cross
	@echo "使用 cross 编译 Windows x86_64 版本..."
	@cross build --release --target x86_64-pc-windows-gnu

# 编译所有平台版本
all-platforms: check-cross
	@echo "使用 cross 编译所有平台版本..."
	@echo ""
	@echo "编译 Linux 版本..."
	@$(MAKE) linux || echo "警告: Linux 编译失败"
	@echo ""
	@echo "编译 Windows 版本..."
	@$(MAKE) windows || echo "警告: Windows 编译失败"
	@echo ""
	@echo "编译完成！"
	@echo ""
	@if [ -f target/x86_64-unknown-linux-gnu/release/$(PROJECT_NAME) ]; then \
		echo "✓ Linux: target/x86_64-unknown-linux-gnu/release/$(PROJECT_NAME)"; \
		ls -lh target/x86_64-unknown-linux-gnu/release/$(PROJECT_NAME); \
	else \
		echo "✗ Linux: 编译失败"; \
	fi
	@if [ -f target/x86_64-pc-windows-gnu/release/$(PROJECT_NAME).exe ]; then \
		echo "✓ Windows: target/x86_64-pc-windows-gnu/release/$(PROJECT_NAME).exe"; \
		ls -lh target/x86_64-pc-windows-gnu/release/$(PROJECT_NAME).exe; \
	else \
		echo "✗ Windows: 编译失败"; \
	fi

# 安装交叉编译目标
install-targets:
	@echo "安装交叉编译目标..."
	@echo "安装 Linux 目标..."
	rustup target add x86_64-unknown-linux-gnu
	@echo "安装 Windows 目标..."
	rustup target add x86_64-pc-windows-gnu
	@echo ""
	@echo "交叉编译目标安装完成！"

# 使用 cross 运行测试
cross-test: check-cross
	@echo "使用 cross 运行测试..."
	@cross test --release -- --show-output
