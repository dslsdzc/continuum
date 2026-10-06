# P3 资源层（子项目 E）设计：Compute Node 注册与节点放置

**范围**：《工程》§4.1 组件表的两行——**Compute Node 注册**（`§287 §288 §289 §290 §291`）与
**节点放置**（`§291 §243 §93`），见 `docs/02-工程.md:225-226`。组件的边界见同文件 §4.2（`:228-240`），
层内入边见 §4.3（`:252-253`），完成判据见 §4.4（`:262`）。

**判据的唯一来源是规范本身**：`§287`–`§291`（`docs/spec/05-normative.md:1649-1737`）、`§243`
（同文件 `:719-731`）、`§93`／`§94`（`docs/spec/02-positioning.md:1024`／`:1054`）、`§42`
（`docs/spec/01-concepts.md:1646`）、`§240`（`docs/spec/05-normative.md:658`）、`§246`（同文件 `:777`）、
`§301`–`§304`（同文件 `:1925-1984`）、`§342`（同文件 `:2680-2707`）、`§343`（同文件 `:2709`）。
规范没给判据的，本设计**不发明**，一律落到 §10 的遗留项并具名收件人。

**本设计的一条主线是 OPEN-007**：`docs/02-工程.md:269` 写着「**OPEN-007 阻断节点放置**。Artifact 隐私
等级的赋值规则与传播规则未定义，放置判定缺少输入」，而 `docs/01-总纲.md:1855` 把同一条记作
「阻塞级别 **可推迟**（v0.1 单机，不触发 placement）」，`docs/superpowers/p3a-followups.md:170`
则记「**接口因此无法冻结**」。三处记录相抵，故 §2 先处置它——**本设计的结论是：接口可以冻结，
取值没有具名的产生方**。这一条决定了本设计的全部重量落在哪一侧。

---

# 1. 范围

## 1.1 建什么、不建什么

| 组件 / 事项 | 规范依据 | 本设计 |
|---|---|---|
| `ComputeNode` 与它的六个字段 | §287 | **建**（§3） |
| Compute Node 注册（进程内注册表） | §287 §288 | **建**（§4） |
| 节点放置的硬闸门（隐私 × 信任） | §243 §94 §42 | **建**（§5.3） |
| 节点放置的排序**接口** | §291 §304 | **建接口，不写数值**（§5.5） |
| `NodeTrust` 的取值域 | §287（未给） | **取二值**，来源 §293／§342.7，**记缺口**（§3.3） |
| `capabilities` / `resources` / `availability` 的取值域 | §287（未给） | **只搬运，不解释**（§3.4） |
| Job Capsule（§290 的五个字段） | §290 | **不建**；只消费它的 `required_artifacts` 那一面（§4.3） |
| Authority Host、Device Join、Trust Graph | §292 §293 | **不建**（E 的组件表里没有它们），故**信任的产生方不存在**（§10 第 12 条） |
| Temporary Node 的租约 | §289 | **不建**：§287 的 `ComputeNode` 没有租约字段，§290 的 `expiry` 在胶囊上（§4.3） |
| 节点状态刷新 / 心跳 | 规范未议 | **不建**（§4.2，记缺口） |
| 分布式执行本身 | §343 明写 v0.1 **可暂不实现** Distributed Compute | **不建**（§8） |

## 1.2 定稿时的现状实读

以下每一条都在 **2026-10-06** 读过源文件，`docs/02-工程.md:269` 关于 OPEN-007 的那句写在此设计定稿之前，
本设计的读法以本节的实读为准。

| 东西 | 现状 | 出处 |
|---|---|---|
| `PrivacyClass`（§243 的五档） | **已存在、已落库**。`Public / Personal / Private / Secret / LocalOnly` 五枚，`ALL` 是五枚，`as_str`／`parse` 是编码的唯一产生点，`parse` 对表外串返回 `None` | `crates/continuum-artifact/src/artifact.rs:87`（枚举）、`:100`（`ALL`）、`:112`（`as_str`）、`:129`（`parse`） |
| `privacy_class` 的落库列 | `NOT NULL`，读出错串即 `PersistError`——**不是默认档次** | `crates/continuum-artifact/src/persist.rs:14-15`、`:120-123` |
| `Artifact` | `pub privacy_class: PrivacyClass`（**不是 `Option`**） | `crates/continuum-artifact/src/artifact.rs:179` |
| D 的 Router 输出面 | **已落地**（Task 11）：`rank`、`RankedExecutionCandidates`（`selected`／`alternatives`／`candidates`）、`ExecutionCandidate` 的五个访问器、`RoutingReason`、`RankingPolicy` | `crates/continuum-model-registry/src/router.rs:379`、`:388`、`:330`、`:340-361`、`:261`、`:409`、`:497` |
| D 的具名基线 `BaselineRankingPolicy` | **尚未落地（Task 12）**。本设计只消费输出面的类型，不依赖它 | 同上 `:40`、`:407` |
| `continuum_graph::NodeId` | **已存在**，指 ADFIR 节点 | `crates/continuum-graph/src/ids.rs:40` |
| `Artifact.producer_node` 的实义 | `Option<NodeId>`，**ADFIR 节点 id**（P1 的读法），且落库列可空 | `docs/superpowers/specs/2026-10-01-p1-execution-layer-design.md:127`；`crates/continuum-artifact/src/artifact.rs:175` |
| `ExecutionProfile.compute_node`（§246） | **本仓零命中**（无该类型、无该字段） | `grep -rn "ExecutionProfile" crates/` 无命中 |
| 迁移号占用 | `1`、`2`（persist）、`10`（artifact）、`20`（graph）、`30`（workspace）、`40`（effect）、`41`（policy）、`50`（capability）、`80`（model-registry）。**`90` 未被占用** | 各 crate 的 `persist.rs`；注册处在 `crates/continuum-runtime/src/main.rs:83-91` |

## 1.3 与《工程》§4.3 的三条入边

`docs/02-工程.md:253` 写的是：

```
节点放置 ← Compute Node 注册 + Artifact 隐私等级 + Router 输出
```

三条入边本设计都接，且**都落在类型上**（不是「将来会接」）：

| 入边 | 落在哪里 | 形状 |
|---|---|---|
| Compute Node 注册 | `PlacementRequest::nodes` | `&[ComputeNode]`，由 §4 的注册表交出 |
| Artifact 隐私等级 | `PlacementRequest::artifacts` | `&[Artifact]`，读它的 `privacy_class` 字段 |
| Router 输出 | `PlacementRequest::candidates` | `&RankedExecutionCandidates`（D 的类型，见 §6） |

**另有 §4.3 与 §9.2 的一条**：`docs/02-工程.md:252` 与 `:598` 都把「Compute Node 注册」列为**入度为零**
的组件。故 §4 的注册表**不依赖任何内部 crate 的数据面**（它只用 E 自己的 `ComputeNode`）；
本 crate 对 `continuum-artifact` 的依赖全部落在**节点放置**一侧。

---

# 2. OPEN-007 的处置：接口能冻结，取值没有具名的产生方

## 2.1 三处记录相抵，先据实并列

| 出处 | 原话 | 读出的后果 |
|---|---|---|
| `docs/01-总纲.md:1855-1858` | OPEN-007「Artifact 隐私等级的设定与传播」……**阻塞级别 可推迟**（v0.1 单机，不触发 placement） | 不阻断 E |
| `docs/02-工程.md:269` | 「**OPEN-007 阻断节点放置。** Artifact 隐私等级的赋值规则与传播规则未定义，放置判定缺少输入」 | 阻断 E |
| `docs/superpowers/p3a-followups.md:170-171` | 「OPEN-007 **整块阻断**子项目 E……**接口因此无法冻结**」 | 阻断 E 的**接口冻结** |
| `docs/superpowers/specs/2026-10-04-p3a-capability-design.md:23`、`2026-10-05-p3d-model-registry-router-design.md:33` | 两份设计的不建表里都写「子项目 E，被 OPEN-007 阻断」 | 同上 |

