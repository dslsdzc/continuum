# PIER / ADFIR v0.1 技术规范

版本：0.1-draft
状态：Draft Specification
适用范围：PIER Runtime、ADFIR、Planner、Model Router、Tool Runtime、Verifier、Compute Fabric、Memory、Authority

---

## 219. 规范术语

本文采用以下规范词：

**MUST**
必须实现，不满足即不符合规范。

**MUST NOT**
绝对禁止。

**SHOULD**
正常实现必须遵循，只有明确理由才能偏离。

**SHOULD NOT**
正常情况下禁止，只有明确理由才能偏离。

**MAY**
可选能力。

本文中的：

```
Intent
Contract
Plan
ADFIR
Artifact
Evidence
Effect
Capability
Node
Verifier
Authority
```

均为 Runtime 一级语义对象。

---

## 220. PIER Runtime 总体接口

PIER 的最高层抽象：

```
execute(
    intent,
    context,
    authority,
    policy
) -> ExecutionHandle
```

ExecutionHandle MUST 支持：

```
status()
stream_events()
suspend()
resume()
cancel()
inspect()
```

执行结果不直接定义为：

```
String
```

而必须返回：

```
ExecutionResult {
    intent_id
    status

    primary_artifacts[]
    evidence[]
    decisions[]
    effects[]

    verification
    final_summary
}
```

---

## 221. 全局对象标识

所有持久对象 MUST 使用稳定 ID。

推荐：

```
intent://...
contract://...
plan://...
graph://...
node://...
artifact://...
evidence://...
effect://...
model://...
tool://...
device://...
memory://...
```

ID MUST：

```
全局唯一
不可随重启变化
与展示名称解耦
```

用户重命名对象 MUST NOT 改变其 ID。

---

## 222. Intent 数据模型

```
Intent {
    id
    version

    kind
    goal

    status
    priority

    created_at
    updated_at

    parent_intent?
    child_intents[]

    contract_id
    active_plan_id?
    graph_id?

    product_id?

    completion_predicate
}
```

"kind" 至少支持：

```
TASK
PRODUCT
MAINTENANCE
RESEARCH
BACKGROUND
RUNTIME_IMPROVEMENT
```

---

## 223. Intent 状态机

```
PROPOSED
   ↓
SPECIFYING
   ↓
PLANNING
   ↓
REVIEW
   ↓
READY
   ↓
RUNNING
```

运行后可进入：

```
WAITING
SUSPENDED
VERIFYING
COMPLETED
FAILED
CANCELLED
```

Product Intent MAY 在达到首次 Shippable 后进入：

```
EVOLVING
```

而不是直接结束。

---

## 224. Task Contract 数据模型

```
TaskContract {
    id
    version
    intent_id

    requirements[]
    preferences[]
    prohibitions[]

    budget
    authority_limit

    data_policy
    model_policy
    node_policy

    acceptance_predicate
}
```

Requirement：

```
Requirement {
    id
    class

    expression
    source

    verification_requirement
}
```

"class"：

```
REQUIRED
PREFERRED
FLEXIBLE
UNSPECIFIED
```

---

## 225. Contract 不可越权原则

执行系统 MUST 满足：

```
RuntimeFreedom ⊆ ContractAllowedSpace
```

Runtime MAY：

```
换模型
换工具
换节点
换算法
重新规划
改变并行度
改变内部执行顺序
```

Runtime MUST NOT：

```
修改 REQUIRED
扩大权限
降低隐私约束
擅自增加高风险外部副作用
```

---

## 226. Contract 版本化

用户修改要求时 MUST 创建：

```
Contract vN+1
```

MUST NOT 原地修改已有版本。

Runtime MUST 计算：

```
ContractDiff {
    added
    removed
    strengthened
    weakened
}
```

受影响的 ADFIR 节点 MUST 被重新判定有效性。

---

## 227. Specification Object

自然语言 Intent 进入 Planner 前 SHOULD 编译成：

```
Specification {
    goals[]
    invariants[]
    acceptance_predicates[]
    forbidden_states[]
    preferences[]
    unknowns[]
}
```

"unknowns" MUST 被正式保存。

系统 MUST NOT 把未知项自动转换成假设。

