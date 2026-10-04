# P3 资源层（子项目 A）设计：Capability

**范围**：《工程》§4.1 的十一个组件中，本子项目建 `Capability Token`（§252 §253 §87 §88）与 Tool Registry，并把 Capability 的**三个强制点**全部接上。三个强制点里有两个落在边界层——它们是 P2 **因「这枚 token 尚不存在」而特意留下的**（P2 设计 §1.1：「后五项在本子项目内不建、也不留桩。它们的强制点在后续阶段落到本层已有的接口上」）。

本子项目是 P3 分解（A–E：Capability / 连接器 / 工具与 Provider 中立边界 / 模型侧与 Router / 计算节点与放置）中的 **A**，次序为 A → B → C → D → E。A 是 B、C、D 的公共前置：`Router ← Model Registry + Capability Token`、`Tool Registry ← Capability Token`（§4.3）。

---

# 1. 范围

## 1.1 建什么、不建什么

| 组件 | 规范依据 | 本子项目 |
|---|---|---|
| `Capability` 类型与词汇表 | §253 | 建 |
| Tool Registry（`Tool` / `ToolProfile`） | §252 §87 §88 | 建 |
| 强制点 (1)：Tool Registry，工具调用前 | §4.2 §4.4 | 建 |
| 强制点 (2)：执行点，实际副作用前 | §4.2 | 建（两处都落，见第 4 节） |
| 强制点 (3)：密钥运行时，凭据签发 | §51 §100 §103 | 建 |
| ModelProfile / SkillVector / Model Registry / Router / 升降级 / 路由探索 | §247–§251 §84–§86 §27 | 不建（子项目 D） |
| Provider Adapter / ToolProvider 中立边界 | §315 §316 §80 | 不建（子项目 C） |
| Connector 与权限细分 | §124 §125 | 不建（子项目 B） |
| Compute Node 注册 / 节点放置 | §287–§291 §243 §93 | 不建（子项目 E，被 OPEN-007 阻断） |

**不建的部分不留桩。**

## 1.2 强制点的归属

《工程》§4.2 原文：

> Capability 的三个强制点分布在三层：本层的 Tool Registry（工具调用前）、第 5 层的执行点（实际副作用前）、第 5 层的密钥运行时（凭据签发）。

「第 5 层」即边界层（P2）。故本子项目**跨层**：它在资源层建 token 与 registry，同时补上边界层当年因 token 缺席而未建的两个组件。P2 设计 §1.1 的表格已声明这条路径：

| 组件 | P2 当时记录的「前置」 | P2 的处置 |
|---|---|---|
| Capability 强制执行点 | 第 4 层 Capability Token（P3） | 不建 |
| 密钥运行时 | Capability 执行点 | 不建 |

## 1.3 内部次序

按依赖序（后件的依赖在前）。「密钥运行时的前置是 Capability 执行点」是 P2 设计表里的原话，故它排在最后：

```
1  Capability 类型与词汇表
2  Tool 与 ToolProfile
3  Tool Registry（持久化）
4  强制点 (1)：Tool Registry，工具调用前
5  强制点 (2)：执行点（驱动侧落点；连接器侧的义务见第 4.2 节）
6  密钥运行时（前置 = 强制点 (2)）
7  审计：AuditKind::CapabilityGrants 第一次有产生方
```

---

# 2. `Capability` 类型

## 2.1 五要素与半封闭词汇表

§253 规定五要素 `resource` / `action` / `scope` / `expiry` / `issuer`，并禁止 `full_access = true` 作为默认权限。

§87 要求「Agent 不应该自由输入任意字符串调用工具」。故词汇表取**半封闭**：`resource` 是封闭枚举，每个 resource 自带它允许的动作集，**非法组合在类型层面不可表达**。

```rust
pub enum CapabilityKind {
    Filesystem(FsAction),      // Read | Write
    Git(GitAction),            // Read | WorktreeWrite | CommitLocal | Push | DeleteRemote
    Github(GithubAction),      // CreatePr
    Email(EmailAction),        // Send
    Registry(RegistryAction),  // Publish
    Payment(PaymentAction),    // Charge
    Environment(EnvAction),    // Deploy
}

pub struct Capability {
    kind: CapabilityKind,
    scope: String,
    expiry: i64,
    issuer: Issuer,
}
```

