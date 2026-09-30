# 研究与工程定位 + 设计深化

本文件对应原始第二份材料。其 §62、§63 与第一份材料编号冲突，按 `README.md` 的约定记为 §62b、§63b。

---

## 62b. 研究与工程定位

本项目不应简单定义为：

```
Agent Framework
```

也不应只是：

```
Multi-Agent System
Model Router
AI Assistant
Agent OS
```

更准确的定位是：

> «面向个人与个人计算网络的通用 Intent Execution Runtime。»

它试图统一目前通常被分散实现的几个层次：

```
自然语言 Intent
        ↓
需求契约
        ↓
动态执行图
        ↓
Agent / Model / Tool 编排
        ↓
计算资源调度
        ↓
权限与隐私控制
        ↓
Artifact
        ↓
独立验证
        ↓
长期反馈
```

研究重点并不在于某一个模型本身，而在于：

> «如何将不可靠、能力各异、成本不同、模态不同的模型组织成一个长期可靠的执行系统。»

因此系统的核心研究对象包括：

```
Intent semantics
Constraint-preserving planning
Dynamic execution graph
Model capability estimation
Model routing
Independent verification
Distributed execution
Capability security
Long-running autonomy
Human intervention boundaries
```

---

## 63b. 系统最终抽象

整个 Runtime 最终可以压缩成：

```
Runtime.solve(Intent, Context, Authority)
    -> Artifact + Evidence
```

其中：

```
Intent
= 用户希望达到什么状态

Context
= 当前允许 Runtime 使用的信息与资源

Authority
= Runtime 在本任务中能够产生什么影响
```

而 Runtime 内部完成：

```
Understand
Plan
Route
Execute
Verify
Commit
Learn
```

整个产品最重要的外部抽象不是：

```
chat()
```

而应是：

```
execute(intent)
```

聊天只是创建、修改和观察 Intent 的一种接口。

---

## 64. Intent 版本化

长期任务中，用户要求一定会发生变化。

因此 Intent 和 Task Contract 必须拥有版本：

```
Intent v1
 ↓
Task Contract v1
 ↓
Execution Graph
```

用户后来提出：

```
"这个接口不要改了。"
```

生成：

```
Task Contract v2
```

此时 Runtime 必须计算：

```
v1 → v2
```

产生的约束差异。

然后对已有执行图做：

```
仍然有效的节点
→ 保留

结果可能受新约束影响
→ INVALIDATE

正在运行但违反新约束
→ CANCEL / SUSPEND
```

不能简单把一句新要求附加进上下文然后继续运行。

---

## 65. Graph Invalidation

ADFIR 必须支持局部失效传播。

例如：

```
A → B → C → D
        ↓
        E
```

如果 B 的输入发生变化：

```
B invalid
 ↓
C invalid
 ↓
D invalid
E invalid
```

但：

```
A
```

仍然有效。

这允许：

```
修改需求
模型失败
Artifact 更新
用户替换输入
```

之后只重新计算必要部分。

---

## 66. Incremental Execution

Runtime 应优先采用：

```
incremental recomputation
```

而不是：

```
重新执行整个任务
```

每个 Operator 应声明：

```
inputs
outputs
dependency hash
side effects
cacheability
determinism
```

如果：

```
input_hash unchanged
+
operator_version unchanged
```

就可以复用已有 Artifact。

---

## 67. Agent 不是一等底层原语

Agent 应被视为一种执行结构，而不是 Runtime 最底层对象。

例如：

```
Agent {
    context
    model policy
    capabilities
    subgraph
}
```

最终仍然 lower 成：

```
ADFIR nodes
```

因此：

```
single agent
multi agent
subagent
team
workflow
```

只是不同的图结构。

---

## 68. Agent 生命周期

Agent 状态：

```
CREATED
READY
RUNNING
WAITING
SUSPENDED
VERIFYING
COMPLETED
FAILED
CANCELLED
```

长期 Agent 可以：

```
RUNNING
 ↓
checkpoint
 ↓
SUSPENDED
 ↓
下一次 activation
 ↓
RUNNING
```

Agent 不应该依赖单个模型 conversation state 才能继续执行。

