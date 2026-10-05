# P3 资源层（子项目 B）设计：Connector

**范围**：§124 / §125 的落地。本子项目是 P3 分解（A–E）中的 **B**，次序 A → B → C → D → E。
A（Capability）已交付，B 是它两处「建了但无生产调用方」的兑现方之一：

- **强制点 (2) 的连接器侧**——`AuthorizedEffect` 今天**只有构造、无消费方**（`docs/superpowers/p3a-followups.md` 第一节的表），B 是它的消费方；
- **`continuum-secrets`**——今天连依赖边都没有（同上），B 是它的第一个真消费方。

本子项目**只写设计，不落实现代码**（共享面 §七）。

---

# 0. 待裁决的抵触点，与两处已闭的错记

写设计前核过共享面与规范原文。按共享面的约定「本文里的判断若与你（用户）的决定冲突，以本文为准并回报」，
凡与共享面或规范对不上的**一律留在此处并回报**，**不在正文里偷偷按我认为对的那一版写**。

第 2、3 条是**待用户裁决**的规范级归属问题（协调者已接手，本子项目不改动它们）；
第 4 条是两份前文之间的收件人冲突，收件人是协调者 / 用户。
第 1 条是我自己的**假归因**，已由协调者核出并订正——留格记其来历，见下。

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

2. **§4.1 把 Connector 列在资源层，但按 §9.1 的无环约束，B 的 crate 应落在边界层。** 见第 7 节，
   那里给出理由与被否掉的替代。

3. **§9.1 的层间依赖图缺 `资源层 (4) → 边界层 (5)` 这条边，而 P3A 已经需要它。**
   `continuum-secrets` 是边界层（5），它依赖 `continuum-capability`（资源层 4），
   依赖边是 `["continuum-secrets", &["continuum-capability"]]`
   （`crates/continuum-runtime/tests/dependency_direction.rs:103`）；
   按 §9.1 的方向约定，这条在层图上写作 **`资源层 (4) → 边界层 (5)`**，
   而 §9.1 的图（`docs/02-工程.md:559-579`）里没有它。**这是 P3A 留下的既成事实，不是 B 引入的**，
   但在 B 这里会再叠一条同类边，故一并报出。**收件人：协调者 / 规范**。

   **§9.1 的方向约定（本节初稿在这里读反过，把来历留在下）**：`A → B` 读作 **「B 依赖 A」**，
   即箭头由**被依赖者**指向**依赖者**。两处独立佐证：

   - §9.2（`docs/02-工程.md:581-583`）说入度为零的组件「不依赖任何其他组件，是依赖图的**源点**」
     ——只有「入度 = 被依赖数」才推得出「源点」；若 `A → B` 读作「A 依赖 B」，入度为零的会是汇点。
   - 代码：`continuum-policy`（边界层 5）依赖 `continuum-effect`（执行层 3）
     （`dependency_direction.rs:79-81`），而 §9.1 画的正是 `执行层 (3) → 边界层 (5)`。

   **本节初稿写的是「即 5 → 4」——那是我按「箭尾依赖箭头」的直觉读的，与上面两处都不符。**
   留此一句，是为了让后来者看到这条约定确实会读反（与本项目「订正时把错误说法的来历留在原地」
   同一条手法）。**这条读反连带推翻了第 7.2 节的第二个理由**，见那里。

