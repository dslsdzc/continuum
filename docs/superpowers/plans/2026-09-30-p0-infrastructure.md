# P0 基础设施与接口冻结 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 建立 Continuum 的五个 crate、持久化机制、Event Stream、Audit Log，并冻结 §315 / §316 / §124 三个外围接口。

**Architecture:** 五个 crate 以单向依赖组织，依赖方向在编译期强制 02 §9.1 的层间方向。持久化采用 SQLite（WAL、同步 FULL），数据库连接在 `continuum-persist` 内私有，外部 crate 只能经事务 API 访问，使 §318 的事务约束在编译期成立。启动流程按 §319 的五个阶段执行，各层通过恢复钩子注册表插入自己的步骤。

**Tech Stack:** Rust 1.95.0 / edition 2024、tokio、serde + serde_json、rusqlite（bundled）、sha2、async-trait、futures-core、thiserror、tempfile（dev）

**依据:** `docs/superpowers/specs/2026-09-30-p0-infrastructure-design.md`

## Global Constraints

- 工具链固定 rustc 1.95.0 / cargo 1.95.0，edition 2024。
- 依赖方向禁止反向。允许的边仅：`runtime → provider → core`、`runtime → persist → events → core`、`runtime → events`。
- `continuum-core` 不含 I/O。
- 数据库连接对象在 `continuum-persist` 内私有，外部 crate 只能通过 `Tx` 访问数据库。
- `continuum-core` 与 `continuum-persist` 不得引用任何 Provider 实现类型（§346）。
- 数据库 PRAGMA 固定为 `journal_mode=WAL`、`synchronous=FULL`、`foreign_keys=ON`。
- 事件类型名与审计类型名取 §310 与 §313 的原文，逐字符一致。
- 代码注释、错误信息、测试断言信息用中文。标识符用英文。
- 每个 task 结束时 `cargo test --workspace` 必须全绿，并提交一次。
- 不在仓库中写入任何凭据。凭据从环境或配置文件读取。

---

### Task 1: workspace 骨架与依赖方向断言

**Files:**
- Create: `Cargo.toml`
- Create: `crates/continuum-core/Cargo.toml`
- Create: `crates/continuum-core/src/lib.rs`
- Create: `crates/continuum-events/Cargo.toml`
- Create: `crates/continuum-events/src/lib.rs`
- Create: `crates/continuum-persist/Cargo.toml`
- Create: `crates/continuum-persist/src/lib.rs`
- Create: `crates/continuum-provider/Cargo.toml`
- Create: `crates/continuum-provider/src/lib.rs`
- Create: `crates/continuum-runtime/Cargo.toml`
- Create: `crates/continuum-runtime/src/main.rs`
- Test: `crates/continuum-runtime/tests/dependency_direction.rs`

**Interfaces:**
- Consumes: 无
- Produces: 五个 crate 名 `continuum-core`、`continuum-events`、`continuum-persist`、`continuum-provider`、`continuum-runtime`。此后的 task 只在这些 crate 内增删文件。

- [ ] **Step 1: 写依赖方向断言测试**

`crates/continuum-runtime/tests/dependency_direction.rs`

```rust
use std::path::Path;
use std::process::Command;

fn cargo_tree(pkg: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    // --edges all 同时覆盖 normal、build、dev。只查 normal 不够：
    // dev-dependency 同样允许测试代码引用 provider 类型，
    // 而 §346 的中立性约束要拦住所有引用路径。
    let out = Command::new(env!("CARGO"))
        .args(["tree", "-p", pkg, "--edges", "all", "--prefix", "none"])
        .current_dir(root)
        .output()
        .expect("cargo tree 无法执行");
    assert!(
        out.status.success(),
        "cargo tree -p {pkg} 失败: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("cargo tree 输出不是 UTF-8")
}

#[test]
fn core_and_persist_do_not_depend_on_provider() {
    for pkg in ["continuum-core", "continuum-persist"] {
        let tree = cargo_tree(pkg);
        assert!(
            !tree.contains("continuum-provider"),
            "{pkg} 依赖了 continuum-provider，违反 §346:\n{tree}"
        );
    }
}
```

- [ ] **Step 2: 建立 workspace 与五个 crate**

`Cargo.toml`

```toml
[workspace]
resolver = "3"
members = [
    "crates/continuum-core",
    "crates/continuum-events",
    "crates/continuum-persist",
    "crates/continuum-provider",
    "crates/continuum-runtime",
]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.95"

[workspace.dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rusqlite = { version = "0.40", features = ["bundled"] }
sha2 = "0.10"
async-trait = "0.1"
futures-core = "0.3"
thiserror = "2"
tempfile = "3"
```

`crates/continuum-core/Cargo.toml`

```toml
[package]
name = "continuum-core"
version.workspace = true
edition.workspace = true
rust-version.workspace = true

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
futures-core = { workspace = true }
thiserror = { workspace = true }
```

`crates/continuum-events/Cargo.toml`

```toml
[package]
name = "continuum-events"
version.workspace = true
edition.workspace = true
rust-version.workspace = true

[dependencies]
continuum-core = { path = "../continuum-core" }
serde = { workspace = true }
serde_json = { workspace = true }
sha2 = { workspace = true }
thiserror = { workspace = true }
```

`crates/continuum-persist/Cargo.toml`

```toml
[package]
name = "continuum-persist"
version.workspace = true
edition.workspace = true
rust-version.workspace = true

[dependencies]
continuum-events = { path = "../continuum-events" }
rusqlite = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

`crates/continuum-provider/Cargo.toml`

```toml
[package]
name = "continuum-provider"
version.workspace = true
edition.workspace = true
rust-version.workspace = true

[dependencies]
continuum-core = { path = "../continuum-core" }
async-trait = { workspace = true }
serde_json = { workspace = true }
```

`crates/continuum-runtime/Cargo.toml`

```toml
[package]
name = "continuum-runtime"
version.workspace = true
edition.workspace = true
rust-version.workspace = true

[dependencies]
continuum-core = { path = "../continuum-core" }
continuum-events = { path = "../continuum-events" }
continuum-persist = { path = "../continuum-persist" }
continuum-provider = { path = "../continuum-provider" }
tokio = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

各 crate 的 `src/lib.rs` 先写一行占位，`continuum-runtime/src/main.rs` 写空 `fn main() {}`：

```rust
// 类型在后续 task 引入。
```

```rust
fn main() {}
```

注意：`continuum-events` 依赖 `continuum-core`，但 P0 的事件与审计不使用 core 的类型。该依赖是为后续层预留的，不引入任何 `use continuum_core`。若实现时编译器报未使用依赖警告，保留依赖，不要删除。

- [ ] **Step 3: 构建确认骨架可用**

Run: `cargo build --workspace`
Expected: 编译成功。首次编译 rusqlite bundled 需要数分钟。

- [ ] **Step 4: 运行测试确认它是有效的断言**

Run: `cargo test -p continuum-runtime --test dependency_direction -v`
Expected: PASS，1 passed。

- [ ] **Step 5: 提交**

```bash
git add Cargo.toml Cargo.lock crates
git commit -m "feat: 建立 workspace 与五个 crate，断言依赖方向"
```

---

### Task 2: continuum-core 的 Provider 接口类型

**Files:**
- Create: `crates/continuum-core/src/error.rs`
- Create: `crates/continuum-core/src/model.rs`
- Create: `crates/continuum-core/src/tool.rs`
- Create: `crates/continuum-core/src/connector.rs`
- Modify: `crates/continuum-core/src/lib.rs`
- Test: `crates/continuum-core/tests/connector_descriptor.rs`

**Interfaces:**
- Consumes: 无
- Produces:
  - `continuum_core::ProviderError`：`Unavailable(String)`、`UnknownModel(String)`、`Cancelled(String)`、`Transport(String)`、`Protocol(String)`
  - `continuum_core::model`：`ModelId`、`ModelDescriptor`、`Role`、`Message`、`InvokeRequest`、`InvokeResponse`、`Usage`、`StreamChunk`、`CallId`、`ProviderHealth`、`ModelStream`
  - `continuum_core::tool`：`ToolId`、`ToolDescriptor`、`ToolInvocation`、`ToolResult`
  - `continuum_core::connector`：`ConnectorId`、`ConnectorOp`、`ConnectorDescriptor`
    - `ConnectorOp::new(op) -> Result<ConnectorOp, CoreError>`，空串与不含 `.` 的标识一律拒绝
    - `ConnectorDescriptor` 字段私有，`new(id, operations) -> Result<Self, CoreError>` 是唯一构造入口；
      `id()`、`operations()` 为访问器；反序列化经 `#[serde(try_from = "RawConnectorDescriptor")]` 回流到 `new`
  - `CoreError`：`ConnectorWithoutOperations { connector }`、`MalformedConnectorOp { op }`

- [ ] **Step 1: 写 §125 的操作级授权断言测试**

`crates/continuum-core/tests/connector_descriptor.rs`

```rust
use continuum_core::connector::{ConnectorDescriptor, ConnectorId, ConnectorOp};

fn op(s: &str) -> ConnectorOp {
    ConnectorOp::new(s).expect("合法操作标识")
}

fn github() -> ConnectorId {
    ConnectorId::new("GitHub")
}

#[test]
fn empty_operation_list_is_rejected() {
    let err = ConnectorDescriptor::new(github(), vec![]).expect_err("空操作列表必须被拒绝");
    assert_eq!(
        err.to_string(),
        "Connector GitHub 未声明任何操作，退回服务级授权（§125）"
    );
}

#[test]
fn operation_scoped_descriptor_is_accepted() {
    let d = ConnectorDescriptor::new(github(), vec![op("GitHub.read_repo"), op("GitHub.push_branch")])
        .expect("非空操作列表应被接受");
    assert_eq!(d.id().as_str(), "GitHub");
    assert_eq!(d.operations().len(), 2);
    assert_eq!(d.operations()[0].as_str(), "GitHub.read_repo");
}

#[test]
fn malformed_operation_id_is_rejected() {
    assert!(ConnectorOp::new("").is_err(), "空串必须被拒绝");
    assert!(ConnectorOp::new("GitHub").is_err(), "不含 . 的标识必须被拒绝");
    assert_eq!(op("GitHub.push_branch").as_str(), "GitHub.push_branch");
}

#[test]
fn deserialization_cannot_bypass_the_operation_check() {
    let empty = r#"{"id":"GitHub","operations":[]}"#;
    let err = serde_json::from_str::<ConnectorDescriptor>(empty)
        .expect_err("反序列化不得绕过 §125 的空操作检查");
    assert!(err.to_string().contains("未声明任何操作"), "实际: {err}");

    let bad_op = r#"{"id":"GitHub","operations":["GitHub"]}"#;
    assert!(
        serde_json::from_str::<ConnectorDescriptor>(bad_op).is_err(),
        "反序列化不得接受形状非法的操作标识"
    );
}

#[test]
fn descriptor_round_trips_through_serde() {
    let d = ConnectorDescriptor::new(github(), vec![op("GitHub.push_branch")])
        .expect("非空操作列表应被接受");
    let text = serde_json::to_string(&d).expect("可序列化");
    let back: ConnectorDescriptor = serde_json::from_str(&text).expect("可反序列化");
    assert_eq!(back, d);
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test -p continuum-core --test connector_descriptor -v`
Expected: FAIL，编译错误 `unresolved import continuum_core::connector`

- [ ] **Step 3: 实现 core 类型**

`crates/continuum-core/src/error.rs`

```rust
//! Provider 边界上的错误类型。core 不引入 I/O，错误只承载信息。

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProviderError {
    #[error("provider 不可用: {0}")]
    Unavailable(String),
    #[error("模型不存在: {0}")]
    UnknownModel(String),
    #[error("调用已取消: {0}")]
    Cancelled(String),
    #[error("传输失败: {0}")]
    Transport(String),
    #[error("协议错误: {0}")]
    Protocol(String),
}

/// Connector 描述符的构造错误。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CoreError {
    #[error("Connector {connector} 未声明任何操作，退回服务级授权（§125）")]
    ConnectorWithoutOperations { connector: String },
    #[error("Connector 操作标识 {op:?} 格式非法，要求形如 GitHub.push_branch（§125）")]
    MalformedConnectorOp { op: String },
}
```

`crates/continuum-core/src/model.rs`

```rust
//! §315 ModelProvider 的接口类型。字段取最小集合，P3 按需扩展。

use crate::error::ProviderError;
use futures_core::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelId(String);

impl ModelId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelDescriptor {
    pub id: ModelId,
    pub provider: String,
    pub display_name: String,
    pub context_window: u32,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvokeRequest {
    pub model: ModelId,
    pub messages: Vec<Message>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvokeResponse {
    pub model: ModelId,
    pub content: String,
    pub usage: Usage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamChunk {
    pub delta: String,
    pub done: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CallId(String);

impl CallId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderHealth {
    Healthy,
    Degraded,
    Unavailable,
}

/// §315 stream() 的返回值。
///
/// 用装箱 Stream 而非回调：调用方在异步上下文中消费分片，
/// 取消经 `CallId` 走 `cancel()`，不经流的 drop。
pub struct ModelStream {
    pub call: CallId,
    pub chunks: Pin<Box<dyn Stream<Item = Result<StreamChunk, ProviderError>> + Send>>,
}
```

`crates/continuum-core/src/tool.rs`

```rust
//! §316 ToolProvider 的接口类型。

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ToolId(String);

impl ToolId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolDescriptor {
    pub id: ToolId,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolInvocation {
    pub tool: ToolId,
    pub input: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolResult {
    pub output: Value,
    pub is_error: bool,
}
```

