# 数据模型

本文档描述界面可能显示的实体、字段、枚举取值和关系。所有内容对应提交 `03e79c7`。

## 术语

- **Intent**：一次工作的目标、状态与关联标识。
- **Artifact**：节点产出的内容对象的元数据；内容本身按 `content_hash` 存放。
- **Evidence**：验证过程产生的证据记录。
- **Node**：图中的执行节点。
- **Edge**：节点端口之间的连接。
- **Effect**：一次对外部世界的副作用记录。
- **Workspace**：某个 Intent 的隔离工作目录及其后端记录。
- **Budget**：预算树节点与只追加的预算账本。

## 1. 读取约定

### 1.1 标识

- 大多数标识是不透明字符串，没有 UUID、长度或字符集约束。
- `Artifact.content_hash` 是唯一有格式约束的标识：64 位小写十六进制（`crates/continuum-artifact/src/content.rs:22`）。
- 标识不是外键。除模型三表外，数据库没有声明外键约束。

### 1.2 时间

- 所有时间字段是 Unix 毫秒，类型为 `i64`，数据库列为 `INTEGER`。
- 时间由调用方提供，不由数据库生成。
- 同一时刻可以写入多条记录；时间不保证严格递增。

### 1.3 枚举

数据库中的枚举列一律使用小写字符串，多词以 `_` 连接。部分 Rust 枚举的 serde 表示是 `SCREAMING_SNAKE_CASE`，与数据库列不同。界面读数据库时以本文档的列值为准。

遇到表外取值时，界面应显示原始字符串并标记为“未知取值”，不要映射为默认枚举值。

### 1.4 JSON 列

标记为 JSON 的列是 `TEXT`，内容为 JSON 文本。读取时需要解析；解析失败时显示原始文本。

### 1.5 可空

`NULL` 表示该字段没有值，不等同于空字符串、`0` 或空数组。界面应分别显示“无”和“空”。

## 2. Artifact

### 2.1 结构

```rust
pub struct Artifact {
    pub id: ArtifactId,
    pub artifact_type: ArtifactType,
    pub content_hash: ContentHash,
    pub size: u64,
    pub producer_node: Option<String>,
    pub input_artifacts: Vec<ArtifactId>,
    pub metadata: serde_json::Value,
    pub provenance: serde_json::Value,
    pub privacy_class: PrivacyClass,
    pub version: u32,
}
```

来源：`crates/continuum-artifact/src/artifact.rs:214`。

| 字段 | 类型 | 数据库列 | 说明 |
|---|---|---|---|
| `id` | string | `artifact.id` | 主键。 |
| `artifact_type` | 枚举 | `artifact.artifact_type` | 17 个取值，见 §2.2。 |
| `content_hash` | string | `artifact.content_hash` | 64 位小写十六进制。 |
| `size` | 整数 | `artifact.size` | 字节数。 |
| `producer_node` | string，可空 | `artifact.producer_node` | 产出该 Artifact 的节点 ID；可为空。 |
| `input_artifacts` | string 数组 | `artifact_input` 表 | 输入 Artifact 的多对多关系。 |
| `metadata` | 任意 JSON | `artifact.metadata` | 内容由生产者决定。 |
| `provenance` | 任意 JSON | `artifact.provenance` | 内容由生产者决定。 |
| `privacy_class` | 枚举 | `artifact.privacy_class` | 5 个取值，见 §2.3。 |
| `version` | 整数 | `artifact.version` | 版本号，`u32`。 |

表定义：`crates/continuum-artifact/src/persist.rs:7`。

### 2.2 Artifact 类型

`ArtifactType` 共 17 个取值（`crates/continuum-artifact/src/artifact.rs:14`）：

| 枚举 | 数据库字符串 |
|---|---|
| `SourceTree` | `source_tree` |
| `Patch` | `patch` |
| `TestResult` | `test_result` |
| `Text` | `text` |
| `Json` | `json` |
| `Blob` | `blob` |
| `Image` | `image` |
| `Audio` | `audio` |
| `Video` | `video` |
| `Timeline` | `timeline` |
| `Mesh` | `mesh` |
| `Scene` | `scene` |
| `Transcript` | `transcript` |
| `Report` | `report` |
| `ShotSet` | `shot_set` |
| `SubtitleTrack` | `subtitle_track` |
| `Render` | `render` |

### 2.3 隐私等级

`PrivacyClass` 共 5 个取值（`crates/continuum-artifact/src/artifact.rs:131`）：

| 枚举 | 数据库字符串 |
|---|---|
| `Public` | `public` |
| `Personal` | `personal` |
| `Private` | `private` |
| `Secret` | `secret` |
| `LocalOnly` | `local_only` |

### 2.4 内容存储

Artifact 的二进制内容不存数据库。`BlobStore` 按内容哈希落盘：

```text
<root>/<哈希前两位>/<完整哈希>
```

来源：`crates/continuum-artifact/src/blobstore.rs:43`。

数据库只存 `content_hash`，不存 blob 根目录。界面要读取内容，必须由部署配置提供 BlobStore 根目录。当前没有 CLI 或表字段提供这个根目录。

