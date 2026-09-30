# 执行方法学 / Resolver / Review Ladder / 命名

本文件对应原始第四份材料。

---

## 185. Product Drive 的双层循环

Product Drive 不直接等价于一个无限 Agent Loop。

系统应明确分为：

```
Product Loop
↓
Milestone Engineering Loop
```

外层负责：

```
当前产品状态
↓
距离成品还缺什么
↓
生成下一 Milestone
↓
重新评估
```

内层负责：

```
Milestone
↓
规划
↓
拆任务
↓
实现
↓
验证
↓
完成
```

因此"无限驱动"的本质是：

> «Milestone 数量没有预设上限，而不是一次生成一个无限长计划。»

---

## 186. Superpowers 式工程执行

单个 Milestone 的工程执行可以参考 Claude Code / Superpowers 一类工作流的思想：

```
Milestone
↓
Requirement / Design Review
↓
Implementation Plan
↓
Engineering Tasks
↓
Fresh Subagent
↓
Implement
↓
Spec Review
↓
Quality Review
↓
Verification
↓
Next Task
```

其优势是：

```
计划先于实现
任务上下文隔离
逐任务验证
规格审查与代码质量审查分离
```

但该流程只负责：

```
"如何把当前 Milestone 做完"
```

而不负责决定：

```
"产品下一阶段还应该做什么"
```

---

## 187. Execution Method Library

Superpowers 类型的方法不应该成为整个 Runtime 的固定核心。

定义：

```
Execution Method Library
```

用于针对不同领域选择不同的可靠执行方法。

例如：

```
software/
    planning
    TDD
    debugging
    worktree
    review
    fuzz
    verification

video/
    shot-analysis
    narrative-plan
    timeline-compose
    render-review

research/
    retrieval
    evidence-analysis
    contradiction-check
    citation-verification

3d/
    reconstruction
    geometry-validation
    render-comparison
```

ADFIR 决定：

```
做什么
依赖什么
```

Execution Method 决定：

```
这一类任务怎样做得可靠
```

---

## 188. TDD 的定位

TDD 不能被当成正确性证明。

正确关系是：

```
TDD
= Implementation Discipline
```

而不是：

```
TDD
= Correctness Proof
```

即使满足：

```
先写测试
↓
测试失败
↓
实现
↓
测试通过
```

仍然可能存在：

```
需求理解错误
测试断言错误
覆盖范围过窄
Mock 绕过真实路径
遗漏边界条件
测试实现细节而非需求
```

因此：

```
Tests Passed
≠
Requirement Satisfied
```

---

## 189. Test Evidence

测试结果在 Runtime 中只是一种：

```
Evidence<Test>
```

而不是：

```
Proof<Correctness>
```

系统必须区分：

```
Implementation
Testing
Verification
Completion
```

分别表示：

```
Implementation
→ 产生候选结果

Testing
→ 产生证据

Verification
→ 判断证据是否充分

Completion
→ 判断需求是否满足
```

---

## 190. Requirement Coverage

每一项 REQUIRED 都必须有对应证据。

例如：

```
Requirement A
├ unit test
├ property test
└ fuzz

Requirement B
├ integration test
└ independent verifier

Requirement C
└ differential test
```

形成：

```
Requirement Coverage Graph
```

如果：

```
所有测试 PASS
```

但：

```
Requirement C
没有有效证据
```

则状态必须是：

```
INCOMPLETE VERIFICATION
```

---

## 191. Test Validity Verification

测试本身也需要被验证。

增加：

```
Test Verifier
```

检查：

```
测试对应哪条 Requirement
断言是否足够强
错误实现是否可能仍通过
Mock 是否绕过真实路径
边界条件是否缺失
```

测试不能因为存在于 test/ 目录里就自动被认为有效。

---

## 192. Mutation Testing

对高价值代码任务自动进行：

```
Mutation Testing
```

例如故意将：

```
x > 10
```

变成：

```
x >= 10
x < 10
true
false
```

如果测试仍通过：

```
test evidence strength ↓
```

因此可以维护：

```
mutation_score
```

作为测试有效性的证据之一。

---

## 193. Property / Fuzz / Differential Verification

Runtime 应根据任务类型选择额外验证方式。

包括：

```
Property-based Testing
Fuzzing
Differential Testing
Metamorphic Testing
Static Verification
Formal Proof
```

例如编译器：

