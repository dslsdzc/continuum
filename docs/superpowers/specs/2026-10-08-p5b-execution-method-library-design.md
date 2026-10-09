# P5b 执行方法库（Execution Method Library）— 设计

> 本设计只覆盖 P5b 一块。切分与归属见
> `docs/superpowers/specs/2026-10-08-p5-scope-and-split.md`（下称「切分文档」）。
> 本块拥有的是**方法登记与选取的接口**（切分文档 §五 P5b 行、§四第 3 条：**本块先冻，
> 在过审前四个算子块不得自造方法登记形态**）。故本文把接口面写到可被照着实现的粒度。
>
> 本文**不写代码、不写计划**；下述 Rust 片段是**待审的接口契约**，其实现在对应实现计划里，
> 由该处的编译与用例兑现。

---

## 一、范围与依据

切分文档 §五对 P5b 的一句话范围（逐字）：

> 把「这一类任务怎样做得可靠」从 ADFIR 的图结构里分出来（§187：ADFIR 决定**做什么**、
> Execution Method 决定**怎样做得可靠**）——定方法的登记形态与按领域选取的入口，
> 并为四个领域各建一份方法目录。

规范依据（逐条实测）：

| 依据 | 位置 | 内容要点 |
|---|---|---|
| §187 | `docs/spec/04-method.md:102-156` | 定义 Execution Method Library；四目录与例名；ADFIR 与 Method 的分工 |
| §186 | `docs/spec/04-method.md:53-99` | Superpowers 式工程执行；「计划先于实现／任务上下文隔离／逐任务验证／规格审查与质量审查分离」 |
| 《工程》§8.1 第 10 行 | `docs/02-工程.md:494` | `| Execution Method Library | §187 |` |
| 《工程》§8.3 | `docs/02-工程.md:526` | `Execution Method Library ← 无`（本块依赖面的**唯一**权威行） |
| 《总纲》§8.1 | `docs/01-总纲.md:1327-1343` | 执行方法库；四目录清单；ADFIR 与 Method 的分工 |

§187 的分工原文（`docs/spec/04-method.md:144-155`，实测）：

```
ADFIR 决定：           做什么 / 依赖什么
Execution Method 决定： 这一类任务怎样做得可靠
```

**本块据此只登记「怎样做得可靠」，不登记「做什么」。** 「做什么／依赖什么」是 ADFIR 的
图结构与节点（P1 的设计 `docs/superpowers/specs/2026-10-01-p1-execution-layer-design.md:185-245`）。

### 本块的接口面是「先冻」的那一份

切分文档 §四第 3 条（`docs/superpowers/specs/2026-10-08-p5-scope-and-split.md:130-133`）逐字：

> **P5b 的方法登记接口先冻。** §187 的四个目录（`software/` `video/` `research/` `3d/`）
> 与四个领域算子块是**多对多**：`software/` 对 P5c、`research/` 对 P5d、`video/` 对 P5e，
> 而 `3d/` **在四个具名算子里没有对家**（见 §八）。故 P5c/P5d/P5e/P5f 的「方法」条目的
> 登记形态归 P5b，四块只填内容；**在 P5b 的设计过审前，四个算子块不得自造方法登记形态**。

故本文的**交付物就是第 4 节的接口类型**，第 4 节之外的章节是它的依据与边界。

---

## 二、不做什么（范围排除）

以下各项**不在 P5b**，本块不为其定形；若发现其中某项无人认领，按切分文档 §四第 4 条报给协调者，
不在 P5 内补：

1. **不定义算子（Operator）。** `Operator` 的登记形态已由 P1 交付的 `continuum-operator` 定
   （§244/§245，`crates/continuum-operator/src/definition.rs:79`、`registry.rs:15`）。
   本块**不加字段、不改签名、不注册方法进 `OperatorRegistry`**。
2. **不自建执行路径**（切分文档 §四第 4 条）。方法**不可执行**，`Queued → Running` 的迁移、
   `OperatorRegistry::resolve` 的调用点都在第 3 层，不属本块。
3. **不碰 `execution_policy`**（切分文档 §四第 5 条）。本块也不碰 `verification_policy`
   （那是 P5a；`crates/continuum-graph/src/node.rs:20` 的注记「结构保存，判定属 P5」指 P5a）。