`BlobStore::get` 会重新计算哈希；内容与哈希不符时返回 `Corrupt`，文件不存在时返回 `Missing`（`crates/continuum-artifact/src/blobstore.rs:99`）。

### 2.5 写入规则

- `save_artifact` 在同一个事务内写 `artifact`、`artifact_input` 和一条 `artifact.created` 事件（`crates/continuum-artifact/src/persist.rs:45`）。
- 同一个 `id` 第二次写入会被主键拒绝，不覆盖既有记录。
- 没有更新 Artifact 的路径。

## 3. Evidence 与验证

### 3.1 Evidence 结构

`Evidence` 有八个字段，字段全部私有，只能经只读访问器读取（`crates/continuum-verify/src/evidence.rs:222`）：

| 字段 | 类型 | 说明 |
|---|---|---|
| `id` | `EvidenceId` | 证据标识，不透明字符串。 |
| `evidence_type` | `EvidenceType` | 14 个取值，见 §3.2。 |
| `subject` | `EvidenceSubject` | `Requirement(<id>)` 或 `Unattached`。 |
| `claim` | `Claim` | 自由文本，不解析。 |
| `producer` | `EvidenceProducer` | `Node { node, backend }` 或 `Human`。 |
| `artifact_refs` | `Vec<ArtifactId>` | 被读取或验证的 Artifact。 |
| `strength` | `EvidenceStrength` | 任意 JSON，无取值域。 |
| `scope` | `EvidenceScope` | 任意 JSON，无取值域。 |

### 3.2 Evidence 类型

`EvidenceType` 共 14 个取值（`crates/continuum-verify/src/evidence_type.rs:15`）：

| 枚举 | 数据库字符串 |
|---|---|
| `Test` | `test` |
| `Fuzz` | `fuzz` |
| `Property` | `property` |
| `FormalProof` | `formal_proof` |
| `StaticAnalysis` | `static_analysis` |
| `Benchmark` | `benchmark` |
| `VisualCheck` | `visual_check` |
| `MetadataCheck` | `metadata_check` |
| `HumanConfirmation` | `human_confirmation` |
| `Differential` | `differential` |
| `Metamorphic` | `metamorphic` |
| `ModelReview` | `model_review` |
| `ContradictionCheck` | `contradiction_check` |
| `CitationVerification` | `citation_verification` |

其中 `human_confirmation` 不要求 `artifact_refs` 非空；其余 13 类要求 `artifact_refs` 非空（`crates/continuum-verify/src/evidence.rs:335`）。

### 3.3 数据库表

`evidence` 表（`crates/continuum-verify/src/persist.rs:63`）：

| 列 | 类型 | 可空 | 说明 |
|---|---|---|---|
| `evidence_id` | TEXT | 否 | 主键。 |
| `graph_id` | TEXT | 否 | 图 ID。 |
| `verified_node` | TEXT | 否 | 被验证节点。 |
| `producer_node` | TEXT | 是 | 产生证据的节点；`Human` 来源写 `NULL`。 |
| `subject_kind` | TEXT | 否 | `requirement` 或 `unattached`。 |
| `subject_requirement` | TEXT | 是 | `Requirement` 的 ID；`unattached` 写 `NULL`。 |
| `evidence_type` | TEXT | 否 | 14 个类型之一。 |
| `claim` | TEXT | 否 | 自由文本。 |
| `strength` | TEXT | 否 | JSON。 |
| `scope` | TEXT | 否 | JSON。 |
| `occurred_at` | INTEGER | 否 | Unix 毫秒。 |

`evidence_artifact` 表：

| 列 | 说明 |
|---|---|
| `evidence_id` | 证据 ID。 |
| `artifact_id` | Artifact ID。 |

复合主键 `(evidence_id, artifact_id)`。没有外键。

`EvidenceProducer` 的 `backend` 字段不落库；`evidence` 表只有 `producer_node`。`Human` 来源写 `NULL`，`Node` 来源写节点 ID，界面据此区分两种来源。

### 3.4 TestValidity

`TestValidity` 表示测试有效性判定（`crates/continuum-verify/src/test_validity.rs:195`）：

| 字段 | 类型 | 说明 |
|---|---|---|
| `evidence` | `EvidenceId` | 被判定证据。 |
| `checks` | 数组 | 五条检查及其判定，每条为 `(检查项, 判定)`。 |
| `verdict` | `Validity` | 聚合判定。 |

检查项 `TestValidityCheck`（`crates/continuum-verify/src/test_validity.rs:20`）：

| 枚举 | 数据库字符串 |
|---|---|
| `RequirementMapping` | `requirement_mapping` |
| `AssertionStrength` | `assertion_strength` |
| `ErrorImplWouldPass` | `error_impl_would_pass` |
| `MockBypassesRealPath` | `mock_bypasses_real_path` |
| `MissingBoundary` | `missing_boundary` |

逐项判定 `CheckVerdict`：`pass`、`fail`、`unknown`。

聚合判定 `Validity`（`crates/continuum-verify/src/test_validity.rs:138`）：

