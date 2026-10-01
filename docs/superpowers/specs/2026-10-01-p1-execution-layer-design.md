# P1 执行层 — 设计

本子项目建立 ADFIR 图的表示、类型系统、调度与失效重算机制，是 P0→P7 建造顺序的第二个子项目。

范围依据《工程文档》第 3 章所列的十二个组件与第 3.4 节的四条完成判据。

---

# 1. 范围

```
ADFIR 图数据结构                      §235
Node 与状态机                         §236 §237
Edge 六类型                           §238
Port 类型系统                         §239
Artifact 数据模型                     §240 §241 §242 §243
Operator 注册表与后端解析接口          §244 §245
ExecutionProfile                      §246
Graph Scheduler                       §303
图失效传播                            §306
增量重算与 Artifact 复用判定           §305
失败分类、重试、检查点                 §307 §308 §309
Plan 到图的接口                        §75 §214
```

# 2. 范围排除

```
模型与工具 Provider 的实现            P3；本子项目只调用其接口
节点放置与隐私裁决                     P3；§243 的 placement 属 Resource Scheduler
Effect Journal 本体                   P2；本子项目只定义 EFFECT 边
Verifier 与 Evidence 判定             P5；本子项目只定义 EVIDENCE 边的结构
Context Compiler                      P6
Plan 对象与审查状态机                  P4；本子项目只做 Plan → 图 的接口
规划算法本身                           P4
领域算子（代码、研究、媒体、图像）      P5
```

`§304` 的 Resource Scheduler 不在本子项目内。`§303` 的 Graph Scheduler 只判定哪些节点 READY 与哪些可并发，不做放置。

# 3. 完成判据

```
1  一张 ADFIR 图能完整执行，节点状态严格按 §237 迁移
2  输入 Port 与输出 Port 不兼容时，连接在构造阶段即被拒绝
3  修改一个节点的输入后，只有其下游被 INVALIDATED，上游与无关分支不动
4  失败按 §309 分类，非幂等 Effect 不进入自动重试（§307）
5  NonDeterministic 的 Operator 不产生缓存键
```

第 1 至 4 条取自 02 §3.4。第 5 条是本子项目对 ENG-002 裁定的可验证形态。

# 4. 前置决策的落地

## 4.1 ENG-003：Artifact 身份分离职责

```
artifact_id      图引用与版本身份。稳定，不随内容变化。
content_hash     内容身份。相同内容产生相同 hash。
```

两条标识分工如下：

```
存储寻址     按 content_hash。同一内容只存一份。
图引用       按 artifact_id。内容变更不改变引用关系。
lineage      记录 (artifact_id, content_hash) 对。
失效判定     以 content_hash 判定内容是否变化。
失效传播     以 artifact_id 确定受影响的引用。
```

`§241` 要求内容不可变。内容变化产生新的 Artifact 版本，新版本有新的 `artifact_id`；若新内容与既有内容相同，则 `content_hash` 相同，存储不新增副本。

## 4.2 ENG-002：按 determinism 分类复用

`§305` 的三个复用前提保持原样，并增加一条前置条件：

```
operator.determinism = Deterministic
∧ input_hash unchanged
∧ operator_version unchanged
∧ contract unaffected
```

`NonDeterministic` 的 Operator 不参与复用，不写缓存键。该 Operator 的输出仍产生 Artifact 并入库，但不记录可供后续复用的输入哈希映射。

`§115` 承认模型输出非确定，只保证 execution reproducibility。本设计不解释该限制。

# 5. crate 布局与依赖方向

```
crates/
  continuum-artifact   ArtifactType 枚举、Artifact 数据模型、内容寻址、
                       lineage、版本保留
  continuum-port       Port 定义与连接兼容性判定
  continuum-operator   Operator 定义（§244）、注册表、后端解析接口（§245）
  continuum-graph      Node、Edge、ADFIR 图、状态机、失效传播、
                       增量重算判定、Graph Scheduler
```