4. **不定义 Evidence、VerificationProfile、Requirement Coverage、Completion Predicate**（P5a），
   也不定义领域的 Operator 集（P5c/P5d/P5e/P5f）。
5. **不扩展 `ArtifactType`**（唯一落地点是 P5e，切分文档 §四第 2 条）。
6. **不定义「方法的领域如何判定」** —— 规范无来源，见第 8 节 R4。

---

## 三、本块要答的核心问题：Method 与 Operator 是什么关系

### 3.1 结论

**Method 与 Operator 是两个不同的登记面，互为不相交的两个注册表。**

| | Operator（§244） | Method（§187） |
|---|---|---|
| 定义处 | `crates/continuum-operator/src/definition.rs:79` | 本设计第 4 节 |
| 注册表 | `OperatorRegistry`（`registry.rs:15`） | `MethodRegistry`（本设计） |
| 携带 | `input_schema`/`output_schema`/`determinism`/`side_effect_class`/`backend_candidates` | `domain`/`purpose`，加一条**文本**的 `realized_by` |
| 语义 | **做什么**的可执行单元 | **怎样做得可靠**的规矩 |
| 可否执行 | 是（经第 3 层执行器） | **否**；它不是执行单元 |

一条 Method **不进入** `OperatorRegistry`，一条 Operator **不进入** `MethodRegistry`。
两者的交集只在本设计第 3.2 节的那一条**弱引用**上。

### 3.2 方法到算子的引用是**文本**的，理由是一个被否决的更显然写法

更显然的写法是 `realized_by: Vec<OperatorRef>`（`OperatorRef` 在
`crates/continuum-graph/src/node.rs:42`，由 `OperatorId`+`OperatorVersion` 定位），
那会让 `continuum-method` **依赖 `continuum-operator`**。

**这里不是那样：因为 §8.3 把本组件的依赖面写死为「无」。**《工程》§8.3
（`docs/02-工程.md:526`，实测原文）是：

```
Execution Method Library ← 无
```

同表紧邻的一行是 `领域算子 ← 第 3 层 Operator 注册表 + 第 4 层 Router`（`:527`）——
**领域算子**对第 3 层 Operator 注册表的依赖是**显式登记**的；EML 这一行的取值是「无」，
不是同一形状。故 EML **不得**命名 `OperatorId`。

落点：`MethodEntry.realized_by: Vec<String>` **以文本登记算子 id**，本 crate **不解析**它；
消费方（方法选取的调用者）拿文本回 `OperatorRegistry::resolve` 核对。**这是刻意的**，
不是遗漏；它换来 `continuum-method` 对 workspace crate 的依赖集合为空集（第 7 节），
正是 §8.3 那一行的直接兑现。

**待对账（R1，具名）**：若复审裁决 §8.3 的「← 无」不约束本 crate 对 `continuum-operator`
的依赖（例如读成仅约束第 8 层**层内**组件），则 `realized_by` 应改为
`Vec<OperatorRef>` 并登记 `continuum-operator` 一条边。本设计**取「无依赖」这一读法**，
因为它是该行字面取值。收件人：本组的复审者与协调者。

---

## 四、冻结的接口（四个算子块照着写的那一份）

新建 crate：**`continuum-method`**（`crates/continuum-method/`）。
切分文档 §一说六个子项目各新建一个 crate；本块占其一。

### 4.1 `MethodDomain`：§187 的目录名，封闭枚举

```
/// §187 的四个目录名（docs/spec/04-method.md:116-142）。
pub enum MethodDomain {
    Software,
    Video,
    Research,
    ThreeD,
}

impl MethodDomain {
    pub const ALL: [MethodDomain; 4];
    pub fn as_str(&self) -> &'static str;        // "software" | "video" | "research" | "3d"
    pub fn parse(s: &str) -> Option<MethodDomain>; // 穷尽 match，无通配臂
}
```

- `as_str` / `parse` 是**两个穷尽 `match`**，与 `ArtifactType` 的用法同形
  （`crates/continuum-artifact/src/artifact.rs` 的 `as_str`/`parse`，切分文档 §四第 2 条 (c) 提到的那一对）。
  加目录时编译失败，不会静默。
