# Product Drive / 工作区 / 图像 / Memory

本文件对应原始第三份材料。

---

## 149. Product Drive Mode

系统增加一种长期运行模式：

```
Product Drive
```

它与普通 Intent 的区别是：

```
普通 Intent
目标明确
→ 完成
→ 结束

Product Intent
目标是形成最终产品
→ 工程化
→ 形成里程碑
→ 执行
→ 验证
→ 继续生成下一里程碑
→ 持续推进
```

因此用户可以只输入：

```
把这个想法做成一个可以真正发布的软件。
```

之后 Runtime 自己持续推动。

---

## 150. Engineering Feasibility Pass

Product Drive 的第一阶段不是立刻写代码。

首先执行：

```
Idea
 ↓
Requirement Analysis
 ↓
Engineering Feasibility
 ↓
Architecture
 ↓
Engineering Baseline
```

系统需要先把模糊产品概念转换成一个：

```
EngineeringBaseline {
    product_goal
    target_users

    required_capabilities
    hard_constraints

    architecture
    technology_choices

    minimum_complete_system

    known_unknowns
    engineering_risks

    acceptance_criteria
}
```

其目标不是形成一个 MVP 清单，而是首先回答：

> «这个产品作为一个完整工程，怎样才能成立。»

例如：

```
"做一个 AI 视频编辑器"
```

不能直接变成：

```
Milestone 1: 写 UI
```

而应该先明确：

```
媒体输入
Timeline
模型系统
渲染
Artifact
项目格式
恢复
导出
权限
资源需求
```

确保架构首先是一个工程上闭合的整体。

---

## 151. 无限里程碑模型

这里所谓"无限"，不是提前生成无限个 milestone。

而是：

> «Milestone 数量不存在预设上限。»

采用惰性生成：

```
Engineering Baseline
        ↓
Generate next milestones
        ↓
Execute
        ↓
Evaluate product state
        ↓
Generate next milestones
        ↓
...
```

例如：

```
M1 基础架构运行
M2 核心编辑流程可用
M3 自动模型路由
M4 项目持久化
M5 崩溃恢复
M6 GPU 优化
M7 完整媒体导出
M8 插件能力
M9 性能优化
M10 UX 修订
...
```

Runtime 不需要知道：

```
最终一定有 20 个 Milestone
```

而是持续问：

```
当前产品距离 Product Contract
还缺什么？
```

然后生成下一阶段。

---

## 152. Milestone Graph

Milestone 本身也不是简单线性列表。

应该允许：

```
                    M1
                  /    \
                M2      M3
                 \      /
                   M4
                 /    \
               M5      M6
```

因此：

```
Milestone
```

本质上仍然是高层 Intent Graph。

进入具体实现以后，再 lower：

```
Milestone
 ↓
Intent
 ↓
ADFIR
```

形成：

```
Product Graph
    ↓
Milestone Graph
    ↓
Intent Graph
    ↓
ADFIR
```

---

## 153. Product State Evaluation

每完成一个 Milestone，都需要重新评估产品。

建立：

```
ProductReadiness {
    functional_completeness
    correctness
    reliability
    performance
    usability
    security
    maintainability
    documentation
    deployability
}
```

Verifier 不只是检查：

```
代码能不能运行
```

而是判断：

```
这个里程碑是否真正使产品更接近成品。
```

---

## 154. 防止无限空转

"无限驱动"不能意味着无限消耗算力。

因此需要：

```
Progress Gate
```

记录：

```
previous_product_state
current_product_state
progress_delta
```

如果连续执行多个 Milestone：

```
progress_delta ≈ 0
```

Runtime 必须：

```
停止当前路线
↓
重新分析
↓
更换方案
```

必要时：

```
Decision Required
```

例如：

```
当前架构已经连续三个里程碑没有解决性能瓶颈。

A. 重构渲染架构
B. 接受当前性能
C. 修改目标硬件要求
```

因此：

> «无限的是可生成的 Milestone 数量，不是无限无意义计算。»

---

## 155. Shippable State

"最终成品"不是一个抽象的完美状态。

Product Contract 必须至少定义：

```
Shippable Predicate
```

例如：

```
核心功能完整
AND
没有 blocker bug
AND
测试通过
AND
安装流程完成
AND
恢复机制完成
AND
用户要求全部满足
```

一旦达到：

```
SHIPPABLE
```

系统可以通知：

```
产品已经达到可发布状态。
```

如果用户启用：

```
Continuous Product Drive
```

系统可以继续：