`ArtifactType` 定义在 `continuum-artifact`，不定义在 `continuum-port`。理由：`§240` 的 Artifact 数据模型以该类型为字段，若枚举定义在 Port 侧则两个 crate 互相依赖。`continuum-port` 只提供 Port 定义与兼容性判定。

依赖方向：

```
continuum-port      → continuum-artifact
continuum-operator  → continuum-artifact
continuum-graph     → continuum-port, continuum-operator, continuum-artifact, continuum-persist,
                      continuum-events
continuum-artifact  → continuum-persist, continuum-core, continuum-events
```

该方向与 02 §3.3 的层内依赖序一致，且无环。Graph Scheduler 不单独成 crate：`§303` 的职责是判定 READY 与可并发，与图结构的耦合高于与其他组件的耦合。

`continuum-events` 的两条边由第 16 节要求：`continuum-graph` 与 `continuum-artifact` 都要构造 `Event`，把事件写在调用方的事务里。`continuum-events` 只依赖 `continuum-core`，不构成环。`continuum-persist` 虽也依赖 `continuum-events`，但不转出 `Event` / `EventType`，故不能替代这两条边。

# 6. Artifact 数据模型

字段取自 `§240`，类型与含义在此确定。

```
Artifact {
    id               ArtifactId，本子项目生成
    artifact_type    ArtifactType，见第 7 节
    content_hash     ContentHash，内容寻址键
    size             u64
    producer_node    Option<NodeId>，原始输入为 None
    input_artifacts  Vec<ArtifactId>
    metadata         serde_json::Value
    provenance       serde_json::Value
    privacy_class    PrivacyClass
    version          u32，同一逻辑产物的第几版
}
```

```
PrivacyClass { PUBLIC, PERSONAL, PRIVATE, SECRET, LOCAL_ONLY }   §243
```

约束：

- `id` 在图的整个生命周期内不变，也不因内容变化而变。
- `content_hash` 在内容相同时必须相同；实现方式为对内容字节做密码学哈希（§240 的 `content_hash`）。
- 提交后的 Artifact 不可修改（`§241`）。修改产生新 `version` 与新 `id`。
- `input_artifacts` 构成 lineage（`§242`），可反向追溯至原始输入。

Failure condition：对已提交 Artifact 的修改请求返回错误，不产生写入。

# 7. Port 类型系统

## 7.1 类型枚举

`§239` 要求每个 Node 的输入输出有明确类型，且运行时拒绝不兼容连接。实现方式为封闭的代数数据类型，定义在 `continuum-artifact`（见第 5 节）：

```
enum ArtifactType {
    SourceTree
    Patch
    TestResult
    Text
    Json
    Blob
}
```

`Blob` 为不透明字节，用于尚无专门类型的产物。

规范举例中的 `Image`、`Video`、`Timeline`、`Scene` 属 P5 的领域算子，本子项目不引入。新增类型即新增枚举变体，是编译期可见的破坏性变更，不得以字符串或数字代替。

## 7.2 校验方式

ADFIR 是运行期构造与版本化的动态图，接口两端的类型无法在编译期绑定。因此本设计不提供静态类型的图，只提供两件事：

```
枚举封闭     类型只能是 ArtifactType 的变体，枚举外的类型不可表达；
             不以字符串或数字表示类型
运行期校验   AdfirGraph::connect 在连接构造时比较两端类型，不同即拒绝
             （§239 的 MUST 由这条承担）
```

枚举封闭不替代运行期校验，两者针对不同的错误：前者排除类型集合之外的取值，后者排除类型不匹配的连接。

Failure condition：`connect` 在两端 `artifact_type` 不同时返回 `PortTypeMismatch { from, to }`，不建立边。

# 8. Node、Edge 与图

## 8.1 Node

字段取自 `§236`。