| 枚举 | 数据库字符串 | 条件 |
|---|---|---|
| `Valid` | `valid` | 五条检查全 `pass`。 |
| `Invalid` | `invalid` | 至少一条 `fail`。 |
| `Unknown` | `unknown` | 无 `fail`，至少一条 `unknown`。 |

表：

- `test_validity(evidence_id PRIMARY KEY, verdict TEXT)`
- `test_validity_check(evidence_id, check_name, verdict, PRIMARY KEY(evidence_id, check_name))`

### 3.5 VerificationRound

`verification_round` 表（`crates/continuum-verify/src/persist.rs:63`）：

| 列 | 类型 | 说明 |
|---|---|---|
| `round_id` | TEXT | 主键。 |
| `graph_id` | TEXT | 图 ID。 |
| `verified_node` | TEXT | 被验证节点。 |
| `selection` | TEXT | JSON，见下。 |
| `aggregate` | TEXT | 五臂聚合判定。 |
| `independence` | TEXT | 独立性判定。 |
| `strategy_verdict` | TEXT | 对抗策略结果。 |
| `occurred_at` | INTEGER | Unix 毫秒。 |

`selection` JSON 的键：

```json
{
  "level": "deterministic_checker",
  "backend": "<backend id>",
  "independence": "independent"
}
```

`level` 取值（`crates/continuum-verify/src/verifier_level.rs:20`）：

| 枚举 | 数据库字符串 |
|---|---|
| `DeterministicChecker` | `deterministic_checker` |
| `FormalStaticChecker` | `formal_static_checker` |
| `IndependentModelVerifier` | `independent_model_verifier` |
| `CrossFamilyVerifier` | `cross_family_verifier` |
| `HumanReview` | `human_review` |

`aggregate` 取值（`crates/continuum-verify/src/verdict.rs:68`）：

| 枚举 | 数据库字符串 | 条件 |
|---|---|---|
| `AdjudicationRequired` | `adjudication_required` | 同时存在 `pass` 与 `fail`。 |
| `Failed` | `failed` | 至少一条 `fail`，且无 `pass`。 |
| `Unknown` | `unknown` | 无 `fail`、无 `pass`，含空集。 |
| `PassedWithUnknown` | `passed_with_unknown` | 有 `pass`、有 `unknown`，无 `fail`。 |
| `Passed` | `passed` | 全部 `pass`。 |

`independence` 取值（`crates/continuum-verify/src/selection.rs:60`）：

| 枚举 | 数据库字符串 |
|---|---|
| `Independent` | `independent` |
| `NotIndependent` | `not_independent` |
| `NotEstablished` | `not_established` |

`strategy_verdict` 取值（`crates/continuum-verify/src/adversary.rs:18`）：

| 枚举 | 数据库字符串 |
|---|---|
| `NoCounterexample` | `no_counterexample` |
| `CounterexampleFound` | `counterexample_found` |

注意：`CounterexampleFound` 的载荷（`passed_verifier`、`counterexample`）不落这一列；列里只有判别串。界面要显示反例内容，需要另找来源，当前数据库没有存储。

### 3.6 写入规则

- `insert_evidence` 只 INSERT，无 UPDATE。二次写同一 `evidence_id` 撞主键失败（`crates/continuum-verify/src/persist.rs:118`）。
- `insert_test_validity` 只 INSERT（`crates/continuum-verify/src/persist.rs:213`）。
- `record_verification_round` 写 `verification_round` 行，并在聚合为 `failed` 或 `adjudication_required` 时同事务写 `verification.failed` 事件（`crates/continuum-verify/src/persist.rs:261`）。
- 没有 `load_evidence`、`load_test_validity` 或 `load_verification_round`。界面只能直接读表。

## 4. 图、节点与边

本节六张表的迁移定义：`crates/continuum-graph/src/persist.rs:13`。

### 4.1 图

`AdfirGraph` 的公开数据（`crates/continuum-graph/src/graph.rs:41`）：

| 数据 | 类型 | 说明 |
|---|---|---|
| `id` | `GraphId` | 图 ID，公开字段。 |
| `version` | `u32` | 图版本，公开字段。 |
| `contract_id` | `ContractIdRef` | 所属 Contract，公开字段。 |
| `entry_nodes` | `Vec<NodeId>` | 入口节点列表，经 `entry_nodes()` 读取。 |
| `terminal_nodes` | `Vec<NodeId>` | 终端节点列表，经 `terminal_nodes()` 读取。 |

节点、端口、边通过方法访问，不直接作为公开字段。`entry_nodes` 与 `terminal_nodes` 的字段本身是私有的，只有读取方法公开。

数据库 `adfir_graph`：

| 列 | 类型 | 说明 |
|---|---|---|
| `id` | TEXT | 主键。 |
| `version` | INTEGER | 图版本。 |
| `contract_id` | TEXT | Contract ID。 |
| `entry_nodes` | TEXT | JSON 字符串数组。 |
| `terminal_nodes` | TEXT | JSON 字符串数组。 |

### 4.2 节点

`Node` 结构（`crates/continuum-graph/src/node.rs:10`）：