```
Interpreter(program)
vs
Compiled(program)
```

应该具有一致 observable behavior。

Parser：

```
parse(serialize(x))
≈ x
```

这些性质比单个手写 case 更难被实现"针对测试"。

---

## 194. Verification Profile

不同任务自动生成：

```
VerificationProfile
```

例如：

```
简单文案修改
→ basic verifier

parser 修改
→ unit
 + integration
 + property
 + fuzz
 + mutation

大型重构
→ unit
 + integration
 + E2E
 + compatibility
 + benchmark
 + differential

安全关键代码
→ deterministic vectors
 + static analysis
 + independent review
 + formal verification
```

避免所有任务都承担相同验证成本。

---

## 195. Completion Predicate 强化

任务完成不再仅仅依赖：

```
tests_pass
```

而是：

```
artifact_exists

AND requirement_coverage_sufficient

AND mandatory_tests_pass

AND applicable_properties_hold

AND applicable_fuzz_budget_complete

AND applicable_mutation_quality_sufficient

AND independent_verification_pass

AND task_contract_satisfied
```

核心原则：

> «测试通过只是完成证据的一部分。»

---

## 196. Input Canonicalization Frontend

用户输入不能直接进入 Intent Compiler。

增加：

```
Input Canonicalization Frontend
```

负责处理：

```
错别字
ASR 错误
音译
简称
中英混写
昵称
工具别名
Skill 名称
代词
上下文引用
```

流程：

```
Raw Input
↓
Mention Detection
↓
Candidate Generation
↓
Context Resolution
↓
Canonical Representation
↓
Intent Compiler
```

---

## 197. Noisy Surface Restoration

例如：

```
"克拉的code"
```

可能实际表示：

```
Claude Code
```

该过程不能完全交给后面的主模型猜测。

候选生成可以利用：

```
edit distance
phonetic similarity
ASR hypotheses
pinyin / phoneme
embedding retrieval
alias registry
entity registry
```

这些组件负责：

```
高召回候选生成
```

而不是直接决定最终语义。

---

## 198. Joint Reference Resolution

多个模糊引用应该联合解析。

例如：

```
"克拉的code里面那个ss技能"
```

不能独立解析：

```
"克拉的code" → ?
"ss技能" → ?
```

而应建立：

```
ToolCandidate
SkillCandidate
Relation
```

例如：

```
Skill belongs_to Tool
```

然后进行联合约束。

最终解决的是：

```
最一致的 Entity Graph
```

而不是单个字符串匹配。

---

## 199. Resolution Scope

实体解析必须拥有 namespace / scope。

例如：

```
Global
User
Project
Workspace
Tool
Plugin
Conversation
Session
```

解析：

```
"ss"
```

如果当前 scope 明确是：

```
Claude Code
```

应优先执行：

```
resolve("ss", scope=ClaudeCode)
```

而不是全局搜索所有 "ss"。

---

## 200. Open-World Resolution

Resolver 必须承认：

```
正确答案可能不在 Registry 中
```

因此 Candidate Set 永远允许：

```
NIL
UNKNOWN
OUT_OF_REGISTRY
```

禁止：

```
没有正确候选
↓
强行选择最相似候选
```

否则系统会产生稳定的错误确定性。

---

## 201. Constraint-Based Resolution

Resolver 的核心不应是一个永久维护的神秘评分公式。

更稳定的核心语义是：

```
Candidate Set
↓
Explicit Constraints
↓
Registry Relations
↓
Scope
↓
Conversation Relations
↓
Constraint Propagation
```

最终：

```
1 candidate
→ Exact

>1 candidates
→ Ambiguous

0 candidate
→ NIL / Expand Retrieval
```

概率和相似度主要用于：

```
candidate retrieval / ranking
```

而不是系统语义本身。

---

## 202. Resolution Confidence Gate

尽管系统核心采用候选集合与约束，仍然可以保留经过校准的：

```
Resolution Confidence
```

但它的作用不是：

```
决定哪个解释是真的
```

而是：

> «Runtime 是否有资格在不询问用户的情况下按照该解释执行。»

流程：

```
Candidate Resolution
↓
Resolution Confidence
↓
Execution Gate
```

---

## 203. Risk-Adaptive Resolution

自动解析要求应随着后续动作风险提高。

概念上：

```
RequiredResolutionConfidence
=
f(
    effect_level,
    reversibility,
    cost,
    privacy,
    trust_change
)
```

