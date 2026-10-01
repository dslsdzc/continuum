# P1 执行层 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 建立 ADFIR 图的表示、Port 类型校验、状态机、失效传播与增量重算判定、Graph Scheduler 与失败重试判定。

**Architecture:** 四个新 crate 按 `graph → {port, operator, artifact, persist}`、`{port, operator} → artifact` 单向依赖。Artifact 的 `id` 为图引用身份、`content_hash` 为内容身份（ENG-003 裁定）。复用判定先按 `Operator.determinism` 分类，只有 `Deterministic` 参与按输入哈希复用（ENG-002 裁定）。失效传播为保守标记，由复用判定收窄。

**Tech Stack:** Rust 1.95.0 / edition 2024、serde + serde_json、sha2、thiserror、rusqlite（经 `continuum-persist`）、tempfile（dev）

**依据:** `docs/superpowers/specs/2026-10-01-p1-execution-layer-design.md`

## Global Constraints

- 工具链固定 rustc 1.95.0 / cargo 1.95.0，edition 2024。
- 依赖方向禁止反向。P1 允许的边：`graph → port`、`graph → operator`、`graph → artifact`、`graph → persist`、`port → artifact`、`operator → artifact`、`artifact → persist`、`artifact → core`。
- `continuum-core` 不含 I/O。
- 数据库连接对象在 `continuum-persist` 内私有，外部 crate 只能通过 `Tx` 访问数据库。
- `ArtifactType` 定义在 `continuum-artifact`，不在 `continuum-port`。
- `§237` 的状态迁移以上述设计第 9 节的表为准；表外迁移一律拒绝。
- 非幂等 Effect 不进入自动重试（§307）。`NonDeterministic` 的 Operator 不写缓存键（ENG-002）。
- 代码注释、错误信息、测试断言信息用中文。标识符用英文。
- 每个 task 结束时 `cargo test --workspace` 必须全绿，并提交一次。
- 各 task 的 Step 里写了预期通过数。该数是**陈旧检查**，不是验收标准：若与实际不符，如实报告实际数并说明差异来源（多出或少掉的用例），**不要改预期去迁就实际，也不要为了让数对上而增删用例**。
- 不在仓库中写入任何凭据。

## Global Constraints 的执行事实

P0 已建立的既有事实，后续 task 依赖它们：

- `continuum-persist` 导出 `Db`（`open` / `open_with` / `migrate` / `begin`）、`Tx`（`query` / `execute` / `commit` / `append_event` / `append_audit`）、`Value`、`Migration`、`builtin_migrations`、`RecoveryRegistry`、`RecoveryPhase`、`run_recovery`。
- `continuum-events` 导出 `Event`（`new` / `with_intent` / `with_node` / `with_ignorable`）、`EventType`（九个变体，`ALL` / `as_str`）。
- `Tx::execute` 的参数类型是 `&[Value]`，返回值是 `Result<u64, PersistError>`；`Tx::query` 返回 `Vec<Vec<Value>>`。
- 运行时装配在 `crates/continuum-runtime/src/main.rs`，目前用 `Db::open`（只含内置迁移）。

---

### Task 1: 四个新 crate 与依赖方向断言

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/continuum-artifact/Cargo.toml`
- Create: `crates/continuum-artifact/src/lib.rs`
- Create: `crates/continuum-port/Cargo.toml`
- Create: `crates/continuum-port/src/lib.rs`
- Create: `crates/continuum-operator/Cargo.toml`
- Create: `crates/continuum-operator/src/lib.rs`
- Create: `crates/continuum-graph/Cargo.toml`
- Create: `crates/continuum-graph/src/lib.rs`
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`

**Interfaces:**
- Consumes: P0 的五个 crate
- Produces: crate 名 `continuum-artifact`、`continuum-port`、`continuum-operator`、`continuum-graph`。后续 task 只在这些 crate 内增删文件。

- [ ] **Step 1: 扩充依赖方向断言**

`crates/continuum-runtime/tests/dependency_direction.rs` 的两处修改。把包列表改为：

```rust
/// 每个包：自身 + 允许依赖的内部 crate。
/// 只查「不含某字符串」不够——P1 新增四个 crate，
/// 必须逐包断言允许集合，否则反向边不会被发现。
const ALLOWED: &[(&str, &[&str])] = &[
    ("continuum-core", &[]),
    ("continuum-events", &["continuum-core"]),
    ("continuum-persist", &["continuum-events"]),
    ("continuum-provider", &["continuum-core"]),
    (
        "continuum-artifact",
        &["continuum-core", "continuum-persist"],
    ),
    ("continuum-port", &["continuum-artifact"]),
    ("continuum-operator", &["continuum-artifact"]),
    (
        "continuum-graph",
        &[
            "continuum-artifact",
            "continuum-port",
            "continuum-operator",
            "continuum-persist",
        ],
    ),
    // 本 task 结束时 runtime 的直接依赖只有这四个。
    // Task 12 会给它加上 continuum-artifact 与 continuum-graph，届时此表要同步扩充。
    (
        "continuum-runtime",
        &[
            "continuum-core",
            "continuum-events",
            "continuum-persist",
            "continuum-provider",
        ],
    ),
];

const ALL_CRATES: &[&str] = &[
    "continuum-core",
    "continuum-events",
    "continuum-persist",
    "continuum-provider",
    "continuum-artifact",
    "continuum-port",
    "continuum-operator",
    "continuum-graph",
    "continuum-runtime",
];

/// 只取直接依赖（`--depth 1`）。
///
/// 既有的 `cargo_tree()` 不带 `--depth`，输出的是整棵传递树，
/// 与 `ALLOWED` 表的语义不符——`ALLOWED` 列的是直接边，
/// 与设计第 5 节的依赖方向一致。直接边足以拦住反向依赖：
/// 任何反向边本身必然是一条直接边。
fn cargo_tree_direct(pkg: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = Command::new(env!("CARGO"))
        .args([
            "tree", "-p", pkg, "--depth", "1", "--edges", "all", "--prefix", "none",
        ])
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
fn every_crate_depends_only_on_its_allowed_set() {
    for (pkg, allowed) in ALLOWED {
        let tree = cargo_tree_direct(pkg);
        for other in ALL_CRATES {
            if other == pkg {
                continue;
            }
            // cargo tree 的 --prefix none 输出每行形如 "continuum-port v0.1.0 (path...)"
            let appeared = tree
                .lines()
                .any(|l| l.split_whitespace().next() == Some(*other));
            assert_eq!(
                appeared,
                allowed.contains(other),
                "{pkg} 对 {other} 的依赖与允许集合不符（出现={appeared}，允许={}）:\n{tree}",
                allowed.contains(other)
            );
        }
    }
}
```

保留既有的 `core_and_persist_do_not_depend_on_provider` 测试不动。

- [ ] **Step 2: 建四个 crate**

`Cargo.toml` 的 `members` 追加四个路径，`[workspace.dependencies]` 追加 `sha2` 已在（P0 已声明），无需新增。

`crates/continuum-artifact/Cargo.toml`

```toml
[package]
name = "continuum-artifact"
version.workspace = true
edition.workspace = true
rust-version.workspace = true

[dependencies]
continuum-core = { path = "../continuum-core" }
continuum-persist = { path = "../continuum-persist" }
serde = { workspace = true }
serde_json = { workspace = true }
sha2 = { workspace = true }
thiserror = { workspace = true }
```

`crates/continuum-port/Cargo.toml`

```toml
[package]
name = "continuum-port"
version.workspace = true
edition.workspace = true
rust-version.workspace = true

[dependencies]
continuum-artifact = { path = "../continuum-artifact" }
serde = { workspace = true }
thiserror = { workspace = true }
```

`crates/continuum-operator/Cargo.toml`

```toml
[package]
name = "continuum-operator"
version.workspace = true
edition.workspace = true
rust-version.workspace = true

[dependencies]
continuum-artifact = { path = "../continuum-artifact" }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
```

`crates/continuum-graph/Cargo.toml`

```toml
[package]
name = "continuum-graph"
version.workspace = true
edition.workspace = true
rust-version.workspace = true

[dependencies]
continuum-artifact = { path = "../continuum-artifact" }
continuum-operator = { path = "../continuum-operator" }
continuum-persist = { path = "../continuum-persist" }
continuum-port = { path = "../continuum-port" }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
```

四个 `src/lib.rs` 各写一行占位：

```rust
// 类型在后续 task 引入。
```

- [ ] **Step 3: 构建并运行断言**

Run: `cargo build --workspace`
Expected: 编译成功。

Run: `cargo test -p continuum-runtime --test dependency_direction -v`
Expected: PASS，2 passed（既有的 `core_and_persist_do_not_depend_on_provider` 与新增的 `every_crate_depends_only_on_its_allowed_set`）。

新断言必须能失败。临时把 `continuum-port/Cargo.toml` 的 `continuum-artifact` 依赖改成 `continuum-persist`，确认 `every_crate_depends_only_on_its_allowed_set` FAILED，随后还原。把这次变异记录进报告。

- [ ] **Step 4: 提交**

```bash
git add Cargo.toml crates
git commit -m "feat(p1): 建立执行层四个 crate，依赖方向改为逐包允许集合断言"
```

---

### Task 2: continuum-artifact 的 Artifact 模型与内容寻址

**Files:**
- Create: `crates/continuum-artifact/src/content.rs`
- Create: `crates/continuum-artifact/src/artifact.rs`
- Create: `crates/continuum-artifact/src/store.rs`
- Modify: `crates/continuum-artifact/src/lib.rs`
- Test: `crates/continuum-artifact/tests/artifact_store.rs`

**Interfaces:**
- Consumes: 无
- Produces:
  - `continuum_artifact::ArtifactType`：`SourceTree`、`Patch`、`TestResult`、`Text`、`Json`、`Blob`
  - `continuum_artifact::PrivacyClass`：`Public`、`Personal`、`Private`、`Secret`、`LocalOnly`
  - `continuum_artifact::ContentHash`：`of(&[u8]) -> Self`、`as_str() -> &str`
  - `continuum_artifact::ArtifactId`：`new(impl Into<String>) -> Self`、`as_str() -> &str`
  - `continuum_artifact::Artifact`：字段见设计第 6 节
  - `continuum_artifact::ArtifactStore`：`commit`、`get`、`find_by_hash`、`lineage`
  - `continuum_artifact::ArtifactError`

- [ ] **Step 1: 写 Artifact 与 ArtifactStore 的测试**

`crates/continuum-artifact/tests/artifact_store.rs`

```rust
use continuum_artifact::{
    Artifact, ArtifactError, ArtifactId, ArtifactStore, ArtifactType, ContentHash, PrivacyClass,
};
use serde_json::json;

fn artifact(id: &str, bytes: &[u8], inputs: Vec<ArtifactId>) -> Artifact {
    Artifact {
        id: ArtifactId::new(id),
        artifact_type: ArtifactType::Text,
        content_hash: ContentHash::of(bytes),
        size: bytes.len() as u64,
        producer_node: Some("n1".to_owned()),
        input_artifacts: inputs,
        metadata: json!({}),
        provenance: json!({}),
        privacy_class: PrivacyClass::Personal,
        version: 1,
    }
}

#[test]
fn identical_content_produces_identical_hash() {
    assert_eq!(ContentHash::of(b"hello"), ContentHash::of(b"hello"));
    assert_ne!(ContentHash::of(b"hello"), ContentHash::of(b"world"));
}

#[test]
fn hash_is_lowercase_hex_sha256() {
    // 已知向量：b"abc" 的 SHA-256
    assert_eq!(
        ContentHash::of(b"abc").as_str(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn commit_persists_and_is_readable() {
    let mut store = ArtifactStore::new();
    store.commit(artifact("a1", b"one", vec![])).unwrap();

    let got = store.get(&ArtifactId::new("a1")).expect("应能读回");
    assert_eq!(got.artifact_type, ArtifactType::Text);
    assert_eq!(got.size, 3);
}

#[test]
fn recommitting_the_same_id_is_rejected() {
    // §241：已提交的 Artifact 不可修改，修改必须产生新版本
    let mut store = ArtifactStore::new();
    store.commit(artifact("a1", b"one", vec![])).unwrap();

    match store.commit(artifact("a1", b"two", vec![])) {
        Err(ArtifactError::AlreadyCommitted { id }) => assert_eq!(id.as_str(), "a1"),
        other => panic!("同 id 重复提交必须被拒绝，实际 {other:?}"),
    }
    // 原内容未被覆盖
    assert_eq!(
        store.get(&ArtifactId::new("a1")).unwrap().content_hash,
        ContentHash::of(b"one")
    );
}

#[test]
fn same_content_under_two_ids_keeps_one_storage_entry() {
    // ENG-003：内容寻址，相同内容只存一份
    let mut store = ArtifactStore::new();
    store.commit(artifact("a1", b"same", vec![])).unwrap();
    store.commit(artifact("a2", b"same", vec![])).unwrap();

    assert_eq!(store.stored_blob_count(), 1, "相同内容不得存两份");
    assert_eq!(
        store.find_by_hash(&ContentHash::of(b"same")).map(|a| a.id.as_str()),
        // 先提交者胜出，顺序稳定
        Some("a1")
    );
}

#[test]
fn lineage_is_traceable_back_to_the_roots() {
    // §242：任意结果可反向追溯到原始输入
    let mut store = ArtifactStore::new();
    store.commit(artifact("root", b"r", vec![])).unwrap();
    store
        .commit(artifact("mid", b"m", vec![ArtifactId::new("root")]))
        .unwrap();
    store
        .commit(artifact("leaf", b"l", vec![ArtifactId::new("mid")]))
        .unwrap();

    let mut traced = store.lineage(&ArtifactId::new("leaf"));
    traced.sort();
    assert_eq!(traced, vec![ArtifactId::new("mid"), ArtifactId::new("root")]);
}

#[test]
fn committing_with_an_unknown_input_is_rejected() {
    let mut store = ArtifactStore::new();
    match store.commit(artifact("a1", b"x", vec![ArtifactId::new("nope")])) {
        Err(ArtifactError::UnresolvedInput { id }) => assert_eq!(id.as_str(), "nope"),
        other => panic!("输入 Artifact 不存在时必须拒绝，实际 {other:?}"),
    }
}
```

`find_by_hash` 返回 `Option<&Artifact>`；`lineage` 返回 `Vec<ArtifactId>`，含传递闭包、不含自身；`stored_blob_count` 返回去重后的内容份数——它在生产代码里也有用（内容寻址的容量统计），不是为测试专设。

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test -p continuum-artifact -v`
Expected: FAIL，编译错误 `unresolved import continuum_artifact`

- [ ] **Step 3: 实现 content.rs**

`crates/continuum-artifact/src/content.rs`

```rust
//! 内容寻址键（ENG-003）。

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// 内容身份。相同内容必须产生相同值；不同内容不得产生相同值。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHash(String);

impl ContentHash {
    pub fn of(bytes: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        Self(hex(&hasher.finalize()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// 小写十六进制编码。与 continuum-events 的同类辅助函数重复，
// 见本计划「遗留」一节。
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(DIGITS[(b >> 4) as usize] as char);
        out.push(DIGITS[(b & 0x0f) as usize] as char);
    }
    out
}
```

- [ ] **Step 4: 实现 artifact.rs**

`crates/continuum-artifact/src/artifact.rs`

```rust
//! Artifact 数据模型（§240）与双身份（ENG-003）。

use crate::content::ContentHash;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// `§239` 的类型集合。枚举外的类型不可表达。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    SourceTree,
    Patch,
    TestResult,
    Text,
    Json,
    Blob,
}

/// `§243`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyClass {
    Public,
    Personal,
    Private,
    Secret,
    LocalOnly,
}

/// 图引用与版本身份。稳定，不随内容变化。
///
/// 派生 `Ord`：id 上的字典序构成全序，排序场景需要它。
/// 实现 `Display`：`ArtifactError` 的三条格式串以 `{id}` 引用该字段，
/// thiserror 要求该字段实现 `Display`。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ArtifactId(String);

impl ArtifactId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ArtifactId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Artifact {
    pub id: ArtifactId,
    pub artifact_type: ArtifactType,
    pub content_hash: ContentHash,
    pub size: u64,
    pub producer_node: Option<String>,
    pub input_artifacts: Vec<ArtifactId>,
    pub metadata: Value,
    pub provenance: Value,
    pub privacy_class: PrivacyClass,
    pub version: u32,
}
```

该类型只有读取方法，字段在构造后不改（§241）。内容的二进制本身由 `ArtifactStore` 按 `content_hash` 存，不在本类型内。

- [ ] **Step 5: 实现 store.rs**

`crates/continuum-artifact/src/store.rs`

```rust
//! Artifact 的提交与寻址（§241、§242）。

use crate::artifact::{Artifact, ArtifactId};
use crate::content::ContentHash;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ArtifactError {
    #[error("Artifact {id} 已提交，修改必须产生新版本（§241）")]
    AlreadyCommitted { id: ArtifactId },
    #[error("Artifact {id} 不存在")]
    NotFound { id: ArtifactId },
    #[error("输入 Artifact {id} 不存在")]
    UnresolvedInput { id: ArtifactId },
}

/// 内容寻址的 Artifact 集合。相同内容只保留一份。
///
/// P1 的实现保存在内存中；落库在 Task 11。
#[derive(Debug, Default)]
pub struct ArtifactStore {
    by_id: HashMap<ArtifactId, Artifact>,
    /// content_hash → 首次提交该内容的 artifact_id。
    /// 此映射是「相同内容只存一份」的载体。
    by_hash: HashMap<ContentHash, ArtifactId>,
    /// 内容份数：by_hash 的大小。
    blobs: usize,
}

impl ArtifactStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// 提交一个 Artifact。
    ///
    /// 同 id 已存在时返回 `AlreadyCommitted`，不覆盖（§241）。
    /// 输入 Artifact 不存在时返回 `UnresolvedInput`。
    pub fn commit(&mut self, artifact: Artifact) -> Result<(), ArtifactError> {
        for input in &artifact.input_artifacts {
            if !self.by_id.contains_key(input) {
                return Err(ArtifactError::UnresolvedInput { id: input.clone() });
            }
        }
        if self.by_id.contains_key(&artifact.id) {
            return Err(ArtifactError::AlreadyCommitted {
                id: artifact.id.clone(),
            });
        }
        match self.by_hash.entry(artifact.content_hash.clone()) {
            std::collections::hash_map::Entry::Occupied(_) => {}
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert(artifact.id.clone());
                self.blobs += 1;
            }
        }
        self.by_id.insert(artifact.id.clone(), artifact);
        Ok(())
    }

    pub fn get(&self, id: &ArtifactId) -> Option<&Artifact> {
        self.by_id.get(id)
    }

    /// 返回首次提交该内容的 Artifact。
    pub fn find_by_hash(&self, hash: &ContentHash) -> Option<&Artifact> {
        let id = self.by_hash.get(hash)?;
        self.by_id.get(id)
    }

    /// 去重后的内容份数。
    pub fn stored_blob_count(&self) -> usize {
        self.blobs
    }

    /// 反向追溯至原始输入。返回传递闭包，不含自身。
    /// 不存在的 id 返回空向量。
    pub fn lineage(&self, id: &ArtifactId) -> Vec<ArtifactId> {
        let mut seen: Vec<ArtifactId> = Vec::new();
        let mut stack: Vec<ArtifactId> = match self.by_id.get(id) {
            Some(a) => a.input_artifacts.clone(),
            None => return seen,
        };
        while let Some(current) = stack.pop() {
            if seen.contains(&current) {
                continue;
            }
            if let Some(a) = self.by_id.get(&current) {
                stack.extend(a.input_artifacts.iter().cloned());
            }
            seen.push(current);
        }
        seen
    }
}
```

`crates/continuum-artifact/src/lib.rs`

```rust
//! Artifact 数据模型、内容寻址与 lineage。P1 执行层的数据底座。

pub mod artifact;
pub mod content;
pub mod store;

pub use artifact::{Artifact, ArtifactId, ArtifactType, PrivacyClass};
pub use content::ContentHash;
pub use store::{ArtifactError, ArtifactStore};
```

- [ ] **Step 6: 运行测试确认通过**

Run: `cargo test -p continuum-artifact -v`
Expected: PASS，7 passed。

- [ ] **Step 7: 提交**

```bash
git add crates/continuum-artifact
git commit -m "feat(artifact): Artifact 模型、内容寻址与 lineage"
```

---

### Task 3: continuum-port 的 Port 与兼容性判定

**Files:**
- Create: `crates/continuum-port/src/port.rs`
- Modify: `crates/continuum-port/src/lib.rs`
- Test: `crates/continuum-port/tests/compatibility.rs`

**Interfaces:**
- Consumes: Task 2 的 `continuum_artifact::ArtifactType`
- Produces:
  - `continuum_port::PortId`：`new(impl Into<String>) -> Self`、`as_str() -> &str`
  - `continuum_port::Direction`：`Input`、`Output`
  - `continuum_port::Port`：`new(PortId, Direction, impl Into<String>, ArtifactType)`、`id()`、`direction()`、`name()`、`artifact_type()`
  - `continuum_port::compatible(&Port, &Port) -> Result<(), PortError>`
  - `continuum_port::PortError`：`TypeMismatch { from, from_type, to, to_type }`

Port 不引用 Node：Node 持有 `Vec<PortId>`（设计第 8.1 节），反向引用会与 `continuum-graph` 成环。

- [ ] **Step 1: 写兼容性测试**

`crates/continuum-port/tests/compatibility.rs`

```rust
use continuum_artifact::ArtifactType;
use continuum_port::{compatible, Direction, Port, PortError, PortId};

fn port(id: &str, direction: Direction, t: ArtifactType) -> Port {
    Port::new(PortId::new(id), direction, id, t)
}

#[test]
fn same_type_is_compatible() {
    let out = port("o1", Direction::Output, ArtifactType::Patch);
    let inp = port("i1", Direction::Input, ArtifactType::Patch);
    compatible(&out, &inp).expect("同类型应兼容");
}

#[test]
fn different_type_is_rejected_with_both_sides_named() {
    let out = port("o1", Direction::Output, ArtifactType::SourceTree);
    let inp = port("i1", Direction::Input, ArtifactType::Patch);
    match compatible(&out, &inp) {
        Err(PortError::TypeMismatch {
            from,
            from_type,
            to,
            to_type,
        }) => {
            assert_eq!(from.as_str(), "o1");
            assert_eq!(from_type, ArtifactType::SourceTree);
            assert_eq!(to.as_str(), "i1");
            assert_eq!(to_type, ArtifactType::Patch);
        }
        other => panic!("类型不同必须被拒绝，实际 {other:?}"),
    }
}

#[test]
fn two_outputs_are_rejected() {
    // 连接的两端必须一进一出
    let a = port("o1", Direction::Output, ArtifactType::Text);
    let b = port("o2", Direction::Output, ArtifactType::Text);
    match compatible(&a, &b) {
        Err(PortError::DirectionMismatch { .. }) => {}
        other => panic!("同向端口不得连接，实际 {other:?}"),
    }
}

#[test]
fn six_types_round_trip_through_serde() {
    for t in [
        ArtifactType::SourceTree,
        ArtifactType::Patch,
        ArtifactType::TestResult,
        ArtifactType::Text,
        ArtifactType::Json,
        ArtifactType::Blob,
    ] {
        let text = serde_json::to_string(&t).expect("可序列化");
        let back: ArtifactType = serde_json::from_str(&text).expect("可反序列化");
        assert_eq!(back, t, "类型 {t:?} 往返不一致");
    }
}
```

`port` 的兼容性检查以「一进一出且类型相同」为条件，故 `PortError` 需要第二个变体 `DirectionMismatch { from, to }`。

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test -p continuum-port -v`
Expected: FAIL，编译错误 `unresolved import continuum_port`

- [ ] **Step 3: 实现 port.rs**

`crates/continuum-port/src/port.rs`

