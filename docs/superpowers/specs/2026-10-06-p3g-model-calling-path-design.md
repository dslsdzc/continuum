# P3 模型调用路径（子项目 G）设计：取快照、调排序、发起模型调用与失败分类

**范围**：把**模型调用路径**接上生产路径——取 `ProviderHealth` 快照、组装 `RoutingRequest` 并调 D 的
`rank`、消费 `RankedExecutionCandidates`、对选中的模型发 `ModelProvider::invoke` / `stream`、
把适配器的失败转成 `FailureClass`。

**归属与具名来历**：G 由用户 2026-10-05 拍板新开，依据是
`docs/superpowers/specs/2026-10-05-p3-bcdf-set-decisions.md` 第一节第 1 条——**它此前是承担约 28 处义务、
却从未具名的「执行侧」**。该角色在本轮之前的四份设计里被写成「执行侧」「消费 `RankedExecutionCandidates`
的那个子项目」「调用方（驱动，子项目 F）」三种记法（`docs/superpowers/p3bcdf-followups.md` 第一节与第五节）。

**G 与 F 是两条不同的路径，本设计不得借用 F 的名字**（同上裁决）：

| | 路径 | 规范依据 | 强制点覆盖 |
|---|---|---|---|
| **F**（已设计） | 工具调用路径：`authorize` → `ToolProvider::invoke` | §4.2 §252 §253 §313 | 强制点 **(1)**（共享面 §四.1） |
| **G**（本设计） | **模型调用路径**：`rank` → `ModelProvider::invoke` / `stream` | §315 §247–§251 §84 | **没有任何强制点** |

**「模型调用没有任何强制点」是本设计的核心命题，不是一句免责**：G 不得用「反正 D 那边过了闸门」把这句
话糊过去。§2 正面处理它——三个强制点逐个核、给出分工的判据、并写明分工之外的一处**收不回的残留**与
一处**规范级缺口**。

**与共享面的关系**：路径归属、已冻结的接口、四条横切约束以
`docs/superpowers/specs/2026-10-05-p3-bcdf-ownership-and-interfaces.md` 为准。本文与共享面冲突处会在正文点名。

**G 的输入接口由 C 与 D 定**（`p3bcdf-followups.md` 第五节）：C 的 `ProviderRegistry` 调用面
（`model_for` / `model_providers` / `ModelProvider` 的七项方法）与 D 的 `RankedExecutionCandidates` /
`RoutingRequest` / `BudgetView`。**本设计不定义这两侧的任何类型**，只在它们的现文上判 G 该怎么用。

**执行序**：G 依赖 C 与 D 都落地（C 的注册表与 D 的 `rank` 是它的两个输入端）。裁决 §四 写的是
「G 本轮不设计，等 B/C/D/F 落计划后再起」——本文即那一次设计，写在 C 与 D 的实现期（C 刚到 Task 1，
D 到 Task 9），故 §4 的两处接口请求**此刻改最便宜**。

---

# 1. 范围

## 1.1 建什么、不建什么

| 事项 | 规范依据 | 本子项目 |
|---|---|---|
| 候选集的**取用**（从 D 的表读、过 D 的闸门） | §249 §21 §22 §247 | 建（§3.2） |
| **可用性快照**（适配器级 `ProviderHealth` → 模型级 `availability`） | §250 §315 | 建（§3.3） |
| 组装 `RoutingRequest` 并**调 `rank`** | §250 §84 ENG-005 | 建（§3.1） |
| 消费 `RankedExecutionCandidates`，对选中模型**发起** `invoke` / `stream` | §315 | 建（§3.1） |
| 流的消费与**中止**（`cancel(&stream.call)`） | §315 | 建（§3.5） |
| **模型侧失败分类**（`ProviderError` → `FailureClass`） | §309 §5.1 | 建（§6） |
| `usage()` 的语义 | §315 | **判为不定**（§5） |
| `ExecutionProfile` 的模型侧两个字段 | §246 | 只定口径，**不落实现**（§7） |
| 候选**排序**与打分（`rank`、`RankingPolicy`） | §250 §84 | **不建**（子项目 D） |
| 画像、Registry 生命周期、三张表 | §247–§249 | **不建**（子项目 D） |
| Provider 的注册与发现（`ProviderRegistry`） | §315 §80 | **不建**（子项目 C） |
| 记账（预扣 / 结算 / 父子分配 / 判预计超预算） | §333 §110 ENG-005 | **不建**（语义层；§8） |
| 重试裁决与升级触发（`decide_retry`、`next_step` 的驱动） | §251 §309 | **不建**（执行层；§6.4） |
| 工具调用路径的任何部分 | §316 §252 | **不建，也不得借用它的名字**（§1.3） |

**不建的部分不留桩。**

## 1.2 归属：G 是 Runtime 的模型调用路径

依据是《总纲》§1：**「执行过程中的模型选择、Agent 编排、工具调用、资源调度、验证、失败恢复和结果交付
由 Runtime 负责」**（`docs/01-总纲.md:16`），职责表里第一项是「拆解、**模型选择**、推理强度……」
（`docs/01-总纲.md:22`）。这与 F 的归属是同一句话的两个分句（`docs/superpowers/specs/2026-10-05-p3f-tool-call-path-design.md`
引的是同一行的「工具调用」）。

**「模型选择由 Runtime 负责」与「Router 归子项目 D」不矛盾，但要把两句话各指什么写清**：
§250/§84 把 Router 的交付物定为**带 confidence / alternatives / reason 的排序候选**（那是**判断**），
而 G 拿这份判断**去执行**。故 `docs/02-工程.md` §4.3 的那行现文是
「`Provider Adapter → 被模型调用路径（子项目 G）调用，接口中立`」（`docs/02-工程.md:249`，2026-10-05 订正）
——**它说的正是本设计**。C 的设计 §1 与本设计的读法一致
（`docs/superpowers/specs/2026-10-05-p3c-provider-boundary-design.md:14-19`）。

**同址而不同路径**：G 落在 `continuum-runtime` 的库里（§10），与 F 同 crate——这是**唯一**的共同点。

## 1.3 用词纪律：本设计的模块里不出现工具调用路径的名字

裁决原文是「G 不得借用 F 的名字」（set-decisions §一.1）。本设计把这条落成一条**模块面判据**：

- **`crates/continuum-runtime/src/model_call.rs`（及其测试）的源码文本里，不出现 `AuthorizedTool`、
  `AuthorizedToolInvocation`、`ToolProvider`、`invoke_tool`、`authorize` 这五个标识符**。
- **照片**：照 C 的模块面守卫形状（守卫及其空白归一化在
  `2026-10-05-p3c-provider-boundary-design.md:309-322`，用例表行在 `:814`），一条断言读该文件的源码文本、
  **匹配前折叠空白**（裁决 C7 的口径：空白变体是同一个字面拼法），逐项断言五个标识符零命中。
- **它的逃逸面与 C 那条同形，两侧都写明**：它匹配的是**字面拼法**，故 `use … as` 别名、全限定路径、
  以及「把工具路径的代码 `include!` 进来」都不命中。**归一化之后它仍是一个下界，不是封闭判定**——
  与 C §4.1 末段的处置相同（那边的上界要 `syn`，本阶段不做）。
  > **订正（2026-10-08，G Task 4 实测：本段的逃逸面清单对本 needle 集不成立）**：
  > 本处五枚 needle 是**单 token 标识符**（`AuthorizedTool` / `AuthorizedToolInvocation` /
  > `ToolProvider` / `invoke_tool` / `authorize`）。**`use … as` 别名与全限定路径都必须把原拼法写出来
  > ⇒ 它们命中，不是逃逸。** 那三条逃逸面**出自 C**，那边的 needle 是 `impl ModelProvider for`
  > 这种**多 token 短语**——别名与换序才会改变文本。**同一个词在两种 needle 下含义相反。**
  > **真正的逃逸面（据实测改写，不穷尽）**：他处写出后经 `crate::` 拐弯引用、`include!`、
  > 宏展开、**清单外的同义名字**。**只有前两条与新清单绑定；后两条是「下界」本身的限度。**
- **为什么值得有这条守卫**：这不是洁癖。混称已经发生过三次（裁决 §一.1 记「四份设计混称过」，
  `p3bcdf-followups.md` §六.4 记协调者的派单措辞是漂移源），而**「同一件事两个词汇表」在本项目一贯
  判为 Critical**。守卫逮住的是最便宜的那一类复发。

---

# 2. 「模型调用没有任何强制点」——正面处理

## 2.1 三个强制点逐个核：没有一个管模型

共享面 §四列了三个强制点。逐条核它们管什么、为什么管不到 G：

| 强制点 | 管什么 | 落在哪 | 为什么管不到模型调用 |
|---|---|---|---|
| **(1)** | 工具调用前必须持 `AuthorizedTool` | F，类型强制落在 C 的 `invoke_tool`（C §7.1） | 模型调用**不是工具调用**。`ModelProvider` 与 `ToolProvider` 是两个 trait、两套类型（`crates/continuum-provider/src/model.rs:14-21` 与 `…/tool.rs:9-13`）。`invoke_tool` 在工具侧不加模板地收 `AuthorizedTool`，模型侧的 `invoke` 收的是 `InvokeRequest`（`crates/continuum-core/src/model.rs:44-48`），**里面没有任何授权位** |
| **(2)** | 实际副作用前必须持驱动铸的 `AuthorizedEffect` | 驱动，B 收下才做副作用（共享面 §四.2） | §268 的「外部操作」指**不可通过 Artifact 回滚**的外部动作。一次模型调用不写外部世界（它返回内容），故它不是效应；G 也**不写 Effect Journal**（§9） |
| **(3)** | 凭据由 `continuum-secrets` 按能力**逐枚签发** | B 是第一个真消费方（共享面 §四.3） | 逐枚签发的前提是「有作用域与有效期、按能力派生」。模型适配器持有的是它自己的端点凭据、在装配期就固定，**不由能力派生**；§315 的请求面里也没有承载它的位置（判据与收件人见 §13.1 对 C §12 第 23 条的处置） |

**结论一句**：**这条路径上没有「必须出示凭据才能发起」这个结构，因为规范在这条路径上没有任何
「必须出示 X」的要求。** 三个强制点的措辞里没有一个主语是模型调用（《工程》§4.2 的三句与共享面 §四的四条
都可以逐句核）。

## 2.2 那 G 凭什么不做越权的事

两条，合起来才是答案。**单有其中一条都不够**，故两条都要写。

**第一条：它没有可越的权。** 模型调用路径不触碰能力、不写外部效应、不取凭据。它能做的最大一件事是
「按一个模型 id 发一次模型调用」。规范对这条路径**没有权限要求**，故「越权」在这条路径上**没有定义**
——第 2.1 节的三行就是这条的依据。

**第二条：它能选哪个模型，是 D 的产出，不是它的判断。** 这条要有结构，不能靠 G 自觉。
**三层里有两层是代码事实，第三层是 D 的设计与计划的承诺、尚未落地**——两者不能混说：

| 层 | 状态 | 判据 |
|---|---|---|
| `ModelProfile` | **已交付（代码事实）** | 字段私有、无公开构造函数；`pub(crate) fn try_new` 在 `crates/continuum-model-registry/src/profile.rs:415`（结构体在 `:387`），故 **crate 外构造不出来** |
| `RoutableModel` | **已交付（代码事实）** | 字段私有；唯一构造点 `RoutableModel::try_new(profile, state)` 在 `crates/continuum-model-registry/src/lifecycle.rs:167`（结构体与闸门在 `:153-170`），入参是**十态** `LifecycleState`，闸门在函数体内发生 |
| `RankedExecutionCandidates` / `ExecutionCandidate` | **尚未落地**（以 2026-10-06 的工作树为准） | **可核的判据**：`grep -rn "pub fn rank\|pub struct RankedExecutionCandidates\|pub trait RankingPolicy" crates/continuum-model-registry/src/` **零命中**（那天在落地的是请求面 `router.rs`：`RoutingRequest` / `TaskSkillRequirement` / `FamilyPreference`）。字段私有的写法与「唯一构造点在 `rank` 内」只见于 **D 的设计 §5.2**（`docs/superpowers/specs/2026-10-05-p3d-model-registry-router-design.md:727-757`）与 **D 的计划 Task 11**（`docs/superpowers/plans/2026-10-05-p3d-model-registry-router.md:1189-1205` 的类型与 `rank` 签名、`:1164` 的 trybuild 样例那一行） |

**故 G 的 plan 里要把它记成一条对 D 的**输出面**（`rank` 与上面那三个类型）的依赖**：
在它们落地之前，G 的第②③段无从编译。**这一点写出来是为了别让后来者以为它已经有了**——
「把设计里说了的写成已经有了」是本项目反复出现的形状（D 的设计 §8.4 记过同类的一处三翻面，
`…p3d-…md:1145-1157`）。

**G 侧把这条落成自己的公开面**（与 C 的 §7.1 同一取向）：**G 的两个发起入口（`call` 与 `call_stream`）
都收 `&ExecutionCandidate`，都不收 `ModelId`**（§3.1）。于是「G 去跑一个没排过序的 id」在 G 的公开面上
放不下——与 C 的 `invoke_tool(&AuthorizedTool, ..)` 是同一取向的另一处落点。

## 2.3 分工的判据（不能只说「反正 D 过了」）

**闸门落在 D、G 只做消费者**，三条判据：

1. **闸门的输入只存在于 D。** 判「能不能自动路由」要 `LifecycleState`（§249）与画像的存在性，两者都是
   D 的表与类型；`RoutableModel::try_new` 还要一枚 D 造的 `ModelProfile`。G 手里没有第二份，**也不该有**
   ——若 G 自己维护一份「哪些模型能路由」的判定，那件事就有了两个产生点。
