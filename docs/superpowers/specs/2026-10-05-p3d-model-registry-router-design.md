# P3 资源层（子项目 D）设计：模型画像、Model Registry 与 Router

**范围**：《工程》§4.1 的十一个组件中，本子项目建**模型侧的五项**——ModelProfile 与 SkillVector 存储（§247 §248）、
Model Registry 生命周期（§249）、Router 与候选排序（§250 §84）、升级与降级（§251 §85 §86）、路由探索（§27）
（`docs/02-工程.md:215-221`）。

本子项目是 P3 分解（A–E：Capability / 连接器 / 工具与 Provider 中立边界 / 模型侧与 Router / 计算节点与放置）中的 **D**，
次序 A → B → C → D → E（P3A 设计第 1 行）。A 是 D 的前置——按 §4.3 的现文，即
`Router ← Model Registry + continuum-capability 的两个类型（Cost / Latency）`（`docs/02-工程.md:248`；
该行 2026-10-05 订正，来历见 §8.4）。

**与共享面的关系**：路径归属、已冻结接口、四条横切约束以
`docs/superpowers/specs/2026-10-05-p3-bcdf-ownership-and-interfaces.md` 为准。**成本输入是预算视图**这一条由 ENG-005 裁决
（`docs/superpowers/specs/2026-10-05-eng-005-budget-accounting.md` 第三、五节）。本设计对共享面**不提出异议**；
但它对《工程》§4.3 的两条依赖边有异议，见 §8.4。

---

# 1. 范围

## 1.1 建什么、不建什么

| 组件 | 规范依据 | 本子项目 |
|---|---|---|
| ModelProfile 与 SkillVector 存储 | §247 §248 | 建（三张表，§3） |
| Model Registry 生命周期 | §249 §21 §22 §82 | 建（状态类型 + 迁移表 + 可路由闸门，§4） |
| Router 与候选排序 | §250 §84 | 建（输入/输出类型 + 排序策略接口 + 具名基线策略，§5） |
| 成本输入（预算视图） | ENG-005 §4.1 §4.3 | 只建**接口投影**，不建记账（§6） |
| 升级与降级 | §251 §85 §86 | 只建**阶梯的数据形状**；触发与预算前提的检查不在本层（§7.1） |
| 路由探索 | §27 | **不建**（被 OPEN-014 与 OPEN-008 双阻断，§7.2） |
| Provider Adapter / ToolProvider 注册与发现 | §315 §316 §80 | 不建（子项目 C） |
| Connector 与权限细分 | §124 §125 | 不建（子项目 B） |
| Compute Node 注册 / 节点放置 | §287–§291 §243 §93 | 不建（子项目 E，被 OPEN-007 阻断） |

**不建的部分不留桩。**

## 1.2 与子项目 C 的一个交界，本设计先行划定

`docs/02-工程.md:250` 写 `Provider Adapter → 被 Router 调用，接口中立`。本设计**不让 Router 直接调用适配器**：
Router 的「当前可用性」输入取 §315 既有的 `ProviderHealth`
（`crates/continuum-core/src/model.rs:83`），由调用方**子项目 G（模型调用路径）**取好后作为**值**传入（见 §1.2 末段）。
理由有二，都不是偏好：

1. 排序因此是**同步纯函数**——没有 I/O、没有 `.await`，可以在表驱动用例里穷举；
2. §4.3 的那条箭头说的是「适配器的信息到达 Router」，**以值为载体同样到达**，且不产生一条 C 与 D 之间的新边。

**本子项目的交付物是一个「判断」，不是一次执行。** §250 的输出是 `RankedExecutionCandidates`、§84 要的是
`confidence` / `alternatives` / `reason`——两处要的都是**排序后的候选与理由**。故 **`ModelProvider::invoke` /
`stream` 的调用方不在本层**：那是**子项目 G（模型调用路径）**拿着一张 `RankedExecutionCandidates` 去做的下一步，
本层既不持有 `CallId`，也不消费 `ModelStream`。**本设计对 `continuum-provider` 的引用为零**（§8.2），
`invoke` / `stream` 在本 crate 里一次都不出现。

**这一条是本设计对 C↔D 交界的正式表态**：责任的分界是「**选哪个模型**」在本层、
「**拿它跑**」在**子项目 G（模型调用路径）**。此交界记在 §11 第 13 条。

### 调用方是**子项目 G**，**不是 F**（已裁）

本设计先前的措辞是「调用方（驱动，**子项目 F**）」——**那句是错的，已订正**（错误说法的来历留在此段）。
判据是 **F 的范围**：F 是**工具调用路径**（`authorize` → `ToolProvider::invoke`），
而这里是**模型调用路径**（`ModelProvider::invoke` / `stream`）——**两条不同的路径、不同的强制点覆盖**
（F 过强制点 (1)；模型调用没有任何强制点）。

**收件人现已具名：子项目 G。** 2026-10-05 的整套终审把它裁了出来——G 是**模型调用路径**，
承担此前从未具名的「执行侧」那一串义务（取 `ProviderHealth` 快照、调本层的 `rank`、
发 `invoke` / `stream`、消费 `RankedExecutionCandidates`、定义 `usage()` 的语义、收紧 `ExecutionProfile`
的字段、做模型侧失败分类）。依据见
`docs/superpowers/specs/2026-10-05-p3bcdf-set-decisions.md` 第一节第 1 条。故本设计的写法是：

- **指名到子项目**：调用方 = **子项目 G**，即 `RankedExecutionCandidates` 的消费者。
- **本条与本层的关系**：G 是**消费方**，不是本层的依赖边；本层对它零依赖，
  本层的交付物仍是那份**判断**（§5）——G 拿着它去发起调用。

---

# 2. `ModelProfile` 与 `SkillVector`（§247 §248）

## 2.1 §247 的十二个字段逐项处置

规范 §247（`docs/spec/05-normative.md:802-824`）给的是字段名与分组，**未给任何字段的类型、取值域或单位**。
下表逐项写明本设计的处置，**不发明度量**：

| §247 字段 | 本设计的处置 | 理由 / 出处 |
|---|---|---|
| `id` | `ModelId`（既有类型） | §315 的 `continuum_core::model::ModelId`（`crates/continuum-core/src/model.rs:9`）。**不新建第二个模型 id**——同一件事两个类型是本项目一贯判为 Critical 的那一类（P3A 对 `ToolId` 的同一裁定，P3A 设计 §3.1） |
| `version` | `String` | 规范只给名字。取值域留空，**不解释**（与 P3A 的 `Tool.version` 同一处置） |
| `provider` | `String` | 同上 |
| `model_revision` | `String` | §81 要求身份不能只用 `"model-name"`。本字段是「供应商侧版本」的落点，取值域留空 |
| `modalities[]` | `Vec<String>`，落库为 JSON 数组 | **词表规范未定义**（§247 未列取值，§12 未给封闭集合）。见 §2.4 的编码约定与 §11 第 8 条 |
| `tools[]` | `Vec<ToolId>` | 元素复用 §252/§316 的 `ToolId`（`crates/continuum-core/src/tool.rs:7`）——「这个模型能用哪些工具」正是那个类型的含义。**不新建第二个 ToolId** |
| `skill_vector` | `SkillVector` | §248，见 §2.2 |
| `failure_modes[]` | `Vec<String>`，落库为 JSON 数组 | §83 给的是自由文本例（`loses constraints in very long tasks` 等，`docs/spec/02-positioning.md:779-784`），**无封闭词表**。见 §11 第 9 条 |
| `latency_profile` | `Option<Latency>`（复用 P3A 的类型） | 见 §2.5 |
| `cost_profile` | `Option<Cost>`（复用 P3A 的类型） | 见 §2.5 |
| `evidence_count` | `u64` | 计数，没有单位问题 |
| `confidence` | `Ratio` | §84 的示例把 confidence 写成 `0.94` / `0.51`（`docs/spec/02-positioning.md:809-816`），故取 **[0, 1] 的实数**。这是**照 §84 的示例推导**，不是发明 |

**§81 与 §247 不一致，本设计按 §247。** §81（`docs/spec/02-positioning.md:708-734`）说模型身份至少含
`provider / model_id / revision / deployment / capability fingerprint` 五项，而 §247 的 `ModelProfile` 只有
`provider` 与 `model_revision`，**没有 deployment 与 capability fingerprint**。本设计**不擅自补两个 §247 没有的字段**
（「不预先发明」），把这个不一致记在 §11 第 7 条。

**同一类不一致还有第三处**：§22 的「Profile 记录」清单（`docs/spec/01-concepts.md:1047-1064`）列的是
`modalities / context / tool use / reasoning / coding / vision / audio / spatial / instruction following /
constraint adherence / latency / cost / failure modes / known quirks`——它既有 §247 没有的项
（`context`、`known quirks`、`audio`、`instruction following`、`constraint adherence`），又不含 §248 的
`planning` 与 `verification` 两维。本设计按 §247／§248 这对**规范层**的条目办，三处不一致一并记在 §11 第 7 条。

### `ModelProfile` 的类型与**唯一产生点**

```rust
/// §247 的模型画像。字段私有，**无公开构造函数**。
pub struct ModelProfile {
    id: ModelId,
    version: String,
    provider: String,
    model_revision: String,
    modalities: Vec<String>,
    tools: Vec<ToolId>,
    skill_vector: SkillVector,
    failure_modes: Vec<String>,
    latency_profile: Option<Latency>,
    cost_profile: Option<Cost>,
    evidence_count: u64,
    confidence: Ratio,
}

impl ModelProfile {
    pub fn id(&self) -> &ModelId;
    pub fn skill_vector(&self) -> &SkillVector;
    pub fn confidence(&self) -> Ratio;
    pub fn evidence_count(&self) -> u64;
    pub fn modalities(&self) -> &[String];
    pub fn tools(&self) -> &[ToolId];
    pub fn failure_modes(&self) -> &[String];
    // 其余字段的访问器按消费方需要增补，不预先铺开
}
```

**crate 外没有构造入口；crate 内有两个产生点**——`continuum_model_registry::persist::load_profile`
（从库读，**唯一的生产路径**）与 `try_new`（`load_profile` 与测试用）。这不是洁癖，
是 §4.2 那条「没有画像就没有候选」的第二条腿——**它必须与 `RoutableModel` 同为结构性保证，
否则那条腿是纸的**：

- 若 `ModelProfile` 能在内存里自由构造，则 `RoutableModel::try_new(自造画像, LifecycleState::Discovered)`
  就能把一个**从未落库、从未过画像流水线**的模型送进 `rank`——**注意入参是 `LifecycleState`（十态），
  不是 `RoutableState`**：闸门要在 `try_new` 内部发生，收窄后的类型不能是入参（§4.2）。那样 §21 的「新增模型不能直接进入自动 Router」
  就退化成「Router 记得只收真画像」，而 §4.2 那半边的保证随之失效。
- 照片两张，与 P3A 的 `AuthorizedTool` 同形：`tests/compile_fail/model_profile_cannot_be_built.rs`
  的编译失败样例（crate 外无公开构造），以及一条用例断言**该 profile 的 id 在库里不存在**时
  `load_profile` 返回 `Ok(None)`——即「画像必须来自库」这条路的反例。
- **措辞与保证等强**：被挡住的是 **crate 外构造**；crate 内 `try_new` 是公开的（`load_profile` 要用它）。
  若写成「唯一产生点」就过头了——**它是「crate 外无入口」，不是「全仓只有一个构造点」**。

## 2.2 `SkillVector` 与「评分是时间序列，不是常数」

§248（`docs/spec/05-normative.md:828-842`）给九个维度：`reasoning` / `coding` / `vision` / `planning` /
`tool_use` / `constraint_following` / `verification` / `spatial` / `media`，并说「每个维度 MAY 继续分层」。

§24（`docs/spec/01-concepts.md:1106-1131`）说评分**不是常数**：「每个值同时包含 `score` / `confidence` /
`sample_count` / `version` / `time_range`」，「评分是时间序列，不是常数」。

这两条决定了存储形态，**不是可以自由选的**：只存每个维度一个标量，就等于把 §24 明写的「时间序列」实现成常数。
故：

```rust
/// §248 的九个维度。半封闭枚举：非法维度名不可表达。
pub enum SkillDimension {
    Reasoning, Coding, Vision, Planning, ToolUse,
    ConstraintFollowing, Verification, Spatial, Media,
}

/// §24 的一次观测：score / confidence / sample_count / version / time_range。
pub struct SkillObservation {
    score: SkillScore,
    confidence: Ratio,
    sample_count: u64,
    version: u32,
    time_range: (i64, i64),   // Unix 毫秒，闭区间
}

/// §248 的向量：九维各自**当前**的一次观测。
/// `None` = 该维度尚无观测——**缺席不是 0**。
pub struct SkillVector {
    dimensions: [Option<SkillObservation>; 9],
}
```

三条要写明的：

- **`Option` 而非默认值。** 「这个维度没有观测」与「这个维度评分是 0」是两件事。§24 说评分有
  `sample_count`，没有样本就没有评分；把它读成 0 会让一个未画像的模型看起来「所有维度都很差」，
  而这在 §23 的语义下与「很强」一样是假的。用例逐维度钉这一条（§9）。