```
优化
发现问题
增加质量
降低成本
改进 UX
```

但这些属于：

```
post-shippable evolution
```

而不是无限推迟"完成"。

---

## 156. Product Drive 与 Background Supervisor

Product Drive 可以长期驻留。

例如：

```
用户离开
 ↓
Background Supervisor
 ↓
继续测试
继续 fuzz
继续 benchmark
继续静态分析
继续完成低风险 Milestone
```

用户再次打开软件：

```
昨晚完成：
M17 parser optimization
M18 regression suite

当前：
M19 benchmark validation
```

因此系统可以真正持续向成品推进。

---

## 157. 默认工作区永久只读

前面的 L2 Worktree 应进一步升级成硬规则：

> «AI 默认永远不能直接修改用户原始工作区。»

定义：

```
Base Workspace
= READ ONLY
```

所有 Agent 修改都只能发生在：

```
Task Workspace
```

结构：

```
Project Base
    │
    │ read-only
    ▼
Task Workspace A

Project Base
    │
    ▼
Task Workspace B
```

---

## 158. Writable Workspace

对于 Git：

```
git worktree
```

是默认实现。

例如：

```
project/
    ↑ 用户工作区

.ai/worktrees/
    intent-102/
    intent-103/
```

对于非 Git 项目，可以使用：

```
OverlayFS
Btrfs snapshot
ZFS clone
copy-on-write filesystem
container workspace
```

因此概念不是：

```
Git branch
```

而是更一般的：

```
Writable Overlay
```

---

## 159. Workspace Invariant

Runtime 建立硬约束：

```
AI_WRITE(BaseWorkspace) = DENY
```

即使模型：

```
要求修改 /home/user/project/file
```

Tool Runtime 也必须拒绝。

只能：

```
write(TaskWorkspace/file)
```

因此不是依靠 prompt：

```
"请不要改用户文件。"
```

而是文件系统级保证。

---

## 160. Integration Gate

AI 完成修改以后生成：

```
Patch Artifact
Commit Artifact
Validation Artifact
```

然后：

```
Task Workspace
      ↓
Integration Gate
      ↓
Base Workspace
```

只有进入更高 Effect Level 时才能真正影响用户工作区。

例如：

```
[查看 Diff]
[应用修改]
[Cherry-pick]
[Merge]
[丢弃]
```

这样：

```
AI 写错
AI 重构失败
Agent 崩溃
模型误删文件
```

默认都不会损坏用户工作区。

---

## 161. Image Artifact 原生操作

图像系统不应该统一成一个：

```
image_tool(prompt)
```

至少应该区分两种原生操作。

第一种：

```
GenerateImage
```

从零生成完整图像。

第二种：

```
LocalImageEdit
```

针对已有图片的局部区域进行修改。

---

## 162. 完整图像生成

```
GenerateImage {
    prompt
    aspect_ratio
    resolution
    reference_artifacts
    constraints
}
```

输出：

```
Artifact<Image>
```

这和 ChatGPT 中直接生成完整图片的体验一致：

```
用户描述
↓
生成完整 Image Artifact
↓
直接显示
```

---

## 163. 坐标局部修改

已有图片允许用户直接指出一个位置。

例如 UI 中：

```
点击人物右手
```

产生：

```
ImageEditAnchor {
    x
    y
}
```

坐标可以统一归一化：

```
x ∈ [0, 1]
y ∈ [0, 1]
```

避免与图片实际分辨率绑定。

例如：

```
x = 0.71
y = 0.42
```

用户：

```
"这里换成拿着一杯咖啡。"
```

---

## 164. Point Edit 不是修改一个像素

所谓：

```
point edit
```

只是用户交互入口。

实际 pipeline：

```
User Point
 ↓
Object / Region Detection
 ↓
Semantic Mask
 ↓
Local Generative Edit
 ↓
Composite
```

因为生成模型不可能只通过修改单个像素完成：

```
"把手里的手机换成杯子"
```

Runtime 应根据点击位置找到：

```
hand + held object
```

作为编辑区域。

---

## 165. Local Edit Invariant

局部修改最重要的约束应该是：

```
preserve_outside_mask = REQUIRED
```

流程：

```
Original Image
      ↓
Region Mask
      ↓
Generative Inpainting
      ↓
Generated Patch
      ↓
Hard Composite
      ↓
Final Image
```

即：

mask 外像素

直接从原图复制。

不让生成模型偷偷重新生成整张图片。

---

## 166. Region Edit

除了单点，还应该支持：