`resource()` 与 `action()` 作访问器，把 §253 的二字段形状呈现出来；存储与比较用 `kind`，故「`Filesystem.Push`」这类组合无从写出。

## 2.2 词汇表的来源：三处照录、四处推导

§88 给例：`filesystem.read`、`git.read`、`git.worktree.write`、`git.commit.local`、`git.push`。
§253 给例：`filesystem.read:/project`、`git.write:/task-worktree`、`github.create_pr:repo/X`。

**照录**：`Filesystem.Read/Write`、`Git.Read/WorktreeWrite/CommitLocal/Push`、`Github.CreatePr`。

**推导**：`EffectType`（P2 已建的六个外部效应类型，§6.8）余下的**四个 resource**（其下五个
`kind`）在本表里**没有对应项**，而不对应就会让两个词汇表各走各的——那正是 P2 出过 Critical 的那一类。故按一一对应补齐：

| `EffectType` | `CapabilityKind` | 来源 |
|---|---|---|
| `push_branch` | `Git.Push` | §88 照录 |
| `delete_remote` | `Git.DeleteRemote` | 推导 |
| `send_email` | `Email.Send` | 推导 |
| `publish` | `Registry.Publish` | 推导 |
| `charge` | `Payment.Charge` | 推导 |
| `deploy` | `Environment.Deploy` | 推导 |

**这四条是推导不是照录**，依据是「与 `EffectType` 一一对应」；`tests/vocabulary.rs` 以逐项有照片的用例钉住这条对应（六个各一条），并断言对应是**全的**（每个 `EffectType` 都有一枚）与**单的**（无一枚对应两个）。

## 2.3 三条结构性保证

**一、`full_access` 不存在。** 不是「禁止作为默认」，而是没有这个成员、也没有这个字段——§253 的那个形状在本类型上写不出来。

**二、`expiry` 必填且短命。** §51 要求「short-lived credential」。`expiry: i64`（Unix 毫秒）为必填，无「不过期」的表示。**本类型不读时钟**：校验收 `now`（本项目的既有约定：核不收时钟、由调用方给）。

**三、不可与裸字符串互换。** 字段私有、无公开构造函数、无 `From<&str>`、无 `FromStr`；只有**单向的** `Display`（供审计与日志）。理由：若字符串能还原出 `Capability`，则凡能写字符串之处即可伪造能力——那正是 §253 要拦的。

**凭据不落库。** `Capability` 只在内存中传递。Effect Journal 的 `authorization` 字段记的是「授权是怎么来的」的**描述**（今天写的是 `approve=false;policy=deny` 这类），**不是凭据本身**。故「字符串 ↔ 能力」这条通道不存在。

## 2.4 签发点

§253 的 `issuer` 字段记签发来源。规范里的终极来源是 Authority Host（§292，长期阶段）；本阶段尚无该宿主。

**本子项目采用**：`Issuer::PolicyWithExplicitApproval`——即**策略裁决 + 显式确认**这条路径，也就是 `continuum-workspace` 的 `gate.rs` 里已经点名的那个位置（其文档原文：「Capability 与 Authority 就位后，其产生点移交给那一处」）。签发点**唯一且具名**，与 P2 的 `GateApproval` 同形。`Issuer` 为封闭枚举，`AuthorityHost` 那一变体在本阶段**不产出**，它标出移交的去向。

**移交义务**：Authority Host 就位后，本子项目的签发点移交给它。该义务以两件东西固定，而不是一句将来时——`Issuer` 枚举里那个尚未产出的变体，与本节这段文字。**本项目的教训**：只写在文档里的将来时会烂（P2b 终审查出的「可拒绝后重跑」即是一例），故凡可承载于类型的移交，不留在文字里。

---

# 3. Tool Registry

## 3.1 `Tool` 与 `ToolProfile`

规范给了两处，字段有重叠：

