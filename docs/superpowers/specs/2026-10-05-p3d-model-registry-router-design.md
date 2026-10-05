# P3 资源层（子项目 D）设计：模型画像、Model Registry 与 Router

**范围**：《工程》§4.1 的十一个组件中，本子项目建**模型侧的五项**——ModelProfile 与 SkillVector 存储（§247 §248）、
Model Registry 生命周期（§249）、Router 与候选排序（§250 §84）、升级与降级（§251 §85 §86）、路由探索（§27）
（`docs/02-工程.md:215-221`）。

本子项目是 P3 分解（A–E：Capability / 连接器 / 工具与 Provider 中立边界 / 模型侧与 Router / 计算节点与放置）中的 **D**，
次序 A → B → C → D → E（P3A 设计第 1 行）。A 是 D 的前置（`Router ← Model Registry + Capability Token`，§4.3）。

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
（`crates/continuum-core/src/model.rs:83`），由调用方（驱动，子项目 F）取好后作为**值**传入。
理由有二，都不是偏好：

1. 排序因此是**同步纯函数**——没有 I/O、没有 `.await`，可以在表驱动用例里穷举；
2. §4.3 的那条箭头说的是「适配器的信息到达 Router」，**以值为载体同样到达**，且不产生一条 C 与 D 之间的新边。

**若 C 的设计判定 Router 必须经它的注册表调用适配器**，那是 C 与 D 的实现次序问题，不改本设计：本设计只定「Router 收到
一个可用性快照」。此交界记在 §11 第 13 条。

---

# 2. `ModelProfile` 与 `SkillVector`（§247 §248）

## 2.1 §247 的十一个字段逐项处置

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
| `failure_modes[]` | `Vec<String>`，落库为 JSON 数组 | §83 给的是自由文本例（`loses constraints in very long tasks` 等，`docs/spec/02-positioning.md:780-786`），**无封闭词表**。见 §11 第 9 条 |
| `latency_profile` | `Option<Latency>`（复用 P3A 的类型） | 见 §2.5 |
| `cost_profile` | `Option<Cost>`（复用 P3A 的类型） | 见 §2.5 |
| `evidence_count` | `u64` | 计数，没有单位问题 |
| `confidence` | `Ratio` | §84 的示例把 confidence 写成 `0.94` / `0.51`（`docs/spec/02-positioning.md:796-800`），故取 **[0, 1] 的实数**。这是**照 §84 的示例推导**，不是发明 |

**§81 与 §247 不一致，本设计按 §247。** §81（`docs/spec/02-positioning.md:708-734`）说模型身份至少含
`provider / model_id / revision / deployment / capability fingerprint` 五项，而 §247 的 `ModelProfile` 只有
`provider` 与 `model_revision`，**没有 deployment 与 capability fingerprint**。本设计**不擅自补两个 §247 没有的字段**
（「不预先发明」），把这个不一致记在 §11 第 7 条。

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
/// §23 的能力评分（示例 `coding = 9.2`，`docs/spec/01-concepts.md:1068-1104`）。
///
/// **取值域与单位规范未定义**：§23 的示例是 9.2，§24 未给范围与方向。故本类型
/// 只保证一件事——**可比较**（全序）。**本设计只使用它的序，从不使用它的量**：
/// 任何跨维度的求和、加权、归一化都需要一个规范未给的尺度，故一处都不做。
/// 这一条正是 §248 禁令在算法上的对应物：量纲未知时，唯一合法的用法是比较。
pub struct SkillScore(f64);        // 构造时拒 NaN / ±∞

