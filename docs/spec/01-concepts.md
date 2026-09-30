# 通用智能任务执行系统计划书

版本：v0.1
状态：总体架构草案
定位：以自然语言 Intent 为入口、以统一动态执行图为核心的个人 AI Runtime

---

## 1. 项目概述

本项目目标不是开发另一个传统意义上的聊天机器人，也不是简单复刻 ChatGPT、Claude Code 或现有 Agent Framework。

系统的基本抽象从：

```
Message → Response
```

转变为：

```
Intent → Autonomous Execution
```

用户描述自己想完成什么，以及不能违反的条件；系统负责自动完成执行过程中的模型选择、Agent 编排、工具调用、资源调度、验证、失败恢复和结果交付。

核心原则：

```
User specifies WHAT
Runtime decides HOW
```

用户主要负责：

```
目标
要求
约束
权限
必要决策
```

Runtime 自动负责：

```
任务拆解
模型选择
推理强度选择
Agent 创建
工具调用
模态选择
执行顺序
并行度
设备选择
资源调度
重试
重新规划
结果验证
Artifact 管理
后台维护
```

系统最终应表现为一个持续存在的个人智能计算环境，而不是一次请求对应一次回答的聊天系统。

---

## 2. 产品体验目标

最基本的交互仍然保持 ChatGPT 式的简单体验。

用户可以只输入：

```
把 Core 最近 parser 的问题彻底查清并修复，
不要修改我的当前工作区。
```

系统内部可以执行：

```
理解需求
↓
生成 Task Contract
↓
创建 Git worktree
↓
分析最近提交
↓
派生多个 Agent
↓
运行测试
↓
执行 fuzz
↓
定位根因
↓
修改代码
↓
验证
↓
独立 Verifier
↓
生成 commit
↓
返回结果
```

但默认 UI 不展示全部过程。

默认状态：

```
正在处理…
```

用户展开后才看到：

```
分析代码
创建隔离工作区
运行测试
启动 3 个并行分析任务
发现候选根因
修改 4 个文件
验证修改
```

最终主输出只展示真正与用户有关的结果：

```
修复完成。

根因：
……

修改：
……

验证：
……

隔离分支：
……
```

设计原则：

> «内部复杂度不能直接转化为用户界面复杂度。»

---

## 3. 总体架构

```
                         User
                           │
                           ▼
                 Interaction Layer
                           │
                           ▼
                  Intent Compiler
                           │
                           ▼
                   Task Contract
                           │
                           ▼
                Session Orchestrator
                           │
                           ▼
                        ADFIR
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
        ▼                  ▼                  ▼
   Model Router       Tool Runtime      Compute Fabric
        │                  │                  │
        ▼                  ▼                  ▼
 Model Registry       Capabilities        Nodes
        │                                     │
        └──────────────┬──────────────────────┘
                       ▼
                    Execute
                       │
                       ▼
                    Evidence
                       │
                       ▼
                  Verifier Gate
                       │
                       ▼
                    Artifact
                       │
                       ▼
                Feedback / Learning
```

外围长期服务：

```
Session Orchestrator
Background Supervisor
Resource Arbiter
Model Intelligence Registry
Compute Fabric
Artifact Store
Permission Runtime
Interaction Broker
```

---

## 4. Intent

Intent 是整个系统的最高层任务对象。

例如：

```
修复这个 bug

剪一个 3 分钟视频

研究某个问题并生成报告

把这组照片重建成 3D 场景
```

Intent 不是 prompt。

它是一个可以持续存在、暂停、恢复、分解和重新规划的对象。

示例：

```
Intent {
    goal
    status
    priority

    contract
    artifacts
    child_intents

    execution_graph

    created_at
    updated_at
}
```

Intent 可以处于：

```
Proposed
Active
Suspended
Waiting
Completed
Failed
Cancelled
```

---

## 5. Task Contract

任何任务执行之前，都必须先将自然语言需求编译成结构化 Task Contract。

约束分四类：

```
REQUIRED
必须满足。
系统不得自行违反。

PREFERRED
尽量满足。
存在充分理由时允许调整。

FLEXIBLE
明确允许 Runtime 自动调整。

UNSPECIFIED
用户没有要求，由 Runtime 自由决定。
```

