# P3 子项目 B 设计：Connector（**边界层**）

> **层归属订正（2026-10-05）**：标题原标题为「P3 **资源层**（子项目 B）设计：Connector」。
> 用户当日裁定 Connector 归**边界层**，规范亦随之订正（§4.1 的组件表移行、§5.3 的层内图补行）——
> 依据与后果见第 7.2 节、裁决全文见 `docs/superpowers/specs/2026-10-05-p3-bcdf-set-decisions.md`
> 第一节第 3 条。**「子项目 B」这个编号不变**：它在 P3 分解 A–E 中的位置是**次序**，与层归属无关。
> 标题改而不只加注记，是因为**标题是最常被引用的一行**，把一处已知为假的层名留在那里、
> 把订正埋进第 7.2 节，正是本项目一贯拒收的「静默退化」——`final-b` 复核也点了这一处。

**范围**：§124 / §125 的落地。本子项目是 P3 分解（A–E）中的 **B**，次序 A → B → C → D → E。
A（Capability）已交付，B 是它两处「建了但无生产调用方」的兑现方之一：

- **强制点 (2) 的连接器侧**——`AuthorizedEffect` 今天**只有构造、无消费方**（`docs/superpowers/p3a-followups.md` 第一节的表），B 是它的消费方；
- **`continuum-secrets`**——今天连依赖边都没有（同上），B 是它的第一个真消费方。

本子项目**只写设计，不落实现代码**（共享面 §七）。

---

# 0. 四处错记与冲突，**全部已闭**（留来历，供后来者）

写设计前核过共享面与规范原文。按共享面的约定「本文里的判断若与你（用户）的决定冲突，以本文为准并回报」，
凡与共享面或规范对不上的**一律留在此处并回报**，**不在正文里偷偷按我认为对的那一版写**。

**四条的现状（2026-10-05 终审后）**：第 1 条是我的**假归因**（协调者核出并订正）；
第 2、3 条是我报出的**规范级缺口**（用户已裁、规范已订正，见
`docs/superpowers/specs/2026-10-05-p3-bcdf-set-decisions.md` 第一节第 3 条与第二节）；
第 4 条是两份前文之间的**收件人冲突**（协调者已订正）。**四条现在都已闭合，无待裁决项。**
本节整体留格，是为了让后来者看到每一处「当时为什么这么判、后来怎么合的」。

1. **§124 / §125 不在 `docs/spec/05-normative.md` 里。** 该文件是 §219 起的那一卷，
   这两个编号在**另一卷**：**§124 在 `docs/spec/02-positioning.md:1803`**（Service Connectors）、
   **§125 在同文件 `:1828`**（Connector Permission）；`docs/01-总纲.md:688-700` 是同一内容的
   工程侧复述（`§4.6 外部服务`）。本设计正文按真实出处引用。

   **这处记录已订正过一次，来历留在此处**：本节初稿曾写成「共享面 §二把 §124 / §125 记在
   `docs/spec/05-normative.md`」——**那句是假的**。共享面当时**只写了节号、没有写文件出处**，
   `05-normative` 这个串在它里面零命中，故不存在一条可被引错的出处，是我把「按习惯以为 § 都在
   05-normative」这根刺记到了别人头上。协调者核出后已在共享面 §二 补上两个真实出处
   （共享面 `2026-10-05-p3-bcdf-ownership-and-interfaces.md:56-57`），本处随之改成上面这一版。
   **收件人：无（已闭）**——留这段是为了让后来者看到「假断言连纠错本身也会生产」这条本项目的
   已知形态（`p3a-followups.md` 第七节第 1 条）在**文档**上也成立。

2. **§4.1 把 Connector 列在资源层，而 §4.2 把强制点 (2) 判在第 5 层——两处不一致（本条已闭）。**
   **用户已裁**：Connector 归边界层，规范随之订正（§4.1 的组件表里已无 Connector 一行，
   它移入 §5.1；§5.3 的层内图已补「`Connector ← 密钥运行时 + 第 4 层 Capability Token`」）。
   见第 7.2 节（含「我初稿的第二个理由已作废」的来历）与裁决记录第一节第 3 条。
   **收件人：无（已闭）。**

3. **§9.1 的层间依赖图缺 `资源层 (4) → 边界层 (5)` 这条边，而 P3A 已经需要它（本条已闭）。**
   `continuum-secrets` 是边界层（5），它依赖 `continuum-capability`（资源层 4），
   依赖边是 `["continuum-secrets", &["continuum-capability"]]`
   （`crates/continuum-runtime/tests/dependency_direction.rs:103`）；
   按 §9.1 的方向约定，这条在层图上写作 **`资源层 (4) → 边界层 (5)`**，
   而 §9.1 的图（`docs/02-工程.md`）里当时没有它。**这是 P3A 留下的既成事实，不是 B 引入的**，
   但在 B 这里会再叠一条同类边，故一并报出。
   **已订正**：协调者已把该边补进 §9.1，并**在同一处写明了箭头的读法**（「被依赖者 → 依赖者」），
   判据与佐证与本条下面那段逐条相同。**收件人：无（已闭）。**

   **§9.1 的方向约定（本节初稿在这里读反过，把来历留在下）**：`A → B` 读作 **「B 依赖 A」**，
   即箭头由**被依赖者**指向**依赖者**。两处独立佐证：

   - §9.2（`docs/02-工程.md:581-583`）说入度为零的组件「不依赖任何其他组件，是依赖图的**源点**」
     ——只有「入度 = 被依赖数」才推得出「源点」；若 `A → B` 读作「A 依赖 B」，入度为零的会是汇点。
   - 代码：`continuum-policy`（边界层 5）依赖 `continuum-effect`（执行层 3）
     （`dependency_direction.rs:79-81`），而 §9.1 画的正是 `执行层 (3) → 边界层 (5)`。

   **本节初稿写的是「即 5 → 4」——那是我按「箭尾依赖箭头」的直觉读的，与上面两处都不符。**
   留此一句，是为了让后来者看到这条约定确实会读反（与本项目「订正时把错误说法的来历留在原地」
   同一条手法）。**这条读反连带推翻了第 7.2 节的第二个理由**，见那里。

4. **`AuthorizedTool` 的消费方：两份前文曾冲突，已由协调者订正为一致（本条已闭）。**
   `p3a-followups.md` 第一节「B 的义务」第 3 条原写「**B 怎么消费 `AuthorizedTool` 属 B 的设计**」，
   而共享面 §一（2026-10-05，用户拍板）把「工具调用路径」判给 **F**、其载体正是 `AuthorizedTool`。
   协调者已于 2026-10-05 15:14 订正该条（`docs/superpowers/p3a-followups.md:41-44`）：
   「今天 `granted()` 没有任何消费方——工具调用路径（子项目 F）只读 `tool_id()`，
   **连接器的调用面（子项目 B）消费的是 `AuthorizedEffect::capability()`**」，并保留 P3A
   对「照录 + 保留重复项」的类型层决定不动。**两份现在一致：B 不消费 `AuthorizedTool`。**
   留格是为了让「曾经冲突、后来怎么合的」可查（第 11 节第 14 条）。

---

# 1. 建什么、不建什么

| 组件 | 规范依据 | 本子项目 |
|---|---|---|
| 连接器的操作集与它的**声明完整性** | §125 | 建 |
| **注册入口**（`ConnectorImpl` / `ConnectorRegistry::register`，绑定在这里产生） | §125 | 建（第 3.2.1 节） |
| 操作 → 授权的映射（**承重**，见第 3 节） | §125 §253 | 建 |
| 连接器侧的强制点 (2)：收下 `AuthorizedEffect` | §4.2 §51 | 建 |
| 凭据取得（`SecretsRuntime::issue` / `material`） | §51 §100 §103 | 建 |
| 连接器实现的装配与调用入口（新的 `continuum-connector` crate） | §124 §346 | 建 |
| `Connector` trait 本身（§124 的接口） | §124 | **不建**——P0/P1 已落地（`crates/continuum-provider/src/connector.rs:11-17`），本子项目不动它的形状 |
| 工具调用路径（`authorize` → `ToolProvider::invoke`） | §4.2 §252 §253 §313 | 不建（**子项目 F**；共享面 §一 已把它判给 Runtime） |
| Provider 适配器的注册/发现与中立性强制 | §315 §316 §80 | 不建（子项目 C） |
| 真实的外部服务连接器实现（GitHub / Email / …） | §124 | **不建**——零个。本子项目的全部用例跑在假连接器上（第 9 节明写这件事意味着什么） |
| §125 的五个操作（`read_repo` / `create_issue` / `merge` / `Email.read` / `Email.draft`）的**语义相配 kind** | §125 | **不补**——词汇表里没有与它们相配的那一枚（第 3.4 节）。**注意这五个操作本身仍注册得成**（作者可绑到任意一枚已有 kind，核对不查语义），本行不建的是**相配的臂**，不是「拒收这些操作」。臂的**机制**建，臂的**内容**不预先发明 |

**不建的部分不留桩。**

---

# 2. 操作集从哪来，谁验证一个「请求的操作」

## 2.1 声明者

`ConnectorDescriptor::new(id, operations)` 已经要求**显式声明**操作集，且非空
（`crates/continuum-core/src/connector.rs:77-84`，空列表 → `CoreError::ConnectorWithoutOperations`）；
`ConnectorOp::new` 在**构造期**拒掉空串与不含 `.` 的标识（同文件 `:31-37`，→ `MalformedConnectorOp`）。
两条合起来已把 §125 的字面要求（不得退化为服务级授权）落在类型上——这一点共享面 §二 已认定，
本设计**不推翻、不重做**。

故：**声明者是连接器实现自己**，经已冻结的 `Connector::descriptor()`
（`crates/continuum-provider/src/connector.rs:12-16`）。描述符是操作集的**唯一权威**。

## 2.2 验证者：今天没有，本子项目补上

`Connector::invoke` 的文档写着「`op` 必须是 `descriptor().operations` 中的一项」
（`crates/continuum-provider/src/connector.rs:15`）——**那是一句注释，不是类型**。
今天唯一检查它的是测试里的假实现（`crates/continuum-provider/tests/fake_provider.rs:136-143`
手写了一遍 `any(|o| o == op)`）。真实现可以忘掉它，而编译与测试都不会红。