- **封闭**是设计决定，不是规范断言：§187 在列目录前写了「例如：」（`docs/spec/04-method.md:114`），
  《总纲》§8.1 又以四行界定（`docs/01-总纲.md:1331-1336`）。**没有规范说这四个是穷尽的**；
  取封闭是为了给四个算子块一个共同的词汇表（开放目录会让「未知域」到运行期才失败，
  与 P2 的 `EffectType` 取封闭同一理由）。此caveat 与它引出的缺口见第 8 节 R3。

### 4.2 `MethodId`：方法的标识，开放

```
pub struct MethodId(String);
impl MethodId {
    pub fn new(id: impl Into<String>) -> Self;
    pub fn as_str(&self) -> &str;
    // Display：MethodError 的格式串以 {id} 引用该字段（同 OperatorId 的理由，
    // crates/continuum-operator/src/definition.rs:32-38）。
}
```

**开放**而非封闭枚举，依据是 §187 的「例如：」（`docs/spec/04-method.md:114`）：
清单是**举例**，不是穷尽；且 P5b 的职责含「为四个领域各建一份方法目录」，目录要能容纳
§187 名之外的条目（见第 8 节 R3 的 image 域）。

### 4.3 `MethodEntry`：一条方法

```
pub struct MethodEntry {
    pub id: MethodId,
    pub domain: MethodDomain,
    /// 「这一类任务怎样做得可靠」的一句话，取自 §187 名对应的说明。
    pub purpose: String,
    /// 实现该方法的算子 id，**以文本登记**，本 crate 不解析（第 3.2 节）。
    pub realized_by: Vec<String>,
}
```

`MethodEntry` **不带** `input_schema`/`output_schema`/`determinism`/`side_effect_class`/
`backend_candidates` —— 那些是 `Operator` 的字段（§244），一条 Method 不是 Operator。

### 4.4 `MethodError`

```
pub enum MethodError {
    NotFound { id: MethodId },
    Duplicate { id: MethodId },
}
```

与 `OperatorError`（`crates/continuum-operator/src/registry.rs:7-13`）同形，只两枚变体。
**不新增第三个错误类型**（切分文档 §二第 4 条把「第二份错误类型」的禁令写给了
`Checkpointable` 那一处，本处同理不另造）。

### 4.5 `MethodRegistry`：登记与选取

```
pub struct MethodRegistry { /* entries: HashMap<MethodId, MethodEntry> */ }

impl MethodRegistry {
    pub fn new() -> Self;

    /// §187 的四份目录，逐名照录（第 5 节）。四条目录共 18 条。
    /// realized_by 初始为空，由四个算子块经 bind 填入。
    pub fn seeded() -> Self;

    pub fn register(&mut self, entry: MethodEntry) -> Result<(), MethodError>;

    pub fn resolve(&self, id: &MethodId) -> Result<&MethodEntry, MethodError>;

    /// 四个算子块填内容用的唯一入口：把某条方法的 realized_by 设为给定算子 id（文本）。
    /// 方法不存在时返回 NotFound；重复 bind 覆盖旧值。
    pub fn bind(&mut self, id: &MethodId, realized_by: &[&str]) -> Result<(), MethodError>;

    /// **按领域选取的入口**（切分文档 §五 P5b 行）。返回该领域的全部条目，
    /// **按 id 升序**排列（确定性：同一输入两次选取给出同一序列）。
    pub fn select(&self, domain: MethodDomain) -> Vec<&MethodEntry>;
}
```

**选取入口的形状就是 `select(MethodDomain) -> Vec<&MethodEntry>`**：
按领域取一份方法目录。§187 只以**领域**为键（`docs/spec/04-method.md:112`「针对不同领域」），
故 `select` **不吃**除领域外的参数；更细的选取（按任务性质、按风险）规范无来源，见第 8 节 R5。

`register` 撞同一 `MethodId` 返回 `Duplicate`；`resolve` 未注册返回 `NotFound`；
`bind` 对未注册 id 返回 `NotFound`。三条判据与 `OperatorRegistry` 的两条同形
（`registry.rs:24-48`）。

---