---

## 69. Agent Context Isolation

不同 Agent 默认拥有不同上下文。

例如：

```
Root Agent
├── Coding Agent
├── Research Agent
└── Verification Agent
```

Coding Agent 不应自动获得：

```
用户全部聊天
其他项目
邮件
个人 Memory
```

它只获得：

```
本任务需要的信息
Task Contract
相关 Artifact
必要工具
```

原则：

> «Context 也属于 capability。»

---

## 70. Context Compiler

不能直接把所有信息塞进模型 context。

增加：

```
Context Compiler
```

负责从：

```
Intent
Artifact
Memory
Tool Result
Conversation
Project State
```

选择当前节点真正需要的信息。

输出：

```
Model Context Package
```

目标：

```
减少 token
降低隐私暴露
减少无关信息
降低 prompt injection 面
```

---

## 71. Context Provenance

进入模型的每一段重要数据都应该知道来源。

例如：

```
source = user_instruction
source = web_untrusted
source = repository
source = verifier
source = memory
source = tool_output
```

模型上下文中的信息不能全部被等价处理。

尤其：

```
user_instruction
```

与：

```
untrusted_document_content
```

必须在 Runtime 层严格区分。

---

## 72. Memory Architecture

Memory 不应该只是一个巨大向量数据库。

建议分：

```
Session Memory
Task Memory
Project Memory
Personal Memory
Runtime Memory
```

Session Memory：

```
当前交互临时状态
```

Task Memory：

```
一个 Intent 的长期执行状态
```

Project Memory：

```
某个代码库、媒体项目等长期事实
```

Personal Memory：

```
用户长期偏好
```

Runtime Memory：

```
模型表现
工具行为
调度经验
```

这些数据的权限级别不同。

---

## 73. Memory Write Gate

Agent 不能因为"觉得重要"就随便写长期 Memory。

写入流程：

```
Candidate Memory
 ↓
classification
 ↓
scope selection
 ↓
privacy check
 ↓
deduplication
 ↓
commit
```

例如：

```
"这个 parser 使用 recursive descent"
```

应该进入：

```
Project Memory
```

而不是 Personal Memory。

---

## 74. Memory Expiration

不是所有 Memory 永久存在。

支持：

```
TTL
decay
superseded
invalidated
archived
```

例如：

```
Model X 当前价格
```

属于高时效信息。

而：

```
Project coding style
```

可能长期存在。

---

## 75. Planner Architecture

Planner 不应一次性生成巨型完整计划。

默认采用：

```
receding-horizon planning
```

也就是：

```
当前状态
 ↓
规划最近几个步骤
 ↓
执行
 ↓
获得新信息
 ↓
重新规划
```

避免：

```
第一分钟规划未来三小时
```

之后现实已经改变但计划仍死板执行。

---

## 76. Planner 与 Executor 分离

**Planner**

负责：

```
What should happen next?
```

而：

**Executor**

负责：

```
How is this node actually executed?
```

这样 Planner 不需要知道：

```
具体 API
具体 GPU
具体 shell command
```

这些交给 Router 和 Operator backend。

---

## 77. Scheduler 分层

至少需要三个调度层。

```
Intent Scheduler
决定哪个任务现在值得推进

Graph Scheduler
决定哪些 ADFIR 节点可以并发执行

Resource Scheduler
决定在哪个模型/设备/资源上执行
```

不能把它们全部塞进一个 scheduler。

---

## 78. Priority Model

任务优先级不能只依赖用户手动设置。

可以考虑：

```
deadline
dependency
user pin
expected value
blocking degree
resource opportunity
background suitability
```

但用户显式 priority 应拥有最高语义权重。

---

## 79. Fairness

后台存在多个长期 Intent 时，需要避免：

```
一个大型任务永远占满所有资源
```

可以支持：

```
weighted fair scheduling
```

并给交互式任务更高优先级：

```
Interactive
> Foreground autonomous
> Background
> Maintenance
```

---

## 80. Model Provider Adapter

系统不能直接绑定某一家模型 API。

需要统一 Provider Interface：