`docs/superpowers/specs/2026-10-05-eng-005-budget-accounting.md:124` 另记「E 仍被 OPEN-007 阻断，
与本决策无关」。**四处说阻断、一处说可推迟**，本设计不替它们合并，只把判据分开写：

## 2.2 本设计的取法：把「等级是什么」与「谁给等级」拆开

OPEN-007 的正文（`docs/01-总纲.md:1855`）是两件事：

1. **Artifact 隐私等级的设定**——谁在什么时候写入这个等级；
2. **传播规则**——一个节点产出的制品如何继承其输入的等级。

**这两件事都不在 E 的组件表里。** E 的入边是 `Artifact 隐私等级`（`docs/02-工程.md:253`），
即 E 是**这一事实的消费方**，不是它的产生方。而这一事实**在代码里已经有一个可用形状**：
`Artifact.privacy_class: PrivacyClass`，非 `Option`、落库 `NOT NULL`、五档封闭（§1.2 的实读）。

故 §2.1 的相抵这样处置：

- **「接口无法冻结」这句不成立**：E 要的接口是「一枚 `PrivacyClass` 随一枚 `ArtifactId` 进来」，
  它今天就能写出来，判据是 §1.2 那一行的实读，不是推测。
- **「阻断节点放置」这句成立，但阻断的是保证的强度**：E 的硬闸门以**制品上已经记着的等级**为输入。
  等级若赋错，闸门就按错的等级放行——**E 无法察觉，也无法补**。这是本设计最强的一条限度，
  写在 §8 与 §10 第 1 条，不藏在别处。
- **收件人**：规范维护者（本项目无此角色）。
  **阻塞范围**：阻塞**「完成判据的真实性」**（在等级赋值规则落地之前，`docs/02-工程.md:262`
  那条判据只在「等级已正确赋值」的前提下成立），**不阻塞本设计的接口与实现计划**。

**一处附带的事实，据实记**：《工程》§4.5 与《总纲》§10.3 对同一条 OPEN 记了不同的阻塞级别。
本设计按 §2.2 的分法处置后，两者不再相抵（一个是「接口」、一个是「取值」），
但**两处文字的相抵本身没有被消除**，收件人仍是规范维护者（§10 第 1 条）。

---

# 3. `ComputeNode` 与 §287 的六个字段

## 3.1 §287 的六个字段逐项

§287（`docs/spec/05-normative.md:1649-1671`）给出：

```
ComputeNode { id  class  capabilities  resources  trust  availability }
```

| §287 字段 | 规范给的形状 | 本设计 | 说明 |
|---|---|---|---|
| `id` | 未给 | `ComputeNodeId`（E 自己的 newtype，见 §3.6） | 注册表的键，也是放置结果的身份 |
| `class` | **封闭二值**：`PERSONAL \| TEMPORARY` | `NodeClass::{Personal, Temporary}`，逐字照录 | §287 唯一给出取值域的字段（§3.2） |
| `capabilities` | 只给名字 | `Vec<String>`，**E 不解释、不比较** | 取值域规范未给（§3.4、§10 第 4 条） |
| `resources` | 只给名字 | 同上 | 同上 |
| `trust` | 只给名字 | `NodeTrust`，二值（§3.3） | §94 称之为 "Node trust **class**"，但没给这个 class 的取值域 |
| `availability` | 只给名字 | `Vec<String>`，**E 不解释** | §291 的 "current load" 落在它上面；因注册表无刷新入口，它连「有一份状态」都谈不上（§10 第 10 条） |

**这张表是为了堵一处漏项**：§287 的六个字段里，只有 `class` 的取值域是规范给的。若只写
「本设计实现 §287 的 `ComputeNode`」而不逐项说清另外四个字段的形状来自哪里，
后来者会把本设计取的两值信任域与 `Vec<String>` 读成规范的形状。

## 3.2 `NodeClass`：§287 唯一给出取值域的字段

```rust
/// §287 的 "class"。两枚，逐字照录。
pub enum NodeClass { Personal, Temporary }
```

`Personal` 对应 §288 的 Personal Node，`Temporary` 对应 §289 的 Temporary Node。
**本设计不给它加第三枚。** 特别地，§4.4 的完成判据说的是「**云端**节点」，而 §287 里
**没有「云端／本地」这个轴**——处置见 §5.3 末段（取 `class ≠ Personal` 作超集）。

## 3.3 `NodeTrust`：取值域未给，本设计取二值

§287 只给字段名；§94（`docs/spec/02-positioning.md:1054-1072`）说 Scheduler 按
「Artifact privacy **× Node trust class**」决定是否允许传输，也没有给这个 class 的取值域。
规范里能读出的最小区分有四处：

| 出处 | 原话 |
|---|---|
| `docs/spec/05-normative.md:1777` | Device Join 流程的终态是 `Trusted Personal Node` |
| `docs/spec/05-normative.md:1756-1758` | `Authority Host` 可以「改变 Trust Graph」、批准／撤销 Personal Node |
| `docs/spec/05-normative.md:2697`（§342.7） | `Temporary Node 不得自动进入 Personal Trust Domain` |
| `docs/spec/05-normative.md:1704`（§289） | Temporary Node 默认禁止 `persistent trust` |

**取法**：

```rust
/// 节点在信任域中的位置。**取值域是本设计定的**：§287 只给字段名，
/// §94 只给「Node trust class」这个词。两枚的来源见本节的四处引文。
pub enum NodeTrust {
    /// §293 的终态 `Trusted Personal Node`。
    TrustedPersonal,
    /// 在 Personal Trust Domain 之外。§342.7 说 Temporary Node 不得**自动**进入它。
    OutsidePersonalTrustDomain,
}
```

**为什么是二值而不是「照 §292 的 Trust Graph 建一个域」**：§292／§293（Authority Host 与
Device Join）**不在 E 的组件表里**（`docs/02-工程.md:225` 只列了 §287–§291），Trust Graph 的
形状与变更归 Authority。E 需要的是「一枚判据」，不是「一张图」。

**为什么与 `class` 不是同一个轴**：§36 明写「**设备加入网络不等于获得信任**」
（`docs/spec/01-concepts.md:1475`），故 `class == Personal` **不蕴含** `trust == TrustedPersonal`。
§287 同时给这两个字段，正是这个理由。

**两处必须写明的限度**（都是缺口，见 §10 第 3、12 条）：

1. 二值域是**本设计定的**，不是规范的形状。若规范后来给出更细的信任分级，
   `NodeTrust` 要重取，且**放行方向会变**（今天的两个值只在「能不能放 `LocalOnly`」这一处被读，
   加值时必须重新逐值归类，见 §5.4 的通道 (a)）。
2. **信任的产生方不存在**：谁判定一枚节点是 `TrustedPersonal`，按 §292 是 Authority Host，
   而它不在 E 的范围。故本设计**照抄调用方给的标签**，不做任何推断、也不校验它与 `class` 的关系
   （§342.7 是 Authority 的义务，且它的措辞是「不得**自动**进入」，不是「不得进入」）。

## 3.4 `capabilities` / `resources` / `availability`：只搬运，不解释

三者的取值域规范都没有给（§1.2 的实读：§291 只列了因子名，无单位、无形状）。本设计取
**标签集合**（`Vec<String>`）并**只做三件事**：存、交访问器、随节点交给策略。
**E 不解释它们、不比较它们、不据它们排序**——一旦比较两个标签，就要给出一套词表与一个序，
两者规范都没有。