| 字段 | 类型 | 说明 |
|---|---|---|
| `id` | `NodeId` | 节点 ID。 |
| `operator` | `OperatorRef` | `{ id, version }`。 |
| `inputs` | `Vec<PortId>` | 输入端口。 |
| `outputs` | `Vec<PortId>` | 输出端口。 |
| `constraints` | `Vec<String>` | 约束列表。 |
| `capabilities` | `Vec<String>` | 能力列表。 |
| `execution_policy` | 任意 JSON | 结构保存，判定属 P4。 |
| `verification_policy` | 任意 JSON | 结构保存，判定属 P5。 |
| `state` | `NodeState` | 13 个状态之一。 |

数据库 `adfir_node`：

| 列 | 类型 | 说明 |
|---|---|---|
| `graph_id` | TEXT | 图 ID。 |
| `node_id` | TEXT | 节点 ID。 |
| `operator_id` | TEXT | 算子 ID。 |
| `operator_version` | INTEGER | 算子版本。 |
| `state` | TEXT | 小写状态字符串。 |
| `execution_policy` | TEXT | JSON。 |
| `verification_policy` | TEXT | JSON。 |
| `constraints` | TEXT | JSON 字符串数组。 |
| `capabilities` | TEXT | JSON 字符串数组。 |

主键 `(graph_id, node_id)`。节点本身不存 `inputs` / `outputs` 列；端口在 `adfir_port` 中按 `graph_id`、`node_id`、`direction` 查询。

### 4.3 端口

数据库 `adfir_port`：

| 列 | 类型 | 说明 |
|---|---|---|
| `graph_id` | TEXT | 图 ID。 |
| `node_id` | TEXT | 节点 ID。 |
| `port_id` | TEXT | 端口 ID。 |
| `direction` | TEXT | `input` 或 `output`。 |
| `name` | TEXT | 端口名。 |
| `artifact_type` | TEXT | 17 个 Artifact 类型之一。 |

主键 `(graph_id, port_id)`。

### 4.4 边

`Edge` 结构（`crates/continuum-graph/src/edge.rs:48`）：

| 字段 | 类型 |
|---|---|
| `from_node` | `NodeId` |
| `from_port` | `PortId` |
| `to_node` | `NodeId` |
| `to_port` | `PortId` |
| `kind` | `EdgeKind` |

`EdgeKind` 共 6 个取值（`crates/continuum-graph/src/edge.rs:9`）：

| 枚举 | 数据库字符串 | 语义 |
|---|---|---|
| `Data` | `data` | 携带 Artifact 的端口连接。参与失效传播。 |
| `Control` | `control` | 仅执行序关系。不参与失效传播。 |
| `Dependency` | `dependency` | 结构性依赖。参与失效传播。 |
| `Evidence` | `evidence` | 验证关系。不参与调度与失效传播。 |
| `Effect` | `effect` | 外部副作用的记入关系。不参与调度与失效传播。 |
| `Invalidation` | `invalidation` | 纯失效传播边。不参与调度。 |

数据库 `adfir_edge`：

| 列 | 类型 |
|---|---|
| `graph_id` | TEXT |
| `from_node` | TEXT |
| `from_port` | TEXT |
| `to_node` | TEXT |
| `to_port` | TEXT |
| `kind` | TEXT |

`adfir_edge` 没有主键。重复边的拦截在图层完成（`crates/continuum-graph/src/graph.rs:206`）。

### 4.5 执行档案与尝试

`execution_profile` 表：

| 列 | 类型 | 可空 |
|---|---|---|
| `graph_id` | TEXT | 否 |
| `node_id` | TEXT | 否 |
| `attempt` | INTEGER | 否 |
| `backend` | TEXT | 是 |
| `timeout_ms` | INTEGER | 是 |
| `retry_policy` | TEXT | 否 |
| `cost_budget` | TEXT | 是 |

主键 `(graph_id, node_id, attempt)`。

`node_attempt` 表：

| 列 | 类型 | 可空 |
|---|---|---|
| `graph_id` | TEXT | 否 |
| `node_id` | TEXT | 否 |
| `attempt` | INTEGER | 否 |
| `state` | TEXT | 否 |
| `failure_class` | TEXT | 是 |

主键 `(graph_id, node_id, attempt)`。

`execution_profile` 当前没有生产读写路径。`node_attempt` 由恢复钩子写入：每个被标记为 `LOST` 的节点追加一行，`failure_class = unknown`（`crates/continuum-graph/src/persist.rs:156`）。

## 5. Workspace

迁移定义：`crates/continuum-workspace/src/persist.rs:20`。

### 5.1 结构

```rust
pub struct WorkspaceRecord {
    pub intent_id: IntentId,
    pub backend: WorkspaceBackend,
    pub path: PathBuf,
    pub base_path: PathBuf,
    pub created_at: i64,
}
```

来源：`crates/continuum-workspace/src/persist.rs:40`。