`crates/continuum-core/src/connector.rs`

```rust
//! §124 / §125 Connector 的接口类型。
//!
//! §125 要求按操作细分，不得退化为服务级授权。该约束对三条构造路径
//! 全部生效：`new()`、结构体字面量、以及反序列化。后者靠字段私有
//! 加 `try_from` 回流到 `new()` 实现。

use crate::error::CoreError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectorId(String);

impl ConnectorId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 操作标识，形如 `GitHub.push_branch`（§125）。
///
/// 空串与不含 `.` 的标识一律拒绝：前者不构成操作，
/// 后者只能表达服务级授权。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct ConnectorOp(String);

impl ConnectorOp {
    pub fn new(op: impl Into<String>) -> Result<Self, CoreError> {
        let op = op.into();
        if op.is_empty() || !op.contains('.') {
            return Err(CoreError::MalformedConnectorOp { op });
        }
        Ok(Self(op))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ConnectorOp {
    type Error = CoreError;

    fn try_from(op: String) -> Result<Self, CoreError> {
        ConnectorOp::new(op)
    }
}

/// 字段私有，`new` 是唯一构造入口。反序列化经 `try_from`
/// 回流到 `new`，因此三条构造路径都受 §125 约束。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RawConnectorDescriptor")]
pub struct ConnectorDescriptor {
    id: ConnectorId,
    operations: Vec<ConnectorOp>,
}

#[derive(Deserialize)]
struct RawConnectorDescriptor {
    id: ConnectorId,
    operations: Vec<ConnectorOp>,
}

impl TryFrom<RawConnectorDescriptor> for ConnectorDescriptor {
    type Error = CoreError;

    fn try_from(raw: RawConnectorDescriptor) -> Result<Self, CoreError> {
        ConnectorDescriptor::new(raw.id, raw.operations)
    }
}

impl ConnectorDescriptor {
    /// 操作列表为空时返回 `CoreError::ConnectorWithoutOperations`。
    pub fn new(id: ConnectorId, operations: Vec<ConnectorOp>) -> Result<Self, CoreError> {
        if operations.is_empty() {
            return Err(CoreError::ConnectorWithoutOperations {
                connector: id.as_str().to_owned(),
            });
        }
        Ok(Self { id, operations })
    }

    pub fn id(&self) -> &ConnectorId {
        &self.id
    }

    pub fn operations(&self) -> &[ConnectorOp] {
        &self.operations
    }
}
```

`crates/continuum-core/src/lib.rs`

```rust
//! Continuum 核心类型。本 crate 不含 I/O。

pub mod connector;
pub mod error;
pub mod model;
pub mod tool;

pub use error::{CoreError, ProviderError};
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test -p continuum-core -v`
Expected: PASS，5 passed。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-core
git commit -m "feat(core): 定义 §315/§316/§124 的接口类型"
```

---

### Task 3: continuum-provider 的三个 trait

**Files:**
- Create: `crates/continuum-provider/src/model.rs`
- Create: `crates/continuum-provider/src/tool.rs`
- Create: `crates/continuum-provider/src/connector.rs`
- Modify: `crates/continuum-provider/src/lib.rs`
- Test: `crates/continuum-provider/tests/fake_provider.rs`

**Interfaces:**
- Consumes: Task 2 的 `continuum_core::model`、`tool`、`connector`、`ProviderError`
- Produces:
  - `continuum_provider::model::ModelProvider`
  - `continuum_provider::tool::ToolProvider`
  - `continuum_provider::connector::Connector`

方法集固定为 §315 / §316 的清单，不得增删。

- [ ] **Step 1: 写假 Provider 测试**

`crates/continuum-provider/tests/fake_provider.rs`

```rust
use async_trait::async_trait;
use continuum_core::connector::{ConnectorDescriptor, ConnectorId, ConnectorOp};
use continuum_core::model::{
    CallId, InvokeRequest, InvokeResponse, ModelDescriptor, ModelId, ModelStream, ProviderHealth,
    StreamChunk, Usage,
};
use continuum_core::tool::{ToolDescriptor, ToolId, ToolInvocation, ToolResult};
use continuum_core::ProviderError;
use continuum_provider::connector::Connector;
use continuum_provider::model::ModelProvider;
use continuum_provider::tool::ToolProvider;
use futures_core::Stream;
use serde_json::json;
use std::pin::Pin;
use std::task::{Context, Poll};

/// 测试用单分片流。`futures-core` 只提供 trait 与类型别名，不提供构造函数，
/// `stream::iter` 属于 `futures-util`。为不引入第五个外部依赖，在此手写。
struct OnceStream(Option<Result<StreamChunk, ProviderError>>);

impl Stream for OnceStream {
    type Item = Result<StreamChunk, ProviderError>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(self.0.take())
    }
}

struct FakeModel;

#[async_trait]
impl ModelProvider for FakeModel {
    async fn list_models(&self) -> Result<Vec<ModelDescriptor>, ProviderError> {
        Ok(vec![ModelDescriptor {
            id: ModelId::new("fake-1"),
            provider: "fake".into(),
            display_name: "Fake One".into(),
            context_window: 8192,
            capabilities: vec!["text".into()],
        }])
    }

    async fn describe_model(&self, id: &ModelId) -> Result<ModelDescriptor, ProviderError> {
        let all = self.list_models().await?;
        all.into_iter()
            .find(|m| &m.id == id)
            .ok_or_else(|| ProviderError::UnknownModel(id.as_str().to_owned()))
    }

    async fn invoke(&self, request: InvokeRequest) -> Result<InvokeResponse, ProviderError> {
        Ok(InvokeResponse {
            model: request.model,
            content: "pong".into(),
            usage: Usage {
                input_tokens: 1,
                output_tokens: 1,
            },
        })
    }

    async fn stream(&self, _request: InvokeRequest) -> Result<ModelStream, ProviderError> {
        let chunks: Pin<Box<dyn Stream<Item = Result<StreamChunk, ProviderError>> + Send>> =
            Box::pin(OnceStream(Some(Ok(StreamChunk {
                delta: "pong".into(),
                done: true,
            }))));
        Ok(ModelStream {
            call: CallId::new("call-1"),
            chunks,
        })
    }

    async fn cancel(&self, _call: &CallId) -> Result<(), ProviderError> {
        Ok(())
    }

    async fn usage(&self) -> Result<Usage, ProviderError> {
        Ok(Usage {
            input_tokens: 0,
            output_tokens: 0,
        })
    }

    async fn health(&self) -> ProviderHealth {
        ProviderHealth::Healthy
    }
}

struct FakeTool;

#[async_trait]
impl ToolProvider for FakeTool {
    async fn list_tools(&self) -> Result<Vec<ToolDescriptor>, ProviderError> {
        Ok(vec![ToolDescriptor {
            id: ToolId::new("echo"),
            description: "回显输入".into(),
            input_schema: json!({"type": "object"}),
        }])
    }

    async fn describe_tool(&self, id: &ToolId) -> Result<ToolDescriptor, ProviderError> {
        self.list_tools()
            .await?
            .into_iter()
            .find(|t| &t.id == id)
            .ok_or_else(|| ProviderError::Unavailable(id.as_str().to_owned()))
    }

    async fn invoke(&self, call: ToolInvocation) -> Result<ToolResult, ProviderError> {
        Ok(ToolResult {
            output: call.input,
            is_error: false,
        })
    }

    async fn cancel(&self, _call: &CallId) -> Result<(), ProviderError> {
        Ok(())
    }
}

struct FakeConnector;

#[async_trait]
impl Connector for FakeConnector {
    fn descriptor(&self) -> ConnectorDescriptor {
        ConnectorDescriptor::new(
            ConnectorId::new("GitHub"),
            vec![ConnectorOp::new("GitHub.push_branch").expect("合法操作标识")],
        )
        .expect("非空操作列表")
    }

    async fn invoke(
        &self,
        op: &ConnectorOp,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, ProviderError> {
        if !self.descriptor().operations().iter().any(|o| o == op) {
            return Err(ProviderError::Protocol(format!(
                "未声明的操作: {}",
                op.as_str()
            )));
        }
        Ok(input)
    }
}

#[tokio::test]
async fn fake_implementations_satisfy_the_frozen_interfaces() {
    let m = FakeModel;
    assert_eq!(m.list_models().await.unwrap().len(), 1);
    assert_eq!(
        m.describe_model(&ModelId::new("fake-1")).await.unwrap().provider,
        "fake"
    );
    assert!(matches!(
        m.describe_model(&ModelId::new("nope")).await,
        Err(ProviderError::UnknownModel(_))
    ));
    assert_eq!(m.health().await, ProviderHealth::Healthy);

    let t = FakeTool;
    assert_eq!(t.list_tools().await.unwrap().len(), 1);

    let c = FakeConnector;
    assert_eq!(c.descriptor().operations().len(), 1);
    assert!(c
        .invoke(
            &ConnectorOp::new("GitHub.merge").expect("合法操作标识"),
            json!({})
        )
        .await
        .is_err());
    assert!(c
        .invoke(
            &ConnectorOp::new("GitHub.push_branch").expect("合法操作标识"),
            json!({"b": "x"})
        )
        .await
        .is_ok());
}
```

`tokio` 需要加到 `continuum-provider` 的 dev-dependencies。

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test -p continuum-provider -v`
Expected: FAIL，编译错误 `unresolved import continuum_provider::model`

- [ ] **Step 3: 实现三个 trait**

`crates/continuum-provider/src/model.rs`

```rust
//! §315 Provider Adapter 的 ModelProvider。
//!
//! 方法集固定为 §315 的清单：list_models / describe_model / invoke / stream / cancel / usage / health。
//! §80 的 discover / capabilities 已裁决为以 §315 为准，见 P0 设计第 7.1 节。

use async_trait::async_trait;
use continuum_core::model::{
    CallId, InvokeRequest, InvokeResponse, ModelDescriptor, ModelId, ModelStream, ProviderHealth,
    Usage,
};
use continuum_core::ProviderError;

#[async_trait]
pub trait ModelProvider: Send + Sync {
    async fn list_models(&self) -> Result<Vec<ModelDescriptor>, ProviderError>;
    async fn describe_model(&self, id: &ModelId) -> Result<ModelDescriptor, ProviderError>;
    async fn invoke(&self, request: InvokeRequest) -> Result<InvokeResponse, ProviderError>;
    async fn stream(&self, request: InvokeRequest) -> Result<ModelStream, ProviderError>;
    async fn cancel(&self, call: &CallId) -> Result<(), ProviderError>;
    async fn usage(&self) -> Result<Usage, ProviderError>;
    async fn health(&self) -> ProviderHealth;
}
```

`crates/continuum-provider/src/tool.rs`

```rust
//! §316 Tool Adapter 的 ToolProvider。

use async_trait::async_trait;
use continuum_core::model::CallId;
use continuum_core::tool::{ToolDescriptor, ToolId, ToolInvocation, ToolResult};
use continuum_core::ProviderError;

#[async_trait]
pub trait ToolProvider: Send + Sync {
    async fn list_tools(&self) -> Result<Vec<ToolDescriptor>, ProviderError>;
    async fn describe_tool(&self, id: &ToolId) -> Result<ToolDescriptor, ProviderError>;
    async fn invoke(&self, call: ToolInvocation) -> Result<ToolResult, ProviderError>;
    async fn cancel(&self, call: &CallId) -> Result<(), ProviderError>;
}
```

`crates/continuum-provider/src/connector.rs`

```rust
//! §124 Service Connector。
//!
//! §124 未给方法清单，§125 约束授权粒度：按操作细分，不得退化为服务级授权。
//! 因此 trait 没有服务级入口，每次调用必须携带 `ConnectorOp`。

use async_trait::async_trait;
use continuum_core::connector::{ConnectorDescriptor, ConnectorOp};
use continuum_core::ProviderError;
use serde_json::Value;

#[async_trait]
pub trait Connector: Send + Sync {
    fn descriptor(&self) -> ConnectorDescriptor;

    /// `op` 必须是 `descriptor().operations` 中的一项。
    async fn invoke(&self, op: &ConnectorOp, input: Value) -> Result<Value, ProviderError>;
}
```

`crates/continuum-provider/src/lib.rs`

```rust
//! 外围中立接口。本 crate 只定义 trait，不含实现，也不引用任何实现类型（§346）。

pub mod connector;
pub mod model;
pub mod tool;

pub use connector::Connector;
pub use model::ModelProvider;
pub use tool::ToolProvider;
```

在 `crates/continuum-provider/Cargo.toml` 追加：

```toml
[dev-dependencies]
tokio = { workspace = true, features = ["rt-multi-thread", "macros"] }
futures-core = { workspace = true }
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test -p continuum-provider -v`
Expected: PASS，1 passed。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-provider
git commit -m "feat(provider): 冻结 §315/§316/§124 三个 trait"
```

---

### Task 4: Event Stream 的事件信封与九类事件

**Files:**
- Create: `crates/continuum-events/src/event.rs`
- Modify: `crates/continuum-events/src/lib.rs`
- Test: `crates/continuum-events/tests/event_envelope.rs`

**Interfaces:**
- Consumes: 无
- Produces:
  - `continuum_events::EventType`，九个变体，`EventType::ALL`、`EventType::as_str()`
  - `continuum_events::Event`：`event_id`、`event_type`、`schema_version`、`occurred_at`、`intent_id`、`node_id`、`ignorable`、`payload`
  - `continuum_events::CURRENT_SCHEMA_VERSION: u32 = 1`
  - `continuum_events::Event::new(event_type: EventType, event_id: impl Into<String>, occurred_at: i64, payload: serde_json::Value) -> Event`
  - `continuum_events::EventCodecChain`：`register(version, decoder)`、`validate_contiguous()`、`decode(version, json)`
  - `continuum_events::decode_event(json: &str) -> Result<DecodedEvent, EventLogError>`
  - `continuum_events::DecodedEvent`：`Event(Event)`、`Skippable(SkipReason)`
  - `continuum_events::SkipReason`：`UnknownIgnorableType { event_type }`、`MalformedPayload { message }`
  - `continuum_events::EventLogError`：`UnknownEventType { event_type }`、`CodecChainGap { missing }`

- [ ] **Step 1: 写信封与兼容性测试**

`crates/continuum-events/tests/event_envelope.rs`

```rust
use continuum_events::{
    decode_event, DecodedEvent, Event, EventCodecChain, EventLogError, EventType, SkipReason,
    CURRENT_SCHEMA_VERSION,
};
use serde_json::json;

#[test]
fn nine_event_types_match_spec_310() {
    let names: Vec<&str> = EventType::ALL.iter().map(|t| t.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "intent.created",
            "plan.review_required",
            "node.started",
            "node.completed",
            "artifact.created",
            "verification.failed",
            "decision.required",
            "effect.committed",
            "intent.completed",
        ]
    );
}