```
Point
Bounding Box
Brush Mask
Semantic Object
```

例如：

```
点击人物头发
```

或者：

```
圈住天空
```

或者：

```
选中"左边这个人"
```

最后都转换成：

```
EditMask
```

---

## 167. Image Lineage

每次修改不覆盖原始 Image Artifact。

应该形成：

```
Image v1
 ↓ edit(face)
Image v2
 ↓ edit(background)
Image v3
 ↓ edit(hand)
Image v4
```

每一步记录：

```
source image
mask
prompt
model
seed
output
```

用户可以：

```
撤销
比较
分叉
继续编辑
```

---

## 168. 图像任务的 Verifier

Local Edit 应验证：

```
修改区域是否符合要求
非修改区域是否变化
分辨率是否保持
Alpha / 色彩空间是否异常
```

例如：

```
outside-mask pixel difference
```

可以直接确定性检查。

这比让模型说：

```
"我只修改了手。"
```

可靠得多。

---

## 169. Memory 采用最新 ChatGPT 式综合记忆

Memory 不建议按旧式设计：

```
用户说一句
↓
保存一个 memory 条目
```

作为核心模型。

截至 2026 年，ChatGPT 最新 Memory 路线已经转向一种持续综合的记忆机制。OpenAI 将新的综合过程称为 Dreaming，目标是提高记忆的时效性、连续性和相关性；Memory 会自动更新，而不是仅依赖一组长期不变的人工保存条目。

当前 ChatGPT 的 Memory 来源还可以根据可用性包含过去聊天、保存的信息、自定义指令、Library 文件以及连接应用内容；Memory Summary 则提供高层概括，并不代表系统可能引用的全部上下文。

因此本系统也应该采用：

```
Memory Synthesis
```

而不是：

```
Saved Memory Database
```

作为第一抽象。

---

## 170. Memory Source Graph

所有长期记忆首先来自 Source。

```
Memory Sources
├── Conversations
├── Task History
├── Product History
├── Project Artifacts
├── Files
├── Connected Services
├── Explicit User Corrections
└── User Policies
```

这些才是真正事实来源。

Memory 本身属于：

```
derived representation
```

---

## 171. Dreaming / Memory Synthesis

系统后台周期运行：

```
Memory Synthesis Job
```

流程：

```
Recent Sources
      ↓
Relevant historical sources
      ↓
Contradiction detection
      ↓
Temporal reconciliation
      ↓
Importance estimation
      ↓
Memory synthesis
      ↓
Memory Summary
```

它可以发生：

```
session结束后
系统空闲时
大量项目状态改变以后
用户主动要求时
```

---

## 172. Memory 不保存每件事

Memory 不应该试图：

```
保存所有聊天内容的摘要
```

原始聊天本身已经是 Source。

Memory 应只综合：

```
长期稳定偏好
长期项目
重要约束
持续目标
重要关系
常用工作方式
当前长期状态
```

因此：

```
Source Store
```

负责保存历史。

```
Memory Synthesis
```

负责理解历史。

---

## 173. Memory Summary

用户可以看到一份：

```
Memory Summary
```

例如：

```
项目
- Core
- LCM
- dslsmusic

工作偏好
- 喜欢 Runtime 自动处理执行细节
- 默认不修改原始工作区

长期设计
- Personal Compute Network
- ADFIR

当前持续目标
- 完成 AI Runtime 设计
```

Memory Summary 是：

```
human-readable projection
```

不是底层数据库本身。

ChatGPT 当前同样提供 Memory Summary，并会随着新的上下文自动更新。

---

## 174. Memory Provenance

Memory 中每个事实必须保留：

```
source references
confidence
last confirmed
temporal validity
```

例如：

```
preferred_language = Chinese

sources:
chat #128
chat #361
explicit preference

confidence = high
```

或者：

```
current_project_priority = Core

confidence = medium
last_seen = 5 days ago
```

---

## 175. Contradiction Resolution

例如：

```
2026-01
用户：
"我喜欢 Rust。"

2026-09
用户：
"以后这个项目改成 Core。"
```

Memory 不应该永久保存：

```
用户使用 Rust
```

而应该理解：

```
older state
↓
newer state
```

形成时间化事实：

```
Project X language:
Rust → Core
```

因此 Dreaming 的主要价值之一就是：

```
freshness
```

而不是无限累积历史描述。

---

## 176. Explicit Remember

用户仍然可以说：

```
记住这个。
```

但实现上不必创建旧式特殊 Saved Memory。