```
Node {
    id
    operator         OperatorId
    inputs           Vec<PortId>
    outputs          Vec<PortId>
    constraints      Vec<ConstraintRef>
    capabilities     Vec<CapabilityRef>
    execution_policy ExecutionPolicy
    verification_policy VerificationPolicy
    state            NodeState
}
```

`execution_policy` 与 `verification_policy` 在本子项目内只作为结构保存，其判定属 P4 与 P5。

## 8.2 Edge 类型

`§238` 列出六类，只给出三类的语义。其余三类在此确定。

```
DATA         携带 Artifact 的端口连接。参与失效传播。
CONTROL      仅执行序关系，不携带数据。不参与失效传播。
DEPENDENCY   结构性依赖，不携带数据但参与失效传播。用于配置与外部状态。
EVIDENCE     验证关系。指向被验证节点。不参与调度与失效传播。
EFFECT       外部副作用的记入关系（§268）。不参与调度与失效传播。
INVALIDATION 纯失效传播边（§306）。不参与调度。
```

参与失效传播的边类型为 DATA、DEPENDENCY、INVALIDATION。`§306` 只提及 DEPENDENCY 与 INVALIDATION 两类，本设计将 DATA 一并纳入，理由为 DATA 是内容依赖的载体，其上游内容变化必然影响下游。

## 8.3 图

字段取自 `§235`。

```
AdfirGraph {
    id
    version
    nodes            Vec<Node>
    edges            Vec<Edge>
    entry_nodes      Vec<NodeId>
    terminal_nodes   Vec<NodeId>
    contract_id      ContractId
}
```

约束：

- `entry_nodes` 的每个节点的所有输入 Port 必须无入边。
- `terminal_nodes` 的每个节点的所有输出 Port 必须无出边。
- 图必须无环。DATA、CONTROL、DEPENDENCY、INVALIDATION 四类边参与环检测；EVIDENCE 与 EFFECT 不参与。

Failure condition：违反任一条的图构造请求返回错误，不产生图。

# 9. 状态机

`§237` 列出十三个状态，未定义迁移关系。迁移在此确定。

```
PENDING ──→ READY
READY ──→ QUEUED
QUEUED ──→ RUNNING
RUNNING ──→ VERIFYING ──→ COMPLETED
RUNNING ──→ WAITING ──→ READY
RUNNING ──→ BLOCKED ──→ READY
PENDING ──→ BLOCKED ──→ READY
READY   ──→ BLOCKED ──→ READY
RUNNING │ WAITING │ BLOCKED ──→ SUSPENDED ──→ READY
RUNNING │ VERIFYING ──→ FAILED
RUNNING │ VERIFYING ──→ LOST
任意状态 ──→ CANCELLED
任意状态 ──→ INVALIDATED
```

终态为 COMPLETED、FAILED、CANCELLED、INVALIDATED、LOST。`INVALIDATED` 可由任意状态到达，包括 COMPLETED：`§306` 要求输入变化时受影响节点被 INVALIDATED，已完成节点同样适用。

Failure condition：不在上表内的迁移返回 `IllegalTransition { from, to }`，节点状态不变。

`§319` 的 `reconcile running nodes` 与 `mark lost executions` 两个阶段，本子项目注册恢复钩子实现：处于 RUNNING 或 VERIFYING 的节点在重启后迁移为 LOST。

# 10. 失效传播与增量重算

## 10.1 传播

输入或 Contract 变化时：

```
受影响节点
  ↓ 沿 DATA、DEPENDENCY、INVALIDATION 边前进
下游全部节点
```

标记为 INVALIDATED。上游与不在此传播范围内的分支不改变状态。

传播是保守标记。是否真正重算由第 10.2 节判定。

## 10.2 复用判定

```
can_reuse(node) =
    operator.determinism = Deterministic
    ∧ input_hash unchanged
    ∧ operator_version unchanged
    ∧ contract unaffected
```

判定为真时，复用已有 Artifact，节点由 INVALIDATED 直接回到 COMPLETED，不重新执行。判定为假时，节点回到 READY 并重新执行。