```
ModelProvider {
    discover()
    capabilities()
    invoke()
    stream()
    cancel()
    usage()
}
```

具体实现：

```
OpenAI Adapter
Anthropic Adapter
Local Model Adapter
Custom API Adapter
```

---

## 81. Model Identity

模型身份不能只使用：

```
"model-name"
```

应该至少包含：

```
provider
model_id
revision
deployment
capability fingerprint
```

因为：

```
同名模型
```

可能被供应商静默更新。

---

## 82. Behavioral Fingerprint

Runtime 可以维护轻量：

```
behavior fingerprint
```

周期性执行少量稳定 probes。

如果表现突然变化：

```
profile drift detected
```

则：

```
ACTIVE
 ↓
STALE
 ↓
重新 profiling
```

避免使用已经过时的 Model Profile。

---

## 83. Model Failure Modes

Model Registry 必须记录负能力。

不是只有：

```
coding = 9.2
```

还应该有：

```
failure_modes:
- loses constraints in very long tasks
- unreliable exact visual counting
- premature completion claims
- weak JSON compliance
```

Router 可针对任务避开这些缺陷。

---

## 84. Routing Confidence

每次 Model Routing 不只输出：

```
selected_model
```

还应该输出：

```
confidence
alternatives
reason
```

例如：

```
Model A
compatibility = 0.91
confidence = 0.94

Model B
compatibility = 0.90
confidence = 0.51
```

此时 A 更合理。

---

## 85. Automatic Escalation

如果低档模型执行失败，不应立即让用户处理。

可以：

```
Tier 1
 ↓ fail
Tier 2
 ↓ fail
Tier 2 + high reasoning
 ↓ fail
cross-family
```

但必须仍满足：

```
Task Contract
Cost Policy
```

---

## 86. Automatic De-escalation

反过来，复杂任务中也不应该一直使用昂贵模型。

例如：

```
复杂规划
→ Tier 2 High

随后 400 个机械文件检查
→ Tier 1 Low
```

这属于：

```
intra-task model scaling
```

---

## 87. Tool Registry

工具也需要像模型一样维护 Registry。

```
ToolProfile {
    capabilities
    input_schema
    output_schema
    effects
    cost
    latency
    trust
}
```

Agent 不应该自由输入任意字符串调用工具。

---

## 88. Tool Capability

工具权限通过 capability token 提供。

例如：

```
filesystem.read("/project")
```

而不是：

```
full filesystem access
```

Git 工具：

```
git.read
git.worktree.write
git.commit.local
```

与：

```
git.push
```

必须是不同 capability。

---

## 89. Tool Result Validation

工具调用成功不代表语义成功。

例如：

```
exit code = 0
```

不一定说明：

```
任务完成
```

所以工具结果必须转换成：

```
Evidence
```

再交给节点 Completion Predicate 判断。

---

## 90. Plugin / Extension System

系统需要扩展能力，但不采用"一切皆插件"。

核心 Runtime 语义固定：

```
ADFIR
Artifact
Capability
Effect
Verifier
Model
Node
```

插件只能扩展：

```
Operator
Provider
Tool
Artifact type
Verifier backend
```

不能改变核心安全模型。

---

## 91. Plugin Sandbox

第三方插件默认运行于隔离进程。

插件声明：

```
required capabilities
network access
filesystem access
secret access
```

用户或 Policy Engine 决定授权。

第三方插件不能直接访问 Runtime 数据库。

---

## 92. Artifact Store

Artifact Store 是系统真正的数据中心。

需要支持：

```
content-addressed storage
versioning
deduplication
metadata
provenance
encryption
replication
```

可以使用：

```
hash(content)
```

作为底层 identity。

---

## 93. Artifact Locality

分布式任务调度必须知道 Artifact 在哪里。

例如：

```
100GB video source
```

已经位于 Desktop。

那么把任务派给：

```
Desktop GPU
```

可能比传去 Server 更合理。

因此 placement cost 包括：

```
compute cost
+
data movement cost
```

---

## 94. Artifact Privacy Class

每个 Artifact 可以拥有：

```
PUBLIC
PERSONAL
PRIVATE
SECRET
LOCAL_ONLY
```