例如：

```
用户：
生成一个 10 秒 4K 视频。

得到：

duration = 10s       REQUIRED
resolution >= 4K     REQUIRED
codec                UNSPECIFIED
fps                  UNSPECIFIED
```

如果 GPU 无法生成 4K：

系统可以自动：

```
换模型
换节点
改变执行 pipeline
分块
延长执行时间
```

因为这些没有改变用户需求。

但不能：

```
4K → 1080p
```

因为这违反 REQUIRED。

此时必须进入：

```
Decision Required
```

让用户选择。

---

## 6. Constraint Violation Gate

任何执行计划进入运行之前，都经过约束验证：

```
Candidate Plan
      ↓
Constraint Validator
      │
      ├── 满足 Contract
      │       ↓
      │     Execute
      │
      └── 违反 REQUIRED
              ↓
          Suspend
              ↓
        Ask User Decision
```

Runtime 可以自动改变 HOW。

Runtime 不能擅自改变用户已经确定的 WHAT。

这是整个系统必须硬编码执行的原则，而不能依赖模型自觉。

---

## 7. Autonomous Exploration Budget

在发现当前方案不可行时，Runtime 可以寻找替代路径。

但寻找路径本身可能很昂贵。

因此定义：

```
Autonomous Exploration Budget
```

默认可以包括：

```
wall_time <= 60s
money_cost = 0
external_side_effect = none
network_transfer <= threshold
compute_cost <= threshold
```

例如：

```
检查另一节点 GPU
→ 2 秒
→ 自动执行

跑一个 30 秒 probe
→ 自动执行

下载 40GB 模型
→ 需要用户确认

测试 30 分钟 alternative pipeline
→ 需要用户确认

租收费 GPU
→ 需要用户确认
```

因此存在两个独立 Gate：

```
Constraint Gate

Exploration Cost Gate
```

---

## 8. ADFIR

系统只维护一种通用执行 IR：

```
ADFIR — Adaptive Dynamic Flow Intermediate Representation
```

所有任务最终表示成动态执行图。

不为代码、视频、3D、研究分别发明独立 Runtime。

统一表示：

```
Operator
Artifact
Port
Edge
Activation
Constraint
Effect
Evidence
Verifier
```

示例：

```
Video
 ↓
ShotDetect
 ↓
VisionAnalysis
 ↓
NarrativePlan
 ↓
Timeline
 ↓
Render
 ↓
Verify
```

代码：

```
Repository
 ↓
Analyze
 ↓
ReproduceBug
 ↓
Patch
 ↓
Compile
 ↓
Test
 ↓
Verify
```

3D：

```
Images
 ↓
CameraPose
 ↓
Reconstruction
 ↓
Scene
 ↓
Render
 ↓
Verify
```

都属于同一个 ADFIR。

---

## 9. Typed Artifact

领域差异主要通过 Artifact 类型表达。

例如：

```
Artifact<Text>
Artifact<Image>
Artifact<Audio>
Artifact<Video>
Artifact<Timeline>
Artifact<Mesh>
Artifact<Scene>
Artifact<SourceTree>
Artifact<Patch>
Artifact<TestResult>
Artifact<Transcript>
Artifact<Report>
```

Artifact 必须包含：

```
content
type
hash
version
producer
dependencies
provenance
created_at
```

复杂任务不能只依赖聊天记录保存状态。

Artifact 是正式执行结果。

---

## 10. Domain Operator

不同领域通过 Operator 扩展：

```
ShotDetect(Video) -> ShotSet

Transcribe(Audio) -> Transcript

ComposeTimeline(...) -> Timeline

Reconstruct3D(ImageSet) -> Scene

RenderScene(Scene) -> Video

ApplyPatch(SourceTree, Patch) -> SourceTree
```

需要调用具体后端时，再进行 lowering：

```
ADFIR
 ↓
FFmpeg

ADFIR
 ↓
Blender

ADFIR
 ↓
Git

ADFIR
 ↓
GPU backend
```

---

## 11. Session Orchestrator