这正是本项目点名的形状：机制写在一句注释里，谁都可以不接。**决定 B-2：把它变成结构性保证。**

**决定 B-2**：本子项目提供**唯一一条通往 `Connector::invoke` 的入口**，该入口在调用实现之前
完成**四步核对**（未注册 / 操作未声明 / 臂错 / kind 不符，逐条见第 5.3 节），
任一步不通过即拒，且**不调用实现**。
实现侧不再需要自己检查操作是否声明——那条检查只有一个产生点，就在入口里。

**限度（必须写下来）**：`Connector::invoke` 是公开方法，任何人都能直接调它。
故 B 能给的**不是**「外部调不到 `invoke`」，而是「**B 的入口**要过一个只有核对通过才产生的值」——
与 `mint` 公开所导致的限度同形（P3A 设计 §4.2 的执行期订正）。
若 B 将来绕开，那是**类型上表达得出来的选择**（收的是别的类型），而不是「忘了接线」这种查不出来的漏。

---

# 3. 承重决定：连接器操作与授权的映射

这是本子项目最重的一处。搞错的形态是**为「什么被授权」造出第二套词汇表**——本项目一贯判为 Critical。

## 3.1 先把规范里的两套串摆出来

| 出处 | 形状 | 给例 |
|---|---|---|
| §253（`docs/spec/05-normative.md:984-986`） | 能力的 `resource.action:scope` | `github.create_pr:repo/X`、`git.write:/task-worktree` |
| §125（`docs/spec/02-positioning.md:1828-1841`） | 连接器操作 `Service.operation` | `GitHub.read_repo`、`GitHub.create_issue`、`GitHub.push_branch`、`GitHub.merge`、`Email.read`、`Email.draft`、`Email.send` |

两者**同形**（都是 `A.B`），这正是本决定难的地方：看起来像是同一套词汇，于是很容易顺手让连接器操作
直接当能力用。它也确实**在一条通道上是同一套**——`CapabilityKind::Github(GithubAction::CreatePr)`
就是 §253 照录的 `github.create_pr`，而它与 §125 的 `GitHub.*` 是同一个拼法。

§124 另有一句定性，对本节与第 5 节都适用：**「Connector 不等于 Agent，它只是 Capability Provider」**
（`docs/spec/02-positioning.md:1818-1824`，`docs/01-总纲.md:690` 复述）。它支撑的是本节的主句
——连接器**既不决定、也不产出**授权，它在授权之下行动。**这句话的力度要说准**：它是四个词的一句
定性，规范没有定义「Capability Provider」。与 §253 一致的读法只有一个：能力的产出点是单一的
（`mint`，P3A 设计 §2.4），故连接器不是能力的**来源**，而是能力的**持有者与行使者**。
若反过来读成「连接器提供能力」，就与 §253 的单一签发点冲突——那个冲突里以 §253 为准。

## 3.2 决定

**决定 B-3（承重）：连接器操作不是授权单位，也不构成第二套授权词汇表。授权单位仍是
`CapabilityKind`（§253 的 resource/action 封闭枚举），它的唯一载体仍是 `Capability`，
唯一签发点仍是 `mint`。连接器操作是连接器自己的**调用面**：每个操作在注册时绑定到一枚
`CapabilityKind`。**

具体地：

1. **绑定在注册时声明**，由连接器自己给出：每个声明的操作配一枚 `CapabilityKind`
   （连接器说「我这一路调用对应哪一类动作」）。
2. **绑定在构造期强制双向覆盖**：声明集 ↔ 绑定集，缺一或多一即拒。与
   `ConnectorDescriptor::new` 拒空操作集同形——构造入口是唯一产生点，非法状态不可表达。
3. **绑定必须一一**（同一个连接器内，一枚 `CapabilityKind` 不得被两个操作绑定）。理由见 3.3。
4. **效应是推出来的，不是声明的。** 若绑定的 kind 落在 `CapabilityKind::for_effect`
   （`crates/continuum-capability/src/capability.rs:97`）的**像**里，这条操作就是效应型操作，
   其 `EffectType` 由该对应的**逆**给出；否则它没有外部效应。连接器**永不声明 `EffectType`**
   ——那会让「哪一枚 kind 配哪一条效应」多出一个可以由连接器自由填的产生点，而那条对应今天
   只有一个（`for_effect`），带六条逐项照片（`tests/vocabulary.rs`）。P2 为同类形状出过 Critical。

   逆对应需要一枚新函数（暂名 `CapabilityKind::effect`），**它应落在 `continuum-capability`
   里、紧邻 `for_effect`**：对应关系由那一处拥有，逆也必须只有一个产生点。它是本子项目对
   P3A crate 的唯一新增面（第 11 节第 11 条）。

5. **操作的服务半边必须与连接器自己的 id 相等**（注册期核对）。`ConnectorOp` 只要求串里有一个
   `.`（`crates/continuum-core/src/connector.rs:31-37`），故它**不保证**这个操作属于哪个服务
   ——一个 id 为 `Email` 的连接器可以声明 `GitHub.push_branch`，而双向覆盖与一一绑定都会通过。
   于是那条操作**永不可达**（入口第 1 步按服务半边解析，永远解析不到它）。这条核对把它挡在注册期。

### 3.2.1 绑定的产生点：注册入口（这张表此前缺它）

**形状示意，不是实现**：

```rust
/// 一条「操作 → 授权」的绑定。两个字段都是既有类型，本子项目不新增词汇表。
pub struct OpBinding { op: ConnectorOp, kind: CapabilityKind }

/// 连接器作者实现的是 B 的这个 trait（§4.5 有它为什么不是 §124 那个）。
pub trait ConnectorImpl: Send + Sync {
    /// 复用 §124 的 `ConnectorDescriptor`，**不另建第二个描述类型**（共享面 §二）。
    fn descriptor(&self) -> ConnectorDescriptor;
    /// 每个已声明操作恰一条。这就是绑定的产生点。
    fn bindings(&self) -> Vec<OpBinding>;
    async fn invoke_with(
        &self,
        op: &ConnectorOp,
        input: Value,
        material: &SecretMaterial,
    ) -> Result<Value, ProviderError>;
}

impl ConnectorRegistry {
    /// **注册期全部核对的唯一产生点**：双向覆盖（第 2 条）、一一（第 3 条）、
    /// 服务半边相符（第 5 条）。
    pub fn register(&mut self, connector: Box<dyn ConnectorImpl>) -> Result<(), ConnectorError>;
}
```

**与 `ConnectorDescriptor` 的关系：登记项含定义，不是并列**——`register` 从
`ConnectorImpl::descriptor()` 取出描述符并连绑定一起收下，**不改 `ConnectorDescriptor` 的字段**
（它只收 `(id, operations)`，`crates/continuum-core/src/connector.rs:77-84`），故 §10.3 冻结的接口
一个字不动。这与 P3A 的 `ToolProfile` 是同一手法：P3A 设计 §3.1 判「`Tool` 是**定义**，
`ToolProfile` 是**登记项**，且登记项含定义而非与它并列」。

**三条注册期变体的产生方就在这里**（§6.1）：`UnboundOperation`（声明了没绑）、
`UndeclaredBoundOperation`（绑了没声明）、`DuplicateKindBinding`（两条绑同一枚 kind）；
第 5 条另加 `OperationServiceMismatch { connector, op }`。

**`ConnectorRegistry` 也是入口第 1 步解析的地方**（§5.3）——注册期核过服务半边，解析才有唯一解。

**服务半边怎么取、比不比大小写**（第 5 条判据）：取第一个 `.` 之前的子串，**逐字比较、不折叠大小写**。
**被否掉的替代**：折叠大小写——`p3a-followups.md` 第四节第 11 条记过同一类形状的代价：
折叠只到「modulo ASCII 大小写」，于是两个源对「这是什么」的口径不同。此处同样的取舍，
只不过那时选的是接受，这里选的是拒收先于比较。

## 3.3 为什么绑到 `CapabilityKind` 而不是绑到 `EffectType`

两个理由，第二条是决定性的：

1. **「两个产生点」这条反对意见不成立。** 初稿曾据此选择绑到 `EffectType`（连接器声明效应、
   `for_effect` 推出 kind）。但把 3.2 第 4 条换过来之后，**两种绑法都只有一个产生点**：
   绑到 `CapabilityKind` 时效应由逆对应推出，连接器同样不声明它。故区别不在这里。
2. **`EffectType` 表达不了 §125 的读操作。** `EffectType` 按定义是「外部副作用」的封闭集合
   （`crates/continuum-effect/src/effect.rs:16-23`，六项），而 §125 的给例里有
   `GitHub.read_repo`、`Email.read`、`Email.draft`——它们**不是副作用，将来也不会是**：
   「读」这件事在定义上就不改变外部世界。绑到 `EffectType` 等于把它们从设计里**永久**排除。
   反过来，`CapabilityKind` 本来就装着非效应的动作。**十二枚 kind 里，非像的恰有六枚**
   （`for_effect` 是单射不是满射，像只有六枚），逐枚列全：
   `Filesystem(Read)`、**`Filesystem(Write)`**、`Git(Read)`、`Git(WorktreeWrite)`、
   `Git(CommitLocal)`、`Github(CreatePr)`——它们都是 §88 / §253 的**照录项**而非外部效应类型
   （P3A 设计 §2.2 明写这一点）。
   **这一句初稿只列了五枚，漏 `Filesystem(Write)`**（`docs/superpowers/plans/2026-10-05-p3bcdf-plan-stage-rulings.md`
   的 B1 裁为设计改）；**列全的意义在于它是枚举断言**——凡给数量的地方都必须逐项列得出来，
   否则读者无从发现漏项，而漏的那一枚恰好会让「非像有六枚」这句话在用例里对不上。
   §125 的读操作与它们是同一类东西，故应落在同一张表里。

**「一一」这条的理由**：§125 的意图是「`GitHub.merge` 能单独不授」。若两个操作绑同一 kind，
授权其一即授权另一，粒度退化到 kind 级——§125 禁的是服务级，这条不到服务级，但**它的意图落空**。
故取一一（fail-closed）。**被否掉的替代**：允许共用——理由是「两个操作本就是同一件事」；
不接受，因为本阶段没有任何真实连接器能证明「本就是同一件事」，而放宽的代价是不可逆的。