Node Scheduler 根据：

```
Artifact privacy
×
Node trust class
```

决定是否允许传输。

---

## 95. Personal Node Sync

Personal Node 不代表所有数据必须完全同步。

可以按：

```
metadata sync
artifact on demand
cache
replication policy
```

例如手机只同步：

```
任务状态
缩略图
metadata
```

而不是把：

```
500GB 视频素材
```

全部同步下来。

---

## 96. Temporary Node Data Erasure

Temporary Node 完成任务后：

```
Job Capsule
 ↓
Result returned
 ↓
lease closed
 ↓
credentials revoked
 ↓
workspace destroyed
```

Runtime 应尽可能验证清理成功。

对于完全无法信任的第三方节点，应假设：

> «已传输的数据可能被节点保留。»

因此真正敏感数据仍然不能发送。

---

## 97. Distributed Failure

节点可能在任务中途消失。

ADFIR Node 应声明：

```
restartable
checkpointable
replicable
```

如果 Temporary GPU 消失：

```
Running
 ↓
LOST
 ↓
寻找替代节点
 ↓
恢复 checkpoint
```

而不是整个 Intent 失败。

---

## 98. Distributed Duplicate Execution

某些高价值任务可以：

```
speculative execution
```

同时在两个节点执行。

第一个可靠结果返回后取消另一个。

适用于：

```
高延迟节点
不稳定节点
关键验证任务
```

但必须受资源预算限制。

---

## 99. Authority Registry

Authority Host 维护：

```
Device Registry
Capability Registry
Revocation Registry
Authority Epoch
Standby Identity
```

这些状态应以签名形式复制到 Personal Nodes。

其他节点：

```
可以验证
不能修改
```

---

## 100. TOTP Secret 管理

TOTP secret 属于高敏感 Authority 数据。

应优先：

```
TPM
Secure Enclave
hardware-backed keystore
```

而不是普通配置文件。

TOTP 用于：

```
device onboarding
failover activation
```

但不能：

```
直接派生 DEK
直接作为长期认证密钥
```

---

## 101. Failover State Machine

完整状态：

```
PRIMARY_AVAILABLE
 ↓
PRIMARY_UNREACHABLE
 ↓
FAILOVER_ELIGIBLE
 ↓ TOTP
TEMPORARY_AUTHORITY
```

如果 Primary 回归：

```
TEMPORARY_AUTHORITY
 ↓
RECONCILIATION
 ↓
STANDBY
```

确认永久丢失：

```
TEMPORARY_AUTHORITY
 ↓
PROMOTION
 ↓
epoch++
 ↓
PRIMARY
```

---

## 102. Split-Brain 防护

Runtime 必须假设：

```
unreachable ≠ dead
```

Temporary Authority 期间，限制涉及 Trust Graph 的操作。

Permanent Promotion 后：

```
Authority Epoch N+1
```

旧 Authority：

```
Epoch N
```

立即成为：

```
STALE
```

---

## 103. Key Rotation

以下事件需要触发 key rotation：

```
Authority compromise
Permanent failover
Device revocation
Credential leakage
```

可轮换：

```
transport keys
session keys
TOTP secret
delegated credentials
```

但不一定重新生成所有 Node Identity。

---

## 104. User Policy Engine

用户应该可以配置长期行为策略。

例如：

```
代码任务允许后台自动修复

视频任务超过 30 分钟先问我

禁止使用付费模型

私人照片禁止离开 Personal Network

新模型只允许低风险任务试用
```

这些规则不是聊天 Memory。

它们进入：

```
Policy Engine
```

---

## 105. Policy Precedence

优先级建议：

```
System Safety
>
Explicit Current Task Contract
>
User Persistent Policy
>
Project Policy
>
Runtime Default
>
Model Suggestion
```

避免不同规则冲突时行为不确定。

---

## 106. Human Decision Object

系统请求用户决定时，不应该只发一句自然语言。

应该创建：

```
Decision {
    issue
    alternatives
    consequences
    default?
    timeout?
}
```

例如：