## 五、四份目录（`seeded()` 的内容，逐条出处）

`seeded()` 以 §187 的四目录逐名建立 18 条 `MethodEntry`，`domain` 与 `id` 照录，
`realized_by` 为空。四条目录与例名（`docs/spec/04-method.md:116-142`，与
`docs/01-总纲.md:1331-1336` 一致）。

**逐条计数（实测自上面两处）**：`software/` 7 条、`video/` 4 条、`research/` 4 条、
`3d/` 3 条，**合计 18 条**。

| domain | id（照录 §187） | 条数 |
|---|---|---|
| `software` | `planning` `TDD` `debugging` `worktree` `review` `fuzz` `verification` | 7 |
| `video` | `shot-analysis` `narrative-plan` `timeline-compose` `render-review` | 4 |
| `research` | `retrieval` `evidence-analysis` `contradiction-check` `citation-verification` | 4 |
| `3d` | `reconstruction` `geometry-validation` `render-comparison` | 3 |

`purpose` 的取值：§187 **只给名字，不给说明**。本设计**不从名字发明语义**——
每个 `purpose` 只写该名字在该目录下的原文，逐字取 `docs/spec/04-method.md:118-141` 的标识符本身
（如 `software/TDD` 的 `purpose` 为 `"TDD"`）。**这是刻意的留白**：把「这条方法具体怎么做才叫可靠」
留给四个算子块（它们知道本领域的算子与端口），不由 P5b 替它们写。
**待对账（R6，具名）**：§186（`docs/spec/04-method.md:53-99`）给了 `software/` 一类方法可参考的
执行序列（计划→实现→规格审查→质量审查→验证），但§186 是**流程**不是**方法库条目**；
本设计不把 §186 的流程名（如 `spec-review`、`quality-review`）塞进 `software/` 目录——
§187 的 `software/` 清单里没有它们。收件人：P5c。

---

## 六、四个算子块要照着填什么（逐块点名）

四块**只经 `bind` 填 `realized_by`**（切分文档 §四第 3 条「四块只填内容」），
**不得自造 `MethodEntry` 的字段、不得新建方法类型、不得注册进 `OperatorRegistry`**。
每块对其领域目录里的**每一条已 seed 的 id** 调用一次 `bind(id, &[...])`；
若某领域需要 §187 名之外的条目，**先报协调者**（那会动到第 3 节 R3/R6 的裁决），
不得自行 `register` 一个规范无来源的方法名。

| 块 | 领域目录 | 要填的 id（§187 照录） | 边界 |
|---|---|---|---|
| **P5c**（代码领域算子） | `software/` | `planning` `TDD` `debugging` `worktree` `review` `fuzz` `verification` | `worktree` 指 §16 的 git worktree（`docs/spec/01-concepts.md` §16；`docs/spec/05-normative.md` 无 §16，实质在 `docs/spec/01-concepts.md`）——P5c 填其 `realized_by` 时须与 §16 的 `git worktree` 语义一致；`verification` 只填**代码域的验证算子 id**，不是 P5a 的完成判定（后者不注册为算子） |
| **P5d**（研究领域算子） | `research/` | `retrieval` `evidence-analysis` `contradiction-check` `citation-verification` | `contradiction-check` / `citation-verification` 对应 §330 的 `ContradictionCheck` / `CitationVerification`；`realized_by` 填的是**算子 id**，证据本身走 P5a（切分文档 §四第 1 条，四块**只产出** Evidence） |
| **P5e**（媒体领域算子） | `video/` | `shot-analysis` `narrative-plan` `timeline-compose` `render-review` | 与 §32 剪辑链、§328 七种 Artifact 对齐；`timeline-compose` 的产物是 `Timeline`（§329），其 `ArtifactType` 变体由 P5e 一次性落地 |
| **P5f**（图像领域算子） | **无目录可填** | —— | §187 没有 `image/` 目录；见第 8 节 R3。P5f **不得**自行建域或借 `3d/` 域 |

**四块都不得**：定义自己的「方法」类型或注册表；把方法登记进 `OperatorRegistry`；
在 `MethodEntry` 上加字段；把 Worker 的完成声明塞进方法内容（切分文档 §八末条）。

---

