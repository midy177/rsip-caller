# Rust 方法重写完整指南

## 概述

Rust 不像传统 OOP 语言那样有类继承和虚方法表，但提供了多种实现"方法重写"的机制。

## 方法 1: Trait 默认实现重写 ⭐

这是 Rust 最推荐的方式，类似于接口的默认实现。

```rust
// 定义 trait 及默认实现
trait CallIdGenerator {
    fn make_call_id(&self, domain: Option<&str>) -> String {
        format!("{}@{}", random_text(10), domain.unwrap_or("default.com"))
    }
}

// 重写默认实现
struct UuidCallIdGenerator;

impl CallIdGenerator for UuidCallIdGenerator {
    fn make_call_id(&self, domain: Option<&str>) -> String {
        format!("{}@{}", uuid::Uuid::new_v4(), domain.unwrap_or("example.com"))
    }
}

// 使用
let generator = UuidCallIdGenerator;
let call_id = generator.make_call_id(Some("mydomain.com"));
```

**优点:**
- ✅ 符合 Rust 设计哲学
- ✅ 支持多态（通过 trait object）
- ✅ 类型安全

**缺点:**
- ❌ 需要定义 trait
- ❌ 运行时有轻微开销（如果用 dyn）

---

## 方法 2: 直接替换函数 ⭐⭐⭐（最简单）

直接定义自己的函数，不调用原函数。

```rust
// rsipstack 原始函数
fn make_call_id(domain: Option<&str>) -> rsip::headers::CallId {
    format!("{}@{}", random_text(10), domain.unwrap_or("restsend.com")).into()
}

// 你的替代函数
fn make_call_id(domain: Option<&str>) -> rsip::headers::CallId {
    let uuid = uuid::Uuid::new_v4();
    match domain {
        Some(d) => format!("{}@{}", uuid, d).into(),
        None => uuid.to_string().into(),
    }
}

// 使用
let call_id = make_call_id(Some("example.com"));
```

**优点:**
- ✅ 最简单直接
- ✅ 零开销
- ✅ 完全控制

**缺点:**
- ❌ 函数名可能冲突
- ❌ 不能真正"覆盖"库的内部调用

---

## 方法 3: Extension Trait 模式

为现有类型添加新方法。

```rust
trait CallIdExt {
    fn from_uuid(domain: Option<&str>) -> Self;
}

impl CallIdExt for rsip::headers::CallId {
    fn from_uuid(domain: Option<&str>) -> Self {
        let uuid = uuid::Uuid::new_v4();
        match domain {
            Some(d) => format!("{}@{}", uuid, d).into(),
            None => uuid.to_string().into(),
        }
    }
}

// 使用（需要导入 trait）
use CallIdExt;
let call_id = rsip::headers::CallId::from_uuid(Some("example.com"));
```

**优点:**
- ✅ 扩展外部类型
- ✅ 保持原有 API
- ✅ 类型安全

**缺点:**
- ❌ 需要显式导入 trait
- ❌ 可能与其他扩展冲突

---

## 方法 4: Newtype 包装模式

用新类型包装原类型。

```rust
struct MyCallId(rsip::headers::CallId);

impl MyCallId {
    fn new(domain: Option<&str>) -> Self {
        let uuid = uuid::Uuid::new_v4();
        let call_id = match domain {
            Some(d) => format!("{}@{}", uuid, d),
            None => uuid.to_string(),
        };
        MyCallId(call_id.into())
    }
}

impl From<MyCallId> for rsip::headers::CallId {
    fn from(my_call_id: MyCallId) -> Self {
        my_call_id.0
    }
}

// 使用
let my_call_id = MyCallId::new(Some("example.com"));
let call_id: rsip::headers::CallId = my_call_id.into();
```

**优点:**
- ✅ 完全控制
- ✅ 可以添加额外字段
- ✅ 类型安全

**缺点:**
- ❌ 需要转换
- ❌ 增加代码复杂度

---

## 方法 5: 辅助函数模块 ⭐⭐（推荐用于你的场景）

创建独立模块提供替代实现。

```rust
// src/utils.rs
pub fn make_call_id(domain: Option<&str>) -> rsip::headers::CallId {
    let uuid = uuid::Uuid::new_v4();
    match domain {
        Some(d) => format!("{}@{}", uuid, d).into(),
        None => uuid.to_string().into(),
    }
}

// src/main.rs
mod utils;

fn main() {
    let call_id = utils::make_call_id(Some("example.com"));
    registration.call_id = call_id;
}
```

**优点:**
- ✅ 简单清晰
- ✅ 模块化
- ✅ 易于测试
- ✅ 零开销

**缺点:**
- ❌ 需要手动调用

---

## 实际项目中的应用

在你的 SIP Caller 项目中，我们使用了**方法 5（辅助函数模块）**：

```rust
// src/utils.rs
pub fn make_call_id(domain: Option<&str>) -> rsip::headers::CallId {
    let uuid = Uuid::new_v4();
    match domain {
        Some(d) => format!("{}@{}", uuid, d).into(),
        None => uuid.to_string().into(),
    }
}

// src/main.rs
// REGISTER 请求
let register_call_id = utils::make_call_id(Some(&server_host));
registration.call_id = register_call_id;

// INVITE 请求
let call_id = utils::make_call_id(Some(&server_host));
invite_opt.call_id = Some(call_id.to_string());
```

### 为什么选择这种方式？

1. **简单直接** - 不需要复杂的 trait 定义
2. **模块化** - 所有工具函数集中管理
3. **可测试** - 独立的单元测试
4. **灵活** - 可以轻松添加其他辅助函数
5. **零开销** - 编译时内联优化

---

## 对比表格

| 方法 | 复杂度 | 性能 | 灵活性 | 适用场景 |
|------|--------|------|--------|----------|
| Trait 默认实现 | 中 | 高 | 高 | 需要多态的场景 |
| 直接替换函数 | 低 | 最高 | 中 | 简单替换 |
| Extension Trait | 中 | 高 | 高 | 扩展外部类型 |
| Newtype 包装 | 高 | 高 | 最高 | 需要额外功能 |
| 辅助函数模块 | 低 | 最高 | 中 | 工具函数集合 |

---

## 最佳实践建议

1. **优先使用 Trait** - 如果需要多态或抽象
2. **使用辅助函数** - 如果只是简单替换
3. **Extension Trait** - 扩展外部库类型
4. **Newtype** - 需要强类型封装
5. **避免过度设计** - Rust 推崇简单直接

---

## 总结

Rust 没有传统 OOP 的继承和虚方法，但通过：
- **Trait** 实现接口和多态
- **组合** 代替继承
- **模块化** 组织代码

这些机制提供了更灵活、更安全的代码重用方式。选择哪种方法取决于具体需求：
- 需要多态？用 **Trait**
- 简单替换？用 **辅助函数**
- 扩展类型？用 **Extension Trait**
- 封装控制？用 **Newtype**