```rust
//! Port 定义与连接兼容性判定（§239）。

use continuum_artifact::ArtifactType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PortId(String);

impl PortId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// `PortError` 的两条格式串以 `{from}` / `{to}` 引用 PortId，
// thiserror 要求该字段实现 Display。
impl std::fmt::Display for PortId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Input,
    Output,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Port {
    id: PortId,
    direction: Direction,
    name: String,
    artifact_type: ArtifactType,
}

impl Port {
    pub fn new(
        id: PortId,
        direction: Direction,
        name: impl Into<String>,
        artifact_type: ArtifactType,
    ) -> Self {
        Self {
            id,
            direction,
            name: name.into(),
            artifact_type,
        }
    }

    pub fn id(&self) -> &PortId {
        &self.id
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn artifact_type(&self) -> ArtifactType {
        self.artifact_type
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PortError {
    #[error("端口 {from} 与 {to} 类型不同：{from_type:?} 与 {to_type:?}")]
    TypeMismatch {
        from: PortId,
        from_type: ArtifactType,
        to: PortId,
        to_type: ArtifactType,
    },
    #[error("端口 {from} 与 {to} 方向相同，连接必须一进一出")]
    DirectionMismatch { from: PortId, to: PortId },
}

/// 判定两个端口能否连接。
///
/// 条件为方向不同且 `artifact_type` 相同（§239）。
///
/// 本函数对方向是对称的：只要求两端方向不同，不要求哪一端是输出。
/// 图层的边有方向（`from_node → to_node` 被失效传播与调度排序依赖），
/// 因此**朝向由 `AdfirGraph::connect` 另行校验**，不在本函数内。
pub fn compatible(from: &Port, to: &Port) -> Result<(), PortError> {
    if from.direction == to.direction {
        return Err(PortError::DirectionMismatch {
            from: from.id.clone(),
            to: to.id.clone(),
        });
    }
    if from.artifact_type != to.artifact_type {
        return Err(PortError::TypeMismatch {
            from: from.id.clone(),
            from_type: from.artifact_type,
            to: to.id.clone(),
            to_type: to.artifact_type,
        });
    }
    Ok(())
}
```

`crates/continuum-port/src/lib.rs`

```rust
//! Port 定义与连接兼容性判定。ADFIR 是运行期构造的动态图，
//! 类型校验在连接构造时进行（§239）。

pub mod port;

pub use port::{compatible, Direction, Port, PortError, PortId};
```

`crates/continuum-port/Cargo.toml` 的 `[dev-dependencies]` 追加：

```toml
[dev-dependencies]
serde_json = { workspace = true }
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test -p continuum-port -v`
Expected: PASS，4 passed。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-port
git commit -m "feat(port): Port 定义与连接兼容性判定"
```

---

### Task 4: continuum-operator 的 Operator 定义与注册表

**Files:**
- Create: `crates/continuum-operator/src/definition.rs`
- Create: `crates/continuum-operator/src/registry.rs`
- Modify: `crates/continuum-operator/src/lib.rs`
- Test: `crates/continuum-operator/tests/registry.rs`

**Interfaces:**
- Consumes: Task 2 的 `continuum_artifact::ArtifactType`
- Produces:
  - `continuum_operator::Determinism`：`Deterministic`、`NonDeterministic`
  - `continuum_operator::SideEffectClass`：`Pure`、`Idempotent`、`NonIdempotent`
  - `continuum_operator::Operator`：字段见设计第 11.1 节
  - `continuum_operator::OperatorId`、`OperatorVersion`、`BackendId`
  - `continuum_operator::OperatorRegistry`：`register`、`resolve`
  - `continuum_operator::OperatorError`：`NotFound`、`Duplicate`

- [ ] **Step 1: 写注册表测试**

`crates/continuum-operator/tests/registry.rs`

```rust
use continuum_artifact::ArtifactType;
use continuum_operator::{
    BackendId, Determinism, Operator, OperatorError, OperatorId, OperatorRegistry,
    OperatorVersion, SideEffectClass,
};

fn op(id: &str, version: u32, determinism: Determinism) -> Operator {
    Operator {
        id: OperatorId::new(id),
        version: OperatorVersion::new(version),
        input_schema: vec![ArtifactType::SourceTree],
        output_schema: vec![ArtifactType::Patch],
        determinism,
        side_effect_class: SideEffectClass::Pure,
        backend_candidates: vec![BackendId::new("builtin")],
    }
}

#[test]
fn registered_operator_is_resolvable_by_id_and_version() {
    let mut registry = OperatorRegistry::new();
    registry.register(op("patch.apply", 1, Determinism::Deterministic)).unwrap();

    let got = registry
        .resolve(&OperatorId::new("patch.apply"), &OperatorVersion::new(1))
        .expect("应能解析");
    assert_eq!(got.output_schema, vec![ArtifactType::Patch]);
}

#[test]
fn unknown_operator_is_rejected() {
    let registry = OperatorRegistry::new();
    match registry.resolve(&OperatorId::new("nope"), &OperatorVersion::new(1)) {
        Err(OperatorError::NotFound { id, version }) => {
            assert_eq!(id.as_str(), "nope");
            assert_eq!(version.as_u32(), 1);
        }
        other => panic!("未注册的 Operator 必须被拒绝，实际 {other:?}"),
    }
}

#[test]
fn same_id_with_different_versions_coexist() {
    let mut registry = OperatorRegistry::new();
    registry.register(op("patch.apply", 1, Determinism::Deterministic)).unwrap();
    registry.register(op("patch.apply", 2, Determinism::NonDeterministic)).unwrap();

    let v2 = registry
        .resolve(&OperatorId::new("patch.apply"), &OperatorVersion::new(2))
        .expect("v2 应能解析");
    assert_eq!(v2.determinism, Determinism::NonDeterministic);
}

#[test]
fn duplicate_id_and_version_is_rejected() {
    let mut registry = OperatorRegistry::new();
    registry.register(op("patch.apply", 1, Determinism::Deterministic)).unwrap();
    match registry.register(op("patch.apply", 1, Determinism::NonDeterministic)) {
        Err(OperatorError::Duplicate { id, version }) => {
            assert_eq!(id.as_str(), "patch.apply");
            assert_eq!(version.as_u32(), 1);
        }
        other => panic!("同 id 同版本重复注册必须被拒绝，实际 {other:?}"),
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test -p continuum-operator -v`
Expected: FAIL，编译错误 `unresolved import continuum_operator`

- [ ] **Step 3: 实现 definition.rs**

`crates/continuum-operator/src/definition.rs`

```rust
//! Operator 定义（§244）。

use continuum_artifact::ArtifactType;
use serde::{Deserialize, Serialize};

/// ENG-002 的判定依据：只有 `Deterministic` 的 Operator 参与按输入哈希复用。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Determinism {
    Deterministic,
    NonDeterministic,
}

/// §307 的判定依据：`NonIdempotent` 不进入自动重试。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SideEffectClass {
    Pure,
    Idempotent,
    NonIdempotent,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OperatorId(String);

impl OperatorId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// `OperatorError` 的格式串以 `{id}` 引用 OperatorId，
// thiserror 要求该字段实现 Display。
impl std::fmt::Display for OperatorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OperatorVersion(u32);

impl OperatorVersion {
    pub fn new(version: u32) -> Self {
        Self(version)
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BackendId(String);

impl BackendId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Operator {
    pub id: OperatorId,
    pub version: OperatorVersion,
    pub input_schema: Vec<ArtifactType>,
    pub output_schema: Vec<ArtifactType>,
    pub determinism: Determinism,
    pub side_effect_class: SideEffectClass,
    /// §245：具体后端由 Router 决定，本层只保存候选。
    pub backend_candidates: Vec<BackendId>,
}
```

- [ ] **Step 4: 实现 registry.rs**

`crates/continuum-operator/src/registry.rs`

```rust
//! Operator 注册表。

use crate::definition::{Operator, OperatorId, OperatorVersion};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OperatorError {
    #[error("Operator {id} 版本 {version:?} 未注册")]
    NotFound { id: OperatorId, version: OperatorVersion },
    #[error("Operator {id} 版本 {version:?} 已注册")]
    Duplicate { id: OperatorId, version: OperatorVersion },
}

#[derive(Debug, Default)]
pub struct OperatorRegistry {
    entries: HashMap<(OperatorId, OperatorVersion), Operator>,
}

impl OperatorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, operator: Operator) -> Result<(), OperatorError> {
        let key = (operator.id.clone(), operator.version);
        if self.entries.contains_key(&key) {
            return Err(OperatorError::Duplicate {
                id: operator.id.clone(),
                version: operator.version,
            });
        }
        self.entries.insert(key, operator);
        Ok(())
    }

    pub fn resolve(
        &self,
        id: &OperatorId,
        version: &OperatorVersion,
    ) -> Result<&Operator, OperatorError> {
        self.entries
            .get(&(id.clone(), *version))
            .ok_or_else(|| OperatorError::NotFound {
                id: id.clone(),
                version: *version,
            })
    }
}
```

`crates/continuum-operator/src/lib.rs`

```rust
//! Operator 定义与注册表（§244、§245）。
//! 本子项目只定义与解析，不提供领域算子实现。

pub mod definition;
pub mod registry;

pub use definition::{
    BackendId, Determinism, Operator, OperatorId, OperatorVersion, SideEffectClass,
};
pub use registry::{OperatorError, OperatorRegistry};
```

- [ ] **Step 5: 定义检查点接口**

设计第 11.3 节要求定义 `Checkpointable`，本子项目只定义不实现。在 `crates/continuum-operator/src/definition.rs` 末尾追加：

```rust
/// §308 要求长任务 Operator 支持 checkpoint。本子项目只定义接口。
/// 未实现该接口的 Operator 不参与检查点。
pub trait Checkpointable {
    type Checkpoint: Send + Sync;

    fn checkpoint(&self) -> Result<Self::Checkpoint, String>;
    fn restore(&self, checkpoint: &Self::Checkpoint) -> Result<(), String>;
}
```

用 `String` 而不是自定义错误类型：本子项目无实现，错误类型的形状应由第一个实现它的领域算子（P5）确定。该接口在本子项目内**无调用点、无测试**，这是刻意的，记录在报告里。

- [ ] **Step 6: 运行测试确认通过**

Run: `cargo test -p continuum-operator -v`
Expected: PASS，4 passed（`Checkpointable` 无实现、无测试，不改变计数）。

- [ ] **Step 7: 提交**

```bash
git add crates/continuum-operator
git commit -m "feat(operator): Operator 定义、注册表与检查点接口"
```

---

### Task 5: continuum-graph 的 Node、Edge 与图结构

**Files:**
- Create: `crates/continuum-graph/src/node.rs`
- Create: `crates/continuum-graph/src/edge.rs`
- Create: `crates/continuum-graph/src/graph.rs`
- Modify: `crates/continuum-graph/src/lib.rs`
- Test: `crates/continuum-graph/tests/graph_structure.rs`

**Interfaces:**
- Consumes: Task 2 的 `ArtifactType`、Task 3 的 `Port`/`PortId`/`Direction`/`compatible`、Task 4 的 `OperatorId`/`OperatorVersion`
- Produces:
  - `continuum_graph::NodeId`、`GraphId`、`ContractIdRef`
  - `continuum_graph::Node`：字段见设计第 8.1 节
  - `continuum_graph::EdgeKind`：`Data`、`Control`、`Dependency`、`Evidence`、`Effect`、`Invalidation`
  - `continuum_graph::Edge`
  - `continuum_graph::AdfirGraph`：`new`、`add_node`、`add_port`、`connect`、`port`、`node`、`node_mut`、`nodes`、`edges`、`edges_from`、`edges_to`、`entry_nodes`、`terminal_nodes`、`set_entry_nodes`、`set_terminal_nodes`、`validate`
    （无 `is_acyclic`：环的拒绝在 `connect` 时强制，设计第 8.3 节不要求查询方法）
  - `continuum_graph::GraphError`

- [ ] **Step 1: 写图结构测试**

`crates/continuum-graph/tests/graph_structure.rs`

```rust
use continuum_artifact::ArtifactType;
use continuum_graph::{
    AdfirGraph, ContractIdRef, EdgeKind, GraphError, GraphId, Node, NodeId,
};
use continuum_operator::{OperatorId, OperatorVersion};
use continuum_port::{Direction, Port, PortId};

fn node(id: &str, op: &str) -> Node {
    Node::new(NodeId::new(id), OperatorId::new(op), OperatorVersion::new(1))
}

fn out_port(id: &str, t: ArtifactType) -> Port {
    Port::new(PortId::new(id), Direction::Output, id, t)
}

fn in_port(id: &str, t: ArtifactType) -> Port {
    Port::new(PortId::new(id), Direction::Input, id, t)
}

fn empty_graph() -> AdfirGraph {
    AdfirGraph::new(GraphId::new("g1"), ContractIdRef::new("c1"))
}

#[test]
fn compatible_ports_connect() {
    let mut g = empty_graph();
    g.add_node(node("n1", "a")).unwrap();
    g.add_node(node("n2", "b")).unwrap();
    g.add_port("n1", out_port("o1", ArtifactType::Patch)).unwrap();
    g.add_port("n2", in_port("i1", ArtifactType::Patch)).unwrap();

    g.connect(&PortId::new("o1"), &PortId::new("i1"), EdgeKind::Data)
        .expect("同类型端口应能连接");
    assert_eq!(g.edges().len(), 1);
}

#[test]
fn incompatible_ports_are_rejected_at_construction() {
    // §239 的 MUST：运行时拒绝不兼容连接
    let mut g = empty_graph();
    g.add_node(node("n1", "a")).unwrap();
    g.add_node(node("n2", "b")).unwrap();
    g.add_port("n1", out_port("o1", ArtifactType::SourceTree)).unwrap();
    g.add_port("n2", in_port("i1", ArtifactType::Patch)).unwrap();

    let err = g
        .connect(&PortId::new("o1"), &PortId::new("i1"), EdgeKind::Data)
        .expect_err("类型不同必须被拒绝");
    assert!(matches!(err, GraphError::PortMismatch { .. }), "实际 {err:?}");
    assert!(g.edges().is_empty(), "被拒绝的连接不得留下边");
}

#[test]
fn connecting_an_unknown_port_is_rejected() {
    let mut g = empty_graph();
    match g.connect(&PortId::new("nope"), &PortId::new("nope2"), EdgeKind::Data) {
        Err(GraphError::UnknownPort { .. }) => {}
        other => panic!("未知端口必须被拒绝，实际 {other:?}"),
    }
}

#[test]
fn cycles_through_data_edges_are_rejected() {
    let mut g = empty_graph();
    g.add_node(node("n1", "a")).unwrap();
    g.add_node(node("n2", "b")).unwrap();
    g.add_port("n1", out_port("o1", ArtifactType::Text)).unwrap();
    g.add_port("n1", in_port("i1", ArtifactType::Text)).unwrap();
    g.add_port("n2", out_port("o2", ArtifactType::Text)).unwrap();
    g.add_port("n2", in_port("i2", ArtifactType::Text)).unwrap();

    g.connect(&PortId::new("o1"), &PortId::new("i2"), EdgeKind::Data).unwrap();
    let err = g
        .connect(&PortId::new("o2"), &PortId::new("i1"), EdgeKind::Data)
        .expect_err("成环的连接必须被拒绝");
    assert!(matches!(err, GraphError::Cycle { .. }), "实际 {err:?}");
    assert_eq!(g.edges().len(), 1);
}

#[test]
fn evidence_edges_do_not_participate_in_cycle_detection() {
    // 设计第 8.3 节：EVIDENCE 与 EFFECT 不参与环检测
    let mut g = empty_graph();
    g.add_node(node("n1", "a")).unwrap();
    g.add_node(node("n2", "b")).unwrap();
    g.add_port("n1", out_port("o1", ArtifactType::Json)).unwrap();
    g.add_port("n1", in_port("i1", ArtifactType::Json)).unwrap();
    g.add_port("n2", out_port("o2", ArtifactType::Json)).unwrap();
    g.add_port("n2", in_port("i2", ArtifactType::Json)).unwrap();

    g.connect(&PortId::new("o1"), &PortId::new("i2"), EdgeKind::Data).unwrap();
    g.connect(&PortId::new("o2"), &PortId::new("i1"), EdgeKind::Evidence)
        .expect("EVIDENCE 边成环应被允许");
}

#[test]
fn adding_a_port_to_an_unknown_node_is_rejected() {
    let mut g = empty_graph();
    match g.add_port("nope", out_port("o1", ArtifactType::Text)) {
        Err(GraphError::UnknownNode { .. }) => {}
        other => panic!("未知节点必须被拒绝，实际 {other:?}"),
    }
}

#[test]
fn two_ports_with_the_same_id_are_rejected() {
    let mut g = empty_graph();
    g.add_node(node("n1", "a")).unwrap();
    g.add_port("n1", out_port("o1", ArtifactType::Text)).unwrap();
    match g.add_port("n1", in_port("o1", ArtifactType::Text)) {
        Err(GraphError::DuplicatePort { .. }) => {}
        other => panic!("端口 id 重复必须被拒绝，实际 {other:?}"),
    }
}

#[test]
fn reversed_orientation_is_rejected() {
    // 边有方向（from_node → to_node），失效传播与调度排序按它解读。
    // 以输入端为起点、输出端为终点必须被拒绝。
    let mut g = empty_graph();
    g.add_node(node("n1", "a")).unwrap();
    g.add_node(node("n2", "b")).unwrap();
    g.add_port("n1", out_port("o1", ArtifactType::Text)).unwrap();
    g.add_port("n2", in_port("i1", ArtifactType::Text)).unwrap();

    let err = g
        .connect(&PortId::new("i1"), &PortId::new("o1"), EdgeKind::Data)
        .expect_err("反向连接必须被拒绝");
    assert!(
        matches!(err, GraphError::WrongOrientation { .. }),
        "实际 {err:?}"
    );
    assert!(g.edges().is_empty(), "被拒绝的连接不得留下边");
}

#[test]
fn entry_node_with_incoming_edges_is_rejected() {
    let mut g = empty_graph();
    g.add_node(node("n1", "a")).unwrap();
    g.add_node(node("n2", "b")).unwrap();
    g.add_port("n1", out_port("o1", ArtifactType::Text)).unwrap();
    g.add_port("n2", in_port("i1", ArtifactType::Text)).unwrap();
    g.connect(&PortId::new("o1"), &PortId::new("i1"), EdgeKind::Data)
        .unwrap();

    g.set_entry_nodes(vec![NodeId::new("n2")]);
    let err = g.validate().expect_err("入口节点的输入端口有入边时必须拒绝");
    assert!(
        matches!(err, GraphError::InvalidEntryNode { .. }),
        "实际 {err:?}"
    );

    g.set_entry_nodes(vec![NodeId::new("n1")]);
    g.validate().expect("n1 无输入端口，应通过");
}

#[test]
fn terminal_node_with_outgoing_edges_is_rejected() {
    let mut g = empty_graph();
    g.add_node(node("n1", "a")).unwrap();
    g.add_node(node("n2", "b")).unwrap();
    g.add_port("n1", out_port("o1", ArtifactType::Text)).unwrap();
    g.add_port("n2", in_port("i1", ArtifactType::Text)).unwrap();
    g.connect(&PortId::new("o1"), &PortId::new("i1"), EdgeKind::Data)
        .unwrap();

    g.set_terminal_nodes(vec![NodeId::new("n1")]);
    let err = g.validate().expect_err("出口节点的输出端口有出边时必须拒绝");
    assert!(
        matches!(err, GraphError::InvalidTerminalNode { .. }),
        "实际 {err:?}"
    );

    g.set_terminal_nodes(vec![NodeId::new("n2")]);
    g.validate().expect("n2 无输出端口，应通过");
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test -p continuum-graph -v`
Expected: FAIL，编译错误 `unresolved import continuum_graph`

- [ ] **Step 3: 实现 node.rs 与 edge.rs**

`crates/continuum-graph/src/node.rs`

```rust
//! ADFIR Node（§236）。

use crate::ids::NodeId;
use continuum_operator::OperatorId;
use continuum_port::PortId;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub operator: OperatorRef,
    pub inputs: Vec<PortId>,
    pub outputs: Vec<PortId>,
    pub constraints: Vec<String>,
    pub capabilities: Vec<String>,
    /// 结构保存，判定属 P4。
    pub execution_policy: Value,
    /// 结构保存，判定属 P5。
    pub verification_policy: Value,
    pub state: NodeState,
}

impl Node {
    pub fn new(id: NodeId, operator: OperatorId, version: continuum_operator::OperatorVersion) -> Self {
        Self {
            id,
            operator: OperatorRef::new(operator, version),
            inputs: Vec::new(),
            outputs: Vec::new(),
            constraints: Vec::new(),
            capabilities: Vec::new(),
            execution_policy: Value::Null,
            verification_policy: Value::Null,
            state: NodeState::Pending,
        }
    }
}

/// §236 的 `operator` 字段：id 与版本共同定位一个 Operator。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorRef {
    pub id: OperatorId,
    pub version: continuum_operator::OperatorVersion,
}

impl OperatorRef {
    pub fn new(id: OperatorId, version: continuum_operator::OperatorVersion) -> Self {
        Self { id, version }
    }
}

/// §237 的十三个状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NodeState {
    Pending,
    Ready,
    Queued,
    Running,
    Waiting,
    Blocked,
    Suspended,
    Verifying,
    Completed,
    Failed,
    Cancelled,
    Invalidated,
    Lost,
}
```

状态迁移表在 Task 6 定义。

`crates/continuum-graph/src/ids.rs`

```rust
//! 图的标识类型。

use serde::{Deserialize, Serialize};

// 派生 `Ord`：id 的字典序构成全序，排序场景需要它。
// 实现 `Display`：`GraphError` 的格式串以 `{id}` / `{from}` / `{to}` 引用 id，
// thiserror 要求这些字段实现 `Display`。两者对所有 id 类型统一给出。
macro_rules! id_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        // 便于按字面量构造，例如 `graph.add_port("n1", port)`。
        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }
    };
}

id_type!(GraphId, "ADFIR 图标识。");
id_type!(NodeId, "节点标识。");
id_type!(ContractIdRef, "图所属的 Contract 标识（§235 的 contract_id）。");
```

`crates/continuum-graph/src/edge.rs`

```rust
//! Edge 六类型（§238）。三类语义由本设计确定，见设计第 8.2 节。

use crate::ids::NodeId;
use continuum_port::PortId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EdgeKind {
    /// 携带 Artifact 的端口连接。参与失效传播。
    Data,
    /// 仅执行序关系。不参与失效传播。
    Control,
    /// 结构性依赖。参与失效传播。
    Dependency,
    /// 验证关系。不参与调度与失效传播。
    Evidence,
    /// 外部副作用的记入关系（§268）。不参与调度与失效传播。
    Effect,
    /// 纯失效传播边（§306）。不参与调度。
    Invalidation,
}

impl EdgeKind {
    /// 是否参与失效传播：DATA、DEPENDENCY、INVALIDATION。
    pub fn propagates_invalidation(&self) -> bool {
        matches!(
            self,
            EdgeKind::Data | EdgeKind::Dependency | EdgeKind::Invalidation
        )
    }

    /// 是否参与环检测：DATA、CONTROL、DEPENDENCY、INVALIDATION。
    pub fn participates_in_cycle_check(&self) -> bool {
        !matches!(self, EdgeKind::Evidence | EdgeKind::Effect)
    }