| 字段 | 数据库列 | 说明 |
|---|---|---|
| `intent_id` | `workspace.intent_id` | 主键。 |
| `backend` | `workspace.backend` | `worktree` 或 `overlay`。 |
| `path` | `workspace.path` | Task 工作区根目录，绝对路径。 |
| `base_path` | `workspace.base_path` | 创建时的 Base 根目录，绝对路径。 |
| `created_at` | `workspace.created_at` | Unix 毫秒。 |

### 5.2 后端

`WorkspaceBackend` 两个取值（`crates/continuum-workspace/src/backend.rs:22`）：

| 枚举 | 数据库字符串 | 判定 |
|---|---|---|
| `Worktree` | `worktree` | Base 中存在 `.git`。 |
| `Overlay` | `overlay` | Base 中不存在 `.git`。 |

后端在创建时判定并写入记录。后续操作从记录读取后端，不重新判定（`crates/continuum-workspace/src/persist.rs:135`）。

### 5.3 目录

| 后端 | Task 根目录 |
|---|---|
| worktree | `<base>/.ai/worktrees/<intent>`（`crates/continuum-workspace/src/worktree.rs:57`） |
| overlay | `<overlay 根>/<Base 标识>/<intent>`（`crates/continuum-workspace/src/overlay.rs:9`） |

overlay 根默认是 `$HOME/.local/share/continuum/overlays`，可由 `CONTINUUM_OVERLAY_ROOT` 覆盖。

worktree 后端的分支名是 `ai/<intent>`（`crates/continuum-workspace/src/worktree.rs:63`）。分支名不落 `workspace` 表；界面只能按同一规则从 `intent_id` 推导。

### 5.4 生命周期

| 操作 | `workspace` 行 |
|---|---|
| 创建 Task 工作区 | 插入。 |
| 未给 `--apply` 且命令结束 | 放弃工作区后删除。 |
| 命令失败 | 放弃工作区后删除。 |
| 效果登记失败或被拒 | 放弃工作区后删除。 |
| 给了 `--apply`，集成成功 | 保留。 |
| 给了 `--apply`，集成被拒 | 保留。 |

来源：`crates/continuum-runtime/src/task_cmd.rs:180`、`:550`。

`workspace` 表没有事件。放弃与集成操作写 `audit_log`。

## 6. Effect

迁移定义：`crates/continuum-effect/src/persist.rs:26`。

### 6.1 结构

```rust
pub struct Effect {
    pub id: EffectId,
    pub effect_type: EffectType,
    pub target: String,
    pub parameters: serde_json::Value,
    pub authorization: String,
    pub idempotency_key: String,
    pub state: EffectState,
    pub planned_at: i64,
    pub updated_at: i64,
}
```

来源：`crates/continuum-effect/src/effect.rs:190`。

| 字段 | 数据库列 | 说明 |
|---|---|---|
| `id` | `effect.id` | 主键。 |
| `effect_type` | `effect.effect_type` | 6 个类型之一。 |
| `target` | `effect.target` | 作用对象，自由文本。 |
| `parameters` | `effect.parameters` | JSON；当前生产路径固定写 `{}`。 |
| `authorization` | `effect.authorization` | 不透明字符串，只记录，不校验。 |
| `idempotency_key` | `effect.idempotency_key` | 唯一索引。 |
| `state` | `effect.state` | 7 个状态之一。 |
| `planned_at` | `effect.planned_at` | Unix 毫秒。 |
| `updated_at` | `effect.updated_at` | Unix 毫秒。 |

### 6.2 类型

`EffectType` 六个取值（`crates/continuum-effect/src/effect.rs:16`）：

`send_email`、`push_branch`、`publish`、`delete_remote`、`charge`、`deploy`。

### 6.3 状态

`EffectState` 七个取值（`crates/continuum-effect/src/effect.rs:85`）：

`planned`、`authorized`、`executing`、`committed`、`failed`、`rolled_back`、`unknown`。

状态迁移规则见 `02-events-and-state.md` §6。

### 6.4 授权字段

`effect.authorization` 的当前格式：

```text
approve=<true|false>;policy=<allow|deny|require_approval>
```

来源：`crates/continuum-runtime/src/tool_call.rs:613`。

该字段只记录，不参与任何校验。界面可以显示它，但不应把它当作授权证明。

### 6.5 不含 Intent 列

`effect` 表没有 `intent_id` 列。Intent 信息在 `id` 与 `idempotency_key` 的字符串前缀中，格式见 `02-events-and-state.md` §6.3。

## 7. 语义层实体

本节七张表的迁移定义：`crates/continuum-semantics/src/persist.rs:206`。

### 7.1 Intent

`Intent` 结构（`crates/continuum-semantics/src/intent.rs:434`）：