## 七、`3d/` 无对家：怎么处置

切分文档 §八（`docs/superpowers/specs/2026-10-08-p5-scope-and-split.md:203-205`）已记：
§187 列 `3d/`（3 条方法），而 P1 的设计与《工程》§8.1 只列代码、研究、媒体、图像四个算子，
《总纲》第 8 章无 3D 节（§8.3–§8.6 是代码、研究、媒体、图像），**故 `Scene` 与 `3d/` 的归属本轮无解**。

**处置：记缺口，不硬塞给某一块。** 具体落点有三条：

1. **`3d/` 目录照样由 P5b 建立。** §五 P5b 的范围逐字是「为**四个领域**各建一份方法目录」——
   四个领域含 `3d/`。故 `MethodDomain::ThreeD` 存在，`seeded()` 建其 3 条条目，
   `select(MethodDomain::ThreeD)` 可选取。**这一半是无争议的**（P5b 自己的范围里就写着四个目录）。
2. **它的 `realized_by` 留空。** 没有算子块认领 `3d/`，故 3 条条目的 `realized_by` 恒为空 `Vec::new()`——
   **不是漏填**，是没有对家（第 6 节表里 P5f 那一行同理）。
3. **`Scene` 的归属不动。** P1 的设计 `:168` 把 `Scene` 列为 P5 的类型，但哪一块负责它
   **本轮无解**（切分文档 §八已记）；本设计**不认领**，也不把它牵进 `3d/` 的方法条目。

**据实标注**：上面第 1 条是**实测**（切分文档 §五 `:150` 与 §187 `:116-142` 的原文）；
第 2 条是**本设计的决定**（`realized_by` 可为空）——它不来自规范，来源是「没有块认领」这一事实。
**不写成构造性论证**，因为「无块认领」是对当前分块的观察，不是规范性质。

---

## 八、缺口与待对账条目

以下每条都具名，区分「实测」与「设计决定」。

- ⚠️ **R2（实测，缺口）`3d/` 无算子块。** 见第 7 节。规范与切分文档均未指定对家，
  本轮在 P5b 内只建目录、不填绑定。**无规范判据可依，据实留成缺口，不发明对家。**
- ⚠️ **R3（实测，缺口，切分文档 §八未记）image 域无目录，但 §五 P5f 声称有目录条目。**
  切分文档 §五 P5f 行（`:154`）逐字含「本领域的 Operator 集**与它在 P5b 目录里的条目**」，
  而 §187（`docs/spec/04-method.md:116-142`）与《总纲》§8.1（`docs/01-总纲.md:1331-1336`）
  **都没有 `image/` 目录**（实测：`grep -n "image/\|images/" docs/spec/ docs/01-总纲.md docs/02-工程.md`
  无命中）。**故 P5f 被要求有条目，却没有可落入的目录名。** 这是与 `3d/` **对偶**的缺口——
  §八只记了 `3d/` 一侧。**处置同 R2：不发明 `image/` 目录**（发明即与 §187 的四目录冲突），
  记为缺口，待协调者裁决：或给 §187 补一个 `image/` 目录，或裁定 image 的方法并入既有某目录。
  在裁决前，`MethodDomain` 无 image 取值，P5f 无 `bind` 目标。
- ⚠️ **R4（缺口，无规范判据）任务的「领域」如何判定无来源。** §187 说「针对不同领域选择」
  （`docs/spec/04-method.md:112`），但**没有任何字段把 Intent / Contract / Plan / Node 关联到
  `MethodDomain`**：P1 的 `Node`（`crates/continuum-graph/src/node.rs:8-21`）无领域字段，
  P4 的 `Plan`（P4 设计 §9.1，`docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md:1133-1142`）
  与 `TaskContract`（§224）也无。故 `select` 的**输入从何而来**规范未定。
  本设计只冻 `select(domain)` 这个入口，**不发明「领域判定器」**。**收件人：P4（Plan/TaskContract
  是否携带领域）与协调者。**