2. **闸门的效果必须是「谁都过不去」，不是「G 记得检查」。** C 的注册表在模型侧**直接交出裸适配器**
   （`model_for`，C §3.1；这是为保证排序纯函数与模型侧无强制点而定的），故**任何持有注册表的一方都能
   绕过 G 直接 `invoke`**（§2.4）。把闸门放在 D、并让 G 的输入面只接受 `RankedExecutionCandidates`，
   能保证的只是「**经 G 这条生产路径发出的调用，其模型必已过闸门**」——**这一条有射程，§2.4 明写它到哪为止**。
3. **可测。** 闸门在 D 侧时，它的照片是 D 的十态逐项用例
   （`only_the_six_routable_states_pass_the_gate`，在 `crates/continuum-model-registry/src/lifecycle.rs:331`
   的 `#[cfg(test)] mod tests` 里——它要一份画像，而画像的构造是 crate 内的）与 `tests/compile_fail/`
   的 `routable_model_fields_are_private.rs` 那份 trybuild 样例——**两者都已交付**（2026-10-06 实读）。
   若闸门搬到 G，同一件事要在两处各测一遍，而两份可以各自漂移——「同一个失败两个裁决者」的同一病灶。

## 2.4 残留一：`model_for` 交出裸适配器，直发 `invoke` 绕得过 G

**据实记，不声称不存在的保证。** C 的 `ProviderRegistry::model_for(&ModelId)` 返回
`Arc<dyn ModelProvider>`（C 计划 Task 1 的签名），故**持有注册表的一方可以不经 G、不排序、不看闸门，
直接对一个任意 `ModelId` 发 `invoke`**。G 收不回这一份：注册表不是 G 的，`Arc` 已经交出去了。

**与 F 的对照，以及为什么这里没有对应的收紧手段**：F 的路径在裁决 §一.2 之后从「注册表不交出适配器」
升到了 **trait 级**——`ToolProvider::invoke` 的请求参数换成 `AuthorizedToolInvocation<'_>`，
**连裸适配器也不能不持 `AuthorizedTool` 就 `invoke`**（C §7.5、C §12.4）。模型侧**没有这一步可做**，
原因是**规范的，不是设计的**：§316 的 `invoke` 有一条「必须先获准」的规范要求（强制点 (1)），
故请求面上有东西可加；**§315 的 `invoke` 没有任何规范要求**（§2.1 的表），故
`InvokeRequest`（`crates/continuum-core/src/model.rs:44-48`）的三个字段里没有、也**不该**有一个授权位。
**在规范没给判据处发明一个位置，正是本项目明令禁止的那一类。**

## 2.5 规范级缺口：§249 的 `MUST NOT` 主语是 **Router**

§249 的原文是「**Router** MUST NOT 自动使用 UNPROFILED / QUARANTINED / DISABLED 模型」
（`docs/spec/05-normative.md:876-884`），《工程》§4.2 的重复也把主语写成 Router
（`docs/02-工程.md:236`）。D 的闸门（`RoutableModel`）正是这条禁令的落点。

**缺口是**：**直接 `ModelProvider::invoke` 一个未画像的模型，不被这条禁令的字面覆盖。**
它约束的是「自动路由」这件事，而 §2.4 那条绕行路径上**连路由都没有发生**。
「§249 的禁令该不该从『Router』扩到『模型调用』」——**规范未给判据**，本设计**不发明**，
记在 §14 第 1 条（收件人：规范维护者）。**这三句是本设计对「没有强制点意味着什么」的最诚实的一段**：
不是「有强制点但没接」，而是「**规范在这条路径上没有设强制点，而绕行路径是否需要一条禁令，规范未答**」。

---

# 3. 调用面

## 3.1 流程：四段，两个同步边界

C 的设计 §6 给了调用方七步（`…p3c-provider-boundary-design.md:462-475`）。本设计把它与 G 的两段
（同步读库 / 异步调用）交错起来，落成四个函数：

```rust
// ① 同步段：读 D 的表、过 D 的闸门、解析适配器。**收 `Tx`**。
pub fn plan_candidates(tx: &Tx<'_>, registry: &ProviderRegistry)
    -> Result<Vec<Candidate>, ModelCallError>;

// ② 异步段：取可用性快照 → 组装请求 → 调 rank，并把**句柄与排序结果成对带出**。**不收 `Tx`**（§3.4）。
pub async fn select(candidates: Vec<Candidate>, input: RouteInput, policy: &dyn RankingPolicy)
    -> Result<CallPlan, ModelCallError>;

// ③ 异步段：对**一个候选**发起调用。**不收 `Tx`**；**也不收 `ModelId`**（§2.2）；
//    句柄是**显式的一枚入参**——`ExecutionCandidate` 里没有它（D 的类型不含适配器）。
pub async fn call(candidate: &ExecutionCandidate, adapter: &Arc<dyn ModelProvider>,
                  input: CallInput<'_>, deadline: Option<Duration>)
    -> Result<InvokeResponse, ModelCallError>;

// ④ 流式的那一支，与 ③ 同形；中止是它自己的一个入口（§3.5）。
pub async fn call_stream(candidate: &ExecutionCandidate, adapter: &Arc<dyn ModelProvider>,
                         input: CallInput<'_>, deadline: Option<Duration>)
    -> Result<ModelStream, ModelCallError>;
pub async fn abort(adapter: &Arc<dyn ModelProvider>, stream: &ModelStream)
    -> Result<(), ModelCallError>;
```

两枚随行类型（字段私有，构造点各只有一个）：

```rust
/// ① 的产物：**一个候选与它自己的适配器**。构造点唯一（在 `plan_candidates` 内，
/// §3.2 的三条合取同一次完成）。
pub struct Candidate { model: RoutableModel, adapter: Arc<dyn ModelProvider> }

/// ② 的产物：**D 的排序结果** 与 **候选 id → 句柄** 的配对表。
///
/// **为什么需要它**：`select` 调完 `rank` 之后，`ExecutionCandidate` 只带 `ModelId`
/// （D §5.2 的五个访问器里没有句柄），而 ③ 与 `abort` 都要句柄——故句柄必须在 ② 与 ③ 之间
/// 有地方安放。**配对的键是 `ModelId`**（见正文第 6 条）。
pub struct CallPlan { /* ranked + adapters，字段私有 */ }

impl CallPlan {
    pub fn selected(&self) -> (&ExecutionCandidate, &Arc<dyn ModelProvider>);
    pub fn alternatives(&self) -> Vec<(&ExecutionCandidate, &Arc<dyn ModelProvider>)>;
}
```

**代码块只是示意，正文才是约束**（本项目既有口径：设计里的代码块未被编译器核过）。要写死的约束是：

1. **候选集与适配器句柄同一次定下**：`Candidate` 里带 `RoutableModel` 与它对应的
   `Arc<dyn ModelProvider>`（由 `registry.model_for(id)` 得到）。**解析只发生一次**，因为解析失败要
   影响候选集（§3.2）；若在 `call` 里再解析一次，那就是同一件事的第二个产生点。
2. **`select` 收的是候选本身，不是库句柄**：它够不着 D 的表，故「没有画像就没有候选」不可能在它这里被绕过。
3. **`select` 按值收 `Vec<Candidate>`，不是 `&[Candidate]`**。判据是**类型上的不可投影**：
   `rank` 的第二个入参是 `&[RoutableModel]`（D 计划 Task 11 的签名），而 `RoutableModel`
   **字段私有、不可克隆**（`crates/continuum-model-registry/src/lifecycle.rs:153-170`），
   故 `&[Candidate]` **变不出** `&[RoutableModel]`——一组按值持有 `RoutableModel` 的元素，
   投影不出它们内部那个字段的切片。按值收之后，`select` 内部把它们**解构成**一枚
   `Vec<RoutableModel>`（正是 `rank` 要的那个）与一枚句柄表。**本条的来历**：本设计初稿的代码块写的是
   `&[Candidate]`，**那是一处写不出来的签名**（G 的计划在 Task 7 据实报出，本设计据此订正）。
4. **`call` / `call_stream` 各收一枚 `&Arc<dyn ModelProvider>`；`abort` 非收不可**。
   判据两条：(a) 正文第 1 条要求句柄在 `select` 与 `call` 之间不丢；(b) **`ModelStream` 自己不带句柄**
   （`crates/continuum-core/src/model.rs:93-96` 只有 `call: CallId` 与 `chunks`），
   故「中止一条流」除了 `CallId` 还必须知道**问哪个适配器**。**来历**：初稿的代码块在这两处
   把句柄丢了，G 的计划补齐，本设计据此订正。
5. **`call` 收 `&ExecutionCandidate`**，不是 `ModelId`（§2.2）。
6. **配对靠 `ModelId`，且它在排序之后仍唯一**。`CallPlan` 按 `ModelId` 把 D 的每条候选配回它自己的句柄；
   **唯一性的两个来源**：(a) G 的候选集**逐行来自 `model_registry`，`id` 是该表的 `PRIMARY KEY`**
   （D §3.1 的建表语句），故 G 交出去的候选 id 两两不同；(b) D 在 `rank` 内**另有一道判重**
   （`DuplicateModelCandidate`，D §5.3——它在 G 这条路径上不可达，正是 (a) 的结果，§4.5）。
   **若少了 (a)**，按 id 配对就可能把两条候选配到同一个句柄上，而那是**静默的错误配对**。
7. **`RouteInput` 按值、无生命周期参数**。它持 `TaskSkillRequirement` / `FamilyPreference` / `BudgetView`
   三样（§250 的第 1、5 项与 ENG-005），而 `TaskSkillRequirement` **没有 `Clone`**
   （D 计划 Task 10：字段私有 + `try_new`）——**收引用会逼出一次不可得的克隆**。
   不取「给 `TaskSkillRequirement` 加 `Clone`」这条替代：那要改 D 的类型，而收益只是省一次移动。
   **来历**：初稿的代码块写成 `RouteInput<'_>`，那个生命周期在正文里没有对应物。
8. **截止只包住一次调用**（§3.5）。
9. **候选集为空时不调 `rank`**：直接以 `ModelCallError::Routing(RoutingError::NoEligibleCandidate)` 返回——
   这一条让「一个适配器都没登记」与「全部被闸门挡下」两种情形走同一条失败路径，
   与 D 的 `NoEligibleCandidate`（D 设计 §5.4）形状一致。**它不是 G 自己新造一枚同名的错**：
   G 交出的就是 D 那一枚（§6.1 的错误类型）。

**这三笔订正（第 3、4、7 条）的来历是一处**：初稿的代码块是示意，**示意的部分写错了两处**
（一个写不出来的签名、一个没有对应物的生命周期），并**漏了一处**（句柄在 ②③ 之间丢失）。
G 的计划在写的过程中逐条报出并按最小改动补齐；本设计据此把口径**写回正文**
（口径以设计为准、计划不替设计做决定）。**第 6 条是这次补齐逼出来的新口径**——配对键与它的唯一性
来源，初稿一个字都没写，而它是「句柄与候选不会错配」这句话的唯一依据。

**`RouteInput` 里的三个值来自别处，G 不产它们**：`TaskSkillRequirement` / `FamilyPreference`
（§250 的第 1、5 项）来自规划侧；`BudgetView` 来自驱动的投影（D §6.1）。**这三个值今天的生产方都不存在**
（§12）。

**`Deadline { elapsed_ms: u64 }` 的口径是「实测耗时」**（从发起到截止触发那一段），
**不是「截止值」**。判据：字段名与语义必须同宽——写成「截止值」会让 `elapsed_ms` 这个名字说谎，
而**名字与语义两说正是本项目反复出错的形状**。**它今天没有消费方**，故 §11 的用例
**只断言是哪一枚 `Err`、不断言数值**（断一个数值就是钉一次巧合：实测耗时在调度抖动下不等于截止值）。
若将来有人要按它做退避或遥测，那是**消费方出现**那一刻的事（§14 第 16 条）。
**来历**：初稿给了这个字段、没给口径，G 的计划据实报出。

## 3.2 候选集：三个来源，一条接缝规则

**「哪些模型存在」在这条路径上共有三个来源**，这是必须先点明的事实：

| 来源 | 它回答什么 | 归属 |
|---|---|---|
| D 的 `model_registry` 表 | 这个模型**被登记过**吗、它现在什么状态 | 子项目 D |
| C 的 `ProviderRegistry` | 这个 id **由哪个适配器**服务 | 子项目 C |
| 适配器自己的 `list_models()` | 它**自报**服务哪些 id | 适配器 |

前两个来源的差异，C 已经记过一处同类（C §3.3 代价一：登记 id 与 `list_models` 是两个产生点；
C §3.3 代价三：`tool` 表与 `ProviderRegistry` 是两个登记点，且**那一处的收件人是 F，因为只有 F 同时
经两个入口**——`…p3c-provider-boundary-design.md:241`）。**模型侧的相应接缝的收件人就是 G**：
它是唯一同时经 D 的表与 C 的注册表的一方。本设计据此写死一条规则：

> **候选集只从 D 的 `model_registry` 出发。** 一个模型成为候选，条件是三件事的合取：
> （a）它在 `model_registry` 里被登记过；（b）它有画像且当前状态过了 §249 的闸门（由
> `RoutableModel::try_new` 判，G **不自己再判一次**）；（c）`registry.model_for(id)` 命中。