#[test]
fn all_event_types_round_trip() {
    for t in EventType::ALL {
        let ev = Event::new(t, "ev-1", 1_700_000_000_000, json!({"k": "v"}));
        let text = serde_json::to_string(&ev).expect("可序列化");
        let back: Event = serde_json::from_str(&text).expect("可反序列化");
        assert_eq!(back, ev, "事件 {t:?} 往返不一致");
        assert_eq!(back.schema_version, CURRENT_SCHEMA_VERSION);
    }
}

#[test]
fn missing_optional_fields_deserialize_as_none() {
    let text = r#"{
        "event_id": "ev-1",
        "event_type": "node.started",
        "schema_version": 1,
        "occurred_at": 1700000000000,
        "payload": {}
    }"#;
    let ev: Event = serde_json::from_str(text).expect("缺省可选字段应可反序列化");
    assert_eq!(ev.intent_id, None);
    assert_eq!(ev.node_id, None);
    assert_eq!(ev.event_type, EventType::NodeStarted);
}

#[test]
fn unknown_optional_field_does_not_break_deserialization() {
    let text = r#"{
        "event_id": "ev-1",
        "event_type": "node.started",
        "schema_version": 1,
        "occurred_at": 1700000000000,
        "intent_id": "i-1",
        "node_id": "n-1",
        "payload": {},
        "future_field": 42
    }"#;
    let ev: Event = serde_json::from_str(text).expect("新增可选字段不得破坏既有反序列化");
    assert_eq!(ev.intent_id.as_deref(), Some("i-1"));
}

#[test]
fn serde_names_match_as_str_for_all_nine_types() {
    for t in EventType::ALL {
        let wire = serde_json::to_string(&t).expect("可序列化");
        assert_eq!(
            wire,
            format!("\"{}\"", t.as_str()),
            "serde rename 与 as_str 不一致: {t:?}"
        );
    }
}

#[test]
fn nine_types_are_never_ignorable() {
    for t in EventType::ALL {
        let ev = Event::new(t, "ev-1", 1, json!({}));
        assert!(!ev.ignorable, "既有事件类型 {t:?} 的 ignorable 必须为 false");
    }
}

#[test]
fn unknown_non_ignorable_type_is_fatal() {
    let text = r#"{"event_id":"e","event_type":"future.thing","schema_version":1,
                   "occurred_at":1,"ignorable":false,"payload":{}}"#;
    match decode_event(text) {
        Err(EventLogError::UnknownEventType { event_type }) => {
            assert_eq!(event_type, "future.thing")
        }
        other => panic!("未知且非 ignorable 的类型必须致命，实际 {other:?}"),
    }
}

#[test]
fn unknown_ignorable_type_is_skippable() {
    let text = r#"{"event_id":"e","event_type":"future.thing","schema_version":1,
                   "occurred_at":1,"ignorable":true,"payload":{}}"#;
    match decode_event(text) {
        Ok(DecodedEvent::Skippable(SkipReason::UnknownIgnorableType { event_type })) => {
            assert_eq!(event_type, "future.thing")
        }
        other => panic!("未知且 ignorable 的类型必须可跳过，实际 {other:?}"),
    }
}

#[test]
fn malformed_payload_is_skippable() {
    // schema_version 给了字符串而非整数，解码失败但不是未知类型
    let broken = r#"{"event_id":"e","event_type":"node.started","schema_version":"one",
                     "occurred_at":1,"ignorable":false,"payload":{}}"#;
    match decode_event(broken) {
        Ok(DecodedEvent::Skippable(SkipReason::MalformedPayload { .. })) => {}
        other => panic!("畸形 payload 必须可跳过，实际 {other:?}"),
    }
}

#[test]
fn codec_chain_rejects_gaps() {
    let mut chain = EventCodecChain::new();
    chain.register(1, |_| Ok(DecodedEvent::Skippable(SkipReason::MalformedPayload {
        message: "占位解码器".into(),
    })));
    chain.register(3, |_| Ok(DecodedEvent::Skippable(SkipReason::MalformedPayload {
        message: "占位解码器".into(),
    })));
    match chain.validate_contiguous() {
        Err(EventLogError::CodecChainGap { missing }) => assert_eq!(missing, 2),
        other => panic!("版本号缺口必须被拒绝，实际 {other:?}"),
    }
}

#[test]
fn codec_chain_accepts_contiguous_versions() {
    let mut chain = EventCodecChain::new();
    chain.register(1, |_| Ok(DecodedEvent::Skippable(SkipReason::MalformedPayload {
        message: "占位解码器".into(),
    })));
    chain.register(2, |_| Ok(DecodedEvent::Skippable(SkipReason::MalformedPayload {
        message: "占位解码器".into(),
    })));
    chain.validate_contiguous().expect("相邻版本应通过校验");
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test -p continuum-events -v`
Expected: FAIL，编译错误 `unresolved import continuum_events`

- [ ] **Step 3: 实现事件类型**

`crates/continuum-events/src/event.rs`

```rust
//! §310 Event Stream 的事件信封。
//!
//! 信封字段与版本策略取 P0 设计第 5 节的 v0.1 默认，
//! A12 / OPEN-006 的完整答案仍开放。

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// 九个事件类型，名称逐字符取 §310。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventType {
    #[serde(rename = "intent.created")]
    IntentCreated,
    #[serde(rename = "plan.review_required")]
    PlanReviewRequired,
    #[serde(rename = "node.started")]
    NodeStarted,
    #[serde(rename = "node.completed")]
    NodeCompleted,
    #[serde(rename = "artifact.created")]
    ArtifactCreated,
    #[serde(rename = "verification.failed")]
    VerificationFailed,
    #[serde(rename = "decision.required")]
    DecisionRequired,
    #[serde(rename = "effect.committed")]
    EffectCommitted,
    #[serde(rename = "intent.completed")]
    IntentCompleted,
}

impl EventType {
    pub const ALL: [EventType; 9] = [
        EventType::IntentCreated,
        EventType::PlanReviewRequired,
        EventType::NodeStarted,
        EventType::NodeCompleted,
        EventType::ArtifactCreated,
        EventType::VerificationFailed,
        EventType::DecisionRequired,
        EventType::EffectCommitted,
        EventType::IntentCompleted,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            EventType::IntentCreated => "intent.created",
            EventType::PlanReviewRequired => "plan.review_required",
            EventType::NodeStarted => "node.started",
            EventType::NodeCompleted => "node.completed",
            EventType::ArtifactCreated => "artifact.created",
            EventType::VerificationFailed => "verification.failed",
            EventType::DecisionRequired => "decision.required",
            EventType::EffectCommitted => "effect.committed",
            EventType::IntentCompleted => "intent.completed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub event_id: String,
    pub event_type: EventType,
    pub schema_version: u32,
    /// Unix 毫秒。
    pub occurred_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    /// 未知事件类型在旧构建中是否可跳过（P0 设计第 5.2 节）。
    /// 九类既有事件恒为 false；后续新增类型按需置 true。
    #[serde(default)]
    pub ignorable: bool,
    pub payload: Value,
}

impl Event {
    pub fn new(
        event_type: EventType,
        event_id: impl Into<String>,
        occurred_at: i64,
        payload: Value,
    ) -> Self {
        Self {
            event_id: event_id.into(),
            event_type,
            schema_version: CURRENT_SCHEMA_VERSION,
            occurred_at,
            intent_id: None,
            node_id: None,
            ignorable: false,
            payload,
        }
    }

    /// 只有新增的事件类型才需要置为 true。九类既有事件不得调用本方法。
    pub fn with_ignorable(mut self, ignorable: bool) -> Self {
        self.ignorable = ignorable;
        self
    }

    pub fn with_intent(mut self, intent_id: impl Into<String>) -> Self {
        self.intent_id = Some(intent_id.into());
        self
    }