---

## 228. Plan 数据模型

```
Plan {
    id
    version
    intent_id
    type

    objectives[]
    milestones[]
    assumptions[]
    risks[]

    acceptance_conditions[]

    review_state
}
```

"type"：

```
STRATEGIC
EXECUTION
```

---

## 229. Plan Review State

```
DRAFT
SELF_REVIEWED
AI_REVIEWED
USER_APPROVED
REJECTED
SUPERSEDED
```

Strategic Plan MUST 在需要重大用户承诺时进入：

```
USER_APPROVED
```

才可以执行。

---

## 230. Plan Change Classification

```
PlanChange {
    class
    affected_scope
}
```

"class"：

```
MINOR
MATERIAL
```

MINOR 示例：

```
模型替换
重试
并行调整
工具替换
内部步骤重排
```

MATERIAL 示例：

```
重大架构变化
Requirement 改变
权限扩大
成本显著扩大
Milestone 目标变化
```

MATERIAL MUST 重新触发相应 Review Gate。

---

## 231. Product Object

```
Product {
    id
    intent_id

    baseline
    readiness
    current_milestone
    milestone_history[]

    shippable_predicate
}
```

---

## 232. Product Readiness

```
ProductReadiness {
    functional
    correctness
    reliability
    performance
    security
    usability
    maintainability
    deployability
    documentation
}
```

该对象 SHOULD 保存：

```
value
evidence
last_verified
```

而不是仅保存主观评分。

---

## 233. Product Drive

Product Drive MUST 使用有限视野规划。

禁止：

```
一次生成整个未来无限 Milestone
```

应采用：

```
observe
↓
gap analysis
↓
next milestone
↓
execute
↓
verify
↓
observe again
```

---

## 234. Milestone 数据模型

```
Milestone {
    id
    product_id

    objective
    dependencies[]

    entry_conditions[]
    exit_conditions[]

    state
}
```

状态：

```
PROPOSED
APPROVED
ACTIVE
VERIFYING
COMPLETED
FAILED
SUPERSEDED
```

---

## 235. ADFIR 定义

ADFIR：

```
Adaptive Dynamic Flow Intermediate Representation
```

ADFIR MUST 表示为：

```
ADFIRGraph {
    id
    version

    nodes[]
    edges[]

    entry_nodes[]
    terminal_nodes[]

    contract_id
}
```

ADFIR 是 Runtime 的统一执行 IR。

---

## 236. ADFIR Node

```
Node {
    id
    operator

    inputs[]
    outputs[]

    constraints[]
    capabilities[]

    execution_policy
    verification_policy

    state
}
```

节点 MUST NOT 直接隐式访问未声明资源。

---

## 237. ADFIR Node State

```
PENDING
READY
QUEUED
RUNNING

WAITING
BLOCKED
SUSPENDED

VERIFYING

COMPLETED
FAILED
CANCELLED
INVALIDATED
LOST
```

---

## 238. Edge 类型

ADFIR 至少支持：

```
DATA
CONTROL
DEPENDENCY
EVIDENCE
EFFECT
INVALIDATION
```

"DATA"：

```
Artifact flow
```

"CONTROL"：

```
execution dependency
```

"EVIDENCE"：

```
verification relationship
```

---

## 239. Port 类型系统

每个 Node input/output MUST 有明确类型。

例如：

```
Artifact<Image>
Artifact<Video>
Artifact<Timeline>
Artifact<SourceTree>
Artifact<Patch>
Artifact<TestResult>
Artifact<Scene>
```

运行时 MUST 拒绝不兼容连接。

---

## 240. Artifact 数据模型

```
Artifact {
    id
    type

    content_hash
    size

    producer_node
    input_artifacts[]

    metadata
    provenance

    privacy_class
    version
}
```

---

## 241. Artifact 不可变原则

Artifact 内容一旦提交：

```
MUST be immutable
```

修改必须产生：

```
Artifact v2
```

而不是覆盖 v1。

---

## 242. Artifact Lineage

系统 MUST 能追踪：

```
Artifact A
   ↓
Node X
   ↓
Artifact B
   ↓
Node Y
   ↓
Artifact C
```

任意最终结果 SHOULD 能反向追溯到原始输入。