这与 D 对 `§247` 的 `latency_profile` / `cost_profile` 的处置同形（D 的设计 §2.5：有字段、
不可计算），与 P4 对 `PlanFootprint` 的四个字段取 `Opaque` 的处置同形
（`docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md:1346-1349`）。

**代价，据实记**：§291 的十一项因子里，有九项挂在它们与 `latency`／`money` 上，
本设计**一项都算不出来**（§5.6 的表）。

## 3.5 类型与唯一产生点

```rust
/// §287 的 `ComputeNode`。字段私有，唯一构造入口是 `new`。
pub struct ComputeNode { /* id, class, trust, capabilities, resources, availability */ }

impl ComputeNode {
    pub fn new(
        id: ComputeNodeId,
        class: NodeClass,
        trust: NodeTrust,
        capabilities: Vec<String>,
        resources: Vec<String>,
        availability: Vec<String>,
    ) -> Self;
    pub fn id(&self) -> &ComputeNodeId;
    pub fn class(&self) -> NodeClass;
    pub fn trust(&self) -> NodeTrust;
    pub fn capabilities(&self) -> &[String];
    pub fn resources(&self) -> &[String];
    pub fn availability(&self) -> &[String];
}
```

**字段私有而不是六个 `pub` 字段**：`ComputeNode` 要进放置的硬闸门，而闸门只读 `class` 与 `trust`。
六个 `pub` 字段会让「闸门读到的 `class`」与「构造时给的 `class`」之间没有任何一处可挂不变量；
私有字段加访问器把这两处收成一条路径。**代价照实记**：今天没有任何不变量可挂，故这一处私有性的
收益是**结构性的**（将来加字段不许外部直接写字面量），不是今天可观察的。

**访问器的消费方逐条**：`id`／`class`／`trust` 的消费方是 §5.3 的硬闸门与 §5.8 的错误变体；
`capabilities`／`resources`／`availability` 的消费方是**调用方自己实现的策略**（`PlacementPolicy`
的实现体）——**E 内部零读取**，这一句比上面弱，按 D 对 `RoutingReason` 访问器的处置照实写
（D 的设计 §5.2 末段）。

## 3.6 一处撞名：不新建第二个 `NodeId`

`continuum_graph::NodeId` 已存在，指 **ADFIR 节点**（`crates/continuum-graph/src/ids.rs:40`）。
§287 的 `id` 指的是**计算节点**。故本设计取 **`ComputeNodeId`**，
**不新建第二个 `NodeId`**——这与 P3A 当年判定「`ToolId` 复用而不新建第二个」是同一条纪律的反面
用法：那里两者是同一个概念、故复用；这里两者不是同一个概念、故**改名**，而不是沿用同一个词。

**一处既有事实，须记**：§240 的 `Artifact.producer_node` 在本仓的实现是 `Option<NodeId>`
（P1 设计 `:127`），即**ADFIR 节点 id**，而 §93 要的「制品在哪台机器上」是**计算节点**。
同一个词 `node` 在 §240 与 §287 指两件事（§10 第 5 条）。

---

# 4. Compute Node 注册

## 4.1 进程内注册表

```rust
/// 已注册的计算节点。登记顺序即遍历顺序。
pub struct NodeRegistry { /* nodes: Vec<ComputeNode> */ }

impl NodeRegistry {
    pub fn new() -> Self;
    /// 同一 `ComputeNodeId` 二次登记返回 `Err`，**不静默覆盖**。
    pub fn register(&mut self, node: ComputeNode) -> Result<(), NodeRegistryError>;
    pub fn nodes(&self) -> &[ComputeNode];
}

pub enum NodeRegistryError {
    Duplicate { id: ComputeNodeId },
}
```

**三条判据**：

1. **形状沿用本仓已有的两个注册表**——`continuum-provider` 的 `ProviderRegistry`（C 的设计 §3.1）
   与 `continuum-operator` 的 `registry.rs`：重复登记同一 id 返回 `Err(Duplicate)`，已有先例，
   不另发明一套。
2. **重复登记不静默覆盖**：B 的实现阶段查出的 §八.20 与 §八.23 两处「静默后者胜」，
   在本仓一律判为留白。故新节点的登记要么成功、要么具名失败。
3. **`nodes()` 的消费方是 `place` 的调用方**（它把 `&[ComputeNode]` 填进 `PlacementRequest`），
   不是 E 内部——E 不持有「从注册表直接放置」的旁路（与 C 的工具侧「不交出适配器」同形：
   注册是事实来源，放置是纯函数，两者不在同一处收口）。

**不建的东西，逐个点名**：`get(&ComputeNodeId)`（零消费方）、`deregister`（§292 的撤销归 Authority）、
`len`／`is_empty`（零消费方）。C 当年为 `model_providers()` 留下的教训是「零消费方的公开方法即
建好没人用」（C 的设计 §3.1），本设计照办。

## 4.2 不落库：理由、代价，且**不取迁移号**

**结论：本设计不写数据库，故不取任何迁移号。**

三条判据：

1. **规范没有一句话使已登记的节点成为持久事实。** §287–§291 只描述节点是什么；§319 的
   「reconcile running nodes」（`docs/spec/05-normative.md:2242-2252`）说的是**执行中的节点**
   （§303 的 ADFIR 节点），不是已注册的设备。
2. **持久事实的来源在 Authority，不在 E。** §292 说只有 Authority Host 能批准／撤销 Personal Node、
   能改变 Trust Graph；§313 把 `device join`／`device revoke` 列为 MUST 记录
   （`docs/spec/05-normative.md:2132-2145`；`crates/continuum-events/src/audit.rs:22-24`）。
   Authority 不在 E 的范围（§1.1）。**若 E 自己建一张节点表，信任就有两个来源**——这正是本项目
   一贯判为缺陷的形状。
3. **C 为「适配器是代码不是数据」给的三条理由里，有两条在这里也成立**（C 的设计 §3.2）：
   一张表最终还是要在进程内再建一次映射；且本仓的数据库访问一律经 `Tx`，而 `place` 是纯函数
   （§5.2），它不该够得着库。

**代价，据实记**：进程重启后节点集合为空，`place` 在重启后只会返回 `NoPlaceableNode`
（§5.8），直到有人重新登记。今天的唯一登记方是测试，故这一条**没有可观察形态**。

**号段，只记不占**：D 的设计 §11 第 14 条记了号段裁决
（A 50、B 60、C 70、D 80、E 90）。**本设计不据此占号**——§1.2 的实读只说明 `90` 今天未被占用，
不说明它归谁。若将来要落库，取号前须现场核对当时该档未占用（这是本项目删过一次「预留号」的教训，
D 的计划 Task 5 与 §11 第 14 条都记着）。

## 4.3 §288 / §289 / §290 逐条处置

§4.1 组件表的规范列把 §288–§291 都挂在「Compute Node 注册」这一行（`docs/02-工程.md:225`），
故这里逐条交代，免得被读成漏项。

