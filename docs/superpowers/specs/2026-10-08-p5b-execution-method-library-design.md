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
| §187 | `docs/spec/04-method.md:102-155` | 定义 Execution Method Library；四目录与例名；ADFIR 与 Method 的分工 |
| §186 | `docs/spec/04-method.md:53-99` | Superpowers 式工程执行；「计划先于实现／任务上下文隔离／逐任务验证／规格审查与质量审查分离」 |
| 《工程》§8.1 第 10 行 | `docs/02-工程.md:494` | `| Execution Method Library | §187 |` |
| 《工程》§8.3 | `docs/02-工程.md:526` | `Execution Method Library ← 无`（本块依赖面的权威行；**不写「唯一」**——同文件 §9.2 的入度为零表本应含层 8 而未含，见第 3.2 节末） |
| §218 | `docs/spec/04-method.md:1342`、`:1373-1377` | 组件表列 Execution Method Library；闭环步 `ADFIR ↓ Execution Method ↓ Model / Tool / Compute Routing` |
| §216 | `docs/spec/04-method.md:1177`、`:1185` | Runtime Learning Loop 优化「哪些 Execution Method 更可靠」 |
| 《总纲》§8.1 | `docs/01-总纲.md:1327-1343` | 执行方法库；四目录清单；ADFIR 与 Method 的分工 |

§187 的分工原文（`docs/spec/04-method.md:144-155`，实测）：

```
ADFIR 决定：           做什么 / 依赖什么
Execution Method 决定： 这一类任务怎样做得可靠
```

**本块据此只登记「怎样做得可靠」，不登记「做什么」。** 「怎样做得可靠」在这里**是一组名字
（§187 的方法名）加它们的领域目录**，不是一段说明文本——为什么没有说明文本，见第 4.3 节。
「做什么／依赖什么」是 ADFIR 的
图结构与节点（P1 的设计 `docs/superpowers/specs/2026-10-01-p1-execution-layer-design.md:185-245`）。

### 本块的接口面是「先冻」的那一份

切分文档 §四第 3 条逐字（**按小节引、不引行号**：该文件在本设计定稿期间被并行编辑，
行号实测一次即失效；文件是 `docs/superpowers/specs/2026-10-08-p5-scope-and-split.md`），
其中「…」是本设计写入的订正注记（见下段）：

> **P5b 的方法登记接口先冻。** §187 的四个目录（`software/` `video/` `research/` `3d/`）
> 与四个领域算子块**在已配对的那三对上是一对一**：`software/` 对 P5c、`research/` 对 P5d、
> `video/` 对 P5e，而 `3d/` **在四个具名算子里没有对家**（见 §八）、P5f 则没有目录（见 §八）。
> …故 P5c/P5d/P5e/P5f 的「方法」条目的
> 登记形态归 P5b，四块只填内容；**在 P5b 的设计过审前，四个算子块不得自造方法登记形态**。

**这一句在 2026-10-09 订正过，来历须记**：切分文档 §四第 3 条原写「与四个领域算子块是**多对多**」，
而它自己给的配对（`software/`→P5c、`research/`→P5d、`video/`→P5e）以及同文件 §八的
「**目录与算子块不是一一对应，而是「四对四减去两处错位」**」都指向**一对一的三对配对**，
故「多对多」与它自己的正文相抵。**实测结论是「一对一（三对）＋两处无对应」**，
见第 5.1 节末对 `bind` 覆盖语义的处置。切分文档那一句已按同口径改（改动来历记在该句下）。

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
   （那是 P5a；`crates/continuum-graph/src/node.rs:19` 的注记「结构保存，判定属 P5」指 P5a，
   注记在 `:19`、它注释的字段 `verification_policy` 在 `:20`）。
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
| 携带 | `input_schema`/`output_schema`/`determinism`/`side_effect_class`/`backend_candidates` | `id`/`domain`，加一条**文本**的 `realized_by`（**没有**「怎样做得可靠」的文本字段，见第 4.3 节） |
| 语义 | **做什么**的可执行单元 | **怎样做得可靠**的规矩 |
| 可否执行 | 是（经第 3 层执行器） | **否**；它不是执行单元 |

一条 Method **不进入** `OperatorRegistry`，一条 Operator **不进入** `MethodRegistry`。
两者的交集只在本设计第 3.2 节的那一条**弱引用**上。