- **「当前值」的判据是 `version` 最大的那次观测**，不是 `time_range` 最晚的那次。二者不一致时（先记后补）以
  `version` 为准——它是 §24 列的字段里唯一由产生方显式递增的量。该选择记在 §11 第 17 条。
- **分层（§23 的子技能树）不建。** §248 说的是 MAY；规范没有给子维度的词表或聚合规则，
  建它就要发明一套。见 §11 第 8 条。

## 2.3 `overall_score` 禁令落在哪里（§248）

§248 明写「禁止只维护 `overall_score` 作为唯一路由依据」；《工程》§4.4 的完成判据写「不依赖单一总分」
（`docs/02-工程.md:259`）。本设计把它落成**三处结构性事实**：

1. **`ModelProfile` 没有 `overall_score` 字段**，`SkillVector` 也不是标量——它是一个按维度索引的数组，
   没有「求和」这个操作，也没有任何函数把九维折成一个数。
2. **`model_profile` 表没有这一列**，且 `model_skill_score` 的主键是 `(model_id, dimension, score_version)`
   ——一个总分没有 `dimension` 可填，**在表结构上无处安放**。
3. **有照片**：(a) trybuild 样例 `tests/compile_fail/overall_score_cannot_be_read.rs` 读
   `profile.overall_score()`，判据是**编译失败**（与 P3A 钉 `full_access` 不存在同一形，P3A 设计 §2.3）；
   (b) 用例直接对库执行 `SELECT overall_score FROM model_profile` 并断言得到**具体** `Err`。

**§84 的 `compatibility` 与 §248 的禁令不冲突。** 本设计的读法已由协调者核对确认，两条证据：

1. **§84 的示例位于「每次路由的输出」块内**：该节先写「每次 Model Routing 除输出 `selected_model` 外，
   还应输出 `confidence`、`alternatives`、`reason`」，**随后**才给出 `Model A compatibility = 0.91 /
   confidence = 0.94` 那张表（`docs/spec/02-positioning.md:790-820`）。故 `compatibility` 是**一次路由算出来的、
   任务相关的匹配度**，不是画像上的一个字段。
2. **§247 的字段清单里没有 `compatibility`**，逐字段核过：`id`、`version`、`provider`、`model_revision`、
   `modalities[]`、`tools[]`、`skill_vector`、`failure_modes[]`、`latency_profile`、`cost_profile`、
   `evidence_count`、`confidence`（`docs/spec/05-normative.md:802-824`）——十二项，无一项是它。

§248 禁的是**画像上只维护一个 `overall_score`**，不是「每次路由算一个匹配度」。故本设计：画像上不存在任何总分；
每次路由可以算出一个 `compatibility`，它是策略的产物、随任务变化、不落库。该读法仍列入 §11 第 4 条供复审追认。

## 2.4 取值域与编码辅助函数

**编码挂在类型上，不在 `persist.rs` 里另建表**（本项目既有约定，P3A 计划第 22 行、P3A 设计 §3.3）。
本设计需要三个新的取值类型，各自的 `as_str` / `parse` 与类型同址：

```rust
/// §23 §24 的能力评分。示例 `coding = 9.2` 的出处是 **§83**（`docs/spec/02-positioning.md:771-773`，
/// 另见 `docs/01-总纲.md:563`）；§23 给的是能力**向量**（`docs/spec/01-concepts.md:1068-1104`），
/// §24 给的是评分的**字段组成**（`docs/spec/01-concepts.md:1106-1131`）。
/// **订正**：第一版稿子把 9.2 这个示例的出处写成了 `01-concepts.md:1068-1104`——那里是 §23 的技能树，
/// **没有 9.2 这个字面量**（`grep -n "9\.2" docs/spec/01-concepts.md` 零命中）。
///
/// **取值域与单位规范未定义**：§23 的示例是 9.2，§24 未给范围与方向。故本类型
/// 只保证一件事——**可比较**（全序）。**本设计只使用它的序，从不使用它的量**：
/// 任何跨维度的求和、加权、归一化都需要一个规范未给的尺度，故一处都不做。
/// 这一条正是 §248 禁令在算法上的对应物：量纲未知时，唯一合法的用法是比较。
pub struct SkillScore(f64);        // 构造时拒 NaN / ±∞

/// §84/§247/§24 的 [0,1] 实数（confidence、compatibility）。
pub struct Ratio(f64);             // 构造时拒 NaN / ±∞，且须落在 0.0..=1.0
```

**三个取值类型与 `SkillVector` 的构造失败都收在同一个错误类型里**——本设计先前只写「各返回具体 `Err`」
而没给类型名，那会逼计划自定一个名字（**这一处是计划作者报出来的，已补**）：

```rust
/// 画像侧取值类型的构造错误。**与 [`LifecycleError`] / [`RoutingError`] 分开**：
/// 它标的是「这个值根本不是合法取值」，不是「这次操作不合法」。
pub enum ProfileError {
    /// 不是有限实数（`NaN`、`+∞`、`-∞`）。**`Ratio` 与 `SkillScore` 都报这一枚**。
    NotFinite,
    /// **有限**但落在 [0,1] 之外（`Ratio`）。**不收 `NaN` / `±∞`**——那些归 [`ProfileError::NotFinite`]，
    /// 故 `value` 到这里时**总是一个有意义的数**（不是「一个无法比较的占位」）。
    OutOfRange { value: f64 },
    /// 该维度的观测在时间窗上不自洽（`end < start`）。
    BadTimeRange { start: i64, end: i64 },
}
```

判据是**每一种 `Err` 各有一条用例**（§9）：**两个越界输入（`1.5`、`-0.1`）各断言是 `OutOfRange`，
外加一个非有限输入（`NaN`）断言是 `NotFinite`**，再外加时间窗反序一条（`BadTimeRange`）。
**`ProfileError` 不进 `RoutingError`**：`rank` 收到的是构造好的值，
构造失败在构造期就被拒（同 §5.4 对 `RequirementError` 的处置）。

> **一处订正，来历留在原地**（本项目既有做法）。第一版稿子这句写的是
> 「**三个越界输入（`NaN`、`1.5`、`-0.1`）**各断言是哪一枚」，而本节上文 `NotFinite` 的文档写的是
> 「不是有限实数（**NaN 或 ±∞**）」——**两处相抵**：同一个 `NaN`，一处归「非有限」、一处归「越界」。
> **裁决（2026-10-05，协调者）**：**`Ratio` 的非有限输入（`NaN`、`±∞`）报 `NotFinite`**，
> **`OutOfRange` 只收「有限但落在 [0,1] 之外」**。
> **判据是跨类型一致性**：同一个 `NaN` 在 `Ratio` 上判 `OutOfRange`、在 `SkillScore` 上判 `NotFinite`
> 说不通——两枚类型对同一个输入给出不同的类别，调用方无从据此分支；而且 `NaN` 根本不是
> 「落在某个区间之外」的点（与任何区间的比较都是 false），把它塞进 `OutOfRange` 会让那个
> `value` 字段装着一个无法比较的值。改完之后，两枚变体的文档各自成立，`OutOfRange.value` 也总是有意义的数。
> **本节末那条 bullet 里的同一句话**（`Ratio` 拒 NaN 的用例）一并按此改。

- **为什么 `Ratio` 要拒 NaN**：NaN 与任何值的比较都是 false，`sort_by` 在含 NaN 的列表上不是全序
  ——排序结果随实现细节漂移。这不是洁癖：§84 的输出要被比对与记录，不确定的排序无法有照片。
  用例钉**两个越界输入（`1.5`、`-0.1` → `OutOfRange`）**外加**一个非有限输入（`NaN` → `NotFinite`）**，
  各返回具体 `Err` 且逐条断言是**哪一个变体**（订正说明见本节上文那句判据处）。
- **这对辅助函数的返回类型与 P3A 的不同，且是有理由的偏离**：`CapabilityKind::as_str` 返回
  `&'static str`，因为它是封闭枚举、字面量固定；`SkillScore` / `Ratio` 的域不是封闭枚举，`as_str` 返回
  `String`。**契约不变**：`parse(as_str(x)) == Some(x)` 逐值成立（用例钉往返），
  且 `parse` 对表外取值（非数值、越界、NaN）一律 `None`，由 `persist.rs` 转成具体 `Err`，**不取默认值**。
  浮点的往返用 `format!("{}", x)`——Rust 的 `Display` 对 `f64` 保证的是**可精确往返**（不是**最短长度**：
  `1e300` 会打出三百多个字符）。**被用例钉住的是往返**，故这里只声称往返；这一点有用例钉住，
  因为「浮点存文本会丢精度」是这里最容易想当然的地方。
- **`SkillDimension` / `LifecycleState` 落库一律小写、多词以 `_` 连接**（本项目既有约定，
  P3A 计划第 22 行）。九维因此写成 `reasoning` / `coding` / `vision` / `planning` / `tool_use` /
  `constraint_following` / `verification` / `spatial` / `media`。

## 2.5 `cost_profile` / `latency_profile`：复用 §87 的 `Cost` / `Latency`

P3A 把 §87 的 `cost` / `latency` / `trust` 留成了**单位结构体**（只定容器形状、取值域留空，
`crates/continuum-capability/src/tool.rs:176-251`），并在 P3A 设计 §10 第 9 条写明「它们服务的是子项目 D 的候选排序」。

**P3A 把三个字段一并判给了 D，本设计只接其中两个——`trust` 明写退件**（理由见本节末，记在 §11 第 23 条）。

**本设计的回答：D 是消费者，但 D 不是给出取值域的那一个。** 具体地：

- `ModelProfile.cost_profile` / `latency_profile` **复用** `continuum_capability::{Cost, Latency}`，
  以 `Option<Cost>` / `Option<Latency>` 出现（`None` = 尚未登记画像，与 P3A 的 `tool.cost` 列同一语义）。
- **不给出取值域**。理由：§247 与 §87 都只给了字段名；§333 的五个量纲也没有单位（见 §6）；
  在三个都没有单位的地方挑一个尺度填进去，就是 P3A 明令禁止的「预先发明度量」。故 P3A 遗留第 9 条的
  「可达 `Some` 的路径」在本设计里**仍然只有「画像已登记」这一个语义**，没有变成数值。见 §11 第 15 条。
- **为什么不另建一对自己的 `CostProfile` / `LatencyProfile`**：若本设计另建一对单位结构体，
  「一个成本画像」就有了两套词汇表，这正是本项目一贯判为 Critical 的那一类。**这是本设计依赖
  `continuum-capability` 的唯一理由**（§8）。被否掉的替代方案是「把 `Cost`/`Latency` 上移到
  `continuum-core` 再两边共用」——它更干净，但要改动 P3A 已落地的类型与它的落库编码，超出本子项目范围；
  记在 §11 第 6 条。

### `trust`：**D 不接，退件**（P3A 把三个字段一并判给 D，本设计只接两个）

P3A 设计 §3.1 把 `cost` / `latency` / `trust` **三个一起**写成「服务子项目 D 的候选排序」
（`crates/continuum-capability/src/tool.rs:171`、`:229`）。本设计**只接 `Cost` / `Latency`**，`trust` 退件，
理由三条，都是本设计自己文档里的判据而不是偏好：

1. **§247 的 `ModelProfile` 没有信任字段。** 十二个字段逐项核过（§2.1 的表）：`id`、`version`、`provider`、
   `model_revision`、`modalities[]`、`tools[]`、`skill_vector`、`failure_modes[]`、`latency_profile`、
   `cost_profile`、`evidence_count`、`confidence`——**没有 `trust`**。故模型侧**没有地方安放它**，
   而本设计**不擅自加一个 §247 没有的字段**（同一判据见 §2.1 对 §81 那两项的处置）。
2. **§250 的「最终选择 MUST 考虑」八项里也没有信任。** 八项逐项列在 §5.1：能力匹配／失败模式／成本／延迟／
   上下文长度／模态／工具支持／当前可用性——**无一项是 trust**。故路由侧同样没有它的消费点。
3. **D 不读 `ToolProfile`。** P3A 的 `trust` 是 `ToolProfile` 的字段；本设计复用的是 `Cost` / `Latency`
   两个**类型**，落在模型画像上，`ToolProfile` 本身在本 crate 里一次都没出现（§8.2 的边表可证）。

**它该去哪儿**：`trust` 的语义是「一个工具登记进 Registry 时必然有信任判定」（P3A §3.1），
故它的消费方是**决定要不要调这个工具**的那条路径——即工具调用路径（**子项目 F**，不是模型调用路径的 G）；
若最终判定规范里根本没有它的位置，则由规范维护者处置。**收件人：子项目 F ＋ 规范维护者**，
记在 §11 第 23 条。**退件不是「悄悄不管」**：本设计对它的处置就是这三条理由加这两个收件人。

---

# 3. 持久化

## 3.1 三张表，两个事实

§21（`docs/spec/01-concepts.md:991-1020`）叫 **Model Intelligence Registry**，管的是**生命周期**；
§247 叫 **Model Capability Profile**，管的是**画像**。这是两个对象，故两张表——一张表会让
「一个模型被登记了」与「一个模型有画像了」这两件事不可分辨，而 §21 的「新增模型不能直接进入自动 Router」
恰恰靠这两件事的差别成立（§4.2）。