## 3.4 有语义相配的 kind 的操作只有两条，而「配得对不对」无人判

**本节初稿断言「§125 的那五条操作在本子项目里注册不了」——那句是假的，已按复审重写。**
假在哪：绑定是**连接器作者自己填的**（第 3.2 节第 1 条），而 `register` 的四条核对
（双向覆盖、一一、服务半边）**没有一条**核对「这条操作该绑哪一枚 kind」。作者把
`GitHub.merge` 绑到 `Github(CreatePr)` 或 `Filesystem(Read)` 上，三条核对全过、**注册成功**。
初稿把「表里没有语义相配的那一枚」读成了「注册不了」，那是两件事。

| §125 的给例 | 有没有**语义相配**的 kind | 语义相配的那一枚是什么 |
|---|---|---|
| `Email.send` | **有** | `Email(Send)` ＝ `for_effect(SendEmail)`（落在像里 → 效应臂） |
| `GitHub.push_branch` | **有** | `Git(Push)` ＝ `for_effect(PushBranch)`（同上） |
| `GitHub.read_repo` | **没有** | —（表里十二枚无一表示「读 GitHub 仓库」） |
| `GitHub.create_issue` | **没有** | — |
| `GitHub.merge` | **没有** | — |
| `Email.read` | **没有** | — |
| `Email.draft` | **没有** | — |

**可证的部分只到这里**：本子项目**不提供**与那五个操作语义相配的 kind。五条操作**照样能注册**
——把它们绑到任一枚已有 kind 上，四条核对全过，`register` 返回 `Ok`。

**而规范没有判定「配得对不对」的判据。这是本节的实质结论，也是一处规范级缺口。**
§125 只规定**粒度**（不得退化为服务级），**没有**规定「一条操作必须绑到哪一枚 kind」；
本设计也没有发明这条判据（那会是「替规范补一条它没写的对应」）。故本子项目给到的保证是**有边界的**：

- **保住了**：一条操作不拿到**它自己声明的那一枚 kind** 的能力，就走不到实现；
- **没保住**：那一枚 kind **是不是这条操作该要的那一枚**。作者绑错，没有任何东西会红。

**后果，逐条说清**：

- **本子项目不补这些臂。** 判据是「不预先发明：没有消费方的取值域不建」——今天零个真实连接器，
  补了也没有东西用它，而且补哪些（`ReadRepo`？`CreateIssue`？`Merge`？`Email.Draft`？）
  只能靠猜 §125 的给例，而 §125 明说那是「例如」。**补臂的消费方是第一个真提出这些操作的连接器。**
- **补臂也治不了这条缺口**：补一枚臂只让「语义相配的那一枚终于存在」，**判据本身仍然不存在**
  ——作者依旧可以绑到别的臂上。补臂是**让正确的绑法变得可写**，不是**让错误的绑法变得不可写**。
  这一点与本节初稿的说法不同（初稿把这个缺口与「没臂」混为一谈），故单列一条。
  补臂的形态按 P3A 设计 §2.2 的先例：往 `CapabilityKind` 那张封闭枚举里**加一枚臂**，
  那是**同一套词汇表的扩展**，不是第二套。
- **可注册的那两条也有一处接缝**：`GitHub.push_branch` 这个**操作**的服务半边是 `GitHub`，
  而它必须绑的那枚 kind 是 `Git(Push)`——**resource 是 `git` 不是 `github`**。这不是错：
  §88 规定 `git.push` 是一枚能力，驱动为 `--effect push_branch` 铸的正是它
  （`crates/continuum-runtime/src/task_cmd.rs:456-462`），故连接器若不绑它，`AuthorizedEffect`
  就配不上。但两套串的**服务半边对不齐**（连接器说 GitHub、能力说 git）这件事要记下来
  （第 11 节第 12 条）。

- **还有一条替代没被否掉，只是被判给别处**：把 `read_repo` 这类读操作表达成 **§316 的工具**，
  于是它落到强制点 (1)（`authorize`，载体 `AuthorizedTool`，`crates/continuum-capability/src/registry.rs`）
  上——强制点 (1) 判的是「**工具调用前**」，与「有没有副作用」无关，故读操作在它那里是够得着的。
  **本设计不采用，理由**：共享面 §一 刚把「工具调用路径」判给 **F**、把连接器判给 **B**，并
  写了一句话——「**不要**把工具当作『另一种外部资源』并进 §124 连接器」。把连接器操作并进工具
  是同一个错误的**镜像**，两边都做则「工具」与「连接器操作」两个概念重叠，正是本项目一贯判为
  缺陷的形态。**但这扇门不由本设计关死**：收件人 **F / C**（若两文判定读操作该走工具，
  第 3.5 节的非效应臂在这里要重新处置）。这条替代此前缺失，是复审指出的。

**照片（三张，与上面三段一一对应）**：

1. **「语义相配的那一枚是什么」有照片**：`Email.send` → `Email(Send)`、`GitHub.push_branch` →
   `Git(Push)`，各一条注册 + 一次成功调用。
2. **「绑错了照样注册」也有照片**——这条正是初稿写反的那一侧，现在**必须**照出来：
   建一个 id 为 `GitHub` 的假连接器，声明 `GitHub.merge`，绑定写成 `Github(CreatePr)`，
   断言 `register` 返回 **`Ok`**（而不是初稿写的「注册期拒」）。
   **没有这张照片，本节就是在替规范声称一条它没有的判据。**
3. **「配得对不对」没有照片，且写不出**：没有任何判据，故不存在「正确绑定」这一状态可照。
   这是这条缺口的形状本身，不是本子项目没做够（第 11 节第 16 条）。

## 3.5 两条臂的强度相等（写下来，免得被读反）

- **效应臂**：kind 落在 `for_effect` 的像里 → 必须出示 `AuthorizedEffect`（共享面 §四.2 强制点 (2)）。
- **非效应臂**：kind 不在像里 → 这条操作**没有外部效应**，故没有 `AuthorizedEffect` 可出示；
  B 收一枚**直接出示的 `Capability`**。

两条臂**在类型强度上相等**：`AuthorizedEffect::new` 是公开的，且它只核对「kind 与效应配对」
（P3A 设计 §4.2 的执行期订正），故两条臂能保证的都是「**出示方持有一枚 `mint` 铸出的、kind 相符的
能力**」——不多不少。效应臂多出来的那一分**不是类型强度**，而是它必然经过驱动侧强制点 (2)
那条路径（共享面 §四.2、P3A 设计 §10 第 7 条）。**不要把效应臂读成「不可伪造」**，
也不要由此推出「非效应臂没有强制点，故不该有」——规范对连接器的读权限**没有规定**
强制点落在哪里（§125 只规定粒度），这是本子项目不得不选的一处，**决定 B-3b**：
两条臂都由 B 的入口把关，判据同一条（出示的 kind 必须等于绑定的 kind）。

**「强度相等」说的是类型那一层，不包括来源。** 两条臂的那枚能力**从哪来**并不相同——
效应臂由驱动铸、作用域取自 `spec.target`；非效应臂由调用方铸、**作用域没有规定来源**。
那处不对称见第 4.2.1 节，**不要用本节的「相等」把它盖过去**。

---

# 4. 凭据怎么取

## 4.1 哪一枚能力，什么时候

出示的那枚能力**按臂取**，两臂都只从出示值里读，B 不自己造：

- **效应臂** → `AuthorizedEffect::capability()`（`crates/continuum-capability/src/effect.rs:73`）；
- **非效应臂** → 出示值里那枚 `Capability` 本身。

时机在**入口四步核对全过之后、调用实现之前**，**逐次调用**签发一枚：

```
入口四步核对（未注册 / 操作未声明 / 臂错 / kind 不符 —— 四者任一即拒）
  → SecretsRuntime::issue(<按臂取的那枚能力>, now)          // 强制点 (3)
  → SecretsRuntime::material(&credential, now)
  → 把材料交给实现
```

**逐次签发而不是连接器构造时签发一次**：作用域随能力逐次不同（同一次运行里可以有
`Git(GitPush, origin/main)` 与 `Git(GitPush, origin/release)` 两枚），
连接器级的一次签发没有可以放它的作用域。

**`SecretsRuntime` 由谁构造：本设计未定，且这一条有下游后果。** 两种形状都可以：

- **驱动装配好传进来**（`ConnectorRegistry::new(runtime)`）：于是
  `continuum-runtime → continuum-secrets` 是一条**真边**（那个 task 登记它）；
- **`continuum-connector` 自建**（自己读凭据源配置）：于是那条边**不是真边**，改由
  `continuum-connector` 承担 `continuum-secrets` 的构造，而 `runtime → secrets` **不该被登记**
  ——B 自己引的 §四.4 正是「零使用的边即假边」。

**本设计取第一种**（驱动装配），理由：`SecretsRuntime` 的构造要读凭据源的位置与内容
（`crates/continuum-secrets/src/source.rs`），那是**外部配置**，与驱动的装配处
（`crates/continuum-runtime/src/main.rs`）同一职责；连接器自带凭据源会让「凭据从哪来」
在装配处看不见。**这条决定把第 11 节第 15 条与 `runtime → secrets` 的边一起定了。**

## 4.2 凭据的作用域为什么不可能超出**出示的那枚能力**（**效应臂**；非效应臂见节末的限度）

节标题初稿写的是「不可能超出**这个操作被授权**的范围」——那是**说满了**，已改。
改后能证的只有「凭据不超出**出示的那枚能力**」，两条臂都成立；而「那枚能力配不配得上这条操作」
是**另一件事**，第 3.4 节说清了它为什么无人判。

三条合起来，且**每一条都落在已有的类型上，不是本子项目新加的**：

1. **`issue` 的签名里没有可以指定作用域的位置**——它只收那枚能力本身
   （`crates/continuum-secrets/src/runtime.rs:176`），凭据的 `scope` 直接取 `cap.scope()`
   （同文件 `:186`）。「拿窄能力去要宽凭据」不是被检查掉，而是**要不到**
   （照片：`crates/continuum-secrets/tests/compile_fail/issue_has_no_scope_parameter.rs`）。