```
当前无法满足 4K。

A. 使用云 GPU
   ¥8–12
   保持 4K

B. 改成 1080p
   免费
   违反当前 4K 要求

C. 等待 Desktop 节点上线
```

让用户真正知道自己在选择什么。

---

## 107. Decision Resume

用户做出决定后：

```
Decision Result
 ↓
Task Contract / Policy 更新
 ↓
Graph invalidation
 ↓
Resume
```

不能把用户回复当一条普通聊天消息然后重新理解整个任务。

---

## 108. Cost Model

成本包括：

```
money
latency
tokens
GPU time
CPU time
network traffic
power
storage
device contention
```

Router 根据任务策略优化。

---

## 109. Budget Object

每个 Intent 可以拥有：

```
Budget {
    money
    wall_time
    tokens
    gpu_time
    network_transfer
}
```

Runtime 自动执行时不得无限消耗资源。

---

## 110. Budget Escalation

如果预计超预算：

```
Current Plan
 ↓
Budget Validator
 ↓
exceeds budget
```

先寻找合法低成本方案。

找不到时：

```
Decision Required
```

---

## 111. Output Quality Contract

对于生成型任务，可以显式定义：

```
minimum_quality
target_quality
```

例如：

```
minimum resolution = 4K
preferred visual quality = high
```

这让系统知道：

```
什么绝不能退
什么只是优化目标
```

---

## 112. Long Task Progress

长任务不能只显示：

```
正在思考…
```

可以默认保持简洁，但展开后显示：

```
当前阶段
已完成阶段
阻塞原因
预计剩余工作类型
当前资源
```

不必展示内部 chain-of-thought。

---

## 113. Execution Trace

Execution Trace 应是结构化事实：

```
14:02 创建 worktree
14:03 reproducer confirmed
14:06 fuzz found failing case
14:11 patch generated
14:13 tests passed
```

而不是：

```
模型完整内部思维过程
```

---

## 114. Audit Log

涉及：

```
权限
外部副作用
Trust
设备加入
模型升级
用户要求修改
```

的操作必须进入不可轻易篡改的 Audit Log。

---

## 115. Replay

对于无副作用任务，应尽量允许：

```
replay
```

通过：

```
Task Contract
Graph version
Artifact hashes
Model identity
Tool versions
```

重现执行。

模型本身可能非确定，因此只能做到：

```
execution reproducibility
```

而不是保证逐 token 一致。

---

## 116. Deterministic Mode

部分高可靠任务可以开启：

```
Deterministic Execution Policy
```

要求：

```
固定模型 revision
固定 tool version
固定 temperature
固定 seed（如果支持）
禁止未知 provider fallback
```

适用于：

```
benchmark
formal verification
regression analysis
```

---

## 117. Testing Strategy

Runtime 自己需要多层测试：

```
Unit
Graph semantics
Constraint
Security
Distributed
Chaos
End-to-end
```

尤其要做：

```
节点突然断线
Provider API 返回错误
模型输出恶意工具参数
磁盘满
网络分区
Authority crash
Verifier disagreement
```

---

## 118. Chaos Testing

分布式部分需要主动注入故障：

```
packet loss
latency
node disappearance
clock drift
disk failure
duplicate message
reordering
```

确保 Runtime 不会因为现实网络异常破坏任务状态。

---

## 119. Security Testing

需要建立专门 adversarial suite：

```
prompt injection
tool injection
malicious plugin
malicious temporary node
stolen personal node
replayed join request
fake Authority
TOTP brute force
artifact substitution
model collusion
```

---

## 120. Model Collusion / Correlated Error

Worker 与 Verifier 可能共享错误模式。

因此关键任务应提高 diversity：

```
different family
different prompt
different evidence path
deterministic checker
```

而不是简单：

```
same model run twice
```

---

## 121. Verifier Disagreement

如果：

```
Verifier A = PASS
Verifier B = FAIL
```

不能简单多数投票。

应进入：

```
Adjudication
```

可能：

```
运行额外确定性检查
增加第三 verifier
降低结果 confidence
请求用户决定
```

---

## 122. Result Confidence

最终 Artifact 可以附带：

```
confidence
evidence coverage
verification level
```

例如：