```
Tool {              §252
    id, version
    input_schema, output_schema
    required_capabilities[]
    effect_class
    deterministic
}

ToolProfile {       §87
    capabilities
    input_schema, output_schema
    effects
    cost, latency, trust
}
```

本子项目按职责分开，**不让重叠字段出现两份**：`Tool` 是**定义**，`ToolProfile` 是**登记项**，且**登记项含定义**而非与它并列。

```rust
pub struct ToolProfile {
    tool: Tool,
    cost: Option<Cost>,
    latency: Option<Latency>,
    trust: Trust,
}
```

§87 的 `capabilities` 与 §252 的 `required_capabilities` 是同一件事（工具需要哪些能力），取 `Tool.required_capabilities` 一处；§87 的 `effects` 与 §252 的 `effect_class` 同理，见下。

§87 的 `cost` / `latency` / `trust` 三个画像字段，规范同样只给了名字。本子项目**只定它们的容器形状**（`Option<Cost>`、`Option<Latency>`、`Trust`），取值域留空并在文档里标明——它们服务的是子项目 D 的候选排序，而排序依据要到那时才存在（§250 §84）。**不预先发明度量。** 其中 `trust` 非 `Option`：一个工具登记进 Registry 时必然有信任判定，没有「尚未判定」这一状态。

## 3.2 `effect_class` 的绑定

**`effect_class` 的取值集规范没有定义**（§244 的 `side_effect_class` 同样只给了字段名）。

本子项目**不发明第二套分类**，而把它绑到既有封闭枚举上：

```rust
pub effect_class: Option<EffectType>   // None = 无外部副作用（纯计算或只读工具）
```

依据：`EffectType` 是本项目「外部副作用」的封闭集合，策略亦按它裁决（§6.8 的理由：「开放类型会让策略表漏判」）。另造一个 `effect_class` 枚举等于**让同一件事有两个词汇表**。

## 3.3 持久化

`tool` 表（登记项落库；`Capability` 不落库，见 §2.3）。沿用本项目既有约定：枚举列小写、多词以 `_` 连接、经显式辅助函数读写且**编码挂在其类型上**（不另建表，理由见 P2 的 Task 4：同一事实两个产生点）；迁移编号取未占用值；读写函数与表定义同址。

## 3.4 强制点 (1)：`AuthorizedTool`

不做成「调用方记得先校验」，而做成**非法状态不可表达**（与 P2 的 `WritablePath` 同形）：

```rust
impl ToolRegistry {
    /// 唯一能把工具交给调用方的路径。
    pub fn authorize(
        &self,
        tool_id: &ToolId,
        presented: &[Capability],
        now: i64,
    ) -> Result<AuthorizedTool<'_>, CapabilityError>;
}
```

`AuthorizedTool` 字段私有、无公开构造函数。**拿不到它就无法调用工具**——「忘了校验」这一路径在类型上不存在。

**两个方向都拒**：工具声明了而调用方没给（§252「Tool MUST NOT 接收未声明 capability」）；调用方给了而工具没声明（超范围的能力同样不许）。两条各有一张照片。

**审计**：每次成功授权写一条 `AuditKind::CapabilityGrants`——该变体自 P0 起预留、至今无产生方，在此第一次有。拒绝则返回 `Err` 交由调用方处置，本层不替它记账。

---

# 4. 强制点 (2)：执行点

## 4.1 落点

「实际副作用前」。驱动按「命令即效应」运行调用方给定的命令，**命令跑起来就是副作用**，故该点排在 `Sandbox::spawn` 之前。

**检查的对象是命令声明的那几条 `--effect`**——它们是「这条命令会对外做什么」的声明。这接得上 P2 的既有机构：`PolicyContext` 本就有 `effect_type` 字段，故签发点可逐条问策略（裁决结果 + `--approve` 是否给出），据裁决铸出 `Capability`；铸不出即拒绝运行命令。**不发明新机构。**

## 4.2 两处都落：连接器侧的义务由类型承载