---

## 243. Artifact Privacy Class

至少支持：

```
PUBLIC
PERSONAL
PRIVATE
SECRET
LOCAL_ONLY
```

Scheduler MUST 根据 Artifact privacy 和 Node trust 决定 placement。

---

## 244. Operator 定义

```
Operator {
    id
    version

    input_schema
    output_schema

    determinism
    side_effect_class

    backend_candidates[]
}
```

---

## 245. Operator Backend

同一个 Operator MAY 有多个 backend。

例如：

```
Transcribe
├ local-whisper
├ cloud-model
└ specialized-asr
```

ADFIR 只表达：

```
Transcribe
```

具体 backend 由 Router 决定。

---

## 246. Execution Profile

每次 Node 执行产生：

```
ExecutionProfile {
    model?
    provider?
    tool?
    backend?

    compute_node

    reasoning_effort
    parallelism

    timeout
    retry_policy

    cost_budget
}
```

---

## 247. Model Capability Profile

```
ModelProfile {
    id
    version

    provider
    model_revision

    modalities[]
    tools[]

    skill_vector
    failure_modes[]

    latency_profile
    cost_profile

    evidence_count
    confidence
}
```

---

## 248. Skill Vector

```
SkillVector {
    reasoning
    coding
    vision
    planning
    tool_use
    constraint_following
    verification
    spatial
    media
}
```

每个维度 MAY 继续分层。

禁止只维护：

```
overall_score
```

作为唯一路由依据。

---

## 249. Model Lifecycle

```
DISCOVERED
UNPROFILED
RESEARCHED
PROBED
VERIFIED
ACTIVE
```

异常：

```
STALE
DEGRADED
QUARANTINED
DISABLED
```

Router MUST NOT 自动使用：

```
UNPROFILED
QUARANTINED
DISABLED
```

模型。

---

## 250. Model Routing

Router 输入：

```
TaskSkillRequirement
ModelProfile
CostPolicy
LatencyPolicy
FamilyPreference
FailureHistory
```

输出：

```
RankedExecutionCandidates
```

最终选择 MUST 考虑：

```
能力匹配
失败模式
成本
延迟
上下文长度
模态
工具支持
当前可用性
```

---

## 251. Automatic Escalation

当模型失败时 MAY：

```
Tier1
↓
Tier2
↓
Tier2 High
↓
Cross-family
↓
Specialized Model
```

前提：

```
仍满足 Contract 和 Budget
```

---

## 252. Tool Definition

```
Tool {
    id
    version

    input_schema
    output_schema

    required_capabilities[]
    effect_class

    deterministic
}
```

Tool MUST NOT 接收未声明 capability。

---

## 253. Capability Token

Capability：

```
Capability {
    resource
    action
    scope
    expiry
    issuer
}
```

例如：

```
filesystem.read:/project
git.write:/task-worktree
github.create_pr:repo/X
```

禁止：

```
full_access = true
```

作为默认权限。

---

## 254. Workspace Model

基础工作区：

```
BaseWorkspace
```

默认 MUST：

```
READ_ONLY
```

AI 写操作只能进入：

```
TaskWorkspace
```

---

## 255. Task Workspace

Git 项目 SHOULD 使用：

```
git worktree
```

非 Git 项目 MAY 使用：

```
OverlayFS
Btrfs snapshot
ZFS clone
CoW workspace
container layer
```

---

## 256. Workspace Invariant

Runtime 必须保证：

```
AI_WRITE(BaseWorkspace) = DENY
```

即使模型请求直接写入 BaseWorkspace，Tool Runtime MUST 拒绝。

---

## 257. Integration Gate

TaskWorkspace 的修改进入 BaseWorkspace 前 MUST 经过：

```
Integration Gate
```

支持：

```
view diff
apply patch
cherry-pick
merge
discard
```

---

## 258. Evidence 数据模型

```
Evidence {
    id
    type

    subject
    claim

    producer
    artifact_refs[]

    strength
    scope
}
```

Evidence 类型例如：

```
TEST
FUZZ
PROPERTY
FORMAL_PROOF
STATIC_ANALYSIS
BENCHMARK
VISUAL_CHECK
METADATA_CHECK
HUMAN_CONFIRMATION
```