2. **那枚能力的作用域不是 B 定的**：B 只是把那枚能力原样交给 `issue`。
   **B 不得重铸能力**——`mint` 虽公开，B 自己铸一枚就把下面整条链变成自证。
3. **凭据不越期**：`expiry` 取「源声称的」与「能力的」两者中更早的（同文件 `:183`）。

### 4.2.1 两条臂的作用域来源不同，这条区别不能糊过去（复审指出）

第 2 条初稿写的是「驱动铸能力时取 `spec.target`」——**那只是效应臂**。两条臂分开说：

| | 那枚能力从哪来 | 作用域从哪来 |
|---|---|---|
| **效应臂** | 驱动铸（`crates/continuum-runtime/src/task_cmd.rs:459-463`），随 `AuthorizedEffect` 传入 | 驱动的 `spec.target`——即命令声明的 `--effect <类型>:<目标>` |
| **非效应臂** | **调用方铸**（`mint` 公开），B 原样收下 | **本设计不规定。** 非效应的连接器操作在 §253 里没有一条规定作用域由谁给 |

**这是本设计承认的第二处缺口**（第一处是第 3.4 节的「配得对不对」无人判）：非效应臂的能力
**没有具名的产生方**，故它的作用域**没有来源**可言。B 能给到的保证因此只有
「凭据的作用域 == 出示的那枚能力的作用域」这一条**同义反复式**的相等，
**不是**「作用域是对的」。**收件人：第 11 节第 17 条。**

**为什么不因此删掉非效应臂**：删了它就等于说「§125 的读操作在设计上无处安放」，
而 §125 明写它们是权限（复审也据此推翻了初稿）。留着它、把缺口写明，
比删掉它、让缺口不可见，更符合本项目「不许含糊过去」的口径。

## 4.3 能力时效在 B 这里第一次进入一条会被执行的比较（**力度已按复审下调**）

**本节初稿写的是「B 是能力时效第一次变得可观察的地方」——那句无论证，已改。** 改后的说法只到
可证的那一步：

- `CAPABILITY_LIFETIME_MS` 今天只是个被记下的数——驱动不读时钟地用它
  （P3A 设计 §10 第 11 条、`p3a-followups.md` 第四节第 4 条）；
- B 是**第一个在生产路径上调用 `issue`** 的地方，而 `issue` 会走 `Capability::is_valid_at`
  并把 `CapabilityError` 转成 `SecretsError::Capability`（`runtime.rs:177`）。
  故这个常量**第一次进入一条会被执行的比较**。

**它「何时真的判出过期」本设计尚未规定，也不该在这里规定**：那取决于「铸出」与「取料」之间
由谁引入间隔。铸出在驱动（`task_cmd.rs:459-463`，取 `spec.target`，`expiry = now + CAPABILITY_LIFETIME_MS`），
而 B 的入口**今天没有生产调用方**（第 11 节第 **13b** 条），故这条间隔由谁引入**在接线之前无从判断**。

**与 F 的相抵，据实记下**：F 的设计从同一个前提（同一步铸出）判 `Expired` 在它的路径上**不可达**；
本节的初稿从同一个前提判它「第一次可观察」。**两条不可能同时为真，而至少有一条无据。**
本设计按上面改后的说法退到可证的一步，并把这条接缝记给 F（**第 11 节第 18 条**；
初稿这里引的「第 13 条」是**误引**——13 讲的是 §316 的交付通道，与本条不是同一件事，
`final-b` 复核据此指出本条在 §11 里**没有落点**，现已补上）。（**已由 F 选 (i) 收口**，
见 §11 第 18 条；F 的判据是「铸出 → 取用之间有间隔」，其中**本设计这条区间是「铸出 → `issue`」**，
见 F 设计 §11 `:821`、§8.1 `:697-712`。）

**B 不另写一次时效比较**：本项目里「能力还有效吗」只有 `Capability::is_valid_at` 一个产生点，
`issue` 已把它转出。B 要做的是**不吞这个 `Err`**，让它带着原来的变体出到调用方。

## 4.4 轮换的可观察范围和它的限度

`material` 会判三件事（`runtime.rs:238-256`）：凭据是否本运行时签出（`ForeignCredential`）、
签发它的轮换代是否仍是当前代（`Superseded`，§103 的轮换落点）、是否已到期（`CredentialExpired`）。

**限度（写下来而不是含糊过去）**：这三个判定只在**取料那一刻**发生。
材料一旦取出并交给实现，后续的 `rotate`（`runtime.rs:199`）**不影响这次调用**——
本阶段没有「用后即焚」或「调用中途复查」的机构。故「轮换使旧凭据失效」这条在本子项目里的
可观察范围是「下一次取料」，不是「正在进行中的那一次调用」。

## 4.5 材料怎么到实现手里：**不改 §124 的签名**

`Connector::invoke(&self, op, input) -> Result<Value, ProviderError>`
（`crates/continuum-provider/src/connector.rs:16`）**没有放凭据的位置**。这是本子项目发现的
一处接口缺口，处理如下。

**决定 B-4：用「逐次调用的适配器」承载凭据，§124 的 `invoke` 签名一个字不改。**

- 连接器作者实现的是 B 定义的扩展 trait **`ConnectorImpl`**（形状见第 3.2.1 节），
  它的 `invoke_with` 比 §124 的 `invoke` 多一个材料入参；
- B 的入口在签发凭据、取出材料之后，构造一个**逐次存活**的适配器（持有实现的引用与材料），
  对它调用 §124 的 `Connector::invoke`；适配器把参数转发给 `ConnectorImpl` 并附上材料。
- 适配器**只在入口里构造**，材料**只在调用期间存活**。

**被否掉的替代：把材料塞进 `input: Value`。** 否决理由不是风格：`input` 是
`serde_json::Value`，实现可以把它原样回显到返回值，而返回值会流向 Artifact / Journal / 审计
——那正是 §51 要拦的（「Agent 不直接获得所有密钥」）。要拦回来就得在出口扫材料，
那是 §1.2 点名的「手工校验层」，也正是本项目最容易失效的位置。

**另一条被否掉的替代：改 `Connector::invoke` 的签名加一个凭据入参。** 共享面 §二 声明三组接口
「无需重新冻结」，故不动它；适配器方案在不动签名的前提下拿到了同样的类型区分。

**本决定的限度**：适配器是 B 的入口构造的，但 `Connector::invoke` 是公开方法——
拿着连接器实现的引用直接调它的人绕过的是 B 的入口，不是 B 的接线。
与第 2.2 节的限度同一条，不重复。

---

# 5. 如何收下 `AuthorizedEffect`

## 5.1 收什么

B 的入口的签名收一个**两臂的出示值**（暂名 `ConnectorAuthorization`），两臂对应第 3.5 节的两条：

```rust
// 形状示意，不是实现
pub enum ConnectorAuthorization {
    /// 效应臂：绑定 kind 落在 for_effect 的像里时，只能走这一臂。
    Effect(AuthorizedEffect),
    /// 非效应臂：绑定 kind 不在像里时（无外部效应，故没有 AuthorizedEffect 可出示）。
    Capability(Capability),
}

pub async fn invoke(
    &self,
    authorization: ConnectorAuthorization,
    op: &ConnectorOp,
    input: Value,
    /// `now` 由调用方给。**入口不收时钟**——与 `Capability::is_valid_at`、`mint`、`issue`、
    /// `material`、`authorize` 同一条既有约定（本仓的核一律不取时钟）。
    now: i64,
) -> Result<Value, ConnectorError>
```

**`now` 是必需的入参，不是可选**：第 4.1 节的流程要 `issue(cap, now)` 与 `material(&cred, now)`，
两个 `now` 都从这一处来；**入口自己不取第二个时钟**——两次判定若各取一次，
「签发时未过期、取料时已过期」这条正好落在两次取值之间的情形就会被抹掉，
而那正是 §103 与 §51 要看得见的那一格。**入口内的两次判定共用同一个 `now`。**
（本条由 `plan-b` 查出、裁决文件 B2 裁为设计改。）
而本项目点名的缺陷形态正是「机制建好、路径不经过」。臂与绑定的对应由入口按 3.2 第 4 条
**推出来**再核对，不由调用方选。

**编译失败样例钉住的是哪一件事，要说准**：`ConnectorAuthorization` 有一个**公开可达的**
`Capability` 臂，故任何调用方都能写出 `ConnectorAuthorization::Capability(cap)`——样例只钉住
「`Effect` 变体的字段类型是 `AuthorizedEffect`」（在那个位置传 `Capability` 不编译），
**钉不住**「效应型操作不得用裸能力驱动」。后者是**运行期**核对（第 5.3 节第 3 步），
它的照片是第 9 节的 `EffectAuthorizationRequired` 那一条。**不要把两者混为一谈**
（本节初稿与第 10 节曾把它写成编译期性质，已订正）。

## 5.2 凭什么认为这个值经过了校验

本节说的是**效应臂**（非效应臂没有「值经过了校验」这回事——它出示的就是一枚裸能力，
第 3.5 节已说明两臂强度相等）。

**凭的是配对，不是不可构造**——这一条是 P3A 执行期订正过的，不能按原稿读
（P3A 设计 §4.2 的执行期订正、§10 第 10 条；`p3a-followups.md` 第五节第 2 条）。

能说的准确版本是：

- `AuthorizedEffect::new` 在收下之前核对 `capability.kind() == CapabilityKind::for_effect(effect)`，
  不符即 `CapabilityError::EffectCapabilityMismatch`（`crates/continuum-capability/src/effect.rs:55-65`）；
- `Capability` 的唯一公开产出路径是 `mint`（设计 §2.4），`mint` 是公开的——**这是设计承认的**。

故**持有某效应的 `AuthorizedEffect`，就必然持有 `mint` 铸出、且 kind 与那条效应相符的能力**。
B 能且只能相信到这个程度。B **不能**从 `AuthorizedEffect` 判断的事，一并列出：