真正的副作用最终由 P3 的连接器（子项目 B）执行。故强制点 (2)**两处都落**：驱动侧今天落下，连接器侧留待 B。

**B 不下于文字**：本子项目定义**只有校验路径能产出**的值（与 `AuthorizedTool` 同形），连接器要做副作用就必须收它。若 B 将来绕过，那是**类型上表达得出来的选择**（收的是别的类型），而不是「忘了接线」这种查不出来的漏——**后者正是 P2b 全分支终审花了一整轮才查出来的那一类**（驱动的清理路径绕过 Gate 的 `discard`，于是审计行从未写下）。

## 4.3 与 `GateApproval` 的关系：并存

`gate.rs` 写「其产生点移交给那一处」。但两者**绑的不是同一件事**：

- `Capability` 绑的是「**这一类动作**准不准」（如 `git.push` 到 `origin/main`）；
- `GateApproval` 绑的是「**这一次集成**准不准」，且经摘要绑到那一份具体差异（P2 的 Task 7 为它专门做过一轮）。

故两者**并存**，互不取代。「移交」指**授权决定**搬到 Capability，不是把 `GateApproval` 删掉——后者的摘要绑定是防「拿这一次的批准去用另一次」的唯一手段，删掉即失去。

---

# 5. 密钥运行时

## 5.1 形状

§51：Agent 不直接获得所有密钥；每个节点只获得 minimum capability、short-lived credential、scoped token。

```rust
impl SecretsRuntime {
    /// 唯一取得凭据的路径。凭据的作用域不超出所给的能力。
    pub fn issue(&self, cap: &Capability, now: i64) -> Result<Credential, SecretsError>;
}
```

## 5.2 三条结构性约束

1. **作用域由能力决定，调用方无从放宽。** `issue` 收的就是那枚能力本身，而 `Capability` 字段私有、无公开构造；故「拿一枚 `github.create_pr:repo/X` 去要 `repo/Y` 的令牌」不是被检查掉，而是**要不到**。
2. **凭据短命且不越权。** `Credential` 自带到期时刻，**不得晚于**所据能力的 `expiry`（§51）。其内容为「作用域 + 到期时刻 + 取得凭据材料的句柄」，**不含能力本身**：凭据一旦签出即与那枚能力解耦，故后续无法凭它反推或扩大权限。材料本身由凭据源给出，见 §5.3。
3. **没有「取全部」的入口。** 运行时只按能力逐枚签发——§253 禁止的那个形状在这里同样没有对应成员。

## 5.3 凭据源，与一处据实的偏离

§100 要求 TOTP 那类高敏 secret「**应优先**存放在 TPM / Secure Enclave / 硬件密钥库」。本项目没有硬件后端。

**本阶段采用文件 + 环境变量的凭据源。这是真实现，不是桩**：能签发、能按能力校验作用域、能过期、能轮换。§100 是 SHOULD 而非 MUST，故这是**据实记录的偏离**（见第 10 节），指向长期阶段的替换点。

## 5.4 轮换

§103 列了四类触发事件（Authority 被攻破、永久故障转移、设备撤销、凭据泄露）。本阶段能做到的是：**按事件使旧凭据失效并重发**。

---

# 6. 依赖与 crate 划分

**按层分两个新 crate**，顺序即依赖方向：

```
continuum-capability   （资源层：Capability、词汇表、Tool/ToolProfile、ToolRegistry、签发点、tool 表）
continuum-secrets      （边界层：密钥运行时）        ← 依赖 continuum-capability
```

**不把边界层的组件塞进资源层的 crate**：《工程》§4.3 写的是「密钥运行时 ← Capability 执行点」，crate 边界按层走，混了会让依赖图说谎。

**新依赖边**（一律进 `dependency_direction.rs` 的 `ALLOWED`，那张表的断言是逐对 `assert_eq!`）。

**下面列的是终态。边由「需要它的那个 task」增量加上，不在 Task 1 一次声明齐**——
`ALLOWED` 表对**叶子 crate** 记的是**实际依赖**（`p2-followups` 第四节已记该表两种语义之别），
而 P2b 为此删过两条零使用的边（`bfdb29b` 与终审修复波各一条）。若一次声明齐，
本 crate 的条目在中间若干 task 里都在说谎。这与「迁移的注册由用它的那个 task 自己完成」
是同一条判据。