/// §84/§247/§24 的 [0,1] 实数（confidence、compatibility）。
pub struct Ratio(f64);             // 构造时拒 NaN / ±∞，且须落在 0.0..=1.0
```

- **为什么 `Ratio` 要拒 NaN**：NaN 与任何值的比较都是 false，`sort_by` 在含 NaN 的列表上不是全序
  ——排序结果随实现细节漂移。这不是洁癖：§84 的输出要被比对与记录，不确定的排序无法有照片。
  用例钉三个越界输入（`NaN`、`1.5`、`-0.1`）各返回具体 `Err`。
- **这对辅助函数的返回类型与 P3A 的不同，且是有理由的偏离**：`CapabilityKind::as_str` 返回
  `&'static str`，因为它是封闭枚举、字面量固定；`SkillScore` / `Ratio` 的域不是封闭枚举，`as_str` 返回
  `String`。**契约不变**：`parse(as_str(x)) == Some(x)` 逐值成立（用例钉往返），
  且 `parse` 对表外取值（非数值、越界、NaN）一律 `None`，由 `persist.rs` 转成具体 `Err`，**不取默认值**。
  浮点的往返用 Rust `{}` 的最短表示（`format!("{}", x)`），它对 `f64` 是**精确往返**的；这一点有用例钉住，
  因为「浮点存文本会丢精度」是这里最容易想当然的地方。
- **`SkillDimension` / `LifecycleState` 落库一律小写、多词以 `_` 连接**（本项目既有约定，
  P3A 计划第 22 行）。九维因此写成 `reasoning` / `coding` / `vision` / `planning` / `tool_use` /
  `constraint_following` / `verification` / `spatial` / `media`。

## 2.5 `cost_profile` / `latency_profile`：复用 §87 的 `Cost` / `Latency`