```sql
CREATE TABLE model_registry (          -- §21 §249：登记与生命周期
    id               TEXT PRIMARY KEY, -- §81／§315 的模型 id
    lifecycle_state  TEXT NOT NULL     -- §249 十态，小写 _ 连接
);

CREATE TABLE model_profile (           -- §247：画像（probed → verified 时才产生，见 §4.3）
    id               TEXT PRIMARY KEY REFERENCES model_registry(id),
    version          TEXT NOT NULL,
    provider         TEXT NOT NULL,
    model_revision   TEXT NOT NULL,
    modalities       TEXT NOT NULL,    -- JSON 字符串数组
    tools            TEXT NOT NULL,    -- JSON 字符串数组，元素是 ToolId
    failure_modes    TEXT NOT NULL,    -- JSON 字符串数组
    cost_profile     TEXT,             -- NULL = 尚未登记；非 NULL = 已登记（P3A 的存在性编码）
    latency_profile  TEXT,             -- 同上
    evidence_count   INTEGER NOT NULL,
    confidence       TEXT NOT NULL     -- Ratio 的十进制串
);

CREATE TABLE model_skill_score (       -- §248 §24：时间序列，不是常数
    model_id       TEXT NOT NULL REFERENCES model_profile(id),
    dimension      TEXT NOT NULL,      -- §248 九维，小写 _ 连接
    score_version  INTEGER NOT NULL,   -- §24 的 version
    score          TEXT NOT NULL,
    confidence     TEXT NOT NULL,
    sample_count   INTEGER NOT NULL,
    time_range_start INTEGER NOT NULL, -- Unix 毫秒
    time_range_end   INTEGER NOT NULL,
    PRIMARY KEY (model_id, dimension, score_version)
);
```

四处要写明的：

1. **`model_skill_score` 不挂 `model_registry` 而挂 `model_profile`**：观测是画像的一部分，没有画像就没有观测。
   `load_skill_vector` 因此不可能是「对未画像的模型返回空向量」而只能是「返回该画像的向量」。
2. **主键 `(model_id, dimension, score_version)` 是「同一维度的同一版本只有一次观测」这条保证的落点**，
   与 P3A 的裸 `INSERT`（同 id 第二次写入由主键拒绝）同一判据。`model_skill_score` 同样是裸 `INSERT`，
   **不 `OR REPLACE`**，否则「补记一次观测」会静默覆盖历史，而 §24 要的正是历史。
3. **列表列取 JSON 数组容器**，与 P3A 的 `tool.required_capabilities` 同形（P3A 设计 §3.3 给的理由：
   元素是自由文本，定长分隔符在有取值域的那天会撞上取值本身）。这里的三个列表**都没有逐元素属性**，
   故不建子表；**`model_skill_score` 建子表是因为它有逐元素属性（版本、样本数、时间窗）与多版本**，
   不是因为它更长。这条分界是判据，不是巧合。
4. **`model_profile` 里没有 `overall_score`，也不会有**——见 §2.3 第 2 条的用例。

## 3.2 读写函数

与 P3A 同址（表定义、编码委托、行级读写同处一文件）。

```rust
pub fn p3d_model_migrations() -> Vec<Migration>;
pub fn save_profile(tx: &Tx<'_>, profile: &ModelProfile) -> Result<(), PersistError>;
pub fn load_profile(tx: &Tx<'_>, id: &ModelId) -> Result<Option<ModelProfile>, PersistError>;
pub fn save_skill_observation(tx: &Tx<'_>, id: &ModelId, dim: SkillDimension,
                              obs: &SkillObservation) -> Result<(), PersistError>;
pub fn load_skill_vector(tx: &Tx<'_>, id: &ModelId) -> Result<Option<SkillVector>, PersistError>;
pub fn load_skill_series(tx: &Tx<'_>, id: &ModelId, dim: SkillDimension)
                         -> Result<Vec<SkillObservation>, PersistError>;   // 按 score_version 升序
pub fn register_model(tx: &Tx<'_>, id: &ModelId) -> Result<(), PersistError>;  // §21 发现即登记
pub fn load_lifecycle(tx: &Tx<'_>, id: &ModelId) -> Result<Option<LifecycleState>, PersistError>;
/// 迁移一个模型的登记状态。**返回的是迁移前的旧态（`from`）**，不是 `to`——
/// 调用方要记「从哪来」，而「到哪去」它就是自己传的 `to`，返回它没有信息量。
/// 旧态另有一个来源：`load_lifecycle` 在同一事务里读。**若登记项不存在**：
/// `Err(LifecycleError::UnknownModel { id })`（**不静默创建**——登记是 `register_model` 的活）。
pub fn transition(tx: &Tx<'_>, id: &ModelId, to: LifecycleState)
                  -> Result<LifecycleState, LifecycleError>;                    // §4.3
```

`load_skill_series` 的消费方是 §82 的行为指纹（「如果表现突然变化」，`docs/spec/02-positioning.md:736-764`）
——判「变化」至少要看两次观测，故它现在就必须有；只存当前值会让 §82 的判据无法成立。
**它的产生方（周期性 probe）本阶段不存在**，这一点写在 §10 的「拍不到的照片」里，不靠一句将来时糊过去。

## 3.3 迁移编号

已占用（`crates/continuum-runtime/src/main.rs:57-65` 的装配集合）：`1`、`2`（P0 内建）、`10`（artifact）、
`20`（graph）、`30`（workspace）、`40`（effect）、`41`（policy）、`50`（P3A capability）——共 8 条。
「前几位是十位一档」是本仓的既有取法（P3A 计划第 441 行）。

**号段已由协调者裁定：一个子项目一个十位档——A 50、B 60、C 70、D 80、E 90**，每一档**取用前仍须核对该档未被占用**。
**本设计取 `80`，且只取一个**（三张表在一条迁移里，`Db::migrate` 用 `execute_batch`，一条迁移可含多条语句，
`crates/continuum-persist/src/db.rs:117`）。**不预留 `81`**——第一版稿子写过「预留 `81` 给后续 task 新增的表」，
**那句已删**（来历留在此段）：三张表全在 `80` 里，`81` 当下**没有使用点**，预留一个没有表要建的编号
就是留一条死迁移；而按本仓既有的分工，「用它的那个 task 自己注册迁移、自己取号」本来就不需要预留。
将来真有新表时，那一刻取一个未占用的号即可，与 P3A 的取号方式相同（判据是「未占用」）。

- **「未占用」的判据是按库说的，不是按全仓说的。** 编号在**别的库里**也出现是**无害的**：唯一的判据是
  「同一个库里的编号不重复」，而**驱动装配的那个集合是唯一会碰上面的一条**。两处实例：
  `crates/continuum-persist/src/bin/crash-writer.rs:16` 的 `crash_fixture` 用了 `60`
  （该二进制自建的库），`crates/continuum-persist/tests/recovery.rs` 的探针表也用过 `50`
  （P3A 在 `crates/continuum-capability/src/persist.rs:36-39` 就地订正过「50 未被占用」这条全仓层面的假命题）。
  **故本设计不声称 `80` 在全仓未被占用**——它只声称 `80` 在 `runtime_migrations()` 的集合里未被占用，
  且这一点由 `crates/continuum-runtime/tests/migrations.rs` 的 `migration_versions_are_unique` 逐条断言。
- B、C 若已取 `80`，实现时重编。此协调点记在 §11 第 14 条（现按裁定收敛为「核对」）。
- 注册由用它的那个 task 完成，`expected_migrations()`（`crates/continuum-runtime/tests/migrations.rs:30`）
  与 `startup.rs` 的计数断言随之更新——**这是两份转录，不是重复**，该文件的注释说明了理由。

---

# 4. Model Registry 生命周期（§249）

## 4.1 十个状态，一张迁移表

§249（`docs/spec/05-normative.md:856-884`）给十个状态名，**未给迁移关系**——与 §237 的情形相同
（`crates/continuum-graph/src/state.rs:1-2` 的原话：「规范只给出十三个状态名，未定义迁移关系；本表由 P1 设计
第 9 节确定」）。故本设计定一张迁移表，落成 `transition(from, to) -> Result<LifecycleState, LifecycleError>`，
与 `continuum-graph` 的同名函数同形（含 `Illegal { from, to }` 这一具体 `Err`）：

```rust
/// 本 crate 的生命周期错误。**不叫 `RegistryError`**——C 的设计在 `continuum-provider` 里
/// 已有一个同名不同物的 `RegistryError`（`NotFound` / `Duplicate`，是适配器注册表的错误）。
/// 两件事一个名字会让调用方与后来者混淆，与「同一件事两个词汇表」是同一种病灶的两面。
pub enum LifecycleError {
    /// 迁移对不在 §4.1 的表里。`from` / `to` 原样带出（形状取自 `continuum-graph` 的同名变体）。
    Illegal { from: LifecycleState, to: LifecycleState },
    /// `save_profile` 时登记项的当前状态尚未产出正式画像（§4.3）。
    ProfileBeforeVerified { state: LifecycleState },
    /// 登记项不存在。
    UnknownModel { id: ModelId },
    /// 读写出错。
    Persist(#[from] PersistError),
}
```

```
正常阶梯（线性、单调，§21 §22）:
  discovered → unprofiled → researched → probed → verified → active

边：
  verified → active            上线
  active   → stale             §82 行为指纹漂移
  stale    → researched        §82「重新 profiling」——回到画像流水线的入口
  active   → degraded          性能退化
  degraded → active            恢复
  *        → quarantined       §249 的异常态，可由任意状态进入
  *        → disabled
  quarantined → disabled
  disabled → unprofiled        重新启用须重走 §22 的 onboarding
```

三条设计说明，都是**被选中的方向**而不是规范给的：

- **`stale` 回到 `researched` 而不是 `active`**：§82 说漂移后要「重新 profiling」。若允许它直接回 `active`，
  「避免使用已经过时的 Model Profile」这句话就没有落点。
- **`* → quarantined` / `* → disabled` 可由任意状态进入**，与 P1 让 `INVALIDATED` / `CANCELLED`
  由任意状态到达同一判据（`crates/continuum-graph/src/state.rs:16-19`）：事故与人工下线不挑时机。
- **`disabled → unprofiled` 有一条件**：重新启用**必须重走画像流水线**（故出口是 `unprofiled` 而不是 `active`），
  依据是 §21「新增模型不能直接进入自动 Router」的同一条道理。被否掉的替代方案是「`disabled` 为终态」——
  它会让一次事故性的下线只能靠删行恢复，而删行会丢掉画像与观测历史。

**自环（`x → x`）在表里列到了的那些对上是合法的，其余一律 `Err(Illegal { from, to })`。**
判据就是上表本身：`* → quarantined` 与 `* → disabled` 里的 `*` **含它自己**，故
`quarantined → quarantined` 与 `disabled → disabled` 是 `Ok`（把已隔离的再隔离一次、把已停用的再停用一次，
都是无害的幂等动作）；而 `active → active`、`verified → verified` 这类**没有列进表**，
仍是 `Err(Illegal { from, to })`。

> **一处订正，来历留在原地**（本项目既有做法）：第一版稿子写的是「**自环（`x → x`）非法**，
> 未列出的对一律 `Err(Illegal …)`」——**那句与本节自己的表相抵**：`* → quarantined` / `* → disabled`
> 两条规则把 `quarantined → quarantined` 与 `disabled → disabled` 也判成 `Ok`，于是同一条断言在
> 两个地方给出相反结论。**一条与自己那张表相抵的「一律」正是本项目要消灭的形状**，故按表那一侧改。
> 另有旁证：P1 的既有实现 `crates/continuum-graph/src/state.rs:16-19` 恰恰**放行自环**
> （`if matches!(to, Invalidated | Cancelled) { return Ok(to); }` 在自环检查之前返回），
> 本设计与之同形。

## 4.2 可路由闸门：四态**不可进入路由路径**（§249 ＋ 已裁的一条）

§249 的硬约束：「Router MUST NOT 自动使用 `UNPROFILED` / `QUARANTINED` / `DISABLED` 模型」。
《工程》§4.2 重复了它（`docs/02-工程.md:237`）。

**这三点是下限，不是上限；`stale` 同样不得进入自动路由——已裁（2026-10-05，用户拍板）。**
依据是 §82 的 `ACTIVE → STALE → 重新 profiling`：一个已漂移的模型**正在退出服役**，
把它放行给自动路由是一个 **fail-open 的形状**，而本项目把 fail-open 当作要消除的形状而非可容忍的偏差。
裁决记录见 `docs/superpowers/specs/2026-10-05-p3bcdf-set-decisions.md` 第一节第 4 条。

**这不是「本设计的解释」，故不再有可推翻的标记。** 本设计先前在此写过
「这一条是协调者的解释，可被用户或规范推翻，此标记不省」——**那句已随裁决作废**（来历留在此段，
以便后来者看到这条禁令是从一次可推翻的解释升为裁决的）。仍要说清的是：**§249 的正文没有被改动**——
本设计读它「三态是下限」，并据裁决把 `stale` 一并挡下；这两句话不矛盾。