`§306` 的「不相关节点不得无意义重算」由第 10.1 节的传播范围与本节判定共同承担。

Failure condition：`NonDeterministic` 的 Operator 不写入缓存键，因此其输出永远不满足 `input_hash unchanged`，判定恒为假。

# 11. Operator 与注册表

## 11.1 定义

字段取自 `§244`。

```
Operator {
    id
    version
    input_schema     Vec<ArtifactType>
    output_schema    Vec<ArtifactType>
    determinism      Determinism
    side_effect_class SideEffectClass
    backend_candidates Vec<BackendId>
}

Determinism { Deterministic, NonDeterministic }

SideEffectClass { Pure, Idempotent, NonIdempotent }
```

`backend_candidates` 的具体后端由 Router 决定（`§245`），本子项目只保存候选列表并暴露解析接口。

## 11.2 执行接口

三个类型定义在 `continuum-graph`，不定义在 `continuum-operator`。理由：`NodeContext` 需要携带 `NodeId`，而 `NodeId` 属于 `continuum-graph`；若把执行接口放在 `continuum-operator`，则它必须反向依赖 `continuum-graph`，与设计第 5 节的依赖方向冲突。

```
struct ArtifactRef {
    id            ArtifactId
    content_hash  ContentHash
}

struct NodeContext {
    node          NodeId
    profile       ExecutionProfile
    cancelled     Arc<AtomicBool>
}

trait OperatorImpl {
    fn execute(&self, inputs: &[ArtifactRef], ctx: &NodeContext)
        -> Result<Vec<Artifact>, OperatorError>;
}
```

`NodeContext` 携带取消信号，不携带 Capability 句柄：Capability 的强制点在第 5 层（P2），本子项目只保留 `ExecutionProfile` 中的记录。

`§245` 的后端解析接口只做判定，不做选择：

```
fn is_candidate_backend(operator: &Operator, backend: &BackendId) -> bool
```

具体后端的选择由 Router 决定（P3）。本子项目不实现选择逻辑，也不定义选择失败的错误类型——那应在 P3 有真实选择器时确定。

本子项目不提供领域算子。完成判据的验证由测试内的确定性实现承担。

Failure condition：注册表中不存在 `(operator_id, operator_version)` 时，节点不进入 RUNNING，返回 `OperatorNotFound`。

## 11.3 检查点

`§308` 要求长任务 Operator 支持 checkpoint。本子项目定义接口，不实现：

```
trait Checkpointable {
    fn checkpoint(&self) -> Result<Checkpoint, OperatorError>;
    fn restore(&self, checkpoint: &Checkpoint) -> Result<(), OperatorError>;
}
```

未实现该接口的 Operator 不参与检查点。

# 12. Graph Scheduler

`§303` 的职责有两个：判定哪些节点 READY，哪些可并发。

```
READY 条件   所有输入 Port 已有可用的 DATA 入边来源，
             且所有 CONTROL 与 DEPENDENCY 入边来源处于 COMPLETED
可并发条件   所有 READY 节点之间均可并发
```

可并发条件无需单独判定。「两个可执行节点之间存在 CONTROL 或 DATA 路径」在结构上不可能：候选节点处于 PENDING 或 READY，而排序前驱已 COMPLETED 的节点不在此列，故任两个候选之间不存在排序路径。

参与调度排序的三类边（DATA、CONTROL、DEPENDENCY）的前驱进入 FAILED、CANCELLED、INVALIDATED 或 LOST 时，依赖它的节点迁移为 BLOCKED，不进入 READY。

边集与 READY 判据一致是必需的：DATA 前驱失败时其 Artifact 永不出现，下游同样不可推进；若阻塞只看 CONTROL 与 DEPENDENCY，这类节点既不可 READY 也不 BLOCKED，会永久搁死在 PENDING。