用户每次打开软件，形成一次 Session Activation。

系统提供两种模式。

**Manual**

Runtime 自动给用户生成候选：

```
继续 Core
继续 LCM
查看昨晚发现的问题
开始新对话
```

用户选择。

**Auto**

启动专门的 Planner Agent：

```
App Open
 ↓
Session Orchestrator
 ↓
分析未完成 Intent
 ↓
分析后台新结果
 ↓
选择当前最值得继续的任务
 ↓
创建 Session Plan
```

例如：

```
昨晚 Core parser 的测试结果已经出来。
当前最值得继续处理这个问题。
```

然后直接恢复。

---

## 12. Background Supervisor

软件关闭后，Runtime 可以继续执行授权过的后台任务。

典型后台任务：

```
Bug scanning
CI verification
static analysis
fuzz
dependency checking
documentation drift
benchmark
project maintenance
artifact preprocessing
media indexing
```

例如：

```
Repository changed
 ↓
Background Supervisor
 ↓
affected modules
 ↓
spawn scanner
 ↓
发现 regression
 ↓
生成 Intent
```

第二天：

```
昨晚发现两个问题。
```

后台自治针对的是项目与任务世界，而不是持续监测用户行为。

---

## 13. Resource Arbiter

后台任务不能影响用户正常使用机器。

Resource Arbiter 独立于 AI 模型运行。

默认只读取聚合资源状态：

```
CPU
RAM
GPU
VRAM
Disk IO
Network
```

当资源明显竞争时，才最小化读取高占用进程。

允许读取：

```
process name
CPU usage
GPU usage
PID
```

默认不读取：

```
command line
working directory
window title
screen
open files
keyboard input
```

进程名只在本机 Resource Arbiter 中分类：

```
INTERACTIVE_GAME
COMPUTE_WORKLOAD
COMPILER
VIDEO_RENDER
```

交给 AI 的仅是：

```
resource_pressure = HIGH
workload_class = INTERACTIVE
```

避免把用户行为直接送进模型。

---

## 14. Cooperative Suspension

后台任务不能粗暴 kill。

状态：

```
Running
 ↓
Throttled
 ↓
Checkpointing
 ↓
Suspended
 ↓
Resume
```

支持 hysteresis：

```
suspend threshold
resume threshold
cooldown
```

避免资源利用率在临界点反复启停。

---

## 15. 权限等级

执行权限按照影响域，而不是具体命令划分。

建议：

```
L0
只读。

L1
计算：
build
test
fuzz
benchmark
analysis

L2
可写隔离工作空间。

L3
影响用户正式环境。

L4
产生外部副作用。
```

---

## 16. Git L2 Sandbox

对于代码任务，L2 默认使用：

```
git worktree
```

每个 Intent 创建独立：

```
ai/task-id
```

例如：

```
repo/
repo/.ai-worktrees/fix-parser/
```

Agent 可以：

```
修改
commit
reset
测试
重构
```

但完全不碰用户当前 branch。

因此：

```
L2
= 可以自由修改自己的隔离世界
```

真正跨越：

```
AI worktree
→ user branch
```

才属于 L3。

---

## 17. Model Capability Tier

模型能力分三档。

**Tier 1 — Free / Light**

用途：

```
简单分类
提取
总结
格式转换
简单工具调用
机械性代码操作
```

特点：

```
低成本
低延迟
不适合复杂长链任务
```

**Tier 2 — Normal**

主力模型：

```
复杂推理
代码开发
研究
长上下文
Agent planning
视觉理解
复杂工具调用
```

**Tier 3 — Special**

特殊原生能力：

```
Video generation
3D generation
3D reconstruction
specialized audio
advanced spatial model
specialized scientific model
```

Tier 3 不代表"更聪明"。

而代表：

```
specialized capability
```

---

## 18. Reasoning Effort

Capability Tier 与 reasoning effort 分离。

例如：

```
Tier 1 + High
Tier 2 + Low
Tier 2 + High
Tier 3 + specialized execution
```

大型机械任务：

```
Tier 1
low reasoning
large parallelism
```

复杂核心推理：

```
Tier 2
high reasoning
```

Runtime 自动决定。