**`DEGRADED` 不在此列**，判据见本节末段。

**做成结构性保证，而不是「Router 记得跳过」**——与 P3A 的 `AuthorizedTool` 同一形（P3A 设计 §3.4）：

```rust
/// §249 的十个状态（照录）。
pub enum LifecycleState {
    Discovered, Unprofiled, Researched, Probed, Verified, Active,
    Stale, Degraded, Quarantined, Disabled,
}

/// 可被**自动路由**的那六个状态。
///
/// `unprofiled` / `quarantined` / `disabled` **不在此枚举里**——§249 禁的那三态
/// 在路由路径上无处安放（`stale` 是额外挡下的第四态，见本节上文的已裁那段：
/// `set-decisions` 第一节第 4 条）。
/// 这与 P3A §2.3「`full_access` 不是禁止作为默认，而是没有
/// 这个成员」是同一种做法。
pub enum RoutableState { Discovered, Researched, Probed, Verified, Active, Degraded }

impl TryFrom<LifecycleState> for RoutableState {
    type Error = RoutingError;   // NotRoutable { state: LifecycleState }
}

/// 一个**可交给 Router** 的模型：画像 + 已过闸门的状态。
/// 字段私有、无公开构造函数；唯一的产生点是 `RoutableModel::try_new`。
pub struct RoutableModel { profile: ModelProfile, state: RoutableState }

impl RoutableModel {
    /// **入参的状态是十态的 [`LifecycleState`]，不是收窄后的 [`RoutableState`]**——
    /// 闸门就在这个函数里发生；若入参已经是 `RoutableState`，闸门就跑到调用方去了，
    /// 那道「Router 忘了检查」的保证随之失效。收窄在函数体内经 `RoutableState::try_from` 完成，
    /// 失败即 `Err(RoutingError::NotRoutable { state })`，**`state` 带的是传入的那个十态值**。
    pub fn try_new(profile: ModelProfile, state: LifecycleState)
        -> Result<Self, RoutingError>;
}
```

于是 `pub fn rank(request, models: &[RoutableModel], policy) -> ...` 的签名里**不存在**
未画像、被隔离、被停用的模型——**「Router 忘了检查状态」这条路径在类型上不存在**。

**这里的措辞要与保证等强，故写准**：这四态在 `LifecycleState` 上**是表达得出来的**（画像流水线、漂移标记、
隔离与停用都要用它），被挡住的是**路由路径**。本设计**不**声称「四态不可表达」。

**逐项有照片**：十个状态各一条用例喂进 `RoutableModel::try_new`，六个返回 `Ok`，四个返回
`Err(NotRoutable { state })` 且**断言是哪一枚**（不是「返回了 Err」）。这是枚举式绝对断言，按本项目纪律**逐项**钉。

**`degraded` 仍可路由，且它不是 `ProviderHealth` 的那个同名变体。** 判据是出处：`DEGRADED` 出现在 §249 的
**模型生命周期异常态清单**里（`docs/spec/05-normative.md:868-871`），与 `ProviderHealth::Degraded`
（`crates/continuum-core/src/model.rs:83-87`，那是**供应商侧的可用性**）是两个轴上的两个东西，只是名字撞了。
本设计**不因这次撞名而动它**：§249 列了它，本轮裁定没有点它，故它保持可路由，状态原样带进候选的 `reason`
（§5.2），让策略可以据此降权——**可见但不禁**。

## 4.3 谁迁移

本层只提供**迁移表与写入口**；触发事件的产生方在别处：

| 迁移 | 触发事件的产生方 | 出处 |
|---|---|---|
| `discovered → unprofiled` | 发现新模型的登记方（`register_model`） | §21 |
| `unprofiled → researched → probed → verified` | **Model Onboarding Task**（尚未建） | §22 |
| `verified → active` | 上线动作 | §21 |
| `active → stale` | **行为指纹**的周期性 probe（尚未建） | §82 |
| `active ↔ degraded` | 性能观测（尚未建） | §249 |
| `* → quarantined / disabled` | 人工或事故处置（本阶段可由驱动命令触发） | §249 |

**本阶段这些产生方一个都不存在**：§22 的 onboarding 属于语义层/长期循环，§82 的 probe 属于 Runtime。
本设计因此只保证**闸门与迁移表可用、可拍**；「谁真的推进了生命周期」在本阶段只能由测试直接调 `transition` 来演。
这一条写在 §10 的「拍不到的照片」里。

**画像在 `probed → verified` 时才产生**（一个产生点）。这是一个**十项枚举**：允许集是
`{verified, active, stale, degraded, quarantined, disabled}`（画像已产出，异常态只是改了它的可用性），
拒绝集是 `{discovered, unprofiled, researched, probed}`（画像流水线尚未走完）。§9 按本项目纪律**逐项**钉
（十态各一条，四拒各断言是哪一枚），不只钉 `researched` 一条。`save_profile` 读登记项，若当前状态不在
`{verified, active, stale, degraded, quarantined, disabled}` 之内，返回
`Err(LifecycleError::ProfileBeforeVerified { state })`。依据是 §22 的流水线顺序——「生成初步画像 → 执行 Active Probe
→ Verifier → **生成正式 Profile**」，正式画像在 Verifier 之后。

**§22 的「初步画像」是一个被**刻意丢弃**的中间物，本设计不落它。** 这一句必须写明，否则读了 §22
的流水线的人会以为本设计漏了一环：§22 的顺序是「读官方文档 → 搜 model card → 收集 benchmark → 收集公开 failure mode
→ **生成初步画像** → 执行 Active Probe → Verifier → 生成正式 Profile」（`docs/spec/01-concepts.md:1022-1066`）。
初步画像**先于** probe，而本设计的 `model_profile` 行只在 `probed → verified` 时产生，故初步画像**没有落库的落点**。

**它不落库是决定，不是遗漏**：初步画像的全部内容来自公开资料、**样本数为零**；把它作为一个可路由的画像存下来，
等于给一个从未实测过的模型一个与实测画像同形的身份，而那正是 §21「新增模型不能直接进入自动 Router」要拦的。
**它的效果由 §247 的两个字段承载**——`evidence_count` 与 `confidence`：初步阶段二者分别是「证据条数」与
「低置信」，**正式画像生成时一并写入**（`evidence_count` 记入收集到的证据条数，`confidence` 记入画像的置信），
故初步阶段的成果**不是被丢掉，而是被折进正式画像的两个字段**。见 §11 第 16 条。

**这条与 §249 的关系要说清，免得被读成全称的**：§249 没禁 `discovered` / `researched` / `probed` 三态，
所以**类型闸门放过它们**；把它们挡在自动路由之外的**不是第二张禁令表，而是「没有画像就没有候选」**——
`rank` 收的是 `RoutableModel`，而它的构造需要一份 `ModelProfile`。于是 §21 的「新增模型不能直接进入自动 Router」
由**两件事的合取**成立：状态未过闸门，**或**尚无画像。见 §11 第 16 条（「初步画像是否算 ModelProfile」
规范未定义，本设计按 §22 的顺序取「不算」）。

---

# 5. Router（§250 §84）

## 5.1 §250 的六个输入逐项处置

| §250 输入 | 处置 | 说明 |
|---|---|---|
| `TaskSkillRequirement` | **本设计定义类型**：**非空**的 §248 维度集合（`try_new` 拒空） | 规范只给名字、未给形状。本设计的形状是「这个任务需要哪几个 §248 维度」，依据是 §250 的「能力匹配」以 §248 的向量为基准。**不带权重、不带阈值**——两者都是规范没有的数，加了就是发明。**非空是决定**，理由见下文「空需求」一段 |
| `ModelProfile` | **已有类型**（§2） | 经 `RoutableModel` 传入，见 §4.2 |
| `CostPolicy` | **不定义形状**，落在排序策略接口之后（§5.3） | 规范只给名字。见 §11 第 5 条 |
| `LatencyPolicy` | 同上 | 同上 |
| `FamilyPreference` | **本设计定义类型**：§19 的五项照录（`Auto` / `OpenAiPreferred` / `ClaudePreferred` / `LocalPreferred` / `Custom`，`docs/spec/01-concepts.md:933-960`），落库小写 `_` 连接（本阶段它不落库，只作输入） | §19 给的是**封闭清单**，可照录。`Custom` 在 §19 里不带载荷，本设计也不给它加。§19 的「明显收益足够高时才跨 family」**没有阈值**，见 §11 第 10 条 |
| `FailureHistory` | **不定义形状**，落在排序策略接口之后 | 规范只给名字，且**它的产生方也没有指定** |

### 请求与策略的分界（六个输入各落在哪一边）

```rust
/// 一次自动路由的请求。**纯数据，无 I/O**。
pub struct RoutingRequest {
    pub requirements: TaskSkillRequirement,   // §250 #1
    pub family: FamilyPreference,             // §250 #5
    pub availability: Vec<(ModelId, ProviderHealth)>,  // §250 的「当前可用性」
    pub budget: BudgetView,                   // ENG-005：**必填，不是 Option**
}

/// 非空的需求维度集合。空集合被构造期拒绝。
pub struct TaskSkillRequirement { dimensions: Vec<SkillDimension> }  // 字段私有

impl TaskSkillRequirement {
    pub fn try_new(dims: Vec<SkillDimension>) -> Result<Self, RequirementError>;  // 空 → Err
    pub fn dimensions(&self) -> &[SkillDimension];
}

pub enum RequirementError {
    /// 一个需求维度都没有。**具体是哪一种 `Err`，有用例。**
    RequirementEmpty,
}
```

| §250 的输入 | 落在 | 理由 |
|---|---|---|
| `TaskSkillRequirement` | **请求** | 每次路由都不同，且是排序的输入 |
| `ModelProfile` | **请求**的间接成分（经 `&[RoutableModel]` 传入，不在 `RoutingRequest` 里） | 它是**候选集**，不是单次请求的参数 |
| `CostPolicy` | **策略** | 它的形状规范未定义（§5.1 表），且它是「怎么算」而不是「算什么」 |
| `LatencyPolicy` | **策略** | 同上 |
| `FamilyPreference` | **请求** | §19 的封闭清单，纯数据 |
| `FailureHistory` | **策略**（今天不定义形状） | 同上；见 §11 第 18 条的签收与退件 |

**请求是纯数据、策略是可替换实现，这条分界是刻意的**：请求进得去表驱动用例，策略可以换一份重跑同一组用例。

### 空需求：为什么做成非空类型

`compatibility = matched / required` 在 `required` 为空时是 `0/0`。而 `CandidateScore.compatibility: Ratio`
**拒绝 NaN**（§2.4），`evaluate` 又**不返回 `Result`**——于是实现者只剩两条路：panic，或编一个值（如 1.0）。
**两条都能过 §9 的原有全套用例**，这正是本设计要堵的形状。选非空类型，被否掉的替代方案是
「规定空需求时 `compatibility = 1.0`」：那要为一个规范没定义的状态发明一个语义值，而「一次不需要任何能力的任务」
是不是一个合法的任务，规范没有说——**类型层拒掉它比替它选一个数诚实**。照片：`try_new(vec![])` 返回
`Err(RequirementEmpty)`（断言是哪一种），外加一条 trybuild 样例钉「字段私有、无 `From<Vec<_>>`」。

### §250 的「最终选择 MUST 考虑」八项，逐项

| MUST 考虑项 | 本设计 | 出处 / 处置 |
|---|---|---|
| 能力匹配 | 有 | §248 的 `SkillVector` ＋ 请求里的 `TaskSkillRequirement` |
| 失败模式 | **有字段、无因子** | §247 `failure_modes[]` 已存；**匹配规则无词表**，策略暂不用它。见 §11 第 9 条 |
| 成本 | **有输入、不可计算** | §250 点名的是**成本**；ENG-005 把它的输入定为**预算视图**（§6）。量纲无单位，故不可计算。见 §11 第 5 条 |
| 延迟 | **有输入、不可计算** | 同上：`latency_profile` 是空取值域的单位结构体（§2.5） |
| **上下文长度** | **无输入** | **§247 的十二个字段里没有它，§250 的六个输入里也没有。** §22 的 Profile 清单有 `context`（`docs/spec/01-concepts.md:1049`），§315 的 `ModelDescriptor` 有 `context_window`（`crates/continuum-core/src/model.rs:25`）。本设计**不擅自加一个 §247 没有的字段**，故该因子**未实现**。见 §11 第 19 条 |
| 模态 | 有字段，策略可读 | §247 `modalities[]`；`ModelProfile::modalities()` 是显式给出的访问器（§2.1） |
| 工具支持 | 有字段，策略可读 | §247 `tools[]`（`ToolId`）；`ModelProfile::tools()` 同上 |
| 当前可用性 | 有 | `RoutingRequest.availability`，取 §315 的 `ProviderHealth`（§1.2） |

**这一表是为了堵一处漏项**：本设计先前只为 `failure_modes` 写了「该因子未实现」，却对**同属 §250 MUST 考虑、
同样没有输入的 `上下文长度`** 一字未提——枚举式断言漏了一项。

## 5.2 输出：`RankedExecutionCandidates`