| B**不能**从它判断 | 现状 |
|---|---|
| 那枚能力的**作用域**是不是「该去的地方」 | 连接器的操作不约束作用域（绑定的是 kind，不是目标），B 无从判。作用域随能力原样进入凭据——第 4.2 节说了它为什么不会被放宽 |
| 那枚能力**是不是真的过了策略裁决** | 不能判。若调用方自己 `mint`，B 看不出来。这是 `mint` 公开的性质，不是 B 的漏 |
| 「谁」铸的、为什么铸 | 不在类型里（P3A 设计 §2.4：那一位信息属驱动，写在 Effect Journal 的 `authorization` 字段） |

## 5.3 入口核对的四步与它们的次序

1. 按操作串的**服务半边**（第一个 `.` 之前的子串，逐字比较）解析连接器；未注册 → `UnknownConnector`。
   **解析之所以唯一，靠的是注册期那条「服务半边 == 连接器 id」的核对**（第 3.2 节第 5 条），
   没有它，一个连接器可以声明不属于自己的操作，而那条操作在这里永远解析不到；
2. `op ∈ descriptor().operations()`；否则 → `UndeclaredOperation`；

   **订正（Task 4 评审，实现已按更严的一侧做）：实现这里读的是「绑定表」，不是
   `descriptor().operations()`。** 原句照留在此，理由与等价性写在下面——**不写清楚，
   后来者会把「读绑定表」读成偷懒**：

   - **为什么等价**：注册期那两条核对已经给出**双向包含**——`UnboundOperation`
     （声明了没绑，第 3.2.1 节）挡掉「声明集 ⊄ 键集」，`UndeclaredBoundOperation`
     （绑了没声明，同处）挡掉「键集 ⊄ 声明集」；两者都在**写表之前**、且都覆盖全集，
     合起来即 **键集 == 声明集**。故读哪一份**判据上等价**。
   - **那为什么还选绑定表**：**避免同一个判断有两个产生点**。两者若各读一份，就有了
     「两次 `descriptor()` 之间声明集被换掉」这条分叉——第 2 步拿 `descriptor()` 判、
     第 3、4 步拿绑定表判，同一枚操作可能在两处得出不同的结论。读同一份表把这条分叉消掉。
   - **等价性的前提要记住**：它**依赖注册期那两条核对的双向包含**。若将来注册期只剩一侧，
     这句「等价」立刻不成立，而**编译器不会提醒**。
3. **臂必须对**：由绑定 kind 推出这条操作该走哪一臂（3.2 第 4 条）；
   若推出是**效应臂**却没出示 `AuthorizedEffect` → `EffectAuthorizationRequired { op, effect }`。
4. **kind 必须相等**：出示值给出的 kind 必须等于绑定的 kind；不等 → `AuthorizationMismatch`。

**第 3、4 步各管一个方向，谁也不多管**（这是复审揪出来的，初稿把两个方向都记在第 3 步上）：

| 绑定 kind | 出示 | 谁拦 | 为什么 |
|---|---|---|---|
| 落在 `for_effect` 像里 | 非效应臂 | **第 3 步** `EffectAuthorizationRequired` | 有外部效应却没走强制点 (2) |
| 不在像里 | 效应臂 | **第 4 步** `AuthorizationMismatch` | 效应臂给出的 kind 必在像里（`AuthorizedEffect::new` 保证），而绑定的不在，**两者必不相等**——可判 |

第二行**到不了**第 3 步那枚变体：它的 `effect` 字段是「绑定 kind 推出的那条效应」，
而绑定 kind 不在像里时**推不出效应**，那个字段**没有值可填**。故初稿把它记在第 3 步是错的。

**次序只决定同时犯两种错时报哪一种 `Err`，不决定放行与否**——四步全过才可能返回 `Ok`
（与 P3A `authorize` 的写法同形，见 `crates/continuum-capability/src/registry.rs:63-78` 的说明）。
四步都在**调用实现之前**，故任一步拒，实现都没被调用——这一条要有照片（第 9 节）。

第 3、4 步是**两条不同的核对**，不能并成一条：第 3 步管的是「有没有走强制点 (2)」，
第 4 步管的是「拿的是不是那一枚动作」。若并成一条，用 `<正确的 kind> + 错误的臂` 就能过，
而那条路径恰恰是「绕开强制点 (2) 做副作用」——**这正是 fail-open 的那一侧**。

---

# 6. 失败面与审计

## 6.1 本子项目的错误类型

新增一个连接器侧的错误类型（名字暂定 `ConnectorError`），**每个变体都要有真实产生方**
——没有产生方的变体不建：

| 变体 | 产生方 | 是哪一种失败 |
|---|---|---|
| `UnknownConnector { connector }` | 入口第 1 步 | 该服务没有任何已注册的连接器 |
| `UndeclaredOperation { connector, op }` | 入口第 2 步 | **操作不在声明集内**——§125 的那条注释第一次有了强制 |
| `EffectAuthorizationRequired { op, effect }` | 入口第 3 步 | 绑定落在 `for_effect` 的像里（有外部效应），却没走效应臂——**绕开强制点 (2) 的那条路** |
| `AuthorizationMismatch { op, bound, presented }` | 入口第 4 步 | 出示的那枚能力的 kind 与绑定的 kind 不是同一枚 |
| `Credentials(#[from] SecretsError)` | **转出**（不是「产生」）：`issue` 与 `material` 各转出它们**自己**的那几个内层变体，见下 | 能力已失效（`Capability`）、源不覆盖该作用域、源读不出来、凭据已到期 |
| `Provider(#[from] ProviderError)` | 实现自身 | **后端错误**（§124 的接口本就以 `ProviderError` 报错） |
| `UnboundOperation { connector, op }` | **注册期**（第 3.2 节第 2 条） | **声明了却没绑**——绑定不完备 |
| `UndeclaredBoundOperation { connector, op }` | **注册期**（第 3.2 节第 2 条） | **绑了却没声明**——另一侧；只建一侧不算钉住 |
| `DuplicateKindBinding { connector, kind }` | **注册期**（第 3.2 节第 3 条） | 一个连接器内两条操作绑了同一枚 `CapabilityKind`（「一一」） |
| `OperationServiceMismatch { connector, op }` | **注册期**（第 3.2 节第 5 条） | 操作串的服务半边不等于本连接器的 id——该操作永不可达 |

四处取舍，各给理由：

- **上面四条注册期变体不是留桩**：它们在 `ConnectorRegistry::register`（第 3.2.1 节）里都有真实
  产生方（各有照片，见第 9 节）。**调用期**不存在「绑定缺失」这条路径——完备性在构造期就强制了，
  故调用期不建对应的变体。这一条要说准：不是「这个形状不存在」，而是「它只能在注册期出现」。
- **`Credentials` 是「转出时保留内层变体」，不是「产生」**：入口不新造凭据侧的失败，
  只把 `issue` / `material` 给出的 `SecretsError` **原样**带出去（`#[from]`，不重编成新变体、
  不吞成一句笼统的话）。`SecretsError` 的八个变体各有产生方与照片
  （`crates/continuum-secrets/src/error.rs`），B 再抄一层就会与它漂移——与 P3A
  `CapabilityError::Persist` 的取舍同形。**「是哪一种失败」永远由内层变体给出。**
- **八个内层变体里，有两个经本入口不可达——写进设计，不是留给读者推**（裁决文件 B3）：
  - **`Superseded`**（§103 的轮换）：它要求「签发之后、取料之前**发生过一次 `rotate`**」。
    入口的一次调用里**没有可以插入 `rotate` 的位置**——`issue` 与 `material` 是同一个调用内的
    前后两步，而 `rotate` 要 `&mut SecretsRuntime`、在入口之外。故经入口签出的那张凭据
    不可能被取代；
  - **`ForeignCredential`**：它要求凭据是**另一个运行时**签的。而入口的凭据是**本次调用自己
    刚签出的**，故不可能是外来的。
  **这两条仍然保留在 `Credentials` 的类型里**（转出不展开的那条决定即含此意）：它们在内层
  各有用例与照片，只是**本入口这条路走不到**——若将来入口变成持有凭据的形态（例如跨调用复用），
  两条会立刻可达，那时无需改类型。**故本子项目的测试策略里，这两条没有照片，
  且原因是「经入口不可达」而不是「没做够」**（第 6.3、第 9 节）。
- **`Provider` 独立于 `Credentials`**：后端不可用与凭据拿不到是两回事，并成一个变体会让
  「凭据被拒」这条路径上的消息说一件不真的事（P3A 为同类情形专门分过 `ForeignCredential`
  与 `Superseded`，`continuum-secrets/src/error.rs:90-99`）。

## 6.2 审计：**B 不新写审计行**

§313（`docs/spec/05-normative.md:2132`）的必录清单里，与本子项目相关的是两项：
**external effects** 与 **capability grants**。两项都已有产生方：

- `AuditKind::ExternalEffects`：**两个产生点，各写一行**——
  `crates/continuum-effect/src/journal.rs:59` 的 `record_planned` 写**计划**时刻那一行
  （它的 `audit` 调用在 `:62`，`audit` 函数本身在 `:34`），
  同文件 `:117` 的 `advance` **在每一次状态迁移都写一行**（其 `audit` 调用在 `:129`）；
  另有 workspace 门控的 `discard`（`crates/continuum-workspace/src/gate.rs:157` 起，其
  `append_audit` 在 `:168`）。
- `AuditKind::CapabilityGrants`：`continuum-capability` 的 `authorize`（`crates/continuum-capability/src/registry.rs` 的 `append_audit`，P3A 为该变体建的第一个产生方）。

B 再写一条，就是同一个事实两个产生点。**决定 B-5：B 的入口用那个值、不重记它。**
照片是「不写」的那一侧：一次成功的连接器调用前后，`audit` 表行数不变（第 9 节）。

### 6.2.1 「经 B 驱动一次副作用，§313 的 external effects 由谁写」——**驱动**

这条此前只有「两项都已有产生方」这句，**没有点名**，故复审要求把先后次序写明。**答案是驱动**，
按既有的机构走，B 一个字节都不写：

1. 驱动**逐条效应**铸一枚 `AuthorizedEffect`（`crates/continuum-runtime/src/task_cmd.rs:445-470`），
   **同期**已经按既有的效应记录机构写下 `PLANNED → AUTHORIZED → EXECUTING`
   （`crates/continuum-effect/src/journal.rs` 的 `record_planned` 与 `advance`），
   并且**提交在执行之前**（§268「执行前写入」；F 的设计 §5 第 5 步是同一条）；