- **（c）不命中时丢弃，不报错**（`RegistryError::NotFound` 就是丢掉这一条）。**判据与 D 的
  `Unavailable` 那一档同源**：§250 的输出是**可执行的**候选（D §5.3 的原话），而一个没有适配器的模型
  定义上不可执行——它比 `Unavailable` 更硬。**这是 fail-closed 的方向**：丢弃意味着不执行；
  同一方向的对照是 D 对 `availability` 缺席的处置——`Err(UnknownAvailability)`，并明写「按未知放行是
  fail-open 的形状」（D §5.3）。两处判据一致：**不确定就不放行**（差别只在形式：D 拒绝，G 丢弃——
  G 丢弃得更早，那时它连一个候选都没有）。
- **§3.1 的 `Candidate` 里那条对应关系在这一步就定死**：它的 `adapter` 由
  `registry.model_for(candidate.model.profile().id())` 解析而来（`RoutableModel::profile()` 在
  `crates/continuum-model-registry/src/lifecycle.rs:175`，`ModelProfile::id()` 在
  `src/profile.rs:446`），故「候选 → 句柄」的对应不需要在排序之后**推断**，只需按 `ModelId` **复原**
  （§3.1 第 6 条）。**这条同时是「解析只发生一次」的落点**：`call` 不再解析。
- **丢弃必须是显式的、有照片的，不是静默的**：§11 给两侧各一条（丢 vs 登记后回来）。
- **G 不调 `list_models()`**。登记是**路由**的权威、`list_*` 是**描述**的权威（C §3.1 的口径），
  而 G 要的是路由事实。第三个来源因此**在这条路径上一次都不出现**——它有否定式照片（§11）。

## 3.3 可用性快照：适配器级 → 模型级

§250 的「当前可用性」在 D 的类型上是**模型级**：`RoutingRequest.availability: Vec<(ModelId, ProviderHealth)>`
（D 计划 Task 10）。而 §315 给可用性的方法只有一个：`async fn health(&self) -> ProviderHealth`，
**挂在适配器上、无参**（`crates/continuum-provider/src/model.rs:21`）。

**G 的构造**：对每个候选调**一次**它所对应的适配器的 `health()`，把结果按该候选的 `ModelId` 记下。

**由此产生一条本设计写明的上限**：**同一个适配器名下某个模型单独不可用这件事，在 §315 的类型上不可表达。**
G 只能把一个适配器的健康度**原样摊给它服务的每个模型**。判据：`ProviderHealth` 的三个变体
（`crates/continuum-core/src/model.rs:83-87`）没有一个是「按模型」的；**唯一以 `&ModelId` 为入参的方法**
是 `describe_model(&ModelId)`，它返回的是 `ModelDescriptor`（`:21-27`）、**不含健康度**——它能给的最多是
某个按模型的 `Err`，而 `Unavailable` 这个变体在本仓已被另一件事占用（C §5.1 记的既有夹具：
`crates/continuum-provider/tests/fake_provider.rs:106` 把「无此工具」这个**永久性配置缺陷**报成
`Unavailable`；**订正 2026-10-07（G Task 1 实测，原引坐标照留）**：该坐标**已不成立**——工具侧夹具已搬走，
`fake_provider.rs` 现存 104 行、只剩 `FakeConnector`。实读的新坐标（p3g 树 `404c464`）是
**`crates/continuum-provider/tests/common/mod.rs:177`**，`FakeTool::describe_tool` 的 `.ok_or_else`），
故它**不是可靠的可用性信号**。记在 §14 第 2 条（收件人：C 的设计 + 规范维护者）。
（**措辞射程**：带模型的不止它一个——`InvokeRequest` 里也有 `ModelId`
（`crates/continuum-core/src/model.rs:44-48`），`invoke` 和 `stream` 都收它。本条要说的是
**按 id 询问状态**这件事只有 `describe_model` 一个方法，故措辞收窄到「以 `&ModelId` 为入参」。）

**一条不做的事**：G **不发明探活超时**。`health()` 的时长没有上界（§315 无超时参数，C §5.3 已判
「超时不进签名」），故一次挂住的探活会把整条路由挂住。**要不要给探活一个截止，规范未给判据**，
本设计不发明；记在 §14 第 3 条。

**顺序**：快照按候选的次序生成（确定性）。是否并发探活**不影响可观察结果**（D 的 `rank` 对
`availability` 的次序无判据，只按 id 查条目），故它是实现选择，**没有照片，本设计不规定**。

## 3.4 `Tx` 不得跨 `await`——结构性约束，两条独立理由

**事实一（类型）**：`Tx<'a>` 内部持一枚 `std::sync::MutexGuard<'a, rusqlite::Connection>`
（`crates/continuum-persist/src/tx.rs:12-15`）。**`MutexGuard<T>` 是无条件 `!Send`**（std 的负向实现：
在某些平台上解锁必须发生在同一个线程上，故它与 `T` 是不是 `Sync` 无关），`Tx` 因此也不是 `Send`。
**持着它跨 `await` 的 future 不是 `Send`。**
（**订正**：本段初稿把这条归因到「`Connection` 不是 `Sync`，故 `MutexGuard<Connection>` 不是 `Send`」
——**机制写错了**：那是一条条件性的推断，而 `MutexGuard` 的 `!Send` 是无条件的。结论不变，理由据实改正。）

**事实二（资源）**：整个库是**单连接 + 单 `Mutex`**——`Db::begin` 的文档明写「同一时刻只有一个事务」
（`crates/continuum-persist/src/db.rs:140-148`）。故跨 `await` 持有 `Tx` 不只是类型问题：
**它会把模型调用期间的整仓库访问全部挡住**（一次模型调用可以跑任意久）。

**故 G 的异步段一律不收 `Tx`**（§3.1 的②③）。这条不是风格，是上面两条事实的直接后果。

**照片（编译期，形态是「编译得过」而不是「编译不过」）**：一条用例取 `select` / `call` 返回的 future，
断言它是 `Send`——例如 `fn assert_send<T: Send>(_: &T) {}` 后 `assert_send(&fut);`，再 `.await` 它。
**若实现把 `Tx` 带进了异步段，这条用例编译不过**。**为什么用「编译得过」而不是 trybuild**：
trybuild 钉的是「这样写编译不过」，而这里要钉的是「**这个签名的 future 是 `Send`**」——
被钉的对象是签名本身的性质，写在用例里比写成样例更直接。**它的证明力有边界，写明**：
它只覆盖被断言的那一个 future，**不证明 G 的每一处异步代码都不持 `Tx`**；
后者由「异步段的参数表里没有 `Tx`」这条构造性事实兜住（`plan_candidates` 是唯一的 `Tx` 收口）。

## 3.5 超时与取消

**截止的来源是值，不是 `ExecutionProfile` 本体。** C §5.3 把「对 async 调用加异步截止」判给调用方，
承载处是 `ExecutionProfile.timeout_ms`（`crates/continuum-graph/src/execution.rs:28`）。
但 `ExecutionProfile` **今天零生产构造点**（§7），故 G **收一个 `Option<Duration>`**，
由驱动从那个字段投影过来——**与 D 收 `ProviderHealth`、收 `BudgetView` 的形状相同**
（D §1.2、§6.1 已为这条路径立了「以值传入」的先例）。

**截止只包住一次调用，不包住流的消费。** 判据：一个长回答本来就要跑很久，分片的节奏由适配器决定；
把截止套在分片消费上，会把「回答长」误判成「调用失败」。**中止一条流走 `cancel(&stream.call)`**，
不走截止——`ModelStream` 的文档已写死这条契约：「取消经 `CallId` 走 `cancel()`，**不经流的 drop**」
（`crates/continuum-core/src/model.rs:89-92`；C §5.3 同）。

**否定式照片两侧对钉**（「丢弃流不是取消」这条只有一侧会被实现者理解反）：
（a）**丢弃流之后 `cancel` 未被调用**（假适配器记录 `cancel` 的 `CallId`，断言记录为空）；
（b）**显式经 G 的中止入口中止时，`cancel` 被调用且 `CallId` 与 `stream.call` 相同**。
只钉 (b) 会让「drop 即取消」的实现全绿——而那种实现**在适配器侧什么也没做**。

**取消的语义是幂等、尽力而为**（C §5.3 定案），故 G 的中止入口不把「这个 `CallId` 已完成」
判成错误——一次取消与一次完成天然竞态。

**一处机制边界（据实记，无照片）**：截止只到「一次调用的建立」。**一个既不来分片、也不结束的流会把
G 挂住**——要它可中止，须先有「流的分片消费也带截止」的机构，而规范没有给判据（§14 第 4 条）。
**这不是本设计没做够，是本阶段的机构边界**（照 B 的 §11 第 18 条的同一写法）。

**超时需要一处本仓今天没有的设施**：工作区的 `tokio` 只开了 `rt-multi-thread` 与 `macros`
（`Cargo.toml:29`），**`time` feature 未开**，全仓也没有一处 `tokio::time` 或 `tokio::spawn`
（实读 `grep -rn "tokio::time\|tokio::spawn" crates/` 零命中）。故本设计**明写这一条前置**：
实现时要打开 `tokio` 的 `time` feature；它是**外部 crate 的 feature**，`ALLOWED` 不受影响
（那张表只断言 workspace 成员之间的边），**是否牵动 `Cargo.lock` 的包集合由实现时实测**，本设计不预判。

## 3.6 G 不做的事（逐条，免得被读成全称的）

| 不做 | 归属 | 出处 |
|---|---|---|
| 记账（预扣 / 结算 / 父子分配 / 判预计超预算） | 语义层 | ENG-005 §三、§五；§8 |
| 候选打分与排序 | 子项目 D | §250 §84 |
| 重试裁决（`decide_retry`） | 执行层（`ExecutionProfile.retry_policy` 的持有者） | §309；`p1-followups.md:62-70`；§6.4 |
| 升级 / 降级的触发 | 同上（见 §6.4 的三条判据） | §251 §86 |
| 路由探索 | 子项目 D 不建（D §7.2），G 也不建 | §27（OPEN-014 / OPEN-008 双阻断） |
| 取模型描述（`list_models` / `describe_model`） | 把工具/模型 schema 交给模型的那条路（Planner） | C §3.3 代价一；§3.2 |
| 写审计行、写执行画像 | 见 §9（两处都没有落点） | §313 §246 |
| 选工具、判工具是否获准 | 子项目 F | 共享面 §四.1 |

---

# 4. 与 C、D 的接缝：一条接口请求（已认领）＋两条判为**不请求**，与三条已定的口径

**本节写在 C 刚落地 Task 1、D 到 Task 9 的时刻。** 每条写明「哪个接口 / 该怎么改 / 为什么 / 不改会怎样」。
**2026-10-06 回扫（实读 C 与 D 的现文、D 的计划、以及 G 的计划）**：§4.2、§4.3 两条**已由 C 落实**
（C 的设计已就地改，并把这处订正的来历留在原地）；§4.1 那条已由 **D 的计划 Task 14 Step 1** 认领
（尚未落地为代码）；§4.6 的两条**判为不请求**（判据在彼处）。**故对 D 的请求最终只有一条。**

## 4.1 请求 D：缺一个「枚举已登记模型」的读函数（**硬缺口；已由 D 的计划 Task 14 Step 1 认领**）

**事实**：D 的持久化面今天有九个公开函数——`p3d_model_migrations` / `register_model` /
`load_lifecycle` / `transition_in_tx` / `save_profile` / `load_profile` / `save_skill_observation` /
`load_skill_vector` / `load_skill_series`（`crates/continuum-model-registry/src/lib.rs:42-45` 的导出清单，
实读相符）。**其中没有任何一个能列出 `model_registry` 表的行**：`load_lifecycle` 与 `load_profile`
都要求调用方**先给一个 id**。

**为什么这是 G 的硬缺口**：§3.2 的候选集**必须**从 `model_registry` 出发，而 G 手里没有 id 清单。

**该怎么改（建议）**：加一个

```rust
/// 已登记的全部模型与它们当前的生命周期状态。**只读、不过滤、不判闸门**——
/// 闸门在 `RoutableModel::try_new` 里，本函数不代它作判。
pub fn list_registered(tx: &Tx<'_>) -> Result<Vec<(ModelId, LifecycleState)>, PersistError>;
```

返回 `(ModelId, LifecycleState)` 而非只返回 id：G 要判闸门就需要状态，而**在同一次读里把两者取出来**，
使「id 与它当时的状态」是同一次快照（分两次读会读到中间态，而中间态没有判据）。
若 D 判「只给 id」，G 仍能工作（多一次逐行的 `load_lifecycle`）——**这一处取舍属 D**。

**判据（为什么是 D 的活，不是 G 的）**：那是 D 的三张表之一。「谁的表谁的读写」是本仓既有的分工
（P3A 的计划第 22 行口径、D 的计划 Task 5 亦按「谁的表谁注册」落）。**G 不能自己写这个 SELECT**。

**不改会怎样**（两条，都不好）：G 只有两条路——(a) 从别处（配置、CLI、规划侧）拿一份模型 id 清单，
于是「哪些模型存在」出现**第二个来源**且与 D 的登记表可以不一致，而 §4.3 那条「没有画像就没有候选」
会退化成「配置里写了的才有候选」；(b) **绕过 D 的接口裸查它的表**——那就是**同一张表的第二个读写点**，
正是本项目一贯判为 Critical 的形状（同一件事两个词汇表/两个产生点）。