§250 的输出是 `RankedExecutionCandidates`；§84 说「每次路由除输出 `selected_model` 外，还应输出
`confidence`、`alternatives` 和 `reason`」。两者合起来只有一个自洽读法：**输出是有序的候选列表，
表头即 `selected_model`，其余即 `alternatives`**——故**不另设 `alternatives` 字段**（那会是同一份数据的第二个落点）。

```rust
pub struct RankedExecutionCandidates { candidates: Vec<ExecutionCandidate> }  // 字段私有

impl RankedExecutionCandidates {
    pub fn selected(&self) -> &ExecutionCandidate;      // §84 的 selected_model
    pub fn alternatives(&self) -> &[ExecutionCandidate]; // §84 的 alternatives
    pub fn candidates(&self) -> &[ExecutionCandidate];
}

pub struct ExecutionCandidate {
    model: ModelId,
    state: RoutableState,     // §249 的状态对策略可见（degraded 据此降权，见 §4.2）
    compatibility: Ratio,     // §84
    confidence: Ratio,        // §84
    reason: RoutingReason,    // §84
}

/// §84 的 reason。结构化而非一行文本：字段要供后续对账取用（与 P3A 的审计 payload 同一判据）。
pub struct RoutingReason {
    family: FamilyRelation,       // SameFamily | CrossFamily
    matched: Vec<SkillDimension>, // 命中的需求维度
    missing: Vec<SkillDimension>, // 未命中（**不是**「差」，见下）
    notes: Vec<String>,           // 策略写入的事实陈述
}
```

**`ExecutionCandidate` 必须开访问器，这是接口冻结处的必需品，不是提前铺开的 API。**
`selected()` 是要**交给消费者**的，而消费者下一步就是拿着 `model` 去（在它自己那一层）调 provider——
拿不到 `ModelId` 这一步就走不下去。故下列访问器各有具名消费方（**子项目 G**，§1.2 末段），**不是「将来可能有人用」**：

```rust
impl ExecutionCandidate {
    pub fn model(&self) -> &ModelId;         // 消费方据此发起调用
    pub fn state(&self) -> RoutableState;    // 消费方据此决定是否降权／重试
    pub fn compatibility(&self) -> Ratio;    // §84
    pub fn confidence(&self) -> Ratio;       // §84
    pub fn reason(&self) -> &RoutingReason;  // §84
}
```

`RoutingReason` 同样开 `family()` / `matched()` / `missing()` / `notes()` 四个访问器，但**它们今天没有具名消费方**——
本设计只声称它们「供后续对账取用」（§84 要求输出 reason 本身即是消费方）。**这句话刻意写得比上面弱**：
五个 `ExecutionCandidate` 访问器是**必需的**，四个 `RoutingReason` 访问器是**§84 要输出的内容的读口**。

三条断言，都问过「它在每个产生方上都为真吗」：

- **`candidates` 非空**：`rank` 在任何情况下都不返回空列表——无候选时返回
  `Err(RoutingError::NoEligibleCandidate)`（§5.4）。故 `selected()` 不必返回 `Option`。
  这条由**构造函数的私有性**保证（`RankedExecutionCandidates` 字段私有，唯一构造点在 `rank` 内，
  构造前先判空），有用例。
- **`missing` 用的是 `Vec<SkillDimension>` 而不是一个数值**：本设计**不把「缺一个维度」折算成任何扣分**，
  因为那需要一个规范没有的权重（§2.4）。理由是同一个：量纲未知时只做集合运算，不做算术。
- **`confidence` 与 `compatibility` 都是 `Ratio`**：§84 的示例给的就是 [0,1] 的两个数。
  **它们的含义规范没有定义**（§84 只说「A 更合理」），本设计不替它定义，只保证它们是 [0,1] 的有限实数、
  因而可比较、可记录。见 §11 第 4 条。

## 5.3 排序：接口在本层，具体打分**明确推迟**

§84 只说「A 更合理」，§248 只说不许用总分，**规范没有给打分函数**；§25 的 Population Feedback
是 OPEN-008（`docs/02-工程.md:271`），冷启动期「Router 退化为仅使用画像与个人反馈」；§333 的量纲没有单位（§6）。
把这些合起来，本设计**不假装能写出一个规范的排序器**。它分两步：

**第一步（本设计交付）**：排序的**机制**与**接口**。

```rust
/// 一个候选的打分。**没有总分字段**——§248 的禁令在接口上也看得见。
pub struct CandidateScore {
    pub compatibility: Ratio,
    pub confidence: Ratio,
    pub reason: RoutingReason,
}

pub trait RankingPolicy {
    fn evaluate(&self, request: &RoutingRequest, model: &RoutableModel) -> CandidateScore;
    /// 全序比较：`Ordering::Less` 表示 `a` 排在 `b` 前面。
    /// 缺省实现按 (compatibility, confidence) 降序，再按 family、model id 兜底。
    ///
    /// 操作数就是 [`ExecutionCandidate`]——**不另立一个 `Scored` 类型**：
    /// 打分完的候选本来就要装成它，再立一个「已打分候选」类型就是同一件事的第二个落点。
    /// 缺省实现经 §5.2 的访问器读数，不动私有字段。
    fn compare(&self, a: &ExecutionCandidate, b: &ExecutionCandidate) -> Ordering { /* 缺省 */ }
}
```

`rank` 的骨架：过闸门的候选逐个 `evaluate` → 装成 `ExecutionCandidate` → `sort_by(policy.compare)` → 取头。
**排序是全序且确定的**：`compare` 的兜底一档按 `ModelId` 升序，使同分候选的次序不随输入顺序漂移
（用例钉：打乱输入顺序，输出不变）。**「全序」是本层的一个断言，故须逐条落实**：`Ratio` 拒 NaN 保证前两档可比，
`ModelId` 升序保证兜底档是**全序的最后兜底**（`ModelId` 两两可比且无相等），故 `compare` 的总序成立；
若两条候选连 `ModelId` 都相同，那它们本就是同一个模型的两次打分——该情形由「候选集来自 `&[RoutableModel]`
且同一 id 不重复」排除，建候选集时判重，重复即 `Err(RoutingError::DuplicateModelCandidate { id })`。

**第二步（明确推迟）**：具体打分函数的**数值**。推迟的三条依据，每条都有出处，不是「以后再说」：

1. §84 未给 `compatibility` / `confidence` 的算法与含义；
2. §25 Population Feedback 未定义（OPEN-008），冷启动期可用证据残缺；
3. §247 / §87 / §333 的量纲都没有单位，成本与延迟因子**算不出来**（§6）。

**但本设计交付一个具名基线策略**，使 §4.4 的完成判据「Router 输出带 confidence 的候选排序」**现在就可拍**：

```rust
/// 只用**已定义**的输入：§248 维度的**有无**（不读分数）、§247 的 confidence、§19 的家族偏好、§249 的状态。
pub struct BaselineRankingPolicy;
```

- `compatibility` = 需求维度中**有观测**的那几个的占比（`matched / required`）。
  **只用「有没有观测」，不读 `score` 的数值**——这正是 §2.4「只用序、不用量」的落实：
  使用占比（一个有界的集合运算结果）而不使用任何跨维度求和。
- `confidence` = 画像上的 `confidence`（§247）。
- `reason` 记 `matched` / `missing` / family / state。
- family 偏好按 §19 排在数值之前：`SameFamily` 优先于 `CrossFamily`（`Auto` 时全部视为 `SameFamily`）。

**基线是具名的、可替换的，不是对规范的声称。** 被否掉的两个候选打分法：**(a) 分数加权求和**——需要
一个规范没有的尺度与方向；**(b) 阈值匹配**——需要一个规范没有的阈值。两者都撞在 §2.4 的同一条判据上。

### 「当前可用性」怎么用：**过滤，且只过滤 `Unavailable`**（写死，不留计划自定）

§250 的八项 MUST 考虑里有「当前可用性」，而它是**唯一一项八项里既能判、又只判一档的**。本设计写死如下：

- **`ProviderHealth::Unavailable` 的模型不进候选集**。这不是发明阈值，而是 §250 的输出本身要求的：
  输出是 `RankedExecutionCandidates`——**可执行的**候选；一个供应商侧已不可用的模型不是执行候选，
  把它排进去，§1.2 说的「拿它跑」那一步必然失败，排序因此变成空话。**故这一条是「不可用即不是候选」，
  不是「可用性低就降权」。**
- **`Healthy` 与 `Degraded` 都进候选集**，且**两者之间没有判据**：`Degraded` 该不该降权、降到什么程度，
  **规范未给判据**（§250 只说「考虑」，§84 没有给这一维的算法），故本设计**不发明**——
  它把健康度原样带进 `reason`，让策略自己决定；**基线策略不因 `Degraded` 改变排序**。
- **`availability` 里没有条目的候选**（含候选集里有、列表里无）：`Err(RoutingError::UnknownAvailability { id })`，
  **不当作可用**——按未知放行是 fail-open 的形状。

**照片**：`Unavailable` 被排除（一条）、`Healthy`/`Degraded` 都留下且**排序不变**（一条，逐项）、
缺条目得 `Err(UnknownAvailability { id })`（一条）。**`Degraded` 的降权判据落下一条遗留**（§11 第 24 条）。

**§250 的「MUST 考虑成本」在本设计里只是结构性地不可省略，而不是已实现**：`RoutingRequest` 里
`budget: BudgetView` 是**必填参数**（不是 `Option`），故一个策略想忽略预算，是一次**看得见的选择**，
而不是一次遗漏。它的量值无法计算的理由见 §6。**这一条是本设计最需要在复审里被挑战的地方**，
故写在此处而不藏在脚注里。

## 5.4 失败路径：说清是哪一种 `Err`

```rust
pub enum RoutingError {
    /// §249 的三态，加本轮裁定挡下的 `stale`（§4.2）。`state` 原样带出，
    /// 调用方可分辨是被隔离、没画像，还是画像已漂移。
    NotRoutable { state: LifecycleState },
    /// 一个候选都没有（含「全部被闸门挡下」与「一个模型都没登记」两种情形）。
    NoEligibleCandidate,
    /// 候选集里同一个模型出现了两次。**它是 `compare` 的「全序」这条断言的守门人**：
    /// 两条 `ModelId` 相同的候选无从定序，`ModelId` 兜底档也就兜不住。
    DuplicateModelCandidate { id: ModelId },
    /// 候选集里的某个模型在 `RoutingRequest::availability` 里没有条目（§5.3）。
    /// **不把它当「可用」**：缺席即未知，而按未知放行是 fail-open 的形状。
    UnknownAvailability { id: ModelId },
}
```

每条各有用例断言**具体是哪一枚**。`NoEligibleCandidate` **不合并**进 `NotRoutable`：前者是「没有可用的」，
后者是「有一枚被点名挡下了」，调用方（§110 的流程）对两者的处置不同。

**这个枚举里只有 `rank` 真的会产出的变体，三个「没有产生方」的一律不收**——本仓对这类变体的处置是删或写明理由：

- **`Persist` 已删**（原第一版有 `Persist(#[from] PersistError)`）：**`rank` 是纯函数，签名里根本没有 `Tx`**
  （§8.1「Router 的纯由签名保证」），故它**产不出**读库失败。留着它会让人以为本层会写库，
  且它是一个死物。**这不同于「分支不可达但保留」那一类**：那一类的前提是类型确被别处需要，此处不是。
- **`Requirement(#[from] RequirementError)` 不收**：空需求在 `TaskSkillRequirement::try_new` 就被拒，
  而 `RoutingRequest` 装的是一个**已构造的** `TaskSkillRequirement`，故「空需求」这条路径**到不了 `rank`**。
- **`ProfileBeforeVerified` 不收**：它是 [`LifecycleError`] 的变体（§4.1、§4.3），产生方是 `save_profile`，
  不是 `rank`；本层把两类错误分开，不合成一个「什么都收」的枚举。

---

# 6. 成本输入 = 预算视图（ENG-005）

## 6.1 接口：本层定义只读投影，驱动把它投影出来

ENG-005 裁决：D 的成本输入是**预算视图**（剩余额度，不是「一个数」），它来自**语义层**，而语义层尚未建
（`docs/superpowers/specs/2026-10-05-eng-005-budget-accounting.md` 第五节；共享面第三节）。
§9.1 的层间方向是 **`语义层 (2) → 资源层 (4)`**（`docs/02-工程.md:560-579`，「依赖方向单向，无环」）。

**先定方向，因为它决定这个类型最终归谁。** §9.1 的箭头读**被依赖者 → 依赖者**（判据见 §6.3 那段订正：
§2.1 说「语义层……**本层不依赖任何下层**」，`docs/02-工程.md:72`，而 §9.1 画了 `语义层 (2) → 执行层 (3)`／`→ 资源层 (4)`
——只有一种读法能让这两处同时成立）。故 `语义层 (2) → 资源层 (4)` 说的是**资源层依赖语义层**，
本层是**消费者**。方向定下来之后，剩下的是「本层消费谁」——**答案不是「消费语义层的类型」**，
因为语义层未建；见下。

**但语义层尚未建，本层无法引用一个不存在的 crate，故本层先定义这个类型。**