- ⚠️ **R5（缺口，无规范判据）`select` 的消费点无来源。** 谁在何时调 `select`——
  规划期（P4 的 Planner）还是 `Queued → Running`（第 3 层执行器）——《工程》§8.3 未写，
  §187 只说「用于选择」。**本设计不指定消费点**（切分文档 §四第 4 条亦禁止 P5 自建执行路径）。
  **收件人：P1 执行器与 P4 Planner 的设计者，协调者裁决。**
- **R1（待对账）§8.3「← 无」的读法。** 见第 3.2 节：它决定 `realized_by` 取文本还是 `OperatorRef`。
  本设计取「无依赖」读法。**收件人：复审者与协调者。**
- **R6（待对账）§186 的流程名是否入 `software/` 目录。** 见第 5 节末。本设计取「不入」。
  **收件人：P5c。**
- ⚠️ **R7（全称措辞降级）本设计不写「方法库只有 N 种方法」。** §187 的清单以「例如：」开头
  （`docs/spec/04-method.md:114`），**非穷尽**。故全文只说「`seeded()` 依 §187 建 18 条」，
  不说「方法共 18 种」。同理不写「四个领域是全部领域」。

**找不到规范判据、据实留成缺口的项**：R2、R3、R4、R5（打 ⚠️ 者）。
**这些项本设计均未发明填法**，只登记入口形状（`MethodDomain` / `select`）并把输入来源留空。

---

## 九、依赖边（切分文档 §四第 6 条）

`continuum-method` 的 workspace 内依赖集合为**空集**（§8.3「← 无」，第 3.2 节）。
按切分文档 §一（`:36`），新增 crate 会**先让 `dependency_direction` 门变红**再被 `ALLOWED` 补齐，
这是刻意的。落点两处（均由实现计划执行，不在本设计内改）：

- `Cargo.toml` 的 `[workspace] members` 加 `"crates/continuum-method"`。
- `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED`（`:25`）加一行
  `("continuum-method", &[])`——**空数组是本设计的硬断言**：任何反向边（含指向
  `continuum-operator` 者）都会让该门变红。这正是 R1 那条读法的**编译期照片**。

外部依赖（`serde` / `thiserror` 等）不进 `ALLOWED`（该表断言的是 workspace 成员之间的边，
见 `dependency_direction.rs:200-204`）；是否需要它们由实现计划定，本设计不预设。

---

## 十、实现计划的验收判据（设计层预先挂出）

以下是实现该接口时**必须**成立的判据，供实现计划的用例承接（本设计不写用例，只挂判据）：

1. **依赖门**：`ALLOWED` 含 `("continuum-method", &[])`，且 `continuum-method` 对每个
   其它 workspace 成员的断言均为「未出现」——逐对 `assert_eq!`（`dependency_direction.rs:307-325` 的形状）。
2. **目录封闭而可往返**：`MethodDomain::ALL.len() == 4`；`as_str`/`parse` 对**四值逐一**往返
   （不是一值抽样）；`parse` 对四个 `as_str` 之外的输入返回 `None`（逐项，含 `"image"`）。
3. **名册逐项有照片**：`seeded()` 后逐 domain 计数 `software=7 / video=4 / research=4 / 3d=3`，
   且四条目录的 id 集合**逐名**等于第 5 节表（漏一名即红，多一名即红）。
4. **两条错误路径各一枚**：`register` 撞同名 → `Duplicate`；`resolve` 未注册 → `NotFound`；
   `bind` 未注册 → `NotFound`。
5. **选取确定性**：同一 registry 对同一 domain 两次 `select` 给出同一序列，且序列按 id 升序。
6. **`3d/` 留空是一项被断言的事实**：`select(MethodDomain::ThreeD)` 的条目 `realized_by` 逐条为空
   （第 7 节第 2 条）——这条断言在 R2 裁决前成立，裁决后按裁决改。

---

## 十一、与其它块的接口（一句话汇总）

- **P5c/P5d/P5e/P5f**：经 `bind` 填 `realized_by`（第 6 节表）；不自造登记形态。
- **P1（执行层）**：不接口。EML 不入 `OperatorRegistry`，不接管状态迁移。
- **P4（语义层）**：无接口面；但 R4（领域从何而来）若 P4 的 Plan/Contract 要携带领域，
  需与 P4 设计对账。
- **P5a**：不接口。方法库不含证据与判定（切分文档 §四第 1 条）。