前驱处于 BLOCKED 同样计为不可推进，因此阻塞沿依赖链传递。

BLOCKED 节点在其全部此类前驱重新回到 COMPLETED 后迁移为 READY。

阻塞是传递的：某节点被标记 BLOCKED 后，依赖它的节点同样不可推进，一并标记。标记迭代到不动点。

迁移表中的 `PENDING → BLOCKED` 与 `READY → BLOCKED` 两条为此而设——被阻塞的节点通常尚未运行。已处于 QUEUED 或 RUNNING 的节点不因前驱失败被拽回，只在下一轮调度时因前置条件不满足而不再进入 READY。

`§303` 未要求并发上限。本子项目取一个由配置给出的并行上限，默认值为 1，理由是在 P3 接入模型资源之前，资源约束不可知。

Failure condition：存在 READY 节点但并行上限已满时，节点保持在 READY，不进入 QUEUED。

# 13. 失败分类、重试与检查点

## 13.1 分类

`§309` 的七个类别：

```
TRANSIENT       可重试
PERMANENT       不可重试
CONSTRAINT      不可重试，应升级为决策
AUTHORIZATION   不可重试，应升级为决策
RESOURCE        可重试，退避后仍失败则升级
VERIFICATION    不可重试，进入验证路径
UNKNOWN         不可自动重试
```

上表是该类别在**固有归属**上是否允许重试，不是对某次执行的保证：一次失败是否真的重试，还要经第 13.2 节的白名单与尝试余量两道判定。`RESOURCE` 与前两类的区别在升级时机——`CONSTRAINT` / `AUTHORIZATION` 立即升级为决策，`RESOURCE` 是退避重试耗尽后再升级。

## 13.2 重试

```
RetryPolicy {
    max_attempts
    backoff
    retryable_errors   Vec<FailureClass>
    escalation_policy  EscalationPolicy
}
```

`attempt` 自 1 起计：首次尝试的 `attempt` 为 1。`max_attempts` 是该节点允许的**总尝试次数**，不是重试次数。判据为 `attempt < max_attempts` 时重试（`next_attempt = attempt + 1`），否则升级。故 `max_attempts = 1` 表示只尝试一次、不重试。

`retryable_errors` 是在 `§13.1` 固有归属之上**收窄**的白名单：类别既要在固有归属上可重试，又要在白名单内，才会被重试。空白名单使所有类别都不重试。

`RetryPolicy::default()`（`max_attempts = 1`、空白名单、`EscalationPolicy::None`）表示**不重试**，适用于未声明重试策略的 Operator。它不满足 `§13.1` 对 `RESOURCE` 的「退避后仍失败则升级」：该类别拿到默认策略时会在首次失败即失败，没有任何退避重试。故 `RESOURCE` 的固有策略必须由后端解析方（P3 的 Router）在构造 `ExecutionProfile` 时显式给出 `max_attempts >= 2` 与退避参数，不能依赖默认值。默认值是「未配置」的表示，不是任何类别的固有策略。

`§307` 要求非幂等 Effect 不得直接自动重试。判定依据为 `Operator.side_effect_class`：

```
Pure           可自动重试
Idempotent     可自动重试
NonIdempotent  不自动重试，直接进入升级
```

Failure condition：`NonIdempotent` 的节点在失败后不产生第二次尝试，无论 `retryable_errors` 是否包含该类别。

# 14. ExecutionProfile

字段取自 `§246`。每次 Node 执行产生一条记录，按尝试序号保留历史。

```
ExecutionProfile {
    model            Option<String>
    provider         Option<String>
    tool             Option<String>
    backend          Option<BackendId>
    compute_node     Option<String>
    reasoning_effort Option<String>
    parallelism      Option<u32>
    timeout_ms       Option<u64>
    retry_policy     RetryPolicy
    cost_budget      Option<String>
}
```

`backend` 的类型是 `Option<BackendId>`——`BackendId` 已由 `§244` 的 Operator 定义提供，无需占位。