---

## 19. Model Family

用户可以设置：

```
Auto
OpenAI preferred
Claude preferred
Local preferred
Custom
```

默认优先使用同一模型 family。

只有明显收益足够高时才跨 family。

Router 考虑：

```
quality
cost
latency
context transfer
family switching
failure risk
```

---

## 20. Model Router

每个 ADFIR 节点独立生成 Execution Profile：

```
ExecutionProfile {
    family
    model
    tier
    reasoning_effort

    modalities
    tools

    node
    parallelism

    context_budget
    token_budget

    cost_target
    latency_target
}
```

模型选择不是会话级，而是节点级。

---

## 21. Model Intelligence Registry

新增模型不能直接进入自动 Router。

生命周期：

```
DISCOVERED
 ↓
UNPROFILED
 ↓
RESEARCHED
 ↓
PROBED
 ↓
VERIFIED
 ↓
ACTIVE
```

异常状态：

```
STALE
DEGRADED
QUARANTINED
DISABLED
```

---

## 22. Model Onboarding

新增模型后自动创建：

```
Model Onboarding Task
```

流程：

```
读取官方文档
搜索 model card
收集 benchmark
收集公开 failure mode
 ↓
生成 Preliminary Profile
 ↓
执行 Active Probe
 ↓
Verifier
 ↓
生成正式 Model Profile
```

Profile 记录：

```
modalities
context
tool use
reasoning
coding
vision
audio
spatial
instruction following
constraint adherence
latency
cost
failure modes
known quirks
```

---

## 23. Skill Graph

模型不维护单一总评分，而是能力向量。

例如：

```
Reasoning
├ deduction
├ causal reasoning
├ planning
└ constraint reasoning

Coding
├ understanding
├ bug localization
├ fixing
├ architecture
├ testing
└ formal reasoning

Agent
├ tool use
├ decomposition
├ recovery
└ long-task completion

Vision
├ understanding
├ grounding
├ spatial
└ diagram
```

Benchmark 自动映射到 Skill Graph。

---

## 24. Skill Scoring

模型 Skill Score 来源：

```
Standard Benchmark
Active Probe
Population Feedback
Personal Feedback
Verifier Outcomes
Real Task Results
```

每个值同时包含：

```
score
confidence
sample_count
version
time_range
```

评分是时间序列，不是常数。

---

## 25. Population Feedback

大量用户实际任务结果参与模型评分。

不能简单平均。

需要考虑：

```
任务难度
模型版本
任务类型
用户是否真实使用
Verifier 是否能客观验证
异常评分
刷分行为
```

高质量证据例如：

```
大型项目 patch
全部测试通过
用户接受
长期未回滚
```

权重大于：

```
简单 hello world
5 星
```

---

## 26. Personal Model Profile

对于当前用户：

```
PersonalScore
```

随着实际使用积累逐渐提高权重。

冷启动：

```
Benchmark
Population
```

长期使用后：

```
Personal evidence
```

占更高权重。

因此系统最终优化的是：

> «这个模型对这个用户、这种任务到底是否好用。»

---

## 27. Router Exploration

Router 不能永远使用当前最高分模型。

否则新模型永远没有真实反馈。

低风险任务允许：

```
small exploration probability
```

高风险任务：

```
exploration ≈ 0
```

可以使用 contextual bandit 类策略实现。

---

## 28. Verifier Gate

关键节点默认独立验证。

流程：

```
Worker
 ↓
Artifact
 ↓
Evidence
 ↓
Verifier
 ↓
PASS / FAIL
```

优先级：

```
1. Deterministic verifier
2. Independent model verifier
3. Cross-family verifier
4. Multiple verifier
5. Human decision
```

---

## 29. 避免模型"耍小聪明"

Verifier 不默认接受 Worker 的解释。

优先输入：

```
Original Task Contract
Artifact
Evidence
```

先进行 blind verification。

之后才比较 Worker reasoning summary。

例如：

用户要求 4K

Verifier 实际读取：

```
resolution metadata
```

发现：

```
1920×1080
```

直接失败。

即使 Worker 声称：

```
任务完成
```

也没有意义。

---