**它落得进 D 自己的计划**：D 的 Tasks 10–14 尚未落地（以 2026-10-06 的工作树为准，该分支的提交到
Task 9 的收口为止）。**回扫（2026-10-06，实读 D 的计划）**：**本条已被认领**——
`docs/superpowers/plans/2026-10-05-p3d-model-registry-router.md:1395` 的
**Task 14 Step 1**「补 `list_registered`（G 的设计查出的硬缺口，2026-10-06，本步排在最前）」
给出了与本节同一形状的签名（`:1408`）与一条照片要求（`:1415`：
`list_registered_lists_every_registered_model`——登记两行、两行都在、按 id 升序），
并在 `## 遗留`（`:1589-1600`）记了这一条。**故本设计不再有未落点**；
G 的 plan 只需把「候选集依赖 `list_registered`」写成一条对 D 的依赖（**它尚未落地为代码**）。

## 4.2 请求 C：`model_providers()` 在 G 这条路径上没有消费方——**已落实（2026-10-06）**

**事实**：C 的 `ProviderRegistry` 原先开过一个 `model_providers() -> Vec<Arc<dyn ModelProvider>>`，
C §3.1 说它「供调用方（子项目 G）枚举全部模型适配器」（当时在
`…p3c-provider-boundary-design.md:140-141`；现文已改）。

**G 不用它**，判据是**返回类型里丢掉的那一项**：G 的候选集**按 id 索引**（来自 D 的表），
而那个方法交出的是**按登记顺序的适配器列表**、**不带 id**——「这个适配器服务哪些模型」这个信息在
返回类型里被丢掉了，故 G 拿它无从与候选集对齐。G 走的是 `model_for(&id)`（逐个解析），它在两边都有 id。

**处置（C 的现文，实读）**：**该入口已裁删**——`…p3c-provider-boundary-design.md:147`
（「**不提供「枚举全部模型适配器」的入口**（`model_providers()` 已裁删，见下）」）与 `:153-165`
（三条备选与判据，选定「删」，并把「零消费方」「形状对不上：不带 id」两条原样记了进去）。
C §6 第 4 步亦据此写明「**id 来自 D 的 Model Registry 表**（模型清单的权威），**不是**从本注册表枚举来的」
（`:469-471`），C §11 的对应用例行随之删除（`:806`）。**本设计据此把 §11 里「G 不调 `list_models()`」
那一条保持不变**（它与本请求无关：`list_models` 是适配器的方法，不是注册表的入口）。

**代价（照 C 记的）**：将来若有人真需要「跨全部适配器聚合」，他得重新提出这个入口并自带消费方与形状。
**本设计不再有未落点。**

## 4.3 请求 C：§6 第 2 步把「取描述」与「取可用性」混称——**已落实（2026-10-06）**

**事实**：C §6 第 2 步原写「对每个适配器 `list_models()`，或对已知 id 用 `describe_model(&ModelId)`；
得到 `ModelDescriptor`……**这一步的产物（可用性）随后作为值交给 D 的 Router**」
（当时在 `…p3c-provider-boundary-design.md:441-443`；现文已拆开）。

**那是两个方法、两个类型、两件事**：描述是 `ModelDescriptor`（`crates/continuum-core/src/model.rs:21-27`），
可用性是 `ProviderHealth`（`:83-87`），取可用性的方法只有 `health()`（`crates/continuum-provider/src/model.rs:21`）。
`list_models()` **不返回任何健康度**。

**处置（C 的现文，实读）**：**已拆成第 1、2 两步**——第 1 步「取描述」（`:462-464`）、第 2 步
「**取可用性**：`health()` → `ProviderHealth`。**「当前可用性」就是它**……**不是**第 1 步的
`ModelDescriptor`——这两步的产物是两样东西，实现者不得拿第 1 步的产物当可用性（本节初稿把两者混称成一步，
已拆开，2026-10-06）」（`:465-467`）。**本设计据此照办**：G 用 `health()` 取快照（§3.3），
**不调 `list_models()` / `describe_model()`**（§3.2 末条，有否定式照片）。

## 4.4 已由 D 写死、G 照办的两条

1. **`availability` 必须覆盖候选集**：候选集里的模型在 `availability` 里没有条目 →
   `Err(RoutingError::UnknownAvailability { id })`（D §5.3）。G 逐候选取快照，故**天然全覆盖**（§3.3）；
   这条对 G 是一条**构造性事实**（见 §4.5 的可达性）。
2. **只过滤 `Unavailable`**：`Healthy` / `Degraded` 都进候选集，且**两者之间没有判据**（D §5.3），
   `Degraded` 原样带进 `reason`。G **不因健康度做任何二次裁剪**——那会是在 D 已写死的地方加第二个判据。

## 4.5 D 的 `RoutingError` 在 G 这条路径上的可达性（逐变体）
D 的 `RoutingError` 有四枚（D §5.4）：`NotRoutable` / `NoEligibleCandidate` / `DuplicateModelCandidate` /
`UnknownAvailability`。**G 是它的第一个生产消费方，故逐变体判「从 G 的路径到得了吗」**：

| 变体 | 从 G 的路径 | 判据 |
|---|---|---|
| `NotRoutable { state }` | **不可达** | 它由 `RoutableModel::try_new` 产出，而 G **只对已构造成功的候选**调 `rank`（§3.2 的合取 (b)） |
| `NoEligibleCandidate` | **可达**，且是主要失败路径之一 | 候选集为空（无画像 / 无适配器 / 一个都没登记）、或全部被 `Unavailable` 滤掉 |
| `DuplicateModelCandidate { id }` | **不可达** | 候选集逐行来自 `model_registry`，`id` 是该表的**主键**（D §3.1 的建表语句），故同 id 不可能出现两次 |
| `UnknownAvailability { id }` | **不可达** | §3.3 逐候选取快照，条目集与候选集**同一次构造**（§11 有一条断言钉这个对应） |

**「不可达」不等于「该删」**：这三枚是 **D 的类型上的守门人**（`DuplicateModelCandidate` 是 D 的
`compare` 全序断言的守门人，D §5.3；`UnknownAvailability` 是 D 侧对「按未知放行」的 fail-closed 处置，
别的调用方可以传一份不全的列表）。**G 这边不重复钉它们的逐变体用例**（D 已有），
G 钉的是**上表这三处不可达性**——它们各有照片，写在 §11（一条钉候选集与快照的一一对应，
一条钉候选集逐行来自主键表，一条钉候选集的元素类型是 `RoutableModel`）。

## 4.6 两条**判为不请求**的（逐条给判据，免得被读成漏提）

**（一）`rank` 的第二个入参 `&[RoutableModel]` 不改。**

- **问的是**：G 手里是「候选 + 句柄」的成对值，而 `rank` 要 `&[RoutableModel]`——**要不要请 D 改成
  收成对值**（例如 `&[(RoutableModel, Arc<dyn ModelProvider>)]`）？
- **判：不改。** 三条判据：
  1. **那会把调用侧的实现细节推进排序层**。`rank` 是**纯函数**（D §8.1「Router 的纯由签名保证」），
     它不接 `Tx`、不持有 `CallId`、不消费 `ModelStream`；把「适配器句柄」写进它的入参，
     就是让排序层知道适配器存在——与 D §1.2 那条「**判断**在本层、**执行**在 G」的分界正面冲突。
  2. **`RoutableModel` 的语义正是「一个可交给 Router 的模型」**（D §4.2），它不该长出一条与执行有关的边。
  3. **G 侧的解不贵**（这就是答「能不能不改 D 就解决」）：`select` 内部把 `Vec<Candidate>`
     解构成 `Vec<RoutableModel>`（正是 `rank` 要的那个）与一枚句柄表，`rank` 之后**按 `ModelId`
     复原**配对——`ModelId` 是唯一键，两个来源已在 §3.1 第 6 条写明。**代价是一次解构 + 一次按 id 配对**，
     而收益是 D 的排序层不认识适配器。
- **不改会怎样**：G 的 `select` 里多两步（解构、按 id 配对），**没有别的后果**；而改的后果是
  上一段的三条。

**（二）不向 D 要「画像的播种入口」或测试用的构造口。**

- **问的是**：G 的测试要拿候选，而 `ModelProfile` crate 外造不出、`save_profile` 又收一枚画像
  ⇒ 只能裸 SQL 播行 ⇒ **G 的测试里嵌 D 的表结构**。要不要请 D 开一个口？
- **判：不要，接受耦合。** 判据与「接受的是什么」写在 §11 的前置二（三条替代全部更坏，
  其中第一条**正**是 D §2.2 要挡的东西）。
- **不改会怎样**：G 的测试知道 D 的列名，D 改掉那些列时 G 红——**这就是护栏的用意**。

**故对 D 的请求最终是 1 条**：§4.1 的 `list_registered`（已由 D 的计划 Task 14 Step 1 认领）。
C 侧 2 条已落实（§4.2、§4.3）。**本节的两条「不请求」也是判断，不是沉默**——它们的判据在上面，
收件人是「无（已判）」。

---

# 5. `usage()` 的语义：两种读法都不成立

**裁决把「定义 `usage()` 的语义」判给 G**（set-decisions §一.1），C 的设计把它记成
「语义未定义且无消费方……**收件人：子项目 G**」（C §12 第 3 条，`…p3c-provider-boundary-design.md:846-849`）。
**本设计对它的处置是：G 不定义它，也不用它。** 这不是推诿，是两种读法各自不成立：

- **读作「累计」**：它就成了同一事实的**第二个产生点**——`InvokeResponse.usage` 是**逐次**的
  （`crates/continuum-core/src/model.rs:51-61`），累计量由求和可得；一个无参的 `usage()` 与「逐次返回值之和」
  是同一个量的两种来源，而适配器**自报的累计**与**客户端求和**可以不一致且无从核对。
  本项目对「同一件事两个产生点」的处置一贯是消除其中一处。
- **读作「本会话」**：**「会话」在 §315 里没有定义**——谁开、谁关、跨不跨进程重启，规范一个字都没给。
  在一个没有定义的边界上定语义，就是发明。

**G 实际依赖的是逐次的那个**：一次调用的用量从它自己的返回值里取（`InvokeResponse.usage`）。
它是**与那次调用同一物证**的量，不需要与任何「累计」对账。

**照片是否定式的，可以写出来**（本仓接受否定式照片，`p3bcdf-followups.md` §四.4）：假适配器的
`usage()` 返回 `Err(ProviderError::Unavailable(…))`，而 G 的正常路径**照过**——
这就钉住了「G 的路径不调 `usage()`」。同理钉 `list_models()` / `describe_model()`（§3.2 末条），
三条共用一个假适配器（§11）。

**收件人（§14 第 5 条）**：规范维护者 + 将来要做对账的那一方（语义层）。`usage()` 是 §315 的冻结方法，
**删不掉，只能定或标为无消费方**；定它的前提是先有一个「会话」或「累计」的规范定义。

---

# 6. 模型侧失败分类

## 6.1 分类表（五个变体逐项）

`ProviderError` 有五个变体（`crates/continuum-core/src/error.rs:3-15`）。`FailureClass` 有七个
（`crates/continuum-graph/src/failure.rs:8-16`）。G 这一侧的分类写死如下：

| `ProviderError` | `FailureClass` | 判据 |
|---|---|---|
| `Transport(_)` | `Transient` | 传输失败通常是连接抖动；适配器**若明知是永久错，应改报 `Protocol`**（C §5.1 的同一句） |
| `Unavailable(_)` | `Transient` | 供应商侧暂时不可用。**注意它与 `ProviderHealth::Unavailable` 是两个类型**（一个是调用失败、一个是探活读数），此处只是两者的方向一致；D 挡候选用的是后者（D §5.3） |
| `Protocol(_)` | `Permanent` | 契约不符，重试无益 |
| `UnknownModel(_)` | `Permanent` | 配置缺陷（登记表里的 id 与适配器对不上，C §3.3 代价一的那一格） |
| `Cancelled(_)` | **不进入分类** | **它是调用方自己发的取消，不是失败**（C §5.1 的同一条判据） |

**这条落到 G 的错误类型上**（`Cancelled` 不落在「失败」那一枚里，以免一次正常中止被记成一次失败）：

```rust
pub enum ModelCallError {
    /// D 的排序失败。**带出 D 的错，不压平成一枚同名的变体**——那会是同一件事两个类型
    /// （本仓一贯判为缺陷的形状）。从 G 这条路径到得了的只有 `NoEligibleCandidate`，
    /// 其余三枚的可达性逐条记在 §4.5。
    Routing(RoutingError),
    /// D 的表读失败。
    Storage(PersistError),
    /// 这一次调用在截止内没有完成（§3.5）。
    Deadline { elapsed_ms: u64 },
    /// 适配器调用失败，**带出它的类别**。
    Provider { class: FailureClass, source: ProviderError },
    /// 调用方发起的取消（来源是 `ProviderError::Cancelled`）。**它不是失败，故不带类别**。
    Cancelled { source: ProviderError },
}
```

**逐变体有照片**（§11）：四个「失败」来源各断言是哪一枚 `class`，`Cancelled` 断言走 `Cancelled`
那一枚**且不带 `class`**。

## 6.2 这张表不是对适配器的断言

C §5.1 拒绝把这张映射写成**边界上的**函数，理由之一是「映射在每条路径上是否为真，本子项目答不出：
`Transport` 在一个适配器上可能是瞬时的，在另一个上可能是永久的」。**那条理由在这里同样成立**，
故本设计把它写准：**这是 G 这一侧的默认口径，不是对适配器的断言**；适配器要对某次失败改口径，
它应当在 `ProviderError` 的取值上表达（明知永久就报 `Protocol`）。