**投影由驱动做，不由语义层做——这一条决定了过渡期也不出现反向边。** 本设计先前的措辞是
「过渡期后语义层来生产它」，**那句是错的，已订正**（错误说法的来历留在此段）：若语义层去「造一个资源层的类型」，
在它落地的那一刻就写出了 `语义层 → 资源层`，而类型搬迁写在「落地**之后**」——**生产者出现与类型搬迁之间有一个窗口，
那个窗口里正是本节声称避免的反向边**。订正后的分工：

- **语义层产出它自己的 `Budget`（§333 的分配对象）**，那是它本来就有的东西；
- **驱动把它投影成 `BudgetView`** 传进 `RoutingRequest`。驱动**本来就同时依赖两侧**（它装配迁移、组装
  `RoutingRequest`），故这条投影不新增任何边；**这与「驱动把可用性快照传进来」是同一个形状**（§1.2）。
- 本层的 `BudgetView` 因此**不需要任何一方来「实现」它**，只需要驱动构造它——**过渡期没有生产者依赖本层**，
  `语义层 → 资源层` 这条反向边在任何时刻都不出现。

**类型的归属**：`BudgetView` 是资源层的类型（它描述的是本层排序要用的输入形状），保留在本层；
语义层的 `Budget` 是另一个类型（含分配与预留），两者**不是同一件事的两个词汇表**——
一个是**记账对象**，一个是**只读投影**。若复审判定二者应合一，那是把投影塞进记账对象里，
本设计不同意，但记在 §11 第 5 条供推翻。

```rust
/// §333 Budget 的**只读投影**：该任务当前可用的剩余额度。
///
/// 五个字段**照录 §333**（`docs/spec/05-normative.md:2502-2516`）：
/// money / wall_time / token / gpu_time / network_transfer。
///
/// **单位与取值域规范未定义**（§333 只给字段名）。本层**不解释、不换算、不记账**：
/// 它只把这个投影交给排序策略。`None` = 该量纲当前不构成约束。
pub struct BudgetView {
    pub money: Option<i64>,
    pub wall_time: Option<i64>,
    pub token: Option<i64>,
    pub gpu_time: Option<i64>,
    pub network_transfer: Option<i64>,
}
```

**路由器是纯函数，它不「调用」语义层**：`BudgetView` 由调用方（驱动）从语义层取好后作为值传入。

**共享面第三节要求「把这个视图写成**一个待实现的接口**」，本设计的兑现方式是**一个类型、加一个构造方**，
不是一个 trait：`BudgetView` 的字段是 `pub` 的（它是一个**纯数据投影**，没有需要靠私有性守的不变量），
**驱动直接构造它**。**不定义 trait 是刻意的**——一个没有实现者的 trait 是**假接口**，
而本仓对「声明了没有产生方的东西」一贯的处置是删或写明理由，此处按同一条办。
若语义层落地后确实需要多态（例如「剩余」有两种取法），那时再抽 trait，并同时补照片。

## 6.2 可以假设什么、绝不自己算什么

**可以假设**（三条，都是 ENG-005 明文给的）：

1. 视图记的是**剩余额度**，是「成本相对什么来算」的那个基准（ENG-005 第五节）；
2. 视图的生产方**将来**是语义层的 Budget Validator（ENG-005 第二节第 1 条点名的组件）；
3. 一个节点的预算与一次探索的预算都是**同一棵预算树上的分配**（ENG-005 第三节第 1 条），
   视图因此可以按量纲给出剩余，而不必区分来源。

**绝不自己做**（按 ENG-005 的归属，逐条列出，免得越界）：

| 不做的事 | 归属 | 出处 |
|---|---|---|
| **预扣 / 结算** | 语义层 | ENG-005 第三节第 1 条 |
| **父子分配**（子 Intent 的预算、兄弟预留之和 ≤ 父剩余） | 语义层 | 同上 |
| **判「预计超预算」**（§110 的 Budget Validator） | 语义层 | ENG-005 第二节第 1 条 |
| **运行期超支的裁决**（`Constraint` + `EscalationPolicy::Decision`） | 失败分类与驱动 | ENG-005 第三节第 2 条 |
| **§334 的高成本探索阈值与 Decision Gate** | 语义层 | ENG-005 第三节第 1 条末行 |

## 6.3 一处**没有**按 ENG-005 办的事，连同理由

ENG-005 第五节把「`execution_profile.cost_budget` 的类型收紧」列为「**D 落地时**」。
**本设计仍然不做这一步**，理由两条（**第一版稿子里还有第三条「会写出反向边」，那条已作废**，见下）：

- **要收到的那个类型现在不存在，且它不该由本层发明。** 按 §246 与 ENG-005，节点预算是**语义层的 `Budget`（分配）**；
  语义层未建。本设计若替它发明一个中间类型占位，正是本项目明令禁止的「预先发明」。
  故这一条的**解除条件是语义层落地**，不是本子项目的某个 task。
- **角色不同：`cost_budget` 是一个上限，`BudgetView` 是一个余量。**
  `ExecutionProfile.cost_budget`（`crates/continuum-graph/src/execution.rs:31`）是 §246
  （`docs/spec/05-normative.md:796`）的**节点级声明式约束**；本设计的 `BudgetView` 是**剩余额度的投影**。
  把余量放进约束的位置，会让同一个类型有两个含义——那是本项目一贯判为缺陷的那一类。
  故「用 `BudgetView` 顶上去」这条替代方案**被否掉**，理由是角色的混同，与依赖方向无关。

**作废的那条理由，连同它为什么错，留在原地**（订正时保留错误说法的来历，是本项目的既有做法）：

> ~~「`continuum-graph` 属**执行层 (3)**，引用语义层的类型会写出 `执行层 → 语义层` 这条**反向边**。」~~

**这一句建立在一条**反了的**箭头约定上。** §9.1 的箭头读**被依赖者 → 依赖者**，不是依赖者 → 被依赖者。
判据两条：§9.2 说「入度为零的组件……**不依赖任何其他组件**，是依赖图的源点」（`docs/02-工程.md:588`），
故「入度为 0」= 无上游 = 被依赖者；代码亦印证——`continuum-policy`（边界层 5）依赖 `continuum-effect`（执行层 3），
见 `crates/continuum-runtime/tests/dependency_direction.rs:67-81` 的 `ALLOWED` 条目，而 §9.1 画的正是
`执行层 (3) → 边界层 (5)`（`docs/02-工程.md:565`）。故**执行层引用语义层，正是 §9.1 已列的那条
`语义层 (2) → 执行层 (3)`，不是反向边**。ENG-005 把这一步判给 D，在依赖方向上**没有越界**。

**这段订正现已升格为规范侧的明文**：2026-10-05 的规范侧订正已把这条读法**写进 §9.1 正文**
（并补上原图缺的 `资源层 → 边界层`），依据见 `docs/superpowers/specs/2026-10-05-p3bcdf-set-decisions.md`
第二节。故本节不再是「本设计的一种读法」，而是与规范一致。

记在 §11 第 12 条，收件人是协调者与语义层设计。

---

# 7. 升降级与路由探索（§251 §85 §86 §27）

## 7.1 升级与降级：阶梯的形状在本层，触发不在

《工程》§4.1 把「升级与降级」列在资源层。本设计把它的**数据形状**建在本层，把**触发**留给别处：

```rust
/// §251 的阶梯五档，**逐档照录**（`docs/spec/05-normative.md:922-944`）。
/// `Tier2High` 是 §251 的原词：按 §17／§18「能力档位与推理强度分离」，
/// 它对应的是「Tier2 ＋ 高推理强度」这个**组合**，不是第六档能力。
/// 本类型**不另建** `Tier` / `ReasoningEffort` 两个枚举：本层没有消费者需要按轴拆分，
/// 拆了就是给两个没有产生方的类型建形状（「不预先发明」）。
pub enum EscalationStep {
    Tier1,
    Tier2,
    Tier2High,
    CrossFamily,
    Specialized,
}

/// §251 的阶梯（照录其**顺序**）：Tier1 → Tier2 → Tier2High → CrossFamily → Specialized。
pub struct EscalationLadder { steps: Vec<EscalationStep> }
```

**本层建**：阶梯的**有序步骤**，以及「下一档」这个纯函数 `next_step(cur) -> Option<EscalationStep>`。

**§86 的 `Tier 1 Low` 不在这五档里，且这是规范自身的不一致。** §86 的降级例写「复杂规划 → Tier 2 High，
随后 400 个机械文件检查 → **Tier 1 Low**」（`docs/spec/02-positioning.md:847-867`），而 §251 的阶梯五档里
没有 `Low` 这一档——「Low」是 §18 的**推理强度**取值，不是 §17 的 Tier。本设计的处理是：
**§86 说的那一档对应「Tier1 ＋ 低推理强度」，本设计不为它加第六个成员**（§7 的 `EscalationStep`
是 §251 阶梯的照录，加一个不在阶梯里的成员会让「照录」这句话变假）。
这条不一致记在 §11 第 21 条。它在本层不致命：降级走「第二次调用 `rank`」，**不经过 `next_step`**（§7.1 末段）。

**本层不建**：

- **失败检测与升级触发**——它要读一次执行的失败（§309 的 `FailureClass`），那是**子项目 G**的活
  （G 做**模型侧失败分类**，见 `set-decisions` 第一节第 1 条）；**不记在子项目 F 名下**：
  F 是工具调用路径，与这里要读的模型调用失败不是同一条路径（§1.2 末段）；
- **§251 的前提检查「仍满足 Contract 和 Budget」**——按 §110，判预算是 **Budget Validator**（语义层）；
- **降级（§86）的调度**——「400 个机械文件检查降回 Tier 1 Low」是**任务级**的模型再选择，
  即用一个新的 `TaskSkillRequirement` 再调一次 `rank`。**本设计不为它建任何新机构**：
  intra-task model scaling 因此是「同一入口的第二次调用」，不是一条新路径。这是本设计在此处的**全部**主张。

**为什么这样切**：若本层自己判失败，就会出现「同一个失败两个裁决者」（驱动按 §309 判、本层又判一次），
正是 ENG-005 被否掉的 (b) 方向（多层各自裁决）的同一种病灶。

## 7.2 路由探索（§27）：不建，且理由是双重的

**不建**。不是推迟到以后再说，而是**现在建就要挑边站**：

1. **规范层有未决冲突**：C1 / OPEN-014（`docs/01-总纲.md:1725`、`:1892`）——§249 禁止自动使用 UNPROFILED 模型，
   而 §27 的探索天生要把任务分给「当前不是最高分」的模型，§22 的 Active Probe 又必须在任务分布上跑。
   「probe 是否属于 §249 所称的使用」**规范未定义**。本设计按 §249 的封闭清单办（§4.2），
   而探索的落点恰恰悬在这个冲突上。
2. **它的输入不存在**：§27 说可用 contextual bandit 类策略，那需要反馈信号；§25 的 Population Feedback
   是 OPEN-008（未定义）。没有反馈，探索概率就没有可调的依据。

故本设计**不建探索概率、不建 bandit、不建 exploration 配额**。§334 的探索预算记账归语义层（ENG-005），
本层也不碰。记在 §11 第 2、3 条。

---

# 8. 依赖与 crate 划分

## 8.1 一个新 crate

```
continuum-model-registry   （资源层：§247–§251 的画像、Registry、Router）
```

```
crates/continuum-model-registry/
  Cargo.toml
  src/lib.rs          导出面与 crate 文档
  src/profile.rs      ModelProfile、SkillVector、SkillDimension、SkillObservation、SkillScore、Ratio
  src/lifecycle.rs    LifecycleState、RoutableState、RoutableModel、迁移表
  src/persist.rs      三张表的迁移与读写（与表定义同址）
  src/router.rs       RoutingRequest、RankedExecutionCandidates、RankingPolicy、BaselineRankingPolicy
  src/escalation.rs   EscalationStep、EscalationLadder、next_step（§7.1）
  src/budget.rs       BudgetView（§333 的只读投影）
  src/error.rs        LifecycleError、RoutingError、ProfileError、RequirementError
  tests/{profile,lifecycle,persist,router,escalation,budget}.rs
  tests/compile_fail/*.rs
```

**为什么是一个 crate 而不是两个（Registry / Router 分开）**：`ModelProfile` 是两者共享的类型，
拆开就要为它写一条内部边，而那条边除了搬运类型之外没有别的用途。**Router 的「纯」由签名保证**——
`rank` 收 `&[RoutableModel]` 与 `&RoutingRequest`，不接 `Tx`，故它够不着库。

**为什么叫 `continuum-model-registry` 而不叫 `continuum-model`**：`continuum-core` 里已有一个 `model` 模块
（§315 的接口类型，`crates/continuum-core/src/model.rs`）。同层里两个都叫「model」的东西，会成为
「同一件事两个词汇表」的入口。被否掉的替代方案即 `continuum-model`。

## 8.2 依赖边（只登记实际用到的）

```
continuum-model-registry → continuum-core, continuum-capability, continuum-persist
```