| 字段 | 类型 | 数据库列 | 说明 |
|---|---|---|---|
| `id` | `IntentId` | `intent.id` | 主键。 |
| `version` | `u32` | `intent.version` | 版本号。 |
| `kind` | `IntentKind` | `intent.kind` | 6 个取值。 |
| `goal` | `EntityId` | `intent.goal` | 规范化后的目标表示。 |
| `status` | `IntentStatus` | `intent.status` | 13 个状态。 |
| `priority` | `Priority` | `intent.priority` | 不透明字符串，不排序。 |
| `created_at` | `i64` | `intent.created_at` | Unix 毫秒。 |
| `updated_at` | `i64` | `intent.updated_at` | Unix 毫秒。 |
| `parent_intent` | `Option<IntentId>` | `intent.parent_intent` | 可空。 |
| `contract_id` | `ContractRef` | `contract_id` + `contract_version` | 两列共同表示 ID 与版本。 |
| `active_plan_id` | `Option<PlanId>` | `intent.active_plan_id` | 可空。 |
| `graph_id` | `Option<GraphIdRef>` | `intent.graph_id` | 不透明字符串，可空。 |
| `product_id` | `Option<ProductIdRef>` | `intent.product_id` | 不透明字符串，可空。 |
| `completion_predicate` | `Opaque` | `intent.completion_predicate` | 只存不判。 |

`intent.kind` 取值：`task`、`product`、`maintenance`、`research`、`background`、`runtime_improvement`。

`intent.status` 取值：`proposed`、`specifying`、`planning`、`review`、`ready`、`running`、`waiting`、`suspended`、`verifying`、`completed`、`failed`、`cancelled`、`evolving`。

注意：`task` 子命令不会创建 `intent` 行。界面不能假设每个 `workspace.intent_id` 都能在 `intent` 表中找到对应记录。

### 7.2 Specification

`specification` 表：

| 列 | 类型 | 说明 |
|---|---|---|
| `intent_id` | TEXT | 复合主键第一列。 |
| `version` | INTEGER | 复合主键第二列，与 Intent 版本同号。 |
| `goals` | TEXT | JSON 字符串数组。 |
| `invariants` | TEXT | JSON 字符串数组。 |
| `acceptance_predicates` | TEXT | JSON 字符串数组。 |
| `forbidden_states` | TEXT | JSON 字符串数组。 |
| `preferences` | TEXT | JSON 字符串数组。 |
| `unknowns_kind` | TEXT | `none_found` 或 `enumerated`。 |
| `unknowns` | TEXT | JSON 数组；`none_found` 时为空数组。 |

主键 `(intent_id, version)`。

### 7.3 Contract 与 Requirement

`contract` 表：

| 列 | 类型 | 说明 |
|---|---|---|
| `id` | TEXT | 复合主键第一列。 |
| `version` | INTEGER | 复合主键第二列。 |
| `intent_id` | TEXT | 所属 Intent。 |
| `budget` | TEXT | 不透明字符串。 |
| `authority_limit` | TEXT | 不透明字符串。 |
| `data_policy` | TEXT | 不透明字符串。 |
| `model_policy` | TEXT | 不透明字符串。 |
| `node_policy` | TEXT | 不透明字符串。 |
| `acceptance_predicate` | TEXT | 不透明字符串。 |

主键 `(id, version)`。

`contract_requirement` 表：

| 列 | 类型 | 说明 |
|---|---|---|
| `contract_id` | TEXT | 复合主键第一列。 |
| `contract_version` | INTEGER | 复合主键第二列。 |
| `id` | TEXT | 复合主键第三列。 |
| `class` | TEXT | `required`、`preferred`、`flexible`、`unspecified`。 |
| `expression` | TEXT | 自由文本。 |
| `source` | TEXT | 自由文本。 |
| `verification_requirement` | TEXT | 自由文本，只存不判。 |

主键 `(contract_id, contract_version, id)`。

### 7.4 Plan

`plan` 表：

| 列 | 类型 | 说明 |
|---|---|---|
| `id` | TEXT | 复合主键第一列。 |
| `major` | INTEGER | 复合主键第二列。 |
| `minor` | INTEGER | 复合主键第三列。 |
| `intent_id` | TEXT | 所属 Intent。 |
| `type` | TEXT | `strategic` 或 `execution`。 |
| `objectives` | TEXT | JSON 字符串数组。 |
| `milestones` | TEXT | JSON 字符串数组。 |
| `assumptions` | TEXT | JSON 字符串数组。 |
| `risks` | TEXT | JSON 字符串数组。 |
| `acceptance_conditions` | TEXT | JSON 字符串数组。 |
| `review_state` | TEXT | 6 个状态之一。 |

主键 `(id, major, minor)`。

`review_state` 取值（`crates/continuum-semantics/src/plan.rs:204`）：

`draft`、`self_reviewed`、`ai_reviewed`、`user_approved`、`rejected`、`superseded`。

### 7.5 Decision 与 DecisionResult

`decision` 表：

| 列 | 类型 | 说明 |
|---|---|---|
| `id` | TEXT | 主键。 |
| `intent_id` | TEXT | 所属 Intent。 |
| `issue` | TEXT | 不透明字符串。 |
| `options` | TEXT | JSON 数组，元素形如 `{"option": "...", "consequence": "..."}`。 |
| `blocking` | INTEGER | `0` 或 `1`。 |

`decision_result` 表：

| 列 | 类型 | 可空 | 说明 |
|---|---|---|---|
| `decision_id` | TEXT | 否 | 主键，指向 `decision.id`。 |
| `option_index` | INTEGER | 否 | 选中项在 `options` 数组中的下标。 |
| `decided_at` | INTEGER | 否 | Unix 毫秒。 |
| `contract_update` | TEXT | 是 | JSON 或 `NULL`。 |
| `plan_update` | TEXT | 是 | JSON 或 `NULL`。 |