例如：

```
普通解释
→ 较低门槛

只读研究
→ 中等门槛

隔离工作区修改
→ 更高门槛

正式环境修改
→ 极高门槛

发布 / 删除 / Trust Change
→ 显式确认
```

因此：

```
语义确定
```

与：

```
允许产生副作用
```

仍是两个不同 Gate。

---

## 204. Manual Candidate Selection

如果 Resolver 没达到自动执行要求：

Runtime 不继续猜

而显示候选：

```
你说的"ss技能"是：

○ Candidate A
○ Candidate B
○ Candidate C
○ 都不是
```

用户选择以后形成：

```
ExplicitBinding
```

例如：

```
mention = "ss"
entity = X
source = USER_SELECTION
```

该 Binding 在当前任务中拥有最高解析权重。

---

## 205. Alias Lifetime

用户选择产生的 alias 默认不能无限持久化。

分：

```
Turn Alias
Session Alias
Project Alias
Persistent User Alias
```

例如：

```
当前会话：
"ss" = Skill X
```

只产生：

```
SessionAlias
```

只有用户明确表示：

```
"以后我说 ss 都是这个"
```

才提升成：

```
PersistentUserAlias
```

防止 Resolver 对历史上下文长期过拟合。

---

## 206. Resolver 四态模型

Resolver 最终统一输出：

```
RESOLVED
CONFIRM_REQUIRED
AMBIGUOUS
UNKNOWN
```

其中：

```
RESOLVED
→ 允许自动绑定

CONFIRM_REQUIRED
→ 第一候选明显，但不足以安全执行

AMBIGUOUS
→ 多个候选仍然成立

UNKNOWN
→ 当前没有可靠解析
```

系统必须支持：

```
abstain
```

而不是要求 Resolver 每次必须给答案。

---

## 207. Strategic Plan Review Ladder

重大任务在进入 ADFIR 前必须经过：

```
Draft Plan
↓
Self Review
↓
Independent Strong AI Review
↓
Revision
↓
User Review
↓
Approved Strategic Plan
```

三个 Review 的职责不同。

---

## 208. Self Review

Planner 自审主要检查：

```
前后依赖
遗漏步骤
重复工作
不可能步骤
Task Contract 冲突
Milestone 顺序
Acceptance Criteria
资源假设
```

它可以自动修正内部一致性问题。

---

## 209. Independent AI Review

第二层由独立且能力更强的 Reviewer 执行。

输入：

```
Original Intent
Task Contract
Engineering Baseline
Draft Plan
```

重点查：

```
Planner 漏掉什么
是否偷偷降低要求
架构是否工程可行
验证是否不足
是否存在必然返工
是否有更简单方案
Milestone 是否真的向成品推进
```

输出：

```
BLOCKER
MAJOR
MINOR
SUGGESTION
```

Planner 根据结果重新生成计划。

---

## 210. User Plan Review

用户审查主要面向战略层，而不是每个执行步骤。

用户应看到：

```
目标理解
硬约束
重大架构选择
Milestone
排除项
主要风险
最终完成条件
成本 / 权限影响
```

而不需要审批：

```
具体函数修改顺序
模型选择
并发度
工具内部步骤
```

---

## 211. Strategic Plan 与 Execution Plan 分离

定义：

```
Strategic Plan
→ 用户审核

Execution Plan
→ AI 自审 + Independent AI Review
```

因此：

```
用户决定：
"我们准备做什么"

Runtime 决定：
"如何把它做出来"
```

避免高自治系统变成不断询问用户的低效率系统。

---

## 212. Material Plan Change

计划执行过程中允许动态重规划。

但必须区分：

```
Minor Replan
Material Change
```

Minor：

```
换模型
换工具
改变执行顺序
并发调整
普通重试
```

可以自动处理。

Material：

```
改变 REQUIRED
修改重大架构
扩大权限范围
增加外部副作用
显著增加成本
改变 Milestone 目标
修改用户明确否决的设计
```

必须重新进入相应 Review Gate。

---

## 213. Plan Versioning

计划本身必须版本化：

```
Plan v1
APPROVED
```

普通执行调整：

```
Plan v1.1
Plan v1.2
```

可以自动。

重大变化：

```
Plan v2
REVIEW_REQUIRED
```

最高原则：