---

## 259. Test Evidence 原则

测试通过 MUST NOT 直接代表：

```
requirement_satisfied = true
```

只能产生：

```
Evidence<Test>
```

是否足够由 Verification Runtime 决定。

---

## 260. Requirement Coverage Graph

每项 REQUIRED SHOULD 建立：

```
Requirement
↓
Evidence Set
```

例如：

```
R1
├ UnitTest
├ PropertyTest
└ FuzzEvidence
```

Requirement 没有充分 evidence 时：

```
verification = INCOMPLETE
```

---

## 261. Verification Profile

```
VerificationProfile {
    required_evidence_types[]
    optional_evidence_types[]

    fuzz_budget?
    mutation_threshold?
    property_requirements[]

    independent_verifier_required
}
```

---

## 262. Verifier 优先级

默认顺序：

```
1 deterministic checker
2 formal/static checker
3 independent model verifier
4 cross-family verifier
5 human review
```

可确定性验证 SHOULD 优先于模型判断。

---

## 263. Independent Verifier

Verifier MUST 尽可能与 Worker 隔离。

SHOULD 不共享：

```
完整 Worker reasoning
Worker 自己的完成声明
```

初次验证 SHOULD 输入：

```
Task Contract
Artifact
Evidence
```

---

## 264. Verifier Adversary

高价值任务 MAY 创建：

```
VerifierAdversary
```

目标：

```
寻找错误实现
但当前 verifier 仍 PASS
```

若找到反例：

```
verification_strategy = insufficient
```

---

## 265. Mutation Verification

适用任务 SHOULD 执行：

```
Mutation Testing
```

如果大量 mutation 无法被测试捕获：

```
test evidence strength ↓
```

---

## 266. Completion Predicate

Intent 完成必须由：

```
CompletionPredicate
```

裁定。

例如：

```
artifact_exists
AND
all_required_requirements_verified
AND
contract_satisfied
AND
mandatory_effects_completed
AND
no_blocking_verification_failure
```

模型 MUST NOT 自己宣告：

```
"我认为已经完成"
```

作为最终依据。

---

## 267. Effect 数据模型

```
Effect {
    id
    type

    target
    parameters

    authorization
    idempotency_key

    state
}
```

Effect 示例：

```
send_email
push_branch
publish
delete_remote
charge
deploy
```

---

## 268. Effect Journal

所有不可纯 Artifact 回滚的 Effect MUST 写入：

```
EffectJournal
```

执行前必须记录：

```
intent
authorization
idempotency_key
planned_effect
```

---

## 269. Effect State

```
PLANNED
AUTHORIZED
EXECUTING
COMMITTED
FAILED
ROLLED_BACK
UNKNOWN
```

Runtime 重启后 MUST 根据 Journal 恢复状态。

---

## 270. Input Canonicalization Frontend

用户输入进入 Intent Compiler 前 SHOULD 经过：

```
Input Canonicalization Frontend
```

组件：

```
Surface Normalizer
Mention Detector
Candidate Retriever
Reference Resolver
Alias Resolver
Temporal Resolver
Ambiguity Manager
```

---

## 271. Resolution Result

```
ResolutionResult =
    Resolved
  | ConfirmRequired
  | Ambiguous
  | Unknown
```

---

## 272. Resolution Candidate

```
Candidate {
    entity_id
    evidence[]
    retrieval_score?
}
```

"retrieval_score" MAY 用于排序。

MUST NOT 自动被当成真实概率。

---

## 273. Open-World Resolution

每次解析 MUST 保留：

```
UNKNOWN / NIL
```

可能性。

系统 MUST NOT 因 Registry 中存在候选就强行绑定。

---

## 274. Resolution Scope

```
ResolutionScope {
    explicit
    session
    conversation
    project
    workspace
    tool
    plugin
    user
    global
}
```

显式 scope 优先级最高。

---

## 275. Resolution Confidence Gate

Runtime MAY 使用校准后的：

```
ResolutionConfidence
```

但该值只决定：

```
是否允许自动执行
```

不定义：

```
"实体为真的概率"
```

---

## 276. Risk-Adaptive Resolution Threshold

要求：