### 3.2 方法到算子的引用是**文本**的，理由是一个被否决的更显然写法（两条）

更显然的写法是 `realized_by: Vec<OperatorRef>`（`OperatorRef` 的唯一定义在
`crates/continuum-graph/src/node.rs:42`，由 `OperatorId`+`OperatorVersion` 定位）。
**这里不是那样的理由有两条，各自独立、方向一致**：

1. **它带来的直接边不是 `continuum-operator`，而是 `continuum-graph`。**
   `OperatorRef` 住在 `continuum-graph`（`crates/continuum-graph/src/lib.rs:24` 只 `pub use`），
   故用它会登记 `("continuum-method", &["continuum-graph"])`。
   `continuum-operator` 是 `continuum-graph` 自己的 `[dependencies]` 项
   （`crates/continuum-graph/Cargo.toml`），对 `continuum-method` 是**传递**边——
   而本仓的依赖门只查**直接**边（`cargo tree -p <pkg> --depth 1`，
   `crates/continuum-runtime/tests/dependency_direction.rs:270-285`），传递边**不进 `ALLOWED`**。
   （**订正 2026-10-09**：本节原写「那会让 `continuum-method` 依赖 `continuum-operator`」
   并据此提「登记 `continuum-operator` 一条边」，指错了 crate——按字面写会登记一条本不存在的边。）
2. **`continuum-graph` 正是 EML 被要求从中分出来的那一侧。**
   `continuum-graph/src/node.rs:1` 自称「ADFIR Node（§236）」，而切分文档 §五 P5b 行
   把本块的范围写成「把『这一类任务怎样做得可靠』从 ADFIR 的图结构里分出来」。**故 `Vec<OperatorRef>`
   不是「另一条对等读法下的合法选项」，而是让 EML 反向依赖它被要求分出去的那一侧。**

**这里不是那样：因为 §8.3 把本组件的依赖面写死为「无」。**《工程》§8.3
（`docs/02-工程.md:526`，实测原文）是：

```
Execution Method Library ← 无
```

同表紧邻的一行是 `领域算子 ← 第 3 层 Operator 注册表 + 第 4 层 Router`（`:527`）——
**领域算子**对第 3 层 Operator 注册表的依赖是**显式登记**的；而该行是**跨层**目标
（第 8 层行指向第 3 层），故 §8.3 这张表的取值**不限于层内**，不能读成「只约束层内组件」。
EML 这一行的取值是「无」，不是同一形状。故 EML **不得**命名 `OperatorId`。

**据实补一句「§8.3 不是唯一的对齐处」**：同文件 §9.2「入度为零的组件」
（`docs/02-工程.md:590` 的小节，表体 `:594-601`）该当把 `Execution Method Library` 列进去（§8.3 的「← 无」
正意味着它是层 8 的入度零点），而该表按层列了六项、**独缺「跨领域(8)」一行**。
两处对同一事实**一记一漏**，是工程文档内部的不一致。**本设计取 §8.3 的取值**，
并把它记为待对账（第 8 节 R8）。

落点：`MethodEntry.realized_by: Vec<String>` **以文本登记算子 id**，本 crate **不解析**它；
消费方（方法选取的调用者）拿文本回 `OperatorRegistry::resolve` 核对。**这是刻意的**，
不是遗漏；它换来 `continuum-method` 对 workspace crate 的依赖集合为空集（第 9 节），
正是 §8.3 那一行的直接兑现。

**这条文本弱引用没有照片，须写明（与第 9 节的「编译期照片」是两件事）**：

- **写错的 `realized_by` 照样编得过。** 它是 `String`，本 crate 不解析它，编译器无从核。
- **`continuum-method` 自身无法为它的内容出用例。** 因为本 crate 的 workspace 依赖集合是空集
  （第 9 节），它既看不到 `OperatorRegistry`，也无从知道某条方法该绑哪些算子 id。
  故「回 `OperatorRegistry::resolve` 核对」这件事只能落在**消费块**（方法选取的调用者）里。
- **故第 10 节的验收判据里没有任何一条覆盖 `realized_by` 的内容**：那里能断的只有
  「哪个 id 被绑过」（形状、是否非空），断不了「绑的是不是对的算子」。
  第 9 节所说「编译期照片」指的是**依赖边**（`ALLOWED` 里的空数组），**不是文本内容**；
  两件事分写在此，免得读者以为依赖门顺带保住了文本的正确性。