    pub fn with_node(mut self, node_id: impl Into<String>) -> Self {
        self.node_id = Some(node_id.into());
        self
    }
}
```

`crates/continuum-events/src/codec.rs`

```rust
//! 事件解码与版本迁移链。
//!
//! 规则取 P0 设计第 5.1、5.2 节：版本迁移按相邻链组织，缺口链拒绝装配；
//! 未知事件类型默认为读时必需，只有 ignorable = true 才可跳过。

use crate::event::{Event, EventType};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EventLogError {
    #[error("未知事件类型 {event_type}，且未标记 ignorable")]
    UnknownEventType { event_type: String },
    #[error("解码器链缺少版本 {missing}")]
    CodecChainGap { missing: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipReason {
    UnknownIgnorableType { event_type: String },
    MalformedPayload { message: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum DecodedEvent {
    Event(Event),
    Skippable(SkipReason),
}

type Decoder = Box<dyn Fn(&str) -> Result<DecodedEvent, EventLogError> + Send + Sync>;

/// 相邻版本迁移链。装配时校验版本号无缺口。
pub struct EventCodecChain {
    decoders: BTreeMap<u32, Decoder>,
}

impl EventCodecChain {
    pub fn new() -> Self {
        Self {
            decoders: BTreeMap::new(),
        }
    }

    pub fn register<F>(&mut self, version: u32, decoder: F)
    where
        F: Fn(&str) -> Result<DecodedEvent, EventLogError> + Send + Sync + 'static,
    {
        self.decoders.insert(version, Box::new(decoder));
    }

    /// 版本号必须从 1 开始连续。返回缺口处的最小缺失版本号。
    pub fn validate_contiguous(&self) -> Result<(), EventLogError> {
        for (i, version) in self.decoders.keys().enumerate() {
            let expected = i as u32 + 1;
            if *version != expected {
                return Err(EventLogError::CodecChainGap { missing: expected });
            }
        }
        Ok(())
    }

    pub fn decode(&self, version: u32, json: &str) -> Result<DecodedEvent, EventLogError> {
        match self.decoders.get(&version) {
            Some(decoder) => decoder(json),
            None => Err(EventLogError::CodecChainGap { missing: version }),
        }
    }
}

impl Default for EventCodecChain {
    fn default() -> Self {
        Self::new()
    }
}

/// 解码一条事件记录。
///
/// 返回 `Err` 只用于致命情形：事件类型未知且未标记 `ignorable`。
/// payload 畸形返回 `Ok(Skippable(..))`；是否致命由日志级调用方判定
/// （P0 设计第 4.1 节：可跳过事件之后存在 intent.completed 时升级为致命）。
pub fn decode_event(json: &str) -> Result<DecodedEvent, EventLogError> {
    let raw: Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(e) => {
            return Ok(DecodedEvent::Skippable(SkipReason::MalformedPayload {
                message: e.to_string(),
            }))
        }
    };

    let type_str = raw.get("event_type").and_then(Value::as_str).unwrap_or("");
    let ignorable = raw.get("ignorable").and_then(Value::as_bool).unwrap_or(false);

    if !EventType::ALL.iter().any(|t| t.as_str() == type_str) {
        if ignorable {
            return Ok(DecodedEvent::Skippable(SkipReason::UnknownIgnorableType {
                event_type: type_str.to_owned(),
            }));
        }
        return Err(EventLogError::UnknownEventType {
            event_type: type_str.to_owned(),
        });
    }

    match serde_json::from_value(raw) {
        Ok(event) => Ok(DecodedEvent::Event(event)),
        Err(e) => Ok(DecodedEvent::Skippable(SkipReason::MalformedPayload {
            message: e.to_string(),
        })),
    }
}
```

`crates/continuum-events/src/lib.rs`

```rust
//! Event Stream 与 Audit Log。P0 的第二个交付物。

pub mod codec;
pub mod event;

pub use codec::{decode_event, DecodedEvent, EventCodecChain, EventLogError, SkipReason};
pub use event::{Event, EventType, CURRENT_SCHEMA_VERSION};
```

`skip_serializing_if` 与 `default` 并存是本节兼容性策略的实现：新增字段必须可选（`default`），既有字段不输出空值（`skip_serializing_if`）。两者同时改变会导致 `missing_optional_fields_deserialize_as_none` 之外的往返测试失败，不要只改一处。

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test -p continuum-events -v`
Expected: PASS，11 passed。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-events
git commit -m "feat(events): 定义 §310 事件信封与九类事件"
```

---

### Task 5: Audit Log 的哈希链

**Files:**
- Create: `crates/continuum-events/src/audit.rs`
- Modify: `crates/continuum-events/src/lib.rs`
- Test: `crates/continuum-events/tests/audit_chain.rs`

**Interfaces:**
- Consumes: 无
- Produces:
  - `continuum_events::AuditKind`，八个变体，`AuditKind::ALL`、`AuditKind::as_str()`
  - `continuum_events::AuditRecord`：`seq: i64`、`kind`、`occurred_at: i64`、`payload: Value`、`prev_hash: String`、`record_hash: String`
  - `continuum_events::GENESIS_HASH: &str`
  - `continuum_events::record_hash(prev_hash: &str, seq: i64, kind: AuditKind, occurred_at: i64, payload: &Value) -> String`
  - `continuum_events::verify_chain(records: &[AuditRecord]) -> Result<(), AuditError>`
  - `continuum_events::AuditError`

- [ ] **Step 1: 写哈希链测试**

`crates/continuum-events/tests/audit_chain.rs`

```rust
use continuum_events::audit::{
    record_hash, verify_chain, AuditKind, AuditRecord, AuditError, GENESIS_HASH,
};
use serde_json::json;

fn chain(n: usize) -> Vec<AuditRecord> {
    let mut out: Vec<AuditRecord> = Vec::new();
    for i in 1..=n {
        let prev = out.last().map(|r| r.record_hash.clone()).unwrap_or_else(|| GENESIS_HASH.to_owned());
        let kind = AuditKind::ALL[(i - 1) % AuditKind::ALL.len()];
        let occurred_at = 1_700_000_000_000 + i as i64;
        let payload = json!({"i": i});
        let h = continuum_events::audit::record_hash(&prev, i as i64, kind, occurred_at, &payload);
        out.push(AuditRecord {
            seq: i as i64,
            kind,
            occurred_at,
            payload,
            prev_hash: prev,
            record_hash: h,
        });
    }
    out
}

#[test]
fn eight_audit_kinds_match_spec_313() {
    let names: Vec<&str> = AuditKind::ALL.iter().map(|k| k.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "authority changes",
            "device join",
            "device revoke",
            "capability grants",
            "external effects",
            "contract changes",
            "user approvals",
            "runtime self-update",
        ]
    );
}

#[test]
fn untampered_chain_verifies() {
    verify_chain(&chain(8)).expect("未改动的链应通过校验");
}

#[test]
fn tampered_payload_is_detected() {
    let mut c = chain(3);
    c[1].payload = json!({"i": 999});
    match verify_chain(&c) {
        Err(AuditError::HashMismatch { seq }) => assert_eq!(seq, 2),
        other => panic!("应检出 seq=2 的哈希不匹配，实际 {other:?}"),
    }
}

#[test]
fn tampered_kind_is_detected() {
    let mut c = chain(2);
    c[0].kind = AuditKind::DeviceJoin;
    match verify_chain(&c) {
        Err(AuditError::HashMismatch { seq }) => assert_eq!(seq, 1),
        other => panic!("应检出 seq=1 的哈希不匹配，实际 {other:?}"),
    }
}

#[test]
fn broken_link_is_detected() {
    let mut c = chain(3);
    c[2].prev_hash = GENESIS_HASH.to_owned();
    match verify_chain(&c) {
        Err(AuditError::BrokenLink { seq }) => assert_eq!(seq, 3),
        other => panic!("应检出 seq=3 的断链，实际 {other:?}"),
    }
}

#[test]
fn empty_chain_verifies() {
    verify_chain(&[]).expect("空链应通过校验");
}

#[test]
fn serde_names_match_as_str_for_all_eight_kinds() {
    // Task 7 的 audit_records() 要把库里的 §313 文本反序列化回 AuditKind，
    // 写入用 as_str、读回用 serde rename，两者必须一致，
    // 否则库里的记录读不回来。往返测试是对称的，发现不了这类偏差。
    for k in AuditKind::ALL {
        let wire = serde_json::to_string(&k).expect("可序列化");
        assert_eq!(
            wire,
            format!("\"{}\"", k.as_str()),
            "serde rename 与 as_str 不一致: {k:?}"
        );
    }
}

#[test]
fn record_hash_matches_the_frozen_vector() {
    // 黄金向量锁定 record_hash 的输入顺序与大端编码。
    // 期望值由两份相互独立的实现算出并核对一致（Python hashlib 与
    // 部署后的 Rust 函数分别计算），非本实现自产；推导命令见实现报告。
    let payload = json!({"effect": "push_branch", "target": "origin/main"});
    let got = record_hash(
        GENESIS_HASH,
        1,
        AuditKind::ExternalEffects,
        1_700_000_000_000,
        &payload,
    );
    assert_eq!(
        got, "ec9b6a9a3fc381a83dd6daca1a2d9fee76d0cf3ea4bbfd26c46688eba8b39f87",
        "record_hash 的输入顺序或编码被改动；既有审计记录将全部失效"
    );
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test -p continuum-events --test audit_chain -v`
Expected: FAIL，编译错误 `unresolved import continuum_events::audit`

- [ ] **Step 3: 实现审计链**

`crates/continuum-events/src/audit.rs`

```rust
//! §313 Audit Log 与 §114 的防篡改。
//!
//! v0.1 取法：append-only、每条含 prev_hash 与 record_hash 构成哈希链、
//! 保留期不设上限、不做外部时间戳与签名。见 P0 设计第 6 节。

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// 链首的 prev_hash。
pub const GENESIS_HASH: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

/// 哈希域分隔前缀，防止与其它哈希用途串用。
const DOMAIN: &[u8] = b"continuum.audit.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AuditKind {
    #[serde(rename = "authority changes")]
    AuthorityChanges,
    #[serde(rename = "device join")]
    DeviceJoin,
    #[serde(rename = "device revoke")]
    DeviceRevoke,
    #[serde(rename = "capability grants")]
    CapabilityGrants,
    #[serde(rename = "external effects")]
    ExternalEffects,
    #[serde(rename = "contract changes")]
    ContractChanges,
    #[serde(rename = "user approvals")]
    UserApprovals,
    #[serde(rename = "runtime self-update")]
    RuntimeSelfUpdate,
}

impl AuditKind {
    pub const ALL: [AuditKind; 8] = [
        AuditKind::AuthorityChanges,
        AuditKind::DeviceJoin,
        AuditKind::DeviceRevoke,
        AuditKind::CapabilityGrants,
        AuditKind::ExternalEffects,
        AuditKind::ContractChanges,
        AuditKind::UserApprovals,
        AuditKind::RuntimeSelfUpdate,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            AuditKind::AuthorityChanges => "authority changes",
            AuditKind::DeviceJoin => "device join",
            AuditKind::DeviceRevoke => "device revoke",
            AuditKind::CapabilityGrants => "capability grants",
            AuditKind::ExternalEffects => "external effects",
            AuditKind::ContractChanges => "contract changes",
            AuditKind::UserApprovals => "user approvals",
            AuditKind::RuntimeSelfUpdate => "runtime self-update",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditRecord {
    pub seq: i64,
    pub kind: AuditKind,
    /// Unix 毫秒。
    pub occurred_at: i64,
    pub payload: Value,
    pub prev_hash: String,
    pub record_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AuditError {
    #[error("seq={seq} 的 record_hash 与重算结果不符")]
    HashMismatch { seq: i64 },
    #[error("seq={seq} 的 prev_hash 与前一条的 record_hash 不符")]
    BrokenLink { seq: i64 },
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(DIGITS[(b >> 4) as usize] as char);
        s.push(DIGITS[(b & 0x0f) as usize] as char);
    }
    s
}

/// 输入顺序固定为 域前缀 → prev_hash → seq → kind → occurred_at → payload。
/// 改变顺序会改变所有既有记录的重算结果，视为破坏兼容。
pub fn record_hash(
    prev_hash: &str,
    seq: i64,
    kind: AuditKind,
    occurred_at: i64,
    payload: &Value,
) -> String {
    let mut h = Sha256::new();
    h.update(DOMAIN);
    h.update(prev_hash.as_bytes());
    h.update(seq.to_be_bytes());
    h.update(kind.as_str().as_bytes());
    h.update(occurred_at.to_be_bytes());
    h.update(serde_json::to_vec(payload).expect("payload 可序列化"));
    hex(&h.finalize())
}

/// 校验顺序、断链与逐条哈希。空链通过。
pub fn verify_chain(records: &[AuditRecord]) -> Result<(), AuditError> {
    let mut expected_prev = GENESIS_HASH.to_owned();
    for r in records {
        if r.prev_hash != expected_prev {
            return Err(AuditError::BrokenLink { seq: r.seq });
        }
        let recomputed =
            record_hash(&r.prev_hash, r.seq, r.kind, r.occurred_at, &r.payload);
        if recomputed != r.record_hash {
            return Err(AuditError::HashMismatch { seq: r.seq });
        }
        expected_prev = r.record_hash.clone();
    }
    Ok(())
}
```

`crates/continuum-events/src/lib.rs`

```rust
//! Event Stream 与 Audit Log。P0 的第二个交付物。

pub mod audit;
pub mod event;

pub use audit::{verify_chain, AuditError, AuditKind, AuditRecord, GENESIS_HASH};
pub use event::{Event, EventType, CURRENT_SCHEMA_VERSION};
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test -p continuum-events -v`
Expected: PASS，19 passed。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-events
git commit -m "feat(events): 实现 §313 Audit Log 哈希链与篡改检测"
```

---

### Task 6: 打开数据库与迁移框架

**Files:**
- Create: `crates/continuum-persist/src/db.rs`
- Create: `crates/continuum-persist/src/error.rs`
- Create: `crates/continuum-persist/src/value.rs`
- Modify: `crates/continuum-persist/src/lib.rs`
- Test: `crates/continuum-persist/tests/migrations.rs`

**Interfaces:**
- Consumes: 无
- Produces:
  - `continuum_persist::Value`：`Null`、`Int(i64)`、`Real(f64)`、`Text(String)`、`Blob(Vec<u8>)`
  - `continuum_persist::Migration { version: i64, name: &'static str, sql: &'static str }` 与 `Migration::new`
  - `continuum_persist::builtin_migrations() -> Vec<Migration>`
  - `continuum_persist::PersistError`
  - `continuum_persist::Db::open(path) -> Result<Db, PersistError>`、`Db::open_with(path, Vec<Migration>)`、`Db::migrate(&self) -> Result<u32, PersistError>`
  - `Db` 的 `pub(crate) conn` 字段，外部 crate 不可见

- [ ] **Step 1: 写迁移框架测试**

`crates/continuum-persist/tests/migrations.rs`

```rust
use continuum_persist::{builtin_migrations, Db, Migration, Value};

#[test]
fn builtin_migrations_create_event_and_audit_tables() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let db = Db::open(&path).unwrap();
    let applied = db.migrate().unwrap();
    assert_eq!(applied, builtin_migrations().len() as u32);

    let tx = db.begin().unwrap();
    // sqlite_sequence 由 AUTOINCREMENT 自动创建，不属于本项目的表
    let rows = tx
        .query(
            "SELECT name FROM sqlite_master
             WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
            &[],
        )
        .unwrap();
    let names: Vec<String> = rows
        .into_iter()
        .map(|r| match r[0].clone() {
            Value::Text(s) => s,
            other => panic!("表名应为文本，实际 {other:?}"),
        })
        .collect();
    assert_eq!(names, vec!["audit_log", "events", "schema_migrations"]);
}

#[test]
fn migrate_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let db = Db::open(&path).unwrap();
    assert_eq!(db.migrate().unwrap(), builtin_migrations().len() as u32);
    assert_eq!(db.migrate().unwrap(), 0, "重复迁移不得重复应用");
}

#[test]
fn layer_supplied_migration_is_applied_after_builtin() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut all = builtin_migrations();
    all.push(Migration::new(
        100,
        "layer_table",
        "CREATE TABLE layer_table (id TEXT PRIMARY KEY);",
    ));
    let db = Db::open_with(&path, all).unwrap();
    let applied = db.migrate().unwrap();
    assert_eq!(applied, builtin_migrations().len() as u32 + 1);

    let tx = db.begin().unwrap();
    let rows = tx
        .query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='layer_table'",
            &[],
        )
        .unwrap();
    assert_eq!(rows.len(), 1);
}

#[test]
fn failed_migration_leaves_later_migrations_unapplied() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut all = builtin_migrations();
    all.push(Migration::new(101, "broken", "THIS IS NOT SQL;"));
    all.push(Migration::new(102, "never", "CREATE TABLE never (id TEXT);"));
    let db = Db::open_with(&path, all).unwrap();
    let err = db.migrate().expect_err("坏迁移必须返回错误");
    assert!(
        err.to_string().contains("101"),
        "错误信息应含失败版本号，实际: {err}"
    );

    let tx = db.begin().unwrap();
    let rows = tx
        .query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='never'",
            &[],
        )
        .unwrap();
    assert!(rows.is_empty(), "迁移 102 不得被应用");
}

#[test]
fn wal_mode_is_enabled() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let db = Db::open(&path).unwrap();
    let tx = db.begin().unwrap();
    let rows = tx.query("PRAGMA journal_mode", &[]).unwrap();
    match &rows[0][0] {
        Value::Text(s) => assert_eq!(s, "wal"),
        other => panic!("journal_mode 应为文本，实际 {other:?}"),
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test -p continuum-persist -v`
Expected: FAIL，编译错误 `unresolved import continuum_persist`

- [ ] **Step 3: 实现错误类型与值类型**

`crates/continuum-persist/src/error.rs`

```rust
//! 持久化错误。错误信息用字符串承载，不外泄 rusqlite 类型，
//! 这样 `continuum-persist` 的使用者不需要依赖 rusqlite。

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PersistError {
    #[error("数据库错误: {0}")]
    Database(String),
    #[error("迁移 {version} 失败: {message}")]
    Migration { version: i64, message: String },
    #[error("参数类型不匹配: 第 {index} 个参数为 {actual}")]
    ParamType { index: usize, actual: &'static str },
    #[error("结果类型不匹配: 第 {index} 列为 {actual}")]
    ColumnType { index: usize, actual: &'static str },
    #[error("事务未提交即被丢弃")]
    TransactionDropped,
}

pub(crate) fn db(e: rusqlite::Error) -> PersistError {
    PersistError::Database(e.to_string())
}
```

`crates/continuum-persist/src/value.rs`

```rust
//! 与 rusqlite 解耦的参数与结果值。

use crate::error::PersistError;
use rusqlite::types::{Value as SqlValue, ValueRef};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Int(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

impl Value {
    pub fn text(s: impl Into<String>) -> Self {
        Value::Text(s.into())
    }

    pub(crate) fn to_sql(&self) -> SqlValue {
        match self {
            Value::Null => SqlValue::Null,
            Value::Int(i) => SqlValue::Integer(*i),
            Value::Real(f) => SqlValue::Real(*f),
            Value::Text(s) => SqlValue::Text(s.clone()),
            Value::Blob(b) => SqlValue::Blob(b.clone()),
        }
    }

    pub(crate) fn from_ref(r: ValueRef<'_>) -> Self {
        match r {
            ValueRef::Null => Value::Null,
            ValueRef::Integer(i) => Value::Int(i),
            ValueRef::Real(f) => Value::Real(f),
            ValueRef::Text(t) => Value::Text(String::from_utf8_lossy(t).into_owned()),
            ValueRef::Blob(b) => Value::Blob(b.to_vec()),
        }
    }
}

pub(crate) fn params(values: &[Value]) -> Vec<SqlValue> {
    values.iter().map(Value::to_sql).collect()
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::Text(s.to_owned())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::Text(s)
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::Int(i)
    }
}

/// 供调用方在测试与断言中使用。
pub fn as_text(v: &Value) -> Result<&str, PersistError> {
    match v {
        Value::Text(s) => Ok(s),
        other => Err(PersistError::ColumnType {
            index: 0,
            actual: kind_name(other),
        }),
    }
}

pub fn kind_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Int(_) => "int",
        Value::Real(_) => "real",
        Value::Text(_) => "text",
        Value::Blob(_) => "blob",
    }
}
```

- [ ] **Step 4: 实现 Db 与迁移框架**

`crates/continuum-persist/src/db.rs`

```rust
//! 数据库打开、PRAGMA 与迁移框架。
//!
//! 连接对象是本 crate 私有字段。外部 crate 只能经 `Tx` 访问数据库，
//! 因此 §318 的事务约束在编译期成立。见 P0 设计第 4 节。

use crate::value::{params, Value};
use crate::{PersistError, Tx};
use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

/// 一次迁移。`version` 递增且唯一，`sql` 允许包含多条语句。
#[derive(Debug, Clone)]
pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub sql: &'static str,
}

impl Migration {
    pub const fn new(version: i64, name: &'static str, sql: &'static str) -> Self {
        Self { version, name, sql }
    }
}

/// P0 内置迁移：事件表与审计表。业务表由各层通过 `open_with` 追加。
pub fn builtin_migrations() -> Vec<Migration> {
    vec![
        Migration::new(
            1,
            "events",
            "CREATE TABLE events (
                event_id TEXT PRIMARY KEY,
                event_type TEXT NOT NULL,
                schema_version INTEGER NOT NULL,
                occurred_at INTEGER NOT NULL,
                intent_id TEXT,
                node_id TEXT,
                ignorable INTEGER NOT NULL DEFAULT 0,
                payload TEXT NOT NULL
            );
            CREATE INDEX idx_events_type ON events(event_type);
            CREATE INDEX idx_events_intent ON events(intent_id);
            CREATE INDEX idx_events_occurred ON events(occurred_at);",
        ),
        Migration::new(
            2,
            "audit_log",
            "CREATE TABLE audit_log (
                seq INTEGER PRIMARY KEY AUTOINCREMENT,
                kind TEXT NOT NULL,
                occurred_at INTEGER NOT NULL,
                payload TEXT NOT NULL,
                prev_hash TEXT NOT NULL,
                record_hash TEXT NOT NULL
            );
            CREATE INDEX idx_audit_kind ON audit_log(kind);",
        ),
    ]
}

pub struct Db {
    pub(crate) conn: Mutex<Connection>,
    migrations: Vec<Migration>,
}

impl Db {
    pub fn open(path: &Path) -> Result<Db, PersistError> {
        Db::open_with(path, builtin_migrations())
    }

    pub fn open_with(path: &Path, migrations: Vec<Migration>) -> Result<Db, PersistError> {
        let conn = Connection::open(path).map_err(crate::error::db)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=FULL;
             PRAGMA foreign_keys=ON;",
        )
        .map_err(crate::error::db)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at INTEGER NOT NULL
            );",
        )
        .map_err(crate::error::db)?;
        Ok(Db {
            conn: Mutex::new(conn),
            migrations,
        })
    }

    /// 按 version 升序应用未记录的迁移。每个迁移单独一个事务。
    /// 返回本次应用的迁移数。任一迁移失败时其后的迁移不应用。
    pub fn migrate(&self) -> Result<u32, PersistError> {
        let mut ordered = self.migrations.clone();
        ordered.sort_by_key(|m| m.version);

        let mut applied = 0u32;
        for m in ordered {
            let guard = self.conn.lock().unwrap_or_else(|e| e.into_inner());
            let done: i64 = guard
                .query_row(
                    "SELECT COUNT(*) FROM schema_migrations WHERE version = ?1",
                    [m.version],
                    |r| r.get(0),
                )
                .map_err(crate::error::db)?;
            if done > 0 {
                continue;
            }
            guard
                .execute_batch("BEGIN IMMEDIATE")
                .map_err(crate::error::db)?;
            let result = guard
                .execute_batch(m.sql)
                .and_then(|_| {
                    guard.execute(
                        "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
                        rusqlite::params![m.version, m.name, now_millis()],
                    )
                });
            match result {
                Ok(_) => {
                    guard.execute_batch("COMMIT").map_err(crate::error::db)?;
                    applied += 1;
                }
                Err(e) => {
                    let _ = guard.execute_batch("ROLLBACK");
                    return Err(PersistError::Migration {
                        version: m.version,
                        message: e.to_string(),
                    });
                }
            }
        }
        Ok(applied)
    }

    /// 开启写事务。同一时刻只有一个事务，第二个未提交时调用会阻塞。
    pub fn begin(&self) -> Result<Tx<'_>, PersistError> {
        Tx::begin(self)
    }
}

pub(crate) fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub(crate) fn query_rows(conn: &Connection, sql: &str, p: &[Value]) -> Result<Vec<Vec<Value>>, PersistError> {
    let mut stmt = conn.prepare(sql).map_err(crate::error::db)?;
    let bound = params(p);
    let refs: Vec<&dyn rusqlite::ToSql> = bound.iter().map(|v| v as &dyn rusqlite::ToSql).collect();
    let mut rows = stmt.query(refs.as_slice()).map_err(crate::error::db)?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().map_err(crate::error::db)? {
        let n = row.as_ref().column_count();
        let mut one = Vec::with_capacity(n);
        for i in 0..n {
            let v = row.get_ref(i).map_err(crate::error::db)?;
            one.push(Value::from_ref(v));
        }
        out.push(one);
    }
    Ok(out)
}
```

`crates/continuum-persist/src/lib.rs`

```rust
//! 持久化机制：迁移框架、事务 API、恢复钩子注册表。
//!
//! 数据库连接在本 crate 内私有，外部 crate 只能经 `Tx` 访问（见 P0 设计第 4 节）。

pub mod db;
pub mod error;
pub mod value;

mod tx;

pub use db::{builtin_migrations, Db, Migration};
pub use error::PersistError;
pub use tx::{EventLogScan, Tx};
pub use value::Value;
```

`crates/continuum-persist/src/tx.rs` 在本 task 先写最小可编译版本，Task 7 补全：

```rust
//! 事务 API。Task 7 补全。

use crate::{Db, PersistError, Value};

pub struct Tx<'a> {
    pub(crate) guard: std::sync::MutexGuard<'a, rusqlite::Connection>,
    done: bool,
}

impl<'a> Tx<'a> {
    pub(crate) fn begin(db: &'a Db) -> Result<Tx<'a>, PersistError> {
        let guard = db.conn.lock().unwrap_or_else(|e| e.into_inner());
        guard
            .execute_batch("BEGIN IMMEDIATE")
            .map_err(crate::error::db)?;
        Ok(Tx { guard, done: false })
    }

    pub fn query(&self, sql: &str, params: &[Value]) -> Result<Vec<Vec<Value>>, PersistError> {
        crate::db::query_rows(&self.guard, sql, params)
    }

    pub fn execute(&self, sql: &str, params: &[Value]) -> Result<u64, PersistError> {
        use crate::value::params as to_sql;
        let bound = to_sql(params);
        let refs: Vec<&dyn rusqlite::ToSql> =
            bound.iter().map(|v| v as &dyn rusqlite::ToSql).collect();
        self.guard
            .execute(sql, refs.as_slice())
            .map(|n| n as u64)
            .map_err(crate::error::db)
    }
}
```

- [ ] **Step 5: 运行测试确认通过**

Run: `cargo test -p continuum-persist -v`
Expected: PASS，5 passed。

- [ ] **Step 6: 提交**

```bash
git add crates/continuum-persist
git commit -m "feat(persist): 实现数据库打开、PRAGMA 与迁移框架"
```

---

### Task 7: 事务 API 与事件、审计的同事务写入

**Files:**
- Modify: `crates/continuum-persist/src/tx.rs`
- Test: `crates/continuum-persist/tests/transaction.rs`

**Interfaces:**
- Consumes: Task 4 的 `Event`、Task 5 的 `AuditRecord` 与 `record_hash`、Task 6 的 `Tx`
- Produces:
  - `Tx::commit(self) -> Result<(), PersistError>`
  - `Tx::append_event(&self, event: &Event) -> Result<(), PersistError>`
  - `Tx::append_audit(&self, kind: AuditKind, occurred_at: i64, payload: Value) -> Result<AuditRecord, PersistError>`
  - `Tx::audit_records(&self) -> Result<Vec<AuditRecord>, PersistError>`
  - `Tx::verify_audit_chain(&self) -> Result<(), PersistError>`
  - `Tx::last_audit_hash(&self) -> Result<String, PersistError>`，空链返回 `GENESIS_HASH`
  - `Tx::scan_event_log(&self) -> Result<EventLogScan, PersistError>`
  - `continuum_persist::EventLogScan { decoded: usize, skipped_records: usize }`
  - 丢弃未提交的 `Tx` 时执行 ROLLBACK

- [ ] **Step 1: 写事务测试**

`crates/continuum-persist/tests/transaction.rs`

```rust
use continuum_events::audit::{verify_chain, AuditKind, GENESIS_HASH};
use continuum_events::{Event, EventType};
use continuum_persist::{Db, Value};
use serde_json::json;

fn db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let db = Db::open(&path).unwrap();
    db.migrate().unwrap();
    (dir, db)
}

#[test]
fn committed_event_is_visible() {
    let (_d, db) = db();
    let tx = db.begin().unwrap();
    tx.append_event(&Event::new(EventType::IntentCreated, "e1", 1, json!({"a": 1})))
        .unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let rows = tx.query("SELECT event_id FROM events", &[]).unwrap();
    assert_eq!(rows.len(), 1);
}

#[test]
fn dropped_transaction_rolls_back() {
    let (_d, db) = db();
    {
        let tx = db.begin().unwrap();
        tx.append_event(&Event::new(EventType::NodeStarted, "e1", 1, json!({})))
            .unwrap();
        // 未 commit，离开作用域时回滚
    }
    let tx = db.begin().unwrap();
    let rows = tx.query("SELECT event_id FROM events", &[]).unwrap();
    assert!(rows.is_empty(), "未提交的事务必须回滚");
}

#[test]
fn event_and_audit_written_in_one_transaction_are_both_visible() {
    let (_d, db) = db();
    let tx = db.begin().unwrap();
    tx.append_event(&Event::new(
        EventType::EffectCommitted,
        "e1",
        1,
        json!({"effect": "push_branch"}),
    ))
    .unwrap();
    let rec = tx
        .append_audit(
            AuditKind::ExternalEffects,
            1,
            Value::text("push_branch"),
        )
        .unwrap();
    assert_eq!(rec.seq, 1);
    assert_eq!(rec.prev_hash, GENESIS_HASH);
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    assert_eq!(tx.query("SELECT event_id FROM events", &[]).unwrap().len(), 1);
    assert_eq!(
        tx.query("SELECT seq FROM audit_log", &[]).unwrap().len(),
        1
    );
}

#[test]
fn audit_chain_continues_across_transactions() {
    let (_d, db) = db();
    for i in 0..3 {
        let tx = db.begin().unwrap();
        tx.append_audit(
            AuditKind::ALL[i],
            100 + i as i64,
            Value::text(format!("p{i}")),
        )
        .unwrap();
        tx.commit().unwrap();
    }
    let tx = db.begin().unwrap();
    let recs = tx.audit_records().unwrap();
    assert_eq!(recs.len(), 3);
    assert_eq!(
        recs.iter().map(|r| r.seq).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    verify_chain(&recs).expect("跨事务链应连续");
    tx.verify_audit_chain().unwrap();
}

#[test]
fn tampered_audit_row_is_detected_on_read() {
    let (_d, db) = db();
    let tx = db.begin().unwrap();
    tx.append_audit(AuditKind::DeviceJoin, 1, Value::text("dev-a"))
        .unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    tx.execute(
        "UPDATE audit_log SET payload = ?1 WHERE seq = 1",
        &[Value::text("dev-b")],
    )
    .unwrap();
    let err = tx.verify_audit_chain().expect_err("改写的记录必须被检出");
    assert!(err.to_string().contains("seq=1"), "实际: {err}");
}

#[test]
fn last_audit_hash_is_genesis_on_empty_chain() {
    let (_d, db) = db();
    let tx = db.begin().unwrap();
    assert_eq!(tx.last_audit_hash().unwrap(), GENESIS_HASH);
    tx.commit().unwrap();
}

/// 直接插一行，绕过 append_event，用于构造非法记录。
fn insert_raw_event(
    tx: &continuum_persist::Tx<'_>,
    event_id: &str,
    event_type: &str,
    schema_version: Value,
    ignorable: i64,
) {
    tx.execute(
        "INSERT INTO events
           (event_id, event_type, schema_version, occurred_at, intent_id, node_id, ignorable, payload)
         VALUES (?1, ?2, ?3, ?4, NULL, NULL, ?5, ?6)",
        &[
            Value::text(event_id),
            Value::text(event_type),
            schema_version,
            Value::Int(1),
            Value::Int(ignorable),
            Value::text("{}"),
        ],
    )
    .unwrap();
}

#[test]
fn scan_counts_skippable_records() {
    let (_d, db) = db();
    let tx = db.begin().unwrap();
    tx.append_event(&Event::new(EventType::NodeStarted, "e1", 1, json!({})))
        .unwrap();
    // schema_version 写成文本，解码失败但不致命
    insert_raw_event(&tx, "e2", "node.started", Value::text("one"), 0);
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let scan = tx.scan_event_log().unwrap();
    assert_eq!(scan.decoded, 1);
    assert_eq!(scan.skipped_records, 1);
}

#[test]
fn scan_is_clean_when_all_records_decode() {
    let (_d, db) = db();
    let tx = db.begin().unwrap();
    tx.append_event(&Event::new(EventType::IntentCreated, "e1", 1, json!({})))
        .unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let scan = tx.scan_event_log().unwrap();
    assert_eq!(scan.decoded, 1);
    assert_eq!(scan.skipped_records, 0);
}

#[test]
fn unknown_event_type_is_fatal_on_scan() {
    let (_d, db) = db();
    let tx = db.begin().unwrap();
    insert_raw_event(&tx, "e1", "future.thing", Value::Int(1), 0);
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let err = tx
        .scan_event_log()
        .expect_err("未知且非 ignorable 的类型必须致命");
    assert!(err.to_string().contains("future.thing"), "实际: {err}");
}

#[test]
fn skippable_record_before_intent_completed_is_fatal() {
    let (_d, db) = db();
    let tx = db.begin().unwrap();
    // 一条解码失败的记录
    insert_raw_event(&tx, "e1", "node.started", Value::text("one"), 0);
    // 其后存在收口事件
    tx.append_event(&Event::new(EventType::IntentCompleted, "e2", 2, json!({})))
        .unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let err = tx
        .scan_event_log()
        .expect_err("收口前的记录不可读时必须致命");
    assert!(err.to_string().contains("intent.completed"), "实际: {err}");
}

#[test]
fn skippable_record_after_intent_completed_is_only_counted() {
    let (_d, db) = db();
    let tx = db.begin().unwrap();
    tx.append_event(&Event::new(EventType::IntentCompleted, "e1", 1, json!({})))
        .unwrap();
    insert_raw_event(&tx, "e2", "node.started", Value::text("one"), 0);
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let scan = tx
        .scan_event_log()
        .expect("收口之后的坏记录不升级为致命");
    assert_eq!(scan.skipped_records, 1);
}
```

`audit_records` 需要把行还原为 `AuditRecord`，因此 `Tx` 必须能读 `payload` 文本并反序列化。`kind` 从 §313 字符串反序列化。

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test -p continuum-persist --test transaction -v`
Expected: FAIL，编译错误 `no method named append_event`

- [ ] **Step 3: 补全事务 API**

`crates/continuum-persist/src/tx.rs` 全文替换为：

```rust
//! 事务 API。
//!
//! 状态更新与对应事件必须在同一事务内提交（§318）。
//! 由于数据库连接是 `Db` 的私有字段，外部 crate 只能经本类型写入。

use crate::value::Value;
use crate::{Db, PersistError};
use continuum_events::audit::{record_hash, AuditKind, AuditRecord, GENESIS_HASH};
use continuum_events::{DecodedEvent, Event};
use serde_json::Value as JsonValue;

pub struct Tx<'a> {
    pub(crate) guard: std::sync::MutexGuard<'a, rusqlite::Connection>,
    done: bool,
}

impl<'a> Tx<'a> {
    pub(crate) fn begin(db: &'a Db) -> Result<Tx<'a>, PersistError> {
        let guard = db.conn.lock().unwrap_or_else(|e| e.into_inner());
        guard
            .execute_batch("BEGIN IMMEDIATE")
            .map_err(crate::error::db)?;
        Ok(Tx { guard, done: false })
    }

    pub fn query(&self, sql: &str, params: &[Value]) -> Result<Vec<Vec<Value>>, PersistError> {
        crate::db::query_rows(&self.guard, sql, params)
    }

    pub fn execute(&self, sql: &str, params: &[Value]) -> Result<u64, PersistError> {
        let bound = crate::value::params(params);
        let refs: Vec<&dyn rusqlite::ToSql> =
            bound.iter().map(|v| v as &dyn rusqlite::ToSql).collect();
        self.guard
            .execute(sql, refs.as_slice())
            .map(|n| n as u64)
            .map_err(crate::error::db)
    }

    /// 提交事务。消费 `self`，之后不能再用。
    pub fn commit(mut self) -> Result<(), PersistError> {
        self.guard
            .execute_batch("COMMIT")
            .map_err(crate::error::db)?;
        self.done = true;
        Ok(())
    }

    pub fn append_event(&self, event: &Event) -> Result<(), PersistError> {
        let payload =
            serde_json::to_string(&event.payload).map_err(|e| PersistError::Database(e.to_string()))?;
        self.execute(
            "INSERT INTO events (event_id, event_type, schema_version, occurred_at, intent_id, node_id, payload)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            &[
                Value::text(event.event_id.clone()),
                Value::text(event.event_type.as_str()),
                Value::Int(event.schema_version as i64),
                Value::Int(event.occurred_at),
                match &event.intent_id {
                    Some(s) => Value::text(s),
                    None => Value::Null,
                },
                match &event.node_id {
                    Some(s) => Value::text(s),
                    None => Value::Null,
                },
                Value::text(payload),
            ],
        )?;
        Ok(())
    }

    /// 追加一条审计记录。seq 与 prev_hash 由本方法从当前链尾推出，
    /// 调用方不指定，避免链断裂。
    pub fn append_audit(
        &self,
        kind: AuditKind,
        occurred_at: i64,
        payload: Value,
    ) -> Result<AuditRecord, PersistError> {
        let prev_hash = self.last_audit_hash()?;
        let seq: i64 = self
            .query("SELECT COALESCE(MAX(seq), 0) + 1 FROM audit_log", &[])?
            .into_iter()
            .next()
            .and_then(|r| match r.into_iter().next() {
                Some(Value::Int(i)) => Some(i),
                _ => None,
            })
            .unwrap_or(1);
        let payload_json = match &payload {
            Value::Text(s) => {
                serde_json::from_str::<JsonValue>(s).unwrap_or_else(|_| JsonValue::String(s.clone()))
            }
            Value::Int(i) => JsonValue::from(*i),
            Value::Real(f) => JsonValue::from(*f),
            Value::Null => JsonValue::Null,
            Value::Blob(_) => {
                return Err(PersistError::ParamType {
                    index: 2,
                    actual: "blob",
                })
            }
        };
        let hash = record_hash(&prev_hash, seq, kind, occurred_at, &payload_json);
        self.execute(
            "INSERT INTO audit_log (seq, kind, occurred_at, payload, prev_hash, record_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            &[
                Value::Int(seq),
                Value::text(kind.as_str()),
                Value::Int(occurred_at),
                Value::text(serde_json::to_string(&payload_json).expect("payload 可序列化")),
                Value::text(prev_hash.clone()),
                Value::text(hash.clone()),
            ],
        )?;
        Ok(AuditRecord {
            seq,
            kind,
            occurred_at,
            payload: payload_json,
            prev_hash,
            record_hash: hash,
        })
    }

    /// 当前链尾的 record_hash。空链返回 `GENESIS_HASH`。
    pub fn last_audit_hash(&self) -> Result<String, PersistError> {
        let rows = self.query(
            "SELECT record_hash FROM audit_log ORDER BY seq DESC LIMIT 1",
            &[],
        )?;
        match rows.into_iter().next().and_then(|r| r.into_iter().next()) {
            Some(Value::Text(s)) => Ok(s),
            Some(other) => Err(PersistError::ColumnType {
                index: 0,
                actual: crate::value::kind_name(&other),
            }),
            None => Ok(GENESIS_HASH.to_owned()),
        }
    }

    pub fn audit_records(&self) -> Result<Vec<AuditRecord>, PersistError> {
        let rows = self.query(
            "SELECT seq, kind, occurred_at, payload, prev_hash, record_hash
             FROM audit_log ORDER BY seq ASC",
            &[],
        )?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let mut it = row.into_iter();
            let seq = match it.next() {
                Some(Value::Int(i)) => i,
                other => {
                    return Err(PersistError::ColumnType {
                        index: 0,
                        actual: other.as_ref().map(crate::value::kind_name).unwrap_or("missing"),
                    })
                }
            };
            let kind_text = match it.next() {
                Some(Value::Text(s)) => s,
                other => {
                    return Err(PersistError::ColumnType {
                        index: 1,
                        actual: other.as_ref().map(crate::value::kind_name).unwrap_or("missing"),
                    })
                }
            };
            let occurred_at = match it.next() {
                Some(Value::Int(i)) => i,
                other => {
                    return Err(PersistError::ColumnType {
                        index: 2,
                        actual: other.as_ref().map(crate::value::kind_name).unwrap_or("missing"),
                    })
                }
            };
            let payload_text = match it.next() {
                Some(Value::Text(s)) => s,
                other => {
                    return Err(PersistError::ColumnType {
                        index: 3,
                        actual: other.as_ref().map(crate::value::kind_name).unwrap_or("missing"),
                    })
                }
            };
            let prev_hash = match it.next() {
                Some(Value::Text(s)) => s,
                other => {
                    return Err(PersistError::ColumnType {
                        index: 4,
                        actual: other.as_ref().map(crate::value::kind_name).unwrap_or("missing"),
                    })
                }
            };
            let record_hash = match it.next() {
                Some(Value::Text(s)) => s,
                other => {
                    return Err(PersistError::ColumnType {
                        index: 5,
                        actual: other.as_ref().map(crate::value::kind_name).unwrap_or("missing"),
                    })
                }
            };
            let kind: AuditKind = serde_json::from_value(JsonValue::String(kind_text.clone()))
                .map_err(|_| PersistError::Database(format!("未知审计类型: {kind_text}")))?;
            let payload: JsonValue = serde_json::from_str(&payload_text)
                .map_err(|e| PersistError::Database(e.to_string()))?;
            out.push(AuditRecord {
                seq,
                kind,
                occurred_at,
                payload,
                prev_hash,
                record_hash,
            });
        }
        Ok(out)
    }

    /// 重算整条链。改写任一记录后本条或其后任一条的哈希不匹配。
    pub fn verify_audit_chain(&self) -> Result<(), PersistError> {
        let records = self.audit_records()?;
        continuum_events::audit::verify_chain(&records)
            .map_err(|e| PersistError::Database(e.to_string()))
    }
}

impl Drop for Tx<'_> {
    fn drop(&mut self) {
        if !self.done {
            let _ = self.guard.execute_batch("ROLLBACK");
        }
    }
}

/// 事件日志扫描结果（P0 设计第 4.1 节）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventLogScan {
    pub decoded: usize,
    pub skipped_records: usize,
}