```
required_resolution_strength
=
f(effect, cost, reversibility, privacy)
```

风险越高，自动解析要求越严格。

---

## 277. Manual Binding

当解析不足时：

用户选择候选

生成：

```
ExplicitBinding {
    mention
    entity
    scope
    source = USER
}
```

该 Binding SHOULD 在当前 scope 拥有最高优先级。

---

## 278. Alias Scope

Alias 分：

```
TURN
SESSION
PROJECT
USER_PERSISTENT
```

系统 MUST NOT 默认把 Session Alias 永久化。

---

## 279. Context Compiler

Node 执行前 MUST 通过：

```
Context Compiler
```

从：

```
Conversation
Memory
Artifacts
Project
Tool results
```

选择最小必要上下文。

---

## 280. Context Package

```
ContextPackage {
    instructions[]
    references[]
    artifacts[]
    memories[]

    provenance[]
    trust_labels[]
}
```

---

## 281. Context Provenance

每个上下文片段 SHOULD 标注：

```
USER_INSTRUCTION
TRUSTED_RUNTIME
PROJECT_DATA
WEB_UNTRUSTED
TOOL_OUTPUT
MEMORY
MODEL_OUTPUT
```

外部内容 MUST NOT 与用户指令处于同一信任级别。

---

## 282. Memory Source Graph

长期 Memory MUST 基于：

```
Memory Sources
```

而不是仅维护一个人工条目表。

Source 可能来自：

```
conversation
artifact
task history
project
explicit correction
connected source
```

---

## 283. Memory Claim

```
MemoryClaim {
    id
    scope

    statement
    sources[]

    validity
    confidence
    last_confirmed
}
```

---

## 284. Memory Scope

至少支持：

```
PERSONAL
PROJECT
PRODUCT
TASK
RUNTIME
```

---

## 285. Memory Synthesis

后台 MAY 执行：

```
Memory Synthesis
```

用于：

```
去重
冲突处理
更新时效
归纳长期事实
生成 Memory Summary
```

---

## 286. Memory Correction

用户纠正 MUST 产生新的：

```
Correction Source
```

并使旧 claim：

```
SUPERSEDED
```

而不是静默覆盖历史。

---

## 287. Compute Node

```
ComputeNode {
    id
    class

    capabilities
    resources

    trust
    availability
}
```

"class"：

```
PERSONAL
TEMPORARY
```

---

## 288. Personal Node

Personal Node MAY：

```
执行任务
缓存 Artifact
运行模型
后台计算
```

但设备加入信任域 MUST 由 Authority 控制。

---

## 289. Temporary Node

Temporary Node MUST：

```
lease-based
least-privilege
job-scoped
```

默认禁止：

```
personal memory
global secret
network discovery
persistent trust
```

---

## 290. Job Capsule

Temporary Node 只接收：

```
JobCapsule {
    subgraph
    required_artifacts
    scoped_credentials
    execution_policy
    expiry
}
```

---

## 291. Node Placement

Scheduler SHOULD 考虑：

```
CPU
GPU
VRAM
RAM
bandwidth
artifact locality
latency
money
privacy
trust
current load
```

---

## 292. Authority Host

个人网络 MUST 存在唯一：

```
Authority Host
```

只有它可以：

```
批准新 Personal Node
撤销 Personal Node
改变 Trust Graph
签发长期设备身份
```

---

## 293. Device Join

```
New Device
↓
Join Request
↓
Authority Host user approval
↓
TOTP on new device
↓
public key binding
↓
Trusted Personal Node
```

其他 Personal Node MUST NOT 批准新设备。

---

## 294. Standby Device

可配置一个：

```
Standby Device
```

平时：

```
Authority = false
```

只保存：

```
signed registry replica
encrypted state
failover capability
```

---

## 295. Temporary Authority

激活条件：

```
Primary unreachable
AND
user TOTP verified
```

之后进入：

```
TEMPORARY_AUTHORITY
```

---

## 296. Temporary Authority 限制

默认 SHOULD 禁止：

```
永久新增 Personal Node
改变根信任
永久撤销核心身份
```

主要用于：

```
继续运行现有任务
调度已有节点
访问已有 Artifact
```

---

## 297. Permanent Promotion