2. 副作用真的发生之后，驱动把该效应推进到终态（`task_cmd.rs:269-273` 定 `Committed` / `Failed`，
   `:481` 的 `finish_declared_effects` 逐条 `advance`）——**`external effects` 的「实际发生」那一行
   就写在这里**；
3. **B 的入口在这两步之间被调用**：它收下那枚 `AuthorizedEffect`（第 5.1 节），
   做凭据签发与调用，**不写任何效应行、也不写任何审计行**（决定 B-5）。

故「谁写 §313 的 external effects」= **驱动**，答案具名、且不依赖 B 是否被接上。
**这条路径今天不存在**（B 的入口没有生产调用方，第 11 节第 **13b** 条）——上面写的是**接线之后**
的分工，不是今天的运行状况，**不要读成今天的照片**。

**本节初稿曾断言「`external effects` 记的是计划那一刻，不是实际发生」——那句是假的，已撤。**
实际是：驱动在命令跑完之后才推进终态（`crates/continuum-runtime/src/task_cmd.rs:269-273` 定
`Committed` / `Failed`，`:481` 的 `finish_declared_effects` 逐条 `advance`），
而 `advance` 每次迁移写一行，故**终态那一行就是「实际发生」**。§313 的 external effects
因此**两侧都有记录**。（本设计初稿引的三处行号也全错：`journal.rs:23` 与 `gate.rs:45` 都是**文档注释**，
不是代码。）**本条订正由复审推翻，来历留在此处。**

## 6.3 哪些失败路径有照片、哪些没有

**有照片**：操作未声明、效应臂走错、kind 不匹配、绑定不完备（两侧）、绑定不一一、
服务半边不符、**「绑错了照样注册成功」**（第 3.4 节照片 2）、能力已失效、源不覆盖作用域、
后端错误、未注册的连接器、以及「拒时不调用实现」与「成功时不写审计行」两条。

**没有照片，且写不出照片的**：

1. **真实的外部服务**——本仓零个连接器实现，故「连接器真的把一次推送发出去了」没有照片。
   第 9 节的假连接器只能钉**门**，钉不了门后的东西。
2. **「作者把某条操作绑对了没有」**——**这是没有照片的那一条，而且不是「本子项目没做够」**：
   规范没有「一条操作必须绑到哪一枚 kind」的判据，本设计也没有发明，故不存在「绑对了」这一状态可照。
   能照的只有反面（第 3.4 节照片 2：绑到 `Github(CreatePr)` 的 `GitHub.merge` **注册成功**）。
   收件人见第 11 节第 16 条。
3. **§125 那五个操作的「相配 kind」**——本子项目不补臂，故不存在「补了之后」的状态可照
   （第 3.4 节照片 3）。**这与第 2 条是两回事**：补了臂，「相配的那一枚」存在了，
   但「配得对不对」的判据**仍然不存在**——补臂让正确的绑法**可写**，不让错误的绑法**不可写**。
4. **`Superseded` 与 `ForeignCredential` 经本入口不可达**（第 6.1 节的 B3 段）：
   前者要「签发后、取料前发生过一次 `rotate`」，后者要凭据属另一个运行时，**两条在入口的一次
   调用里都构造不出来**，故没有照片。**原因是「走不到」，不是「没做够」**——两条在内层
   （`continuum-secrets`）各有用例，那不属于本子项目（第 9 节的轮换行同此）。
5. **凭据不越权在真实调用上的表现**——「作用域没被放宽」在类型上有照片（4.2 节三条），
   但「拿着这枚令牌真的调不动别的仓库」要真实服务才照得出来。
6. **漂移**：本子项目**没有**「真实连接器」这个产生方，故一切「按操作细分」的断言都只在
   假连接器的操作集上为真。这条留给后来者，不要读成对真实连接器的保证。

---

# 7. crate 划分与依赖边

## 7.1 新 crate：`continuum-connector`

**为什么不放进 `continuum-provider`**：那一 crate 的定位写在它的模块文档里——
「外围中立接口。本 crate 只定义 trait，不含实现，也不引用任何实现类型（§346）」
（`crates/continuum-provider/src/lib.rs:1`），它今天只依赖 `continuum-core`。
B 的入口需要 `continuum-capability`（`AuthorizedEffect`）、`continuum-secrets`（`issue` / `material`
/ `SecretMaterial`）、`continuum-effect`（`EffectType`）与 `continuum-provider` 本身。
把这几条塞进 provider，等于让**「外围 / 中立」那一侧的 crate**（`docs/02-工程.md` §1.1 的绑定范围表，
**不是**第 5 层的「边界层」——两个词共用一个「边界」但指两件事，这里说的是前者）
依赖核心层的实现——§346 直接违反。**`continuum-provider` 的层归属是资源层**（§4.1 的组件表），
见第 7.3 节那条边的算法；本段说的「外围 / 中立」与层无关。

**为什么不塞进 `continuum-capability` 或 `continuum-secrets`**：crate 边界按层走（P3A 设计 §6 的判据）。
把连接器塞进 capability 会让资源层的 crate 承载连接器的装配与实现细节；塞进 secrets
会让密钥运行时承载一个与它无关的组件。

## 7.2 层归属：**边界层**（**已由用户拍板确认，规范已订正**）

**订正（2026-10-05）**：本节初稿报的是一处**冲突**——`docs/02-工程.md` §4.1 的组件表把
「Connector 与权限细分（中立）」列在**资源层工程**下，而 §4.2 把强制点 (2) 判在**第 5 层**
（原文：「本层的 Tool Registry（工具调用前）、第 5 层的执行点（实际副作用前）、
第 5 层的密钥运行时（凭据签发）」）。**该冲突已裁**：用户取「Connector 归边界层」，
并已订正规范——`docs/02-工程.md` §4.1 的组件表里**已无** Connector 一行（它移入 §5.1 的边界层表），
§5.3 的层内依赖图已补上一行
「`Connector ← 密钥运行时 + 第 4 层 Capability Token`」。
**裁决全文见 `docs/superpowers/specs/2026-10-05-p3-bcdf-set-decisions.md` 第一节第 3 条。**
**故本条不再是「待裁决的冲突」，本节第 0 节第 2 条随之标为已闭。**

**决定 B-6：crate 落在边界层。**

**存活的理由（一条）**：**§4.2 的强制点归属**——B 要落的正是强制点 (2) 的连接器侧那半个
（P3A 设计 §4.2 明写「连接器侧留待 B」），而 §4.2 把它判在**第 5 层**。

**已倒掉的理由（第二条，留其来历）**：本节初稿还写了第二条，说 `continuum-connector` 落在资源层
会构成层环。**那条不成立，且不成立的原因是方向读反了。** 按 §9.1 的方向约定
（`A → B` 读作「B 依赖 A」，佐证见第 0 节第 3 条），`continuum-secrets` 依赖 `continuum-capability`
是 **`4 → 5`**；而 `continuum-connector` 落在资源层时依赖密钥运行时，也是 **`4 → 5`**。
**两条边同向，无环。** 我初稿按「箭尾依赖箭头」的直觉读，才读出了那个不存在的环。

**这条纠正把问题整个倒了过来**：按正确的方向约定，**两种落法在层图上要的边是同一条
（`4 → 5`）**，而那条边 §9.1 里本来就没有、且 P3A 已经需要它（第 0 节第 3 条）。
故「放资源层还是边界层」在层图上**不产生差别**——差别只剩 §4.2 的强制点归属那一条。

**据实说明这条决定的强度**：`dependency_direction.rs` 断言的是 **crate 级直接边**，
两种落法在 crate 图上都无环（`capability` 不依赖 `connector`，也不依赖 `secrets`）。
故这不是构建约束，是**归属与文档的一致性**问题；若把 §9.1 读成只约束 crate 图，
两种落法都可行，此时仍以 §4.2 的强制点归属为准取边界层。

**收件人：无（已闭）。** 用户已按这条存活的理由拍板，规范侧的两处订正（§4.1 移行、§5.3 补行）
已由协调者落地——**故本决定只由 §4.2 的强制点归属一条支撑，第二条理由作废这件事也已由裁决记录
（`…set-decisions.md` 第一节第 3 条的「注」）独立确认**，两份文书对同一处的记载一致。

## 7.3 依赖边（只登记实际用到的）

**本节 `→` 的方向与 §9.1 的层图相反，读之前先看这一句**：这里是 crate 图，
`A → B` 读作 **「A 依赖 B」**（与 `dependency_direction.rs` 的 `ALLOWED` 表同一读法）；
§9.1 的层图则是 `A → B` = 「B 依赖 A」（第 0 节第 3 条给了佐证）。
**两个图的两套箭头方向，本项目已经在这上面栽过一次**（本设计初稿，第 7.2 节），故两处都写明。

```
continuum-connector → continuum-core         （ConnectorId / ConnectorOp / ConnectorDescriptor / ProviderError）
continuum-connector → continuum-effect       （EffectType：它出现在本 crate 的公开错误类型里——
                                               `EffectAuthorizationRequired { effect }`；取值本身由下面那条边给出）
continuum-connector → continuum-capability   （AuthorizedEffect / Capability / CapabilityKind 及新加的 effect 逆）
continuum-connector → continuum-provider     （§124 的 Connector trait）
continuum-connector → continuum-secrets      （issue / material / SecretMaterial）——强制点 (3) 的第一条真消费边
continuum-runtime   → continuum-connector    （装配处；**由用它的那个 task 登记**）
continuum-runtime   → continuum-secrets      （**由把密钥运行时接上的那个 task 登记**——见 p3a-followups 第三节末段；
                                               它是不是真边取决于第 4.1 节那条「谁构造 SecretsRuntime」——本设计取
                                               「驱动构造」，故它是真边）
```

`connector` 与 `secrets` 两条到 runtime 的边**不在本子项目的第一个 task 一次声明齐**：
本仓口径是「零使用的边即假边」，`ALLOWED` 必须等于实际依赖，P2b 为此删过两条边。
这与 P3A 设计 §6「边由需要它的那个 task 增量加上」是同一条判据。