4. **`AuthorizedTool` 的消费方，两份前文指的不是同一个子项目。** `p3a-followups.md` 第一节
   「B 的义务」第 3 条写「**B 怎么消费 `AuthorizedTool` 属 B 的设计**」；共享面 §一（2026-10-05，
   已由用户拍板）把「工具调用路径（`authorize` → `ToolProvider::invoke`）」判给 **F**——
   而 `AuthorizedTool` 正是那条路径的载体。**两份前文互相冲突，本设计不自行选边**：
   按后出的、且经用户拍板的共享面，`AuthorizedTool` 的消费方是 F，B 不收它（第 11 节第 14 条）。
   **收件人：协调者 / 用户**（要么订正 `p3a-followups.md` 第一节第 3 条的收件人，要么说明
   「B 消费」指的是另一件事）。

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
| §125 的非效应操作（`read_repo` / `create_issue` / `merge` / `Email.read` / `Email.draft`） | §125 | **不建，且注册期拒收**——词汇表里没有对应的 `CapabilityKind`（第 3.4 节）。臂的**机制**建，臂的**内容**不预先发明 |

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
完成三项核对（操作 ∈ 声明集；操作已绑定；效应配对，见第 3、5 节），核对不通过即拒且**不调用实现**。
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
   反过来，`CapabilityKind` 本来就装着非效应的动作：`Filesystem(Read)`、`Git(Read)`、
   `Git(WorktreeWrite)`、`Git(CommitLocal)`、`Github(CreatePr)` 都是 §88 / §253 的**照录项**
   而非外部效应类型（P3A 设计 §2.2 明写这一点，`for_effect` 因此不是满射）。
   §125 的读操作与它们是同一类东西，故应落在同一张表里。

**「一一」这条的理由**：§125 的意图是「`GitHub.merge` 能单独不授」。若两个操作绑同一 kind，
授权其一即授权另一，粒度退化到 kind 级——§125 禁的是服务级，这条不到服务级，但**它的意图落空**。
故取一一（fail-closed）。**被否掉的替代**：允许共用——理由是「两个操作本就是同一件事」；
不接受，因为本阶段没有任何真实连接器能证明「本就是同一件事」，而放宽的代价是不可逆的。

## 3.4 今天能表达什么、不能表达什么，代价是什么

| §125 的给例 | 绑到哪一枚 kind | 今天 |
|---|---|---|
| `Email.send` | `Email(Send)` ＝ `for_effect(SendEmail)` | **可注册** |
| `GitHub.push_branch` | `Git(Push)` ＝ `for_effect(PushBranch)` | **可注册** |
| `GitHub.read_repo` | 表里没有这一枚 | **注册不了** |
| `GitHub.create_issue` | 表里没有这一枚 | **注册不了** |
| `GitHub.merge` | 表里没有这一枚 | **注册不了** |
| `Email.read` | 表里没有这一枚 | **注册不了** |
| `Email.draft` | 表里没有这一枚 | **注册不了** |

**是的，这就是我的读法**：§125 的读操作与草稿操作，在本子项目里**一条连接器操作都注册不了**——
不是因为它们「没有强制点」（那条初稿理由站不住，见 3.5），而是因为**词汇表里没有对应的那一枚
`CapabilityKind`**，而绑定必须落到一枚真有的 kind 上。

**代价，逐条说清**：

- **本子项目不补这些臂。** 判据是「不预先发明：没有消费方的取值域不建」——今天零个真实连接器，
  补了也没有东西用它，而且补哪些（`ReadRepo`？`CreateIssue`？`Merge`？`Email.Draft`？）
  只能靠猜 §125 的给例，而 §125 明说那是「例如」。**补臂的消费方是第一个真提出这些操作的连接器。**
- **补臂的形态是既定的**：按 P3A 设计 §2.2 的先例，往 `CapabilityKind` 那张封闭枚举里**加一枚臂**
  ——那是**同一套词汇表的扩展**，不是第二套。故这不是「设计表达不了」，而是「今天还没到补的时候」。
  这一区别是本节与初稿的关键差别：初稿（绑 `EffectType`）把读操作**永久**排除；现在它们只是**尚未**有臂。
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

**照片**：可注册的两条各有一条注册 + 一次成功调用的用例；**「注册不了」这一条也有照片**
——以 `GitHub.merge` 建一条绑定 → 注册期拒，并断言**是哪一种** `Err`（第 9 节）。
**「补一枚臂之后就能注册」这条没有照片**：本子项目不补臂，故不存在「补了之后」的状态可照。
这是这条限制照不出的那一半，写在这里而不是含糊过去。

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