> «Planner 可以自由优化如何完成已批准计划，但不能自行改变用户批准了什么。»

---

## 214. Specification Loop

Intent 不能直接假设已经完整。

增加：

```
Specification Loop
```

把自然语言逐渐转成：

```
Goal
Invariant
Acceptance Predicate
Forbidden State
Preference
Unknown
```

流程：

```
User Intent
↓
Specification Mining
↓
发现隐含要求
↓
检测冲突 / 未定义行为
↓
形成 Executable Specification
```

只有关键歧义才询问用户。

---

## 215. Verification Design Loop

不仅实现需要验证，验证机制本身也需要主动攻击。

增加：

```
Verification Strategy Generator
↓
Verifier
↓
Verifier Adversary
```

Verifier Adversary 尝试寻找：

```
明显错误
但现有 verifier 仍然 PASS
```

的反例。

如果能找到：

```
verification strategy insufficient
```

必须增强验证体系。

---

## 216. Runtime Learning Loop

Runtime 应记录长期执行结果：

```
哪些计划经常返工
哪些模型在哪些 Skill 上失败
哪些验证方式经常漏错
哪些 Execution Method 更可靠
哪些 Context 选择导致噪声
```

然后优化：

```
Planning Policy
Routing Policy
Execution Method
Verification Strategy
Context Selection
Scheduling Heuristic
```

形成：

```
执行任务
↓
收集 Evidence
↓
学习任务执行方式
↓
改善未来任务
```

---

## 217. Self-Improving Runtime Boundary

Runtime 本身也可以成为 Product Drive 的对象。

例如发现：

```
Planner 经常漏 dependency
Resolver 经常误解析
Verifier false-pass 偏高
Model Router 经常选错
```

系统可以创建：

```
Runtime Improvement Intent
```

然后：

```
隔离工作区
↓
设计改进
↓
实现
↓
Benchmark
↓
Independent Verification
↓
用户审重大变化
↓
升级 Runtime
```

但只允许自优化：

```
Planner policy
Routing policy
Skill library
Execution method
Verification strategy
Context strategy
Scheduling strategy
```

不能自动取消：

```
Task Contract
Authority
Security Invariant
Permission Boundary
User Review Requirement
```

最高原则：

> «Runtime 可以学习怎样更好地使用规则，但不能自行取消规则。»

---

## 218. 正式命名

整套技术架构正式建议命名：

```
PIER

Persistent Intent Execution Runtime
```

中文：

```
持久化意图执行运行时
```

含义：

```
Persistent
→ 长期状态
→ Memory
→ Product Drive
→ Background Execution
→ Recovery

Intent
→ 用户定义 WHAT
→ Contract
→ Specification

Execution
→ Planner
→ ADFIR
→ Model / Tool / Node Routing
→ Artifact
→ Verification

Runtime
→ 不属于单个模型
→ 不属于单个 Agent
→ 不属于单一 Workflow
→ 是整个智能执行环境
```

产品名称可独立采用：

```
Continuum
```

形成：

```
Continuum
└── PIER Runtime
    ├── Input Canonicalization
    ├── Intent Compiler
    ├── Specification Loop
    ├── Task Contract
    ├── Product Drive
    ├── Plan Review Ladder
    ├── ADFIR
    ├── Execution Method Library
    ├── Verification Runtime
    ├── Model Intelligence Registry
    ├── Memory Synthesis
    ├── Compute Fabric
    ├── Artifact Store
    ├── Policy Engine
    └── Authority Runtime
```

最终完整闭环：

```
User Intent
    ↓
Input Canonicalization
    ↓
Specification
    ↓
Task Contract
    ↓
Strategic Planning
    ↓
Self Review
    ↓
Strong AI Review
    ↓
User Review
    ↓
Product / Milestone Drive
    ↓
ADFIR
    ↓
Execution Method
    ↓
Model / Tool / Compute Routing
    ↓
Isolated Execution
    ↓
Artifact
    ↓
Evidence
    ↓
Verification
    ↓
Product State Update
    ↓
Memory Synthesis
    ↓
Runtime Learning
    └──────────────↺
```

最外层始终由：

```
User Authority
Task Contract
Policy
Security Invariants
```

约束。

最终系统目标：

> «用户只负责说明自己真正想要什么，以及不可越过的边界；PIER 负责持续规划、执行、验证、恢复、学习并将产品向最终可交付状态推进。»