`decision_result` 当前只写不读，没有 `load_decision_result`。

## 8. 预算

两张表的迁移定义：`crates/continuum-budget/src/persist.rs:59`。

### 8.1 预算所有者

`BudgetOwner` 有四个变体（`crates/continuum-budget/src/tree.rs:40`）。数据库 `budget_node.owner` 与 `budget_ledger.owner` 存 JSON 字符串：

| 变体 | 数据库键 |
|---|---|
| `SystemMaintenance` | `{"kind":"system_maintenance"}` |
| `Intent(id)` | `{"id":"<id>","kind":"intent"}` |
| `NodeRun { intent, node }` | `{"intent":"<id>","kind":"node_run","node":"<id>"}` |
| `Exploration { intent, of_node }` | `{"intent":"<id>","kind":"exploration","of_node":"<id>"}` |

来源：`crates/continuum-budget/src/tree.rs:71`。

键是 JSON 对象的紧凑序列化。`serde_json` 在当前依赖配置下按对象键名排序，因此上表的键序是实际写入顺序。界面若要用 `WHERE owner = ?` 做等值查询，必须产生同样的键序；更稳妥的做法是读取 `owner` 列后在应用层解析 JSON，再按字段比较。

### 8.2 量纲

`Dimensions` 有五个可空字段（`crates/continuum-budget/src/dimensions.rs:91`）：

| 字段 | 数据库列 | 说明 |
|---|---|---|
| `money` | `budget_node.money` | 金额。 |
| `wall_time` | `budget_node.wall_time` | 墙钟时间。 |
| `token` | `budget_node.token` | Token 数。 |
| `gpu_time` | `budget_node.gpu_time` | GPU 时间。 |
| `network_transfer` | `budget_node.network_transfer` | 网络传输量。 |

`NULL` 表示该量纲不构成约束；`0` 表示额度为零。两者不同。

`budget_ledger.dimension` 的取值：`money`、`wall_time`、`token`、`gpu_time`、`network_transfer`。

### 8.3 账本

`budget_ledger` 表：

| 列 | 类型 | 说明 |
|---|---|---|
| `owner` | TEXT | 预算所有者键。 |
| `dimension` | TEXT | 五维之一。 |
| `kind` | TEXT | `allocate`、`reserve`、`release`、`settle`。 |
| `amount` | INTEGER | 名义量。 |
| `at` | INTEGER | Unix 毫秒。 |

表没有主键。账本只追加，不 UPDATE，不 DELETE。余额是派生值，不存列。

`LedgerKind` 四个取值（`crates/continuum-budget/src/ledger.rs:43`）：

| 枚举 | 数据库字符串 |
|---|---|
| `Allocate` | `allocate` |
| `Reserve` | `reserve` |
| `Release` | `release` |
| `Settle` | `settle` |

### 8.4 三个角色

| 类型 | 含义 | 是否落库 |
|---|---|---|
| `Allocation` | 预算上限。 | 是，`budget_node` 的五维列。 |
| `Reservation` | 一次在飞的预扣。 | 是，`budget_ledger` 的 `reserve` 行。 |
| `Remaining` | `allocation - settled - reserved`，逐维现算。 | 否。 |

来源：`crates/continuum-budget/src/account.rs:32`、`:54`、`:70`。

当前没有 CLI 命令写预算表。界面可以读表，但不能通过 CLI 创建预算。

## 9. 其它可显示实体

### 9.1 Policy

迁移定义：`crates/continuum-policy/src/persist.rs:30`。

`policy` 表：

| 列 | 类型 | 说明 |
|---|---|---|
| `id` | TEXT | 主键。 |
| `level` | TEXT | 6 个层级之一。 |
| `scope` | TEXT | `user` 或 `project`。 |
| `condition` | TEXT | JSON 谓词。 |
| `decision` | TEXT | `allow`、`deny`、`require_approval`。 |

`level` 取值（`crates/continuum-policy/src/rule.rs:18`）：

`system_safety`、`explicit_current`、`user_persistent`、`project`、`runtime_default`、`model_suggestion`。

### 9.2 Tool 登记

迁移定义：`crates/continuum-capability/src/persist.rs:52`。

`tool` 表：

| 列 | 类型 | 可空 | 说明 |
|---|---|---|---|
| `id` | TEXT | 否 | 主键。 |
| `version` | TEXT | 否 | 版本字符串。 |
| `input_schema` | TEXT | 否 | JSON。 |
| `output_schema` | TEXT | 否 | JSON。 |
| `required_capabilities` | TEXT | 否 | JSON 字符串数组。 |
| `effect_class` | TEXT | 是 | 6 个 EffectType 之一或 `NULL`。 |
| `deterministic` | INTEGER | 否 | `0` 或 `1`。 |
| `cost` | TEXT | 是 | 文本。 |
| `latency` | TEXT | 是 | 文本。 |
| `trust` | TEXT | 否 | 文本。 |