| 规范句 | 它约束的是什么 | 本设计的处置 |
|---|---|---|
| §288：Personal Node MAY 执行任务／缓存 Artifact／运行模型／后台计算 | **许可**（MAY），不是形状 | **不建**。许可不产生类型。 |
| §288：设备加入信任域 MUST 由 Authority 控制 | Authority 的义务 | **不建**（§292 §293 不在 E 的范围）。后果：`trust` 的来源不存在（§10 第 12 条）。 |
| §289：Temporary Node MUST `lease-based` | 租约 | **不建**：§287 的 `ComputeNode` **没有租约字段**，`expiry` 在 §290 的 Job Capsule 上。收件人：规范维护者（§287 要不要 lease 字段），§10 第 9 条。 |
| §289：MUST `least-privilege`／`job-scoped` | 能力与凭据的作用域 | **不建**：归 P3A 的 Capability 与 B 的凭据（`continuum-capability`／`continuum-secrets`）。 |
| §289：默认禁止 `personal memory`／`global secret` | **数据准入** | **不建，且这是一处真缺口**：它要的是「哪一档 `PrivacyClass` 对应 personal memory」，而规范没有给这个映射；「global secret」同理。E 的硬闸门**不会**替它们兜底——一枚被标为 `Public` 的个人记忆会照常上临时节点。收件人：规范维护者，§10 第 9 条。 |
| §289：默认禁止 `network discovery` | 网络层 | **不建**：E 里没有任何网络行为。 |
| §289：默认禁止 `persistent trust` | 信任 | **不建**：与 §342.7 同源，属 Authority（§3.3 限度 2）。 |
| §290：`JobCapsule { subgraph, required_artifacts, scoped_credentials, execution_policy, expiry }` | 临时节点**接收**什么 | **不建这个类型**。五个字段分属四处：`subgraph` 是 ADFIR 图（P1）、`scoped_credentials` 是能力与凭据（P3A／B）、`execution_policy` 是策略（P2）、`expiry` 是租约。**在 E 里建它就要为这四处各拉一条边**，而胶囊的**装配**（§40 的依赖闭包）也不在 E 的组件表里。**E 只消费它的 `required_artifacts` 那一面**：`PlacementRequest::artifacts`（§5.2）就是那一面。收件人：协调者（§290 的 Job Capsule 与 §40 的依赖闭包**无主**），§10 第 9 条。 |

---

# 5. 节点放置

## 5.1 两段式：硬闸门（MUST）＋ 排序（SHOULD）

规范的语气把这一节劈成两半，本设计照这个劈法走：

| 段 | 规范语气 | 本设计 |
|---|---|---|
| **硬闸门** | §243「Scheduler **MUST** 根据 Artifact privacy 和 Node trust 决定 placement」；§94「**决定是否允许传输**」 | **写死、策略不可放宽**（§5.3） |
| **排序** | §291「Scheduler **SHOULD** 考虑……」（十一项）；§93「placement cost 包括 compute cost ＋ data movement cost」 | **冻结接口，不写数值**（§5.5） |

这与 D 把「闸门（§249 的不可路由态，写死）＋ 打分（`RankingPolicy`，推迟）」分开是同一形状
（D 的设计 §4.2 与 §5.3）。**两段的分界不是口味**：MUST 一侧若交给策略，一条宽松策略就能绕过
§4.4 的完成判据；SHOULD 一侧若写死数值，就是发明规范没有的数。

## 5.2 请求面

```rust
/// 一次放置的请求。**纯数据，无 I/O**——与 D 的 `RoutingRequest` 同形（D 的设计 §5.1）。
pub struct PlacementRequest<'a> {
    /// §4.3 的「Router 输出」。见 §6。
    pub candidates: &'a RankedExecutionCandidates,
    /// §290 的 `required_artifacts` 那一面。§243 的隐私输入从这里读。
    pub artifacts: &'a [Artifact],
    /// §4.3 的「Compute Node 注册」。由 `NodeRegistry::nodes()` 交出。
    pub nodes: &'a [ComputeNode],
}
```

**三个字段都是引用、都是必填**（不是 `Option`）：一个想忽略节点集或制品集的调用方，
必须先编出一个空切片，那是一次**看得见的选择**，不是一次遗漏——同 D 把 `budget: BudgetView`
写成必填的理由（D 的设计 §5.3 末段）。

**为什么 `artifacts` 是 `&[Artifact]` 而不是一个新的 `(id, level)` 类型**：§243 的输入就是
制品自己的等级，而 `Artifact` 已经把它记在 `pub privacy_class` 上（§1.2）。另建一个投影类型
是同一份数据的第二个落点（C 的设计 §3.1 为同一问题删过 `model_providers()`）。
**代价**：本 crate 因此依赖 `continuum-artifact` 的 `Artifact` 与 `PrivacyClass` 两个名字（§7）。

**`rank` 与 `place` 都是纯函数，都由签名保证**：`place` 不接 `Tx`、不做 I/O、不读时钟。
故它的错误枚举里**不收** `Persist` 一类变体（D 为同一理由删过 `RoutingError::Persist`，裁决 D1）。

## 5.3 硬闸门：§243 的隐私 × 信任

**规则本身，规范只给了一条可照录的**：§42 的例（`docs/spec/01-concepts.md:1646-1672`）——
「Private photo: `LOCAL_ONLY`，则 **Cloud Node 即使性能最好，也不能被选择**」。
`§94`／`§243` 说存在一张「privacy × trust class」的表，**但那张表规范没有给出**（§10 第 2 条）。

故本设计把闸门拆成**规范侧的下限**与**调用方给的表**两层，**取更严的一侧**：

```rust
/// 一档隐私等级允许被放置在什么样的节点上。
/// **取值域里没有「`LocalOnly` 可以上任何节点」这一项**——见 `spec_floor`。
pub enum TransferRule {
    /// 只允许放在 `class == Personal` **且** `trust == TrustedPersonal` 的节点上。
    TrustedPersonalOnly,
    /// 可以放在任何已登记的节点上。
    AnyNode,
}

/// §42 的那一条例，逐字落在 `LocalOnly` 上；其余四档规范未给，故下限取 `AnyNode`
/// （**这不是「允许」，是「规范没有说不允许」**，见 §5.4 与 §10 第 2 条）。
fn spec_floor(level: PrivacyClass) -> TransferRule {
    match level {
        // §42：「Private photo: LOCAL_ONLY …… Cloud Node 即使性能最好，也不能被选择」
        PrivacyClass::LocalOnly => TransferRule::TrustedPersonalOnly,
        // 其余四档：§243／§94 说「按表决定」，而表未给出。本设计不替它填。
        PrivacyClass::Public
        | PrivacyClass::Personal
        | PrivacyClass::Private
        | PrivacyClass::Secret => TransferRule::AnyNode,
    }
}

/// 两层取更严者。**这是闸门不可被策略放宽的全部机制**：`spec_floor` 不读策略。
fn strictest(a: TransferRule, b: TransferRule) -> TransferRule {
    match (a, b) {
        (TransferRule::TrustedPersonalOnly, _) | (_, TransferRule::TrustedPersonalOnly) => {
            TransferRule::TrustedPersonalOnly
        }
        (TransferRule::AnyNode, TransferRule::AnyNode) => TransferRule::AnyNode,
    }
}
```

`place` 对**每一枚** `required_artifacts` 里的制品做同一件事：读它的 `privacy_class`，
算 `strictest(policy.rules().rule_for(level), spec_floor(level))`，据此过滤节点集。
过滤后为空即 `Err(NoPlaceableNode)`。

**`LocalOnly` 由此一概上不了非 personal、或未被信任的节点**——包括调用方把五档全写成 `AnyNode`
的情形。这条绝对措辞的照片在 §9 第 3 行（最宽策略下的反例），不是靠读代码。

**「云端节点」怎么落**：§4.4 的判据（`docs/02-工程.md:262`）用的是「云端节点」，
而 §287 没有这个轴（§3.2）。本设计取 **`class ≠ Personal`**，判据是 §40
（`docs/spec/01-concepts.md:1576-1600`）把「租用云 GPU」列为 Temporary Node 的一例——
故 `class ≠ Personal` 是**云端节点的超集**，判据因此**至少与 §4.4 要求的一样强**。
**多出来的那部分照实记**：朋友的电脑（§40 的另一例）也是 Temporary Node，本设计同样把它排除在
`LocalOnly` 之外。若规范的本意是「按**地理／网络位置**判云端」而不是按 `class`，那需要 §287 多一个轴，
这是缺口（§10 第 6 条），不是本设计可以替他决定的。