本子项目内 `model`、`provider`、`tool`、`compute_node` 恒为 None——P3 与 P7 之前无对应资源。

该类型定义在 `continuum-graph`，与第 11.2 节的执行接口同处一 crate。

六处字段在本子项目内以 `String` 承载——`model`、`provider`、`tool`、`compute_node`、`reasoning_effort`、`cost_budget`。前四者对应 `§246` 的资源标识，P3 接入 `ModelProvider` 与 `ToolProvider` 时收紧为强类型 id；后两者对应的类型在本子项目内不存在，先以 `String` 占位。此时收紧不引入迁移成本：本子项目不产生 `ExecutionProfile` 记录（第 15 节的表已建，无写入方）。

`retry_policy` 非可选：`RetryPolicy` 自带默认值，语义为「单次尝试、不重试」。第 15 节的对应列也是 `NOT NULL`，两处一致。

# 15. 持久化

`§317` 要求 ADFIR 持久化。P0 的迁移框架已备好，本子项目注册自己的迁移：

```
adfir_graph          id, version, contract_id, entry_nodes, terminal_nodes
adfir_node           graph_id, node_id, operator_id, operator_version, state,
                     execution_policy, verification_policy
adfir_port           graph_id, node_id, direction, name, artifact_type
adfir_edge           graph_id, from_node, from_port, to_node, to_port, kind
artifact             id, artifact_type, content_hash, size, producer_node,
                     privacy_class, version, metadata, provenance
artifact_input       artifact_id, input_artifact_id
execution_profile    node_id, attempt, backend, timeout, retry_policy, cost_budget
node_attempt         node_id, attempt, state, failure_class
```

Artifact 的二进制内容按 `content_hash` 寻址落磁盘，不存库。元数据入库。

**枚举列的编码。** `adfir_node.state`、`node_attempt.state`、`node_attempt.failure_class`、`adfir_edge.kind`、`adfir_port.direction`、`artifact.artifact_type`、`artifact.privacy_class` 一律用**小写**、多词以 `_` 连接（`source_tree`、`local_only`）。这不等于 Rust 枚举的 serde 表示：`NodeState` 与 `FailureClass` 的 serde 是 `SCREAMING_SNAKE_CASE`，直接反序列化会失败。落库与读回一律经 `continuum-graph::persist` 的显式辅助函数（`state_str` / `parse_state`），不得依赖 serde，也不得在别处硬写字面量。

**`attempt` 的键空间。** `node_attempt` 与 `execution_profile` 共用同一套 `attempt` 编号，键为 `(graph_id, node_id)`。编号自 1 起，同一节点每新增一次尝试取 `MAX(attempt) + 1`。分配方是执行器；恢复钩子把崩溃时正在运行的节点记为一次新尝试，也走同一规则。两处若各起计数器会错位或撞主键。

**表的读写方。** 每张表的读写函数与表定义放在同一 crate。`continuum-runtime` 不直接对这些列写 SQL 字面量——编码分歧正是这样产生的。查询条件里的列取值同样属于该 crate：`WHERE state IN (...)` 的两个取值是编码，不是策略，故也参数化并取自 `state_str`。

这条约束是**构造性质，没有行为守卫**：把 WHERE 的参数换回取值正确的字面量，`cargo test --workspace` 全绿——「用参数」与「用字面量且取值恰好正确」在行为上不可区分。把它变成可执行约束需要源码层检查（仿 `dependency_direction.rs` 的结构性做法），本项目尚未引入这种测试形态。当前由 crate 边界与本节约束维持；若后续有第二个 crate 开始对同一批列写 SQL，再考虑加检查。

`continuum-runtime` 的启动改为 `Db::open_with(builtin_migrations + p1_migrations)`，否则 P0 的 `run_recovery` 内建迁移与 P1 的表不会同时生效。

# 16. 与 P0 的接口