**待对账（R1，具名）**：本设计**取「无依赖」这一读法**（它是 §8.3 该行的字面取值）。
**原先那条「若读成仅约束层内，则改 `Vec<OperatorRef>` 并登记 `continuum-operator` 一条边」
的让步已收回**：§8.3 该行不是层内目标（见上），且 `OperatorRef` 的直接边是
`continuum-graph`——即 EML 被要求从中分出来的那一侧——故它不是对等的合法选项。
**本条的收件人：本组的复审者与协调者**（裁决点收窄为「§8.3 与 §9.2 两表不一致时以哪一张为准」）。

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

    /// 本轮**是否有算子块作它的对家**（第 7 节、第 8 节 R2）。
    /// Software/Video/Research → true；ThreeD → false。
    /// 这是**本设计的决定**（来源是当前分块，不是规范），不是规范断言。
    pub fn has_counterpart(&self) -> bool;
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
    // crates/continuum-operator/src/definition.rs:36-38 的那段注释；该文件 :38-42 是 Display 本体）。
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
    /// 实现该方法的算子 id，**以文本登记**，本 crate 不解析（第 3.2 节）。
    pub realized_by: Vec<String>,
}
```

`MethodEntry` **不带** `input_schema`/`output_schema`/`determinism`/`side_effect_class`/
`backend_candidates` —— 那些是 `Operator` 的字段（§244），一条 Method 不是 Operator。

#### 为什么**没有** `purpose`（「怎样做得可靠」的文本承载）

**本层不定义「怎样做得可靠」的文本承载。** 理由三条，按分量排：

1. **§187 只给名，不给说明。** §187 的表（`docs/spec/04-method.md:116-142`）是四目录下的
   标识符清单（`planning`/`TDD`/…），**没有任何一句话描述某条方法怎样才叫可靠**。
   **本设计逐行读过的 §186（`:53-99`）、§187（`:102-155`）、§218（`:1279-1380`）里没有
   「方法库条目的说明文本」这一形态**（这是已读范围的实测，不是对 `docs/spec/` 全量的穷尽结论）——
   故填 `purpose` 没有规范来源。
2. **没有写入路径的字段就是死字段。** 先前版本写了一个 `purpose: String`，注解说它
   「取自 §187 名对应的说明」（**那说明不存在**），旧版第 5 节又说这个内容「留给四个算子块」——
   而唯一的填充入口 `bind` 只碰 `realized_by`、`register` 被四块禁止（第 6 节）。
   两处说法互斥，且**都填不进去**：`purpose` 恒等于 `id`，永无真实内容。
3. **留一个填得进去的口等于让四块自造语义。** 若把 `bind` 拓宽成也写 `purpose`，
   就等于让四块各自发明「这条方法怎样才可靠」——那正是本节的取向（不从名字发明语义）
   与 R7 要避免的。故两条路里取**去掉字段**这一条。

**去掉的是字段，不是这件事。** 「怎样做得可靠」的内容并不因此丢失：它落在各领域的
**Operator 定义**（§244 的 `Operator`，那是可执行、带 schema 的一侧）上，而方法条目是
**按领域选取的目录索引**——`id` 本身就是 §187 给的方法名，`realized_by` 指向兑现它的算子。
**条目不是知识的承载者，是索引。**

**据实记的缺口**：本层**不定义**「方法的可靠性说明」这一文本形态。若后续某块需要它
（例如 P5c 想把 §186 的流程写成条目说明），那是**新需求**，须先报协调者，不在本设计内发明。
**这条与 R7 同口径**：§187 的清单以「例如：」开头、非穷尽，本设计只登记 §187 给得出的东西。
收件人：四个算子块与协调者。

### 4.4 `MethodError`

```
pub enum MethodError {
    NotFound { id: MethodId },
    Duplicate { id: MethodId },
}
```

与 `OperatorError`（`crates/continuum-operator/src/registry.rs:7-12`）同形，只两枚变体。
**不新增第三个错误类型**（切分文档 §二第 4 条把「第二份错误类型」的禁令写给了
`Checkpointable` 那一处，本处同理不另造）。

### 4.5 `MethodRegistry`：登记与选取

```
pub struct MethodRegistry { /* entries: HashMap<MethodId, MethodEntry> */ }

impl MethodRegistry {
    pub fn new() -> Self;