## 5.4 fail-closed 的三条通道

隐私等级「未知」或「未归类」时不许上云，这是本项目的硬要求。本设计不靠假设，靠三条**各自可指认**的通道：

| 通道 | 机制 | 判据 / 照片 |
|---|---|---|
| **(a) 枚举穷尽** | `spec_floor` 对 `PrivacyClass` 的 `match` **穷尽且无通配臂**。加第六档时本函数编译不过，必须逐档归类 | 与 `PrivacyClass::as_str` 同形（`crates/continuum-artifact/src/artifact.rs:112-124` 的注释明写「加档时本函数编译不过，编码不会漏分支」）。**这条不是运行期可拍的**，据实记在 §8 第 5 条。 |
| **(b) 表覆盖** | 调用方给的 `PlacementRules` **必须覆盖五档**，缺档即构造期 `Err`（见下） | 照片：只给四档的 `try_new` 返回 `Err(MissingLevel { level })`；给重复档返回 `Err(DuplicateLevel { level })` |
| **(c) 字符串解码** | **E 从不经字符串读隐私等级**——进来的就是 `PrivacyClass` 值 | 唯一解码点是 `PrivacyClass::parse`（`artifact.rs:129`），表外串返回 `None`，落库侧把它翻成 `PersistError`（`persist.rs:120-123`）。故「未知等级」到不了 E。列 NOT NULL 使得「没有等级」也不可表达（`persist.rs:14`） |

通道 (b) 的类型：

```rust
/// §94 的「privacy × trust class」表。**值由调用方给，本设计一个都不填。**
pub struct PlacementRules { /* entries: Vec<(PrivacyClass, TransferRule)>，字段私有 */ }

impl PlacementRules {
    /// 必须覆盖 `PrivacyClass::ALL` 的每一档，**多一条、少一条、重复一条都是 `Err`**。
    pub fn try_new(entries: Vec<(PrivacyClass, TransferRule)>) -> Result<Self, RulesError>;
    /// 构造期已保证覆盖，故这里不含 `Option`。查询落空时的返回值是 `TrustedPersonalOnly`
    /// ——**不可达**（构造期已排除），但若真到达，它是 fail-closed 的那一侧。
    pub fn rule_for(&self, level: PrivacyClass) -> TransferRule;
}

pub enum RulesError {
    MissingLevel { level: PrivacyClass },
    DuplicateLevel { level: PrivacyClass },
}
```

**覆盖判据的来源是 `PrivacyClass::ALL`**（`artifact.rs:100`），不是本设计手写的一张五档清单：
一张手抄清单在规范加档时**不会失败**，而 `ALL` 会。因此第 (b) 通道对**将来新增的档**仍然成立
——那时的旧表只有五条，`try_new` 判它 `MissingLevel`，即 fail-closed。

## 5.5 排序：接口在本层，数值不写

```rust
pub trait PlacementPolicy {
    /// §94 的表。见 §5.4 (b)。
    fn rules(&self) -> &PlacementRules;
    /// 全序比较：`Ordering::Less` 表示 `a` 排在 `b` 前面。
    ///
    /// **操作数就是 `ComputeNode`**——不另立一个「已打分节点」类型（同 D 的 `compare`
    /// 不另立 `Scored` 的判据，D 的设计 §5.3）。
    fn compare(&self, a: &ComputeNode, b: &ComputeNode) -> Ordering;
}

/// 具名基线。**它不对 §291 的任何一项因子作主张**——十一项里一项都算不出来（§5.6）。
///
/// `compare` 对任何一对返回 `Ordering::Equal`；次序完全由 `place` 的兜底档决定（§5.9）。
pub struct BaselinePlacementPolicy { /* rules: PlacementRules */ }

impl BaselinePlacementPolicy {
    pub fn new(rules: PlacementRules) -> Self;
}
```

**为什么不给一个「保守的」数值基线**：§291 的十一项因子里，能算的只有 `privacy`（§243 给了判据）
与 `trust`（判据是本设计取的二值域）；其余九项**连量纲都没有**（§5.6）。要对它们取一个「保守方向」，
先得有一个**序**，而这个序规范没有给——这与 P4 拒绝为 §276 的风险门槛写基线的理由是同一条
（P4 设计 §4.5：「无中生有的序会把『高成本』判成『低风险』」）。

**故本设计交付的是：闸门（可拍）＋ 接口（可换一份策略重跑同一组用例）＋ 一条确定的兜底（§5.9）。**
被否掉的替代方案是「照 §291 的列举顺序逐项加权求和」：它需要十一项之间的权重与各自的方向，
两者规范都没有。**收件人：规范维护者 ＋ 语义层设计**（§10 第 7 条）。

## 5.6 §291 的十一项因子逐项

§291（`docs/spec/05-normative.md:1725-1737`）是 SHOULD，十一项。逐项处置：

| §291 因子 | 本设计 | 依据 |
|---|---|---|
| `privacy` | **有**，落在硬闸门 | §243／§94；§5.3 |
| `trust` | **有字段、判据本设计取二值** | §287 只给字段名；§3.3、§10 第 3 条 |
| `CPU` | **无输入** | 节点侧 `resources` 取值域未给；**任务侧没有任何字段说它需要多少 CPU**（§247 的十二个字段里没有资源项） |
| `GPU` | **无输入** | 同上 |
| `VRAM` | **无输入** | 同上 |
| `RAM` | **无输入** | 同上 |
| `bandwidth` | **无输入** | 同上 |
| `artifact locality` | **无输入** | §93 要「制品在哪台机器上」，而 `Artifact.producer_node` 在本仓是 ADFIR 节点 id（§3.6），且表里没有「当前所在计算节点」这一列。§5.7、§10 第 5 条 |
| `latency` | **无输入** | 无单位、无取值域 |
| `money` | **无输入** | 无单位；§93 的 compute cost 与 data movement cost 两项都算不出来 |
| `current load` | **无输入** | 挂在 `availability` 上，而它取值域未给；且注册表**没有刷新入口**（§4.1），故它连「会变」都不成立。§10 第 10 条 |

**九项无输入、一项部分可用。** 这张表是为了堵一处漏项：只说「§291 是 SHOULD、故推迟」会让人以为
缺的是**数值**，而实际缺的是**输入**——即便有人真去写一个基线策略，他也拿不到这九项里的任何一项。
这与 D 的 §5.1 那张表（§250 的八个 MUST 项里「上下文长度」无输入、该因子未实现）是同一处置。

## 5.7 §93 的 locality：没有输入

§93（`docs/spec/02-positioning.md:1024-1052`）说「分布式任务调度**必须知道 Artifact 在哪里**」，
并给出 `placement cost = compute cost + data movement cost`。本设计的实读：

- **「在哪里」这一事实在本仓没有落点**：`Artifact` 的 `producer_node` 是 `Option<NodeId>`
  且是 ADFIR 节点 id（§3.6）；`artifact` 表（`crates/continuum-artifact/src/persist.rs:9-30`）
  没有位置列。
- **两项成本都算不出来**：compute cost 需要 §291 的资源量与单价（都没有）；data movement cost
  需要制品大小（**有**，`Artifact.size: u64`）与两端的网络成本（无）。

故 §93 在本设计里**没有产生方**，`PlacementPolicy` 的调用方也拿不到 locality。
**收件人：规范维护者**（§10 第 5 条）。

## 5.8 失败路径：每条各是哪一枚