| 边 | 用到的具体东西 | 判据 |
|---|---|---|
| `continuum-core` | `ModelId`（§315，§2.1）、`ToolId`（§252/§316，§247 的 `tools[]` 元素）、`ProviderHealth`（§315，§5.1 的可用性输入） | 三个都是**已存在的类型**，本设计一个都不新建 |
| `continuum-capability` | `Cost`、`Latency` | §2.5：复用而不另建一对，否则「一个成本画像」两套词汇表 |
| `continuum-persist` | `Migration`、`Tx`、`Value`、`value::kind_name`、`PersistError` | §3 的三张表：本仓的数据库访问一律经 `Tx` |

**明确不登记的边，以及为什么**：

- **不登记 `continuum-events`**：§313 的 `AuditKind` 八个变体里（`crates/continuum-events/src/audit.rs:18-35`）
  **没有**「模型登记变更」与「路由决定」这两类，本设计也没有找到必须记的理由。为它新增一个变体等于
  扩 §313 的必录清单——那是规范的事，不是本子项目的事。**零使用的边即假边**（P2b 为此删过两条）。
- **不登记 `continuum-provider`**：见 §1.2，可用性以值传入。
- **不登记 `continuum-graph`**：见 §6.3。
- **不登记 `continuum-effect` / `continuum-workspace` / `continuum-sandbox` / `continuum-policy`**：无任何使用点。

`continuum-runtime` 将来新增一条边（装配迁移 + 调用 Router），由**用它自己的那个 task**加上。

## 8.3 `ALLOWED` 表

`crates/continuum-runtime/tests/dependency_direction.rs:25` 的 `ALLOWED` 逐对 `assert_eq!`。
本 crate 的条目**按实际依赖**登记，边**由需要它的那个 task 增量加上**，不在 Task 1 一次声明齐
（P3A 设计 §6 判据：一次声明齐会让中间若干 task 的条目在说谎）。

## 8.4 与《工程》§4.3 的两处不一致（**两处均已订正**）

`docs/02-工程.md:245-254` 的层内依赖图里有两条边与本设计的实际依赖不符：

| §4.3 的原样 | 本设计的读出 | 现状（2026-10-05 核对） |
|---|---|---|
| `Provider Adapter → 被 Router 调用，接口中立` | 适配器**不被 Router 调用**；可用性以值到达 Router，调用方是**子项目 G**（模型调用路径） | **已订正**：该行现读 `Provider Adapter → 被模型调用路径（子项目 G）调用，接口中立`（`docs/02-工程.md:249`）——与本设计的读法一致，并附了订正日期与来历 |
| `Router ← Model Registry + Capability Token` | 到 `continuum-capability` 的那条边**存在但含义不同**：本层取的是 `Cost`/`Latency` 两个画像容器，**不是** Capability Token | **已订正（2026-10-05，commit `a139fa9`）**：该行现读 `Router ← Model Registry + continuum-capability 的两个类型（Cost / Latency）`（`docs/02-工程.md:248`），并附了来历注记——与本设计的读出一致 |

**两处都已落地**：第一处依据 `docs/superpowers/specs/2026-10-05-p3bcdf-set-decisions.md` 第二节；
第二处依据同一次收口（commit `a139fa9`）。故 §11 第 13 条里对应的两项都可销。

**这条记录前后翻过两回，三段来历都留在此处**——本项目最常犯的形状正是「**一处事实的三种记法各自漂移**」，
所以三种记法都要看得见，而不是只留最后那一种：

1. **第一版：据转述写成「已订正」（过头话）。** 协调者在派工口信里说「§4.3 那两处订正已在」，
   本设计照转述写进了「两处均已订正」。
2. **第二版：据实移回「未订正」。** 定点复核时去读 `docs/02-工程.md`，发现 `:248` 那行**原样未动**、
   无订正注记，于是改回未闭，并写了「核对日期 2026-10-05，不等于『已经改过』」。
3. **第三版（当前）：规范侧真订正后复归「已订正」。** commit `a139fa9` 把那行改成
   `Router ← Model Registry + continuum-capability 的两个类型（Cost / Latency）`，并附来历注记。

**教训留在此段**：第 1 版的错不在「转述」，而在**没有在读源文件之前就写定论**——
第 2 版的订正之所以必要，是因为第 1 版把一次口头的「已做」当成了文件事实。
**一条关于别的文件的断言，判据只能是那个文件本身。**

第二处的判据（记此以免被当成口味）：`Router ← Model Registry + Capability Token` 里
`← Model Registry` 那半没有争议（§5 的 Router 正是读画像）；争议全在 `+ Capability Token`——
本设计对 `continuum-capability` 的依赖是**取 `Cost` / `Latency` 两个类型**（§2.5），
**不是取「能力凭据」**：全仓找不到 Router 对 `Capability` / `AuthorizedTool` 的引用，
路由选的是**模型**，能力约束的强制点在工具调用路径（归子项目 A 与 F）。
**这条边在 crate 层面成立**（本设计确实依赖那个 crate），**在组件层面名不符实**，
故订正的方向是**含义**，即现在《工程》写的那一版。

---

# 9. 测试策略

| 验什么 | 怎么验 |
|---|---|
| §249 三态＋`stale` 不入路由路径 | **十态逐项**：六态 `Ok`、四态 `Err(NotRoutable { state })` 并断言是哪一枚 |
| `RoutableModel` 不可外部构造 | trybuild 样例（与 P3A 的 `AuthorizedTool` 同形） |
| `overall_score` 读不到 | trybuild 样例；另：`SELECT overall_score FROM model_profile` 得到具体 `Err` |
| `ModelProfile` 无总分 | 断言 `model_profile` 的列清单**逐列**（加一列即红） |
| 生命周期迁移表 | 合法对逐条 `Ok`；**未列出的对**给若干条 `Err(Illegal { from, to })` |
| 自环**单独一条**（不靠矩阵顺带） | `quarantined → quarantined` 与 `disabled → disabled` 各一条 `Ok`（逐项，两条各断言各的）；**反向**：`active → active` 与 `verified → verified` 各一条 `Err(Illegal { from, to })`——**两侧都钉**，只钉放行那侧会让「没列进表的自环」漂过去 |
| 画像早于 `verified` | **十态逐项**：`save_profile` 在六态 `Ok`、四态 `Err(ProfileBeforeVerified { state })` 并断言是哪一枚 |
| 「画像必须来自库」 | `load_profile` 对库里不存在的 id 返回 `Ok(None)`；`ModelProfile` crate 外不可构造（trybuild） |
| 空需求 | `TaskSkillRequirement::try_new(vec![])` 返回具体 `Err(RequirementEmpty)` |
| 编码纪律 | 直接查表：`lifecycle_state` / `dimension` 列是小写、多词 `_` 连接——**不是** `Debug` 表示 |
| 表外取值 | 裸 SQL 写入表外取值，读回返回具体 `Err`（枚举列、`Ratio` 越界、`score` 非数值各一条） |
| 浮点往返 | `parse(as_str(x)) == Some(x)` 逐值（含 1.0、0.5、`9.2`、极小/极大有限值） |
| `Ratio` 拒非有限 / 越界 | `1.5`、`-0.1` 各一条断言 `OutOfRange`；`NaN` 一条断言 `NotFinite`（**不归越界**，§2.4 的订正）；`SkillScore` 的同一组输入同法（跨类型一致） |
| §24 的「时间序列」 | 同维度存三个 `score_version`，`load_skill_series` 返回三条且升序；**同 `(id, dim, version)` 二次写入被主键拒且原行不变**（裸 `INSERT`） |
| 「缺席不是 0」 | 某维度无观测时 `SkillVector` 该维为 `None`；九维各断言一次（枚举式，逐项） |
| 路由排序确定 | 打乱输入顺序，输出逐项相同；同分候选按 `ModelId` 升序 |
| 候选集判重 | 同一 `ModelId` 两次 → `Err(DuplicateModelCandidate { id })`（断言是哪一枚） |
| 无候选 | 空输入与「全部被闸门挡下」各一条，各得 `Err(NoEligibleCandidate)` |
| 候选访问器 | 五个访问器各一条（`model()` 的照片是「它给出的 id 正是输入那个模型的 id」） |
| §84 的三样输出 | `selected()` / `alternatives()` / `reason` 各有用例；`alternatives` 是表尾（不是第二份数据） |
| 基线策略只用已定义的输入 | 改 `SkillScore` 的**数值**（保持有无不变）→ 排序**不变**；改「有无」→ 排序变 |
| 预算是必填输入 | 签名层面（无 `Option`）；**没有运行期用例**——见 §10 第 2 条 |
| 迁移编号与计数 | 跨 crate 才可见，按全量套件跑：`migrations.rs` 的集合比对 + `startup.rs` 的三处计数 |

**本项目既有的三条纪律一并适用**：变异须在全量 `--no-fail-fast` 下得出否定结论；凡注释写绝对措辞须有对应用例
（枚举式断言**逐项**有照片）；失败路径须断言是哪一种 `Err`。

---

# 10. 完成判据与**拍不到的照片**

《工程》§4.4 的前两条（`docs/02-工程.md:259-260`）：

```
给定任务能力需求，Router 输出带 confidence 的候选排序，且不依赖单一总分
未完成画像的模型不会进入自动路由
```

第一条由 §5.2 的输出类型与 §5.3 的基线策略共同覆盖（`confidence` 在输出上；「不依赖单一总分」由 §2.3 的三处结构性事实覆盖）。
第二条由 §4.2 的闸门与 §4.3 的画像产生点共同覆盖。

**本设计明确拍不到的照片，逐条列出**（按本项目纪律，写不出照片的把「为什么没有」写出来）：

1. **「评分是时间序列」在真实流水线里的样子**：§22 的 onboarding、§82 的 probe 一个都不存在，
   `model_skill_score` 的多版本只能由测试直接写入。**没有产生方**，故没有端到端照片。
2. **预算视图的真实语义**：语义层未建，`BudgetView` 的每个 `Some` 都只能由测试构造。
   「剩余额度算得对不对」在本阶段**不可观察**——故本设计不为它写运行期用例，
   只钉类型与签名（§6）。**这是「§250 的 MUST 考虑成本尚未实现」的同一件事的另一种说法。**
3. **探索概率与升级阶梯的实际触发**：§7.1 / §7.2 定它们不在本层。本层只有阶梯的数据形状，**没有消费者**，
   故 `next_step` 的照片只能是对纯函数的直接调用，钉不了「驱动真的在失败后升级了」。
4. **排序质量**：「A 比 B 更合理」在本设计里是一个**没有判据**的命题（§84 未给算法、OPEN-008 未决），
   故没有任何用例能断言「排出来的顺序是对的」。能拍的只有机制：全序、确定、字段齐备。
5. **§248 禁令的行为面**：第 1、2 条照片钉的是「类型上没有这个字段」与「表里没有这一列」，
   **钉不了**「将来不会有人绕过策略算一个总分出来」。后者的判据不存在（§5.3 已把打分推迟）。
6. **§249 的「自动」与非自动的边界**：本设计只钉**自动路由**这一条路径（§4.2）。§22 的 Active Probe
   会用 UNPROFILED 模型跑真实任务，而「probe 算不算 §249 所称的使用」正是 OPEN-014（C1）未决的问题——
   故另一条路径的边界**无照片**，因为它**无定义**。

---

# 11. 遗留与未决项

1. ~~**`stale` 被额外挡下自动路由，是协调者的解释，可被推翻**~~ —— **已闭（2026-10-05，用户拍板）**。
   裁决确认 §249 的三态是**下限不是上限**，`stale` 同样不得进入自动路由；依据是 §82 的
   `ACTIVE → STALE → 重新 profiling`（漂移中的模型正在退出服役，放行即 fail-open）。
   裁决记录：`docs/superpowers/specs/2026-10-05-p3bcdf-set-decisions.md` 第一节第 4 条。§4.2 已据此改为**已裁**。
   **残留一项、收件人是规范维护者**：§249 的正文是否要把 `stale` 正式写进 MUST NOT（本设计已按裁决落地，
   不因这一条而阻塞）。**`degraded` 保持可路由**，它是 §249 的异常态而非 `ProviderHealth::Degraded` 的同名变体。
2. **§27 的探索被 OPEN-014（C1）与 OPEN-008 双阻断**（§7.2），本设计不建。**收件人：规范维护者**。
3. **§25 的 Population Feedback 来源未定义（OPEN-008）**，冷启动期排序证据残缺（《工程》§4.5）。
   **收件人：规范维护者**。
4. **§84 未定义 `compatibility` / `confidence` 的含义与算法**（§5.2、§5.3）。本设计的 `BaselineRankingPolicy`
   是**具名替代**，不是对规范的声称；同时 §2.3 给出的「§84 的 `compatibility` 不是画像上的总分」这一读法，
   **需要复审明确表态**——它与 §248 的禁令能否并存，取决于这一读法。**收件人：复审者 → 规范维护者**。
5. **§333 的五个量纲没有单位，§87 的 `Cost` / `Latency` 没有取值域**（§2.5、§6），
   故 §250 的「考虑成本」与「考虑延迟」**无法计算**。本设计只把它们做成**不可省略的输入**。
   **`BudgetView` 住在资源层，投影由驱动做**（§6.1，已订正）：语义层产出它自己的 `Budget`，
   驱动投影成 `BudgetView` 传进来——故**任何时刻都不出现 `语义层 → 资源层`**。
   若复审判定 `BudgetView` 与语义层的 `Budget` 该合一，那是把只读投影塞进记账对象，本设计不同意。
   **收件人：语义层设计 + 规范维护者**。