## 30. 完成条件

模型不能自己认为：

```
差不多完成了
```

每个 Intent 必须拥有：

```
Completion Predicate
```

例如：

```
tests_pass
AND
artifact_exists
AND
contract_satisfied
AND
verification_passed
```

只有满足才进入：

```
Completed
```

---

## 31. 多模态自动编排

如果主模型为纯文本：

```
Text LLM
```

Runtime 可以自动增加：

```
Image Model
Vision Model
Audio Model
Video Model
3D Model
```

如果是多模态模型，则可以允许：

```
Spatial Tool
pySpatial 类 backend
```

主模型不需要拥有所有能力。

---

## 32. 全自动视频剪辑

视频剪辑视为高复杂度 Intent。

流程：

```
素材导入
 ↓
镜头切分
 ↓
ASR
 ↓
Audio Analysis
 ↓
Vision Analysis
 ↓
Narrative Planning
 ↓
Timeline Artifact
 ↓
Effects / Subtitle
 ↓
Render
 ↓
Deterministic Verify
 ↓
Multimodal Verify
```

Timeline 是：

```
Artifact<Timeline>
```

而不是单独 IR。

---

## 33. 视频后台预处理

素材加入以后可以在后台自动：

```
建立 proxy
shot detection
ASR
face/object indexing
visual embedding
beat map
metadata analysis
```

真正开始剪辑时减少等待时间。

---

## 34. Generative Permission

剪辑与生成必须分开授权。

Task Contract 可以定义：

```
allow_generated_broll
allow_generated_voice
allow_generated_music
allow_frame_interpolation
allow_3d_generation
```

系统不得擅自使用生成内容替换原始素材。

---

## 35. Compute Fabric

任何计算设备都可以成为 Runtime 的计算节点。

节点大体分为：

```
Personal Node
Temporary Node
```

---

## 36. Personal Node

个人节点是长期个人计算网络成员。

例如：

```
Phone
Laptop
Desktop
Home Server
NAS
```

可以拥有：

```
compute
storage
background task
artifact cache
models
services
```

但：

> «设备加入网络不等于获得信任。»

---

## 37. Personal Device Join

新设备状态：

```
DISCOVERED
 ↓
PAIRED
 ↓
PENDING_TRUST
 ↓
TRUSTED_PERSONAL
```

只有中心 Authority Host 可以批准。

完整流程：

```
New Device
 ↓
发送 Join Request
 ↓
Authority Host
 ↓
用户在 Authority Host 明确确认
 ↓
New Device
 ↓
输入 TOTP
 ↓
绑定 Device Public Key
 ↓
Trusted Personal Node
```

其他 Personal Node 无权批准新设备。

---

## 38. Authority Host

个人网络必须存在唯一：

```
Authority Host
```

它是：

```
Root of Operational Trust
```

拥有：

```
设备加入审批
设备撤销
权限授权
节点身份管理
TOTP secret management
trust registry
```

普通 Personal Node 不能扩大 Trust Domain。

---

## 39. Device Trust 与 Capability 分离

即使设备成为 Trusted Personal Node，也不意味着获得所有权限。

例如：

```
Trusted Desktop

✓ compute
✓ background tasks
✓ artifact storage

□ personal memory
□ mail credential
□ payment credential
```

因此：

```
Device Trusted
≠
Unlimited Agent Capability
```

---

## 40. Temporary Node

Temporary Node 包括：

```
朋友电脑
实验室机器
租用云 GPU
短时服务器
```

特点：

```
低权限
租约制
计算专用
无长期个人状态
无 Personal Memory
无全局 credential
无网络横向发现
```

执行流程：

```
ADFIR subgraph
 ↓
dependency closure
 ↓
Job Capsule
 ↓
Temporary Node
 ↓
Execute
 ↓
Artifact
 ↓
Verifier
 ↓
destroy capsule
```

---

## 41. Job Capsule

Temporary Node 只得到任务真正需要的数据。

例如视频生成：

```
prompt
reference image
model
generation config
```

不获得：

```
GitHub token
聊天记录
个人 Memory
其他项目
SSH key
```

---

## 42. Distributed Placement

