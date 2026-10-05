# P3 资源层（子项目 B）设计：Connector

**范围**：§124 / §125 的落地。本子项目是 P3 分解（A–E）中的 **B**，次序 A → B → C → D → E。
A（Capability）已交付，B 是它两处「建了但无生产调用方」的兑现方之一：

- **强制点 (2) 的连接器侧**——`AuthorizedEffect` 今天**只有构造、无消费方**（`docs/superpowers/p3a-followups.md` 第一节的表），B 是它的消费方；
- **`continuum-secrets`**——今天连依赖边都没有（同上），B 是它的第一个真消费方。

本子项目**只写设计，不落实现代码**（共享面 §七）。

---

# 0. 两处待裁决的抵触点，与一处已闭的假归因

写设计前核过共享面与规范原文。按共享面的约定「本文里的判断若与你（用户）的决定冲突，以本文为准并回报」，
凡与共享面或规范对不上的**一律留在此处并回报**，**不在正文里偷偷按我认为对的那一版写**。

第 2、3 条是**待用户裁决**的规范级归属问题（协调者已接手，本子项目不改动它们）；
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

3. **§9.1 的层间依赖图缺 `边界层 → 资源层` 这条边，而 P3A 已经需要它。** `continuum-secrets`
   是边界层，其依赖边是 `["continuum-secrets", &["continuum-capability"]]`
   （`crates/continuum-runtime/tests/dependency_direction.rs:103`），即 5 → 4；而 §9.1 的图
   （`docs/02-工程.md:559-579`）里没有这条。**这是 P3A 留下的既成事实，不是 B 引入的**，
   但在 B 这里会再叠一条同类边，故一并报出。**收件人：协调者 / 规范**。

---

# 1. 建什么、不建什么

| 组件 | 规范依据 | 本子项目 |
|---|---|---|
| 连接器的操作集与它的**声明完整性** | §125 | 建 |
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

## 4.3 B 是能力时效**第一次变得可观察**的地方

`CAPABILITY_LIFETIME_MS` 今天只是个被记下的数——驱动不读时钟地用它
（P3A 设计 §10 第 11 条、`p3a-followups.md` 第四节第 4 条）。
B 调用 `issue` 时，`issue` 会走 `Capability::is_valid_at` 并把它的
`CapabilityError` 转成 `SecretsError::Capability`（`runtime.rs:177`）。

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

- 连接器作者实现的是 B 定义的一个扩展 trait（暂名 `ConnectorImpl`），它的方法多一个材料入参；
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

1. 按操作串的**服务半边**解析连接器；未注册 → `UnknownConnector`；
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

四处取舍，各给理由：

- **上面三条注册期变体不是留桩**：它们在注册期都有真实产生方（各有照片，见第 9 节）。
  **调用期**不存在「绑定缺失」这条路径——完备性在构造期就强制了，故调用期不建对应的变体。
  这一条要说准：不是「这个形状不存在」，而是「它只能在注册期出现」。
- **`Credentials` 转出 `SecretsError` 而不展开**：`SecretsError` 的八个变体各有产生方与照片
  （`crates/continuum-secrets/src/error.rs`），B 再抄一层就会与它漂移。
  「是哪一种失败」由内层变体给出——与 P3A `CapabilityError::Persist` 的取舍同形。
- **`Provider` 独立于 `Credentials`**：后端不可用与凭据拿不到是两回事，并成一个变体会让
  「凭据被拒」这条路径上的消息说一件不真的事（P3A 为同类情形专门分过 `ForeignCredential`
  与 `Superseded`，`continuum-secrets/src/error.rs:90-99`）。

## 6.2 审计：**B 不新写审计行**

§313（`docs/spec/05-normative.md:2132`）的必录清单里，与本子项目相关的是两项：
**external effects** 与 **capability grants**。两项都已有产生方：

- `AuditKind::ExternalEffects`：`continuum-effect` 的 `record_planned`（`crates/continuum-effect/src/journal.rs:23,36`）
  与 workspace 门控的 `discard`（`crates/continuum-workspace/src/gate.rs:45`，其记录点在驱动侧）；
- `AuditKind::CapabilityGrants`：`continuum-capability` 的 `authorize`（`crates/continuum-capability/src/registry.rs` 的 `append_audit`，P3A 为该变体建的第一个产生方）。

B 再写一条，就是同一个事实两个产生点。**决定 B-5：B 的入口用那个值、不重记它。**
照片是「不写」的那一侧：一次成功的连接器调用前后，`audit` 表行数不变（第 9 节）。

**要指出的时序问题（不是 B 能处置的）**：今天 `external effects` 记的是**计划**那一刻
（驱动在跑命令之前写效应行），不是**实际发生**那一刻。若判定这条 MUST 的落点应在「实际发生」，
那是 effect journal 的活。**收件人见第 11 节。**

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

**决定 B-6：crate 落在边界层。** 理由两条，任一条单独成立：

1. **§4.2 的强制点归属**：B 要落的正是强制点 (2) 的连接器侧那半个（P3A 设计 §4.2 明写
   「连接器侧留待 B」），而它在第 5 层。
2. **§9.1 的无环约束**：`continuum-secrets` 是边界层而依赖 `continuum-capability`（资源层），
   即已有 5 → 4。若 `continuum-connector` 落在资源层，它就是 4 → 5 → 4 的**层环**，
   与 §9.1「依赖方向单向，无环」冲突。