```rust
pub enum PlacementError {
    /// 闸门过滤之后一个节点都不剩（含 `nodes` 为空的情形）。
    NoPlaceableNode,
    /// `nodes` 里同一个 `ComputeNodeId` 出现了两次。
    ///
    /// **它是 §5.9「确定」这条断言的守门人**：两条同 id 的节点无从定序，
    /// `ComputeNodeId` 兜底档也就兜不住（与 D 的 `DuplicateModelCandidate` 同形，D 的设计 §5.4）。
    DuplicateNode { id: ComputeNodeId },
}
```

**只有 `place` 真的会产出的变体，两枚。** 三处「看起来该收但没有产生方」的，逐条说明：

- **不收「未归类的隐私等级」**：它在 `PlacementRules::try_new` 就被拒（§5.4 (b)），
  而 `PlacementPolicy::rules()` 交出的是一枚**已构造的** `PlacementRules`，
  故「缺档」这条路径**到不了 `place`**（同 D 不收 `RequirementError` 的判据，D 的设计 §5.4）。
- **不收 `Persist(...)`**：`place` 是纯函数、签名里没有 `Tx`（§5.2），产不出读库失败。
- **不收「无候选模型」**：候选集非空是 D 的 `rank` 的既有不变量（D 的设计 §5.2 第一条断言），
  故 `place` 不必替它再判一次；`PlacementRequest` 装的是**已构造的** `RankedExecutionCandidates`。

**`NoPlaceableNode` 不区分「因为隐私被滤掉」与「本来就没节点」**，判据：区分它需要把
「哪几个节点因哪一条被滤掉」记成一个输出（D 的 `RoutingReason` 那样的类型），而 §243／§291
**没有要求放置输出一个理由**（§84 要求 Router 输出 reason，放置没有对偶句）。故不立这个类型。
**代价**：判据 §4.4 的照片必须写成**一对**（§9 第 1、2 行），单看「只有云节点 → `Err`」这一条
分不清两种失败。

## 5.9 确定性

排序必须有全序且不随输入顺序漂移——否则「排出来的节点」取决于调用方怎么排 `nodes` 切片。

**机制**：`place` 用 `sort_by(|a, b| policy.compare(a, b).then_with(|| a.id().cmp(b.id())))`，
即**无论策略给出什么样的比较，`ComputeNodeId` 升序都是最后的兜底档**。
`ComputeNodeId` 是 `String` 的 newtype，两两可比且**在同一份 `nodes` 里无相等**——后者由
`DuplicateNode` 这条 `Err` 保证。

照片：把同一组节点按不同顺序放进 `nodes`，三次调用返回同一个节点（§9 第 4 行）。

---

# 6. 与 D 的接缝：Router 输出怎么进本层

**本条是本设计要向复审与 D 表态的地方，故单独成节。**

§4.3（`docs/02-工程.md:253`）要求 `节点放置 ← Router 输出`，故 `PlacementRequest::candidates`
装的是 D 的 `&RankedExecutionCandidates`（不是 `ModelId`）。**理由有两条，第二条是硬的**：

1. **把 §4.3 的那条边兑现成类型名**。若只装一枚 `ModelId`，本层对「Router」的依赖就只剩
   `continuum-core` 里的一个 id 类型，而 §4.3 那条边会变成「名不符实」——D 设计 §8.4 为
   `Router ← … Capability Token` 那半行做过同样的自我订正，本设计不重犯。
2. **`ExecutionCandidate` 没有 crate 外的构造点**（D 的设计 §5.2：字段私有，唯一构造点在 `rank` 内）。
   故调用方**只能**把它从 `rank` 得到的那枚 `RankedExecutionCandidates` 整体交进来，
   而不能自己拼一枚候选。这条不是设计选择，是既有封装的事实。

**一处必须明说的零读取**：本设计**今天不从 `candidates` 读任何东西**——
`place` 的闸门只读 `artifacts` 与 `nodes`；§291 的十一项因子里没有一项与「选中哪个模型」有关，
因为任务侧根本没有资源需求字段（§5.6）。本设计**不为此发明一个读取**（例如「按模型去挑节点」
需要「哪个节点能跑哪个模型」这一事实，而 §287 的 `ComputeNode` 没有模型字段、§36 的
「可以有 models」只是一句许可）。

**这一条请求复审表态，也请求规范维护者回答**：§4.3 的 `节点放置 ← Router 输出` 到底指
输出的哪一部分？两种读法都自洽——(a) 只指「这次要跑哪个模型」，那这一条边应改述为
`节点放置 ← Model 选择`，且本字段可收窄；(b) 指整份候选排序（例如「首选模型放不下时试下一枚」），
那就需要一条规范没有的**回退规则**，本设计**不发明**它（回退会改变实际运行的模型，
与 §251 的升级阶梯是同一个轴，属 D）。**收件人：复审者 → 规范维护者**（§10 第 8 条）。

**D 侧需要的访问器，本设计一个都不缺**：`selected()`／`model()`／`state()`／`compatibility()`／
`confidence()`／`reason()` 都已存在（`crates/continuum-model-registry/src/router.rs:388`、`:340-361`），
且 `RoutingReason` 的四个访问器也在（`:287-306`）。**本设计不请求 D 新增任何面。**
（D 的 `BaselineRankingPolicy` 是 Task 12，尚未落地；本设计不依赖它。）

---

# 7. 依赖、crate 划分与 ALLOWED

## 7.1 一个新 crate

```
continuum-node   （资源层：§287–§291 的 ComputeNode 与节点放置）
```

```
crates/continuum-node/
  Cargo.toml
  src/lib.rs        导出面与 crate 文档
  src/node.rs       ComputeNode、ComputeNodeId、NodeClass、NodeTrust
  src/registry.rs   NodeRegistry、NodeRegistryError
  src/placement.rs  PlacementRequest、TransferRule、PlacementRules、PlacementPolicy、place
  src/error.rs      PlacementError、RulesError
  tests/{node,registry,placement,rules}.rs
```

**为什么是一个新 crate，而不是落进 `continuum-runtime`**：G 的设计 §10.1 论证 G 不新建 crate
（它是 Runtime 的一条**路径**，且 runtime 的 `ALLOWED` 已含它需要的全部 crate）。本设计与那条判据
逐条比对后结论相反：E 是 §4.1 组件表里的一个**资源层组件**（`docs/02-工程.md:225-226`），
与 D 同类，而 D 建了 `continuum-model-registry`；且 E 在 runtime 里**没有任何调用点**
（§8 第 1 条），把它放进 runtime 等于把资源层的类型搬进驱动层却无人使用。
被否掉的替代方案即「落进 `continuum-runtime`」与「落进 `continuum-artifact`」（后者是 P1 执行层，
且会让放置与制品存储同址）。

## 7.2 依赖边（只登记实际用到的）

```
continuum-node → continuum-artifact, continuum-model-registry
```

| 边 | 用到的具体东西 | 判据 |
|---|---|---|
| `continuum-artifact` | `Artifact`（`PlacementRequest` 的字段）、`PrivacyClass`（`TransferRule` 的判据面与 `PlacementRules` 的键） | 两个都是**已存在的类型**，本设计一个都不新建（§1.2） |
| `continuum-model-registry` | `RankedExecutionCandidates` | §4.3 的那条入边（§6） |

**明确不登记的边，以及为什么**：

- **不登记 `continuum-core`**：本设计对它**零引用**（没有用到 `ModelId`、`ToolId`、
  `ProviderHealth` 中的任何一个）。**零使用的边即假边**——P2b 为此删过两条，
  `continuum-secrets` 也据同一条只登记了 `continuum-capability`（`dependency_direction.rs:103`）。
- **不登记 `continuum-persist` / `continuum-events`**：§4.2 判本设计不写库。
  `device join`／`device revoke` 两条审计是 §313 的必录项，但它们的**产生方是 Authority**，
  不在 E 的范围（§4.3、§10 第 12 条）。**为它新增一个 `AuditKind` 变体等于扩 §313 的必录清单**，
  那是规范的事（D 为同一理由不登记 `continuum-events`，D 的设计 §8.2）。