6. **复用 `continuum_capability::{Cost, Latency}` 是一个决定**（§2.5）：若后续判定模型的
   `cost_profile` 与工具的 `cost` 不是同一轴，此处要拆，并把 `Cost` / `Latency` 上移到 `continuum-core`。
   **收件人：协调者**。
7. **三处画像清单互不一致**（§2.1）：§81 的身份五项比 §247 多 `deployment` 与 `capability fingerprint`；
   §22 的「Profile 记录」清单（`docs/spec/01-concepts.md:1047-1064`）既有 §247 没有的项（`context`、`known quirks`、
   `audio`、`instruction following`、`constraint adherence`），又不含 §248 的 `planning` 与 `verification`。
   本设计按 §247／§248 这对**规范层**的条目办，不擅自补。**收件人：规范维护者**。
8. **§248 的「每个维度 MAY 继续分层」未建**（§2.2）：子维度的词表与聚合规则规范都没有（§23 的树是概念层的例）。
   **收件人：规范维护者**。
9. **§83 的 `failure_modes` 没有封闭词表**（§2.1）：§250 要求 Router「考虑失败模式」、§83 说「Router 可针对任务
   避开这些缺陷」，而自由文本无法与任务匹配。本设计按自由文本存，**该因子因此未实现**。
   **收件人：规范维护者**。
10. **§19 的「明显收益足够高时才跨 family」没有阈值**（§5.1）。基线的处理是「同族优先」，
    跨族的判据缺失。**收件人：规范维护者**。
11. **§249 未定义迁移关系，本设计的状态迁移表是**本设计的决定（§4.1，与 §237 落在 P1 的同一情形）。
    **收件人：复审者**。
12. **`ExecutionProfile.cost_budget` 的类型收紧未做**（§6.3），与 ENG-005 第五节把它判给「D 落地时」相反。
    理由两条：**要收到的 `Budget` 类型随语义层才出现**（本层替它发明占位类型即是「预先发明」），
    以及**上限与余量是两个角色**（不能用 `BudgetView` 顶上去）。
    **依赖方向不是理由**——第一版稿子那条「会写出反向边」已作废（§6.3 保留了它的来历）。
    **收件人：协调者 + 语义层设计**（重新指派，或把这一步改为「语义层落地之后」）。
13. **C↔D 的交界：《工程》侧已闭，C 侧待 C 改**（§1.2、§8.4）。协调者裁定：§4.3 的
    「`Provider Adapter → 被 Router 调用`」属**层间依赖**一章，说的是组成与依赖方向，**不是调用次序**；
    §250/§84 要的是一份**判断**（带 confidence / alternatives / reason 的候选排序），不是一次执行。
    故 Router 不调适配器，`invoke` / `stream` 的调用方是**子项目 G**（**不是 F**，见 §1.2 末段与第 20 条）。
    **C↔D 共三条记录在案的接缝**：调用面、`ALLOWED` 条目、以及「模型调用路径今天不存在」，三条的收件人都是 **G**。
    - **已闭**：§4.3 的 Provider Adapter 一行已订正为「`Provider Adapter → 被模型调用路径（子项目 G）调用，接口中立`」
      （`docs/02-工程.md:249`）——**本设计的读法已写进《工程》**；同一批订正还写出了 §9.1 的箭头读法
      （被依赖者 → 依赖者）并补了 `资源层 → 边界层`，§6.3 那段订正因此有了规范出处。
    - **已闭**：`Router ← Model Registry + Capability Token` 那一行亦已订正为
      `Router ← Model Registry + continuum-capability 的两个类型（Cost / Latency）`
      （`docs/02-工程.md:248`，commit `a139fa9`，2026-10-05）——本设计依赖那个 crate 取的是 `Cost`/`Latency`，
      不是能力凭据（§8.4）。**本条曾两次翻面，三段来历见 §8.4。**
    - **未闭（收件人：C 的设计）**：C 的 §4「调用面」与它关于 `ALLOWED`「只应含 `continuum-provider`」
      的那句（C:274）须照样订正。**D 不改 C 的文档。**
    - **未闭（收件人：子项目 G）**：G 是三条接缝的共同收件人。
    **C 侧须指名真正的调用方并记明今天不存在。** 剩下的待办：《工程》§4.3 那一行与 §4.3 的
    `Router ← … Capability Token`（§8.4 第 1 条，含义是 `Cost`/`Latency` 而非 token）**文档层面的订正**。
    **另：C 的 §4「调用面」与它关于 `ALLOWED`「只应含 `continuum-provider`」的那句（C:274）都须照样订正**——
    同一处矛盾在 C 那边也有一份，两处不一致就还是没收敛。**这一条由 C 改，D 不改。**
    **再另：B 的设计也要求改《工程》§4.3 的同一张图**（B 抱怨图里没有 Connector）——
    **两处改动应由同一个人一次做完**，别各改一半。
    **收件人：《工程》文档维护者 ＋ C 的设计**。
14. **迁移编号 `80`（号段已裁定，核对未做；`81` 已删）**：协调者已裁定一号段一子项目（A 50、B 60、C 70、D 80、E 90），
    但**每一档取用前仍须核对该档未占用**——本设计只核了 `80` 在 `runtime_migrations()` 的集合里未占用。
    实现时最后核一次。**收件人：实现者（D 的第一个 task）**。
15. **P3A 遗留第 9 条在本设计里仍然悬着**：`cost` / `latency` 的取值域**仍然没有被给出**
    （D 是消费者，但不是给出尺度的那个，§2.5），`Cost` / `Latency` 的 `Some` 也仍然只表示「画像已登记」。
    **收件人：复审者**（P3A 要求「要么给出可达 `Some` 的路径并附照片，要么明写为什么没有」；
    本设计选了后者，且给的是**更明确的理由**：三个相关量纲都没有单位）。
    **本设计只答了模型侧**：P3A 那对落在 `ToolProfile` 上的 `tool.cost` / `tool.latency` 的 `Some`，
    **在 B/C/D/F 四份设计里至今无人认领**（F 明写本路径不读它们，D 复用的是类型而非 `ToolProfile`）。
    这一条不能靠 D 再答一次，须由协调者在四份之间指派。**同一个 `ToolProfile` 上的 `trust` 见第 23 条**
    （那一个 D 是明确退件，不是留待触发）。**收件人：协调者**。
16. **§22 的「初步画像」是否算 §247 的 `ModelProfile` 未定义**（§4.3）：本设计按 §22 的流水线顺序取「不算」，
    并把初步画像作为**刻意不落库的中间物**写明（它的效果折进 §247 的 `evidence_count` / `confidence`），
    从而使 §21 的「新增模型不能直接进入自动 Router」成立。若判定「算」，
    则 `discovered` / `researched` / `probed` 三态将获得可路由性。**收件人：规范维护者**。
17. **「当前观测」的判据取 `version` 最大而非 `time_range` 最晚**（§2.2）：§24 未定义二者冲突时的优先。
    **收件人：复审者**。
18. **C 交给 D 的 `ProviderError → FailureClass` 映射：D 有条件签收，映射本身退件。**（`ProviderError`、
    `FailureClass::Resource`、`max_attempts` 在全篇此前**零命中**，这条交接确实挂空过。）分工如下：
    - **映射不可能落在 D**：`ProviderError` 是 `continuum-provider` 的类型，而本设计对那个 crate 的引用为零
      （§1.2、§8.2，交界已由协调者裁定）。把映射搬进来，就得登记一条被裁掉的边。
    - **D 签收的是已分类的结果**：`FailureHistory`（§250 的第六个输入，今天形状未定，落在策略一侧，§5.1）。
      分类发生在**持有 `ProviderError` 的一侧**（**子项目 G**，它做模型侧失败分类）。
    - **`max_attempts >= 2` 与退避参数不是本层的**：`docs/superpowers/p1-followups.md:62-64` 的原话是
      「**P3 的 Router** 必须为 RESOURCE 显式给出 `max_attempts >= 2` 与退避参数」——
      **本设计不接这句话里的「Router」二字**：按 §246，重试参数属 `ExecutionProfile.retry_policy`，
      归**执行层**（它的产生方是节点配置，不是路由）；本层的 `EscalationLadder`（§7.1）是「换哪个模型」，
      与「重试几次」是两个轴。**这是一处与本仓既有 followups 的措辞分歧，明写在此**，
      由协调者指派，别让它第二次挂空。
    - **今天 `FailureClass::Resource` 从 `ProviderError` 不可达**（该枚举无限流／配额变体），
      故 §251 的前提检查里 RESOURCE 那一格**无产生方**——这是「未实现的因子」，不是「本设计漏了」。
    **收件人：C 的设计（改指名真正的分类方）＋ 协调者**。
19. **§250 的八个 MUST 考虑项里，`上下文长度` 在本层没有任何输入**（§5.1 的表）：§247 的十二个字段没有它，
    §250 的六个输入也没有。§22 的 Profile 清单有 `context`、§315 的 `ModelDescriptor` 有 `context_window`，
    但两者都不是本层能凭空取用的。本设计不擅自加字段，**该因子因此未实现**。
    **收件人：规范维护者**（是否把上下文长度补进 §247 的 `ModelProfile`，或补进 §250 的输入清单）。
20. **模型调用路径归子项目 G——收件人已具名**（§1.2）。本设计的两件事
    （取 `ProviderHealth` 快照、消费 `RankedExecutionCandidates` 去发起 `invoke` / `stream`）
    落在**子项目 G（模型调用路径）**上，**不是 F**（F 是工具调用路径，本轮已裁）。
    依据：`docs/superpowers/specs/2026-10-05-p3bcdf-set-decisions.md` 第一节第 1 条——G 是那个
    承担了约 28 处义务却从未具名的「执行侧」，本轮把它裁了出来。**「模型调用路径今天仍无实现方」这条
    须与 C 的同一句一致**——G 只是具了名，本轮**不设计**，故代码路径仍不存在——它构成 C↔D 的**第三条接缝**。
    **收件人：子项目 G**（G 本轮不设计，等 B/C/D/F 落计划后再起；**它的输入接口由 C 与 D 定**——
    本条即 D 侧的那一半）。
21. **§86 的 `Tier 1 Low` 不在 §251 的阶梯五档里**（§7.1）：两者是规范自身的不一致
    （`docs/spec/02-positioning.md:847-867` vs `docs/spec/05-normative.md:922-944`）。
    本设计按 §17／§18 的「档位与推理强度分离」读成「Tier1 ＋ 低推理强度」，**不给
    `EscalationStep` 加第六个成员**。若判定该加，本层的「照录 §251」这句话就要改。
    **收件人：规范维护者**。
22. **C 交给 D 的 `effect_class` 两轴之问：D 明确退件。** C 的 §12 第 11 条把「`Tool.effect_class` 与
    Journal 的 `EffectType` 是否两个轴」判给 D（它源自 P3A 设计 §10 第 3 条）。本设计**不接**，理由：
    - **D 对 `effect_class` 的引用为零。** 本 crate 从不出现该字段——§8.2 的边表里没有 `continuum-effect`
      就是这条的证据；本设计复用 `continuum-capability` 取的是 `Cost` / `Latency` **两个类型**，
      不读 `ToolProfile`（§2.5 末段同一条判据）。
    - **问句涉及的两侧都不在模型侧。** `Tool.effect_class` 属 Tool Registry（P3A），
      `EffectType` 属 Effect Journal（边界层）；判「是不是两个轴」要看的是**策略按哪个裁决**与
      **工具调用时按哪个判定**——两者都在工具调用路径上，与「选哪个模型」无关。
    - **D 侧没有触发条件可挂**：本设计不预先发明一个「将来 D 会读 effect_class」的钩子，
      那正是「不预先发明」禁的形状。故这里不是「留待触发」，是**退件**。
    **收件人：子项目 F（工具调用路径）＋ 规范维护者**（若最终判定规范里二者本就同轴，则由后者销掉此问）。
23. **`trust`：D 退件，理由与收件人见 §2.5 末段。** P3A §3.1 把 `cost` / `latency` / `trust` 一并判给 D；
    本设计只接前两个（§247 的 `ModelProfile` 有 `cost_profile` / `latency_profile` 两个字段，
    **没有 trust**），`trust` 退给**子项目 F（工具调用路径）＋ 规范维护者**。
    协调者已订正 P3A 的设计、记明工具侧的 `cost` / `latency` **与 `trust`** 无人认领——
    本条是 D 对这一格的正式答复，不是沉默。
24. **`ProviderHealth::Degraded` 的降权判据规范未给**（§5.3）。本设计写死的是**只过滤 `Unavailable`**
    （不可用即不是执行候选），`Healthy` 与 `Degraded` 都进候选集、**基线不为 `Degraded` 改变排序**；
    `Degraded` 该不该降权、降多少，**§250 只说「考虑」、§84 没有给这一维的算法**，故不发明。
    它原样带进 `reason` 供策略自用。**收件人：规范维护者**。