**`continuum-connector → continuum-provider` 这一条要在层图上算一笔**（复审指出）：
`continuum-provider` 按 §4.1 的组件表属**资源层**（那表把「Provider Adapter（中立）」
「ToolProvider（中立）」列在资源层工程下），它不是一个跨层的外围 crate。故这条边与
`→ continuum-capability` 同类，都是 **`资源层 (4) → 边界层 (5)`**——**不是新增的一类缺口，
是第 0 节第 3 条那条边上的第二个实例**。C 的设计把 Provider Adapter 与注册表放在资源层，
与 §4.1 一致；两文在 `continuum-provider` 的层归属上并不冲突，冲突只在 **Connector 自己**放哪
（第 7.2 节）。

**主依赖不新增** `continuum-connector → continuum-events` / `→ continuum-persist`：本子项目不写审计、
不建表（第 6.2 节）。将来若 B 有了落库需求再按实际使用加。

**但有一条 dev 边必须有：`continuum-connector → continuum-persist`（dev-dependency）**（裁决文件 B4）。
理由是本设计自己那张照片：§9 的「**不写审计**」要断言「一次成功的连接器调用前后 `audit_log` 行数不变」，
而**裸查行数**要能开库、能读表——那是 `continuum-persist` 的 `Db` / `Tx`。**这条边与上面那句不矛盾**：
上面说的是**主依赖**（生产代码不落库），本条说的是**测试夹具**（照片要读库）。
**`ALLOWED` 表必须覆盖它**——`dependency_direction.rs` 用 `cargo tree --edges all`，
`all` 同时含 normal / build / **dev**（该文件自己的注释即如此写），故 dev 边不登记会被抓；
这与 C 的 `persist` dev 边是同一条判据。
**两句都不写清楚，读者会以为「不建表」与「裸查 `audit_log`」相抵——那正是 `plan-b` 报的这一处。**

---

# 8. 补图：**本节已作废**（原提议补进 §4.3，现归 §5.3）

**本节初稿提议把 Connector 补进 `docs/02-工程.md` §4.3 的层内依赖图，理由是「图漏了它」。
那条提议建立在「Connector 属资源层」这个前提上，而该前提已被推翻——用户裁 Connector 归边界层
（第 7.2 节、裁决记录第一节第 3 条），故 §4.3（资源层的层内图）**本来就不该有它**，
「漏了它」这个判断随之不成立。**本节作废，不改 §4.3。**

**实际落地的是 §5.3**（边界层的层内依赖图），协调者已补上一行：

```
Connector ← 密钥运行时 + 第 4 层 Capability Token
```

`docs/02-工程.md:325`（补于 2026-10-05）。**这行与第 7.3 节的 crate 边一致**：
`continuum-connector → continuum-secrets` 与 `→ continuum-capability` 两条都在那里登记。

**留这一节而不是删掉它**，是因为「图里没有它 → 必须补」这个推理本身看着很顺，
而它错在**默认了组件所在的层**：同一个组件放在不同的层，要补的是**不同的那张图**。
后来者若在本节看到「已作废」三字而不看理由，就会重走一遍。

**收件人：无（已闭）。**

---

# 9. 测试策略：逐项有照片，没有照片的说出为什么

| 验什么 | 怎么验 |
|---|---|
| 操作未声明 | 假连接器声明 `{A}`，请求 `B` → `UndeclaredOperation`，**且断言实现未被调用**（假实现记录调用次数） |
| 效应臂走错 | 绑定 kind 落在像里，却出示非效应臂（且那枚能力的 kind **就是**绑定的那一枚，故第 4 步拦不住）→ `EffectAuthorizationRequired{ effect }`，**且实现未被调用**。**反过来的那一版**（绑定 kind 不在像里却出示 `AuthorizedEffect`）走的是**第 4 步** `AuthorizationMismatch`——那一侧到不了本变体，因为本条没有可填的 `effect` 值（第 5.3 节） |
| kind 不匹配 | 出示的能力 kind 与绑定的 kind 不同 → `AuthorizationMismatch`，**且实现未被调用** |
| 两条臂的强度 | 效应臂（`Email(Send)`）与非效应臂各走通一次——**非效应臂那条今天只能用已有臂拼**（如 `Filesystem(Read)` 绑到一个假连接器的读操作上），它证明的是**臂的机制**，不是 §125 的读操作已经被表达（第 3.4 节） |
| 未注册的连接器 | 请求一个没有注册的服务 → `UnknownConnector` |
| 绑定的双向覆盖 | 声明了没绑、绑了没声明，各一条，各断言**是哪一种** `Err` |
| 绑定的一一 | 两个操作绑同一枚 `CapabilityKind` → 拒 |
| 服务半边相符 | id 为 `GitHub` 的连接器声明 `Email.send` → 注册期拒 `OperationServiceMismatch`；并带一条「同 id 的正例注册成功」的对照臂。**大小写那一侧也要有**：id `GitHub` + 操作 `github.push_branch` → 拒（判据是逐字比较） |
| **「绑错了照样注册」**（第 3.4 节照片 2；**这一条是本设计初稿写反的那一侧**） | 假连接器 id `GitHub` 声明 `GitHub.merge`，绑定写 `Github(CreatePr)` → 断言 `register` 返回 **`Ok`**。初稿在这里写的是「注册期拒」，那是假的 |
| 效应臂只收 `AuthorizedEffect` | trybuild 样例：在效应臂的位置传 `Capability` 不编译；**判据是编译失败** |
| 效应是推出来的，不是声明的 | 用例断言连接器的绑定类型里**没有** `EffectType` 这个字段可填：给一条绑定指定一个与 `for_effect` 逆对不上的效应**写不出来**；并逐项断言 `effect(for_effect(e)) == Some(e)`（P3A 已有 `for_effect` 的六条照片，本处补的是逆） |
| 凭据作用域不越能力 | 两条臂各一条：断言 `credential.scope()` 等于**那一次出示的那枚能力**的 scope；并带一条「能力的 scope 是什么、凭据就是什么」的对照 |
| 能力已失效 | 过期能力经**两条臂各一次** → `Credentials(SecretsError::Capability(CapabilityError::Expired))`（**断言是这一种**，不是笼统的 `Err`） |
| 源不覆盖作用域 | 文件源里没有该作用域 → `Credentials(ScopeNotCovered { .. })` |
| 轮换 | **本子项目没有照片，且写不出**：`Superseded` 要求「签发之后、取料之前发生过一次 `rotate`」，而**入口的一次调用里没有可以插入 `rotate` 的位置**（第 6.1 节的 B3 段），故 `rotate` 之后用旧凭据取料这条**在 B 的入口上构造不出来**。§103 的四类事件逐项那一组**是 `continuum-secrets` 既有的照片**（`crates/continuum-secrets/tests/issue.rs::every_rotation_event_class_invalidates_old_credentials`，由 `error.rs:93-94` 引出）——**那是那一层的证据，不是本子项目的**（§10 末句刚说过「不许用别人的用例代表自己」）。本子项目能说的只有：`Credentials` 转出时**保留内层变体**（第 6.1 节），故一旦可达，`Superseded` 会原样出去——**这是一句关于类型的断言，不是照片** |
| 后端错误 | 假实现返回 `ProviderError::Unavailable` → `Provider(Unavailable(_))` |
| **不写审计** | 一次成功调用前后 `audit` 表行数不变（裸查 `kind` 列） |
| 凭据材料不进返回值 | 假实现把收到的材料原样回显 → B 的返回值里**不含**它（适配器方案下这条天然成立，仍需一条照片：适配器不把材料放进 `input`） |

**本项目既有的三条纪律一并适用**：变异须在全量 `--no-fail-fast` 下得出否定结论；
凡注释写绝对措辞（唯一 / 一概 / 整个 / 从不）须有对应用例；失败路径须断言是哪一种 `Err`。

**「不写审计」那一条特别说明**：它是**否定**的照片，容易被漏——本项目的一贯判据是
「缺的那一侧往往是 fail-open 的那一侧」。

---

# 10. 完成判据

§4.4 的第三条：

```
工具调用前校验 Capability，且 Capability 不可与裸字符串互换
```

其「工具调用前」那半条属**子项目 F**（共享面 §一），本子项目不认领。
本子项目自己的完成判据是两条：

```
连接器做副作用必须收下 AuthorizedEffect（P3A 设计 §10 第 7 条）
凭据由密钥运行时按能力逐枚签发，且其第一个真消费方在生产路径上（共享面 §四.3）
```

两条各自要自己的照片：前者的照片是「效应型操作不传 `AuthorizedEffect` 就调不到实现」，
它由**两部分**拼成，缺一不可——

- **编译期那一半**：`Effect` 变体的字段类型是 `AuthorizedEffect`（trybuild 样例）；
- **运行期那一半**：效应型操作出示非效应臂 → `EffectAuthorizationRequired`（第 5.3 节第 3 步），
  连 `UndeclaredOperation` / `AuthorizationMismatch` 三条**拒时断言实现未被调用**。
  **这一半不能由编译期那一半代表**：`ConnectorAuthorization::Capability` 是公开可达的
  （第 5.1 节），故「裸能力驱动效应型操作」在类型上写得出，只能运行期拦。

后者的照片是第 9 节凭据那一整组。**不许用强制点 (1) 的用例代表它们**
（P3A 设计 §9 的同一条判据）。

---

# 11. 遗留与未决项