`required_capabilities` 的元素取值（`crates/continuum-capability/src/capability.rs:236`）：

`filesystem_read`、`filesystem_write`、`git_read`、`git_worktree_write`、`git_commit_local`、`git_push`、`git_delete_remote`、`github_create_pr`、`email_send`、`registry_publish`、`payment_charge`、`environment_deploy`。

### 9.3 模型登记

迁移定义：`crates/continuum-model-registry/src/persist.rs:106`。

三张表：

- `model_registry(id TEXT PRIMARY KEY, lifecycle_state TEXT NOT NULL)`
- `model_profile(id TEXT PRIMARY KEY, version TEXT, provider TEXT, model_revision TEXT, modalities TEXT, tools TEXT, failure_modes TEXT, cost_profile TEXT NULL, latency_profile TEXT NULL, evidence_count INTEGER, confidence TEXT)`
- `model_skill_score(model_id TEXT, dimension TEXT, score_version INTEGER, score TEXT, confidence TEXT, sample_count INTEGER, time_range_start INTEGER, time_range_end INTEGER, PRIMARY KEY(model_id, dimension, score_version))`

`model_registry.lifecycle_state` 取值（`crates/continuum-model-registry/src/lifecycle.rs:36`）：

`discovered`、`unprofiled`、`researched`、`probed`、`verified`、`active`、`stale`、`degraded`、`quarantined`、`disabled`。

`model_profile` 与 `model_skill_score` 声明了外键；其余业务表没有外键。

### 9.4 Alias

`alias` 表：

| 列 | 类型 | 说明 |
|---|---|---|
| `alias` | TEXT | 主键。 |
| `target` | TEXT | 目标。 |
| `scope` | TEXT | `turn`、`session`、`project`、`user_persistent`。 |

来源：`crates/continuum-canonical/src/persist.rs:83`。

## 10. 关系

| 关系 | 关联字段 | 数据库约束 |
|---|---|---|
| Intent → Contract | `intent.contract_id` + `intent.contract_version` | 无外键。 |
| Intent → Plan | `intent.active_plan_id` | 无外键。 |
| Intent → Graph | `intent.graph_id` | 无外键。 |
| Intent → Workspace | `workspace.intent_id` | 无外键；`intent` 行可能不存在。 |
| Graph → Node | `adfir_node.graph_id` | 无外键。 |
| Graph → Edge | `adfir_edge.graph_id` | 无外键。 |
| Node → Port | `adfir_port.graph_id` + `adfir_port.node_id` | 无外键。 |
| Node → Attempt | `node_attempt.graph_id` + `node_attempt.node_id` | 无外键。 |
| Artifact → Input Artifact | `artifact_input.artifact_id` + `input_artifact_id` | 无外键。 |
| Evidence → Artifact | `evidence_artifact.evidence_id` + `artifact_id` | 无外键。 |
| Evidence → Graph/Node | `evidence.graph_id`、`verified_node`、`producer_node` | 无外键。 |
| VerificationRound → Graph/Node | `verification_round.graph_id`、`verified_node` | 无外键。 |
| Contract → Requirement | `contract_requirement.contract_id` + `contract_version` | 无外键。 |
| Decision → Result | `decision_result.decision_id` | 无外键。 |
| ModelProfile → ModelRegistry | `model_profile.id` | 有外键。 |
| ModelSkillScore → ModelProfile | `model_skill_score.model_id` | 有外键。 |

界面应把这些关系当作“按标识匹配”，不要当作数据库保证的引用完整性。

## 11. 只读范围

界面对数据库的所有表都应按只读处理。当前没有为界面提供的写接口。

| 表 | 写入方 | 界面操作 |
|---|---|---|
| `events` | 库层事件写入函数 | 只读。 |
| `audit_log` | `Tx::append_audit` | 只读。 |
| `artifact`、`artifact_input` | `save_artifact` | 只读。 |
| `adfir_graph`、`adfir_node`、`adfir_port`、`adfir_edge` | `save_graph`、`apply_transition`、恢复钩子 | 只读。 |
| `execution_profile` | 当前无 | 只读。 |
| `node_attempt` | 恢复钩子 | 只读。 |
| `workspace` | `save_workspace`、`remove_workspace` | 只读。 |
| `effect` | `record_planned`、`advance` | 只读。 |
| `policy` | `save_policy` | 只读。 |
| `tool` | `save_tool` | 只读。 |
| `model_registry`、`model_profile`、`model_skill_score` | 模型登记库函数 | 只读。 |
| `alias` | `save_alias` | 只读。 |
| `intent`、`specification`、`contract`、`contract_requirement`、`plan`、`decision`、`decision_result` | 语义层库函数 | 只读。 |
| `budget_node`、`budget_ledger` | 预算库函数 | 只读。 |
| `evidence`、`evidence_artifact`、`test_validity`、`test_validity_check`、`verification_round` | 验证库函数 | 只读。 |

如果界面需要修改数据，当前 CLI 只有 `task`、`tool`、`recover` 三个入口；多数实体没有对应的 CLI 写入路径，也没有通用的写接口。