P3A 把 §87 的 `cost` / `latency` / `trust` 留成了**单位结构体**（只定容器形状、取值域留空，
`crates/continuum-capability/src/tool.rs:176-251`），并在 P3A 设计 §10 第 9 条写明「它们服务的是子项目 D 的候选排序」。

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
pub fn transition(tx: &Tx<'_>, id: &ModelId, to: LifecycleState)
                  -> Result<LifecycleState, RegistryError>;                    // §4.3
```

`load_skill_series` 的消费方是 §82 的行为指纹（「如果表现突然变化」，`docs/spec/02-positioning.md:736-764`）
——判「变化」至少要看两次观测，故它现在就必须有；只存当前值会让 §82 的判据无法成立。
**它的产生方（周期性 probe）本阶段不存在**，这一点写在 §10 的「拍不到的照片」里，不靠一句将来时糊过去。

## 3.3 迁移编号

已占用（`crates/continuum-runtime/src/main.rs:57-65` 的装配集合）：`1`、`2`（P0 内建）、`10`（artifact）、
`20`（graph）、`30`（workspace）、`40`（effect）、`41`（policy）、`50`（P3A capability）——共 8 条。
「前几位是十位一档」是本仓的既有取法（P3A 计划第 441 行）。

**号段已由协调者裁定：一个子项目一个十位档——A 50、B 60、C 70、D 80、E 90**，每一档**取用前仍须核对该档未被占用**。
**本设计取 `80`**（三张表在一条迁移里，`Db::migrate` 用 `execute_batch`，一条迁移可含多条语句，
`crates/continuum-persist/src/db.rs:117`），**预留 `81`** 给后续 task 新增的表。

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
第 9 节确定」）。故本设计定一张迁移表，落成 `transition(from, to) -> Result<LifecycleState, RegistryError>`，
与 `continuum-graph` 的同名函数同形（含 `Illegal { from, to }` 这一具体 `Err`）：

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
  由任意状态到达同一判据（`crates/continuum-graph/src/state.rs:18-21`）：事故与人工下线不挑时机。
- **`disabled → unprofiled` 有一条件**：重新启用**必须重走画像流水线**（故出口是 `unprofiled` 而不是 `active`），
  依据是 §21「新增模型不能直接进入自动 Router」的同一条道理。被否掉的替代方案是「`disabled` 为终态」——
  它会让一次事故性的下线只能靠删行恢复，而删行会丢掉画像与观测历史。

**自环（`x → x`）非法**，未列出的对一律 `Err(Illegal { from, to })`。

## 4.2 可路由闸门：四态**不可进入路由路径**（§249 ＋ 一条本轮的裁定）

§249 的硬约束：「Router MUST NOT 自动使用 `UNPROFILED` / `QUARANTINED` / `DISABLED` 模型」。
《工程》§4.2 重复了它（`docs/02-工程.md:237`）。

**这三点是下限，不是上限。** 本轮协调者裁定：**`stale` 一并挡在自动路由之外**，本设计照办。
理由（裁定原文的意思）：§82 的流程是 `ACTIVE → STALE → 重新 profiling`——一个已漂移的模型**正在退出服役**，
把它放行给自动路由是一个 **fail-open 的形状**，而本项目对 fail-open 一贯的做法是消除而不是容忍。

**标记：这一条是协调者的解释，可被用户或规范推翻。** §249 的 MUST NOT 字面上只列三态，
`stale` 是**本设计额外挡下的一态**。此标记不省。§249 的正文没有改、也未被本设计改。

**做成结构性保证，而不是「Router 记得跳过」**——与 P3A 的 `AuthorizedTool` 同一形（P3A 设计 §3.4）：

```rust
/// §249 的十个状态（照录）。
pub enum LifecycleState {
    Discovered, Unprofiled, Researched, Probed, Verified, Active,
    Stale, Degraded, Quarantined, Disabled,
}

/// 可被**自动路由**的那七个状态。
///
/// `unprofiled` / `quarantined` / `disabled` **不在此枚举里**——§249 禁的那三态
/// 在路由路径上无处安放。这与 P3A §2.3「`full_access` 不是禁止作为默认，而是没有
/// 这个成员」是同一种做法。
pub enum RoutableState { Discovered, Researched, Probed, Verified, Active, Stale, Degraded }

impl TryFrom<LifecycleState> for RoutableState {
    type Error = RoutingError;   // NotRoutable { state: LifecycleState }
}

/// 一个**可交给 Router** 的模型：画像 + 已过闸门的状态。
/// 字段私有、无公开构造函数；唯一的产生点是 `RoutableModel::try_new`。
pub struct RoutableModel { profile: ModelProfile, state: RoutableState }
```

于是 `pub fn rank(request, models: &[RoutableModel], policy) -> ...` 的签名里**不存在**
未画像、被隔离、被停用的模型——**「Router 忘了检查状态」这条路径在类型上不存在**。

**这里的措辞要与保证等强，故写准**：三态在 `LifecycleState` 上**是表达得出来的**（画像流水线、隔离与停用
都要用它），被挡住的是**路由路径**。本设计**不**声称「三态不可表达」。

**逐项有照片**：十个状态各一条用例喂进 `RoutableModel::try_new`，七个返回 `Ok`，三个返回
`Err(NotRoutable { state })` 且**断言是哪一枚**（不是「返回了 Err」）。这是枚举式绝对断言，按本项目纪律**逐项**钉。

**`stale` 与 `degraded` 是可路由的——这是一个判断，写在此处供推翻。** §249 的 MUST NOT 是一张**封闭清单**，
三态之外都不禁；§82 的意图（别用失效画像）与 §249 的字面在这里有张力，本设计按**规范层的封闭清单**办，
并把状态原样带进候选的 `reason`（§5.2），让策略可以据此降权——**可见但不禁**。若复审判定 `stale` 也该禁，
那是规范要改，不是本设计要私自加一条禁令。记在 §11 第 1 条。

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

**画像在 `probed → verified` 时才产生**（一个产生点）：`save_profile` 读登记项，若当前状态不在
`{verified, active, stale, degraded, quarantined, disabled}` 之内，返回
`Err(RegistryError::ProfileBeforeVerified { state })`。依据是 §22 的流水线顺序——「生成初步画像 → 执行 Active Probe
→ Verifier → **生成正式 Profile**」，正式画像在 Verifier 之后。

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
| `TaskSkillRequirement` | **本设计定义类型**：`Vec<SkillDimension>` | 规范只给名字、未给形状。本设计的形状是「这个任务需要哪几个 §248 维度」，依据是 §250 的「能力匹配」以 §248 的向量为基准。**不带权重、不带阈值**——两者都是规范没有的数，加了就是发明。见 §11 第 4 条 |
| `ModelProfile` | **已有类型**（§2） | 经 `RoutableModel` 传入，见 §4.2 |
| `CostPolicy` | **不定义形状**，落在排序策略接口之后（§5.3） | 规范只给名字。见 §11 第 5 条 |
| `LatencyPolicy` | 同上 | 同上 |
| `FamilyPreference` | **本设计定义类型**：§19 的五项照录（`Auto` / `OpenAiPreferred` / `ClaudePreferred` / `LocalPreferred` / `Custom`，`docs/spec/01-concepts.md:933-960`），落库小写 `_` 连接（本阶段它不落库，只作输入） | §19 给的是**封闭清单**，可照录。`Custom` 在 §19 里不带载荷，本设计也不给它加。§19 的「明显收益足够高时才跨 family」**没有阈值**，见 §11 第 10 条 |
| `FailureHistory` | **不定义形状**，落在排序策略接口之后 | 规范只给名字，且**它的产生方也没有指定** |

另有两项输入不在 §250 的清单里、但 §250 的「最终选择 MUST 考虑」里点名了：
**当前可用性**（取 §315 既有的 `ProviderHealth`，一个**值**；见 §1.2）与**预算视图**（ENG-005，见 §6）。

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
    state: RoutableState,     // §249 的状态对策略可见（stale / degraded 据此降权，见 §4.2）
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
    fn compare(&self, a: &Scored, b: &Scored) -> Ordering { /* 缺省 */ }
}
```

`rank` 的骨架：过闸门的候选逐个 `evaluate` → `sort_by(policy.compare)` → 取头。**排序是全序且确定的**：
`compare` 的兜底一档按 `ModelId` 升序，使同分候选的次序不随输入顺序漂移（用例钉：打乱输入顺序，输出不变）。

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

**§250 的「MUST 考虑成本」在本设计里只是结构性地不可省略，而不是已实现**：`RoutingRequest` 里
`budget: BudgetView` 是**必填参数**（不是 `Option`），故一个策略想忽略预算，是一次**看得见的选择**，
而不是一次遗漏。它的量值无法计算的理由见 §6。**这一条是本设计最需要在复审里被挑战的地方**，
故写在此处而不藏在脚注里。

## 5.4 失败路径：说清是哪一种 `Err`

```rust
pub enum RoutingError {
    /// §249 的三态。`state` 原样带出，调用方可分辨是被隔离还是没画像。
    NotRoutable { state: LifecycleState },
    /// 一个候选都没有（含「全部被闸门挡下」与「一个模型都没登记」两种情形）。
    NoEligibleCandidate,
    /// 画像读失败。
    Persist(#[from] PersistError),
}
```

每条各有用例断言**具体是哪一枚**。`NoEligibleCandidate` **不合并**进 `NotRoutable`：前者是「没有可用的」，
后者是「有一枚被点名挡下了」，调用方（§110 的流程）对两者的处置不同。

---

# 6. 成本输入 = 预算视图（ENG-005）

## 6.1 接口：本层定义**投影类型**，语义层生产

ENG-005 裁决：D 的成本输入是**预算视图**（剩余额度，不是「一个数」），它来自**语义层**，而语义层尚未建
（`docs/superpowers/specs/2026-10-05-eng-005-budget-accounting.md` 第五节；共享面第三节）。
§9.1 的层间方向是 **`语义层 (2) → 资源层 (4)`**（`docs/02-工程.md:560-579`，「依赖方向单向，无环」）。

**方向决定了接口放在哪一边**：若本层去调用语义层的一个 trait，就写出一条反向边。故正确形状是
**本层定义投影类型，语义层来生产它**：

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
故本设计**不定义任何 trait**——一个没有第二个实现者的 trait 是假接口。

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
**本设计不做这一步**，理由如下，请复审裁决：

- `ExecutionProfile.cost_budget`（`crates/continuum-graph/src/execution.rs:31`）是 §246（`docs/spec/05-normative.md:796`）
  的**节点级声明式约束**；本设计的 `BudgetView` 是**剩余额度的投影**。「一个上限」与「一个余量」是两个角色。
  把余量放进约束的位置，会让同一个类型有两个含义——那是本项目一贯判为缺陷的那一类。
- 按 §246 与 ENG-005，节点预算的类型应当是**语义层的 `Budget`（分配）**。但 `continuum-graph` 属**执行层 (3)**，
  它引用语义层的类型会写出 `执行层 → 语义层` 这条**反向边**（§9.1 的方向是 `语义层 → 执行层`）。
- 故本设计**不发明一个中间类型**去占位——那正是「预先发明」。记在 §11 第 12 条，收件人是协调者与语义层设计。

---

# 7. 升降级与路由探索（§251 §85 §86 §27）

## 7.1 升级与降级：阶梯的形状在本层，触发不在

《工程》§4.1 把「升级与降级」列在资源层。本设计把它的**数据形状**建在本层，把**触发**留给别处：

```rust
/// §251 的阶梯（照录）：Tier1 → Tier2 → Tier2High → CrossFamily → Specialized。
pub struct EscalationLadder { steps: Vec<EscalationStep> }
```

**本层建**：阶梯的**有序步骤**，以及「下一档」这个纯函数 `next_step(cur) -> Option<EscalationStep>`。
**本层不建**：

- **失败检测与升级触发**——它要读一次执行的失败（§309 的 `FailureClass`），那是驱动/执行层的活（子项目 F）；
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
  src/budget.rs       BudgetView（§333 的只读投影）
  src/error.rs        RegistryError、RoutingError
  tests/{profile,lifecycle,persist,router,budget}.rs
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

## 8.4 与《工程》§4.3 的两处不一致（明写在此，不静默偏离）

`docs/02-工程.md:245-254` 的层内依赖图里有两条边与本设计的实际依赖不符：

| §4.3 的原文 | 本设计的处置 | 理由 |
|---|---|---|
| `Router ← Model Registry + Capability Token` | **只保留 `← Model Registry`**；到 `continuum-capability` 的那条边**存在但含义不同**（取的是 `Cost`/`Latency` 两个画像容器，**不是** Capability Token） | 本设计在全仓找不到 `Router` 对「能力凭据」的使用点：路由选的是**模型**，能力约束的强制点在工具调用路径（§4.2，归子项目 A 与 F）。按「零使用的边即假边」的口径，边存在而含义要订正；若要保留「Router 考虑 Capability」的原意，须先指出它用在哪一步 |
| `Provider Adapter → 被 Router 调用，接口中立` | **改成「适配器的可用性以值到达 Router」**（§1.2） | 排序因此是同步纯函数，且不新增 C↔D 的边 |

两条都记在 §11 第 13 条，收件人是《工程》文档维护者。

---

# 9. 测试策略

| 验什么 | 怎么验 |
|---|---|
| §249 三态不入路由路径 | **十态逐项**：七态 `Ok`、三态 `Err(NotRoutable { state })` 并断言是哪一枚 |
| `RoutableModel` 不可外部构造 | trybuild 样例（与 P3A 的 `AuthorizedTool` 同形） |
| `overall_score` 读不到 | trybuild 样例；另：`SELECT overall_score FROM model_profile` 得到具体 `Err` |
| `ModelProfile` 无总分 | 断言 `model_profile` 的列清单**逐列**（加一列即红） |
| 生命周期迁移表 | 合法对逐条 `Ok`；**未列出的对**给若干条 `Err(Illegal { from, to })`；自环非法 |
| 画像早于 `verified` | `save_profile` 在 `researched` 上返回具体 `Err(ProfileBeforeVerified { state })` |
| 编码纪律 | 直接查表：`lifecycle_state` / `dimension` 列是小写、多词 `_` 连接——**不是** `Debug` 表示 |
| 表外取值 | 裸 SQL 写入表外取值，读回返回具体 `Err`（枚举列、`Ratio` 越界、`score` 非数值各一条） |
| 浮点往返 | `parse(as_str(x)) == Some(x)` 逐值（含 1.0、0.5、`9.2`、极小/极大有限值） |
| `Ratio` 拒 NaN / 越界 | `NaN`、`1.5`、`-0.1` 各一条，各断言具体 `Err` |
| §24 的「时间序列」 | 同维度存三个 `score_version`，`load_skill_series` 返回三条且升序；**同 `(id, dim, version)` 二次写入被主键拒且原行不变**（裸 `INSERT`） |
| 「缺席不是 0」 | 某维度无观测时 `SkillVector` 该维为 `None`；九维各断言一次（枚举式，逐项） |
| 路由排序确定 | 打乱输入顺序，输出逐项相同；同分候选按 `ModelId` 升序 |
| 无候选 | 空输入与「全部被闸门挡下」各一条，各得 `Err(NoEligibleCandidate)` |
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

1. **§249 的封闭清单与 §82 的意图在 `stale` / `degraded` 上有张力**（§4.2）：本设计按封闭清单放行、
   把状态带进 `reason` 供策略降权。**收件人：规范维护者**（是否要把 `stale` / `degraded` 也列入 MUST NOT）。
2. **§27 的探索被 OPEN-014（C1）与 OPEN-008 双阻断**（§7.2），本设计不建。**收件人：规范维护者**。
3. **§25 的 Population Feedback 来源未定义（OPEN-008）**，冷启动期排序证据残缺（《工程》§4.5）。
   **收件人：规范维护者**。
4. **§84 未定义 `compatibility` / `confidence` 的含义与算法**（§5.2、§5.3）。本设计的 `BaselineRankingPolicy`
   是**具名替代**，不是对规范的声称；同时 §2.3 给出的「§84 的 `compatibility` 不是画像上的总分」这一读法，
   **需要复审明确表态**——它与 §248 的禁令能否并存，取决于这一读法。**收件人：复审者 → 规范维护者**。
5. **§333 的五个量纲没有单位，§87 的 `Cost` / `Latency` 没有取值域**（§2.5、§6），
   故 §250 的「考虑成本」与「考虑延迟」**无法计算**。本设计只把它们做成**不可省略的输入**。
   **收件人：语义层设计 + 规范维护者**。
6. **复用 `continuum_capability::{Cost, Latency}` 是一个决定**（§2.5）：若后续判定模型的
   `cost_profile` 与工具的 `cost` 不是同一轴，此处要拆，并把 `Cost` / `Latency` 上移到 `continuum-core`。
   **收件人：协调者**。
7. **§81 与 §247 的模型身份字段不一致**（§2.1）：§81 多 `deployment` 与 `capability fingerprint`，
   §247 没有。本设计不擅自补。**收件人：规范维护者**。
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
    理由：那是声明式约束、`BudgetView` 是余量投影、且执行层引用语义层类型会写出反向边。
    **收件人：协调者 + 语义层设计**（需重新指派或另定一个不产生反向边的类型位置）。
13. **《工程》§4.3 的两条依赖边与实际依赖不符**（§8.4）。**收件人：《工程》文档维护者**。
14. **迁移编号 `80` / 预留 `81` 需与 B、C 核对**（§3.3）。判据是未占用，不是顺位。**收件人：协调者**。
15. **P3A 遗留第 9 条在本设计里仍然悬着**：`cost` / `latency` 的取值域**仍然没有被给出**
    （D 是消费者，但不是给出尺度的那个，§2.5），`Cost` / `Latency` 的 `Some` 也仍然只表示「画像已登记」。
    **收件人：复审者**（P3A 要求「要么给出可达 `Some` 的路径并附照片，要么明写为什么没有」；
    本设计选了后者，且给的是**更明确的理由**：三个相关量纲都没有单位）。
16. **§22 的「初步画像」是否算 §247 的 `ModelProfile` 未定义**（§4.3）：本设计按 §22 的流水线顺序取「不算」
    （画像只在 `probed → verified` 时产生），从而使 §21 的「新增模型不能直接进入自动 Router」成立。
    若判定「算」，则 `discovered` / `researched` / `probed` 三态将获得可路由性。**收件人：规范维护者**。
17. **「当前观测」的判据取 `version` 最大而非 `time_range` 最晚**（§2.2）：§24 未定义二者冲突时的优先。
    **收件人：复审者**。