    /// §187 的四份目录，逐名照录（第 5 节）。四条目录共 18 条。
    /// realized_by 初始为空：`ThreeD` 的三条是**刻意**留空（无对家，第 7 节第 2 条），
    /// 其余 15 条待各自的算子块经 bind 填入。
    pub fn seeded() -> Self;

    pub fn register(&mut self, entry: MethodEntry) -> Result<(), MethodError>;

    pub fn resolve(&self, id: &MethodId) -> Result<&MethodEntry, MethodError>;

    /// 四个算子块填内容用的唯一入口：把某条方法的 realized_by 设为给定算子 id（文本）。
    /// 方法不存在时返回 NotFound；重复 bind 覆盖旧值（同一块重复调它即覆盖）。
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

**空的 `realized_by` 有两个来源，靠 `MethodDomain::has_counterpart` 分开。**
一个空 `Vec` 既可能是「**无对家**」（`ThreeD`，第 7 节第 2 条），也可能是「**四块还没 bind**」。
`MethodEntry` 本身分不清，故判据不能只看空不空，要用谓词一起判：

- `has_counterpart(ThreeD) == false` ⇒ 该域逐条**允许**为空，且**应当**为空（刻意留空）。
- `has_counterpart(d) == true` ⇒ 该域逐条**必须非空**，空就是「漏 bind」。
  这条断言在 P5b 自己的 crate 里**闭不了**——四块在别的 crate、在其后运行（第 6 节），
  故它是**交给四块各自计划的判据**，并由整合作一次收口（第 10 节判据 6）。

**`register`/`resolve` 的具名消费点**：`register` 的唯一调用者是内部的 `seeded()`
（四块被禁止调它），`MethodError::Duplicate` 只在这条路径上可达；方法侧的 `resolve`
与 `select` 的消费点是同一处（方法选取的调用者），而那一处**规范未指定**（第 8 节 R5）。
两者与 `OperatorRegistry` 同形，保留可辩，但本设计**不声称它们今天有具名消费点**。

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

每条条目的 `id` 逐字取 `docs/spec/04-method.md:118-141` 的标识符本身；**没有 `purpose` 之类的
说明字段**，理由见第 4.3 节（§187 只给名，不给说明；本层不定义「怎样做得可靠」的文本承载）。

### 5.1 目录与算子块的对应是**一对一**（三对），不是「多对多」

**实测结论**：§187 的四个目录与四个具名算子块（《工程》§8.1 第 11–14 行：
`docs/02-工程.md:495-498` 的代码/研究/媒体/图像）的对应是——`software/`↔P5c、`research/`↔P5d、
`video/`↔P5e，共**三对一对一**；`3d/` 无对家（第 8 节 R2）、P5f（图像）无目录（第 8 节 R3）。

两处互相独立的实测都指向这个形状：

- 切分文档 §五 的各块行写的都是「**它**在 P5b 目录里的条目」——**单数一份目录**
  （P5f 那一行已因无对应物被订正掉）；
- 切分文档 §八 自己写的「**目录与算子块不是一一对应，而是「四对四减去两处错位」**」——
  四对四减去两处错位，剩的就是三对。

**这不是措辞问题，它决定 `bind` 的覆盖语义能否静默丢东西**：若同一 id 会被两块写，
`bind` 的「重复 bind 覆盖旧值」（第 4.5 节）会**静默丢掉**先写的那一块的贡献——
组装而无合并。**实测的形态是任何 id 只属于一份目录、一份目录只归一块**，
故跨块覆盖不会发生；覆盖语义只在「同一块重复调它」这一种情形下生效，不丢别人的贡献。

**据实留的形状**：本设计**不**加「同一 id 被两块写」的检测（接口里没有 `owner` 之类的字段），
因为一对一成立时它无处可触发。**若后续重新裁决成多对多**（例如 image 域被并进某一目录、
而 P5f 仍独立），则 `bind` 须改成追加或显式合并——**那条不在本设计内**，记为待对账（第 8 节 R9）。

**待对账（R6，具名）**：§186（`docs/spec/04-method.md:53-99`）给了 `software/` 一类方法可参考的
执行序列（计划→实现→规格审查→质量审查→验证），但§186 是**流程**不是**方法库条目**；
本设计不把 §186 的流程名（如 `spec-review`、`quality-review`）塞进 `software/` 目录——
§187 的 `software/` 清单里没有它们。收件人：P5c。

---

## 六、四个算子块要照着填什么（逐块点名）

四块**只经 `bind` 填 `realized_by`**（切分文档 §四第 3 条「四块只填内容」），
**不得自造 `MethodEntry` 的字段、不得新建方法类型、不得注册进 `OperatorRegistry`**。
每块对其领域目录里的**每一条已 seed 的 id** 调用一次 `bind(id, &[...])`——
**「每条」是判据不是劝告**：`has_counterpart(该域) == true`（第 4.1 节），故该域每条条目的
`realized_by` **必须非空**；漏 bind 一条即「空着」，由第 10 节判据 6 抓（该判据须由本表四块的
计划各自承接，见第 4.5 节）。
若某领域需要 §187 名之外的条目，**先报协调者**（那会动到第 8 节 R3/R6/R9 的裁决），
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

切分文档 §八的 `3d/` 那条（同文件，按小节引）已记：
§187 列 `3d/`（3 条方法），而 P1 的设计与《工程》§8.1 只列代码、研究、媒体、图像四个算子，
《总纲》第 8 章无 3D 节（§8.3–§8.6 是代码、研究、媒体、图像），**故 `Scene` 与 `3d/` 的归属本轮无解**。

**处置：记缺口，不硬塞给某一块。** 具体落点有三条：

1. **`3d/` 目录照样由 P5b 建立。** §五 P5b 的范围逐字是「为**四个领域**各建一份方法目录」——
   四个领域含 `3d/`。故 `MethodDomain::ThreeD` 存在，`seeded()` 建其 3 条条目，
   `select(MethodDomain::ThreeD)` 可选取。**这一半是无争议的**（P5b 自己的范围里就写着四个目录）。
2. **它的 `realized_by` 留空，且这条「无对家」由 `MethodDomain::has_counterpart(ThreeD) == false`
   显式承载**（第 4.1 节）。没有算子块认领 `3d/`，故 3 条条目的 `realized_by` 恒为空 `Vec::new()`——
   **不是漏填**，是没有对家。**这个区别必须落在可断言的谓词上，不能只靠注释**：
   空的 `Vec` 同时也是「四块还没 bind」的形状，若无谓词，第 10 节判据 6 就分不清
   「`3d/` 刻意留空」与「某块漏绑」（第 4.5 节）。P5f 那一行（第 6 节表）不存在条目，故不适用。
3. **`Scene` 的归属不动。** P1 的设计 `:168` 把 `Scene` 列为 P5 的类型，但哪一块负责它
   **本轮无解**（切分文档 §八已记）；本设计**不认领**，也不把它牵进 `3d/` 的方法条目。

**据实标注**：上面第 1 条是**实测**（切分文档 §五 P5b 行与 §187 `:116-142` 的原文）；
第 2 条是**本设计的决定**（`realized_by` 可为空，并把「无对家」编码成 `has_counterpart`）
——它不来自规范，来源是「没有块认领」这一事实。
**不写成构造性论证**，因为「无块认领」是对当前分块的观察，不是规范性质。
R2 被重新裁决（例如 `3d/` 分给了某一块）时，`has_counterpart` 与本节第 2 条一并改。

---

## 八、缺口与待对账条目

以下每条都具名，区分「实测」与「设计决定」。

- ⚠️ **R2（实测，缺口）`3d/` 无算子块。** 见第 7 节。规范与切分文档均未指定对家，
  本轮在 P5b 内只建目录、不填绑定。**无规范判据可依，据实留成缺口，不发明对家。**
- ⚠️ **R3（实测，缺口）image 域无目录，而切分文档 §五 P5f 行原先声称有目录条目。**
  **（先前此处写「切分文档 §八未记」——已不成立**：切分文档 §八 现已有这条「`image/` 没有目录，
  而 P5f 声称有——上一条的对偶（2026-10-09 补入）」。**本条的处置对象仍是「P5f 无 `bind` 目标」
  这一事实，它未因 §八 记账而消失**，只是缺口已两侧具名。）
  切分文档 §五 P5f 行**原先**逐字含「本领域的 Operator 集**与它在 P5b 目录里的条目**」
  （该行已订正，现写作「本行原先还写…——那句没有对应物」），
  而 §187（`docs/spec/04-method.md:116-142`）与《总纲》§8.1（`docs/01-总纲.md:1331-1336`）
  **都没有 `image/` 目录**（实测：`grep -n "image/\|images/" docs/spec/ docs/01-总纲.md docs/02-工程.md`
  无命中）。**故 P5f 被要求有条目，却没有可落入的目录名。** 这是与 `3d/` **对偶**的缺口——
  §八只记了 `3d/` 一侧。**处置同 R2：不发明 `image/` 目录，理由是「无规范判据，据实记缺口，
  不发明」**，待协调者裁决：或给 §187 补一个 `image/` 目录，或裁定 image 的方法并入既有某目录。
  在裁决前，`MethodDomain` 无 image 取值，P5f 无 `bind` 目标。
  （**订正 2026-10-09**：本条原写的理由是「发明即与 §187 的四目录冲突」——
  那与 R7「§187 的清单以『例如：』开头、**非穷尽**」自相矛盾：清单非穷尽时，
  补一个 `image/` 目录**不构成冲突**。动作（不发明）不变，理由换成与 R7 一致的那条。）
- ⚠️ **R4（缺口，无规范判据）任务的「领域」如何判定无来源。** §187 说「针对不同领域选择」
  （`docs/spec/04-method.md:112`），但**没有任何字段把 Intent / Contract / Plan / Node 关联到
  `MethodDomain`**：P1 的 `Node`（`crates/continuum-graph/src/node.rs:9-22`）无领域字段，
  P4 的 `Plan`（P4 设计 §9.1，`docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md:1133-1142`）
  与 `TaskContract`（§224）也无。故 `select` 的**输入从何而来**规范未定。
  **补一句「规范里『领域』另有既定的表达面，而 `MethodDomain` 是本设计的新词汇」**：
  `docs/spec/01-concepts.md:498`（§9 Typed Artifact）写「领域差异主要通过 **Artifact 类型**表达」、
  `:538`（§10 Domain Operator）写「不同领域通过 **Operator** 扩展」——规范把「领域」的落点
  放在 Artifact 与 Operator 上，**没有一处说领域是一个可枚举的标签**。`MethodDomain`
  取的是 §187 的**目录名**，是本设计为方法库引入的新词汇，规范未与 Artifact/Operator 那一侧对齐；
  **这是 R4 缺口的一部分**，不只是「没有字段携带它」。本设计只冻 `select(domain)` 这个入口，
  **不发明「领域判定器」**。**收件人：P4（Plan/TaskContract 是否携带领域）与协调者。**
- ⚠️ **R5（缺口，部分有来源）`select` 的消费点：位置钉得住，调用者未定。**
  **先前写的「《工程》§8.3 未写、§187 只说『用于选择』⇒ 无规范来源」过强**，
  漏了同一份规范文件的另两处（**订正 2026-10-09**）：
  §218（`docs/spec/04-method.md:1342`、`:1373-1377`）把 Execution Method 列在闭环步
  `ADFIR ↓ Execution Method ↓ Model / Tool / Compute Routing` 上——**消费位置钉在
  「ADFIR 之后、Routing 之前」的执行路径上**；§216（`:1177`、`:1185`）的 Runtime Learning Loop
  消费并优化「哪些 Execution Method 更可靠」。
  **仍然未定的是「哪个组件在什么时候调 `select`」**——规划期（P4 的 Planner）还是
  `Queued → Running`（第 3 层执行器）。**本设计不指定消费点**（切分文档 §四第 4 条亦禁止
  P5 自建执行路径），只把上面两处来源补进依据表（第 1 节）。
  **收件人：P1 执行器与 P4 Planner 的设计者，协调者裁决。**
- **R1（待对账）§8.3「← 无」的读法。** 见第 3.2 节：它决定 `realized_by` 取文本还是 `OperatorRef`。
  本设计取「无依赖」读法（§8.3 该行是跨层表里的字面取值，不是层内目标）。
  **原先的让步（「若读成仅层内则改 `Vec<OperatorRef>` 并登记 `continuum-operator` 一条边」）
  已收回**：`OperatorRef` 的直接边是 `continuum-graph`，即 EML 被要求从中分出来的那一侧。
  **收件人：复审者与协调者**（裁决点已收窄为 R8）。
- **R6（待对账）§186 的流程名是否入 `software/` 目录。** 见第 5.1 节末。本设计取「不入」。
  **收件人：P5c。**
- ⚠️ **R7（全称措辞降级）本设计不写「方法库只有 N 种方法」。** §187 的清单以「例如：」开头
  （`docs/spec/04-method.md:114`），**非穷尽**。故全文只说「`seeded()` 依 §187 建 18 条」，
  不说「方法共 18 种」。同理不写「四个领域是全部领域」。
- ⚠️ **R8（待对账，工程文档内部不一致）§8.3 与 §9.2 对同一事实一记一漏。**
  §8.3 给层 8 的 EML 写了 `← 无`（`docs/02-工程.md:526`），即它是层 8 的入度零点；
  而 §9.2「入度为零的组件」（表体 `docs/02-工程.md:594-601`）按层列了六项、**独缺「跨领域(8)」**。
  两处本应对齐。**本设计取 §8.3 的取值**（`ALLOWED` 里 EML 的空数组即它的兑现），
  因为这正是本块要落地的那一行。**收件人：协调者**（须裁「两表不一致时以哪一张为准」；
  若改以 §9.2 为准，R1 的读法随之重开）。
- ⚠️ **R9（据实留的形状）「同一 id 被两块写」今天无处触发。** 目录与算子块是一对一（第 5.1 节），
  故 `bind` 的「重复覆盖」不会跨块丢贡献。**本设计不为此加检测**；若后续重裁成多对多
  （例如 image 域并入既有某目录而 P5f 仍独立），`bind` 须改成追加或显式合并。**收件人：协调者。**

**打 ⚠️ 者**（找不到规范判据、或两处来源相抵，据实留成缺口）：R2、R3、R4、R5、R8、R9。
**这些项本设计均未发明填法**，只登记入口形状（`MethodDomain` / `select` / `has_counterpart`）
并把输入来源留空。**R6 与 R7 是设计取向，不是缺口**。

---

## 九、依赖边（切分文档 §四第 6 条）

`continuum-method` 的 workspace 内依赖集合为**空集**（§8.3「← 无」，第 3.2 节）。
按切分文档 §一「共写文件」表里 `dependency_direction.rs` 那一行，新增 crate 会**先让
`dependency_direction` 门变红**再被 `ALLOWED` 补齐，
这是刻意的。落点两处（均由实现计划执行，不在本设计内改）：

- `Cargo.toml` 的 `[workspace] members` 加 `"crates/continuum-method"`。
- `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED`（`:25`）加一行
  `("continuum-method", &[])`——**空数组是本设计的硬断言**：任何**直接的**反向边
  （含指向 `continuum-graph` 者）都会让该门变红。这正是 R1 那条读法的**编译期照片**。
  **这张照片只覆盖依赖边，不覆盖 `realized_by` 的文本内容**（那在 `Vec<String>` 里，
  编译期无从核，见第 3.2 节末的三条）。该门查的是**直接**边（`cargo tree --depth 1`，
  `:270-285`），故对 `continuum-operator` 的传递依赖**不出现在这张照片里**——
  空数组挡的是「本 crate 自己写下 `use continuum_operator`」这一类。

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
6. **空与漏由谓词分开断，不能只看空不空**（第 4.5 节、第 7 节第 2 条）：
   - **无对家的那一侧**：`has_counterpart(ThreeD) == false`，且 `select(ThreeD)` 的条目
     `realized_by` **逐条为空**——这条断言在 R2 裁决前成立，裁决后按裁决改。
   - **有对家的那一侧**：`has_counterpart(d) == true` 的每个域（`Software`/`Video`/`Research`），
     在相应算子块的工作完成后，其条目 `realized_by` **逐条非空**——**任一条为空即「漏 bind」**，
     不是「刻意留空」。
   - **这条判据在 P5b 自己的 crate 里闭不了**（四块在别的 crate、在其后运行），
     它是**交给四块各自计划的判据**（第 6 节表），并由整合作一次收口。

---

## 十一、与其它块的接口（一句话汇总）

- **P5c/P5d/P5e/P5f**：经 `bind` 填 `realized_by`（第 6 节表）；不自造登记形态。
- **P1（执行层）**：不接口。EML 不入 `OperatorRegistry`，不接管状态迁移。
- **P4（语义层）**：无接口面；但 R4（领域从何而来）若 P4 的 Plan/Contract 要携带领域，
  需与 P4 设计对账。
- **P5a**：不接口。方法库不含证据与判定（切分文档 §四第 1 条）。