它可以产生：

```
HighPriorityMemorySource
```

即：

```
explicit_by_user = true
confidence = high
```

然后参与 Memory Synthesis。

---

## 177. Memory Correction

用户说：

```
"这个已经不是这样了。"
```

系统应该：

```
找到 Memory Claim
 ↓
找到 Sources
 ↓
加入 correction source
 ↓
invalidate / supersede
 ↓
重新 synthesis
```

而不是只在 Summary 里改一句文本。

---

## 178. Forget / Do Not Mention

需要区分：

```
forget
```

与：

```
do_not_mention
```

前者作用于 Memory Source / derived memory。

后者属于：

```
Interaction Policy
```

避免把：

```
不希望再提
```

错误理解成：

```
历史事实不存在
```

---

## 179. Memory Scope

Memory 至少划分：

```
Personal
Project
Product
Task
Runtime
```

例如：

```
"用户喜欢中文"
→ Personal

"Core 不再使用 move"
→ Project

"这个产品决定使用 ADFIR"
→ Product

"当前 bug 已定位 parser"
→ Task

"Model X 做 verifier 经常失败"
→ Runtime
```

---

## 180. Memory 与 Product Drive

Product Drive 会大量依赖长期记忆。

它需要记住：

```
已经做过什么
为什么这样设计
哪些路线失败过
哪些决定已经确定
当前 Milestone
历史 verifier 结果
用户否决过什么
```

因此：

```
Product Memory
```

会成为长期 Product Drive 的主要持久知识来源。

否则运行几个月以后，AI 会不断重复过去已经验证失败的路线。

---

## 181. Memory 与 ADFIR 的关系

Memory 不直接变成 prompt。

正确流程：

```
ADFIR Node
 ↓
Context Requirements
 ↓
Memory Retrieval
 ↓
Context Compiler
 ↓
Relevant Memory
 ↓
Model
```

模型只得到当前任务真正需要的部分。

---

## 182. Memory 隐私

Personal Memory 默认只能运行于：

```
Authority Host
Trusted Personal Node
```

并且需要对应 capability。

Temporary Node 默认：

```
Personal Memory Access = DENY
```

如果某计算确实需要少量用户上下文：

```
Context Compiler
↓
裁剪
↓
Task-specific Capsule
```

而不是把完整 Memory DB 发过去。

---

## 183. Memory Sources UI

借鉴当前 ChatGPT Memory Sources 的思想，用户应该能知道：

```
AI 为什么知道这个？
```

例如：

```
你正在设计 ADFIR

来源：
- 9月29日聊天
- Product Runtime 设计文档
- 当前 Product Memory
```

当前 ChatGPT 也已经开始在个性化回答中提供相关 Memory Sources，让用户检查哪些过去聊天、记忆、文件或连接内容参与了回答。

---

## 184. 四项能力合并后的最终长期循环

加入以上设计后，Runtime 的长期循环变成：

```
                  Product Intent
                        │
                        ▼
              Engineering Baseline
                        │
                        ▼
                 Milestone Graph
                        │
                        ▼
                      ADFIR
                        │
              ┌─────────┴─────────┐
              ▼                   ▼
       Read-only Base        Writable Overlay
                                  │
                                  ▼
                               Execute
                                  │
                                  ▼
                              Verifier
                                  │
                                  ▼
                           Product State
                                  │
                     ┌────────────┴────────────┐
                     ▼                         ▼
                 Shippable                Not Ready
                     │                         │
                     │                         ▼
                     │                 Next Milestone
                     │                         │
                     └──────────────┬──────────┘
                                    ▼
                              Memory Synthesis
                                    │
                                    ▼
                              Product Memory
                                    │
                                    └────► next cycle
```

对于图像：

```
Image Artifact
   │
   ├── Generate Full Image
   │
   └── Local Coordinate Edit
             ↓
          Mask
             ↓
        Local Generation
             ↓
      Outside-mask Preserve
             ↓
       New Image Artifact
```

因此这四项并不是额外外挂功能。

它们分别解决：

```
Product Drive
→ 长期向成品收敛

Immutable Workspace
→ AI 修改的工程安全边界

Coordinate Image Editing
→ 图像 Artifact 的精确交互模型

Synthesized Memory
→ 长时间运行所需要的连续认知
```

组合以后，系统已经具备从：

```
一句产品想法
```

长期推进到：

```
工程可行
↓
持续实现
↓
持续验证
↓
形成成品
↓
持续维护
```

的完整生命周期。