节点选择考虑：

```
compute capability
VRAM
memory
latency
network bandwidth
money cost
privacy policy
trust domain
resource pressure
```

例如：

```
Private photo:
LOCAL_ONLY
```

则 Cloud Node 即使性能最好，也不能被选择。

---

## 43. Standby Temporary Authority

个人网络预先指定第二台设备：

```
Standby Device
```

平时：

```
无 Authority 权限
同步加密状态
维护网络 registry 副本
```

当 Primary Authority Host 不可达：

```
Primary heartbeat lost
 ↓
failover eligible
 ↓
用户在 Standby 输入 TOTP
 ↓
Temporary Authority
```

---

## 44. Temporary Authority 权限

Temporary Authority 的主要作用：

```
维持网络运行
```

默认允许：

```
继续任务
调度现有节点
管理 Artifact
调用已有授权
```

是否允许修改 Trust Graph 可以设计成更严格模式。

推荐默认：

```
不可添加永久 Personal Node
不可永久改变 Trust Root
```

防止 split-brain。

---

## 45. Permanent Promotion

确认 Primary 永久丢失后：

```
Temporary Authority
 ↓
Permanent Promotion
 ↓
authority_epoch++
 ↓
New Authority Host
```

旧 Authority 再上线：

```
STALE_AUTHORITY
```

不再拥有 Root 权限。

---

## 46. Standby 数据加密

不要使用 TOTP 加密数据。

数据使用：

```
random 256-bit DEK
```

推荐：

```
XChaCha20-Poly1305
```

DEK 使用 Standby Device Public Key 封装。

例如：

```
HPKE / X25519
```

结构：

```
Network State
 ↓
DEK
 ↓
XChaCha20-Poly1305
 ↓
Encrypted Snapshot

DEK
 ↓
Standby Public Key
 ↓
Wrapped DEK
```

---

## 47. TOTP 角色

TOTP 只作为：

```
Human Failover Authorization
```

而不是：

```
encryption key
```

Failover：

```
Primary unreachable
+
TOTP valid
+
Standby Device Key
=
Temporary Authority Activation
```

---

## 48. Interaction Broker

AI 后台运行和通知用户必须分开。

后台可以：

```
24h scan
test
research
generate result
```

但 Interaction Broker 决定什么时候展示。

例如：

```
critical
→ immediate

high
→ next app open

normal
→ inbox

low
→ history only
```

AI 不需要知道用户具体在干什么。

---

## 49. 隐私原则

系统默认只观察：

```
任务状态
项目状态
已授权服务
资源状态
自己产生的 Intent
```

不默认观察：

```
screen
window title
keyboard
browser history
microphone
other app content
```

原则：

> «能使用更低隐私级别的信息完成任务，就禁止请求更高隐私级别信息。»

---

## 50. Prompt Injection 防护

来自：

```
网页
邮件
Repository
文档
Issue
工具输出
```

的内容默认属于：

```
UNTRUSTED DATA
```

而不是 instruction。

Runtime 必须硬区分：

```
Instruction
Data
Capability
Effect
```

不能依赖 LLM 自己判断：

```
这段文字是不是恶意提示词
```

---

## 51. Secret Runtime

Agent 不直接获得所有密钥。

每个节点只获得：

```
minimum capability
short-lived credential
scoped token
```

例如：

```
read repository
```

不意味着：

```
GitHub account full access
```

---

## 52. Effect Journal

所有无法通过 Artifact 回滚的外部操作必须进入：

```
Effect Journal
```

例如：

```
send_email
delete_remote_file
publish
charge
deploy
```

执行之前记录：

```
intent
idempotency key
authorization
status
```

避免重试重复产生副作用。

---

## 53. Runtime Persistence

ADFIR 和 Intent 状态不能只存在 Agent memory。

必须持久化：

```
Intent state
Graph state
Artifacts
Effect journal
Model profile
Permission grants
```

Runtime 崩溃后：

```
restart
 ↓
restore
 ↓
continue
```

而不是重新问模型：

```
你之前做到哪里了？
```

---

## 54. UI

默认视图极简：

```
User
 ↓
正在处理…
 ↓
Result
```