    /// 是否参与调度排序：DATA、CONTROL、DEPENDENCY。
    pub fn orders_execution(&self) -> bool {
        matches!(
            self,
            EdgeKind::Data | EdgeKind::Control | EdgeKind::Dependency
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edge {
    pub from_node: NodeId,
    pub from_port: PortId,
    pub to_node: NodeId,
    pub to_port: PortId,
    pub kind: EdgeKind,
}
```

- [ ] **Step 4: 实现 graph.rs**

`crates/continuum-graph/src/graph.rs`

```rust
//! ADFIR 图（§235）与连接校验（§239）。

use crate::edge::{Edge, EdgeKind};
use crate::ids::{ContractIdRef, GraphId, NodeId};
use crate::node::Node;
use continuum_port::{compatible, Direction, Port, PortError, PortId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum GraphError {
    #[error("节点 {id} 已存在")]
    DuplicateNode { id: NodeId },
    #[error("节点 {id} 不存在")]
    UnknownNode { id: NodeId },
    #[error("端口 {id} 已存在")]
    DuplicatePort { id: PortId },
    #[error("端口 {id} 不存在")]
    UnknownPort { id: PortId },
    #[error("端口类型不匹配：{0}")]
    PortMismatch(#[from] PortError),
    #[error("连接 {from} → {to} 会成环")]
    Cycle { from: PortId, to: PortId },
    #[error("连接的起点 {from} 必须是输出端口，终点 {to} 必须是输入端口")]
    WrongOrientation { from: PortId, to: PortId },
    #[error("入口节点 {id} 的输入端口存在入边")]
    InvalidEntryNode { id: NodeId },
    #[error("出口节点 {id} 的输出端口存在出边")]
    InvalidTerminalNode { id: NodeId },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdfirGraph {
    pub id: GraphId,
    pub version: u32,
    nodes: Vec<Node>,
    ports: HashMap<PortId, (NodeId, Port)>,
    edges: Vec<Edge>,
    entry_nodes: Vec<NodeId>,
    terminal_nodes: Vec<NodeId>,
    pub contract_id: ContractIdRef,
}

impl AdfirGraph {
    pub fn new(id: GraphId, contract_id: ContractIdRef) -> Self {
        Self {
            id,
            version: 1,
            nodes: Vec::new(),
            ports: HashMap::new(),
            edges: Vec::new(),
            entry_nodes: Vec::new(),
            terminal_nodes: Vec::new(),
            contract_id,
        }
    }

    pub fn entry_nodes(&self) -> &[NodeId] {
        &self.entry_nodes
    }

    pub fn terminal_nodes(&self) -> &[NodeId] {
        &self.terminal_nodes
    }

    pub fn set_entry_nodes(&mut self, nodes: Vec<NodeId>) {
        self.entry_nodes = nodes;
    }

    pub fn set_terminal_nodes(&mut self, nodes: Vec<NodeId>) {
        self.terminal_nodes = nodes;
    }

    /// 校验入口与出口节点的两条约束（设计第 8.3 节）。
    ///
    /// 入口节点的输入端口不得有入边；出口节点的输出端口不得有出边。
    /// 图在投入使用前必须通过本校验——增量构造过程中不检查，
    /// 因为构造完成前这些约束无意义。
    pub fn validate(&self) -> Result<(), GraphError> {
        for id in &self.entry_nodes {
            let node = self
                .node(id)
                .ok_or_else(|| GraphError::UnknownNode { id: id.clone() })?;
            let has_incoming = node
                .inputs
                .iter()
                .any(|p| self.edges.iter().any(|e| e.to_port == *p));
            if has_incoming {
                return Err(GraphError::InvalidEntryNode { id: id.clone() });
            }
        }
        for id in &self.terminal_nodes {
            let node = self
                .node(id)
                .ok_or_else(|| GraphError::UnknownNode { id: id.clone() })?;
            let has_outgoing = node
                .outputs
                .iter()
                .any(|p| self.edges.iter().any(|e| e.from_port == *p));
            if has_outgoing {
                return Err(GraphError::InvalidTerminalNode { id: id.clone() });
            }
        }
        Ok(())
    }

    pub fn add_node(&mut self, node: Node) -> Result<(), GraphError> {
        if self.nodes.iter().any(|n| n.id == node.id) {
            return Err(GraphError::DuplicateNode { id: node.id });
        }
        self.nodes.push(node);
        Ok(())
    }

    /// `node` 接受 `&str` 与 `NodeId` 两种写法。
    pub fn add_port(
        &mut self,
        node: impl Into<NodeId>,
        port: Port,
    ) -> Result<(), GraphError> {
        let node: NodeId = node.into();
        let target = self
            .nodes
            .iter_mut()
            .find(|n| n.id == node)
            .ok_or_else(|| GraphError::UnknownNode { id: node.clone() })?;
        if self.ports.contains_key(port.id()) {
            return Err(GraphError::DuplicatePort {
                id: port.id().clone(),
            });
        }
        match port.direction() {
            Direction::Input => target.inputs.push(port.id().clone()),
            Direction::Output => target.outputs.push(port.id().clone()),
        }
        self.ports.insert(port.id().clone(), (node, port));
        Ok(())
    }

    pub fn port(&self, id: &PortId) -> Option<(NodeId, &Port)> {
        self.ports.get(id).map(|(n, p)| (n.clone(), p))
    }

    pub fn node(&self, id: &NodeId) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == *id)
    }

    pub fn node_mut(&mut self, id: &NodeId) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|n| n.id == *id)
    }

    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    pub fn edges_from(&self, node: &NodeId) -> Vec<&Edge> {
        self.edges.iter().filter(|e| e.from_node == *node).collect()
    }

    pub fn edges_to(&self, node: &NodeId) -> Vec<&Edge> {
        self.edges.iter().filter(|e| e.to_node == *node).collect()
    }

    /// 建立一条连接。
    ///
    /// 校验顺序：两端端口存在 → 朝向（起点输出、终点输入）→ `artifact_type` 相同
    /// （§239）→ 参与环检测的边类型不成环。任一校验失败时不留下边。
    ///
    /// 朝向必须在本层校验：`compatible` 只要求两端方向不同，是方向对称的；
    /// 而本层的边有方向（`from_node → to_node`），失效传播与调度排序都按它解读。
    ///
    /// 同节点的输出连回自身输入不单设检查：那是一条环，由环检测拦下。
    pub fn connect(
        &mut self,
        from: &PortId,
        to: &PortId,
        kind: EdgeKind,
    ) -> Result<(), GraphError> {
        let (from_node, from_port) = self
            .port(from)
            .ok_or_else(|| GraphError::UnknownPort { id: from.clone() })?;
        let (to_node, to_port) = self
            .port(to)
            .ok_or_else(|| GraphError::UnknownPort { id: to.clone() })?;

        if from_port.direction() != Direction::Output || to_port.direction() != Direction::Input {
            return Err(GraphError::WrongOrientation {
                from: from.clone(),
                to: to.clone(),
            });
        }

        compatible(from_port, to_port)?;

        let edge = Edge {
            from_node: from_node.clone(),
            from_port: from.clone(),
            to_node: to_node.clone(),
            to_port: to.clone(),
            kind,
        };

        if kind.participates_in_cycle_check() && self.would_create_cycle(&edge) {
            return Err(GraphError::Cycle {
                from: from.clone(),
                to: to.clone(),
            });
        }

        self.edges.push(edge);
        Ok(())
    }

    fn would_create_cycle(&self, candidate: &Edge) -> bool {
        // 从 candidate.to_node 出发沿参与环检测的边前进，
        // 若能到达 candidate.from_node 则成环。
        let mut stack = vec![candidate.to_node.clone()];
        let mut seen: Vec<NodeId> = Vec::new();
        while let Some(current) = stack.pop() {
            if current == candidate.from_node {
                return true;
            }
            if seen.contains(&current) {
                continue;
            }
            seen.push(current.clone());
            for e in self.edges.iter().filter(|e| {
                e.kind.participates_in_cycle_check() && e.from_node == current
            }) {
                stack.push(e.to_node.clone());
            }
        }
        false
    }
}
```

`crates/continuum-graph/src/lib.rs`

```rust
//! ADFIR 图、Node、Edge 与连接校验。

pub mod edge;
pub mod graph;
pub mod ids;
pub mod node;

pub use edge::{Edge, EdgeKind};
pub use graph::{AdfirGraph, GraphError};
pub use ids::{ContractIdRef, GraphId, NodeId};
pub use node::{Node, NodeState, OperatorRef};
```

- [ ] **Step 5: 运行测试确认通过**

Run: `cargo test -p continuum-graph -v`
Expected: PASS，10 passed。

- [ ] **Step 6: 提交**

```bash
git add crates/continuum-graph
git commit -m "feat(graph): Node、Edge 与 ADFIR 图结构，连接校验"
```

---

### Task 6: Node 状态机迁移表

**Files:**
- Create: `crates/continuum-graph/src/state.rs`
- Modify: `crates/continuum-graph/src/lib.rs`
- Test: `crates/continuum-graph/tests/state_machine.rs`

**Interfaces:**
- Consumes: Task 5 的 `NodeState`
- Produces:
  - `continuum_graph::transition(NodeState, NodeState) -> Result<NodeState, StateError>`
  - `continuum_graph::is_terminal(NodeState) -> bool`
  - `continuum_graph::StateError`：`Illegal { from, to }`
  - `NodeState::ALL: [NodeState; 13]`

- [ ] **Step 1: 写状态机测试**

`crates/continuum-graph/tests/state_machine.rs`

```rust
use continuum_graph::{is_terminal, transition, NodeState, StateError};

#[test]
fn thirteen_states_are_declared() {
    assert_eq!(NodeState::ALL.len(), 13);
}

#[test]
fn legal_transitions_are_accepted() {
    // 设计第 9 节的表
    let legal = [
        (NodeState::Pending, NodeState::Ready),
        (NodeState::Ready, NodeState::Queued),
        (NodeState::Queued, NodeState::Running),
        (NodeState::Running, NodeState::Verifying),
        (NodeState::Verifying, NodeState::Completed),
        (NodeState::Running, NodeState::Waiting),
        (NodeState::Waiting, NodeState::Ready),
        (NodeState::Running, NodeState::Blocked),
        (NodeState::Blocked, NodeState::Ready),
        (NodeState::Suspended, NodeState::Ready),
        (NodeState::Running, NodeState::Failed),
        (NodeState::Verifying, NodeState::Failed),
        (NodeState::Running, NodeState::Lost),
        (NodeState::Verifying, NodeState::Lost),
        (NodeState::Pending, NodeState::Blocked),
        (NodeState::Ready, NodeState::Blocked),
    ];
    for (from, to) in legal {
        assert_eq!(
            transition(from, to).expect("合法迁移应通过"),
            to,
            "迁移 {from:?} → {to:?} 应合法"
        );
    }
}

#[test]
fn invalidation_is_reachable_from_every_state() {
    // §306 要求已完成节点同样可被失效
    for from in NodeState::ALL {
        transition(from, NodeState::Invalidated)
            .unwrap_or_else(|e| panic!("{from:?} → INVALIDATED 应合法，实际 {e}"));
    }
}

#[test]
fn cancellation_is_reachable_from_every_state() {
    for from in NodeState::ALL {
        transition(from, NodeState::Cancelled)
            .unwrap_or_else(|e| panic!("{from:?} → CANCELLED 应合法，实际 {e}"));
    }
}

#[test]
fn running_and_blocked_can_be_suspended() {
    transition(NodeState::Running, NodeState::Suspended).expect("RUNNING → SUSPENDED 应合法");
    transition(NodeState::Waiting, NodeState::Suspended).expect("WAITING → SUSPENDED 应合法");
    transition(NodeState::Blocked, NodeState::Suspended).expect("BLOCKED → SUSPENDED 应合法");
}

#[test]
fn illegal_transitions_are_rejected() {
    let illegal = [
        (NodeState::Pending, NodeState::Running),
        (NodeState::Ready, NodeState::Completed),
        (NodeState::Queued, NodeState::Completed),
        (NodeState::Completed, NodeState::Running),
        (NodeState::Failed, NodeState::Ready),
        (NodeState::Lost, NodeState::Running),
        (NodeState::Verifying, NodeState::Ready),
        (NodeState::Suspended, NodeState::Running),
        (NodeState::Blocked, NodeState::Queued),
    ];
    for (from, to) in illegal {
        match transition(from, to) {
            Err(StateError::Illegal { from: f, to: t }) => {
                assert_eq!((f, t), (from, to), "错误信息应回报实际的两个状态");
            }
            Ok(_) => panic!("迁移 {from:?} → {to:?} 应被拒绝"),
        }
    }
}

#[test]
fn terminal_states_are_identified() {
    for s in [
        NodeState::Completed,
        NodeState::Failed,
        NodeState::Cancelled,
        NodeState::Invalidated,
        NodeState::Lost,
    ] {
        assert!(is_terminal(s), "{s:?} 应为终态");
    }
    for s in [
        NodeState::Pending,
        NodeState::Ready,
        NodeState::Queued,
        NodeState::Running,
        NodeState::Waiting,
        NodeState::Blocked,
        NodeState::Suspended,
        NodeState::Verifying,
    ] {
        assert!(!is_terminal(s), "{s:?} 不应为终态");
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test -p continuum-graph --test state_machine -v`
Expected: FAIL，编译错误 `no function named transition`

- [ ] **Step 3: 实现 state.rs**

`crates/continuum-graph/src/state.rs`

```rust
//! `§237` 的状态迁移表。规范只给出十三个状态名，未定义迁移关系；
//! 本表由 P1 设计第 9 节确定。

use crate::node::NodeState;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StateError {
    #[error("迁移 {from:?} → {to:?} 不在允许的迁移表内")]
    Illegal { from: NodeState, to: NodeState },
}

/// 判定一次状态迁移是否合法。
pub fn transition(from: NodeState, to: NodeState) -> Result<NodeState, StateError> {
    use NodeState::*;

    // INVALIDATED 与 CANCELLED 可由任意状态到达（§306 与取消语义）
    if matches!(to, Invalidated | Cancelled) {
        return Ok(to);
    }

    let legal = matches!(
        (from, to),
        (Pending, Ready)
            | (Ready, Queued)
            | (Queued, Running)
            | (Running, Verifying)
            | (Verifying, Completed)
            | (Running, Waiting)
            | (Waiting, Ready)
            | (Running, Blocked)
            | (Blocked, Ready)
            | (Running, Suspended)
            | (Waiting, Suspended)
            | (Blocked, Suspended)
            | (Suspended, Ready)
            | (Running, Failed)
            | (Verifying, Failed)
            | (Running, Lost)
            | (Verifying, Lost)
            // 设计第 12 节：被阻塞的节点通常尚未运行，故这两条必需
            | (Pending, Blocked)
            | (Ready, Blocked)
    );

    if legal {
        Ok(to)
    } else {
        Err(StateError::Illegal { from, to })
    }
}

pub fn is_terminal(state: NodeState) -> bool {
    matches!(
        state,
        NodeState::Completed
            | NodeState::Failed
            | NodeState::Cancelled
            | NodeState::Invalidated
            | NodeState::Lost
    )
}
```

`crates/continuum-graph/src/lib.rs` 追加：

```rust
pub mod state;

pub use state::{is_terminal, transition, StateError};
```

`crates/continuum-graph/src/node.rs` 的 `NodeState` 追加关联常量：

```rust
impl NodeState {
    /// 全部十三个状态。
    pub const ALL: [NodeState; 13] = [
        NodeState::Pending,
        NodeState::Ready,
        NodeState::Queued,
        NodeState::Running,
        NodeState::Waiting,
        NodeState::Blocked,
        NodeState::Suspended,
        NodeState::Verifying,
        NodeState::Completed,
        NodeState::Failed,
        NodeState::Cancelled,
        NodeState::Invalidated,
        NodeState::Lost,
    ];
}
```

`NodeState` 需要派生 `Hash`（状态机测试里用于比较，`assert_eq!` 只需 `PartialEq`，但 `ALL` 的遍历与去重需要，保留 `Copy` 即可）。若编译报缺少 trait 派生，补齐 `Hash`。

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test -p continuum-graph -v`
Expected: PASS，17 passed（图结构 10 + 状态机 7）。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-graph
git commit -m "feat(graph): Node 状态机迁移表"
```

---

### Task 7: 失效传播

**Files:**
- Create: `crates/continuum-graph/src/invalidation.rs`
- Modify: `crates/continuum-graph/src/lib.rs`
- Test: `crates/continuum-graph/tests/invalidation.rs`

**Interfaces:**
- Consumes: Task 5 的 `AdfirGraph`、`EdgeKind`、Task 6 的 `transition`
- Produces: `continuum_graph::propagate_invalidation(&mut AdfirGraph, &NodeId) -> Result<Vec<NodeId>, GraphError>`

- [ ] **Step 1: 写失效传播测试**

`crates/continuum-graph/tests/invalidation.rs`

```rust
use continuum_artifact::ArtifactType;
use continuum_graph::{
    propagate_invalidation, transition, AdfirGraph, ContractIdRef, EdgeKind, GraphId, Node,
    NodeId, NodeState,
};
use continuum_operator::{OperatorId, OperatorVersion};
use continuum_port::{Direction, Port, PortId};

/// 建一条 x → a → b → c 的 DATA 链，另加一条无关的 d → e 链。
/// 所有节点初始为 COMPLETED。
fn chain() -> AdfirGraph {
    let mut g = AdfirGraph::new(GraphId::new("g1"), ContractIdRef::new("c1"));
    for id in ["x", "a", "b", "c", "d", "e"] {
        let mut n = Node::new(NodeId::new(id), OperatorId::new("op"), OperatorVersion::new(1));
        n.state = NodeState::Completed;
        g.add_node(n).unwrap();
        g.add_port(id, Port::new(PortId::new(format!("{id}-out")), Direction::Output, "o", ArtifactType::Text)).unwrap();
        g.add_port(id, Port::new(PortId::new(format!("{id}-in")), Direction::Input, "i", ArtifactType::Text)).unwrap();
    }
    for (from, to) in [("x", "a"), ("a", "b"), ("b", "c"), ("d", "e")] {
        g.connect(
            &PortId::new(format!("{from}-out")),
            &PortId::new(format!("{to}-in")),
            EdgeKind::Data,
        )
        .unwrap();
    }
    g
}

fn state_of(g: &AdfirGraph, id: &str) -> NodeState {
    g.node(&NodeId::new(id)).expect("节点应存在").state
}

fn invalidate(g: &mut AdfirGraph, id: &str) -> Vec<NodeId> {
    propagate_invalidation(g, &NodeId::new(id)).expect("传播应成功")
}

#[test]
fn invalidation_reaches_the_node_and_all_descendants() {
    let mut g = chain();
    let mut hit = invalidate(&mut g, "a");
    hit.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    assert_eq!(
        hit.iter().map(|n| n.as_str()).collect::<Vec<_>>(),
        vec!["a", "b", "c"]
    );
}

#[test]
fn upstream_and_unrelated_branches_are_untouched() {
    // 02 §3.4 判据 3
    let mut g = chain();
    invalidate(&mut g, "a");

    assert_eq!(state_of(&g, "x"), NodeState::Completed, "上游不得被改动");
    assert_eq!(state_of(&g, "d"), NodeState::Completed, "无关分支不得被改动");
    assert_eq!(state_of(&g, "e"), NodeState::Completed, "无关分支不得被改动");
    assert_eq!(state_of(&g, "a"), NodeState::Invalidated);
    assert_eq!(state_of(&g, "b"), NodeState::Invalidated);
    assert_eq!(state_of(&g, "c"), NodeState::Invalidated);
}

#[test]
fn control_edges_are_not_traversed() {
    let mut g = AdfirGraph::new(GraphId::new("g1"), ContractIdRef::new("c1"));
    for id in ["a", "b"] {
        let mut n = Node::new(NodeId::new(id), OperatorId::new("op"), OperatorVersion::new(1));
        n.state = NodeState::Completed;
        g.add_node(n).unwrap();
        g.add_port(id, Port::new(PortId::new(format!("{id}-out")), Direction::Output, "o", ArtifactType::Text)).unwrap();
        g.add_port(id, Port::new(PortId::new(format!("{id}-in")), Direction::Input, "i", ArtifactType::Text)).unwrap();
    }
    g.connect(&PortId::new("a-out"), &PortId::new("b-in"), EdgeKind::Control).unwrap();

    let hit = invalidate(&mut g, "a");
    assert_eq!(hit.iter().map(|n| n.as_str()).collect::<Vec<_>>(), vec!["a"]);
    assert_eq!(state_of(&g, "b"), NodeState::Completed);
}

#[test]
fn evidence_and_effect_edges_are_not_traversed() {
    let mut g = AdfirGraph::new(GraphId::new("g1"), ContractIdRef::new("c1"));
    for id in ["a", "b"] {
        let mut n = Node::new(NodeId::new(id), OperatorId::new("op"), OperatorVersion::new(1));
        n.state = NodeState::Completed;
        g.add_node(n).unwrap();
        g.add_port(id, Port::new(PortId::new(format!("{id}-out")), Direction::Output, "o", ArtifactType::Json)).unwrap();
        g.add_port(id, Port::new(PortId::new(format!("{id}-in")), Direction::Input, "i", ArtifactType::Json)).unwrap();
    }
    g.connect(&PortId::new("a-out"), &PortId::new("b-in"), EdgeKind::Evidence).unwrap();

    let hit = invalidate(&mut g, "a");
    assert_eq!(hit.iter().map(|n| n.as_str()).collect::<Vec<_>>(), vec!["a"]);
    assert_eq!(state_of(&g, "b"), NodeState::Completed);
}

#[test]
fn dependency_and_invalidation_edges_are_traversed() {
    for kind in [EdgeKind::Dependency, EdgeKind::Invalidation] {
        let mut g = AdfirGraph::new(GraphId::new("g1"), ContractIdRef::new("c1"));
        for id in ["a", "b"] {
            let mut n = Node::new(NodeId::new(id), OperatorId::new("op"), OperatorVersion::new(1));
            n.state = NodeState::Completed;
            g.add_node(n).unwrap();
            g.add_port(id, Port::new(PortId::new(format!("{id}-out")), Direction::Output, "o", ArtifactType::Text)).unwrap();
            g.add_port(id, Port::new(PortId::new(format!("{id}-in")), Direction::Input, "i", ArtifactType::Text)).unwrap();
        }
        g.connect(&PortId::new("a-out"), &PortId::new("b-in"), kind).unwrap();

        let hit = invalidate(&mut g, "a");
        assert_eq!(
            hit.iter().map(|n| n.as_str()).collect::<Vec<_>>(),
            vec!["a", "b"],
            "{kind:?} 边应参与失效传播"
        );
    }
}

#[test]
fn propagation_is_idempotent_over_a_diamond() {
    // a → b, a → c, b → d, c → d：d 只应被标记一次
    let mut g = AdfirGraph::new(GraphId::new("g1"), ContractIdRef::new("c1"));
    for id in ["a", "b", "c", "d"] {
        let mut n = Node::new(NodeId::new(id), OperatorId::new("op"), OperatorVersion::new(1));
        n.state = NodeState::Completed;
        g.add_node(n).unwrap();
        g.add_port(id, Port::new(PortId::new(format!("{id}-out")), Direction::Output, "o", ArtifactType::Text)).unwrap();
        g.add_port(id, Port::new(PortId::new(format!("{id}-in")), Direction::Input, "i", ArtifactType::Text)).unwrap();
    }
    for (from, to) in [("a", "b"), ("a", "c"), ("b", "d"), ("c", "d")] {
        g.connect(&PortId::new(format!("{from}-out")), &PortId::new(format!("{to}-in")), EdgeKind::Data).unwrap();
    }

    let mut hit = invalidate(&mut g, "a");
    hit.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    assert_eq!(
        hit.iter().map(|n| n.as_str()).collect::<Vec<_>>(),
        vec!["a", "b", "c", "d"]
    );
}
```

`transition` 在测试里未使用则从导入行去掉——写测试时以编译器为准。

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test -p continuum-graph --test invalidation -v`
Expected: FAIL，编译错误 `no function named propagate_invalidation`

- [ ] **Step 3: 实现 invalidation.rs**

`crates/continuum-graph/src/invalidation.rs`

```rust
//! 图失效传播（§306）。
//!
//! 传播是保守标记：受影响节点与其下游全部标记为 INVALIDATED。
//! 是否真正重算由复用判定（Task 8）决定。

use crate::graph::{AdfirGraph, GraphError};
use crate::ids::NodeId;
use crate::node::NodeState;
use crate::state::transition;

/// 从 `changed` 出发沿 DATA、DEPENDENCY、INVALIDATION 边传播，
/// 把 `changed` 与其全部下游标记为 INVALIDATED。
///
/// 返回被标记的节点（含 `changed` 自身）。上游与不在此范围内的分支不改变状态。
pub fn propagate_invalidation(
    graph: &mut AdfirGraph,
    changed: &NodeId,
) -> Result<Vec<NodeId>, GraphError> {
    let mut marked: Vec<NodeId> = Vec::new();
    let mut stack: Vec<NodeId> = vec![changed.clone()];

    while let Some(current) = stack.pop() {
        if marked.contains(&current) {
            continue;
        }

        let state = graph
            .node(&current)
            .ok_or_else(|| GraphError::UnknownNode { id: current.clone() })?
            .state;
        let next = transition(state, NodeState::Invalidated)
            .map_err(|_| GraphError::UnknownNode { id: current.clone() })?;
        if let Some(node) = graph.node_mut(&current) {
            node.state = next;
        }
        marked.push(current.clone());

        for edge in graph.edges_from(&current) {
            if edge.kind.propagates_invalidation() {
                stack.push(edge.to_node.clone());
            }
        }
    }

    Ok(marked)
}
```

`crates/continuum-graph/src/lib.rs` 追加：

```rust
pub mod invalidation;

pub use invalidation::propagate_invalidation;
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test -p continuum-graph -v`
Expected: PASS，23 passed（图结构 10 + 状态机 7 + 失效 6）。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-graph
git commit -m "feat(graph): 图失效传播"
```

---

### Task 8: 复用判定

**Files:**
- Create: `crates/continuum-graph/src/reuse.rs`
- Modify: `crates/continuum-graph/src/lib.rs`
- Test: `crates/continuum-graph/tests/reuse.rs`

**Interfaces:**
- Consumes: Task 2 的 `ContentHash`、Task 4 的 `Operator`/`Determinism`/`OperatorVersion`
- Produces:
  - `continuum_graph::CacheKey`：`input_hash`、`operator_version`
  - `continuum_graph::cache_key(&Operator, ContentHash) -> Option<CacheKey>`
  - `continuum_graph::can_reuse(&Operator, Option<&CacheKey>, &ContentHash, bool) -> bool`

`can_reuse` 的第四个参数为 `contract_affected`。

- [ ] **Step 1: 写复用判定测试**

`crates/continuum-graph/tests/reuse.rs`

```rust
use continuum_artifact::{ArtifactType, ContentHash};
use continuum_graph::{cache_key, can_reuse, CacheKey};
use continuum_operator::{
    BackendId, Determinism, Operator, OperatorId, OperatorVersion, SideEffectClass,
};

fn op(determinism: Determinism) -> Operator {
    Operator {
        id: OperatorId::new("op"),
        version: OperatorVersion::new(1),
        input_schema: vec![ArtifactType::Text],
        output_schema: vec![ArtifactType::Text],
        determinism,
        side_effect_class: SideEffectClass::Pure,
        backend_candidates: vec![BackendId::new("builtin")],
    }
}

fn hash(s: &str) -> ContentHash {
    ContentHash::of(s.as_bytes())
}

#[test]
fn deterministic_operator_produces_a_cache_key() {
    let key = cache_key(&op(Determinism::Deterministic), hash("in")).expect("应产生缓存键");
    assert_eq!(key.operator_version, OperatorVersion::new(1));
}

#[test]
fn non_deterministic_operator_produces_no_cache_key() {
    // ENG-002 裁定 + 02 §3.4 判据 5 的验证点
    assert!(
        cache_key(&op(Determinism::NonDeterministic), hash("in")).is_none(),
        "非确定 Operator 不得产生缓存键"
    );
}

#[test]
fn reuse_requires_all_four_conditions() {
    let operator = op(Determinism::Deterministic);
    let current = hash("in");
    let key = CacheKey {
        input_hash: current.clone(),
        operator_version: OperatorVersion::new(1),
    };

    assert!(can_reuse(&operator, Some(&key), &current, false), "四条件全满足应复用");
}

#[test]
fn changed_input_hash_prevents_reuse() {
    let operator = op(Determinism::Deterministic);
    let key = CacheKey {
        input_hash: hash("old"),
        operator_version: OperatorVersion::new(1),
    };
    assert!(!can_reuse(&operator, Some(&key), &hash("new"), false));
}

#[test]
fn changed_operator_version_prevents_reuse() {
    let operator = op(Determinism::Deterministic);
    let key = CacheKey {
        input_hash: hash("in"),
        operator_version: OperatorVersion::new(2),
    };
    assert!(!can_reuse(&operator, Some(&key), &hash("in"), false));
}

#[test]
fn affected_contract_prevents_reuse() {
    let operator = op(Determinism::Deterministic);
    let key = CacheKey {
        input_hash: hash("in"),
        operator_version: OperatorVersion::new(1),
    };
    assert!(!can_reuse(&operator, Some(&key), &hash("in"), true));
}

#[test]
fn non_deterministic_operator_never_reuses() {
    let operator = op(Determinism::NonDeterministic);
    let key = CacheKey {
        input_hash: hash("in"),
        operator_version: OperatorVersion::new(1),
    };
    assert!(!can_reuse(&operator, Some(&key), &hash("in"), false));
}

#[test]
fn missing_cache_entry_prevents_reuse() {
    let operator = op(Determinism::Deterministic);
    assert!(!can_reuse(&operator, None, &hash("in"), false));
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test -p continuum-graph --test reuse -v`
Expected: FAIL，编译错误 `no function named cache_key`

- [ ] **Step 3: 实现 reuse.rs**

`crates/continuum-graph/src/reuse.rs`

```rust
//! 增量重算的复用判定（§305，ENG-002 裁定）。

use continuum_artifact::ContentHash;
use continuum_operator::{Determinism, Operator, OperatorVersion};
use serde::{Deserialize, Serialize};

/// 可复用的凭据。只有 `Deterministic` 的 Operator 才产生它。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheKey {
    pub input_hash: ContentHash,
    pub operator_version: OperatorVersion,
}

/// 为非确定 Operator 返回 `None`。该判定是 ENG-002 裁定的落地点：
/// 非确定节点的输出不写缓存键，因此永远不满足复用前提。
pub fn cache_key(operator: &Operator, input_hash: ContentHash) -> Option<CacheKey> {
    match operator.determinism {
        Determinism::Deterministic => Some(CacheKey {
            input_hash,
            operator_version: operator.version,
        }),
        Determinism::NonDeterministic => None,
    }
}

/// `§305` 的复用条件，加 ENG-002 的 determinism 前置。
///
/// 四项：Operator 确定性、输入哈希未变、Operator 版本未变、Contract 未受影响。
pub fn can_reuse(
    operator: &Operator,
    cached: Option<&CacheKey>,
    current_input_hash: &ContentHash,
    contract_affected: bool,
) -> bool {
    if operator.determinism != Determinism::Deterministic {
        return false;
    }
    if contract_affected {
        return false;
    }
    match cached {
        Some(key) => {
            key.input_hash == *current_input_hash && key.operator_version == operator.version
        }
        None => false,
    }
}
```

`crates/continuum-graph/src/lib.rs` 追加：

```rust
pub mod reuse;

pub use reuse::{cache_key, can_reuse, CacheKey};
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test -p continuum-graph -v`
Expected: PASS，31 passed。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-graph
git commit -m "feat(graph): 增量重算的复用判定与 determinism 分类"
```

---

### Task 9: Graph Scheduler

**Files:**
- Create: `crates/continuum-graph/src/scheduler.rs`
- Modify: `crates/continuum-graph/src/lib.rs`
- Modify: `crates/continuum-graph/src/state.rs`
- Test: `crates/continuum-graph/tests/scheduler.rs`

**本 task 需先修 Task 6 的一处设计漏项。** 设计第 9 节的迁移表原来只有 `RUNNING → BLOCKED`，而第 12 节要求「CONTROL/DEPENDENCY 前驱失败时，依赖它的节点迁移为 BLOCKED」——这类节点通常处于 `PENDING` 或 `READY`。两条规则不能同时成立，`apply_blocking` 因此恒返回空。设计第 9 节与第 12 节已补，`state.rs` 的 `legal` 分支需增加 `(Pending, Blocked)` 与 `(Ready, Blocked)`。

该改动不使既有用例失效：Task 6 的 `illegal_transitions_are_rejected` 所列九条非法迁移不含这两对。

**Interfaces:**
- Consumes: Task 5 的 `AdfirGraph`、`EdgeKind`、Task 6 的 `transition`
- Produces:
  - `continuum_graph::SchedulerConfig { pub max_parallel: usize }`，含 `Default`，默认 `max_parallel: 1`
  - `continuum_graph::select_runnable(&AdfirGraph, &SchedulerConfig) -> Vec<NodeId>`
  - `continuum_graph::apply_blocking(&mut AdfirGraph) -> Vec<NodeId>`
  - `continuum_graph::apply_unblocking(&mut AdfirGraph) -> Vec<NodeId>`

- [ ] **Step 1: 写调度测试**

`crates/continuum-graph/tests/scheduler.rs`

```rust
use continuum_artifact::ArtifactType;
use continuum_graph::{
    apply_blocking, apply_unblocking, select_runnable, AdfirGraph, ContractIdRef, EdgeKind,
    GraphId, Node, NodeId, NodeState, SchedulerConfig,
};
use continuum_operator::{OperatorId, OperatorVersion};
use continuum_port::{Direction, Port, PortId};

fn graph(ids: &[&str]) -> AdfirGraph {
    let mut g = AdfirGraph::new(GraphId::new("g1"), ContractIdRef::new("c1"));
    // `for &id in ids`：ids 是 &[&str]，直接迭代得到 &&str
    for &id in ids {
        let mut n = Node::new(NodeId::new(id), OperatorId::new("op"), OperatorVersion::new(1));
        n.state = NodeState::Pending;
        g.add_node(n).unwrap();
        g.add_port(id, Port::new(PortId::new(format!("{id}-out")), Direction::Output, "o", ArtifactType::Text)).unwrap();
        g.add_port(id, Port::new(PortId::new(format!("{id}-in")), Direction::Input, "i", ArtifactType::Text)).unwrap();
    }
    g
}

fn link(g: &mut AdfirGraph, from: &str, to: &str, kind: EdgeKind) {
    g.connect(
        &PortId::new(format!("{from}-out")),
        &PortId::new(format!("{to}-in")),
        kind,
    )
    .unwrap();
}

fn set_state(g: &mut AdfirGraph, id: &str, state: NodeState) {
    g.node_mut(&NodeId::new(id)).expect("节点应存在").state = state;
}

fn names(v: &[NodeId]) -> Vec<String> {
    let mut out: Vec<String> = v.iter().map(|n| n.as_str().to_owned()).collect();
    out.sort();
    out
}

fn config(n: usize) -> SchedulerConfig {
    SchedulerConfig { max_parallel: n }
}

#[test]
fn default_parallel_limit_is_one() {
    assert_eq!(SchedulerConfig::default().max_parallel, 1);
}

#[test]
fn a_node_without_incoming_execution_edges_is_ready() {
    let g = graph(&["a"]);
    assert_eq!(names(&select_runnable(&g, &config(4))), vec!["a"]);
}

#[test]
fn a_node_waits_until_its_input_source_completes() {
    let mut g = graph(&["a", "b"]);
    link(&mut g, "a", "b", EdgeKind::Data);
    set_state(&mut g, "a", NodeState::Running);

    assert_eq!(names(&select_runnable(&g, &config(4))), Vec::<String>::new());
    set_state(&mut g, "a", NodeState::Completed);
    assert_eq!(names(&select_runnable(&g, &config(4))), vec!["b"]);
}

#[test]
fn control_and_dependency_edges_gate_execution_too() {
    for kind in [EdgeKind::Control, EdgeKind::Dependency] {
        let mut g = graph(&["a", "b"]);
        link(&mut g, "a", "b", kind);
        set_state(&mut g, "a", NodeState::Running);
        assert!(
            select_runnable(&g, &config(4)).is_empty(),
            "{kind:?} 边应约束执行顺序"
        );
    }
}

#[test]
fn evidence_and_effect_edges_do_not_gate_execution() {
    for kind in [EdgeKind::Evidence, EdgeKind::Effect] {
        let mut g = graph(&["a", "b"]);
        link(&mut g, "a", "b", kind);
        set_state(&mut g, "a", NodeState::Running);
        assert_eq!(
            names(&select_runnable(&g, &config(4))),
            vec!["b"],
            "{kind:?} 边不应约束执行顺序"
        );
    }
}

#[test]
fn parallel_limit_caps_the_returned_set() {
    let g = graph(&["a", "b", "c"]);
    assert_eq!(select_runnable(&g, &config(2)).len(), 2);
    assert_eq!(select_runnable(&g, &config(1)).len(), 1);
}

#[test]
fn completed_nodes_are_not_returned_for_execution() {
    let mut g = graph(&["a", "b"]);
    link(&mut g, "a", "b", EdgeKind::Data);
    set_state(&mut g, "a", NodeState::Completed);

    assert_eq!(
        names(&select_runnable(&g, &config(8))),
        vec!["b"],
        "已 COMPLETED 的节点不应再次进入可执行集合"
    );
}

#[test]
fn independent_nodes_are_returned_together_up_to_the_limit() {
    let g = graph(&["a", "b", "c"]);
    assert_eq!(
        names(&select_runnable(&g, &config(8))),
        vec!["a", "b", "c"],
        "无排序依赖的节点应同批返回"
    );
}

#[test]
fn blocking_marks_descendants_of_failed_predecessors() {
    let mut g = graph(&["a", "b", "c"]);
    link(&mut g, "a", "b", EdgeKind::Control);
    link(&mut g, "b", "c", EdgeKind::Control);
    set_state(&mut g, "a", NodeState::Failed);

    let blocked = apply_blocking(&mut g);
    assert_eq!(names(&blocked), vec!["b", "c"]);
    assert_eq!(g.node(&NodeId::new("b")).unwrap().state, NodeState::Blocked);
    assert_eq!(g.node(&NodeId::new("c")).unwrap().state, NodeState::Blocked);
}

#[test]
fn blocking_is_transitive_regardless_of_node_insertion_order() {
    // 插入序取逆拓扑序。这是「迭代到不动点」的唯一守卫：按拓扑序插入时，
    // 单趟扫描同序遍历也能把阻塞传到间接后继，该用例便区分不出两者。
    let mut g = graph(&["c", "b", "a"]);
    link(&mut g, "a", "b", EdgeKind::Control);
    link(&mut g, "b", "c", EdgeKind::Control);
    set_state(&mut g, "a", NodeState::Failed);

    assert_eq!(names(&apply_blocking(&mut g)), vec!["b", "c"]);
}

#[test]
fn an_already_blocked_predecessor_blocks_its_descendants() {
    // 调用开始时前驱已是 BLOCKED，而非本次调用新标记的。
    // 「只算本次新标记」的实现在此返回空，按前驱状态判定的实现能覆盖。
    // 这是该判定的唯一守卫：其余用例的前驱要么是失败态，要么在同一次调用内被标记。
    let mut g = graph(&["b", "c"]);
    link(&mut g, "b", "c", EdgeKind::Control);
    set_state(&mut g, "b", NodeState::Blocked);

    assert_eq!(names(&apply_blocking(&mut g)), vec!["c"]);
    assert_eq!(g.node(&NodeId::new("c")).unwrap().state, NodeState::Blocked);
}

#[test]
fn a_failed_data_predecessor_blocks_its_consumer() {
    // DATA 前驱失败时其 Artifact 永不出现，下游同样不可推进。
    // 边集若只算 CONTROL 与 DEPENDENCY，这类节点会永久搁死在 PENDING。
    let mut g = graph(&["a", "b"]);
    link(&mut g, "a", "b", EdgeKind::Data);
    set_state(&mut g, "a", NodeState::Failed);

    assert_eq!(names(&apply_blocking(&mut g)), vec!["b"]);
    assert_eq!(g.node(&NodeId::new("b")).unwrap().state, NodeState::Blocked);
}

#[test]
fn blocked_nodes_are_unblocked_when_predecessors_complete() {
    let mut g = graph(&["a", "b"]);
    link(&mut g, "a", "b", EdgeKind::Control);
    set_state(&mut g, "a", NodeState::Failed);
    apply_blocking(&mut g);
    assert_eq!(g.node(&NodeId::new("b")).unwrap().state, NodeState::Blocked);

    set_state(&mut g, "a", NodeState::Completed);
    let unblocked = apply_unblocking(&mut g);
    assert_eq!(names(&unblocked), vec!["b"]);
    assert_eq!(g.node(&NodeId::new("b")).unwrap().state, NodeState::Ready);
}

#[test]
fn unblocking_leaves_nodes_with_unfinished_predecessors_blocked() {
    let mut g = graph(&["a", "b"]);
    link(&mut g, "a", "b", EdgeKind::Control);
    set_state(&mut g, "a", NodeState::Failed);
    apply_blocking(&mut g);

    set_state(&mut g, "a", NodeState::Running);
    assert!(
        apply_unblocking(&mut g).is_empty(),
        "前驱未 COMPLETED 时不得解除阻塞"
    );
    assert_eq!(g.node(&NodeId::new("b")).unwrap().state, NodeState::Blocked);
}
```

「返回集合内任两点之间不得存在 CONTROL 或 DATA 路径」这一性质由 READY 条件蕴含，无独立测试对象：候选只取 PENDING 与 READY，而排序前驱已 COMPLETED 的节点不在此列，故两个候选之间不可能有排序路径。实现时不要为它补连通性检查或空转测试。

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test -p continuum-graph --test scheduler -v`
Expected: FAIL，编译错误 `no function named select_runnable`

- [ ] **Step 3: 实现 scheduler.rs**

`crates/continuum-graph/src/scheduler.rs`

```rust
//! Graph Scheduler（§303）。只判定 READY 与可并发，不做放置。

use crate::graph::AdfirGraph;
use crate::ids::NodeId;
use crate::node::NodeState;
use crate::state::transition;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerConfig {
    /// 并行上限。默认 1——P3 接入资源模型之前资源约束不可知。
    pub max_parallel: usize,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self { max_parallel: 1 }
    }
}

/// 返回本轮可提升为 QUEUED 的节点。
///
/// READY 条件：所有 DATA、CONTROL、DEPENDENCY 入边来源处于 COMPLETED。
/// 返回数量不超过 `max_parallel`。
///
/// 返回集合内任两点之间不存在 CONTROL 或 DATA 路径——该性质由 READY 条件
/// 直接蕴含：排序前驱已 COMPLETED 的节点不会被选为候选（候选只取 PENDING
/// 与 READY），因此不另设连通性检查。设计第 12 节已按此改写。
pub fn select_runnable(graph: &AdfirGraph, config: &SchedulerConfig) -> Vec<NodeId> {
    let mut candidates: Vec<NodeId> = graph
        .nodes()
        .iter()
        .filter(|n| n.state == NodeState::Pending || n.state == NodeState::Ready)
        .filter(|n| predecessors_completed(graph, &n.id))
        .map(|n| n.id.clone())
        .collect();
    candidates.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    candidates.truncate(config.max_parallel);
    candidates
}

fn predecessors_completed(graph: &AdfirGraph, node: &NodeId) -> bool {
    graph.edges_to(node).iter().all(|e| {
        if !e.kind.orders_execution() {
            return true;
        }
        graph
            .node(&e.from_node)
            .map(|n| n.state == NodeState::Completed)
            .unwrap_or(false)
    })
}

/// 把调度排序前驱（DATA、CONTROL、DEPENDENCY）已失败或已阻塞的下游节点
/// 迁移为 BLOCKED。
///
/// 不可推进的前驱态包含 BLOCKED、FAILED、CANCELLED、INVALIDATED、LOST。
/// 返回被标记的节点。
///
/// 传递：某节点被标记 BLOCKED 后，依赖它的节点同样不可推进，一并标记。
/// 迭代到不动点。只处理 PENDING 与 READY 的节点——已 QUEUED 或 RUNNING 的
/// 节点不因前驱失败被拽回，只在下一轮调度时因前置条件不满足而不再 READY。
pub fn apply_blocking(graph: &mut AdfirGraph) -> Vec<NodeId> {
    let mut blocked: Vec<NodeId> = Vec::new();

    loop {
        let mut changed = false;
        let ids: Vec<NodeId> = graph.nodes().iter().map(|n| n.id.clone()).collect();

        for id in ids {
            let state = graph.node(&id).expect("节点应存在").state;
            if !matches!(state, NodeState::Pending | NodeState::Ready) {
                continue;
            }

            // 边集与 `select_runnable` 的 READY 判据一致：DATA 前驱失败时其
            // Artifact 永不出现，下游同样不可推进。只算 Control 与 Dependency
            // 会让这类节点既不可 READY 也不 BLOCKED，永久搁死在 PENDING。
            //
            // 前驱处于 BLOCKED 也计为不可推进，且按状态判定而非「本次调用新标记」——
            // 后者在多轮调度下会漏传。
            let stalled = graph.edges_to(&id).iter().any(|e| {
                if !e.kind.orders_execution() {
                    return false;
                }
                graph
                    .node(&e.from_node)
                    .map(|n| {
                        n.state == NodeState::Blocked
                            || matches!(
                                n.state,
                                NodeState::Failed
                                    | NodeState::Cancelled
                                    | NodeState::Invalidated
                                    | NodeState::Lost
                            )
                    })
                    .unwrap_or(false)
            });

            if stalled {
                let next = transition(state, NodeState::Blocked)
                    .expect("PENDING 与 READY 到 BLOCKED 均为合法迁移");
                if let Some(node) = graph.node_mut(&id) {
                    node.state = next;
                }
                blocked.push(id);
                changed = true;
            }
        }

        if !changed {
            break;
        }
    }

    blocked
}

/// 把调度排序前驱（DATA、CONTROL、DEPENDENCY）已全部 COMPLETED 的 BLOCKED
/// 节点迁移为 READY。
///
/// 与 `apply_blocking` 对称（设计第 12 节的后半句）。返回被解除阻塞的节点。
/// 无排序前驱的 BLOCKED 节点视为满足条件——没有阻塞来源。
pub fn apply_unblocking(graph: &mut AdfirGraph) -> Vec<NodeId> {
    let mut unblocked: Vec<NodeId> = Vec::new();
    let ids: Vec<NodeId> = graph.nodes().iter().map(|n| n.id.clone()).collect();

    for id in ids {
        if graph.node(&id).expect("节点应存在").state != NodeState::Blocked {
            continue;
        }
        let all_completed = graph
            .edges_to(&id)
            .iter()
            .filter(|e| e.kind.orders_execution())
            .all(|e| {
                graph
                    .node(&e.from_node)
                    .map(|n| n.state == NodeState::Completed)
                    .unwrap_or(false)
            });
        if all_completed {
            let next = transition(NodeState::Blocked, NodeState::Ready)
                .expect("BLOCKED 到 READY 为合法迁移");
            if let Some(node) = graph.node_mut(&id) {
                node.state = next;
            }
            unblocked.push(id);
        }
    }

    unblocked
}
```

`crates/continuum-graph/src/lib.rs` 追加：

```rust
pub mod scheduler;

pub use scheduler::{apply_blocking, apply_unblocking, select_runnable, SchedulerConfig};
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test -p continuum-graph -v`
Expected: PASS，45 passed。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-graph
git commit -m "feat(graph): Graph Scheduler 与阻塞标记"
```

---

### Task 10: 失败分类与重试判定

**Files:**
- Create: `crates/continuum-graph/src/failure.rs`
- Modify: `crates/continuum-graph/src/lib.rs`
- Test: `crates/continuum-graph/tests/failure.rs`

**Interfaces:**
- Consumes: Task 4 的 `Operator`/`SideEffectClass`
- Produces:
  - `continuum_graph::FailureClass`：七个变体
  - `continuum_graph::EscalationPolicy`：`None`、`Decision`、`Manual`
  - `continuum_graph::RetryPolicy`：`max_attempts`、`retryable_errors`、`escalation_policy`
  - `continuum_graph::RetryDecision`：`Retry { next_attempt }`、`Escalate { to }`、`Fail`
  - `continuum_graph::decide_retry(&Operator, &RetryPolicy, FailureClass, attempt) -> RetryDecision`

- [ ] **Step 1: 写失败判定测试**

`crates/continuum-graph/tests/failure.rs`

```rust
use continuum_artifact::ArtifactType;
use continuum_graph::{
    decide_retry, Backoff, EscalationPolicy, FailureClass, RetryDecision, RetryPolicy,
};
use continuum_operator::{
    BackendId, Determinism, Operator, OperatorId, OperatorVersion, SideEffectClass,
};

fn op(side_effect: SideEffectClass) -> Operator {
    Operator {
        id: OperatorId::new("op"),
        version: OperatorVersion::new(1),
        input_schema: vec![ArtifactType::Text],
        output_schema: vec![ArtifactType::Text],
        determinism: Determinism::Deterministic,
        side_effect_class: side_effect,
        backend_candidates: vec![BackendId::new("builtin")],
    }
}

fn policy(max_attempts: u32, retryable: Vec<FailureClass>) -> RetryPolicy {
    RetryPolicy {
        max_attempts,
        backoff: Backoff::Fixed { interval_ms: 1_000 },
        retryable_errors: retryable,
        escalation_policy: EscalationPolicy::Manual,
    }
}

#[test]
fn backoff_expresses_both_strategies() {
    // 该字段本 task 不消费，但必须能表达 §13.1 给 RESOURCE 的「退避后升级」
    let fixed = Backoff::Fixed { interval_ms: 500 };
    let exponential = Backoff::Exponential {
        initial_ms: 100,
        factor: 2,
        max_ms: 30_000,
    };
    assert_ne!(fixed, exponential);
    assert_eq!(
        serde_json::from_str::<Backoff>(&serde_json::to_string(&exponential).expect("可序列化"))
            .expect("可反序列化"),
        exponential
    );
}

#[test]
fn transient_error_retries_within_the_budget() {
    let p = policy(3, vec![FailureClass::Transient]);
    assert_eq!(
        decide_retry(&op(SideEffectClass::Pure), &p, FailureClass::Transient, 1),
        RetryDecision::Retry { next_attempt: 2 }
    );
}

#[test]
fn exhausting_attempts_escalates() {
    let p = policy(3, vec![FailureClass::Transient]);
    assert_eq!(
        decide_retry(&op(SideEffectClass::Pure), &p, FailureClass::Transient, 3),
        RetryDecision::Escalate {
            to: EscalationPolicy::Manual
        }
    );
}

#[test]
fn non_idempotent_side_effect_never_retries() {
    // §307 的 MUST，也是 02 §3.4 判据 4
    let p = policy(5, vec![FailureClass::Transient]);
    assert_eq!(
        decide_retry(
            &op(SideEffectClass::NonIdempotent),
            &p,
            FailureClass::Transient,
            1
        ),
        RetryDecision::Escalate {
            to: EscalationPolicy::Manual
        },
        "非幂等 Effect 即使类别可重试也不得自动重试"
    );
}

#[test]
fn idempotent_side_effect_may_retry() {
    let p = policy(3, vec![FailureClass::Transient]);
    assert!(matches!(
        decide_retry(&op(SideEffectClass::Idempotent), &p, FailureClass::Transient, 1),
        RetryDecision::Retry { .. }
    ));
}

#[test]
fn constraint_and_authorization_escalate_without_retry() {
    let p = policy(5, vec![FailureClass::Constraint, FailureClass::Authorization]);
    for class in [FailureClass::Constraint, FailureClass::Authorization] {
        assert_eq!(
            decide_retry(&op(SideEffectClass::Pure), &p, class, 1),
            RetryDecision::Escalate {
                to: EscalationPolicy::Manual
            },
            "{class:?} 应升级而非重试"
        );
    }
}

#[test]
fn permanent_verification_and_unknown_fail_without_retry() {
    let p = policy(5, vec![
        FailureClass::Permanent,
        FailureClass::Verification,
        FailureClass::Unknown,
    ]);
    for class in [
        FailureClass::Permanent,
        FailureClass::Verification,
        FailureClass::Unknown,
    ] {
        assert_eq!(
            decide_retry(&op(SideEffectClass::Pure), &p, class, 1),
            RetryDecision::Fail,
            "{class:?} 应直接失败"
        );
    }
}

#[test]
fn resource_retries_when_listed_and_budget_allows() {
    let p = policy(3, vec![FailureClass::Resource]);
    assert_eq!(
        decide_retry(&op(SideEffectClass::Pure), &p, FailureClass::Resource, 1),
        RetryDecision::Retry { next_attempt: 2 }
    );
}

#[test]
fn whitelisting_a_permanent_class_does_not_make_it_retryable() {
    // retryable_errors 只能收窄 §13.1 的固有归属，不能放宽
    let p = policy(5, vec![FailureClass::Permanent]);
    assert_eq!(
        decide_retry(&op(SideEffectClass::Pure), &p, FailureClass::Permanent, 1),
        RetryDecision::Fail
    );
}

#[test]
fn a_class_outside_retryable_errors_fails() {
    let p = policy(5, vec![FailureClass::Transient]);
    assert_eq!(
        decide_retry(&op(SideEffectClass::Pure), &p, FailureClass::Resource, 1),
        RetryDecision::Fail
    );
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test -p continuum-graph --test failure -v`
Expected: FAIL，编译错误 `no function named decide_retry`

- [ ] **Step 3: 实现 failure.rs**

`crates/continuum-graph/src/failure.rs`

```rust
//! 失败分类与重试判定（§307、§309）。

use continuum_operator::{Operator, SideEffectClass};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FailureClass {
    Transient,
    Permanent,
    Constraint,
    Authorization,
    Resource,
    Verification,
    Unknown,
}

impl FailureClass {
    pub const ALL: [FailureClass; 7] = [
        FailureClass::Transient,
        FailureClass::Permanent,
        FailureClass::Constraint,
        FailureClass::Authorization,
        FailureClass::Resource,
        FailureClass::Verification,
        FailureClass::Unknown,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EscalationPolicy {
    None,
    Decision,
    Manual,
}

/// 退避策略。`§307` 的 `RetryPolicy` 含 `backoff`，`§13.1` 给 RESOURCE 的
/// 固有策略是「可重试，退避后仍失败则升级」——缺该字段则接口无法表达退避。
///
/// 本子项目的 `decide_retry` 不使用它：退避是执行方等待时的事，此处只让类型
/// 能表达该策略。消费方在 P3 之后的调度路径。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Backoff {
    /// 固定间隔。
    Fixed { interval_ms: u64 },
    /// 指数退避：首次 `initial_ms`，每次乘 `factor`，单次不超过 `max_ms`。
    Exponential {
        initial_ms: u64,
        factor: u32,
        max_ms: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub backoff: Backoff,
    pub retryable_errors: Vec<FailureClass>,
    pub escalation_policy: EscalationPolicy,
}

impl Default for RetryPolicy {
    /// 默认策略为「单次尝试、不重试」：`max_attempts = 1` 使 `decide_retry`
    /// 在首次失败即升级或失败，不产生第二次尝试。
    ///
    /// `ExecutionProfile` 的 `retry_policy` 非可选，故需要此默认值。
    fn default() -> Self {
        Self {
            max_attempts: 1,
            backoff: Backoff::Fixed { interval_ms: 1_000 },
            retryable_errors: Vec::new(),
            escalation_policy: EscalationPolicy::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryDecision {
    Retry { next_attempt: u32 },
    Escalate { to: EscalationPolicy },
    Fail,
}

/// 判定一次失败后的动作。
///
/// 判定顺序：
///
/// 1. 非幂等副作用直接升级（`§307` 的 MUST），白名单不改变它
/// 2. 按类别取 `§309` 的固有归属：
///    `CONSTRAINT` / `AUTHORIZATION` 升级；`PERMANENT` / `VERIFICATION` /
///    `UNKNOWN` 失败；`TRANSIENT` / `RESOURCE` 继续往下判
/// 3. 类别不在 `retryable_errors` 内时失败
/// 4. 仍有尝试余量时重试
/// 5. 否则升级
///
/// `retryable_errors` 只能**收窄**固有归属，不能放宽：把一个 `PERMANENT`
/// 类别加进白名单不会让它变成可重试。第 2 步先于第 3 步，故这一点由结构保证。
pub fn decide_retry(
    operator: &Operator,
    policy: &RetryPolicy,
    class: FailureClass,
    attempt: u32,
) -> RetryDecision {
    if operator.side_effect_class == SideEffectClass::NonIdempotent {
        return escalate(policy);
    }

    match class {
        FailureClass::Constraint | FailureClass::Authorization => return escalate(policy),
        FailureClass::Permanent | FailureClass::Verification | FailureClass::Unknown => {
            return RetryDecision::Fail
        }
        FailureClass::Transient | FailureClass::Resource => {}
    }

    if !policy.retryable_errors.contains(&class) {
        return RetryDecision::Fail;
    }
    if attempt < policy.max_attempts {
        return RetryDecision::Retry {
            next_attempt: attempt + 1,
        };
    }
    escalate(policy)
}

fn escalate(policy: &RetryPolicy) -> RetryDecision {
    match policy.escalation_policy {
        EscalationPolicy::None => RetryDecision::Fail,
        other => RetryDecision::Escalate { to: other },
    }
}
```

`crates/continuum-graph/src/lib.rs` 追加：

```rust
pub mod failure;

pub use failure::{
    decide_retry, Backoff, EscalationPolicy, FailureClass, RetryDecision, RetryPolicy,
};
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test -p continuum-graph -v`
Expected: PASS，55 passed（Task 9 结束时 45 + 本 task 10 条）。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-graph
git commit -m "feat(graph): 失败分类与重试判定"
```

---

### Task 11: 图与 Artifact 的持久化

> **订正（2026-10-01，合并前终审后补）。** 本 task 与 Task 12 的代码块在
> `adfir_node.state` 这一列上写了大写字面量（`'RUNNING'`、`'VERIFYING'`、`'LOST'`、
> `'UNKNOWN'`），与 `state_str` / `parse_state` 的小写约定冲突（`adfir_edge.kind`、
> `adfir_port.direction`、`artifact.artifact_type` 也都是小写）。两处大写曾导致一个
> Critical：恢复钩子对 `save_graph` 写出的数据恒 0 行命中，且钩子写出的 `'LOST'`
> 读不回图。实际实现已统一为小写（见 `crates/continuum-runtime/src/recovery.rs`）。
> **照抄本节代码时请一律改用小写。** 本段的代码块保留原样，以维持计划作为历史记录的可追溯性。


**Files:**
- Create: `crates/continuum-artifact/src/persist.rs`
- Create: `crates/continuum-graph/src/persist.rs`
- Modify: `crates/continuum-artifact/src/lib.rs`
- Modify: `crates/continuum-graph/src/lib.rs`
- Test: `crates/continuum-graph/tests/persistence.rs`

**Interfaces:**
- Consumes: Task 2 的 `Artifact`、Task 5 的 `AdfirGraph`、`continuum_persist::{Db, Tx, Migration, Value}`
- Produces:
  - `continuum_artifact::p1_artifact_migrations() -> Vec<Migration>`
  - `continuum_artifact::save_artifact(&Tx<'_>, &Artifact) -> Result<(), PersistError>`
  - `continuum_artifact::load_artifact(&Tx<'_>, &ArtifactId) -> Result<Option<Artifact>, PersistError>`
  - `continuum_graph::p1_graph_migrations() -> Vec<Migration>`
  - `continuum_graph::save_graph(&Tx<'_>, &AdfirGraph) -> Result<(), PersistError>`
  - `continuum_graph::load_graph(&Tx<'_>, &GraphId) -> Result<Option<AdfirGraph>, PersistError>`

- [ ] **Step 1: 写往返测试**

`crates/continuum-graph/tests/persistence.rs`

```rust
use continuum_artifact::{
    load_artifact, p1_artifact_migrations, save_artifact, Artifact, ArtifactId, ArtifactType,
    ContentHash, PrivacyClass,
};
use continuum_graph::{
    load_graph, p1_graph_migrations, save_graph, AdfirGraph, ContractIdRef, EdgeKind, GraphId, Node,
    NodeId, NodeState,
};
use continuum_operator::{OperatorId, OperatorVersion};
use continuum_persist::{builtin_migrations, Db, Migration, Value};
use continuum_port::{Direction, Port, PortId};
use serde_json::json;

fn db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut migrations: Vec<Migration> = builtin_migrations();
    migrations.extend(p1_artifact_migrations());
    migrations.extend(p1_graph_migrations());
    let db = Db::open_with(&path, migrations).unwrap();
    db.migrate().unwrap();
    (dir, db)
}

fn sample_graph() -> AdfirGraph {
    let mut g = AdfirGraph::new(GraphId::new("g1"), ContractIdRef::new("c1"));
    let mut n = Node::new(NodeId::new("n1"), OperatorId::new("op"), OperatorVersion::new(1));
    n.state = NodeState::Ready;
    // 非空值：这两项曾不落库，读回时静默归零
    n.constraints = vec!["c1".to_owned()];
    n.capabilities = vec!["cap1".to_owned()];
    g.add_node(n).unwrap();
    g.add_port("n1", Port::new(PortId::new("o1"), Direction::Output, "o", ArtifactType::Patch))
        .unwrap();
    g.add_port("n1", Port::new(PortId::new("i1"), Direction::Input, "i", ArtifactType::SourceTree))
        .unwrap();
    let mut n2 = Node::new(NodeId::new("n2"), OperatorId::new("op2"), OperatorVersion::new(2));
    n2.state = NodeState::Completed;
    g.add_node(n2).unwrap();
    // o2 的类型必须与 i1 一致（SourceTree），否则 §239 会拒绝这条连接。
    // 取 SourceTree 而非把 i1 改成 Patch：前者保住夹具里两种不同的
    // ArtifactType，往返覆盖更强。
    g.add_port("n2", Port::new(PortId::new("o2"), Direction::Output, "o", ArtifactType::SourceTree))
        .unwrap();
    g.add_port("n2", Port::new(PortId::new("i2"), Direction::Input, "i", ArtifactType::Patch))
        .unwrap();
    g.connect(&PortId::new("o2"), &PortId::new("i1"), EdgeKind::Data)
        .unwrap();
    // n2 的输入无入边、n1 的输出无出边，故这一对满足设计 §8.3 的两条约束
    g.set_entry_nodes(vec![NodeId::new("n2")]);
    g.set_terminal_nodes(vec![NodeId::new("n1")]);
    g
}

fn sample_artifact() -> Artifact {
    Artifact {
        id: ArtifactId::new("a1"),
        artifact_type: ArtifactType::Patch,
        content_hash: ContentHash::of(b"diff"),
        size: 4,
        producer_node: Some("n1".to_owned()),
        input_artifacts: vec![],
        metadata: json!({"k": "v"}),
        provenance: json!({}),
        privacy_class: PrivacyClass::Personal,
        version: 1,
    }
}

#[test]
fn graph_round_trips_through_the_database() {
    let (_d, db) = db();
    let graph = sample_graph();

    let tx = db.begin().unwrap();
    save_graph(&tx, &graph).unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let back = load_graph(&tx, &GraphId::new("g1")).unwrap().expect("应能读回");
    tx.commit().unwrap();

    assert_eq!(back.id, graph.id);
    assert_eq!(back.version, graph.version);
    assert_eq!(back.nodes().len(), 2);
    assert_eq!(back.edges().len(), 1);
    assert_eq!(back.node(&NodeId::new("n1")).unwrap().state, NodeState::Ready);
    assert_eq!(back.node(&NodeId::new("n2")).unwrap().state, NodeState::Completed);
    assert_eq!(
        back.node(&NodeId::new("n1")).unwrap().constraints,
        vec!["c1".to_owned()],
        "constraints 不得在读回时归零"
    );
    assert_eq!(
        back.node(&NodeId::new("n1")).unwrap().capabilities,
        vec!["cap1".to_owned()],
        "capabilities 不得在读回时归零"
    );
    assert_eq!(
        back.port(&PortId::new("o1")).unwrap().1.artifact_type(),
        ArtifactType::Patch
    );
    assert_eq!(back.edges()[0].kind, EdgeKind::Data);
    assert_eq!(back.entry_nodes(), graph.entry_nodes());
    assert_eq!(back.terminal_nodes(), graph.terminal_nodes());
    // load_graph 内部调用过 validate()，能读回即说明两条约束成立
}

#[test]
fn artifact_round_trips_through_the_database() {
    let (_d, db) = db();
    let artifact = sample_artifact();

    let tx = db.begin().unwrap();
    save_artifact(&tx, &artifact).unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let back = load_artifact(&tx, &ArtifactId::new("a1"))
        .unwrap()
        .expect("应能读回");
    tx.commit().unwrap();

    assert_eq!(back, artifact);
}

#[test]
fn loading_an_unknown_graph_returns_none() {
    let (_d, db) = db();
    let tx = db.begin().unwrap();
    assert!(load_graph(&tx, &GraphId::new("nope")).unwrap().is_none());
    tx.commit().unwrap();
}

#[test]
fn load_graph_rejects_a_stored_graph_violating_entry_constraints() {
    // 该用例是 load_graph 内 validate() 调用的唯一守卫：
    // 没有它，删掉那行调用不会有任何用例变红。
    let (_d, db) = db();
    let graph = sample_graph();
    let tx = db.begin().unwrap();
    save_graph(&tx, &graph).unwrap();
    // 把入口改成 n1——它的输入端口 i1 有入边（来自 o2），违反设计 §8.3
    tx.execute(
        "UPDATE adfir_graph SET entry_nodes = ?1 WHERE id = ?2",
        &[Value::text(r#"["n1"]"#), Value::text("g1")],
    )
    .unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let err = load_graph(&tx, &GraphId::new("g1")).expect_err("违反入口约束的图必须被拒绝");
    assert!(err.to_string().contains("入口"), "实际: {err}");
}
```

`Cargo.toml` 的 `crates/continuum-graph/Cargo.toml` 追加 dev-dependency：

```toml
[dev-dependencies]
tempfile = { workspace = true }
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test -p continuum-graph --test persistence -v`
Expected: FAIL，编译错误 `no function named p1_graph_migrations`

- [ ] **Step 3: 实现 artifact 的持久化**

`crates/continuum-artifact/src/persist.rs`

```rust
//! Artifact 元数据的落库（§317）。

use crate::artifact::{Artifact, ArtifactId, ArtifactType, PrivacyClass};
use crate::content::ContentHash;
use continuum_persist::{Migration, PersistError, Tx, Value};

pub fn p1_artifact_migrations() -> Vec<Migration> {
    vec![
        Migration::new(10, "p1_artifact", "CREATE TABLE artifact (
            id TEXT PRIMARY KEY,
            artifact_type TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            size INTEGER NOT NULL,
            producer_node TEXT,
            privacy_class TEXT NOT NULL,
            version INTEGER NOT NULL,
            metadata TEXT NOT NULL,
            provenance TEXT NOT NULL
        );
        CREATE INDEX idx_artifact_hash ON artifact(content_hash);
        CREATE TABLE artifact_input (
            artifact_id TEXT NOT NULL,
            input_artifact_id TEXT NOT NULL,
            PRIMARY KEY (artifact_id, input_artifact_id)
        );"),
    ]
}

/// 写入一个 Artifact 的元数据。同 id 已存在时返回数据库错误，不覆盖（§241）。
pub fn save_artifact(tx: &Tx<'_>, artifact: &Artifact) -> Result<(), PersistError> {
    tx.execute(
        "INSERT INTO artifact
           (id, artifact_type, content_hash, size, producer_node, privacy_class,
            version, metadata, provenance)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        &[
            Value::text(artifact.id.as_str()),
            Value::text(type_str(artifact.artifact_type)),
            Value::text(artifact.content_hash.as_str()),
            Value::Int(artifact.size as i64),
            match &artifact.producer_node {
                Some(n) => Value::text(n.clone()),
                None => Value::Null,
            },
            Value::text(privacy_str(artifact.privacy_class)),
            Value::Int(i64::from(artifact.version)),
            Value::text(serde_json::to_string(&artifact.metadata).expect("metadata 可序列化")),
            Value::text(serde_json::to_string(&artifact.provenance).expect("provenance 可序列化")),
        ],
    )?;
    for input in &artifact.input_artifacts {
        tx.execute(
            "INSERT INTO artifact_input (artifact_id, input_artifact_id) VALUES (?1, ?2)",
            &[Value::text(artifact.id.as_str()), Value::text(input.as_str())],
        )?;
    }
    Ok(())
}

pub fn load_artifact(
    tx: &Tx<'_>,
    id: &ArtifactId,
) -> Result<Option<Artifact>, PersistError> {
    let rows = tx.query(
        "SELECT id, artifact_type, content_hash, size, producer_node, privacy_class,
                version, metadata, provenance
         FROM artifact WHERE id = ?1",
        &[Value::text(id.as_str())],
    )?;
    let Some(row) = rows.into_iter().next() else {
        return Ok(None);
    };

    let inputs = tx.query(
        "SELECT input_artifact_id FROM artifact_input WHERE artifact_id = ?1
         ORDER BY input_artifact_id",
        &[Value::text(id.as_str())],
    )?;
    let input_artifacts = inputs
        .into_iter()
        .map(|r| match &r[0] {
            Value::Text(s) => Ok(ArtifactId::new(s.clone())),
            other => Err(PersistError::Database(format!(
                "input_artifact_id 应为文本，实际 {other:?}"
            ))),
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Some(Artifact {
        id: ArtifactId::new(text(&row[0])?),
        artifact_type: parse_type(&text(&row[1])?)?,
        content_hash: ContentHash::parse(&text(&row[2])?)?,
        size: int(&row[3])? as u64,
        producer_node: match &row[4] {
            Value::Text(s) => Some(s.clone()),
            _ => None,
        },
        input_artifacts,
        metadata: serde_json::from_str(&text(&row[7])?)
            .map_err(|e| PersistError::Database(e.to_string()))?,
        provenance: serde_json::from_str(&text(&row[8])?)
            .map_err(|e| PersistError::Database(e.to_string()))?,
        privacy_class: parse_privacy(&text(&row[5])?)?,
        version: int(&row[6])? as u32,
    }))
}

fn text(v: &Value) -> Result<String, PersistError> {
    match v {
        Value::Text(s) => Ok(s.clone()),
        other => Err(PersistError::Database(format!("列应为文本，实际 {other:?}"))),
    }
}

fn int(v: &Value) -> Result<i64, PersistError> {
    match v {
        Value::Int(i) => Ok(*i),
        other => Err(PersistError::Database(format!("列应为整数，实际 {other:?}"))),
    }
}

fn type_str(t: ArtifactType) -> &'static str {
    match t {
        ArtifactType::SourceTree => "source_tree",
        ArtifactType::Patch => "patch",
        ArtifactType::TestResult => "test_result",
        ArtifactType::Text => "text",
        ArtifactType::Json => "json",
        ArtifactType::Blob => "blob",
    }
}

fn parse_type(s: &str) -> Result<ArtifactType, PersistError> {
    Ok(match s {
        "source_tree" => ArtifactType::SourceTree,
        "patch" => ArtifactType::Patch,
        "test_result" => ArtifactType::TestResult,
        "text" => ArtifactType::Text,
        "json" => ArtifactType::Json,
        "blob" => ArtifactType::Blob,
        other => {
            return Err(PersistError::Database(format!(
                "未知 ArtifactType: {other}"
            )))
        }
    })
}

fn privacy_str(p: PrivacyClass) -> &'static str {
    match p {
        PrivacyClass::Public => "public",
        PrivacyClass::Personal => "personal",
        PrivacyClass::Private => "private",
        PrivacyClass::Secret => "secret",
        PrivacyClass::LocalOnly => "local_only",
    }
}

fn parse_privacy(s: &str) -> Result<PrivacyClass, PersistError> {
    Ok(match s {
        "public" => PrivacyClass::Public,
        "personal" => PrivacyClass::Personal,
        "private" => PrivacyClass::Private,
        "secret" => PrivacyClass::Secret,
        "local_only" => PrivacyClass::LocalOnly,
        other => {
            return Err(PersistError::Database(format!(
                "未知 PrivacyClass: {other}"
            )))
        }
    })
}
```

`ContentHash` 需要一个从字符串还原的入口。在 `content.rs` 追加：

```rust
impl ContentHash {
    /// 从已存的十六进制字符串还原。长度非 64 或含非十六进制字符时返回 `None`。
    pub fn parse(value: &str) -> Option<Self> {
        let ok = value.len() == 64
            && value.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        if ok {
            Some(Self(value.to_owned()))
        } else {
            None
        }
    }
}
```

并把 `persist.rs` 中的 `ContentHash::parse(&text(&row[2])?)?` 改为：

```rust
        content_hash: ContentHash::parse(&text(&row[2])?)
            .ok_or_else(|| PersistError::Database("content_hash 不是合法的十六进制摘要".to_owned()))?,
```

- [ ] **Step 4: 实现 graph 的持久化**

`crates/continuum-graph/src/persist.rs` 与上述结构相同，表按设计第 15 节：

```rust
//! 图、节点、端口、边的落库（§317）。

use crate::edge::EdgeKind;
use crate::graph::AdfirGraph;
use crate::ids::{ContractIdRef, GraphId, NodeId};
use crate::node::{Node, NodeState};
use continuum_artifact::ArtifactType;
use continuum_operator::{OperatorId, OperatorVersion};
use continuum_persist::{Migration, PersistError, Tx, Value};
use continuum_port::{Direction, Port, PortId};

pub fn p1_graph_migrations() -> Vec<Migration> {
    vec![Migration::new(
        20,
        "p1_graph",
        "CREATE TABLE adfir_graph (
            id TEXT PRIMARY KEY,
            version INTEGER NOT NULL,
            contract_id TEXT NOT NULL,
            entry_nodes TEXT NOT NULL,
            terminal_nodes TEXT NOT NULL
        );
        CREATE TABLE adfir_node (
            graph_id TEXT NOT NULL,
            node_id TEXT NOT NULL,
            operator_id TEXT NOT NULL,
            operator_version INTEGER NOT NULL,
            state TEXT NOT NULL,
            execution_policy TEXT NOT NULL,
            verification_policy TEXT NOT NULL,
            constraints TEXT NOT NULL,
            capabilities TEXT NOT NULL,
            PRIMARY KEY (graph_id, node_id)
        );
        CREATE TABLE adfir_port (
            graph_id TEXT NOT NULL,
            node_id TEXT NOT NULL,
            port_id TEXT NOT NULL,
            direction TEXT NOT NULL,
            name TEXT NOT NULL,
            artifact_type TEXT NOT NULL,
            PRIMARY KEY (graph_id, port_id)
        );
        CREATE TABLE adfir_edge (
            graph_id TEXT NOT NULL,
            from_node TEXT NOT NULL,
            from_port TEXT NOT NULL,
            to_node TEXT NOT NULL,
            to_port TEXT NOT NULL,
            kind TEXT NOT NULL
        );
        CREATE TABLE execution_profile (
            graph_id TEXT NOT NULL,
            node_id TEXT NOT NULL,
            attempt INTEGER NOT NULL,
            backend TEXT,
            timeout_ms INTEGER,
            retry_policy TEXT NOT NULL,
            cost_budget TEXT,
            PRIMARY KEY (graph_id, node_id, attempt)
        );
        CREATE TABLE node_attempt (
            graph_id TEXT NOT NULL,
            node_id TEXT NOT NULL,
            attempt INTEGER NOT NULL,
            state TEXT NOT NULL,
            failure_class TEXT,
            PRIMARY KEY (graph_id, node_id, attempt)
        );",
    )]
}

pub fn save_graph(tx: &Tx<'_>, graph: &AdfirGraph) -> Result<(), PersistError> {
    let entry: Vec<&str> = graph.entry_nodes().iter().map(|n| n.as_str()).collect();
    let terminal: Vec<&str> = graph.terminal_nodes().iter().map(|n| n.as_str()).collect();
    tx.execute(
        "INSERT INTO adfir_graph (id, version, contract_id, entry_nodes, terminal_nodes)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        &[
            Value::text(graph.id.as_str()),
            Value::Int(i64::from(graph.version)),
            Value::text(graph.contract_id.as_str()),
            Value::text(serde_json::to_string(&entry).expect("可序列化")),
            Value::text(serde_json::to_string(&terminal).expect("可序列化")),
        ],
    )?;

    for node in graph.nodes() {
        tx.execute(
            "INSERT INTO adfir_node
               (graph_id, node_id, operator_id, operator_version, state,
                execution_policy, verification_policy, constraints, capabilities)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            &[
                Value::text(graph.id.as_str()),
                Value::text(node.id.as_str()),
                Value::text(node.operator.id.as_str()),
                Value::Int(i64::from(node.operator.version.as_u32())),
                Value::text(state_str(node.state)),
                Value::text(serde_json::to_string(&node.execution_policy).expect("可序列化")),
                Value::text(serde_json::to_string(&node.verification_policy).expect("可序列化")),
                Value::text(serde_json::to_string(&node.constraints).expect("可序列化")),
                Value::text(serde_json::to_string(&node.capabilities).expect("可序列化")),
            ],
        )?;
        for port_id in node.inputs.iter().chain(node.outputs.iter()) {
            let (_, port) = graph
                .port(port_id)
                .ok_or_else(|| PersistError::Database(format!("端口 {port_id:?} 缺失")))?;
            tx.execute(
                "INSERT INTO adfir_port
                   (graph_id, node_id, port_id, direction, name, artifact_type)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                &[
                    Value::text(graph.id.as_str()),
                    Value::text(node.id.as_str()),
                    Value::text(port.id().as_str()),
                    Value::text(match port.direction() {
                        Direction::Input => "input",
                        Direction::Output => "output",
                    }),
                    Value::text(port.name()),
                    Value::text(artifact_type_str(port.artifact_type())),
                ],
            )?;
        }
    }

    for edge in graph.edges() {
        tx.execute(
            "INSERT INTO adfir_edge
               (graph_id, from_node, from_port, to_node, to_port, kind)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            &[
                Value::text(graph.id.as_str()),
                Value::text(edge.from_node.as_str()),
                Value::text(edge.from_port.as_str()),
                Value::text(edge.to_node.as_str()),
                Value::text(edge.to_port.as_str()),
                Value::text(edge_kind_str(edge.kind)),
            ],
        )?;
    }
    Ok(())
}
```

`load_graph` 用 `AdfirGraph` 的公开构造入口重建，不访问私有字段：

```rust
pub fn load_graph(tx: &Tx<'_>, id: &GraphId) -> Result<Option<AdfirGraph>, PersistError> {
    let head = tx.query(
        "SELECT id, version, contract_id, entry_nodes, terminal_nodes
         FROM adfir_graph WHERE id = ?1",
        &[Value::text(id.as_str())],
    )?;
    let Some(head) = head.into_iter().next() else {
        return Ok(None);
    };

    let mut graph = AdfirGraph::new(
        GraphId::new(text(&head[0])?),
        ContractIdRef::new(text(&head[2])?),
    );
    graph.version = int(&head[1])? as u32;

    let nodes = tx.query(
        "SELECT node_id, operator_id, operator_version, state,
                execution_policy, verification_policy, constraints, capabilities
         FROM adfir_node WHERE graph_id = ?1 ORDER BY node_id",
        &[Value::text(id.as_str())],
    )?;
    for row in &nodes {
        let mut node = Node::new(
            NodeId::new(text(&row[0])?),
            OperatorId::new(text(&row[1])?),
            OperatorVersion::new(int(&row[2])? as u32),
        );
        node.state = parse_state(&text(&row[3])?)?;
        node.execution_policy = parse_json(&text(&row[4])?)?;
        node.verification_policy = parse_json(&text(&row[5])?)?;
        // 这两项若不回填会静默归零：Node::new 把它们初始化为空 Vec。
        // 它们是 Vec<String> 而非 serde_json::Value，故走 parse_strings。
        node.constraints = parse_strings(&text(&row[6])?)?;
        node.capabilities = parse_strings(&text(&row[7])?)?;
        graph.add_node(node)?;
    }

    let ports = tx.query(
        "SELECT node_id, port_id, direction, name, artifact_type
         FROM adfir_port WHERE graph_id = ?1 ORDER BY port_id",
        &[Value::text(id.as_str())],
    )?;
    for row in &ports {
        let direction = match text(&row[2])?.as_str() {
            "input" => Direction::Input,
            "output" => Direction::Output,
            other => {
                return Err(PersistError::Database(format!("未知 direction: {other}")))
            }
        };
        graph.add_port(
            NodeId::new(text(&row[0])?),
            Port::new(
                PortId::new(text(&row[1])?),
                direction,
                text(&row[3])?,
                parse_artifact_type(&text(&row[4])?)?,
            ),
        )?;
    }

    // 每条边都经 connect 重建，因此 §239 的校验在读回路径上同样生效
    let edges = tx.query(
        "SELECT from_port, to_port, kind FROM adfir_edge
         WHERE graph_id = ?1 ORDER BY from_port, to_port",
        &[Value::text(id.as_str())],
    )?;
    for row in &edges {
        graph.connect(
            &PortId::new(text(&row[0])?),
            &PortId::new(text(&row[1])?),
            parse_edge_kind(&text(&row[2])?)?,
        )?;
    }

    let entry: Vec<String> =
        serde_json::from_str(&text(&head[3])?).map_err(|e| PersistError::Database(e.to_string()))?;
    let terminal: Vec<String> =
        serde_json::from_str(&text(&head[4])?).map_err(|e| PersistError::Database(e.to_string()))?;
    graph.set_entry_nodes(entry.into_iter().map(NodeId::new).collect());
    graph.set_terminal_nodes(terminal.into_iter().map(NodeId::new).collect());

    // 读回路径同样要过设计 §8.3 的两条约束校验。这是 validate() 在本子项目内的
    // 唯一调用点：图从库中读回后投入使用前必须成立。
    graph
        .validate()
        .map_err(|e| PersistError::Database(e.to_string()))?;

    Ok(Some(graph))
}

fn parse_json(s: &str) -> Result<serde_json::Value, PersistError> {
    serde_json::from_str(s).map_err(|e| PersistError::Database(e.to_string()))
}

/// 与 `parse_json` 同构，仅返回类型不同：`Node.constraints` 与
/// `Node.capabilities` 是 `Vec<String>` 而非 `serde_json::Value`。
fn parse_strings(s: &str) -> Result<Vec<String>, PersistError> {
    serde_json::from_str(s).map_err(|e| PersistError::Database(e.to_string()))
}
```

`load_graph` 把 `GraphError` 转为 `PersistError`：`GraphError` 未实现 `From`，用 `map_err(|e| PersistError::Database(e.to_string()))?` 逐处转换，或在文件内加一个 `fn graph_err(e: GraphError) -> PersistError` 辅助函数。

字符串映射辅助函数（`state_str`、`artifact_type_str`、`edge_kind_str` 及其反向 `parse_state` / `parse_artifact_type` / `parse_edge_kind`）与 Step 3 的 `type_str` / `parse_type` 同形，逐个覆盖全部变体；未识别的字符串一律返回 `PersistError::Database`，不得回退到默认值。

`crates/continuum-artifact/src/lib.rs` 与 `crates/continuum-graph/src/lib.rs` 各追加：

```rust
pub mod persist;
```

并在 artifact 侧导出：

```rust
pub use persist::{load_artifact, p1_artifact_migrations, save_artifact};
```

graph 侧导出：

```rust
pub use persist::{load_graph, p1_graph_migrations, save_graph};
```

- [ ] **Step 5: 运行测试确认通过**

Run: `cargo test -p continuum-graph -v`
Expected: PASS，59 passed（Task 10 补 backoff 用例后为 55，本 task 4 条）。

- [ ] **Step 6: 提交**

```bash
git add crates/continuum-artifact crates/continuum-graph
git commit -m "feat(p1): 图与 Artifact 元数据的持久化"
```

---

### Task 12: runtime 装配与恢复钩子

> **订正（2026-10-01，合并前终审后补）。** 同 Task 11：本节代码块里的 `'RUNNING'`、
> `'VERIFYING'`、`'LOST'`、`'UNKNOWN'` 应为小写。实际实现已统一为小写。
> **照抄本节代码时请一律改用小写。**
>
> 另有 `node_attempt` 的 attempt 列：代码块写死为 `1`，实际实现已改为
> `(SELECT COALESCE(MAX(attempt), 0) + 1 FROM node_attempt WHERE graph_id = ?1 AND node_id = ?2)`，
> 以免同一节点的第二次尝试覆盖第一次的记录。`attempt` 自 1 起计，与 `execution_profile`
> 共用同一套编号，见设计第 15 节。


**Files:**
- Modify: `crates/continuum-runtime/src/main.rs`
- Modify: `crates/continuum-runtime/Cargo.toml`
- Create: `crates/continuum-runtime/src/recovery.rs`
- Test: `crates/continuum-runtime/tests/startup.rs`

**Interfaces:**
- Consumes: Task 11 的两个迁移函数与读写函数、P0 的 `RecoveryHook`/`RecoveryPhase`/`RecoveryRegistry`
- Produces: 启动流程在 P0 的迁移之外注册 P1 的迁移与恢复钩子

- [ ] **Step 1: 扩充启动测试**

在 `crates/continuum-runtime/tests/startup.rs` 追加：

```rust
#[test]
fn startup_applies_p1_migrations() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let out = run(&path);
    // P0 两条 + P1 两条
    assert!(out.contains("迁移应用 4 项"), "实际输出:\n{out}");
}

#[test]
fn recovery_marks_running_nodes_as_lost() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    run(&path);

    // 造一个 RUNNING 节点，再启动一次，断言它被标记为 LOST
    {
        let db = continuum_persist::Db::open(&path).unwrap();
        let tx = db.begin().unwrap();
        tx.execute(
            "INSERT INTO adfir_graph
               (id, version, contract_id, entry_nodes, terminal_nodes)
             VALUES (?1, ?2, ?3, '[]', '[]')",
            &[
                continuum_persist::Value::text("g1"),
                continuum_persist::Value::Int(1),
                continuum_persist::Value::text("c1"),
            ],
        )
        .unwrap();
        tx.execute(
            "INSERT INTO adfir_node
               (graph_id, node_id, operator_id, operator_version, state,
                execution_policy, verification_policy, constraints, capabilities)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            &[
                continuum_persist::Value::text("g1"),
                continuum_persist::Value::text("n1"),
                continuum_persist::Value::text("op"),
                continuum_persist::Value::Int(1),
                continuum_persist::Value::text("RUNNING"),
                continuum_persist::Value::text("null"),
                continuum_persist::Value::text("null"),
                continuum_persist::Value::text("[]"),
                continuum_persist::Value::text("[]"),
            ],
        )
        .unwrap();
        tx.commit().unwrap();
    }

    let out = run(&path);
    assert!(out.contains("标记 LOST 1 个节点"), "实际输出:\n{out}");

    let db = continuum_persist::Db::open(&path).unwrap();
    let tx = db.begin().unwrap();
    let rows = tx
        .query("SELECT state FROM adfir_node WHERE node_id = 'n1'", &[])
        .unwrap();
    match &rows[0][0] {
        continuum_persist::Value::Text(s) => assert_eq!(s, "LOST"),
        other => panic!("state 应为文本，实际 {other:?}"),
    }
}
```

第一次启动的迁移条数从 2 变为 4，既有的三条用例都要同步：

```
startup_applies_migrations_and_runs_five_phases   「迁移应用 2 项」→ 4 项
second_startup_applies_no_migration               不变（第二次启动仍为 0）
startup_reports_skipped_records                   「迁移应用 0 项」→ 2 项
```

第三条的原因是：该用例用 `Db::open` 预迁移（只含 P0 的两条内置迁移），而二进制启动时还要补应用 P1 的两条，故是 2 项。该用例的真实目的是证明跳过计数从库里读出，迁移数只是顺带——更新断言时加注释说明，不要改动它的其余部分。

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test -p continuum-runtime --test startup -v`
Expected: FAIL，`迁移应用 4 项` 不在输出中。

- [ ] **Step 3: 实现恢复钩子**

`crates/continuum-runtime/src/recovery.rs`

`crates/continuum-runtime/tests/startup.rs` 另需一条用例，它是 Task 11 那处主键修复的唯一守卫：

```rust
#[test]
fn recovery_handles_same_named_nodes_in_different_graphs() {
    // node_id 只在图内唯一。两张图各有 n1 时，node_attempt 的主键若不含
    // graph_id，第二条插入会撞键。Task 11 里无处可写这条用例（它不写该表）。
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    run(&path);

    {
        let db = continuum_persist::Db::open(&path).unwrap();
        let tx = db.begin().unwrap();
        for graph in ["g1", "g2"] {
            tx.execute(
                "INSERT INTO adfir_graph (id, version, contract_id, entry_nodes, terminal_nodes)
                 VALUES (?1, 1, 'c1', '[]', '[]')",
                &[continuum_persist::Value::text(graph)],
            )
            .unwrap();
            tx.execute(
                "INSERT INTO adfir_node
                   (graph_id, node_id, operator_id, operator_version, state,
                    execution_policy, verification_policy, constraints, capabilities)
                 VALUES (?1, 'n1', 'op', 1, 'RUNNING', 'null', 'null', '[]', '[]')",
                &[continuum_persist::Value::text(graph)],
            )
            .unwrap();
        }
        tx.commit().unwrap();
    }

    let out = run(&path);
    assert!(out.contains("标记 LOST 2 个节点"), "实际输出:\n{out}");

    let db = continuum_persist::Db::open(&path).unwrap();
    let tx = db.begin().unwrap();
    let rows = tx.query("SELECT COUNT(*) FROM node_attempt", &[]).unwrap();
    match &rows[0][0] {
        continuum_persist::Value::Int(n) => {
            assert_eq!(*n, 2, "两张图各应有一条 attempt 记录")
        }
        other => panic!("计数应为整数，实际 {other:?}"),
    }
}
```

另两条用例覆盖钩子的两个无守卫分支：

```rust
#[test]
fn recovery_marks_verifying_nodes_as_lost() {
    // §319 的 mark lost executions：崩溃时处于 VERIFYING 的节点同样丢失。
    // 该用例是钩子里 VERIFYING 分支的唯一守卫——没有它，
    // 把 VERIFYING 从 WHERE 条件里删掉不会有任何用例变红。
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    run(&path);
    {
        let db = continuum_persist::Db::open(&path).unwrap();
        let tx = db.begin().unwrap();
        tx.execute(
            "INSERT INTO adfir_graph (id, version, contract_id, entry_nodes, terminal_nodes)
             VALUES ('g1', 1, 'c1', '[]', '[]')",
            &[],
        )
        .unwrap();
        tx.execute(
            "INSERT INTO adfir_node
               (graph_id, node_id, operator_id, operator_version, state,
                execution_policy, verification_policy, constraints, capabilities)
             VALUES ('g1', 'n1', 'op', 1, 'VERIFYING', 'null', 'null', '[]', '[]')",
            &[],
        )
        .unwrap();
        tx.commit().unwrap();
    }

    let out = run(&path);
    assert!(out.contains("标记 LOST 1 个节点"), "实际输出:\n{out}");
}

#[test]
fn recovery_does_not_touch_same_named_nodes_in_other_graphs() {
    // 钩子的 UPDATE 必须带 graph_id：否则会改到另一张图里同名的已完成节点。
    // 该用例是那条 WHERE 条件的唯一守卫。
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    run(&path);
    {
        let db = continuum_persist::Db::open(&path).unwrap();
        let tx = db.begin().unwrap();
        for (graph, state) in [("g1", "RUNNING"), ("g2", "COMPLETED")] {
            tx.execute(
                "INSERT INTO adfir_graph (id, version, contract_id, entry_nodes, terminal_nodes)
                 VALUES (?1, 1, 'c1', '[]', '[]')",
                &[continuum_persist::Value::text(graph)],
            )
            .unwrap();
            tx.execute(
                "INSERT INTO adfir_node
                   (graph_id, node_id, operator_id, operator_version, state,
                    execution_policy, verification_policy, constraints, capabilities)
                 VALUES (?1, 'n1', 'op', 1, ?2, 'null', 'null', '[]', '[]')",
                &[
                    continuum_persist::Value::text(graph),
                    continuum_persist::Value::text(state),
                ],
            )
            .unwrap();
        }
        tx.commit().unwrap();
    }

    run(&path);

    let db = continuum_persist::Db::open(&path).unwrap();
    let tx = db.begin().unwrap();
    let rows = tx
        .query("SELECT graph_id, state FROM adfir_node ORDER BY graph_id", &[])
        .unwrap();
    let states: Vec<(String, String)> = rows
        .iter()
        .map(|r| {
            let graph = match &r[0] {
                continuum_persist::Value::Text(s) => s.clone(),
                other => panic!("graph_id 应为文本，实际 {other:?}"),
            };
            let state = match &r[1] {
                continuum_persist::Value::Text(s) => s.clone(),
                other => panic!("state 应为文本，实际 {other:?}"),
            };
            (graph, state)
        })
        .collect();
    assert_eq!(
        states,
        vec![
            ("g1".to_owned(), "LOST".to_owned()),
            ("g2".to_owned(), "COMPLETED".to_owned())
        ],
        "另一张图里同名的已完成节点不得被改动"
    );
}
```

```rust
//! P1 注册到 P0 恢复流程的钩子。

use continuum_persist::{PersistError, RecoveryHook, RecoveryPhase, Tx, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// 把处于 RUNNING 或 VERIFYING 的节点标记为 LOST（§319 的
/// `reconcile running nodes` 与 `mark lost executions`）。
pub struct MarkRunningNodesLost {
    marked: Arc<AtomicUsize>,
}

impl MarkRunningNodesLost {
    pub fn new() -> Self {
        Self {
            marked: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// 供启动流程读取本次标记的节点数。
    pub fn counter(&self) -> Arc<AtomicUsize> {
        Arc::clone(&self.marked)
    }
}

impl RecoveryHook for MarkRunningNodesLost {
    fn phase(&self) -> RecoveryPhase {
        RecoveryPhase::ReconcileRunningNodes
    }

    fn run(&self, tx: &Tx<'_>) -> Result<(), PersistError> {
        let rows = tx.query(
            "SELECT graph_id, node_id FROM adfir_node WHERE state IN ('RUNNING', 'VERIFYING')",
            &[],
        )?;
        for row in &rows {
            let graph_id = match &row[0] {
                Value::Text(s) => s.clone(),
                other => {
                    return Err(PersistError::Database(format!(
                        "graph_id 应为文本，实际 {other:?}"
                    )))
                }
            };
            let node_id = match &row[1] {
                Value::Text(s) => s.clone(),
                other => {
                    return Err(PersistError::Database(format!(
                        "node_id 应为文本，实际 {other:?}"
                    )))
                }
            };
            // node_id 只在图内唯一，两张图各有 "n1" 会撞主键，故带 graph_id
            tx.execute(
                "UPDATE adfir_node SET state = 'LOST' WHERE graph_id = ?1 AND node_id = ?2",
                &[Value::text(graph_id.clone()), Value::text(node_id.clone())],
            )?;
            tx.execute(
                "INSERT OR REPLACE INTO node_attempt
                   (graph_id, node_id, attempt, state, failure_class)
                 VALUES (?1, ?2, 1, 'LOST', 'UNKNOWN')",
                &[Value::text(graph_id), Value::text(node_id)],
            )?;
        }
        self.marked.store(rows.len(), Ordering::Relaxed);
        Ok(())
    }
}
```

钩子持有 `Arc<AtomicUsize>` 而不是自己在 `run` 里打印：`RecoveryHook::run` 只拿到 `&Tx`，没有输出通道，而计数的消费者是启动流程。

- [ ] **Step 4: 装配启动流程**

`crates/continuum-runtime/src/main.rs` 的 `startup` 改为：

```rust
fn startup(path: &Path) -> Result<(), PersistError> {
    let mut migrations = continuum_persist::builtin_migrations();
    migrations.extend(continuum_artifact::p1_artifact_migrations());
    migrations.extend(continuum_graph::p1_graph_migrations());

    let db = Db::open_with(path, migrations)?;
    let applied = db.migrate()?;
    println!("迁移应用 {applied} 项");

    let hook = recovery::MarkRunningNodesLost::new();
    let marked = hook.counter();

    let mut registry = RecoveryRegistry::new();
    registry.register(Box::new(hook));
    let report = run_recovery(&db, &registry)?;
    println!("跳过记录 {} 条", report.skipped_records);
    println!(
        "标记 LOST {} 个节点",
        marked.load(std::sync::atomic::Ordering::Relaxed)
    );
    for phase in &report.phases {
        println!("{}: {} 钩子", phase.phase.as_str(), phase.hooks_run);
    }
    Ok(())
}
```

`crates/continuum-runtime/Cargo.toml` 的 `[dependencies]` 追加：

```toml
continuum-artifact = { path = "../continuum-artifact" }
continuum-graph = { path = "../continuum-graph" }
```

同时扩充 `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED` 表中 runtime 那一行，把 `continuum-artifact` 与 `continuum-graph` 加入允许集合——本 task 给它加了两条直接依赖，不更新该行会使 `every_crate_depends_only_on_its_allowed_set` 失败。

- [ ] **Step 5: 运行测试确认通过**

Run: `cargo test --workspace`
Expected: 全部 PASS、0 warning。计数：P0 的 62 加本子项目在 Task 2 至 11 新增的测试（artifact 7、port 4、operator 4、graph 50），runtime 由 4 增至 6，共 133 passed。

实际数与上述不符时**如实报告实际数**。

- [ ] **Step 6: 提交**

```bash
git add crates/continuum-runtime
git commit -m "feat(runtime): 装配 P1 迁移与恢复钩子"
```

---

### Task 13: 执行接口与执行档案（补设计 §11.2 与 §14）

**背景：** 本 task 补三处设计覆盖：§11.2 的 `OperatorImpl` 与 `NodeContext`/`ArtifactRef`、§11.1 的 `§245` 后端解析接口、§14 的 `ExecutionProfile`。它们在 Task 4 的评审中被发现无任何 task 覆盖，根因是设计未指定这三个类型的归属 crate。归属已裁定为 `continuum-graph`（`NodeContext` 需携带 `NodeId`，放 `operator` 会与依赖方向冲突）。

**依赖：** 本 task 用到 Task 5 的 `NodeId` 与 Task 10 的 `RetryPolicy`，**必须在 Task 10 之后执行**。

**Files:**
- Create: `crates/continuum-graph/src/execution.rs`
- Modify: `crates/continuum-graph/src/lib.rs`
- Modify: `crates/continuum-graph/src/failure.rs`
- Test: `crates/continuum-graph/tests/execution.rs`

`failure.rs` 的改动是给 `RetryPolicy` 加 `Default`（`ExecutionProfile` 的该字段非可选）。它在 Task 10 已实现，本次为跨 task 的补加。

**Interfaces:**
- `continuum_graph::ExecutionProfile`
- `continuum_graph::ArtifactRef`
- `continuum_graph::NodeContext`：`new`、`cancel`、`is_cancelled`
- `continuum_graph::OperatorImpl`
- `continuum_graph::is_candidate_backend`

- [ ] **Step 1: 写执行接口的测试**

`crates/continuum-graph/tests/execution.rs`

```rust
use continuum_artifact::{Artifact, ArtifactId, ArtifactType, ContentHash, PrivacyClass};
use continuum_graph::{
    is_candidate_backend, ArtifactRef, ExecutionProfile, NodeContext, NodeId, OperatorImpl,
};
use continuum_operator::{
    BackendId, Determinism, Operator, OperatorId, OperatorVersion, SideEffectClass,
};
use serde_json::json;

fn op(backends: &[&str]) -> Operator {
    Operator {
        id: OperatorId::new("op"),
        version: OperatorVersion::new(1),
        input_schema: vec![ArtifactType::Text],
        output_schema: vec![ArtifactType::Text],
        determinism: Determinism::Deterministic,
        side_effect_class: SideEffectClass::Pure,
        backend_candidates: backends.iter().map(|b| BackendId::new(*b)).collect(),
    }
}

#[test]
fn execution_profile_starts_empty() {
    // 本子项目内四个资源字段恒为 None：P3 与 P7 之前无对应资源
    let p = ExecutionProfile::default();
    assert!(p.model.is_none());
    assert!(p.provider.is_none());
    assert!(p.tool.is_none());
    assert!(p.backend.is_none());
    assert!(p.compute_node.is_none());
    assert!(p.reasoning_effort.is_none());
    assert!(p.parallelism.is_none());
    assert!(p.timeout_ms.is_none());
    assert!(p.cost_budget.is_none());
    // retry_policy 非可选：默认策略是「单次尝试、不重试」
    assert_eq!(p.retry_policy.max_attempts, 1);
    assert!(p.retry_policy.retryable_errors.is_empty());
}

#[test]
fn context_starts_uncancelled_and_can_be_cancelled() {
    let ctx = NodeContext::new(NodeId::new("n1"), ExecutionProfile::default());
    assert!(!ctx.is_cancelled());
    ctx.cancel();
    assert!(ctx.is_cancelled(), "取消后应可观察到");
}

#[test]
fn a_second_handle_observes_cancellation() {
    // 取消信号必须能被另一个持有者看到，否则执行中的算子无法中止
    let ctx = NodeContext::new(NodeId::new("n1"), ExecutionProfile::default());
    let ctx2 = ctx.clone();
    ctx.cancel();
    assert!(ctx2.is_cancelled(), "取消信号应跨句柄可见");
}

#[test]
fn declared_backend_is_a_candidate() {
    let operator = op(&["builtin", "remote"]);
    assert!(is_candidate_backend(&operator, &BackendId::new("builtin")));
    assert!(is_candidate_backend(&operator, &BackendId::new("remote")));
}

#[test]
fn undeclared_backend_is_not_a_candidate() {
    let operator = op(&["builtin"]);
    assert!(!is_candidate_backend(&operator, &BackendId::new("remote")));
}

/// 一个确定性算子实现。仅用于证明 OperatorImpl 可被实现，
/// 并证明 ArtifactRef 能被读取。
struct UpperCase;

impl OperatorImpl for UpperCase {
    fn execute(
        &self,
        inputs: &[ArtifactRef],
        _ctx: &NodeContext,
    ) -> Result<Vec<Artifact>, continuum_operator::OperatorError> {
        let first = inputs.first().ok_or_else(|| {
            continuum_operator::OperatorError::NotFound {
                id: OperatorId::new("input"),
                version: OperatorVersion::new(0),
            }
        })?;
        Ok(vec![Artifact {
            id: ArtifactId::new("out"),
            artifact_type: ArtifactType::Text,
            content_hash: ContentHash::of(first.id.as_str().as_bytes()),
            size: 0,
            producer_node: None,
            input_artifacts: vec![first.id.clone()],
            metadata: json!({}),
            provenance: json!({}),
            privacy_class: PrivacyClass::Personal,
            version: 1,
        }])
    }
}

#[test]
fn operator_impl_is_implementable_and_reads_its_inputs() {
    let implementation = UpperCase;
    let input = ArtifactRef {
        id: ArtifactId::new("a1"),
        content_hash: ContentHash::of(b"a1"),
    };
    let ctx = NodeContext::new(NodeId::new("n1"), ExecutionProfile::default());

    let out = implementation.execute(&[input.clone()], &ctx).expect("应能执行");
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].input_artifacts, vec![ArtifactId::new("a1")]);
    assert_eq!(out[0].content_hash, ContentHash::of(b"a1"));
}

#[test]
fn operator_impl_can_report_failure() {
    let implementation = UpperCase;
    let ctx = NodeContext::new(NodeId::new("n1"), ExecutionProfile::default());
    assert!(
        implementation.execute(&[], &ctx).is_err(),
        "无输入时应返回错误而非 panic"
    );
}
```

`NodeContext` 需要 `Clone`（`a_second_handle_observes_cancellation` 用到）。

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test -p continuum-graph --test execution -v`
Expected: FAIL，编译错误 `no struct named ExecutionProfile`

- [ ] **Step 3: 实现 execution.rs**

`crates/continuum-graph/src/execution.rs`

```rust
//! 节点执行接口（设计 §11.2）与执行档案（§246）。
//!
//! 三个类型定义在本 crate 而非 `continuum-operator`：`NodeContext` 需要携带
//! `NodeId`，放在 operator 会使其反向依赖本 crate。

use crate::failure::RetryPolicy;
use crate::ids::NodeId;
use continuum_artifact::{Artifact, ArtifactId, ContentHash};
use continuum_operator::{BackendId, Operator, OperatorError};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// `§246`。每次 Node 执行产生一条。
///
/// 六处字段在本子项目内恒为 None 并以 `String` 承载：`model`、`provider`、
/// `tool`、`compute_node`、`reasoning_effort`、`cost_budget`。
/// 前四者 P3 接入 `ModelProvider` 与 `ToolProvider` 时收紧为强类型 id；
/// 后两者的对应类型在本子项目内不存在。`backend` 已是强类型 `BackendId`。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExecutionProfile {
    pub model: Option<String>,
    pub provider: Option<String>,
    pub tool: Option<String>,
    pub backend: Option<BackendId>,
    pub compute_node: Option<String>,
    pub reasoning_effort: Option<String>,
    pub parallelism: Option<u32>,
    pub timeout_ms: Option<u64>,
    /// 非可选：设计 §14 与第 15 节的表列都是非空。默认值为「单次尝试、不重试」。
    pub retry_policy: RetryPolicy,
    pub cost_budget: Option<String>,
}

/// 对输入 Artifact 的引用。只带定位所需的两个身份（设计第 4.1 节）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactRef {
    pub id: ArtifactId,
    pub content_hash: ContentHash,
}

/// 节点执行上下文。
#[derive(Debug, Clone)]
pub struct NodeContext {
    node: NodeId,
    profile: ExecutionProfile,
    cancelled: Arc<AtomicBool>,
}

impl NodeContext {
    pub fn new(node: NodeId, profile: ExecutionProfile) -> Self {
        Self {
            node,
            profile,
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn node(&self) -> &NodeId {
        &self.node
    }

    pub fn profile(&self) -> &ExecutionProfile {
        &self.profile
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

/// 设计 §11.2 的算子执行接口。本子项目只定义，不提供领域实现。
pub trait OperatorImpl: Send + Sync {
    fn execute(
        &self,
        inputs: &[ArtifactRef],
        ctx: &NodeContext,
    ) -> Result<Vec<Artifact>, OperatorError>;
}

/// `§245` 的后端解析接口：判定给定后端是否在 Operator 声明的候选列表内。
///
/// 只做判定，不做选择。具体后端的选择由 Router 决定（P3）。
pub fn is_candidate_backend(operator: &Operator, backend: &BackendId) -> bool {
    operator.backend_candidates.contains(backend)
}
```

`crates/continuum-graph/src/lib.rs` 追加：

```rust
pub mod execution;

pub use execution::{is_candidate_backend, ArtifactRef, ExecutionProfile, NodeContext, OperatorImpl};
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --workspace`
Expected: 全部 PASS、0 warning。计数：Task 12 结束时为 133，本 task 新增 7 条，共 **140 passed**。实际数不符时如实报告。

- [ ] **Step 5: 提交**

```bash
git add crates/continuum-graph
git commit -m "feat(graph): 执行接口与执行档案（补设计 §11.2 与 §14）"
```

---

### Task 14: 磁盘内容寻址存储与 ArtifactStore 落库桥（补设计 §15 与 §242）

**背景：** 设计 §15 明写「Artifact 的二进制内容按 `content_hash` 寻址落磁盘，不存库。元数据入库。」P1 终审确认：`crates/` 下无 `std::fs` 调用，磁盘存储不存在；`ArtifactStore`（内存）与 `save_artifact`（落库）互不引用，于是 P1 有两条互不相通的 Artifact 写入路径，而唯一断言「相同内容只存一份」的用例打在内存路径上，真正持久的路径只有一个非唯一索引。本 task 闭合这两处。

**依赖：** Task 11（`save_artifact` / `load_artifact` / `p1_artifact_migrations`）。

**Files:**
- Create: `crates/continuum-artifact/src/blobstore.rs`
- Modify: `crates/continuum-artifact/src/lib.rs`
- Modify: `crates/continuum-artifact/src/store.rs`
- Test: `crates/continuum-artifact/tests/blobstore.rs`
- Test: `crates/continuum-artifact/tests/artifact_store.rs`

`crates/continuum-artifact/Cargo.toml` 目前**没有** `[dev-dependencies]` 段，两个测试都要建临时库，
需补 `tempfile = { workspace = true }`。`continuum-persist` 已是正式依赖，测试可直接用。

`artifact_store.rs:6` 已有夹具 `fn artifact(id: &str, bytes: &[u8], inputs: Vec<ArtifactId>) -> Artifact`，
直接复用，不要另造。

**Interfaces:**
- Produces: `continuum_artifact::BlobStore` —— `new`、`put`、`get`、`contains`
- Produces: `continuum_artifact::BlobError` —— `Io` / `Corrupt` / `Missing`
- Produces: `ArtifactStore::persist(&self, tx) -> Result<(), PersistError>`
- Produces: `ArtifactStore::restore(tx) -> Result<Self, PersistError>`

- [ ] **Step 1: 写 BlobStore 的测试**

`crates/continuum-artifact/tests/blobstore.rs`

```rust
use continuum_artifact::{BlobStore, ContentHash};

#[test]
fn same_content_is_stored_once() {
    let dir = tempfile::tempdir().unwrap();
    let store = BlobStore::new(dir.path());

    let a = store.put(b"hello").unwrap();
    let b = store.put(b"hello").unwrap();

    assert_eq!(a, b, "相同内容必须得到相同哈希");
    assert_eq!(a, ContentHash::of(b"hello"));
    // 磁盘上只有一份：目录下的文件总数不随重复写入增长
    assert_eq!(count_files(dir.path()), 1);
}

#[test]
fn different_content_lands_on_different_paths() {
    let dir = tempfile::tempdir().unwrap();
    let store = BlobStore::new(dir.path());
    store.put(b"one").unwrap();
    store.put(b"two").unwrap();
    assert_eq!(count_files(dir.path()), 2);
}

#[test]
fn content_round_trips_through_disk() {
    let dir = tempfile::tempdir().unwrap();
    let store = BlobStore::new(dir.path());
    let hash = store.put(b"\x00\xff binary").unwrap();
    assert_eq!(store.get(&hash).unwrap(), b"\x00\xff binary");
}

#[test]
fn tampered_content_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let store = BlobStore::new(dir.path());
    let hash = store.put(b"original").unwrap();

    // 找到落盘文件并改写它，模拟存储被篡改
    let path = find_file(dir.path());
    std::fs::write(&path, b"tampered").unwrap();

    let err = store.get(&hash).unwrap_err();
    assert!(matches!(err, continuum_artifact::BlobError::Corrupt { .. }), "实际 {err:?}");
}

#[test]
fn missing_content_reports_missing() {
    let dir = tempfile::tempdir().unwrap();
    let store = BlobStore::new(dir.path());
    let err = store.get(&ContentHash::of(b"never written")).unwrap_err();
    assert!(matches!(err, continuum_artifact::BlobError::Missing { .. }), "实际 {err:?}");
}

fn count_files(root: &std::path::Path) -> usize {
    walk(root).len()
}

fn find_file(root: &std::path::Path) -> std::path::PathBuf {
    walk(root).into_iter().next().expect("至少有一个文件")
}

fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(walk(&path));
        } else {
            out.push(path);
        }
    }
    out
}
```

- [ ] **Step 2: 运行测试，确认失败**

```bash
cargo test -p continuum-artifact --test blobstore
```

预期：编译失败，`BlobStore` 不存在。

- [ ] **Step 3: 实现 BlobStore**

`crates/continuum-artifact/src/blobstore.rs`

```rust
//! 内容寻址的二进制存储（设计 §15、§242）。
//!
//! 内容不存库：按 `content_hash` 落磁盘，路径为 `<root>/<哈希前两位>/<哈希>`。
//! 该布局使得相同内容必然落在同一路径，磁盘上的去重由路径本身保证，
//! 不需要额外的索引或引用计数。

use crate::content::ContentHash;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum BlobError {
    #[error("内容 {hash} 的读写失败：{source}")]
    Io {
        hash: ContentHash,
        #[source]
        source: std::io::Error,
    },
    #[error("内容 {hash} 的字节与其哈希不符（存储已损坏或被篡改）")]
    Corrupt { hash: ContentHash },
    #[error("内容 {hash} 不存在")]
    Missing { hash: ContentHash },
}

pub struct BlobStore {
    root: PathBuf,
}

impl BlobStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// 内容的落盘路径。用哈希前两位分目录，避免单目录下文件过多。
    fn path_of(&self, hash: &ContentHash) -> PathBuf {
        let s = hash.as_str();
        self.root.join(&s[..2]).join(s)
    }

    /// 写入内容并返回其哈希。相同内容重复写入不产生第二份。
    pub fn put(&self, bytes: &[u8]) -> Result<ContentHash, BlobError> {
        let hash = ContentHash::of(bytes);
        let path = self.path_of(&hash);
        if path.exists() {
            return Ok(hash);
        }
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|source| BlobError::Io {
                hash: hash.clone(),
                source,
            })?;
        }
        fs::write(&path, bytes).map_err(|source| BlobError::Io {
            hash: hash.clone(),
            source,
        })?;
        Ok(hash)
    }

    /// 读回内容并校验哈希。校验失败说明存储被改动，返回 `Corrupt` 而不是坏数据。
    pub fn get(&self, hash: &ContentHash) -> Result<Vec<u8>, BlobError> {
        let path = self.path_of(hash);
        if !path.exists() {
            return Err(BlobError::Missing { hash: hash.clone() });
        }
        let bytes = fs::read(&path).map_err(|source| BlobError::Io {
            hash: hash.clone(),
            source,
        })?;
        if &ContentHash::of(&bytes) != hash {
            return Err(BlobError::Corrupt { hash: hash.clone() });
        }
        Ok(bytes)
    }

    pub fn contains(&self, hash: &ContentHash) -> bool {
        self.path_of(hash).exists()
    }
}
```

`crates/continuum-artifact/src/lib.rs` 加 `pub mod blobstore;` 与
`pub use blobstore::{BlobError, BlobStore};`。

- [ ] **Step 4: 运行测试，确认通过**

```bash
cargo test -p continuum-artifact --test blobstore
```

- [ ] **Step 5: 写落库桥的测试**

在 `crates/continuum-artifact/tests/artifact_store.rs` 追加（并新建一个临时库文件）：

```rust
#[test]
fn store_round_trips_through_the_database() {
    let dir = tempfile::tempdir().unwrap();
    let db = continuum_persist::Db::open_with(&dir.path().join("t.db"), migrations()).unwrap();
    let tx = db.begin().unwrap();

    let mut store = ArtifactStore::new();
    store.commit(artifact("a1", b"root", vec![])).unwrap();
    store.commit(artifact("a2", b"derived", vec![ArtifactId::new("a1")])).unwrap();
    store.persist(&tx).unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let restored = ArtifactStore::restore(&tx).unwrap();
    assert_eq!(restored.get(&ArtifactId::new("a1")), store.get(&ArtifactId::new("a1")));
    assert_eq!(restored.get(&ArtifactId::new("a2")), store.get(&ArtifactId::new("a2")));
    assert_eq!(restored.lineage(&ArtifactId::new("a2")), vec![ArtifactId::new("a1")]);
    assert_eq!(restored.stored_blob_count(), 2);
}

#[test]
fn deduplication_holds_on_the_persisted_path() {
    // 两个 id 不同的 Artifact 携带相同内容：落库后仍只算一份内容
    let dir = tempfile::tempdir().unwrap();
    let db = continuum_persist::Db::open_with(&dir.path().join("t.db"), migrations()).unwrap();
    let tx = db.begin().unwrap();

    let mut store = ArtifactStore::new();
    let a1 = artifact("a1", b"same", vec![]);
    let a2 = artifact("a2", b"same", vec![]); // 与 a1 内容相同、id 不同
    let hash = a1.content_hash.clone();
    assert_eq!(hash, a2.content_hash, "夹具前提：两者内容相同");
    store.commit(a1).unwrap();
    store.commit(a2).unwrap();
    store.persist(&tx).unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let restored = ArtifactStore::restore(&tx).unwrap();
    assert_eq!(restored.stored_blob_count(), 1, "相同内容只存一份");
    assert!(restored.find_by_hash(&hash).is_some());
}

#[test]
fn restore_follows_dependency_order_not_id_order() {
    // "a1" 的输入是 "z1"：按 id 字典序重建会先撞上输入缺失，按依赖序不会
    let dir = tempfile::tempdir().unwrap();
    let db = continuum_persist::Db::open_with(&dir.path().join("t.db"), migrations()).unwrap();
    let tx = db.begin().unwrap();

    let mut store = ArtifactStore::new();
    store.commit(artifact("z1", b"root", vec![])).unwrap();
    store.commit(artifact("a1", b"derived", vec![ArtifactId::new("z1")])).unwrap();
    store.persist(&tx).unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let restored = ArtifactStore::restore(&tx).expect("按依赖序重建应当成功");
    assert_eq!(restored.lineage(&ArtifactId::new("a1")), vec![ArtifactId::new("z1")]);
}

#[test]
fn restore_reports_dangling_inputs_instead_of_dropping_them() {
    // 库中存在一个输入不存在的 Artifact：重建必须报错，不能静默少一个
    let dir = tempfile::tempdir().unwrap();
    let db = continuum_persist::Db::open_with(&dir.path().join("t.db"), migrations()).unwrap();
    let tx = db.begin().unwrap();

    let mut store = ArtifactStore::new();
    store.commit(artifact("a1", b"root", vec![])).unwrap();
    store.persist(&tx).unwrap();
    // 绕过内存路径直接写一条输入悬空的记录
    tx.execute(
        "INSERT INTO artifact_input (artifact_id, input_artifact_id) VALUES ('a1', 'ghost')",
        &[],
    )
    .unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let err = ArtifactStore::restore(&tx).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("a1"), "错误应点名出问题的 Artifact，实际：{msg}");
}

fn migrations() -> Vec<continuum_persist::Migration> {
    let mut m = continuum_persist::builtin_migrations();
    m.extend(p1_artifact_migrations());
    m
}
```

`source_tree` / `patch` 是本文件已有的夹具函数；若名称不同按实际调整。`ArtifactStore`
与 `save_artifact` 已在同一 crate，`persist` / `restore` 内部应复用它们而非另写 SQL。

- [ ] **Step 6: 实现 persist / restore**

在 `crates/continuum-artifact/src/store.rs` 追加：

```rust
use crate::persist::{load_artifact, save_artifact};
use continuum_persist::{PersistError, Tx, Value};

impl ArtifactStore {
    /// 把内存中的全部 Artifact 元数据落入 `artifact` 与 `artifact_input`。
    ///
    /// 只使用调用方传入的 `Tx`，不自行开启或提交事务——提交由调用方负责，
    /// 以便与同一事务内的其他写入（如事件）一起原子生效。
    pub fn persist(&self, tx: &Tx<'_>) -> Result<(), PersistError> {
        for artifact in self.by_id.values() {
            save_artifact(tx, artifact)?;
        }
        Ok(())
    }

    /// 从库中重建。
    ///
    /// 按**依赖序**重建而非 id 序：`commit` 要求输入 Artifact 已存在，而 id
    /// 的字典序不保证输入在前（id 为 `z` 的 Artifact 可以依赖 `a`）。每轮提交
    /// 所有输入已就位的 Artifact，直到一轮之内没有进展；此时若仍有剩余，说明
    /// 存在环或输入的 id 缺失，返回错误而不是静默丢弃。
    ///
    /// `by_hash` 的「首次提交者胜出」随之由该顺序决定。
    pub fn restore(tx: &Tx<'_>) -> Result<Self, PersistError> {
        let rows = tx.query("SELECT id FROM artifact ORDER BY id", &[])?;
        let mut pending: Vec<Artifact> = Vec::with_capacity(rows.len());
        for row in rows {
            let id = match &row[0] {
                Value::Text(s) => ArtifactId::new(s.clone()),
                other => {
                    return Err(PersistError::Database(format!(
                        "artifact.id 应为文本，实际 {other:?}"
                    )))
                }
            };
            let artifact = load_artifact(tx, &id)?
                .ok_or_else(|| PersistError::Database(format!("artifact {id} 在枚举后读不回")))?;
            pending.push(artifact);
        }

        let mut store = Self::new();
        while !pending.is_empty() {
            let mut deferred = Vec::new();
            let mut progressed = false;
            for artifact in pending {
                if artifact
                    .input_artifacts
                    .iter()
                    .all(|input| store.get(input).is_some())
                {
                    // 走 commit 而不是直接插表：恢复到与内存路径同一套不变量
                    store.commit(artifact).map_err(|e| {
                        PersistError::Database(format!("恢复 Artifact 被拒：{e}"))
                    })?;
                    progressed = true;
                } else {
                    deferred.push(artifact);
                }
            }
            if !progressed {
                let blocked: Vec<&str> = deferred.iter().map(|a| a.id.as_str()).collect();
                return Err(PersistError::Database(format!(
                    "以下 Artifact 的输入在库中不存在或依赖成环：{blocked:?}"
                )));
            }
            pending = deferred;
        }
        Ok(store)
    }
}
```

`blobs` 字段与 `stored_blob_count` 保持不动（它的冗余清理已记为 P2 项）。

- [ ] **Step 7: 把 `store.rs` 的过时注释改掉**

`crates/continuum-artifact/src/store.rs:19` 现写「P1 的实现保存在内存中；落库在 Task 11」，
已不成立。改为说明内存结构是工作副本，经 `persist` / `restore` 与库往返。

- [ ] **Step 8: 运行全部测试并提交**

```bash
cargo test --workspace
git add -A
git commit -m "feat(artifact): 磁盘内容寻址存储与 ArtifactStore 落库桥"
```

---

### Task 15: 状态迁移与事件同事务写入（补设计 §16 与 §17 的事务边界判据）

**背景：** 设计 §16 要求 `NodeStarted` / `NodeCompleted` / `ArtifactCreated` 分别在节点进入 RUNNING、进入 COMPLETED、Artifact 入库时写入；§17 把「节点状态迁移与对应事件在同一事务内提交（§318）」列为 P1 的测试判据。P1 终审确认这三型在 `crates/` 下零引用，`transition()` 是纯函数、不接受 `Tx`，该判据无 task 覆盖。本 task 补上。

**依赖：** Task 6（迁移表）、Task 11（`save_graph` / `persist.rs`）、Task 14。

**Files:**
- Create: `crates/continuum-graph/src/transition_tx.rs`
- Modify: `crates/continuum-graph/src/lib.rs`
- Modify: `crates/continuum-graph/src/persist.rs`（`state_str` / `parse_state` 改 `pub(crate)`）
- Modify: `crates/continuum-artifact/src/persist.rs`（`save_artifact` 加事件参数）
- Modify: `crates/continuum-artifact/src/store.rs`（Task 14 的 `persist` 调用点随签名同步）
- Modify: `crates/continuum-artifact/Cargo.toml`（`[dependencies]` 加 `continuum-events`）
- Modify: `crates/continuum-graph/Cargo.toml`（`[dependencies]` 与 `[dev-dependencies]` 各加 `continuum-events`）
- Modify: `crates/continuum-runtime/tests/dependency_direction.rs`（`ALLOWED` 表）
- Test: `crates/continuum-graph/tests/transition_events.rs`
- Test: `crates/continuum-graph/tests/persistence.rs`（`save_artifact` 调用点同步）

**Interfaces:**
- Produces: `continuum_graph::apply_transition`
- Produces: `continuum_graph::ApplyError`
- 变更: `continuum_artifact::save_artifact(tx, artifact, event_id, occurred_at)`

**依赖方向的两条新边（设计第 5 节已修订）。** `continuum-graph` 与
`continuum-artifact` 都需要构造 `continuum_events::Event`，两者目前的 `Cargo.toml`
都没有该依赖，需补：

- `crates/continuum-artifact/Cargo.toml` 的 `[dependencies]` 加
  `continuum-events = { path = "../continuum-events" }`。
- `crates/continuum-graph/Cargo.toml` 的 `[dependencies]` 加同一条；**另需在
  `[dev-dependencies]` 也加一条**——`tests/transition_events.rs` 是独立 crate，
  要能直接写 `continuum_events::EventType`。（`tempfile` 已在 dev-dependencies 里。）
- `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED` 表加这两条边：
  `continuum-artifact` 的数组加 `"continuum-events"`，`continuum-graph` 的数组同样。
  该表用 `cargo tree --depth 1 --edges all` 取值，**dev-dependencies 也算直接边**，
  漏加会让 `every_crate_depends_only_on_its_allowed_set` 失败。

这两条边不构成环（`continuum-events` 只依赖 `continuum-core`）。

`EventType` 已派生 `Debug, Clone, Copy, PartialEq, Eq, Hash`（`event.rs:12`），
测试可直接 `assert_eq!`，无需补派生。

- [ ] **Step 1: 写事务边界与事件的测试**

`crates/continuum-graph/tests/transition_events.rs`

```rust
use continuum_events::EventType;
use continuum_graph::{apply_transition, save_graph, ApplyError, NodeState};
use continuum_persist::{Db, Migration, Value};

#[test]
fn transition_and_its_event_commit_together() {
    let (_d, db, graph_id) = fixture();
    let tx = db.begin().unwrap();
    // 夹具的节点是 PENDING，而迁移表里 PENDING 只有 →READY / →BLOCKED 两条出边，
    // 故进 RUNNING 必须先走 READY → QUEUED
    apply_transition(&tx, &graph_id, "n1", NodeState::Ready, 900, "e0").unwrap();
    apply_transition(&tx, &graph_id, "n1", NodeState::Queued, 950, "e0b").unwrap();
    apply_transition(&tx, &graph_id, "n1", NodeState::Running, 1_000, "e1").unwrap();
    tx.commit().unwrap();

    // 状态确实落库
    assert_eq!(state_of(&db, &graph_id, "n1"), NodeState::Running);
    // 且对应事件同事务写入
    assert_eq!(events_of(&db, &graph_id, "n1"), vec![EventType::NodeStarted]);
}

#[test]
fn illegal_transition_writes_neither_state_nor_event() {
    let (db, graph_id) = fixture();
    // PENDING → COMPLETED 不在迁移表内
    let tx = db.begin().unwrap();
    let err =
        apply_transition(&tx, &graph_id, "n1", NodeState::Completed, 1_000, "e1").unwrap_err();
    assert!(matches!(err, ApplyError::Illegal(_)), "实际 {err:?}");

    // 事务被丢弃后，状态与事件都不得留下痕迹
    drop(tx);
    assert_eq!(state_of(&db, &graph_id, "n1"), NodeState::Pending);
    assert!(events_of(&db, &graph_id, "n1").is_empty());
}

#[test]
fn completed_transition_emits_node_completed() {
    let (db, graph_id) = fixture();
    let tx = db.begin().unwrap();
    // PENDING → READY → QUEUED → RUNNING → VERIFYING → COMPLETED
    apply_transition(&tx, &graph_id, "n1", NodeState::Ready, 1_000, "e1").unwrap();
    apply_transition(&tx, &graph_id, "n1", NodeState::Queued, 1_100, "e2").unwrap();
    apply_transition(&tx, &graph_id, "n1", NodeState::Running, 1_200, "e3").unwrap();
    apply_transition(&tx, &graph_id, "n1", NodeState::Verifying, 1_300, "e4").unwrap();
    apply_transition(&tx, &graph_id, "n1", NodeState::Completed, 1_400, "e5").unwrap();
    tx.commit().unwrap();

    assert_eq!(
        events_of(&db, &graph_id, "n1"),
        vec![EventType::NodeStarted, EventType::NodeCompleted],
        "五个迁移里只有进入 RUNNING 与 COMPLETED 的两个产生事件"
    );
}

#[test]
fn intermediate_states_emit_no_event() {
    let (db, graph_id) = fixture();
    let tx = db.begin().unwrap();
    apply_transition(&tx, &graph_id, "n1", NodeState::Ready, 1_000, "e1").unwrap();
    tx.commit().unwrap();
    assert!(
        events_of(&db, &graph_id, "n1").is_empty(),
        "READY 不是 §16 列举的外发事件"
    );
}

#[test]
fn unknown_node_is_rejected() {
    let (db, graph_id) = fixture();
    let tx = db.begin().unwrap();
    let err = apply_transition(&tx, &graph_id, "nope", NodeState::Ready, 1_000, "e1").unwrap_err();
    assert!(matches!(err, ApplyError::UnknownNode { .. }), "实际 {err:?}");
}
```

夹具 `fixture()` 建库、跑迁移、`save_graph` 一张图，返回
`(TempDir, Db, String /* graph_id */)`。

**图必须有至少两个节点。** 单节点无法连出 Port 对——`connect` 只接受自环 `o1 → i1`，
而 `EdgeKind::Data` 参与环检测（`edge.rs`，仅 Evidence/Effect 豁免），自环会被判为 `Cycle`。
用 `n1`（PENDING）+ `n2` 两节点、一条 `n1.o1(Text) → n2.i2(Text)` 的 Data 边，
`entry_nodes = [n1]`、`terminal_nodes = [n2]`，与
`crates/continuum-graph/tests/persistence.rs` 的既有夹具同构，且能过 `load_graph` 的 `validate()`。

**`TempDir` 必须被持有到用例结束**（故解构为 `let (_d, db, graph_id) = fixture();`）：
WAL 模式下临时目录一旦回收，后续事务建日志文件会失败。

`state_of(db, graph_id, node_id) -> NodeState` 与
`events_of(db, graph_id, node_id) -> Vec<EventType>` 是两个辅助函数，各自开事务用
`Tx::query` 读 `adfir_node.state` 与 `events`；后者按 `node_id` 过滤、按
`occurred_at` 升序，把 `event_type` 文本经 `EventType` 的反序列化还原
（`continuum-events` 的 serde `rename` 就是 `"node.started"` 这类短名）。

注意 `NodeState` 与 `EventType` 都需要 `Debug + PartialEq` 才能被 `assert_eq!`
比较；`NodeState` 已有，`EventType` 若没有则补派生。

- [ ] **Step 2: 运行测试，确认失败**

```bash
cargo test -p continuum-graph --test transition_events
```

预期：编译失败，`apply_transition` / `ApplyError` 不存在。

- [ ] **Step 3: 实现 apply_transition**

`crates/continuum-graph/src/transition_tx.rs`

```rust
//! 状态迁移与对应事件的同事务写入（设计 §16、§17、`§318`）。

use crate::node::NodeState;
use crate::persist::{parse_state, state_str};
use crate::state::{transition, StateError};
use continuum_events::{Event, EventType};
use continuum_persist::{PersistError, Tx, Value};

#[derive(Debug, thiserror::Error)]
pub enum ApplyError {
    #[error("图 {graph_id} 中不存在节点 {node_id}")]
    UnknownNode { graph_id: String, node_id: String },
    #[error("迁移被拒：{0}")]
    Illegal(#[from] StateError),
    #[error("持久化失败：{0}")]
    Persist(#[from] PersistError),
}

/// 在同一个事务内：读当前状态 → 校验迁移 → 写回状态 → 写对应事件。
///
/// 只使用调用方传入的 `Tx`，不自行开启或提交事务——提交由调用方负责，
/// 这正是 `§318` 所要求的原子性：状态与事件要么一起生效，要么一起不生效。
///
/// `event_id` 的唯一性由调用方保证。P1 内由测试提供；P2 的执行器按自己的
/// 序号分配。
pub fn apply_transition(
    tx: &Tx<'_>,
    graph_id: &str,
    node_id: &str,
    to: NodeState,
    occurred_at: i64,
    event_id: &str,
) -> Result<NodeState, ApplyError> {
    let rows = tx.query(
        "SELECT state FROM adfir_node WHERE graph_id = ?1 AND node_id = ?2",
        &[Value::text(graph_id), Value::text(node_id)],
    )?;
    let Some(row) = rows.into_iter().next() else {
        return Err(ApplyError::UnknownNode {
            graph_id: graph_id.to_owned(),
            node_id: node_id.to_owned(),
        });
    };
    let from = parse_state(&text(&row[0])?)?;

    // 先判合法性再写。非法的迁移不产生任何写入。
    let next = transition(from, to)?;

    tx.execute(
        "UPDATE adfir_node SET state = ?3 WHERE graph_id = ?1 AND node_id = ?2",
        &[
            Value::text(graph_id),
            Value::text(node_id),
            Value::text(state_str(next)),
        ],
    )?;

    // 只有 §16 列举的三个到达态产生事件；其余迁移只改状态。
    if let Some(event_type) = event_for(next) {
        let event = Event::new(
            event_type,
            event_id.to_owned(),
            occurred_at,
            serde_json::json!({ "graph_id": graph_id, "node_id": node_id }),
        )
        .with_node(node_id.to_owned());
        tx.append_event(&event)?;
    }

    Ok(next)
}

/// §16 列举的三个外发事件中与节点状态相关的两个。
fn event_for(state: NodeState) -> Option<EventType> {
    match state {
        NodeState::Running => Some(EventType::NodeStarted),
        NodeState::Completed => Some(EventType::NodeCompleted),
        _ => None,
    }
}

fn text(v: &Value) -> Result<String, PersistError> {
    match v {
        Value::Text(s) => Ok(s.clone()),
        other => Err(PersistError::Database(format!("列应为文本，实际 {other:?}"))),
    }
}
```

`crates/continuum-graph/src/persist.rs` 的 `state_str` 与 `parse_state` 改为
`pub(crate)`。`crates/continuum-graph/src/lib.rs` 加 `mod transition_tx;` 与
`pub use transition_tx::{apply_transition, ApplyError};`。

- [ ] **Step 4: 让 Artifact 入库也产生事件**

`crates/continuum-artifact/src/persist.rs` 的 `save_artifact` 增加两个参数，
使「入库」与「写 `ArtifactCreated`」不可分离——不提供只写元数据的旁路：

```rust
pub fn save_artifact(
    tx: &Tx<'_>,
    artifact: &Artifact,
    event_id: &str,
    occurred_at: i64,
) -> Result<(), PersistError> {
    // ... 原有的元数据写入与 artifact_input 写入保持不变 ...

    let event = continuum_events::Event::new(
        continuum_events::EventType::ArtifactCreated,
        event_id.to_owned(),
        occurred_at,
        serde_json::json!({ "artifact_id": artifact.id.as_str() }),
    );
    tx.append_event(&event)?;
    Ok(())
}
```

唯一调用点在 `crates/continuum-graph/tests/persistence.rs:112`，随之补上
`event_id` 与 `occurred_at` 两个实参。`ArtifactStore::persist`（Task 14）传给
`save_artifact` 的事件 id 用 `format!("artifact/{}", artifact.id)`，
`occurred_at` 用一个常量（该桥不感知时钟，设计未要求它记录真实时间）。

- [ ] **Step 5: 运行全部测试并提交**

```bash
cargo test --workspace
git add -A
git commit -m "feat(graph): 状态迁移与事件同事务写入（§16、§17）"
```

**注意：** `save_artifact` 现在必须与 `apply_transition` 一样在调用方的事务内提交，
否则事件与元数据不同步。这是刻意为之：它消除了「Artifact 入库但不发事件」的旁路，
而这正是终审在 I2 里发现的那类缺陷。

---

### Task 16: 把 BlobStore 接到 ArtifactStore 上（收口设计 §15）

**背景：** Task 14 建了 `BlobStore`（内容寻址落盘）与 `ArtifactStore` 的落库桥，但**两者互不引用**：`BlobStore` 除测试外全仓无调用方，`ArtifactStore::commit` 只登记元数据、不碰字节。结果是 Artifact 的 `content_hash` 入了库，其字节可能从未落盘——设计 §15 的「二进制内容按 `content_hash` 寻址落磁盘」在 P1 内仍无执行路径触发。这与本子项目反复出现的「机制建好但没接上」是同一形态，Task 14 只闭合了一半，本 task 闭合另一半。

**依赖：** Task 14。

**Files:**
- Modify: `crates/continuum-artifact/src/store.rs`
- Test: `crates/continuum-artifact/tests/artifact_store.rs`

**Interfaces:**
- Produces: `ArtifactStore::commit_with_content(&mut self, blob, artifact, bytes) -> Result<(), ArtifactError>`
- Produces: `ArtifactError::ContentMismatch { id }`、`ArtifactError::Blob(BlobError)`

- [ ] **Step 1: 写测试**

```rust
#[test]
fn commit_with_content_puts_the_bytes_on_disk() {
    let dir = tempfile::tempdir().unwrap();
    let blob = BlobStore::new(dir.path());
    let mut store = ArtifactStore::new();

    let a = artifact("a1", b"payload", vec![]);
    let hash = a.content_hash.clone();
    store.commit_with_content(&blob, a, b"payload").unwrap();

    assert!(blob.contains(&hash), "字节必须已按哈希落盘");
    assert_eq!(blob.get(&hash).unwrap(), b"payload");
    assert_eq!(store.stored_blob_count(), 1);
}

#[test]
fn commit_with_content_rejects_bytes_that_do_not_match_the_hash() {
    let dir = tempfile::tempdir().unwrap();
    let blob = BlobStore::new(dir.path());
    let mut store = ArtifactStore::new();

    // artifact 声明的哈希由 b"declared" 算出，实际传入 b"other"
    let a = artifact("a1", b"declared", vec![]);
    let hash = a.content_hash.clone();
    let err = store.commit_with_content(&blob, a, b"other").unwrap_err();

    assert!(matches!(err, ArtifactError::ContentMismatch { .. }), "实际 {err:?}");
    assert!(!blob.contains(&hash), "被拒的提交不得留下字节");
    assert!(store.get(&ArtifactId::new("a1")).is_none(), "被拒的提交不得登记元数据");
}

#[test]
fn identical_content_under_two_ids_shares_one_blob_on_disk() {
    let dir = tempfile::tempdir().unwrap();
    let blob = BlobStore::new(dir.path());
    let mut store = ArtifactStore::new();

    store.commit_with_content(&blob, artifact("a1", b"same", vec![]), b"same").unwrap();
    store.commit_with_content(&blob, artifact("a2", b"same", vec![]), b"same").unwrap();

    assert_eq!(store.stored_blob_count(), 1, "相同内容只登记一份");
    // 磁盘上也只应有一份：BlobStore 按哈希寻址，相同内容落在同一路径。
    // 用一个只数文件的辅助函数钉住（blobstore.rs 的测试里已有同形的 walk，
    // 本文件自行加一个即可）。
    assert_eq!(count_files(dir.path()), 1, "相同内容在盘上只占一份");
}

#[test]
fn content_survives_commit_persist_restore_and_read_back() {
    // 设计 §15 的完整回路：字节落盘、元数据落库、从库重建后仍能按哈希取回字节
    let dir = tempfile::tempdir().unwrap();
    let blob = BlobStore::new(dir.path().join("blobs"));
    // 建库与跑迁移按本文件既有写法（见同文件其它用例的 open_with + migrate）
    let db = continuum_persist::Db::open_with(&dir.path().join("t.db"), migrations()).unwrap();
    db.migrate().unwrap();

    let tx = db.begin().unwrap();
    let mut store = ArtifactStore::new();
    store.commit_with_content(&blob, artifact("z1", b"root", vec![]), b"root").unwrap();
    store
        .commit_with_content(&blob, artifact("a1", b"derived", vec![ArtifactId::new("z1")]), b"derived")
        .unwrap();
    store.persist(&tx).unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let restored = ArtifactStore::restore(&tx).unwrap();
    let hash = restored.get(&ArtifactId::new("z1")).unwrap().content_hash.clone();
    assert_eq!(blob.get(&hash).unwrap(), b"root", "重建后仍能按哈希取回原始字节");
    assert_eq!(restored.lineage(&ArtifactId::new("a1")), vec![ArtifactId::new("z1")]);
}
```

- [ ] **Step 2: 实现**

在 `crates/continuum-artifact/src/store.rs`：

```rust
use crate::blobstore::{BlobError, BlobStore};

// ArtifactError 增加两个变体：
    #[error("Artifact {id} 声明的 content_hash 与其字节不符")]
    ContentMismatch { id: ArtifactId },
    #[error("内容落盘失败：{0}")]
    Blob(#[from] BlobError),
```

```rust
impl ArtifactStore {
    /// 提交一个 Artifact，并把它的字节按 `content_hash` 落盘。
    ///
    /// 与 [`ArtifactStore::commit`] 的分工：`commit` 只登记元数据，用于
    /// [`ArtifactStore::restore`] 这类「字节不在手上」的场景；本函数是**产生**
    /// Artifact 的路径，它保证字节在任何元数据写入之前已按哈希落盘，并校验
    /// `artifact.content_hash` 与实际字节相符——设计 §15 的「内容按哈希寻址
    /// 落磁盘、元数据入库」由它闭合。
    ///
    /// 落盘先于登记：若登记阶段失败（如 id 重复），已写入的字节留在盘上。
    /// 内容寻址存储是幂等的，孤儿内容可被其他 Artifact 复用，故不回收。
    pub fn commit_with_content(
        &mut self,
        blob: &BlobStore,
        artifact: Artifact,
        bytes: &[u8],
    ) -> Result<(), ArtifactError> {
        if ContentHash::of(bytes) != artifact.content_hash {
            return Err(ArtifactError::ContentMismatch { id: artifact.id });
        }
        blob.put(bytes)?;
        self.commit(artifact)
    }
}
```

- [ ] **Step 3: 收尾两处 Task 14 报出的未覆盖项**

1. `BlobStore::contains` 此前零覆盖（变异成恒 `false` 仍全绿）。Step 1 的用例现在会真正经过它；请确认「把 `contains` 改成恒 `false`」能让 `commit_with_content_puts_the_bytes_on_disk` 变红，若不能则说明该断言未承重，如实回报。
2. `restore` 里 `store.commit(artifact).map_err(...)` 那条分支在现有用例下**不可达**（其触发条件与循环前的输入检查同条件）。Task 14 的实现方为此把错误信息改成点名 id，但其变异显示：改回原样测试**仍全绿**——即该断言实际命中的是循环末尾那条「输入不存在或依赖成环」的错误。处置：保留该分支作为防御（`commit` 日后可能新增失败模式），但在其上方加注释写明「当前不可达，现有用例覆盖的是循环末尾那条」，并把 `restore_reports_dangling_inputs_instead_of_dropping_them` 的注释改成如实描述它命中的是哪条分支。**不要为了让它可达而削弱循环前的检查。**

- [ ] **Step 4: 运行全部测试并提交**

```bash
cargo test --workspace
git add -A
git commit -m "feat(artifact): commit_with_content 把字节按 content_hash 落盘（收口 §15）"
```

---

## 遗留

```
hex 辅助函数在 continuum-events 与 continuum-artifact 各有一份，未提取到 core。
  P0 的 audit.rs 已有私有实现，P1 未改动 P0 代码。属可接受的重复，交终审分诊。
Checkpointable 在 Task 4 定义，无实现、无调用点、无测试。设计第 11.3 节如此要求。
ExecutionProfile 的表随 Task 11 建立，但六个资源字段在 P3、P7 之前恒为 None，
  其写入路径无 task 覆盖——本子项目不产生 ExecutionProfile 记录。
SchedulerConfig 的 max_parallel 默认 1 是占位取值，P3 接入资源模型后重估。
本计划未安排「一张图完整执行」的端到端 task。设计第 17 节的判据 1 由
  Task 7 至 Task 10 的单元测试分项覆盖，缺一条把它们串起来的集成用例。
save_graph 是裸 INSERT，同一 graph_id 二次保存撞主键。P2 每次状态变化都要落库，
  届时必须补 UPDATE/UPSERT 路径（Task 15 的 apply_transition 只更新状态列，不受影响）。
operator 的 Checkpointable 与 FailureClass::ALL、ArtifactError::NotFound 无生产者，
  均记为 P2 项，见终审分诊。
```

## 完成判据对照

| 设计判据 | 由哪些 task 的测试覆盖 |
|---|---|
| 1 一张图能完整执行，状态按 §237 迁移 | Task 6 `legal_transitions_are_accepted`、Task 9 `select_runnable` 系列；**缺端到端集成用例，见遗留** |
| 2 不兼容 Port 在构造阶段被拒 | Task 5 `incompatible_ports_are_rejected_at_construction`、Task 11 `graph_round_trips_through_the_database` |
| 3 只有下游被 INVALIDATED | Task 7 `upstream_and_unrelated_branches_are_untouched` |
| 4 失败按 §309 分类，非幂等不自动重试 | Task 10 `non_idempotent_side_effect_never_retries`、`permanent_verification_and_unknown_fail_without_retry` |
| 5 NonDeterministic 不产生缓存键 | Task 8 `non_deterministic_operator_produces_no_cache_key` |
| 事务边界：状态迁移与对应事件同事务提交（§318） | Task 15 `transition_and_its_event_commit_together`、`illegal_transition_writes_neither_state_nor_event` |

## 完成判据对照

| 设计判据 | 由哪些 task 的测试覆盖 |
|---|---|
| 1 一张图能完整执行，状态按 §237 迁移 | Task 6 `legal_transitions_are_accepted`、Task 9 `select_runnable` 系列；**缺端到端集成用例，见遗留** |
| 2 不兼容 Port 在构造阶段被拒 | Task 5 `incompatible_ports_are_rejected_at_construction`、Task 11 `graph_round_trips_through_the_database` |
| 3 只有下游被 INVALIDATED | Task 7 `upstream_and_unrelated_branches_are_untouched` |
| 4 失败按 §309 分类，非幂等不自动重试 | Task 10 `non_idempotent_side_effect_never_retries`、`permanent_verification_and_unknown_fail_without_retry` |
| 5 NonDeterministic 不产生缓存键 | Task 8 `non_deterministic_operator_produces_no_cache_key` |