**一处已记在 C 侧的反例**（G 的分类表在模型路径上带着它）：既有夹具把**永久性配置缺陷**
（「这个 provider 没有这个工具」）报成 `Unavailable`（`crates/continuum-provider/tests/fake_provider.rs:106`，
C §5.1 记录了它；**订正 2026-10-07（G Task 1 实测，原引坐标照留）**：该坐标**已不成立**——工具侧夹具已搬走，
`fake_provider.rs` 现存 104 行、只剩 `FakeConnector`，实读的新坐标（p3g 树 `404c464`）是
**`crates/continuum-provider/tests/common/mod.rs:177`**）——若模型侧的真实适配器也这样用 `Unavailable`，那张表会把一次配置缺陷分成 `Transient`。
**这不是本设计能修的**（修它在适配器），故它在此处是一条**已知的射程边界**，不是漏项。

## 6.3 产物今天没有消费方——据实记，不声称它驱动了重试

**实读**：`decide_retry`（`crates/continuum-graph/src/failure.rs:106`）在全仓**零生产调用点**
（`grep -rn decide_retry crates/` 除定义处与 `crates/continuum-graph/tests/failure.rs` 外零命中），
而 D 的 `FailureHistory`（§250 的第六个输入）**形状未定**、落在策略一侧（D §5.1 的表、D §11 第 18 条）。

**故这条要说清三句**：

1. G 产出的是**逐次的类别**，不是历史（**这是 D §11 第 18 条签收的那一半**：分类发生在持有
   `ProviderError` 的一侧，即 G）；
2. **从逐次类别到 `FailureHistory` 的累积器，在 B/C/D/F 四份设计与本设计里都没有归属**——
   D 只定义了「策略要读一个历史」，没定义历史长什么样，也没定义谁写它；
3. 故 `class` 今天**没有一个按它分支的消费方**。**「有产生方、无消费方」不是漏做**，与 D 的
   `next_step`（「照片只能是对纯函数的直接调用，钉不了驱动真的升级了」，D §10 第 3 条）同形；
   §11 的照片钉的是**映射本身**（表驱动、逐变体），不是「它驱动了重试」。

记在 §14 第 6 条。

## 6.4 G 不做重试裁决，也不做升级触发

D 的设计把「**失败检测与升级触发**」写成「那是**子项目 G**的活」（D §7.1）。**本设计收一半、退一半**：

- **收「失败检测/分类」**（本节的表）：它是「持有 `ProviderError` 的一侧」的活，而那一侧是 G。
- **退「升级触发」**，三条判据：
  1. **§251 的前提是「仍满足 Contract 和 Budget」**——Contract 属语义层、Budget 的判定归
     Budget Validator（ENG-005 §二第 1 条、§五）。**这两样都不是 G 的词汇**，G 手里没有它们，
     故它判不出前提。
  2. **失败裁决者只能有一个。** `decide_retry`（§309 的固有归属）要一枚 `&Operator` 与一枚
     `&RetryPolicy`，两者都在执行层。若 G 也判一次，就是「同一个失败两个裁决者」——
     正是 ENG-005 被否掉的 (b) 方向与 D §7.1 末段自己的理由。
  3. **D 已经把这条订正过一次**：`p1-followups.md:62-70` 那句「**P3 的 Router** 必须为 RESOURCE 给出
     `max_attempts >= 2` 与退避参数」被订正为「`RetryPolicy` 属 `ExecutionProfile`（执行层），
     配置它的是构造 `ExecutionProfile` 的那一方」。**同一条订正适用于「升级触发」**：它属执行层，
     不属产出类别的那一方。

**故本设计的边界一句**：**分类在 G、重试策略在执行层、裁决由 `decide_retry`。**
G 只把类别交出去；「同一条路径再 `rank` 一次」是执行层在拿到 `Escalate` 之后的**第二次调用**，
走的是**同一个 G 入口**（D §7.1 已把 intra-task scaling 判为「同一入口的第二次调用」，本设计与之一致）。

---

# 7. `ExecutionProfile` 的模型侧两个字段：定口径，不落实现

**裁决把「收紧 `ExecutionProfile` 的字段」判给 G**（set-decisions §一.1）。C 的设计把它记成
「收件人：子项目 G（谁先发 `invoke` 谁收紧）或 F」（C §2.2 五 `:120-123` — 该段另有一条 2026-10-06 的
订正 `:123-130`，内容与本节的实读一致；C §12 第 9 条现为 `:872-878`）。
**本设计收一半，但它不是一个可以落成实现的 task——下面是实读出来的四条事实，每条都改变结论。**

1. `ExecutionProfile`（`crates/continuum-graph/src/execution.rs:20-33`）的 `model` / `provider` / `tool`
   是 `Option<String>`（`:21-23`）。
2. **它今天零生产构造点**：全仓除定义处与 `crates/continuum-graph/tests/execution.rs` 外，
   `ExecutionProfile` 零命中（实读 `grep -rn ExecutionProfile crates/`）；驱动的 `task` 子命令跑的是
   一条**命令**，没有节点执行循环，也没有一处读写 `node_attempt` 之外的执行画像。
3. **`execution_profile` 表没有这三列**：建表语句只有 `graph_id / node_id / attempt / backend /
   timeout_ms / retry_policy / cost_budget`（`crates/continuum-graph/src/persist.rs:53-62`），
   而且**这张表没有读写函数**（同文件内零命中）。**故 C 设计里那句「要动一张已落库的表」据实订正**：
   那张表里根本没有这三个字段，收紧**不产生迁移成本**——但也意味着**收紧之后这三列仍然不被记录**。
4. **`continuum-graph` 的 `ALLOWED` 条目里没有 `continuum-core`**（`crates/continuum-runtime/tests/dependency_direction.rs:41-49`），
   而 `ModelId` / `ToolId` 都在 `continuum-core`；收紧要加一条**新边**。

**本设计的口径（三句，逐字段）**：

- **`model` 归 G、应收紧为 `continuum_core::model::ModelId`**，且它的值**只有一个来源**：
  这一次调用实际用的那个 id——即 `ExecutionCandidate::model()`（D §5.2）。
- **`provider` 归 G，但保持 `String`、不收紧了**。判据：**没有 provider id 这个类型**——
  §315 的 `ModelDescriptor.provider` 就是 `String`（`crates/continuum-core/src/model.rs:23`），
  造一个 `ProviderId` 就是发明一个规范与代码都没有的类型（「不预先发明」）。
- **`tool` 归 F**（工具调用路径），**G 不接**。判据：这正是「两条路径各自的画像字段」这条分工
  （C §12 第 9 条写的「G 或 F」在这里被切开）。

**为什么不落实现（缺的是哪一步，写清）**：**缺的是「节点执行时装配 `ExecutionProfile` 并调用 G 的那一步」**
——那个装配点今天不存在（事实 2），而它**不在 G 的交付面里**：G 是模型调用路径，
不是节点执行的装配点。**一条可写的照片也没有**：没有构造点，改类型不产生任何可观察效果。
**且它对 G 的运行路径零影响**——G 的发起面走的是 `InvokeRequest`（内带 `ModelId`），
**不经过 `ExecutionProfile`**（它只从那里取 `timeout_ms`，而那是**以值**传进来的，§3.5）。

**收件人（§14 第 7 条）**：构造 `ExecutionProfile` 的那一方（驱动侧的节点执行装配点）＋
`continuum-graph`（类型）＋ 规范维护者（§246 的 `model` / `provider` 要不要落库，规范未给判据）。

---

# 8. 预算：G 不记账，只提供读数

## 8.1 G 对 `BudgetView` 的全部动作是「搬运」

ENG-005 的归属是逐条列明的：**预扣 / 结算 / 父子分配 / 判「预计超预算」全归语义层**
（`docs/superpowers/specs/2026-10-05-eng-005-budget-accounting.md` §三、§五、§六.2 的表）。
D 的 `BudgetView` 是**只读投影**，且**投影由驱动做**（D §6.1：语义层产出它自己的 `Budget`，
驱动投影成 `BudgetView` 传进 `RoutingRequest`；这样任何时刻都不出现 `语义层 → 资源层`）。

**G 在这一串里的位置是最后一段：它把驱动投影出来的那份 `BudgetView` 按值装进 `RoutingRequest`，
交给 D 的策略。它不解释单位、不换算、不记账、不做减法。** 判据：§333 的五个量纲**没有单位**
（D §6.1、§11 第 5 条），而**用一份没有单位的余量去做减法**是发明。G 也因此**不产生**
`FailureClass::Constraint`（ENG-005 §三.2 把「运行期实际超支」归 `Constraint` ＋
`EscalationPolicy::Decision`，而判超支要预算树）。

## 8.2 `None` ≠ `Some(0)`：G 侧的两向对钉

D 的计划把这条记成遗留并**点名了 G**：「`None`（该量纲当前不构成约束）与 `Some(0)`（额度为零）
不得混同，而 `BudgetView` 只保证**存得下**这个区别，不保证没人把它抹掉——折叠发生在**驱动侧的投影**里，
本 crate 里没有可施加该变异的实现体……**收件人：驱动侧的投影实现与子项目 G**」
（`docs/superpowers/plans/2026-10-05-p3d-model-registry-router.md:1576-1591`）。

**G 接这一条，并把它做成两向对钉的照片**（「守卫须两侧都钉；缺的那侧往往是 fail-open 的那侧」——
这里 fail-open 的是**向二的反面**：把 `Some(0)` 读成 `None` 就是**把「额度为零」读成「不构成约束」**，
于是路由会在额度耗尽时照常花钱，而它在结果上**看不出来**；反向的 `None → Some(0)` 是 fail-closed
（凭空没有候选），错得刺眼、容易发现）：

- **向一**：传入 `budget.money = None` → G 交给策略的那份请求里 `money` **仍是 `None`**。
- **向二**：传入 `budget.token = Some(0)` → 它**仍是 `Some(0)`**（不被折成 `None`）。

**照片的构造**：G 的测试里实现一份**记录用的 `RankingPolicy`**（D 的公开 trait，`evaluate` 收
`&RoutingRequest`），把收到的那五个 `Option<i64>` 抄进自己的记录；断言逐字段相等。
**`Option<i64>` 是 `Copy`，故这条照片不需要 `BudgetView` 派生 `Clone` 或 `PartialEq`**——
D §6.1 的「没有当场消费方的派生一律不加」在这条上不必改（值被移进 `RoutingRequest`，
策略从 `&RoutingRequest` 抄 `Copy` 字段）。**故本设计不向 D 提这条请求。**

## 8.3 流式调用没有用量读数（§315 的类型上限）

`StreamChunk` 只有 `delta` 与 `done` 两个字段（`crates/continuum-core/src/model.rs:64-67`），
**没有 usage**。故**一次流式调用的用量在 §315 的类型上拿不到**——而流式是大模型调用的常态。

**G 的处置**：不编一个用量、也不在 `done` 那一帧假造一个。**这一条对记账方的影响要写明**：
§333 的五个量纲里，模型调用能观测到的只有 token 一项（`Usage { input_tokens, output_tokens }`，
`:51-54`），而**流式路径连这一项都没有**。故「按实际结算」在流式调用上**没有读数来源**。
**这是本阶段的能力边界，不是本设计没做够**（要它有，得先用规范定「流的用量怎么承载」）。
记在 §14 第 8 条（收件人：语义层设计 + 规范维护者）。

---

# 9. 记录面：模型调用没有落点（两处都空）

**这一节是本设计对「这条路径留下什么痕迹」的完整回答。两处都是空的，且两处都不是 G 能填的。**

1. **审计行：§313 的必录清单里没有这一类。** `AuditKind` 的八个变体是
   `AuthorityChanges / DeviceJoin / DeviceRevoke / CapabilityGrants / ExternalEffects /
   ContractChanges / UserApprovals / RuntimeSelfUpdate`（`crates/continuum-events/src/audit.rs:18-35`），
   **没有「模型调用」也没有「路由决定」**。D 已经为同一件事记过一次（D §8.2：它据此**不登记**
   `continuum-events` 这条边）。**G 同样不新增变体**——那等于扩 §313 的必录清单，是规范的事。
2. **执行画像：那三列不存在。** §7 的事实 3：`execution_profile` 表没有 `model` / `provider` / `tool`
   三列，也没有读写函数。故「这一次执行用了哪个模型」在库里**无处记录**。

**G 不写 Effect Journal 的理由**（与 §2.1 的强制点 (2) 那条配套）：一次模型调用不写外部世界，
它不是 §268 意义上的外部效应。**G 的失败路径也不写库**——它的库动作只有同步段那一次读事务。

**照片是否定式的，可以写出来**：跑通 G 的路径（含一条失败路径）后，断言
`audit_log` 与 `node_attempt` 的行数**不变**（形状取自 F 的 P-19 与 B 的「不写审计」照片，
`p3bcdf-followups.md` §四.4 明写否定式照片在本仓被接受）。**这一条同时是「G 不写半写副作用」的照片**：
G 没有可半写的副作用，而这条断言把它钉成事实而不是声明。

---

# 10. 依赖、crate 划分与中立性

## 10.1 不新建 crate，落在 `continuum-runtime` 的库里

**建议**：G 的实现落在 `crates/continuum-runtime/src/model_call.rs`（与 F 的
`crates/continuum-runtime/src/tool_call.rs` 同址），错误类型增补进 F 已建的
`crates/continuum-runtime/src/error.rs`。

**判据三条**：