确认 Primary 永久丢失后：

```
authority_epoch++
```

Standby 可晋升：

```
PRIMARY
```

旧 Authority 自动成为：

```
STALE_AUTHORITY
```

---

## 298. Standby 数据加密

Standby Snapshot SHOULD 使用：

```
随机 256-bit DEK
```

数据：

```
AEAD(DEK, snapshot)
```

DEK：

```
wrapped by standby public key
```

TOTP MUST NOT 直接用于加密数据。

---

## 299. Background Supervisor

后台任务 MUST 来自：

```
authorized Intent
policy
maintenance schedule
product drive
```

不得因为模型"觉得有必要"就无限创建后台任务。

---

## 300. Resource Arbiter

资源优先级：

```
User workload
>
Foreground Runtime
>
Background Runtime
>
Maintenance
```

Resource Arbiter SHOULD 只向 AI 暴露最小化资源结论。

---

## 301. Scheduler 分层

必须区分：

```
Intent Scheduler
Graph Scheduler
Resource Scheduler
```

不得把三者实现成一个不可分离的大模型决策。

---

## 302. Intent Scheduler

负责：

```
现在推进哪个 Intent
```

输入：

```
priority
deadline
blocking
user focus
background suitability
```

---

## 303. Graph Scheduler

负责：

```
哪些 ADFIR Node READY
哪些可以并发
哪些必须等待
```

主要为确定性 Runtime 逻辑。

---

## 304. Resource Scheduler

负责：

```
Node 在哪里执行
用哪个模型
哪个工具
多少并行
```

---

## 305. Incremental Recompute

节点输出 SHOULD 可缓存。

如果：

```
input_hash unchanged
operator_version unchanged
contract unaffected
```

Runtime MAY 直接复用 Artifact。

---

## 306. Graph Invalidation

当输入或 Contract 改变：

```
affected node
↓
dependent descendants
```

必须被：

```
INVALIDATED
```

不相关节点不得无意义重算。

---

## 307. Node Retry

```
RetryPolicy {
    max_attempts
    backoff
    retryable_errors[]
    escalation_policy
}
```

非幂等 Effect MUST NOT 直接自动重试。

---

## 308. Checkpoint

长任务 Operator SHOULD 支持：

```
checkpoint
```

尤其：

```
video generation
training
large compilation
distributed compute
```

---

## 309. Failure Classification

至少支持：

```
TRANSIENT
PERMANENT
CONSTRAINT
AUTHORIZATION
RESOURCE
VERIFICATION
UNKNOWN
```

不同 failure type 必须采用不同恢复策略。

---

## 310. Event Stream

Runtime MUST 提供结构化事件：

```
intent.created
plan.review_required
node.started
node.completed
artifact.created
verification.failed
decision.required
effect.committed
intent.completed
```

---

## 311. UI 默认信息级别

默认 UI：

```
简化执行状态
最终结果
必要 Decision
```

展开后 MAY 显示：

```
Execution Trace
Artifacts
Verifier Results
Plan
Model / Tool selection
```

不得将原始内部 chain-of-thought 作为执行日志。

---

## 312. Execution Trace

Trace MUST 记录事实：

```
node started
tool invoked
artifact produced
verification failed
retry occurred
```

而不是不可验证的内部推理叙述。

---

## 313. Audit Log

以下事件 MUST 记录：

```
authority changes
device join
device revoke
capability grants
external effects
contract changes
user approvals
runtime self-update
```

---

## 314. API

PIER 最低 API：

```
Intent API
Contract API
Plan API
Artifact API
Execution API
Model Registry API
Tool Registry API
Node API
Policy API
Authority API
Memory API
Event API
```

---

## 315. Provider Adapter

```
ModelProvider {
    list_models()
    describe_model()
    invoke()
    stream()
    cancel()
    usage()
    health()
}
```

---

## 316. Tool Adapter

```
ToolProvider {
    list_tools()
    describe_tool()
    invoke()
    cancel()
}
```

---

## 317. Persistence

以下状态 MUST 持久化：

```
Intent
Contract
Plan
ADFIR
Artifact metadata
Effect Journal
Authority state
Model Profile
Memory Claim
Policy
```