## 4.2 为什么凭据的作用域不可能超出这个操作被授权的范围

三条合起来，且**每一条都落在已有的类型上，不是本子项目新加的**：

1. **`issue` 的签名里没有可以指定作用域的位置**——它只收那枚能力本身
   （`crates/continuum-secrets/src/runtime.rs:176`），凭据的 `scope` 直接取 `cap.scope()`
   （同文件 `:186`）。「拿窄能力去要宽凭据」不是被检查掉，而是**要不到**
   （照片：`crates/continuum-secrets/tests/compile_fail/issue_has_no_scope_parameter.rs`）。
2. **那枚能力的作用域就是这次效应的目标**，且它不是 B 给的：驱动铸能力时取
   `spec.target`（`crates/continuum-runtime/src/task_cmd.rs:456-462`）。
   **B 不得重铸能力**——`mint` 虽公开，B 自己铸一枚就把上面整条链变成自证。
3. **凭据不越期**：`expiry` 取「源声称的」与「能力的」两者中更早的（同文件 `:183`）。

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
而 B 的入口**今天没有生产调用方**（第 11 节第 13 条），故这条间隔由谁引入**在接线之前无从判断**。

**与 F 的相抵，据实记下**：F 的设计从同一个前提（同一步铸出）判 `Expired` 在它的路径上**不可达**；
本节的初稿从同一个前提判它「第一次可观察」。**两条不可能同时为真，而至少有一条无据。**
本设计按上面改后的说法退到可证的一步，并把这条接缝记给 F（第 11 节第 13 条）。

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
) -> Result<Value, ConnectorError>
```

**两臂共用一个入口而不是两个入口**：两个入口就是「谁记得调哪一个」的问题，
而本项目点名的缺陷形态正是「机制建好、路径不经过」。臂与绑定的对应由入口按 3.2 第 4 条
**推出来**再核对，不由调用方选。

「效应臂只能收 `AuthorizedEffect`」有一张编译失败样例钉住（在效应臂的位置传 `Capability` 不编译），
见第 9 节。

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
2. `op ∈ descriptor.operations()`；否则 → `UndeclaredOperation`；
3. **臂必须对**：由绑定 kind 推出这条操作该走哪一臂（3.2 第 4 条）；
   走错 → `EffectAuthorizationRequired { op, effect }`（绑定落在像里，却没出示 `AuthorizedEffect`）；
4. **kind 必须相等**：出示值给出的 kind 必须等于绑定的 kind；不等 → `AuthorizationMismatch`。

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
| `Credentials(#[from] SecretsError)` | `issue` / `material` | 能力已失效、源不覆盖该作用域、凭据已被轮换取代、凭据属另一个运行时 |
| `Provider(#[from] ProviderError)` | 实现自身 | **后端错误**（§124 的接口本就以 `ProviderError` 报错） |
| `UnboundOperation { connector, op }` | **注册期**（第 3.2 节第 2 条） | **声明了却没绑**——绑定不完备 |
| `UndeclaredBoundOperation { connector, op }` | **注册期**（第 3.2 节第 2 条） | **绑了却没声明**——另一侧；只建一侧不算钉住 |
| `DuplicateKindBinding { connector, kind }` | **注册期**（第 3.2 节第 3 条） | 一个连接器内两条操作绑了同一枚 `CapabilityKind`（「一一」） |
| `OperationServiceMismatch { connector, op }` | **注册期**（第 3.2 节第 5 条） | 操作串的服务半边不等于本连接器的 id——该操作永不可达 |

四处取舍，各给理由：

- **上面四条注册期变体不是留桩**：它们在 `ConnectorRegistry::register`（第 3.2.1 节）里都有真实
  产生方（各有照片，见第 9 节）。**调用期**不存在「绑定缺失」这条路径——完备性在构造期就强制了，
  故调用期不建对应的变体。这一条要说准：不是「这个形状不存在」，而是「它只能在注册期出现」。
- **`Credentials` 转出 `SecretsError` 而不展开**：`SecretsError` 的八个变体各有产生方与照片
  （`crates/continuum-secrets/src/error.rs`），B 再抄一层就会与它漂移。
  「是哪一种失败」由内层变体给出——与 P3A `CapabilityError::Persist` 的取舍同形。
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

**本节初稿曾断言「`external effects` 记的是计划那一刻，不是实际发生」——那句是假的，已撤。**
实际是：驱动在命令跑完之后才推进终态（`crates/continuum-runtime/src/task_cmd.rs:269-273` 定
`Committed` / `Failed`，`:481` 的 `finish_declared_effects` 逐条 `advance`），
而 `advance` 每次迁移写一行，故**终态那一行就是「实际发生」**。§313 的 external effects
因此**两侧都有记录**。（本设计初稿引的三处行号也全错：`journal.rs:23` 与 `gate.rs:45` 都是**文档注释**，
不是代码。）**本条订正由复审推翻，来历留在此处。**

## 6.3 哪些失败路径有照片、哪些没有

**有照片**：操作未声明、效应臂走错（含**两个方向**）、kind 不匹配、绑定不完备（两侧）、
绑定不一一、`GitHub.merge` 那类操作在注册期被拒、能力已失效、源不覆盖作用域、
后端错误、未注册的连接器、以及「拒时不调用实现」与「成功时不写审计行」两条。

**没有照片，且写不出照片的**：

1. **真实的外部服务**——本仓零个连接器实现，故「连接器真的把一次推送发出去了」没有照片。
   第 9 节的假连接器只能钉**门**，钉不了门后的东西。
2. **§125 的非效应操作**（`read_repo` / `create_issue` / `merge` / `Email.read` / `Email.draft`）
   ——注册不了，故没有任何照片。第 3.4 节说的是它为什么没有。
   **注意它们的机制那一半是有照片的**：非效应臂本身用一枚**已有**的 kind（如 `Filesystem(Read)`）
   就能走通，故「非效应操作在 B 这里怎么被授权」这条照得出来；照不出来的是
   「§125 那五个具体操作」，因为词汇表里没有它们的位置。
3. **「补一枚 `CapabilityKind` 臂之后 `GitHub.merge` 就能注册」**——本子项目不加臂，
   故不存在「补了之后」的状态。第 3.4 节末段已写明。
4. **凭据不越权在真实调用上的表现**——「作用域没被放宽」在类型上有照片（4.2 节三条），
   但「拿着这枚令牌真的调不动别的仓库」要真实服务才照得出来。
5. **漂移**：本子项目**没有**「真实连接器」这个产生方，故一切「按操作细分」的断言都只在
   假连接器的操作集上为真。这条留给后来者，不要读成对真实连接器的保证。

---

# 7. crate 划分与依赖边

## 7.1 新 crate：`continuum-connector`

**为什么不放进 `continuum-provider`**：那一 crate 的定位写在它的模块文档里——
「外围中立接口。本 crate 只定义 trait，不含实现，也不引用任何实现类型（§346）」
（`crates/continuum-provider/src/lib.rs:1`），它今天只依赖 `continuum-core`。
B 的入口需要 `continuum-capability`（`AuthorizedEffect`）、`continuum-secrets`（`issue` / `material`
/ `SecretMaterial`）、`continuum-effect`（`EffectType`）与 `continuum-provider` 本身。
把这几条塞进 provider，等于让中立边界的 crate 依赖核心层的实现——§346 直接违反。

**为什么不塞进 `continuum-capability` 或 `continuum-secrets`**：crate 边界按层走（P3A 设计 §6 的判据）。
把连接器塞进 capability 会让资源层的 crate 承载连接器的装配与实现细节；塞进 secrets
会让密钥运行时承载一个与它无关的组件。

## 7.2 层归属：**边界层**，与 §4.1 的列法不符

`docs/02-工程.md` §4.1 的组件表把「Connector 与权限细分（中立）」列在**资源层工程**下；
§4.2 又把强制点 (2) 判在**第 5 层**（原文：「本层的 Tool Registry（工具调用前）、第 5 层的执行点
（实际副作用前）、第 5 层的密钥运行时（凭据签发）」）。两处不一致。

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
**收件人：协调者 / 规范**（§4.1 的表要么订正，要么说明「组件在资源层、强制点在第 5 层」是可以并存的；
本决定已交用户裁决）。

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

**不新增** `continuum-connector → continuum-events` / `→ continuum-persist`：本子项目不写审计、
不建表（第 6.2 节）。将来若 B 有了落库需求再按实际使用加。

---

# 8. 补图：§4.3 的层内依赖图

`docs/02-工程.md` §4.3 的层内依赖图里既没有 Connector，也没有它与 Router / Tool Registry 的关系。
**它不在那张图里不是有意的——图漏了它**，而 §4.1 的组件表里列着它。

**补法（本子项目要求补进 §4.3 的两行）**：

```
Connector ← Capability Token（资源层的授权单位；经 AuthorizedEffect 传入）
Connector ← 密钥运行时（跨层：第 5 层 → 本处；见 §9.1 与该节缺的那条边）
```

第二行是**跨层**的，故严格说它不属 §4.3 的「层内依赖」，而属 §9.1 的层间图——
这正是第 0 节第 3 条报出的缺口。两条一起补，或按第 7.2 节把 Connector 归到边界层后
把第一行改写成层间的写法，两种都可以；**但「图里没有它」这一点必须修**。

**收件人：协调者**（§4.3 与 §9.1 的图；本子项目无权改规范文件）。

---

# 9. 测试策略：逐项有照片，没有照片的说出为什么

| 验什么 | 怎么验 |
|---|---|
| 操作未声明 | 假连接器声明 `{A}`，请求 `B` → `UndeclaredOperation`，**且断言实现未被调用**（假实现记录调用次数） |
| 效应臂走错 | 绑定 kinds 落在像里，却出示非效应臂 → `EffectAuthorizationRequired`，**且实现未被调用**。**反过来的那一版也要有**：绑定 kind 不在像里却出示 `AuthorizedEffect`（效应与 kind 对不上）→ 拒 |
| kind 不匹配 | 出示的能力 kind 与绑定的 kind 不同 → `AuthorizationMismatch`，**且实现未被调用** |
| 两条臂的强度 | 效应臂（`Email(Send)`）与非效应臂各走通一次——**非效应臂那条今天只能用已有臂拼**（如 `Filesystem(Read)` 绑到一个假连接器的读操作上），它证明的是**臂的机制**，不是 §125 的读操作已经被表达（第 3.4 节） |
| 未注册的连接器 | 请求一个没有注册的服务 → `UnknownConnector` |
| 绑定的双向覆盖 | 声明了没绑、绑了没声明，各一条，各断言**是哪一种** `Err` |
| 绑定的一一 | 两个操作绑同一枚 `CapabilityKind` → 拒 |
| 服务半边相符 | id 为 `GitHub` 的连接器声明 `Email.send` → 注册期拒 `OperationServiceMismatch`；并带一条「同 id 的正例注册成功」的对照臂。**大小写那一侧也要有**：id `GitHub` + 操作 `github.push_branch` → 拒（判据是逐字比较） |
| 假连接器注册不了 §125 的读操作 | 以 `GitHub.merge` 建一条绑定 → 注册期拒（表里没有这一枚 kind），断言是哪一种 `Err` |
| 效应臂只收 `AuthorizedEffect` | trybuild 样例：在效应臂的位置传 `Capability` 不编译；**判据是编译失败** |
| 效应是推出来的，不是声明的 | 用例断言连接器的绑定类型里**没有** `EffectType` 这个字段可填：给一条绑定指定一个与 `for_effect` 逆对不上的效应**写不出来**；并逐项断言 `effect(for_effect(e)) == Some(e)`（P3A 已有 `for_effect` 的六条照片，本处补的是逆） |
| 凭据作用域不越能力 | 两条臂各一条：断言 `credential.scope()` 等于**那一次出示的那枚能力**的 scope；并带一条「能力的 scope 是什么、凭据就是什么」的对照 |
| 能力已失效 | 过期能力经**两条臂各一次** → `Credentials(SecretsError::Capability(CapabilityError::Expired))`（**断言是这一种**，不是笼统的 `Err`） |
| 源不覆盖作用域 | 文件源里没有该作用域 → `Credentials(ScopeNotCovered { .. })` |
| 轮换 | `rotate` 之后用旧凭据取料 → `Credentials(Superseded { .. })`。**§103 的四类事件逐项那一组是复用既有照片**（`crates/continuum-secrets/tests/issue.rs::every_rotation_event_class_invalidates_old_credentials`，由 `error.rs:93-94` 引出），**不是本子项目的新证据**；本子项目要新增的只是「B 的入口把 `Superseded` 原样转出、不吞成别的变体」那一条。§10 末句刚说过「不许用别人的用例代表自己」，此处照那条办 |
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

两条各自要自己的照片：前者的照片是「效应型操作不传 `AuthorizedEffect` 就调不到实现」
（编译失败样例 + `UndeclaredOperation` / `EffectAuthorizationRequired` / `AuthorizationMismatch`
三条拒时断言实现未被调用）；
后者的照片是第 9 节凭据那一整组。**不许用强制点 (1) 的用例代表它们**
（P3A 设计 §9 的同一条判据）。

---

# 11. 遗留与未决项

| # | 内容 | 收件人 |
|---|---|---|
| 1 | ~~共享面把 §124 / §125 的出处记错~~——**已闭**：共享面本就未写文件出处；协调者已补上真实位置（§124 `docs/spec/02-positioning.md:1803`、§125 `:1828`）。本条留格是为了让「本设计初稿那句假归因」的来历可查（见第 0 节第 1 条） | 无（已闭） |
| 2 | **`Connector::invoke` 的签名放不下凭据**；本设计用「逐次调用的适配器」绕开（决定 B-4），若协调者认为适配器方案仍算动了 §124 的边界，须重新裁决 | 协调者 / 用户 |
| 3 | **§125 的非效应操作在本子项目里注册不了**，原因是**词汇表里没有对应的那一枚 `CapabilityKind`**（`GitHub.read_repo` / `GitHub.create_issue` / `GitHub.merge` / `Email.read` / `Email.draft`）。补法按 P3A §2.2 的先例往那张封闭枚举加臂；**本子项目不加**（不预先发明，今天零连接器） | 子项目 B 的实现（当有真连接器提出这些操作时，加臂落在 `continuum-capability`） |
| 3b | **连接器读权限的强制点落在哪里，规范未规定**（§125 只规定粒度）。本子项目选了「B 的入口按出示的 kind 把关」（决定 B-3b），与效应臂同一判据。若后续判定读权限该由别处强制，此处要重新处置 | 长期阶段 / 语义层 |
| 4 | **Connector 的层归属**：§4.1 列在资源层，§4.2 把强制点 (2) 归第 5 层；§9.1 的层图既没有 Connector，也没有 **`资源层 (4) → 边界层 (5)`** 这条边（P3A 的 secrets→capability 已经需要它）。**已交用户裁决** | 协调者 / 用户 |
| 5 | **§4.3 的层内依赖图漏了 Connector**，补法见第 8 节 | 协调者 |
| 6 | **`CAPABILITY_LIFETIME_MS` 的数值无规范来源**（P3A §10 第 11 条）。B 是**第一个在生产路径上调用 `issue`** 的地方，故这个常量第一次进入一条会被执行的比较；**它何时真的判出过期未定**（取决于铸出与取料之间的间隔由谁引入，见第 4.3 节）——首次接上时须核这个数是否合理 | 子项目 B 的实现 |
| 7 | **轮换只在下一次取料时可见**，进行中的一次调用不受影响（第 4.4 节）；本阶段没有调用中途复查的机构 | 子项目 B 的实现 / 长期阶段 |
| 8 | ~~`AuditKind::ExternalEffects` 只记「计划」那一刻~~——**已闭，那句是假的**：`record_planned`（`journal.rs:59`）与 `advance`（`:117`，每次迁移一行、终态那行即「实际发生」）两侧都有记录，故 §313 的两侧都满足（第 6.2 节）。B 不重记这一定论不变 | 无（已闭） |
| 9 | **`SecretMaterial` 出到实现之后无回收机构**：材料在调用期间存活，调用结束随适配器析构。本阶段够用，若将来材料需显式清零（§100 的硬件后端语境），此处是替换点 | 长期阶段 |
| 10 | **真实连接器零个**：本子项目全部用例跑在假连接器上，「按操作细分」在真实服务上的表现没有照片（第 6.3 节） | 长期阶段 |
| 11 | **本子项目对 P3A crate 的唯一新增面**：`CapabilityKind` 上补一枚 `for_effect` 的逆（暂名 `effect`，落在 `crates/continuum-capability/src/capability.rs` 紧邻 `for_effect`）。它是那条对应唯一的逆产生点；`for_effect` 是单射不是满射，故逆返回 `Option`。消费方是 B 的入口（第 3.2 节第 4 条） | 子项目 B 的实现 |
| 12 | **两套串的服务半边对不齐**：连接器操作 `GitHub.push_branch` 必须绑到 `Git(Push)`——resource 是 `git`、不是 `github`（第 3.4 节）。今天这不是错（§88 与驱动的铸法都如此），但**连接器说 GitHub、能力说 git** 这条缝在 §125 的服务清单（§124 列了六个服务）与 `CapabilityKind` 的 resource 集之间普遍存在。若后续出现第二个服务也需要同一枚 kind，须重新处置 | 语义层 / 子项目 B 的实现 |
| 13 | **B 的入口没有生产调用方，`AuthorizedEffect` 的交付通道因此悬空。** 本设计给了入口（第 5.1 节）与边（第 7.3 节），但**没有一条路径说明驱动何时、以什么调用它**——今天驱动执行效应只有「跑命令」一条路（`Sandbox::spawn`），而 §316 上不存在把 `AuthorizedEffect` 交给连接器的通道（F 的设计 `p3f` 记了这条，收件人写的正是「C 与 B 共同处置」）。**故 §10 的第一条完成判据在生产路径上没有落点**，而共享面 §八 把「B 是 `continuum-secrets` 的兑现处」记在本子项目名下。**这不是本设计能单独关掉的**：要么驱动在跑命令之外多一条「经连接器执行效应」的路径，要么这条判据的兑现推迟 | **F / B 共同处置** + 协调者 |
| 14 | **`AuthorizedTool` 的消费方，两份前文冲突**（第 0 节第 4 条）：`p3a-followups.md` 第一节第 3 条判给 B，共享面 §一判给 F。**本设计按后者不消费它**，理由是 `AuthorizedTool` 是「工具调用路径」的载体，而那条路径经用户拍板归 F。**若判定应归 B，第 5.1 节的入口要重新处置**（B 目前收 `ConnectorAuthorization`，不收 `AuthorizedTool`） | 协调者 / 用户 |
| 15 | **`SecretsRuntime` 由谁构造**：本设计取「驱动装配好传进来」（第 4.1 节），故 `continuum-runtime → continuum-secrets` 是**真边**、由那个 task 登记。若改为连接器自建，那条边不该登记，且要重新算第 7.3 节 | 子项目 B 的实现 |