1. **G 是 Runtime 的一条路径**（§1.2，总纲 §1）。F 的归属与它同源、同址。
2. **`continuum-runtime` 的 `ALLOWED` 条目已含 G 需要的全部 crate**（实读
   `crates/continuum-runtime/tests/dependency_direction.rs:164-179`）：
   `continuum-provider`（注册表与适配器句柄）、`continuum-model-registry`（候选集与 `rank`）、
   `continuum-persist`（`Tx`）、`continuum-core`（`ModelId` / `InvokeRequest` / `ProviderError` /
   `ProviderHealth`）、`continuum-graph`（`FailureClass`）。
   **故 G 不新增任何依赖边**（`ALLOWED` 一字不改，`Cargo.toml` 的 `[dependencies]` 也不加）。
   新建一个 crate 只会给每一条边再加一层搬运，换不来隔离。
3. **它使两条「悬空边」变成真使用点**：C §12 第 2 条记「`continuum-runtime → continuum-provider`
   是悬空边（`Cargo.toml` 声明了、引用为零），**收件人：F**」；本设计记明**G 是这条边的第二个使用点**
   （模型侧），F 是第一个（工具侧）。同理 `runtime → core` 与 `runtime → model-registry` 也各自有真使用点。
   **边由用它的那个 task 登记**（P2b 的规矩）——F 先落地就由 F 登记，G 落地时**只核不改**。

## 10.2 两处共享文件要登记（不是新增边）——**其中一处依赖 F**

`crates/continuum-runtime/src/error.rs` 与 `tests/dependency_direction.rs` 是**两个子项目都写**的文件
（与 §七.7 记的「多写者单文件」同形）：G 向 `error.rs` 增补 `ModelCallError`，并且**不新增 `ALLOWED`
条目**（故那张表 G 一处都不动）。**G 的 plan 要把这两件事各自登记自己那几条**，不设集中登记 task。

**订正（2026-10-06，G 的计划据实报出；原话照留、加此注记）**：本段初稿写「（F 已在那里放了 `TaskError`
与 `ToolReportedError`）」——**那个「已」是假的**。实读：`crates/continuum-runtime/src/error.rs` 与
`src/tool_call.rs` **今天都不存在**（F 尚未落地；F 的计划 `docs/superpowers/plans/2026-10-05-p3f-tool-call-path.md:307-308`
才要建它们，且它的 Task 2 还要把 `TaskError` 整块从 bin 搬进 lib）。

**故 G 的实际前置是三个 crate 的流：C、D、F**，而 §4 与本节只把前置写成 C 与 D。
**落点与收件人（写清缺的是哪一步）**：

- **缺的那一步**：F 的 Task 2 建出 `crates/continuum-runtime/src/error.rs` 并把 `TaskError` 从
  `task_cmd.rs` 原样搬进去（`docs/superpowers/plans/2026-10-05-p3f-tool-call-path.md:307` 的 `Create` 行、
  `:158` 的文件表那一行：「新建（lib）：`TaskError` 从 task_cmd.rs 原样搬来」）。
- **G 不代 F 建那个文件**：同一个文件有两个创建者，而 F 还要整块搬入 `TaskError`——两批改动撞在
  同一个文件上（§七.7 的「多写者单文件」）。
- **收件人：协调者**（排期：G 的 Task 1 必须排在 F 的 Task 2 之后），记在 §14 第 17 条。

## 10.3 中立性：G 侧的三条

| C 的条目 | 在 G 上的落点 |
|---|---|
| §4.1 表：调用方只依赖 `continuum-provider`，不依赖 `continuum-adapter-*` | G 只命名 `Arc<dyn ModelProvider>`（由 `model_for` 交出）；**G 的类型签名里不出现任何适配器类型** |
| §4.2 形态 3「调用方持有实现」 | **G 就是那个调用方**，而这条**仍然没有照片**：今天一个适配器 crate 都不存在，没有可被引用的实现类型。**G 落地把这条从「没有调用方」变成「有调用方但没有适配器」**——据实记，不声称它有了守卫 |
| §4.2 形态 4「实现被写进中立 crate 内部」 | 不涉 G（守卫在 C 的 `src/` 与 C 的 Task 8） |

**一条 G 侧的守卫（可写，且它的逃逸面要一并写明）**：断言
`crates/continuum-runtime/src/model_call.rs` 的源码文本里不出现任何
`continuum-adapter`、`DeepSeek`、`OpenAi`、`Anthropic` 之类的实现 crate / 实现类型名。
**它的证明力有边界**：匹配的是**字面拼法**——与 C §4.1 末段那条同形，**是下界不是封闭判定**。
**（订正 2026-10-08，G Task 4 实测：本句原写「别名与全限定路径逃逸」，对这一类单 token needle 为假
——它们必须写出原拼法，故命中。见 §1.3 那条订正。）** **更硬的一层其实已经有了**：若 G 真想持有一个实现类型，它必须依赖那个
crate，而 `every_crate_depends_only_on_its_allowed_set` 会因为 runtime 的条目里没有它而红
（**订正 2026-10-08，G Task 4 实测：原文引 `dependency_direction.rs:246` 与逐对断言 `:276-281`；
现为 `:255`（用例）与 `:285-289`（逐对断言），runtime 条目在 `:172-189`**——
**本计划与设计里凡是给这个文件的行号，都要按当时实测重取**）。

---

# 11. 测试策略

**前置一：G 的用例需要一枚自己的假模型适配器。** 它实现 `ModelProvider` 七个方法，可配置 `health()`
的返回值、可记录 `invoke` / `stream` / `cancel` / `usage` 的调用。**照既有裁定，这是 C 的
`tests/common/mod.rs` 里那个 `FakeModel` 的第二份副本，两份不合并**（`p3bcdf-followups.md` §七.8：
「两份各有其用」）。它需要 `continuum-runtime` 的 **dev 依赖** `async-trait` 与 `futures-core`
（后者用于自写单分片流；判据同 `crates/continuum-provider/tests/fake_provider.rs:17-19` 的注释：
`stream::iter` 属 `futures-util`，本仓不用它；**订正 2026-10-07（G Task 1 实测，原引坐标照留）**：
`fake_provider.rs:17-19` **已不成立**（该区间现存 `FakeConnector::descriptor` 的函数体，不是注释），
那条注释随 `OnceStream` 搬进了 **`crates/continuum-provider/tests/common/mod.rs:31-32`**，实读，p3g 树 `404c464`）。
**订正**：本行初稿写「`async-trait`（F 已按 F8 加）」
——**那个「已」也是假的**（同 §10.2）：F 尚未落地，故那两条 dev 边的登记方是**先落地的那一方**；
G 若先落地就自己登记，F 若先落地就核一遍。

**前置二：G 的夹具必须自己播 D 的画像行，故 G 的测试里嵌着 D 的表结构。** 这条链**不能自举**：
`ModelProfile` 在 crate 外**构造不出来**（字段私有、`try_new` 是 `pub(crate)`，
`crates/continuum-model-registry/src/profile.rs:387`/`:415`），而 `save_profile` **收一枚 `ModelProfile`**
⇒ 第一枚画像只能由**裸 SQL** 播下（D 自己的夹具也是这个形状，D 的计划 Task 11 明写
「第一枚画像必须由裸 SQL 播下」，`docs/superpowers/plans/2026-10-05-p3d-model-registry-router.md:1092`）。

**判：接受这条耦合，不向 D 请求播种入口。** 要写明「接受的是什么」：

- **接受的是**：G 的测试源码里出现 D 的列名（`model_registry` 的 `id` / `lifecycle_state`；
  `model_profile` 的十二列与 `modalities` / `tools` / `failure_modes` 的 JSON 容器编码）。
- **它是有意的耦合，同时是一条护栏**：D 改列名或改编码 → **G 的夹具红**，而**不是静默失配**。
  这正是想要的：G 的候选集是 D 的表的下游，两者不一致必须看得见（与 §3.2 那条「三个来源」的判据同源）。
- **它的射程要写准**：**加「可空或带默认值」的列不会红**（G 的夹具按列名写、不 `SELECT *`，也不断言列数）；
  **只有改列名与改编码会红**。所以它不是「D 一改表 G 就红」的全称，而是「**D 改掉 G 依赖的那些列**才红」。
  **订正（2026-10-06，复核指出；原话的来历留在此句）**：本行初稿写的是「**加列不会红**」——
  **射程宽了一格**：**加一列 `NOT NULL` 且无默认值**的列时，G 按列名写的 `INSERT` **照样红**
  （那一列要值而 INSERT 没给）。决定红不红的是**那一列要不要值**，不是「按不按列名写」。
  **这正是「断言的作用域要与事实同宽」的又一个实例**：措辞说的是「按列名写所以不惧加列」，
  而事实是「新增的列若必填，G 的 INSERT 就缺一个值」。
- **不接受的替代，以及它们为什么更坏**：(i) 给 `ModelProfile` 开一个 crate 外的构造口（`pub` 的
  `try_new` 或 `#[cfg(test)]` 的跨 crate 口）——那**正**是 D §2.2 那条保证要挡的东西
  （「没有画像就没有候选」的第二条腿），**测试用的口子也是口子**；(ii) 把 G 的用例放进 D 的 crate
  ——G 是 runtime 的路径，放错了地方，且会让 D 的 crate 依赖 G 的夹具；
  (iii) 让 D 提供一个公开的「播种」函数——**那是给一个只在测试里存在的需求开生产接口**。
- **收件人：D 的实现者**（表结构的变更须通知 G）＋ **G 的实现者**。记在 §14 第 18 条。

| 验什么 | 怎么验 |
|---|---|
| 候选集的三条合取（§3.2） | 四种输入各一条：有画像且过闸门 → 进；**无画像** → 不进；**未过闸门**（`stale`）→ 不进；**无适配器** → 不进 |
| 「无适配器即不是候选」两侧对钉（§3.2） | 向一：只在 D 的表里 → 不进候选集（若它是唯一候选 → `NoEligibleCandidate`）；向二：**同一条 `registry.register_model` 之后它回到候选集**并被选中 |
| G 不调 `list_models()`（§3.2 末条） | 假适配器的 `list_models()` 返回一个**不在 D 的表里**的 id，或直接返 `Err` → 候选集不变。**否定式照片** |
| G 不调 `usage()` / `describe_model()`（§5） | 这两个方法返 `Err(ProviderError::Unavailable(…))` → 正常路径照过。**否定式照片，三条共用一个假适配器** |
| 快照与候选集一一对应（§3.3） | 断言交给 `rank` 的 `availability` 的 id 集合 **== 候选集的 id 集合**（这就是 `UnknownAvailability` 在 G 路径上不可达的那条构造性断言，§4.5） |
| 句柄与候选**按 id 配对**（§3.1 第 6 条） | 三个候选、三个**互不相同**的适配器 → `CallPlan::selected()` 给出的那对**是同一个模型的那一对**，逐项断言 `candidate.model() == 该适配器服务的那一个 id`；`alternatives()` 同法逐项。**红的条件**：把句柄表按候选集的**输入次序**配回去（而不是按 id）→ 排序之后次序变了，红 |
| 可用性真的通到排序（§3.3） | 假适配器 `health()` 返 `Unavailable` → 该模型不被选中；**两侧对钉**：改回 `Healthy` → 它回到输出 |
| 探活次数（§3.3） | 三个候选 → `health()` 被问**三次**（计数夹具）；钉「不跳着问」 |
| `Tx` 不跨 `await`（§3.4） | 编译期：`assert_send(&future)`；**若实现把 `Tx` 带进异步段，编译不过** |
| 截止（§3.5） | **两侧对钉**：向一：假适配器的 `invoke` 永不返回 → 带 `Some(短截止)` 的调用返回 `Deadline { .. }`；向二：同一个适配器改成「略早于截止返回」→ **成功**（钉住截止不会无故触发）。只钉向一时，一个「永远返回 `Deadline`」的实现全绿 |
| 「丢弃流不是取消」的两侧（§3.5） | 向一：丢弃 `ModelStream` 之后 `cancel` **未被调用**；向二：经 G 的中止入口 → `cancel` 被调用，且 `CallId` == `stream.call` |
| 失败分类逐变体（§6.1） | `Transport` / `Unavailable` / `Protocol` / `UnknownModel` 各一条断言 `class`；`Cancelled` 一条断言走 `Cancelled` 那一枚**且不带 `class`** |
| `BudgetView` 的 `None` ≠ `Some(0)` 两向（§8.2） | 记录用的 `RankingPolicy` 抄下收到的五个 `Option<i64>`：`None` 仍是 `None`；`Some(0)` 仍是 `Some(0)` |
| 失败路径不写库（§9） | 跑通成功与失败两条路径 → `audit_log` / `node_attempt` 行数**不变** |
| `NoEligibleCandidate` 的两条来路（§4.5） | 一条：候选集为空（一个适配器都没登记 / 全部无画像 / 全部未过闸门）；一条：候选集非空但全部 `Unavailable`。各断言是哪一枚 |
| `DuplicateModelCandidate` 不可达（§4.5） | 断言候选集的 id **两两不同**（它们逐行来自 `model_registry` 的主键列） |
| `NotRoutable` 不可达（§4.5） | **构造性**：候选集的元素类型是 `RoutableModel`（字段私有、唯一构造点在 `try_new` 内），故调用方交不出一个未过闸门的候选。G 侧只钉其中一格（`stale` 不进候选集），十态逐项在 D——见本节末「刻意不重复」的 (a) |
| 用词纪律（§1.3） | 模块面断言：`model_call.rs` 的源码文本里 `AuthorizedTool` / `AuthorizedToolInvocation` / `ToolProvider` / `invoke_tool` / `authorize` **五个标识符零命中**（匹配前折叠空白） |
| 中立性（§10.3） | 源码文本里不出现实现 crate / 实现类型名（下界）；`every_crate_depends_only_on_its_allowed_set` 仍绿（**G 不加任何边**） |

