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
continuum-graph     → continuum-port, continuum-operator, continuum-artifact, continuum-persist
continuum-artifact  → continuum-persist, continuum-core
```

该方向与 02 §3.3 的层内依赖序一致，且无环。Graph Scheduler 不单独成 crate：`§303` 的职责是判定 READY 与可并发，与图结构的耦合高于与其他组件的耦合。

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

```
trait OperatorImpl {
    fn execute(&self, inputs: &[ArtifactRef], ctx: &NodeContext)
        -> Result<Vec<Artifact>, OperatorError>;
}
```

`NodeContext` 携带该节点的 `ExecutionProfile`、`Capability` 句柄与取消信号。

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

Control 与 Dependency 的前驱进入 FAILED、CANCELLED、INVALIDATED 或 LOST 时，依赖它的节点迁移为 BLOCKED，不进入 READY。BLOCKED 节点在其全部此类前驱重新回到 COMPLETED 后迁移为 READY。

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

## 13.2 重试

```
RetryPolicy {
    max_attempts
    backoff
    retryable_errors   Vec<FailureClass>
    escalation_policy  EscalationPolicy
}
```

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
    model            Option<ModelId>
    provider         Option<ProviderId>
    tool             Option<ToolId>
    backend          Option<BackendId>
    compute_node     Option<NodeId>
    reasoning_effort Option<ReasoningEffort>
    parallelism      Option<u32>
    timeout          Option<Duration>
    retry_policy     RetryPolicy
    cost_budget      Option<CostBudget>
}
```

本子项目内 `model`、`provider`、`tool`、`compute_node` 恒为 None——P3 与 P7 之前无对应资源。

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
ExecutionProfile  四个资源字段在 P3、P7 之前恒为 None，其写入路径未被覆盖。
检查点            只定义接口，无实现，无测试。
并行上限          默认值 1 是占位取值，P3 接入资源模型后需重估。
```

`OPEN-002` 与 `OPEN-005` 已由第 4 节的两个裁定关闭。关闭结论应写回《总纲》第 10 章的对应条目。