- **不登记 `continuum-graph`**：本设计对 ADFIR 图零引用（§93 的 locality 缺口的**后果**，不是理由）。
  §3.6 提到的 `continuum_graph::NodeId` 只是**解释本设计为何取名 `ComputeNodeId`**，不是一条边。
- **不登记 `continuum-policy`**：见 §7.4。
- **`continuum-runtime` 侧一条边都不加**：E 在 runtime 里没有调用点（§8 第 1 条）。
  **这一条要写明**，否则会被后来者当成漏登记。

**按《工程》§9.1 的箭头读法**（被依赖者 → 依赖者，`docs/02-工程.md:560-566`）：

```
continuum-artifact        ──→ continuum-node
continuum-model-registry  ──→ continuum-node
```

即本 crate **被它们二者依赖的反方向不存在**。反向边核过：`continuum-artifact` 的允许集合是
`core / events / persist`（`:32-35`）、`continuum-model-registry` 的是 `capability / core / persist`
（`:129-136`），**都不含 `continuum-node`**，故不构成环。

## 7.3 `ALLOWED` 表与 workspace 成员

`crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED` 需要**新增一条**，
按字母序放在 `continuum-model-registry` 之后：

```rust
    // P3 子项目 E：计算节点与放置。设计 §7.2 的两条边是 artifact（`Artifact` /
    // `PrivacyClass`，§243 的隐私输入）与 model-registry（`RankedExecutionCandidates`，
    // 《工程》§4.3 的「节点放置 ← Router 输出」）。**不登记 core**：本设计对它零引用。
    // **不登记 persist / events**：本设计不写库（设计 §4.2）。
    ("continuum-node", &["continuum-artifact", "continuum-model-registry"]),
```

**同时 `Cargo.toml` 的 workspace `members` 要加一行**（`Cargo.toml:4-21` 是显式清单，不是通配）。
两者缺一即红，且**红的不是同一条断言**：`dependency_direction.rs` 的
`every_crate_depends_only_on_its_allowed_set` 里有一步「workspace 成员必须在 `ALLOWED` 里」
（`:246-262`），故漏登记 `ALLOWED` 会当场失败——这一点与 C 的 §4.3 说的那个年代不同，是本设计实读到的。

**多写者单文件**：`Cargo.toml` 与 `dependency_direction.rs` 是本轮多份计划共写的文件。
按 `docs/superpowers/p3bcdf-followups.md` §七.7 的裁决，**本子项目只登记自己这两条**，
不设集中登记 task，也不改动别的条目。

## 7.4 与策略层（P2 的 Policy Engine）的接缝

`continuum-policy` 已经有一个以 `privacy_class` 为事实的求值器
（`crates/continuum-policy/src/context.rs:33`、`crates/continuum-policy/src/rule.rs:442-491`），
故必须回答「放置的隐私裁决是不是该走它」。**本设计的判定是：不走，且两者不是同一件事。**

- **§243 的主语是 Scheduler**（`docs/spec/05-normative.md:731`），而 §304
  （`docs/spec/05-normative.md:1973`）把 Resource Scheduler 定在第 4 层。故放置的隐私裁决归 E。
- **策略层的 `PolicyContext` 里没有节点侧的事实**：它的事实集是 `explicit_current` / `privacy_class` /
  `effect_type` / `task_class` / `duration_ms`（`context.rs:28-40`），
  没有 `NodeClass`、没有 `NodeTrust`。故它今天**表达不出**「这一档等级 × 这一类节点」这个二元判定。
- **两条边都登记会成环**吗——不会：`资源层 (4) → 边界层 (5)` 是既有方向（`docs/02-工程.md:577`）。
  故这**不是**一条被方向挡住的边，是**今天没有用处**的边：E 的闸门若要经策略层，
  得先为策略层补两项节点侧事实，而那是「为一个不存在的消费方扩事实集」。

**收件人：复审者 → 协调者**（§10 第 11 条）：若复审判定「放置的隐私裁决应当经 Policy Engine」，
那么要改的是**事实集**与**§304 的落点**两处，且要说明「策略层可 Deny、不可 Allow 放宽 §42」这条
不对称怎么保证。本设计不替它决定。

---

# 8. 完成判据与拍不到的照片

《工程》§4.4 的第三条（`docs/02-工程.md:262`）：

```
隐私等级为 LOCAL_ONLY 的 Artifact 不会被放置到云端节点
```

由 §5.3 的硬闸门覆盖：对每一枚 `required_artifacts`，`spec_floor(LocalOnly)` 恒为
`TrustedPersonalOnly`，且它与调用方规则取**更严者**，故任何策略都放不宽它。

**本设计明确拍不到的照片，逐条列出**（按本项目纪律，写不出照片的把「为什么没有」写出来）：

1. **端到端的一次真实分布式放置**：没有第二台计算设备、没有节点执行循环、
   `ExecutionProfile`（§246）在本仓零命中（§1.2）。**没有产生方**，故没有端到端照片。
2. **§291 九项因子的实际权衡**：它们**没有输入**（§5.6），故没有任何用例能断言
   「选出来的节点是更合适的那一个」。能拍的只有机制：闸门、全序、确定、字段齐备。
3. **OPEN-007 的赋值与传播规则**：制品等级由谁写入、产出制品如何继承输入的等级，规范未定
   （§2）。故所有用例里的 `privacy_class` 都是**测试直接给的值**——
   「等级对不对」在本阶段不可观察，这是 §2.2 那条限度在测试上的形态。
4. **§290 的 Job Capsule 真的把数据限制住了**：胶囊不在本设计里（§4.3），
   故拍不到「临时节点只拿到了该拿的」。
5. **§5.4 通道 (a) 的枚举穷尽**：它是一条**编译期**性质（加档时红），
   没有运行期照片——不能为了拍它去改 `PrivacyClass`。据实记，
   同形先例是 `PrivacyClass::as_str` 的注释（`artifact.rs:112-124`）。
6. **§342.7「Temporary Node 不得自动进入 Personal Trust Domain」**：Authority 不存在，
   信任标签由调用方给（§3.3 限度 2），故拍不到「它没被自动放进去」。
7. **§93 的 data movement cost**：locality 无输入（§5.7），故拍不到「搬运成本被算进去了」。

---

# 9. 测试策略

| 验什么 | 怎么验 |
|---|---|
| **判据 §4.4**：混合节点集下 `LocalOnly` 只落在 personal 且被信任的节点上 | 给定 `[云节点(temporary, outside), 桌面(personal, trusted)]`，断言 `place` 返回的那个，并逐项断言它的 `class() == Personal` 与 `trust() == TrustedPersonal` |
| **判据 §4.4 的否定面** | 节点集只有云节点时返回 `Err(NoPlaceableNode)`（**上面一条与这一条是成对的**，单看任一条分不清两种 `Err`） |
| **闸门不可被策略放宽** | 五档**全写** `AnyNode` 的 `PlacementRules`：`LocalOnly` 的制品仍只落在 `TrustedPersonalOnly` 那一类节点上（**逐档遍历**，不是只测 `LocalOnly` 一档） |
| 确定性 | 打乱 `nodes` 顺序三次调用，返回同一个节点 |
| 重复 id | `nodes` 里同一 `ComputeNodeId` 两次 → `Err(DuplicateNode { id })`，并断言是哪一枚 |
| 空节点集 | `nodes` 为空 → `Err(NoPlaceableNode)` |
| 无制品 | `artifacts` 为空 → 不过滤任何节点（`LocalOnly` 的判据不适用），返回兜底档选中的那个 |
| 五种 `class` × `trust` 组合 | 二值 × 二值逐项：只有 `(Personal, TrustedPersonal)` 通过 `LocalOnly` 的闸门 |
| **`PlacementRules` 的覆盖** | 逐档：少一档 → `Err(MissingLevel { level })`（**五档各一条**）；重复一档 → `Err(DuplicateLevel { level })`；全五档 → `Ok` |
| 注册表 | 登记成功、二次登记同一 id → `Err(Duplicate { id })`、`nodes()` 保持登记顺序 |
| `ComputeNode` 不可外部构造 | trybuild 样例：crate 外写不出字段字面量（与 D 的 `RoutableModel`、P3A 的 `AuthorizedTool` 同形） |
| 策略可替换 | 同一组用例换一份测试策略（按 `capabilities` 标签数排序）重跑，结果随之改变——**钉「排序真的读策略」，不钉「哪个策略对」** |