**本项目既有的三条纪律一并适用**：变异须在全量 `--no-fail-fast` 下得出否定结论（且确认红的**位置**）；
凡注释写绝对措辞须有对应用例（枚举式断言**逐项**有照片）；失败路径须断言是哪一种 `Err`。

**两条刻意不重复的**（判据是唯一入口，避免同一个性质两处各测一遍）：
（a）**§249 的十态逐项**——它的照片在 D（`only_the_six_routable_states_pass_the_gate`，
`crates/continuum-model-registry/src/lifecycle.rs:331`；以及 `tests/compile_fail/routable_model_fields_are_private.rs`）。
G 侧的元素类型就是 `RoutableModel`（字段私有、唯一构造点），**G 构造不出未过闸门的候选**，
故 G 只钉与 §3.2 相异的那一格（`stale` 不进候选集），不重跑十态。
（b）**D 的 `RoutingError` 的逐变体用例**——D 已有；G 钉的是**可达性**（§4.5 的表）。

---

# 12. 拍不到的照片（逐条写明为什么没有）

1. **端到端：从「一个任务」到「一次模型调用」**。G 的三个输入里有**两个没有生产方**：
   `TaskSkillRequirement` / `FamilyPreference`（§250 的第 1、5 项）来自规划侧，`BudgetView` 来自
   驱动从语义层 `Budget` 的投影（D §6.1）——**规划侧与语义层都尚未建**。故所有用例的输入都由测试直接构造，
   **钉不了「驱动真的这么传」**。
2. **「闸门真的挡住了生产路径」**。G 的照片能钉「候选集里没有未过闸门的模型」，但**钉不了
   §2.4 那条绕行路径**（持有注册表者直接 `invoke`）——**它没有强制点可挂**，而这不是实现缺陷（§2.1）。
3. **`FailureClass` 驱动了重试**。`decide_retry` 零生产调用点（§6.3），故**没有任何用例能断言
   「一次模型失败导致了一次重试」**。能拍的只有映射本身。
4. **升级 / 降级的实际触发**。同 3，且 §251 的前提（Contract / Budget）没有输入（§6.4）。
5. **超时对真实适配器的行为**。假适配器只能演「永不返回」；一个真实适配器在截止之后**仍在远端跑**
   （非流式不可取消，C §5.3），这一半**没有照片**，也**没有机构**——G 的截止只停止等待。
6. **探活与调用的真实时长**。`health()` 与 `invoke()` 的时长没有上界、也没有规范给的界（§3.3、§3.5）。
7. **流式的用量**。`StreamChunk` 不带 usage（§8.3），故 G 报不出、也测不出流式调用的消耗。
8. **多模型 / 多适配器的真实形态**。本仓今天**零个真实模型适配器**（`docs/superpowers/specs/2026-10-05-p3c-provider-boundary-design.md:43`
   的同一事实：本仓只有 DeepSeek 单一端点，且无消费方）。故「一个适配器挂掉时它的所有模型一起不可用」
   （§3.3 的粒度）在真实适配器上的表现**拍不到**——假适配器只能钉门，钉不了门后的东西
   （照 B 的 §八.9 的同一写法）。

---

# 13. 义务清单：C 与 D 的「收件人」逐条处置

`p3bcdf-followups.md` §五 说 G 的义务散在 C 与 D 的「收件人」栏里（C 约 19 处、D 约 9 处），
**「G 设计时逐条收——别让它们再悬着」**。本节逐条给出处置。**处置有五种**：

| 标记 | 含义 |
|---|---|
| **接** | 本设计有落点（含「收一半」：一条里的一部分义务归 G、另一部分续指别处） |
| **退** | 收件人写了 G（或指向「执行侧」），**判为不是 G 的**——写明判据与续指收件人 |
| **兑现** | 那条说的就是「G 还不存在」，本文即是它的设计 |
| **已闭** | 据实核对后关闭（本轮折入的 C 侧订正即此列） |
| **不经手** | **收件人本来就不是 G**——记录为「看过、并判过不归 G」。**这一列不是跳过**：它让后来者知道这一类被数过（两节的条数与清单都写全） |

**与 §五 那两个数对账**（免得后来者以为本节漏收）：§五 说的「C 约 19 处、D 约 9 处」是**含「收件人」
字样的行数**——一个话题常散在好几行（一条里既有正文又有续指），故它与**条目数**不是同一件事。
本节记的是条目：**C 的 §12 共 24 条**（G 相关 **5** 条：第 3、7、9、12、23 条；不经手 **18** 条；
另加 C 于 2026-10-06 新增的第 **24** 条），**外加 §12 之外的 4 处**（§6 的调用面、§5.1 的建议表、
§2.2 五、§4.2 形态 3）；**D 的 §11 共 24 条**（收件人为 G 的 **20** 条，不经手 **4** 条），
外加 §1.2、§2.5、§5.1、§6.1、§7.1、§8.2 与 D 计划的 `## 遗留` 各一处。

## 13.1 来自 C 的设计（`docs/superpowers/specs/2026-10-05-p3c-provider-boundary-design.md`）

**行号以 2026-10-06 的现文为准**（C 的设计在这一天折入了本设计提出的 §4.2、§4.3 两条，
其 §12 现在共 **24 条**，且**新增的是第 24 条**——那条由本设计引出）。

| 出处 | 内容 | 处置 |
|---|---|---|
| §6（`:462-475`） | 调用方七步：取描述 / **取可用性** / 路由 / 解析 / 调用 / 消费流 / 健康 | **接**（§3.1、§3.3）；第 2 步的混称已由 C 拆开（§4.3） |
| §6「调用方不得假定」（`:484-489`） | 不得用 `usage()` 当作 Router 的成本输入 | **接**（§5、§8：G 连 `usage()` 都不用） |
| §5.1（`:357-391`） | `ProviderError → FailureClass` 的建议表；收件人是两条路径的调用方 | **接模型侧**（§6.1）；建议表里那条「不得升格为函数」的理由一并接（§6.2） |
| §12 第 3 条（`:846-849`） | `usage()` 语义未定义且无消费方；收件人子项目 G | **接判、退回定义**（§5）：两种读法都不成立，G 不用它；续指规范维护者 + 对账方 |
| §12 第 7 条（`:864-870`） | `ProviderError` 无「限流 / 配额耗尽」变体，`FailureClass::Resource` 丢失；收件人 F + G | **接（模型侧）**：G 的分类表里 `Resource` 这一格**无输入**（§6.1 的五变体里没有它）；**G 不扩别人 crate 的取值域**，续指 C 的设计 + 规范维护者（§14 第 9 条） |
| §12 第 9 条（`:872-878`）+ §2.2 五（`:120-130`） | `ExecutionProfile` 的 `model` / `provider` / `tool` 收紧；收件人「G 或 F」 | **接一半、且不落实现**（§7）：`model` 归 G、`provider` 保持 `String`、`tool` 归 F；缺的是**节点执行的装配点**，续指那一方。C 的 §2.2 五在 2026-10-06 就地折入了同一条实读订正（`:123-130`），**关于那张表的实读两处一致** |
| §12 第 12 条（`:885-891`） | 「子项目 G 今天尚不存在……模型侧方法与注册表无生产调用方」 | **兑现**：本文即是它的设计；代码仍不存在，§12 第 2 条据实记 |
| §12 第 23 条（`:933-937`） | 凭据要不要也交给适配器；收件人控制器 + B / G | **退，且给出判据**（§2.1 的强制点 (3) 那一格）：模型适配器的端点凭据在装配期固定在适配器身上，**不由能力派生**，故不落强制点 (3)；若规范将来要求模型侧凭据也逐次签发，那会动 §315 的请求面，续指规范维护者 + C 的设计 |
| §12 第 24 条（`:938-947`，**C 于 2026-10-06 新增**） | 「模型侧的绕行路径：`model_for` 交出裸适配器，故 `invoke` 可绕过 D 的闸门与 G」——由本设计 §2.3/§2.4/§2.5 引出 | **C 已同意该判断并具名收件人**（规范维护者 + 控制器）；与 §14 第 1、14 条同源。**不是不经手的一条：它是这一轮接缝的收口** |
| §4.2 形态 3（`:336-337`） | 「调用方持有实现」本阶段无照片 | **记**（§10.3）：**G 就是那个调用方**；仍无照片，判据是零个适配器 crate |

**记录为「不经手」的 18 条**（C 的 §12 里**收件人本来就不是 G** 的那些；写成一段是为了让后来者知道
**这 18 条被看过、并判过不归 G**，而不是被跳过）：

> **第 1、2、4、5、6、8、10、11、13、14、15、16、17、18、19、20、21、22 条**——
> 收件人分别是「将来提出该需求的人」（第 1 条）、**F**（第 2、4、5、8、17 条）、
> **C 的实现计划**（第 6、13、21 条及第 8、22 条的一半）、**控制器**（第 4、15、18、22 条）、
> **`continuum-capability`**（第 15 条）、**规范维护者**（第 11 条）、以及**「无（已闭）」**（第 10、14、16、19、20 条）。
> 它们的题材全部是**工具调用路径、C 自己的实现、`continuum-capability`、或已闭项**——
> **与模型调用路径无交集**：G 的模块里连 `ToolProvider` 这个名字都不出现（§1.3）。
> 第 5 条里的 Planner 是**执行层构造工具调用的一方**，也不是 G（G 不构造工具调用）。

**其中四条虽不经手，本设计对它们各留一处记录**（不改变收件人，只为可查）：

- **第 1 条（非流式不可取消）**：G **不需要**它（截止走异步截止，§3.5），但**它的限度记在 §12 第 5 条**
  ——一个真实适配器在截止之后仍在远端跑。
- **第 2 条（`runtime → continuum-provider` 悬空边）**：收件人是 F；**本设计记明 G 是这条边的第二个
  使用点**（模型侧），处置在 §10.1 第 3 条。
- **第 11 条（`effect_class` 两轴）与第 18 条（P1 的 `Resource` 义务）**：**§13.3 与 §6.4 各有一条
  逐条判**（前者判「不落到 G」，后者判「`RetryPolicy` 属执行层」）。
- **第 17 条（工具侧两个登记点）**：收件人是 F；**同一形状在模型侧有一份，那一份由本设计接**（§3.2）。

**故 C 的 §12 逐条都有落点**：**G 相关的 5 条**（第 3、7、9、12、23 条）逐条处置如上的表，
**不经手 18 条**如上，**外加第 24 条**（由本设计引出、C 已认领）。**5 + 18 + 1 = 24。**

## 13.2 来自 D 的设计（`docs/superpowers/specs/2026-10-05-p3d-model-registry-router-design.md`）

**D 的 §11 恰 24 条，其中收件人为 G 的 20 条**（逐条处置如下）；**余下 4 条（第 1、2、3、12 条）
的收件人不是 G**，**记录为不经手**（见本节末）。**行号以 2026-10-06 的现文为准**（D 的设计本轮未改）。

> **一处订正，来历留在原地**：本设计初稿在本节写「D 的 §11 全 24 条」**逐条处置**——**那句是虚的**：
> 实测 24 条里只有 20 条的收件人是 G，另 4 条（第 1、2、3、12 条）的收件人是规范维护者／协调者。
> **不是漏收，但那句话把一个没做过的全称说成了做过**（本项目「把设计里说了的写成已经有了」的同一形状）。

| 出处 | 内容 | 处置 |
|---|---|---|
| §1.2（`:37-72`） | 交界：选的判断在本层、「拿它跑」在 G | **接**（§2、§3） |
| §11 第 5 条（`:1247-1252`） | `BudgetView` 的单位与取值域未定；收件人语义层 + 规范 | **接为口径**（§8.1）：G 只搬运、不解释 |
| §11 第 13 条（`:1274-1295`） | C↔D 三条接缝（调用面 / `ALLOWED` 条目 / 「模型调用路径今天不存在」），**三条的收件人都是 G** | **逐条**：调用面**已闭**（C §6 现文已写明「Router 不调用适配器，调用方是子项目 G」；D 的计划已判那条「未闭」系误读，见 `docs/superpowers/plans/2026-10-05-p3d-model-registry-router.md` 的 `## 遗留` 里「设计定稿时已闭的七条」第 7 格）；`ALLOWED` 条目 → **本设计判不新增 crate、runtime 的条目已含全部所需**（§10.1），故 G 不改那张表；第三条 → **兑现** |
| §11 第 15 条（`:1299-1306`） | 工具侧 `cost` / `latency` 的 `Some` 无人认领 | **退**（§13.3） |
| §11 第 18 条（`:1313-1327`） | `ProviderError → FailureClass`：D 有条件签收（签收已分类的结果），**分类发生在持有 `ProviderError` 的一侧 = 子项目 G** | **接**（§6.1）；**`FailureHistory` 的桥无主**记在 §6.3、§14 第 6 条 |
| §11 第 20 条（`:1332-1339`） | G 的两件事（取 `ProviderHealth` 快照、消费 `RankedExecutionCandidates` 发起 `invoke` / `stream`）；「模型调用路径今天仍无实现方」 | **接 + 兑现**（§3.1、§3.3） |
| §11 第 22 条（`:1345-1355`） | `effect_class` 两轴退件 | **退**（§13.3） |
| §11 第 23 条（`:1356-1360`，理由见 §2.5 末段 `:314-333`） | `trust` 退件 | **退**（§13.3） |
| §11 第 24 条（`:1361-1364`） | `ProviderHealth::Degraded` 的降权判据规范未给；收件人规范维护者 | **退**：降权是**排序策略**的事（D 的 `RankingPolicy`），G 不排序；但 G 是 `availability` 的**唯一生产方**，故**「这条判据缺了会让 G 送出的 `Degraded` 在 D 的基线里不产生任何效果」**据实记在此处，续指规范维护者 |
| §7.1（`:1044-1055`） | 「失败检测与升级触发……那是**子项目 G**的活」 | **收一半、退一半**（§6.4）：收分类、退升级触发（三条判据） |
| §6.3（`:978-1007`） | `ExecutionProfile.cost_budget` 的类型收紧未做；收件人协调者 + 语义层 | **退**：`cost_budget` 不是 G 的字段（G 只碰 `timeout_ms`，且是以值，§3.5） |
| 计划 `## 遗留`（`:1576-1591`） | `BudgetView` 的 `None` ≠ `Some(0)` 谁守；**收件人：驱动侧的投影实现与子项目 G** | **接**（§8.2），两向对钉 |
| §11 第 4、6、7、8、9、10、11、14、16、17、19、21 条 | 规范级未决（§84 的语义 / `Cost`·`Latency` 的复用 / 三处画像清单 / 子维度分层 / `failure_modes` 词表 / §19 阈值 / §249 迁移关系 / 迁移号 / 初步画像 / `version` vs `time_range` / 上下文长度 / `Tier 1 Low`） | **退（逐条一句）**：这些的收件人是**规范维护者 / 复审者 / 协调者 / D 的实现者**，**没有一条落在模型调用路径上**——G 不读画像字段、不打分、不管迁移。**本子项目一条都不发明** |