fn text_at(row: &[Value], index: usize) -> Result<String, PersistError> {
    match row.get(index) {
        Some(Value::Text(s)) => Ok(s.clone()),
        Some(other) => Err(PersistError::ColumnType {
            index,
            actual: crate::value::kind_name(other),
        }),
        None => Err(PersistError::ColumnType {
            index,
            actual: "missing",
        }),
    }
}

/// 宽松取值：列里存了非预期类型时按 JSON 原样交给解码器，
/// 由解码器判定可跳过还是致命。schema_version 被写成文本即走这条路径。
fn json_at(row: &[Value], index: usize) -> JsonValue {
    match row.get(index) {
        Some(Value::Int(i)) => JsonValue::from(*i),
        Some(Value::Real(f)) => JsonValue::from(*f),
        Some(Value::Text(s)) => JsonValue::from(s.clone()),
        Some(Value::Null) | Some(Value::Blob(_)) | None => JsonValue::Null,
    }
}

impl Tx<'_> {
    /// 按 rowid 顺序解码事件日志，实现 P0 设计第 4.1 节的判定规则。
    ///
    /// 致命：未知且未标记 ignorable 的事件类型；
    ///       可跳过事件之后存在 intent.completed。
    /// 可跳过：payload 无法解码的记录，计入 `skipped_records`。
    pub fn scan_event_log(&self) -> Result<EventLogScan, PersistError> {
        let rows = self.query(
            "SELECT event_id, event_type, schema_version, occurred_at, intent_id, node_id,
                    ignorable, payload
             FROM events ORDER BY rowid ASC",
            &[],
        )?;

        let mut decoded = 0usize;
        let mut skipped_records = 0usize;
        let mut saw_skip = false;

        for row in &rows {
            let event_type = text_at(row, 1)?;
            let envelope = serde_json::json!({
                "event_id": text_at(row, 0)?,
                "event_type": event_type,
                "schema_version": json_at(row, 2),
                "occurred_at": json_at(row, 3),
                "intent_id": json_at(row, 4),
                "node_id": json_at(row, 5),
                "ignorable": json_at(row, 6),
                "payload": serde_json::from_str::<JsonValue>(&text_at(row, 7)?)
                    .unwrap_or(JsonValue::Null),
            });

            match continuum_events::decode_event(&envelope.to_string()) {
                Ok(DecodedEvent::Event(_)) => decoded += 1,
                Ok(DecodedEvent::Skippable(_)) => {
                    skipped_records += 1;
                    saw_skip = true;
                }
                Err(e) => return Err(PersistError::Database(e.to_string())),
            }

            if saw_skip && event_type == "intent.completed" {
                return Err(PersistError::Database(
                    "可跳过事件之后存在 intent.completed，收口依据不成立".to_owned(),
                ));
            }
        }

        Ok(EventLogScan {
            decoded,
            skipped_records,
        })
    }
}
```

`Value::text` 接受 `impl Into<String>`，上面用到了 `&String`；`Value::text(&event.event_id)` 传入 `&String` 不满足 `Into<String>` 时改为 `event.event_id.clone()`。实现时以编译器为准，不要给 `Value::text` 加 `&String` 特化。

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test -p continuum-persist -v`
Expected: PASS，16 passed。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-persist tests
git commit -m "feat(persist): 实现事务 API 与事件、审计的同事务写入"
```

---

### Task 8: 恢复钩子注册表与 §319 五阶段

**Files:**
- Create: `crates/continuum-persist/src/recovery.rs`
- Modify: `crates/continuum-persist/src/lib.rs`
- Test: `crates/continuum-persist/tests/recovery.rs`

**Interfaces:**
- Consumes: Task 6 的 `Db`、Task 7 的 `Tx`
- Produces:
  - `continuum_persist::RecoveryPhase`，五个变体，`RecoveryPhase::ALL`、`as_str()`
  - `continuum_persist::RecoveryHook` trait：`phase(&self) -> RecoveryPhase`、`run(&self, tx: &Tx<'_>) -> Result<(), PersistError>`
  - `continuum_persist::RecoveryRegistry`：`new()`、`register(Box<dyn RecoveryHook>)`
  - `continuum_persist::RecoveryReport { phases: Vec<PhaseReport>, skipped_records: usize }`
  - `continuum_persist::PhaseReport { phase: RecoveryPhase, hooks_run: usize }`
  - `continuum_persist::run_recovery(db: &Db, registry: &RecoveryRegistry) -> Result<RecoveryReport, PersistError>`

- [ ] **Step 1: 写恢复测试**

`crates/continuum-persist/tests/recovery.rs`

```rust
use continuum_persist::{
    run_recovery, Db, PersistError, RecoveryHook, RecoveryPhase, RecoveryRegistry, Tx, Value,
};
use std::sync::{Arc, Mutex};

struct Recorder {
    phase: RecoveryPhase,
    label: &'static str,
    log: Arc<Mutex<Vec<&'static str>>>,
    writes: bool,
}

impl RecoveryHook for Recorder {
    fn phase(&self) -> RecoveryPhase {
        self.phase
    }

    fn run(&self, tx: &Tx<'_>) -> Result<(), PersistError> {
        self.log.lock().unwrap().push(self.label);
        if self.writes {
            tx.execute(
                "INSERT INTO probe (label) VALUES (?1)",
                &[Value::text(self.label)],
            )?;
        }
        Ok(())
    }
}

fn registry() -> (RecoveryRegistry, Arc<Mutex<Vec<&'static str>>>) {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut reg = RecoveryRegistry::new();
    // 注册顺序刻意打乱，断言仍按阶段顺序执行
    reg.register(Box::new(Recorder {
        phase: RecoveryPhase::ResumeEligibleTasks,
        label: "resume",
        log: log.clone(),
        writes: false,
    }));
    reg.register(Box::new(Recorder {
        phase: RecoveryPhase::ReconcileRunningNodes,
        label: "running-1",
        log: log.clone(),
        writes: true,
    }));
    reg.register(Box::new(Recorder {
        phase: RecoveryPhase::ReconcileIncompleteEffects,
        label: "effects",
        log: log.clone(),
        writes: true,
    }));
    reg.register(Box::new(Recorder {
        phase: RecoveryPhase::ReconcileRunningNodes,
        label: "running-2",
        log: log.clone(),
        writes: true,
    }));
    (reg, log)
}

fn db_with_probe() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut ms = continuum_persist::builtin_migrations();
    ms.push(continuum_persist::Migration::new(
        50,
        "probe",
        "CREATE TABLE probe (label TEXT NOT NULL);",
    ));
    let db = Db::open_with(&path, ms).unwrap();
    db.migrate().unwrap();
    (dir, db)
}

#[test]
fn five_phases_run_in_spec_319_order() {
    let (_d, db) = db_with_probe();
    let (reg, log) = registry();
    let report = run_recovery(&db, &reg).unwrap();

    assert_eq!(
        log.lock().unwrap().clone(),
        vec!["effects", "running-1", "running-2", "resume"],
        "钩子必须按 §319 的阶段顺序执行，同阶段内保持注册顺序"
    );
    assert_eq!(report.phases.len(), 5, "报告必须覆盖五个阶段");
    let names: Vec<&str> = report.phases.iter().map(|p| p.phase.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "load durable state",
            "reconcile incomplete effects",
            "reconcile running nodes",
            "mark lost executions",
            "resume eligible tasks",
        ]
    );
}

#[test]
fn phase_without_hooks_reports_zero() {
    let (_d, db) = db_with_probe();
    let (reg, _log) = registry();
    let report = run_recovery(&db, &reg).unwrap();
    let load = report
        .phases
        .iter()
        .find(|p| p.phase == RecoveryPhase::LoadDurableState)
        .unwrap();
    assert_eq!(load.hooks_run, 0);
    let lost = report
        .phases
        .iter()
        .find(|p| p.phase == RecoveryPhase::MarkLostExecutions)
        .unwrap();
    assert_eq!(lost.hooks_run, 0);
    let running = report
        .phases
        .iter()
        .find(|p| p.phase == RecoveryPhase::ReconcileRunningNodes)
        .unwrap();
    assert_eq!(running.hooks_run, 2);
}

#[test]
fn hook_writes_are_committed() {
    let (_d, db) = db_with_probe();
    let (reg, _log) = registry();
    run_recovery(&db, &reg).unwrap();

    let tx = db.begin().unwrap();
    let rows = tx.query("SELECT label FROM probe ORDER BY label", &[]).unwrap();
    assert_eq!(rows.len(), 3, "三个写入型钩子各写一行");
}

#[test]
fn failing_hook_stops_recovery_and_rolls_back_its_phase() {
    struct Boom;
    impl RecoveryHook for Boom {
        fn phase(&self) -> RecoveryPhase {
            RecoveryPhase::ReconcileIncompleteEffects
        }
        fn run(&self, tx: &Tx<'_>) -> Result<(), PersistError> {
            tx.execute(
                "INSERT INTO probe (label) VALUES (?1)",
                &[Value::text("boom")],
            )?;
            Err(PersistError::Database("钩子失败".into()))
        }
    }

    let (_d, db) = db_with_probe();
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut reg = RecoveryRegistry::new();
    reg.register(Box::new(Recorder {
        phase: RecoveryPhase::ReconcileRunningNodes,
        label: "after",
        log: log.clone(),
        writes: true,
    }));
    reg.register(Box::new(Boom));

    run_recovery(&db, &reg).expect_err("钩子失败必须向上传播");
    assert!(
        log.lock().unwrap().is_empty(),
        "后续阶段不得执行"
    );
    let tx = db.begin().unwrap();
    let rows = tx.query("SELECT label FROM probe", &[]).unwrap();
    assert!(rows.is_empty(), "失败阶段自身的写入必须回滚");
}

const INSERT_EVENT: &str =
    "INSERT INTO events
       (event_id, event_type, schema_version, occurred_at, intent_id, node_id, ignorable, payload)
     VALUES (?1, ?2, ?3, ?4, NULL, NULL, ?5, ?6)";

#[test]
fn skipped_records_are_reported() {
    let (_d, db) = db_with_probe();
    let tx = db.begin().unwrap();
    // schema_version 写成文本，解码失败但不致命
    tx.execute(
        INSERT_EVENT,
        &[
            Value::text("e1"),
            Value::text("node.started"),
            Value::text("one"),
            Value::Int(1),
            Value::Int(0),
            Value::text("{}"),
        ],
    )
    .unwrap();
    tx.commit().unwrap();

    let report = run_recovery(&db, &RecoveryRegistry::new()).unwrap();
    assert_eq!(report.skipped_records, 1);
    assert_eq!(report.phases.len(), 5, "跳过记录不得改变阶段覆盖");
}

#[test]
fn fatal_event_log_stops_recovery_before_any_phase() {
    let (_d, db) = db_with_probe();
    let tx = db.begin().unwrap();
    tx.execute(
        INSERT_EVENT,
        &[
            Value::text("e1"),
            Value::text("future.thing"),
            Value::Int(1),
            Value::Int(1),
            Value::Int(0),
            Value::text("{}"),
        ],
    )
    .unwrap();
    tx.commit().unwrap();

    let log = Arc::new(Mutex::new(Vec::new()));
    let mut reg = RecoveryRegistry::new();
    reg.register(Box::new(Recorder {
        phase: RecoveryPhase::ResumeEligibleTasks,
        label: "resume",
        log: log.clone(),
        writes: false,
    }));

    run_recovery(&db, &reg).expect_err("致命的事件日志必须中止启动");
    assert!(
        log.lock().unwrap().is_empty(),
        "致命判定后不得执行任何阶段"
    );
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test -p continuum-persist --test recovery -v`
Expected: FAIL，编译错误 `unresolved import continuum_persist::run_recovery`

- [ ] **Step 3: 实现恢复注册表**

`crates/continuum-persist/src/recovery.rs`

```rust
//! §319 启动恢复的五个阶段与钩子注册表。
//!
//! 每个阶段在独立事务内执行，阶段之间不共享事务。
//! 某阶段失败时该阶段的写入回滚，其后的阶段不执行。

use crate::{Db, PersistError, Tx};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecoveryPhase {
    LoadDurableState,
    ReconcileIncompleteEffects,
    ReconcileRunningNodes,
    MarkLostExecutions,
    ResumeEligibleTasks,
}

impl RecoveryPhase {
    /// 顺序即 §319 的执行顺序，不得改动。
    pub const ALL: [RecoveryPhase; 5] = [
        RecoveryPhase::LoadDurableState,
        RecoveryPhase::ReconcileIncompleteEffects,
        RecoveryPhase::ReconcileRunningNodes,
        RecoveryPhase::MarkLostExecutions,
        RecoveryPhase::ResumeEligibleTasks,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            RecoveryPhase::LoadDurableState => "load durable state",
            RecoveryPhase::ReconcileIncompleteEffects => "reconcile incomplete effects",
            RecoveryPhase::ReconcileRunningNodes => "reconcile running nodes",
            RecoveryPhase::MarkLostExecutions => "mark lost executions",
            RecoveryPhase::ResumeEligibleTasks => "resume eligible tasks",
        }
    }
}

/// 各层注册的恢复步骤。`run` 在一个阶段事务内被调用。
pub trait RecoveryHook: Send + Sync {
    fn phase(&self) -> RecoveryPhase;
    fn run(&self, tx: &Tx<'_>) -> Result<(), PersistError>;
}

#[derive(Default)]
pub struct RecoveryRegistry {
    hooks: Vec<Box<dyn RecoveryHook>>,
}

impl RecoveryRegistry {
    pub fn new() -> Self {
        Self { hooks: Vec::new() }
    }

    pub fn register(&mut self, hook: Box<dyn RecoveryHook>) {
        self.hooks.push(hook);
    }

    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseReport {
    pub phase: RecoveryPhase,
    pub hooks_run: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryReport {
    pub phases: Vec<PhaseReport>,
    /// 事件日志解码时被跳过的记录数（P0 设计第 4.1 节）。
    pub skipped_records: usize,
}

/// 按 §319 顺序执行五个阶段。报告覆盖全部五个阶段，
/// 无钩子的阶段也出现在报告中，`hooks_run` 为 0。
///
/// 第一阶段的内建工作由本函数执行，不受注册表控制：解码事件日志、
/// 校验审计链。任一项致命时返回错误，不进入后续四个阶段。
pub fn run_recovery(db: &Db, registry: &RecoveryRegistry) -> Result<RecoveryReport, PersistError> {
    let skipped_records = {
        let tx = db.begin()?;
        let scan = tx.scan_event_log()?;
        tx.verify_audit_chain()?;
        tx.commit()?;
        scan.skipped_records
    };

    let mut phases = Vec::with_capacity(RecoveryPhase::ALL.len());
    for phase in RecoveryPhase::ALL {
        let matched: Vec<&Box<dyn RecoveryHook>> = registry
            .hooks
            .iter()
            .filter(|h| h.phase() == phase)
            .collect();
        if matched.is_empty() {
            phases.push(PhaseReport {
                phase,
                hooks_run: 0,
            });
            continue;
        }
        let tx = db.begin()?;
        for hook in &matched {
            hook.run(&tx)?;
        }
        tx.commit()?;
        phases.push(PhaseReport {
            phase,
            hooks_run: matched.len(),
        });
    }
    Ok(RecoveryReport {
        phases,
        skipped_records,
    })
}
```

`crates/continuum-persist/src/lib.rs` 追加：

```rust
pub mod recovery;

pub use recovery::{
    run_recovery, PhaseReport, RecoveryHook, RecoveryPhase, RecoveryRegistry, RecoveryReport,
};
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test -p continuum-persist -v`
Expected: PASS，22 passed。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-persist
git commit -m "feat(persist): 实现 §319 恢复五阶段与钩子注册表"
```

---

### Task 9: 崩溃原子性验证

**Files:**
- Create: `crates/continuum-persist/src/bin/crash-writer.rs`
- Test: `crates/continuum-persist/tests/crash_atomicity.rs`

**Interfaces:**
- Consumes: Task 7 的 `Tx`、Task 6 的 `Migration`
- Produces: 无。本 task 只增加验证，不增加库 API。

`crash-writer` 是本 crate 的测试夹具：注册两张属于夹具的迁移表，在一个事务内各写一行，打印 `READY` 后停住，等待外部终止。这两张表模拟 §318 列举的「Artifact 已创建但节点状态仍 RUNNING」，不是 P1 的实现。

- [ ] **Step 1: 写崩溃原子性测试**

`crates/continuum-persist/tests/crash_atomicity.rs`

```rust
use continuum_persist::{Db, Migration, Value};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

/// 夹具迁移。两张表模拟 §318 的跨实体一致性要求。
fn fixture_db(path: &std::path::Path) -> Db {
    let mut ms = continuum_persist::builtin_migrations();
    ms.push(Migration::new(
        60,
        "crash_fixture",
        "CREATE TABLE fx_artifact (id TEXT PRIMARY KEY);
         CREATE TABLE fx_node_state (node_id TEXT PRIMARY KEY, state TEXT NOT NULL);",
    ));
    let db = Db::open_with(path, ms).unwrap();
    db.migrate().unwrap();
    db
}

/// `commit` 为真时夹具在 READY 后提交并退出，用作对照组。
fn spawn_writer(path: &std::path::Path, commit: bool) -> std::process::Child {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_crash-writer"));
    cmd.arg(path).stdout(Stdio::piped()).stderr(Stdio::inherit());
    if commit {
        cmd.arg("--commit");
    }
    cmd.spawn().expect("crash-writer 无法启动")
}

/// 读到 READY 表示两行已写入事务但未提交。
fn wait_ready(child: &mut std::process::Child) {
    let out = child.stdout.take().expect("stdout 已管道化");
    let mut reader = BufReader::new(out);
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line).expect("读 stdout 失败");
        assert_ne!(n, 0, "crash-writer 提前退出，未见 READY");
        if line.trim() == "READY" {
            break;
        }
    }
}