| # | 内容 | 收件人 |
|---|---|---|
| 1 | ~~共享面把 §124 / §125 的出处记错~~——**已闭**：共享面本就未写文件出处；协调者已补上真实位置（§124 `docs/spec/02-positioning.md:1803`、§125 `:1828`）。本条留格是为了让「本设计初稿那句假归因」的来历可查（见第 0 节第 1 条） | 无（已闭） |
| 2 | ~~`Connector::invoke` 的签名放不下凭据，故本设计用「逐次调用的适配器」绕开（决定 B-4）——**是否算动了 §124 的冻结边界，待协调者裁决**~~——**已闭：裁决记录已把该形状当作事实引用**。**判据**：`docs/superpowers/specs/2026-10-05-p3-bcdf-set-decisions.md:34`（第一节第 2 条）写「**B 不受影响（连接器侧的凭据交付走的是它自己的「逐次调用的适配器」，不经 §316）**」——它不是在**提议**这条形状，而是在**陈述**它，故「适配器方案算不算越界」这一问已被那句话本身答掉。协调者 2026-10-05 复核确认。**来历**：本条初稿把它列为**待裁**，收件人写的是「协调者（复核那句引用是否即本条确认）」；现由那次引用确认，故本行改标已闭，**待裁的措辞留在删除线里**。**§7.3 与 §4.5 的正文不动**——决定 B-4 本身没变，变的只是它的状态 | 无（已闭） |
| 3 | **§125 的五个操作（`read_repo` / `create_issue` / `merge` / `Email.read` / `Email.draft`）没有语义相配的 `CapabilityKind`**——表里十二枚无一表示它们。**注意它们仍注册得成**：绑定由作者填，`register` 不查语义（第 3.4 节）。补法按 P3A §2.2 的先例往那张封闭枚举加臂；**本子项目不加**（不预先发明，今天零连接器） | 子项目 B 的实现（当有真连接器提出这些操作时，加臂落在 `continuum-capability`） |
| 3b | **连接器读权限的强制点落在哪里，规范未规定**（§125 只规定粒度）。本子项目选了「B 的入口按出示的 kind 把关」（决定 B-3b），与效应臂同一判据。若后续判定读权限该由别处强制，此处要重新处置。**已列入整套终审的规范级缺口**（裁决记录第三节第 1 条，与第 17 条同源） | **规范**（非效应臂的强制点归谁）+ 子项目 B 的实现 |
| 4 | ~~Connector 的层归属~~——**已闭，用户已裁**：归边界层；§4.1 的组件表已移行、§5.3 的层内图已补「`Connector ← 密钥运行时 + 第 4 层 Capability Token`」，§9.1 也已补 `资源层 (4) → 边界层 (5)` 并写明箭头读法（第 0 节第 2、3 条，第 7.2 节，裁决记录第一节第 3 条与第二节） | 无（已闭） |
| 5 | ~~§4.3 的层内依赖图漏了 Connector~~——**已闭，且该提议本身作废**：§4.3 是**资源层**的层内图，Connector 已不属该层，故那里本来就不该有它；实际补的是 §5.3（第 8 节） | 无（已闭） |
| 6 | **`CAPABILITY_LIFETIME_MS` 的数值无规范来源**（P3A §10 第 11 条）。B 是**第一个在生产路径上调用 `issue`** 的地方，故这个常量第一次进入一条会被执行的比较；**它何时真的判出过期未定**（取决于铸出与取料之间的间隔由谁引入，见第 4.3 节）——首次接上时须核这个数是否合理 | 子项目 B 的实现 |
| 7 | **轮换只在下一次取料时可见**，进行中的一次调用不受影响（第 4.4 节）；本阶段没有调用中途复查的机构 | 子项目 B 的实现 / 长期阶段 |
| 8 | ~~`AuditKind::ExternalEffects` 只记「计划」那一刻~~——**已闭，那句是假的**：`record_planned`（`journal.rs:59`）与 `advance`（`:117`，每次迁移一行、终态那行即「实际发生」）两侧都有记录，故 §313 的两侧都满足（第 6.2 节）。B 不重记这一定论不变 | 无（已闭） |
| 9 | **`SecretMaterial` 出到实现之后无回收机构**：材料在调用期间存活，调用结束随适配器析构。本阶段够用，若将来材料需显式清零（§100 的硬件后端语境），此处是替换点 | 长期阶段 |
| 10 | **真实连接器零个**：本子项目全部用例跑在假连接器上，「按操作细分」在真实服务上的表现没有照片（第 6.3 节） | 长期阶段 |
| 11 | **本子项目对 P3A crate 的唯一新增面**：`CapabilityKind` 上补一枚 `for_effect` 的逆（暂名 `effect`，落在 `crates/continuum-capability/src/capability.rs` 紧邻 `for_effect`）。它是那条对应唯一的逆产生点；`for_effect` 是单射不是满射，故逆返回 `Option`。消费方是 B 的入口（第 3.2 节第 4 条） | 子项目 B 的实现 |
| 12 | **两套串的服务半边对不齐**：连接器操作 `GitHub.push_branch` 必须绑到 `Git(Push)`——resource 是 `git`、不是 `github`（第 3.4 节）。今天这不是错（§88 与驱动的铸法都如此），但**连接器说 GitHub、能力说 git** 这条缝在 §125 的服务清单（§124 列了六个服务）与 `CapabilityKind` 的 resource 集之间普遍存在。若后续出现第二个服务也需要同一枚 kind，须重新处置 | 语义层 / 子项目 B 的实现 |
| 13 | **`AuthorizedEffect` 的交付通道在 §316 上不存在——已裁：取 (a)，扩 §316 的请求面。** 今天驱动执行效应只有「跑命令」一条路（`Sandbox::spawn`），而 `ToolProvider::invoke` 收的是 `ToolInvocation`（tool id + input，`crates/continuum-core/src/tool.rs:25-29`），**没有携带授权或凭据的字段**。**用户 2026-10-05 取 (a)**：`invoke` 的**请求侧**要能携带授权/凭据（**改的是请求侧、不是响应侧**，与 `cancel` 那条裁决同一判据）；**C 负责设计那个位置、F 负责传**（裁决记录第一节第 2 条）。<br>**B 不受这条裁决影响，且不必等它** —— 这一句须写明，免得被读成「B 卡在 §316 上」：**B 的凭据交付走的是 §124 上自己那条「逐次调用的适配器」（第 4.5 节），一个字段都不经 §316**。§316 扩请求面解决的是**工具**那条路径（F 传、C 设计）；**连接器**这条路径的凭据通道本设计已经自足。<br>**仍未落地的部分**：B 的入口在生产路径上仍无调用方（本设计给的是入口与边，没有一条路径说明驱动何时调用它），故 §10 的第一条完成判据**在生产路径上仍没有落点**——这一半与 §316 的裁决无关，见第 13b 条 | 无（已裁）；落地由 **C（设计请求面）与 F（传）** 承担 |
| 13b | **B 的入口在生产路径上没有调用方**（与第 13 条是**两件事**：13 是「§316 那条通道窄不窄」，本条是「连接的这条路上有没有人」）。今天驱动执行效应只有跑命令一条路（`Sandbox::spawn`）。**这不是本设计能单独关掉的**：要么驱动在跑命令之外多一条「经连接器执行效应」的路径，要么这条判据的兑现推迟 | **协调者**（派工时决定这条由谁接；裁决记录未涉此条） |
| 14 | ~~`AuthorizedTool` 的消费方，两份前文冲突~~——**已闭**：协调者已于 2026-10-05 15:14 订正 `p3a-followups.md` 第一节第 3 条，两份现在一致（B 消费 `AuthorizedEffect::capability()`，`granted()` 无消费方）。第 0 节第 4 条 | 无（已闭） |
| 15 | **`SecretsRuntime` 由谁构造**：本设计取「驱动装配好传进来」（第 4.1 节），故 `continuum-runtime → continuum-secrets` 是**真边**、由那个 task 登记。若改为连接器自建，那条边不该登记，且要重新算第 7.3 节 | 子项目 B 的实现 |
| 16 | **「一条操作该绑哪一枚 kind」没有判据**（第 3.4 节）：§125 只规定粒度，没规定这条对应；本设计也没发明它。故作者把 `GitHub.merge` 绑到 `Github(CreatePr)` 会**注册成功**，而没有任何东西会红。本子项目保住的只是「不拿到声明的那一枚就走不到实现」，**没保住**「声明的那一枚是对的」。补 kind 臂治不了这条（补臂让正确绑法可写，不让错误绑法不可写）。**已列入整套终审的规范级缺口**（裁决记录 `…set-decisions.md` 第三节第 2 条，原文认可本设计「据实如此写，并**没有**自己发明一条」） | **规范**（是否要定「操作 → kind」的对应，以及定在哪一层）+ 子项目 B 的实现（在本条闭合前，把这条限度写进 `ConnectorRegistry::register` 的文档） |
| 17 | **非效应臂那枚能力没有具名的产生方，故其作用域没有来源**（第 4.2.1 节）：效应臂由驱动铸、作用域取 `spec.target`；非效应臂由调用方铸，§253 没有规定它的作用域从哪来。B 因此只能保证「凭据作用域 == 出示的能力作用域」这条相等，**不能**保证「作用域是对的」。**已列入整套终审的规范级缺口**（裁决记录第三节第 1 条「非效应臂的强制点落在哪里」） | **规范**（非效应的连接器操作，其强制点与作用域由谁给）+ 子项目 B 的实现 |
| 18 | ~~`Expired` 在两条路径上的可达性，两份设计结论相反~~——**已闭：F 选了 (i)**。<br>**两个区间不是一对，各说各的**（本条初稿把两者混成一个「取料」，已改）：**F 那条**是「铸出 → F 的 `authorize`」（F §8.1 的题目即是它）；**B 这条**是「铸出 → B 的 `issue`」（§4.3 那句「第一个在生产路径上调用 `issue`」说的正是它）。<br>**F 给出的依据**（它自己路径上的实况，B 看不到那一半）：`AuthorizedTool` 活到步骤 7 才析构、F 把**整枚**交进 §316 请求类型给适配器、**B 的 `issue` 更在其后**——**这一整段都落在 `invoke` 之内，而 `invoke` 的时长无上界**。故「授权可被铸出而不立即取用」为真 ⇒ F 原稿那条**全称**「`Expired` 经由本路径不可达」**为假**，已收窄为「**F 那次 `authorize` 调用上**不可达」。F 另指出**这不是裁决 2 才引入的**：本波次之前 `task` 也是步骤 4 铸、持有到 `run` 返回。<br>**出处：F 设计 §8.1（`p3f…:697-712`）、§11 的 B↔F 行（`p3f…:821`）。**<br>**对本设计的后果**：**§4.3 那句「第一次进入一条**会被执行**的比较」保持原样、不降格**——它说的正是 B 这条区间的下游那一次，现在成立。(ii) 的降格口径不适用。 | 无（已闭） |