```
continuum-capability → continuum-core, continuum-effect, continuum-persist,
                       continuum-events
                       （effect：§3.2 的 effect_class 绑在 EffectType 上，须能命名它；
                         events：CapabilityGrants 审计记录；persist：tool 表）
continuum-secrets    → continuum-core, continuum-capability
continuum-runtime    → continuum-capability, continuum-secrets（接强制点 (2)）
continuum-workspace  → （不变；gate.rs 只订正文档）
```

---

# 7. 持久化

本子项目注册的迁移：`tool` 表（登记项）。`Capability` 与 `Credential` **不落库**（第 2.3 与 5.2 节）。

编号取未占用值，先查再定。装备点在 `continuum-runtime/src/main.rs`——**由用它的那个 task 注册**（P2 的 Task 10 踩过「注册排在用它的 task 之后」，不重演）。

---

# 8. 测试策略

| 验什么 | 怎么验 |
|---|---|
| 能力不可由字符串构造 | trybuild 样例：无 `From<&str>`、无 `FromStr`、无公开构造；**判据是编译失败** |
| `full_access` 写不出来 | trybuild 样例 |
| 非法组合不可表达 | trybuild 样例：`Filesystem.Push` 这类组合拼不出 |
| `EffectType` ↔ 能力的一一对应 | 六个各一条；并断言**全的**（每个都有）与**单的**（无一对应两个） |
| 强制点 (1) 两个方向 | 声明了没给、给了没声明，各一条，各断言**是哪一种** `Err` |
| `AuthorizedTool` 不可构造 | trybuild 样例 |
| `expiry` 由调用方给 `now` | 边界：恰好到期、已过期、未到期 |
| 凭据作用域不越能力 | 拿窄能力要宽凭据 → `Err`；且到期不晚于能力的 `expiry` |
| 强制点 (2) 端到端 | 经驱动：铸得出即运行、铸不出即拒绝且**命令未跑** |
| 审计 | 成功授权各写一条 `CapabilityGrants`；拒绝不写 |

**本项目既有的三条纪律一并适用**：变异须在全量 `--no-fail-fast` 下得出否定结论；凡注释写绝对措辞须有对应用例（枚举式断言**逐项**有照片）；失败路径须断言是哪一种 `Err`。

---

# 9. 完成判据

《工程》§4.4 的第一条：

```
工具调用前校验 Capability，且 Capability 不可与裸字符串互换
```

由类型样例（不可互换）与强制点 (1) 的用例（调用前校验）共同覆盖。另：本子项目额外承担的两个强制点**各需自己的照片**——驱动侧的强制点 (2) 与密钥运行时的凭据签发，不许用 (1) 的用例代表。

---

# 10. 遗留与未决项

1. **凭据源偏离 §100 的优先项**：无硬件后端，本阶段用文件 + 环境变量。替换点在长期阶段（§100 的 TPM / Secure Enclave）。§100 是 SHOULD，故非违规。
2. **`Issuer::AuthorityHost` 本阶段不产出**：移交义务见 §2.4。移交的触发条件是 Authority Host 就位，属长期阶段。
3. **`effect_class` 的取值集规范未定义**：本子项目绑到 `EffectType`（§3.2）。若后续阶段判定 tool 的 effect_class 与 Journal 的 `EffectType` 是两个轴，此处要重新处置。
4. **`ToolProfile` 与 `Tool` 的重叠由本子项目判为「登记项含定义」**（§3.1）。规范未明说二者关系。
5. **OPEN-003（预算三层无记账模型）**不影响本子项目；它压住的是子项目 D 的 Router 成本决策。
6. **OPEN-007（Artifact 隐私等级未定义）**不影响本子项目；它整块阻断子项目 E。
7. **连接器侧的强制点 (2) 由类型承载**（§4.2），其兑现是子项目 B 的义务。B 的设计须显式说明它收了那个值。