fn count(db: &Db, table: &str) -> usize {
    let tx = db.begin().unwrap();
    tx.query(&format!("SELECT COUNT(*) FROM {table}"), &[])
        .unwrap()
        .into_iter()
        .next()
        .and_then(|r| r.into_iter().next())
        .and_then(|v| match v {
            Value::Int(i) => Some(i as usize),
            _ => None,
        })
        .unwrap_or(0)
}

#[test]
fn killed_mid_transaction_leaves_no_partial_state() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("crash.db");
    fixture_db(&path);

    let mut child = spawn_writer(&path, false);
    wait_ready(&mut child);
    child.kill().expect("SIGKILL 失败");
    child.wait().expect("回收子进程失败");

    let db = fixture_db(&path);
    let a = count(&db, "fx_artifact");
    let n = count(&db, "fx_node_state");
    assert_eq!(
        (a, n),
        (0, 0),
        "事务未提交即被终止，两张表必须同时不可见，实际 artifact={a} node_state={n}"
    );
}

#[test]
fn committed_writer_leaves_both_rows_visible() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ok.db");
    fixture_db(&path);

    let mut child = spawn_writer(&path, true);
    wait_ready(&mut child);
    let status = child.wait().expect("回收子进程失败");
    assert!(status.success(), "夹具应正常退出，实际 {status:?}");

    let db = fixture_db(&path);
    assert_eq!(count(&db, "fx_artifact"), 1, "提交后 fx_artifact 应可见");
    assert_eq!(count(&db, "fx_node_state"), 1, "提交后 fx_node_state 应可见");
}
```

- [ ] **Step 2: 写夹具程序**

`crates/continuum-persist/src/bin/crash-writer.rs`

```rust
//! 崩溃原子性测试夹具。
//!
//! 在一个事务内写两张表，打印 READY 表示写入完成但未提交。
//! 无 `--commit` 时停住等待外部终止，测试用它验证未提交事务在进程被杀后全部回滚。
//! 带 `--commit` 时提交并退出，用作对照组。