展开后：

```
Plan
Agents
Models
Tools
Artifacts
Changes
Verification
```

模型内部原始 chain-of-thought 不作为用户界面的一部分。

展示的是：

```
Execution Trace
Decision Summary
Evidence
```

---

## 55. Decision Required

只有真正需要用户决定时才打断。

典型条件：

```
违反 REQUIRED
高成本探索
收费计算
高风险副作用
权限升级
Trust change
```

而不是每个 Agent 小问题都问用户。

---

## 56. 第一阶段 MVP

目标：

```
验证核心 Runtime，而不是一次实现所有能力。
```

实现：

```
Intent
Task Contract
ADFIR
Tier 1 / Tier 2 routing
simple Model Registry
Git worktree L2
tool execution
Verifier Gate
Artifact Store
Session UI
```

支持场景：

```
代码分析
Bug 修复
研究
文档任务
```

暂不实现：

```
3D
Video generation
Distributed compute
Background autonomy
Temporary authority
```

---

## 57. 第二阶段

加入：

```
Background Supervisor
Resource Arbiter
Model onboarding
Skill Graph
Active probes
Population / Personal feedback
Model family routing
Automatic reasoning effort
```

重点验证：

```
长期模型路由是否越来越准确
后台自治是否可控
```

---

## 58. 第三阶段

加入：

```
Personal Compute Network
Authority Host
Personal Nodes
Temporary Nodes
Job Capsule
Distributed placement
Encrypted state replication
Standby Authority
```

重点验证：

```
跨设备任务透明迁移
隐私约束
临时节点隔离
资源竞争
```

---

## 59. 第四阶段

加入复杂媒体：

```
image
audio
video
3D
spatial
```

建立：

```
Timeline artifact
Scene artifact
media operators
video verification
spatial operators
```

实现：

```
全自动剪辑
自动生成缺失素材
空间重建
跨模态任务
```

---

## 60. 第五阶段

完善长期自治：

```
Intent Graph
long-running task
background maintenance
model drift detection
adaptive routing
automatic re-profiling
distributed background execution
```

最终形成：

```
Persistent Personal AI Runtime
```

---

## 61. 核心评测指标

系统最终不能只看"回答质量"。

需要至少评估：

```
Task success rate
Contract violation rate
Verifier false-pass rate
Model routing regret
Cost per completed task
User intervention frequency
Unnecessary clarification rate
Background usefulness
Rollback rate
Artifact correctness
Permission violation rate
Recovery success rate
Distributed scheduling efficiency
```

尤其应该关注：

```
用户只给一句高层命令，
最终无需干预完成任务的比例。
```

这是产品真正的核心指标之一。

---

## 62. 研究价值

已有工作分别研究：

```
Agent graph
Agent OS
Model routing
Verifier
Capability security
User preference
Distributed inference
Media Agent
```

本项目真正不同的方向在于将这些统一进：

```
Intent
↓
Versioned Task Contract
↓
Typed Dynamic Execution Graph
↓
Model / Tool / Node / Permission Routing
↓
Evidence
↓
Verification
↓
Artifact
↓
Feedback
↓
Model Intelligence Registry
```

它不是单一 Agent Framework。

也不是单一模型 Router。

也不是 Agent OS 的简单复制。

核心研究对象是：

> «如何构建一个能够在严格约束、权限、安全、资源和模型能力条件下，长期自动完成用户 Intent 的通用智能执行 Runtime。»

---

## 63. 最终系统定义

本项目最终可以定义为：

> «一个以 Intent 为输入、以 Task Contract 为约束、以 ADFIR 为统一执行表示、能够自动选择模型、Agent、工具和分布式计算节点，并通过独立验证与动态反馈持续优化执行策略的个人智能任务执行系统。»

其产品表面可以非常简单：

```
"把这个事情做完。"
```

但 Runtime 内部可以自动完成：

```
理解
规划
拆解
编排
执行
验证
恢复
调度
计算
生成
修改
交付
学习
```

最终目标不是让用户学会如何指挥 AI。

而是：

> «用户只需要准确描述自己想要什么，Runtime 负责解决其余执行复杂度。»