```
修复完成

Verification:
compiler      PASS
tests         PASS
fuzz          PASS
independent   PASS

confidence: high
```

---

## 123. Unknown Handling

Runtime 必须允许正式表达：

```
UNKNOWN
```

不能强迫模型在未知情况下给出答案。

例如：

```
当前没有足够证据判断该 patch 是否会影响 ARM backend。
```

然后 Runtime 可以：

```
继续验证
或请求用户决定
```

---

## 124. Service Connectors

外部服务：

```
GitHub
Email
Calendar
Cloud Storage
CI
Issue Tracker
```

统一作为 Connector。

Connector 不等于 Agent。

它只是：

```
Capability Provider
```

---

## 125. Connector Permission

每个 Connector 按操作细分：

```
GitHub.read_repo
GitHub.create_issue
GitHub.push_branch
GitHub.merge

Email.read
Email.draft
Email.send
```

不能只有：

```
GitHub access
Email access
```

---

## 126. Offline Mode

系统必须允许：

```
no cloud
```

模式。

此时 Router 只选择：

```
local models
personal nodes
local tools
```

在线能力自动成为：

```
unavailable
```

而不是任务直接报错。

---

## 127. Graceful Capability Loss

任务过程中如果某能力消失：

```
cloud API unavailable
GPU node offline
paid quota exhausted
```

Runtime 应：

```
replan
```

只要仍然满足 Task Contract，就自动继续。

---

## 128. Provider Outage

Provider Registry 维护：

```
health
latency
rate limit
availability
```

Router 不应该持续把任务派给已明显故障的 Provider。

---

## 129. Rate Limit Scheduler

API rate limit 应作为资源。

例如：

```
Model A:
100 req/min
```

Graph Scheduler 自动控制并发，而不是等 429 后大量重试。

---

## 130. Media Pipeline Cache

视频、音频、3D 前处理很昂贵。

例如：

```
shot detection
ASR
embedding
camera estimation
```

全部应该生成可缓存 Artifact。

不同任务可复用。

---

## 131. Derived Artifact

例如：

```
Video
 ├─ Transcript
 ├─ ShotIndex
 ├─ FaceIndex
 ├─ AudioFeatures
 └─ Embeddings
```

这些属于：

```
Derived Artifacts
```

原始素材没变时无需重新计算。

---

## 132. User-Owned Data

默认原则：

> «用户产生的 Artifact 属于用户，而不是某个模型 Provider。»

应支持：

```
export
backup
delete
migrate
```

避免系统被某一家模型供应商锁死。

---

## 133. Runtime Portability

整个 Runtime 状态最好支持：

```
export personal network state
```

包括：

```
Intent
Artifact metadata
Model profiles
Policies
Project memory
```

敏感 secret 可以单独处理。

---

## 134. API Surface

建议至少提供：

```
Intent API
Artifact API
Model Registry API
Node API
Policy API
Execution API
Event API
```

而不是把所有功能塞进：

```
/chat
```

---

## 135. Streaming Protocol

实时执行需要支持：

```
WebSocket / event stream
```

事件例如：

```
intent.created
node.started
artifact.created
decision.required
verification.failed
task.completed
```

前端只是事件的一个消费者。

---

## 136. Multiple Frontends

统一 Runtime 可以提供：

```
Chat UI
CLI
IDE
Mobile
Web
API
Automation
```

所有前端访问的是同一个 Intent / Artifact 状态。

---

## 137. Mobile Client

手机不应该只是聊天客户端。

它可以同时是：

```
UI
Personal Node
Authentication Device
Standby Device
Light Compute Node
```

但这些角色应独立授权。

---

## 138. Local-First Control Plane

即使大量计算使用云服务，个人网络的：

```
Intent
Authority
Permission
Task Contract
Policy
```

最好由用户控制的 Authority Domain 管理。

云模型属于执行资源，不应成为整个 Runtime 的控制平面。

---

## 139. Control Plane 与 Data Plane

系统明确区分：

```
Control Plane
Intent
Policy
Authority
Scheduling
Capability

Data Plane
Model execution
Tools
Artifacts
Compute nodes
```