use continuum_persist::{Db, Migration, Value};
use std::io::Write;
use std::path::Path;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("用法: crash-writer <db 路径> [--commit]");
    let commit = args.any(|a| a == "--commit");
    let mut ms = continuum_persist::builtin_migrations();
    ms.push(Migration::new(
        60,
        "crash_fixture",
        "CREATE TABLE IF NOT EXISTS fx_artifact (id TEXT PRIMARY KEY);
         CREATE TABLE IF NOT EXISTS fx_node_state (node_id TEXT PRIMARY KEY, state TEXT NOT NULL);",
    ));
    let db = Db::open_with(Path::new(&path), ms).expect("打开数据库失败");
    db.migrate().expect("迁移失败");

    let tx = db.begin().expect("开启事务失败");
    tx.execute(
        "INSERT INTO fx_artifact (id) VALUES (?1)",
        &[Value::text("a1")],
    )
    .expect("写 fx_artifact 失败");
    tx.execute(
        "INSERT INTO fx_node_state (node_id, state) VALUES (?1, ?2)",
        &[Value::text("n1"), Value::text("RUNNING")],
    )
    .expect("写 fx_node_state 失败");

    println!("READY");
    std::io::stdout().flush().expect("flush 失败");

    if commit {
        tx.commit().expect("提交失败");
        println!("COMMITTED");
        return;
    }

    // 不提交，等待外部终止。进程被杀时 drop 不执行，ROLLBACK 由存储层在恢复时完成。
    std::thread::sleep(std::time::Duration::from_secs(60));
}
```

- [ ] **Step 3: 运行测试确认通过**

Run: `cargo test -p continuum-persist --test crash_atomicity -v`
Expected: PASS，2 passed。

若第一个用例失败，说明事务未生效。检查 `Tx::begin` 是否执行了 `BEGIN IMMEDIATE`，以及夹具是否复用了同一个 `Db`（`Db::open_with` 会重新打开连接，重开后 SQLite 会按 WAL 恢复未提交事务）。

- [ ] **Step 4: 提交**

```bash
git add crates/continuum-persist
git commit -m "test(persist): 用 SIGKILL 验证 §318 的事务原子性"
```

---

### Task 10: runtime 装配与启动流程

**Files:**
- Modify: `crates/continuum-runtime/src/main.rs`
- Test: `crates/continuum-runtime/tests/startup.rs`

**Interfaces:**
- Consumes: Task 6 的 `Db::open`、`Db::migrate`、Task 8 的 `RecoveryRegistry`、`run_recovery`
- Produces: 可执行文件 `continuum-runtime`，用法 `continuum-runtime <db 路径>`。

- [ ] **Step 1: 写启动流程测试**

`crates/continuum-runtime/tests/startup.rs`

```rust
use std::path::Path;
use std::process::Command;