**记录为「不经手」的 4 条**（收件人不是 G，我看过并判过它们不归 G）：

> **第 1 条**（`stale` 的残留一项：§249 正文要不要把 `stale` 正式写进 `MUST NOT`）——收件人**规范维护者**；
> **第 2 条**（§27 的探索被 OPEN-014 / OPEN-008 双阻断）——收件人**规范维护者**；
> **第 3 条**（§25 Population Feedback 来源未定义）——收件人**规范维护者**；
> **第 12 条**（`ExecutionProfile.cost_budget` 的类型收紧）——收件人**协调者 + 语义层设计**。
> 四条都不是模型调用路径上的事（第 12 条那一格另见本节表内 §6.3 一行）。
> **一条也不接，也不为凑满 24 条而认领。**

**故 D 的 §11 逐条都有落点**：**收件人为 G 的 20 条**逐条处置如上（含两条「收一半」），
**不经手 4 条**如上。**20 + 4 = 24。**

## 13.3 三条退件：逐条判，明确写「本子项目不接」

`p3bcdf-followups.md` §三 第 3、4 条与 D §11 第 22、23、15 条把三条判给「执行侧（工具调用路径）＋规范维护者」。
**G 是那个「执行侧」在模型侧的名字，故必须逐条判**——**判法是：它落到 G 头上吗？**

| 条目 | 落到 G 吗 | 判据 |
|---|---|---|
| **`effect_class` 是否与 `EffectType` 是两个轴** | **不落到 G** | 问句的两侧是 `Tool.effect_class`（Tool Registry，P3A）与 `EffectType`（Effect Journal，边界层）。**G 对 `effect_class` 的引用为零**：G 的词汇是 `ModelId` / `ModelProfile` / `ProviderHealth` / `ExecutionProfile`，**G 不读 `ToolProfile`、不选工具、不调用工具**。D 退件的三条理由（D §11 第 22 条：D 对它的引用为零、两侧都在工具路径上、没有触发条件可挂）**在 G 上逐条同样成立**。**续指：子项目 F ＋ 规范维护者** |
| **`trust`** | **不落到 G** | 同一条判据：`trust` 是 `ToolProfile` 的字段（P3A §3.1），它的语义是「一个**工具**登记进 Registry 时必然有信任判定」。G 不选工具、不读 `ToolProfile`；§247 的十二个 `ModelProfile` 字段里也没有它（D §2.5 已逐项核过）。**续指：子项目 F ＋ 规范维护者** |
| **工具侧 `cost` / `latency` 的 `Some`** | **不落到 G** | D 复用的是 `Cost` / `Latency` 两个**类型**（落在模型画像上），**不读 `ToolProfile`**；G 连画像的这两个字段都不读（它不做成本与延迟的排序）。而 `p3bcdf-followups.md` §三 第 4 条的判据是**「它们同属『工具选择』，而『工具选择』这个组件在 §4.1 里不存在」**——**G 不是工具选择的候选落点**。**续指：协调者（在四份之间指派）＋ 规范维护者** |

**「别为了收干净把它们揽进来」**：本设计**不接**这三条，理由是它们的两侧都在工具侧，而把工具侧的词汇
揽进模型调用路径，正是裁决 §一.1 那句「**G 不得借用 F 的名字**」要防的形状。

## 13.4 `p3bcdf-followups.md` 的其余各节

- **§三 第 1、2 条**（非效应臂的强制点 / 「一条操作该绑哪一枚 `CapabilityKind`」）：**退**——
  两条都属**连接器（B 的 §124/§125 路径）**，与模型调用无交集。收件人照原文（规范维护者）。
- **§五**（给 G 的交接三条）：**兑现**（本文即其设计；逐条收见 §13.1 / §13.2；不得与 F 混称见 §1.3）。
- **§八**（B 的实现计划的遗留 25 条）：**据实核过，收件人无一条是 G**——它们分别是协调者、规范维护者、
  B 的实现的后续轮次、长期阶段、`continuum-core`、以及「无（已闭）」。
  **故本子项目对 §八 无处置**（不为凑数认领一条）。**留这一行是为了让「核过」这件事可查。**
- **§七**（接缝裁决八条）：第 4 条点名「D 的迁移注册原先被记在『用它的那个 task』名下——
  **那等于子项目 G，本轮不存在**（这是一条空头收件人）」，已由裁决改判给 D。**G 不接**；
  第 3 条（`runtime → connector` 无消费者）不涉 G。

---

# 14. 遗留与未决项

**每条的收件人都具名；凡「缺的是哪一步」都写到步骤，不用「后续」「长期阶段」兜。**

1. **§249 的 `MUST NOT` 主语是 Router，模型调用的绕行路径是否需要一条禁令，规范未给判据**（§2.5）。
   后果：一条不经路由、直接 `invoke` 未画像模型的路径**不受这条禁令的字面约束**，而模型调用路径上
   **没有任何强制点**（§2.1）。**G 不发明**。**收件人：规范维护者。**
2. **`ProviderHealth` 是适配器级，§250 的可用性是模型级**（§3.3）：同一个适配器名下某个模型单独不可用
   这件事**在 §315 的类型上不可表达**；`describe_model` 的 `Err` 不是可靠替代（`Unavailable` 在本仓
   已被另一件事占用）。**收件人：C 的设计（§315 的持有者）＋ 规范维护者。**
3. **探活（`health()`）没有截止**（§3.3）：一次挂住的探活会把整条路由挂住；§315 无超时参数，
   C §5.3 已判「超时不进签名」。**要不要给探活一个截止，规范未给判据。收件人：C 的设计 ＋ 规范维护者。**
4. **流的分片消费没有截止**（§3.5）：一个既不来分片也不结束的流会把 G 挂住；要它可中止，须先有
   「分片消费带截止」的机构，而 §315 的方法集里没有承载它的位置。**收件人：规范维护者。**
5. **`usage()` 的语义两种读法都不成立**（§5）：定它的前提是先有「会话」或「累计」的规范定义。
   **收件人：规范维护者 ＋ 将来做对账的那一方（语义层）。**
6. **`FailureClass` → `FailureHistory` 的桥无主**（§6.3）：G 产出**逐次类别**，D 的策略要一个
   **形状未定**的 `FailureHistory`（D §5.1），而**从类别到历史的累积器在四份设计里无人认领**。
   **收件人：D 的设计（`FailureHistory` 是它的策略输入）＋ 规范维护者。**
7. **`ExecutionProfile` 的模型侧两个字段**（§7）：缺的是**「节点执行时装配 `ExecutionProfile` 并调用 G
   的那一步」**——今天零生产构造点、`execution_profile` 表没有这两列、也没有读写函数；且它对 G 的
   运行路径零影响。**收件人：构造 `ExecutionProfile` 的那一方（驱动侧的节点执行装配点）＋
   `continuum-graph`（类型）＋ 规范维护者（§246 的字段要不要落库）。**
8. **流式调用没有用量读数**（§8.3）：`StreamChunk` 不带 usage，而流式是常态；故「按实际结算」在流式
   调用上没有读数来源。**收件人：语义层设计（对账方）＋ 规范维护者。**
9. **`ProviderError` 没有「限流 / 配额耗尽」的变体**（§6.1、C §12 第 7 条）：模型路径上
   `FailureClass::Resource` 这一格**无输入**；后果是一次配额耗尽会被分成 `Transient`，
   若它在重试白名单里就会**被反复重试一个不会变的状态**。**G 不擅自扩 `continuum-core` 的取值域**
   （那是 §315 的接口类型与 C 的定义面）。**收件人：C 的设计 ＋ 规范维护者。**
10. **请求 D 补一个「枚举已登记模型」的读函数**（§4.1）：**这是本设计查出的硬缺口**，
    G 的候选集构造卡在它上面；不给它，G 只能走「第二个来源」或「裸查 D 的表」两条坏路。
    **已认领（2026-10-06）**：D 的计划 Task 14 Step 1 补上了它
    （`docs/superpowers/plans/2026-10-05-p3d-model-registry-router.md:1395`，签名在 `:1408`，
    照片要求在 `:1415`）。**尚未落地为代码**——G 的 plan 要把它记成一条对 D 的依赖。
    **收件人：D 的实现计划（Task 14）。**
11. **`model_providers()` 零消费方**（§4.2）：**已由 C 落实（2026-10-06）**——该入口已裁删
    （`…p3c-provider-boundary-design.md:147`、`:153-165`），C §6 第 4 步亦写明 id 来自 D 的表而非本注册表
    （`:469-471`）。**本设计不再有未落点。收件人：无（已闭）。**
12. **C §6 第 2 步「取描述」与「取可用性」混称**（§4.3）：**已由 C 落实（2026-10-06）**——
    已拆成第 1、2 两步（`…p3c-provider-boundary-design.md:462-467`），并把这处订正的来历留在原地。
    **本设计不再有未落点。收件人：无（已闭）。**
13. **三条退件**（§13.3）：`effect_class` 两轴 / `trust` / 工具侧 `cost`·`latency` 的 `Some`
    **本子项目不接**（判据逐条在 §13.3）。**收件人照原件**：前两条是**子项目 F ＋ 规范维护者**，
    第三条是**协调者（在四份之间指派）＋ 规范维护者**。**这不是遗留缺口，是退件**——
    列出是为了让「它们不再悬着」有一处明确答复。
14. **`model_for` 交出裸适配器，直发 `invoke` 绕得过 G**（§2.4）：**收不回**，且模型侧**没有可加的
    位置**（§315 的 `invoke` 没有规范要求，故请求面上没有东西可加，与 §316 在裁决 §一.2 之后的处境不同）。
    **它与第 1 条同源**：这是一条**规范级**而非设计级的缺口。**C 已于 2026-10-06 认领并同意该判断**
    （C §12 第 24 条，`…p3c-provider-boundary-design.md:938-947`：不改 `model_for`、不动 §315）。
    **收件人：规范维护者（§249 的射程是否扩到模型调用）＋ 控制器（若要在 C 侧收紧，那要先有一条规范要求）。**
15. **`decide_retry` 零生产调用点**（§6.3）：G 的分类产物**今天没有消费方**——裁决把分类放在 G、
    把重试策略放在执行层，而执行层尚未装配。**这不是 G 的漏做。**
    **收件人：驱动侧的节点执行装配点（同第 7 条）＋ 协调者。**
16. **`Deadline.elapsed_ms` 今天没有消费方**（§3.1 末段）：口径已写死为**实测耗时**，
    但没有任何一方按它分支（没有退避、没有遥测）。**故 §11 的用例只断言是哪一枚 `Err`、不断言数值。**
    若将来有人要按它做退避或记录用量，那是**消费方出现**那一刻的事——届时先定「实测耗时」还是
    「截止值」是否需要分开两个字段。**收件人：将来提出该需求的人。**
17. **G 的实际前置是三个流：C、D、F**（§10.2 的订正）：`ModelCallError` 的落点
    `crates/continuum-runtime/src/error.rs` 由 **F 的 Task 2** 创建
    （`docs/superpowers/plans/2026-10-05-p3f-tool-call-path.md:307` 与 `:158`），**今天不存在**；
    G **不代 F 建它**（同一个文件两个创建者 + F 还要整块搬入 `TaskError`）。
    同理 `async-trait` / `futures-core` 两条 dev 边也由先落地的那一方登记（F8 记的是 F 加，尚未落地）。
    **缺的那一步是「F 的 Task 2 建出 `error.rs` 并搬入 `TaskError`」。**
    **收件人：协调者**（排期：G 的 Task 1 排在 F 的 Task 2 之后）。
18. **G 的测试夹具嵌 D 的表结构**（§11 前置二）：**接受这条耦合，不向 D 要播种入口**（判据三条在彼处）。
    它的性质是**有意的耦合 + 有意的护栏**：**加「可空或带默认值」的列不会红**（G 按列名写、不 `SELECT *`），
    **改掉 G 依赖的那些列名或编码、以及新增一列必填的列，都会红**。**缺的机制是「D 改表时通知 G」这一步**。
    **收件人：D 的实现者（表结构变更须通知）＋ G 的实现者。**