```
Tx                             图与 Artifact 元数据的事务写入
Migration                      本子项目注册第 15 节的表
RecoveryHook                   实现 reconcile running nodes 与 mark lost executions
EventType::NodeStarted         节点进入 RUNNING 时写入
EventType::NodeCompleted       节点进入 COMPLETED 时写入
EventType::ArtifactCreated     Artifact 入库时写入
```

第 9 节的恢复钩子同时闭合 P0 遗留的一项：在此之前 `run_recovery` 的四个阶段无钩子可调用，`§317` 的恢复保证不成立。

**写入函数的事务契约。** 凡「状态变迁 + 事件」这类成对写入，函数只使用调用方传入的 `Tx`，不自行开启或提交事务——提交由调用方负责，这是 `§318` 原子性的前提。由此有两条调用方义务：

1. **函数返回 `Err` 之后，调用方必须回滚该事务，不得提交。** 函数可能在失败前已写入部分行（例如先写状态、再写事件时事件写入失败）。「先收集错误、最后统一提交」的批量写法会写进一个没有对应事件的状态变迁。
2. **事件 id 由调用方分配且必须唯一。** 同一次执行中节点可以多次到达同一状态（`Running → Waiting → Ready → Queued → Running` 是合法路径，会写两条 `NodeStarted`），故不能用可由节点与状态派生的可复用 id。执行器应使用单调序号。

这两条不是实现细节：函数自身提交不了（它拿不到 `Db`），原子性在端到端意义上由调用方兑现。

# 17. 测试策略

```
判据 1  状态机
        合法迁移逐条通过；非法迁移返回 IllegalTransition 且状态不变
判据 2  Port 类型
        类型不同时 connect 被拒；枚举外的类型不可表达
判据 3  失效传播
        修改一个节点的输入，断言只有下游被 INVALIDATED，
        上游与无关分支状态不变
判据 4  失败与重试
        七类失败各自的重试行为；NonIdempotent 在 retryable_errors
        包含该类别时仍不重试
判据 5  复用判定
        Deterministic 且哈希未变时复用；NonDeterministic 不复用；
        NonDeterministic 的输出不产生缓存键
另加    持久化往返
        图与 Artifact 存库再读回，结构与状态一致
另加    崩溃恢复
        图在执行中崩溃，重启后 RUNNING 与 VERIFYING 节点迁移为 LOST
另加    事务边界
        节点状态迁移与对应事件在同一事务内提交（§318）
```

# 18. 遗留风险与未决项

```
§237 的迁移表     规范未定义，本设计定义。若规范后续给出迁移表，以规范为准。
§238 的三类边     规范未描述 DEPENDENCY、EFFECT、INVALIDATION，本设计定义。
ArtifactType 集合 本子项目取六种。P5 引入媒体类型时为编译期可见的破坏性变更。
ExecutionProfile  六个资源字段在 P3、P7 之前恒为 None，其写入路径未被覆盖。
检查点            只定义接口，无实现，无测试。
算子解析的落点    §11.2 的 Failure condition「注册表中不存在 (operator_id, operator_version)
                  时节点不进入 RUNNING，返回 OperatorNotFound」在 P1 无执行点：
                  Node::operator 是 OperatorRef，Node::new 不查注册表，transition 也不收注册表。
                  P2 的 Queued → Running 是它唯一的合法落点，必须在彼处调 OperatorRegistry::resolve
                  并做前置判定，否则未注册的算子也能进入 RUNNING。
RESOURCE 的默认策略 §13.1 给 RESOURCE 的「退避后仍失败则升级」不能由 RetryPolicy::default()
                  满足（默认 max_attempts = 1，首次失败即失败）。P3 的 Router 必须显式给出。
并行上限          默认值 1 是占位取值，P3 接入资源模型后需重估。
```

`OPEN-002` 与 `OPEN-005` 已由第 4 节的两个裁定关闭。关闭结论应写回《总纲》第 10 章的对应条目。