fn run(db: &Path) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_continuum-runtime"))
        .arg(db)
        .output()
        .expect("continuum-runtime 无法执行");
    assert!(
        out.status.success(),
        "启动失败: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("输出不是 UTF-8")
}

#[test]
fn startup_applies_migrations_and_runs_five_phases() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let out = run(&path);
    assert!(out.contains("迁移应用 2 项"), "实际输出:\n{out}");
    assert!(out.contains("跳过记录 0 条"), "实际输出:\n{out}");
    for phase in [
        "load durable state",
        "reconcile incomplete effects",
        "reconcile running nodes",
        "mark lost executions",
        "resume eligible tasks",
    ] {
        assert!(out.contains(phase), "输出缺少阶段 {phase}:\n{out}");
    }
}

#[test]
fn second_startup_applies_no_migration() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    run(&path);
    let out = run(&path);
    assert!(out.contains("迁移应用 0 项"), "实际输出:\n{out}");
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test -p continuum-runtime --test startup -v`
Expected: FAIL，`迁移应用` 不在输出中。

- [ ] **Step 3: 实现启动流程**

`crates/continuum-runtime/src/main.rs`

```rust
//! Runtime 入口。P0 阶段只做：打开数据库、应用迁移、执行 §319 恢复五阶段。
//!
//! 各层的恢复钩子注册在 P1 之后接入，本 task 的注册表为空。

use continuum_persist::{run_recovery, Db, PersistError, RecoveryRegistry};
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "continuum.db".to_owned());
    match startup(Path::new(&path)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("启动失败: {e}");
            ExitCode::FAILURE
        }
    }
}

fn startup(path: &Path) -> Result<(), PersistError> {
    let db = Db::open(path)?;
    let applied = db.migrate()?;
    println!("迁移应用 {applied} 项");

    let registry = RecoveryRegistry::new();
    let report = run_recovery(&db, &registry)?;
    println!("跳过记录 {} 条", report.skipped_records);
    for phase in &report.phases {
        println!("{}: {} 钩子", phase.phase.as_str(), phase.hooks_run);
    }
    Ok(())
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test -p continuum-runtime -v`
Expected: PASS，3 passed。

- [ ] **Step 5: 运行全量测试**

Run: `cargo test --workspace`
Expected: 全部 PASS。计数：core 5、provider 1、events 19、persist 24、runtime 3，共 52 passed。

- [ ] **Step 6: 提交**

```bash
git add crates/continuum-runtime
git commit -m "feat(runtime): 装配启动流程，应用迁移并执行 §319 恢复"
```

---

## 完成判据对照

| P0 设计判据 | 由哪个 task 的实现与测试覆盖 |
|---|---|
| 1 事务原子性，无 §318 不一致 | Task 7 `dropped_transaction_rolls_back`、Task 9 `killed_mid_transaction_leaves_no_partial_state` |
| 2 §319 五阶段与钩子 | Task 8 `five_phases_run_in_spec_319_order`、`phase_without_hooks_reports_zero`、`skipped_records_are_reported`、`fatal_event_log_stops_recovery_before_any_phase`、Task 10 `startup_applies_migrations_and_runs_five_phases` |
| 3 九类事件与版本 | Task 4 `nine_event_types_match_spec_310`、`all_event_types_round_trip`、`missing_optional_fields_deserialize_as_none`、`unknown_optional_field_does_not_break_deserialization`、`nine_types_are_never_ignorable`、`codec_chain_rejects_gaps`、`codec_chain_accepts_contiguous_versions` |
| 4 八类审计与篡改检测 | Task 5 `eight_audit_kinds_match_spec_313`、`tampered_payload_is_detected`、Task 7 `tampered_audit_row_is_detected_on_read` |
| 5 三接口冻结且 core/persist 中立 | Task 1 `core_and_persist_do_not_depend_on_provider`、Task 3 `fake_implementations_satisfy_the_frozen_interfaces` |
| §125 操作级授权不可绕过 | Task 2 `empty_operation_list_is_rejected`、`malformed_operation_id_is_rejected`、`deserialization_cannot_bypass_the_operation_check` |
| 5.2 未知事件类型 | Task 4 `unknown_non_ignorable_type_is_fatal`、`unknown_ignorable_type_is_skippable`、Task 7 `unknown_event_type_is_fatal_on_scan` |
| 4.1 判定规则 | Task 7 `scan_counts_skippable_records`、`scan_is_clean_when_all_records_decode`、`skippable_record_before_intent_completed_is_fatal`、`skippable_record_after_intent_completed_is_only_counted` |

## 遗留项

以下在 spec 第 9 节记录，本计划不处理：

```
ENG-001    crate 方向已定，P6 只在此之上加固
A12        Event Stream 的字段结构与版本规则仍开放，本计划给的是 v0.1 默认
B4         防篡改只覆盖单条改写，不覆盖整链重写
§317       P1 起各层注册自己的表与恢复钩子，在此之前 §317 的恢复保证不成立
```

## 取自 DSH 的两项规则

以下规则不在 02 或规范原文中，取自 DSH 的实现，见 `docs/2026-09-30-DSH-对照分析.md`：

```
第 5.1 节  相邻版本迁移链、缺口链拒绝装配、已提交世代不可移动     Task 4
第 5.2 节  未知事件类型默认为读时必需，ignorable 才可跳过         Task 4、Task 7
第 4.1 节  可跳过记录 + intent.completed 升级为致命               Task 7、Task 8
```

这三项扩大了 P0 的设计范围，是规范层未要求的机制。若判定它们超出 v0.1 的必要性，可以在实现前删除，删除点在 Task 4 的 `codec.rs`、Task 7 的 `scan_event_log`、以及 Task 8 第一阶段的内建工作。