这样云节点失效不会导致整个网络失去控制。

---

## 140. MVP 技术边界

第一版绝对不能一次实现全部功能。

真正的最小闭环应该证明：

```
一句 Intent
 ↓
自动形成 Contract
 ↓
自动拆图
 ↓
多模型自动路由
 ↓
worktree 隔离执行
 ↓
Verifier
 ↓
Artifact
 ↓
返回结果
```

如果这个闭环不成立：

```
分布式
视频
3D
长期自治
```

都没有继续扩张的价值。

---

## 141. MVP 推荐场景

第一批重点场景：

```
代码 Bug 修复
代码审查
Repository 研究
文档生成
Web research
```

因为这些任务：

```
输入明确
验证手段强
Artifact 容易表示
适合测试 Agent 编排
```

---

## 142. 第二验证场景

系统稳定后增加：

```
长任务
后台扫描
自动 benchmark
跨模型 Router
```

验证：

```
长时间自治
模型自动升级/降级
恢复
成本控制
```

---

## 143. 第三验证场景

随后进入：

```
视频
图像
音频
3D
```

验证统一 ADFIR 是否真的可以跨领域，而不需要重新造 Runtime。

---

## 144. 第四验证场景

最后增加：

```
Personal Compute Network
Temporary Compute
Distributed Scheduling
Authority Failover
```

因为这一层的工程复杂度和安全要求最高。

---

## 145. 成功判据

这个项目最终成功与否，不应以：

```
支持多少模型
支持多少 Agent
支持多少插件
```

衡量。

真正核心指标应该是：

```
用户只给高层目标后，
系统能够在不违反要求的情况下，
独立完成任务的比例。
```

以及：

```
需要用户干预的次数是否持续下降
```

但这里不能通过：

```
擅自替用户做决定
```

降低干预次数。

---

## 146. 最终产品哲学

系统不是：

> «让 AI 拥有越来越大的自由。»

而是：

> «在用户明确目标、要求和权限边界内，让 Runtime 获得尽可能大的执行自由。»

因此：

```
high autonomy
+
hard boundaries
```

必须同时存在。

缺少前者：

```
系统变成传统聊天机器人。
```

缺少后者：

```
系统变成不可控 Agent。
```

---

## 147. 核心架构总图

```
                         USER
                           │
                           ▼
                  Interaction Layer
                           │
                           ▼
                    Intent Compiler
                           │
                           ▼
                 Versioned Contract
                           │
                           ▼
                 Session Orchestrator
                           │
                           ▼
                        ADFIR
                           │
          ┌────────────────┼────────────────┐
          │                │                │
          ▼                ▼                ▼
       Planner         Model Router      Tool Router
          │                │                │
          │          Model Registry         │
          │                │                │
          └──────────┬─────┴───────┬────────┘
                     │             │
                     ▼             ▼
               Graph Scheduler  Capability Runtime
                     │
                     ▼
                Compute Fabric
             ┌───────┴────────┐
             ▼                ▼
       Personal Nodes   Temporary Nodes
             │                │
             └───────┬────────┘
                     ▼
                  Execution
                     │
                     ▼
                  Artifact
                     │
                     ▼
                  Evidence
                     │
                     ▼
                Verifier Gate
                     │
             ┌───────┴────────┐
             ▼                ▼
           PASS             FAIL
             │                ▼
             ▼             Replan
          Commit
             │
             ▼
          Feedback
             │
             ▼
       Skill / Model Registry
```

外围：

```
Authority Host
Policy Engine
Resource Arbiter
Artifact Store
Background Supervisor
Interaction Broker
Audit Log
Effect Journal
```

---

## 148. 一句话定义

最终可以把整个项目定义成：

> «一个以用户 Intent 为最高语义、以版本化 Task Contract 约束行为、以统一动态数据流图执行任务，并自动在模型、Agent、工具和分布式计算节点之间进行编排，同时通过 capability 安全边界、Artifact provenance 与独立验证保证执行可靠性的个人智能 Runtime。»

其理想使用方式最终仍然只有一句话：

```
"把这件事做完。"
```

剩余复杂度全部属于 Runtime。