**据实说明这条决定的强度**：`dependency_direction.rs` 断言的是 **crate 级直接边**，
上面两条路在**crate 图上都无环**（`capability` 不依赖 `connector`，也不依赖 `secrets`）。
差别只出现在**层图**上。故这不是构建约束，是归属与文档的一致性问题；
若把 §9.1 读成只约束 crate 图，则两条路都可行，此时仍以 (1) 为准取边界层。
**收件人：协调者 / 规范**（§4.1 的表要么订正，要么说明「组件在资源层、强制点在第 5 层」是可以并存的）。

## 7.3 依赖边（只登记实际用到的）

```
continuum-connector → continuum-core         （ConnectorId / ConnectorOp / ConnectorDescriptor / ProviderError）
continuum-connector → continuum-effect       （EffectType：它出现在本 crate 的公开错误类型里——
                                               `EffectAuthorizationRequired { effect }`；取值本身由下面那条边给出）
continuum-connector → continuum-capability   （AuthorizedEffect / Capability / CapabilityKind 及新加的 effect 逆）
continuum-connector → continuum-provider     （§124 的 Connector trait）
continuum-connector → continuum-secrets      （issue / material / SecretMaterial）——强制点 (3) 的第一条真消费边
continuum-runtime   → continuum-connector    （装配处；**由用它的那个 task 登记**）
continuum-runtime   → continuum-secrets      （**由把密钥运行时接上的那个 task 登记**——见 p3a-followups 第三节末段）
```

`connector` 与 `secrets` 两条到 runtime 的边**不在本子项目的第一个 task 一次声明齐**：
本仓口径是「零使用的边即假边」，`ALLOWED` 必须等于实际依赖，P2b 为此删过两条边。
这与 P3A 设计 §6「边由需要它的那个 task 增量加上」是同一条判据。

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
| 假连接器注册不了 §125 的读操作 | 以 `GitHub.merge` 建一条绑定 → 注册期拒（表里没有这一枚 kind），断言是哪一种 `Err` |
| 效应臂只收 `AuthorizedEffect` | trybuild 样例：在效应臂的位置传 `Capability` 不编译；**判据是编译失败** |
| 效应是推出来的，不是声明的 | 用例断言连接器的绑定类型里**没有** `EffectType` 这个字段可填：给一条绑定指定一个与 `for_effect` 逆对不上的效应**写不出来**；并逐项断言 `effect(for_effect(e)) == Some(e)`（P3A 已有 `for_effect` 的六条照片，本处补的是逆） |
| 凭据作用域不越能力 | 两条臂各一条：断言 `credential.scope()` 等于**那一次出示的那枚能力**的 scope；并带一条「能力的 scope 是什么、凭据就是什么」的对照 |
| 能力已失效 | 过期能力经**两条臂各一次** → `Credentials(SecretsError::Capability(CapabilityError::Expired))`（**断言是这一种**，不是笼统的 `Err`） |
| 源不覆盖作用域 | 文件源里没有该作用域 → `Credentials(ScopeNotCovered { .. })` |
| 轮换 | `rotate` 之后用旧凭据取料 → `Credentials(Superseded { .. })`（§103 的四类事件**逐项**各一行） |
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
| 4 | **Connector 的层归属**：§4.1 列在资源层，§4.2 把强制点 (2) 归第 5 层，§9.1 的层图既没有 Connector 也没有 5 → 4 的边 | 协调者 / 规范 |
| 5 | **§4.3 的层内依赖图漏了 Connector**，补法见第 8 节 | 协调者 |
| 6 | **`CAPABILITY_LIFETIME_MS` 的数值无规范来源**，而 B 是它第一次变得可观察的地方（P3A §10 第 11 条）——首次接上时须核这个数是否合理 | 子项目 B 的实现 |
| 7 | **轮换只在下一次取料时可见**，进行中的一次调用不受影响（第 4.4 节）；本阶段没有调用中途复查的机构 | 子项目 B 的实现 / 长期阶段 |
| 8 | **`AuditKind::ExternalEffects` 记的是「计划」那一刻**，不是「实际发生」（第 6.2 节）。B 不重记；若要改落点，属 effect journal | 语义层 / 后续阶段 |
| 9 | **`SecretMaterial` 出到实现之后无回收机构**：材料在调用期间存活，调用结束随适配器析构。本阶段够用，若将来材料需显式清零（§100 的硬件后端语境），此处是替换点 | 长期阶段 |
| 10 | **真实连接器零个**：本子项目全部用例跑在假连接器上，「按操作细分」在真实服务上的表现没有照片（第 6.3 节） | 长期阶段 |
| 11 | **本子项目对 P3A crate 的唯一新增面**：`CapabilityKind` 上补一枚 `for_effect` 的逆（暂名 `effect`，落在 `crates/continuum-capability/src/capability.rs` 紧邻 `for_effect`）。它是那条对应唯一的逆产生点；`for_effect` 是单射不是满射，故逆返回 `Option`。消费方是 B 的入口（第 3.2 节第 4 条） | 子项目 B 的实现 |
| 12 | **两套串的服务半边对不齐**：连接器操作 `GitHub.push_branch` 必须绑到 `Git(Push)`——resource 是 `git`、不是 `github`（第 3.4 节）。今天这不是错（§88 与驱动的铸法都如此），但**连接器说 GitHub、能力说 git** 这条缝在 §125 的服务清单（§124 列了六个服务）与 `CapabilityKind` 的 resource 集之间普遍存在。若后续出现第二个服务也需要同一枚 kind，须重新处置 | 语义层 / 子项目 B 的实现 |