Runtime crash 后 MUST 能恢复。

---

## 318. Transaction Boundary

关键状态更新 SHOULD 使用事务：

```
node output commit
effect commit
contract update
authority update
```

避免：

```
Artifact 已创建
但节点状态仍 RUNNING
```

之类不一致。

---

## 319. Runtime Recovery

启动时：

```
load durable state
↓
reconcile incomplete effects
↓
reconcile running nodes
↓
mark lost executions
↓
resume eligible tasks
```

---

## 320. Runtime Learning

PIER MAY 根据历史执行更新：

```
routing policy
planning heuristics
execution method
verification strategy
context selection
```

---

## 321. Runtime Self-Modification Boundary

Runtime MUST NOT 自动修改：

```
Authority semantics
Contract semantics
Permission invariants
User approval requirements
Core security policy
```

这些属于：

```
Protected Runtime Core
```

---

## 322. Protected Runtime Core

```
ProtectedCore {
    authority_rules
    capability_rules
    contract_rules
    audit_rules
    effect_rules
}
```

更新 MUST：

```
显式版本化
独立验证
用户批准
```

---

## 323. Image Generation Operator

```
GenerateImage {
    prompt
    references[]
    dimensions
    constraints
}
```

输出：

```
Artifact<Image>
```

---

## 324. Image Local Edit Operator

```
LocalImageEdit {
    source_image
    edit_region
    instruction

    preserve_outside_region = true
}
```

---

## 325. Edit Region

支持：

```
Point
Box
Mask
SemanticObject
```

Point MUST 被解析为：

```
semantic edit region
```

而不是单像素修改。

---

## 326. Local Edit Preservation

默认：

```
outside_mask_change = FORBIDDEN
```

若 backend 会重生成整张图：

Runtime SHOULD 使用：

```
generated patch
+
hard composite
```

保持 mask 外原像素。

---

## 327. Image Lineage

每次编辑必须创建新 Artifact：

```
Image v1
↓
Image v2
↓
Image v3
```

不得默认覆盖原图。

---

## 328. Media Operator

视频领域 SHOULD 使用标准 Artifact：

```
Video
Audio
ShotSet
Transcript
Timeline
SubtitleTrack
Render
```

而不是额外创建媒体专属 Runtime。

---

## 329. Timeline Artifact

```
Timeline {
    tracks[]
    clips[]
    transitions[]
    effects[]
    audio_mix
    metadata
}
```

Timeline 属于普通：

```
Artifact<Timeline>
```

---

## 330. Research Operator

研究任务 SHOULD 拆成：

```
Question
↓
Search
↓
SourceSet
↓
EvidenceExtraction
↓
ContradictionCheck
↓
Synthesis
↓
CitationVerification
```

---

## 331. Decision Object

需要用户决定时 MUST 创建：

```
Decision {
    issue
    options[]
    consequences[]
    blocking
}
```

而不是只发一句自由文本问题。

---

## 332. Decision Resume

用户选择后：

```
DecisionResult
↓
Contract / Plan update
↓
Graph invalidation
↓
Resume
```

---

## 333. Budget Object

```
Budget {
    money
    wall_time
    token
    gpu_time
    network_transfer
}
```

Runtime MUST NOT 无限制消耗。

---

## 334. Exploration Budget

寻找替代方案也属于预算。

默认可自动执行：

```
低成本
可逆
短时间
无外部副作用
```

高成本探索必须进入 Decision Gate。

---

## 335. Shippable Predicate

Product MUST 明确定义：

```
ShippablePredicate
```

达到后才能宣告：

```
SHIPPABLE
```

---

## 336. Continuous Product Drive

如果用户启用：

```
ContinuousProductDrive = true
```

达到 Shippable 后 MAY 继续：

```
维护
优化
性能提升
可靠性提升
技术债处理
```

但 MUST 区分：

```
Shippable
```

与：

```
Post-shippable improvement
```

---

## 337. Progress Gate

Product Drive 必须检测：

```
progress_delta
```

连续多个 Milestone 无实际进展时 MUST：

```
replan
```

不得无限空转。

---

## 338. 计划审查三层结构

重大 Strategic Plan：