---

# 10. 遗留与未决项

每条给：**证据（规范为什么不裁）／收件人／阻塞的是设计还是计划**。

1. **OPEN-007（Artifact 隐私等级的设定与传播规则）**——`docs/01-总纲.md:1855-1858` 把它列为
   「设定与传播」未定；`docs/02-工程.md:269` 记它阻断节点放置；`p3a-followups.md:170` 记
   「接口因此无法冻结」；`eng-005` `:124` 记它仍阻断 E。**本设计的读法见 §2.2：接口能冻结、
   取值没有具名的产生方**。附带一条：《总纲》§10.3 记「可推迟」与《工程》§4.5 记「阻断」相抵，
   **两处文字的相抵本身未被消除**。**收件人：规范维护者。**
   **阻塞范围**：只阻塞「判据的真实性」，不阻塞设计，也不阻塞计划。
2. **§94／§243 的「privacy × trust class」表规范未给出**：本设计只照录了 §42 给出的那一条
   （`LocalOnly` ↛ 云），其余四档的规则**由调用方给，本设计一个都不填**，故那四档的宽严
   完全取决于调用方——一份把四档全写 `AnyNode` 的表会让 `Secret` 上临时节点。
   **这是一个显式选择，不是本设计的保证。** 收件人：规范维护者（给出那张表），
   以及候选产生方（Contract 的 `data_policy`，§224；或用户策略）——**本设计不指定谁产生它**。
   **阻塞范围**：阻塞「保证的完整」，不阻塞接口冻结。
3. **§287 的 `trust` 无取值域**：本设计取二值（§3.3）。若规范给出更细的分级，`NodeTrust` 要重取，
   且**放行方向会变**——`LocalOnly` 的闸门逐值归类，加值时必须重新归（§5.4 通道 (a)）。
   收件人：规范维护者。**阻塞范围**：不阻塞（已取二值并说明来历）。
4. **§287 的 `capabilities` / `resources` / `availability` 无取值域**：本设计只搬运（§3.4），
   故 §291 的十一项因子里九项**无输入**（§5.6）。收件人：规范维护者。
   **阻塞范围**：不阻塞接口，但它使「排序」这一半在数值上不可实现。
5. **§93 的 artifact locality 无输入**：`producer_node` 在本仓是 ADFIR 节点 id（P1 设计 `:127`），
   `artifact` 表无位置列；§240 与 §287 的 `node` 是两件事（§3.6）。收件人：规范维护者。
   **阻塞范围**：不阻塞（§93 是 SHOULD 的输入之一）。
6. **「云端节点」在 §287 里没有对应轴**：本设计取 `class ≠ Personal`（超集，更严），
   多排除了朋友的电脑一类（§5.3 末段）。若规范本意是按地理／网络位置判，需要 §287 多一个轴。
   收件人：规范维护者。**阻塞范围**：不阻塞。
7. **排序没有基线，也没有可写的基线**：§291 的十一项里一项都算不出来（§5.5、§5.6）；
   `BaselinePlacementPolicy::compare` 对任何一对返回 `Equal`，次序由 `ComputeNodeId` 兜底。
   收件人：规范维护者 ＋ 语义层设计（若判定应由 `data_policy` 或用户策略给权重）。
   **阻塞范围**：不阻塞——判据 §4.4 是 MUST 侧的性质，不依赖排序数值。
8. **§4.3 的 `节点放置 ← Router 输出` 指输出的哪一部分，本设计请求表态**：`PlacementRequest::candidates`
   今天**零读取**（§6）。两种读法：(a) 只指「跑哪个模型」→ 该边应改述、字段可收窄；
   (b) 指整份候选排序（含「首选放不下就试下一枚」）→ 需要一条规范没有的回退规则，
   本设计不发明。收件人：**复审者 → 规范维护者**。**阻塞范围**：不阻塞接口；
   但若取 (b)，`place` 与 D 的升降级阶梯之间会多一条接缝，须由 D 与 E 同时改。
9. **§289 的四条默认禁止与 §290 的 Job Capsule 无落点**：
   `personal memory`／`global secret` 是**数据准入**，而「哪一档 `PrivacyClass` 对应它们」
   规范没有给，故 E 的闸门**不会**替它们兜底；`network discovery` 属网络层；
   `persistent trust` 属 §342.7 的 Authority。`JobCapsule` 的五个字段分属四处（§4.3），
   且 §40 的**依赖闭包**（胶囊的装配）在 §4.1 的组件表里没有主。
   收件人：协调者（指派 Job Capsule 与依赖闭包的归属）＋ 规范维护者。**阻塞范围**：不阻塞 E。
10. **注册表没有刷新入口**：§291 的 `current load` 因此连「会变」都不成立。
    未建的理由是「规范未议」，不是遗漏（§4.1）。收件人：规范维护者。**阻塞范围**：不阻塞。
11. **放置的隐私裁决是否应经 P2 的 Policy Engine**：本设计判「不走」（§7.4，三条判据）。
    收件人：复审者 → 协调者。若判「走」，要改策略层的事实集与 §304 的落点两处。
    **阻塞范围**：不阻塞本设计；若改判，E 与 P2 两侧都要改。
12. **`NodeTrust` 的产生方不存在**：§288 说信任域由 Authority 控制，§292／§293 的
    Authority Host 与 Device Join **不在 E 的组件表里**，故本设计照抄调用方给的标签；
    §313 的 `device join`／`device revoke` 两条必录审计因此也没有产生方。
    收件人：协调者（下一个子项目）＋ 规范维护者。**阻塞范围**：不阻塞 E。
13. **本设计不落库、不取迁移号**（§4.2）：`90` 今天未被占用（§1.2 的实读），但**号段裁决不构成占号**
    ——D 的设计 §11 第 14 条已记「每一档取用前仍须核对该档未占用」。
    若复审判定节点应持久化，取号前须现场再核一次。收件人：复审者。**阻塞范围**：不阻塞。
14. **§304 的「多少并行」无主**：§304（`docs/spec/05-normative.md:1973-1984`）的四项里，
    节点放置归 E、用哪个模型归 D／G、哪个工具归 A／F，**「多少并行」在 §4.1 的组件表里
    没有对应组件**。收件人：协调者 ＋ 规范维护者。**阻塞范围**：不阻塞 E。
15. **`ExecutionProfile.compute_node`（§246）在本仓零命中**，而放置的结果正是它的输入；
    G 的设计 §7 已把「装配 `ExecutionProfile` 的那一步」记为缺失，收件人写作
    「**驱动侧的节点执行装配点**」。**本设计的收件人与它同一个人**（§8 第 1 条）。
    收件人：驱动侧的节点执行装配点 ＋ 协调者。**阻塞范围**：不阻塞——本设计的唯一生产调用方
    就是测试；这一条是「建好但无生产调用方」，按 P1 的清单写法据实记，不声称它已被接上。