```
Planner
↓
Self Review
↓
Stronger Independent AI Review
↓
Revision
↓
User Review
```

只有通过后才进入正式执行。

---

## 339. AI Reviewer 输出规范

```
ReviewFinding {
    severity

    subject
    problem
    evidence
    recommendation
}
```

Severity：

```
BLOCKER
MAJOR
MINOR
SUGGESTION
```

---

## 340. Self Review 与 Independent Review 分离

同一模型 MAY 做 Self Review。

Independent Review SHOULD：

```
使用独立上下文
优先使用更强模型
必要时跨模型 family
```

避免相关错误。

---

## 341. User Review View

用户默认只审：

```
目标
重大设计
关键约束
Milestone
风险
成本
权限
验收条件
```

而不是所有内部执行步骤。

---

## 342. 规范最高不变量

所有符合 PIER 的实现 MUST 始终满足以下不变量：

```
1. 用户 REQUIRED 不得被 Runtime 静默降低。

2. Base Workspace 默认不可写。

3. 高风险 Effect 必须服从 Authority 与 Policy。

4. Artifact 修改必须产生新版本。

5. 测试通过不能直接等价于 Requirement 已满足。

6. Resolver 不确定时必须允许 UNKNOWN / AMBIGUOUS。

7. Temporary Node 不得自动进入 Personal Trust Domain。

8. 非 Authority Host 不得批准新 Personal Node。

9. Runtime 可以改变执行方法，但不能自行取消边界规则。

10. 完成必须由 Completion Predicate 与 Verification 决定，
    不由 Worker 模型自行宣布。
```

---

## 343. v0.1 最小实现范围

首个符合 v0.1 的实现只需完成：

```
Intent
Contract
Strategic Plan
Plan Review

ADFIR
Artifact
Node Scheduler

Model Router
Tool Runtime

Read-only Base Workspace
Task Workspace

Evidence
Verifier
Completion Predicate

Event Stream
Persistence
```

可以暂不实现：

```
Distributed Compute
Temporary Authority
Full Product Drive
Image Local Edit
Advanced Memory Synthesis
Runtime Self-Improvement
```

---

## 344. v0.1 MVP 执行闭环

最低必须成功运行：

```
User Intent
↓
Canonicalization
↓
Task Contract
↓
Strategic Plan
↓
Review
↓
ADFIR
↓
Isolated Workspace
↓
Execution
↓
Artifact
↓
Evidence
↓
Independent Verification
↓
Completion Predicate
↓
Result
```

---

## 345. v0.1 推荐首个测试任务

推荐使用：

```
真实代码仓库 Bug 修复
```

因为它可以同时验证：

```
Requirement
Plan
Worktree
Tool use
Model routing
Patch artifact
Test evidence
Verifier
Completion
```

---

## 346. 兼容性原则

PIER SHOULD：

```
模型无关
Provider 无关
工具无关
计算节点无关
前端无关
```

核心语义不能绑定某一个商业模型。

---

## 347. 最终执行定义

符合本规范的 Runtime，其执行语义可以定义为：

```
Execute(Intent I):
    S = Specify(I)
    C = CompileContract(S)
    P = Plan(C)
    P = Review(P)

    G = CompileADFIR(P)

    while not Completed(I):
        N = ScheduleReadyNodes(G)

        for node in N:
            E = ExecuteNode(node)
            A = CommitArtifacts(E)
            V = Verify(A)

            UpdateGraph(G, V)

        ReplanIfNecessary(G)

    return VerifiedArtifacts(I)
```

---

## 348. PIER 核心定义

PIER 最终定义：

> «PIER（Persistent Intent Execution Runtime）是一个以持久 Intent 为最高任务语义，以版本化 Contract 约束执行，以 ADFIR 表示动态任务图，以 Artifact 与 Evidence 表示可验证执行结果，并在模型、工具、权限和计算节点之间自动进行安全调度的通用智能执行运行时。»

核心思想不是：

```
让模型决定一切
```

而是：

```
把能够确定化的系统行为交给 Runtime，
把需要智能判断的部分交给模型，
再由独立 Evidence 与 Verification 判断结果是否成立。
```

最终边界：

```
User defines intent and authority.
PIER controls execution.
Evidence controls completion.
```
