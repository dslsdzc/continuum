# P3 资源层（子项目 F）设计：驱动侧工具调用路径

**范围**：把 Capability 的**强制点 (1)**（§4.2）接上生产路径——`authorize` → `ToolProvider::invoke`，
含审计行与失败路径。子项目 A 把强制点 (1) 建了出来（`docs/superpowers/specs/2026-10-04-p3a-capability-design.md` §3.4），
但今天的驱动走的是 `AuthorizedEffect::new`，**本 crate 之外没有任何 `authorize` 的调用点**
（`docs/superpowers/p3a-followups.md` 第一节的表）。**本子项目就是那一格的兑现处。**

**归属**：工具调用路径归 **Runtime（驱动）**，依据是《总纲》§1
（`docs/01-总纲.md:16`：「执行过程中的模型选择、Agent 编排、**工具调用**、资源调度、验证、失败恢复和
结果交付由 Runtime 负责」）与 §22（`docs/01-总纲.md:22` 的职责表）。这是用户 2026-10-05 的拍板
（`docs/superpowers/specs/2026-10-05-p3-bcdf-ownership-and-interfaces.md` 第一节）。

**对这条归属的复核（写在最前，不留给读者推断）**：另一条可选的分派是把工具当作「另一种外部资源」
并进 §124 的连接器（子项目 B）。**否掉它的理由不是「工具不是资源」，而是接口不是同一组**：
§10.3 把 §124 / §315 / §316 列为**三组各自待冻结**的接口，§316 的 `ToolProvider`
（`crates/continuum-provider/src/tool.rs:9-14`）与 §124 的 `Connector`
（`crates/continuum-provider/src/connector.rs`）是两个不同的 trait、两套不同的类型；把工具路径给 B
会要么让 B 跨到 §316 上，要么让「工具」这件事有两个描述类型——后者正是本项目一贯判为 **Critical** 的
那一类。故维持 2026-10-05 的决定：**路径归 Runtime，接口归 §316，两者在驱动里接上。**

本子项目**不建**：适配器（子项目 C）、连接器与凭据签发（子项目 B）、模型侧与 Router（Router 归
子项目 D，**模型调用路径归子项目 G**——**G 不是 F**，见 §11 的 G 行与
`docs/superpowers/specs/2026-10-05-p3-bcdf-set-decisions.md` §一.1）。
**不建的部分不留桩**；唯一的一处接缝是 §10.2 点名的**装配点**，它是 C 的插接处，不是桩。

---

# 1. F 与今天的驱动是什么关系

## 1.1 今天的驱动是什么

`task` 子命令（`crates/continuum-runtime/src/cli.rs:90`、`crates/continuum-runtime/src/task_cmd.rs`）
按 8 步跑一条**命令**：

1. `detect_backend`；2. 必要时 `unshare -Urm` 重执行自身；3. 建 Task 工作区并落库；
4. 查一次策略（集成那次裁决）→ **强制点 (2)**：逐条 `--effect` 问策略、据 `mints` 六格表铸能力
   → 写 `PLANNED → AUTHORIZED → EXECUTING`（**提交之后**才执行，`§268` 的「执行前写入」）；
5. `Sandbox::spawn` 跑 `--exec`；6. 按退出形态写终态；7. `--apply` 那一支经 Gate 集成；
8. 未集成即清理工作区与记录。

「命令即效应」是它的前提：**命令跑起来就是副作用**，故校验排在 `Sandbox::spawn` 之前
（`task_cmd.rs:23-28`）。

## 1.2 决定 F1：工具调用是**第二子命令**，与命令路径并列

**决定**：新增子命令 `continuum tool`，它走一条与 `task` 并列的路径，**共用**强制点 (2) 的铸币路径、
`mints` 六格表、效应状态机与「执行前写入」的次序，**不共用**工作区与集成那几步（工具调用不建工作区，
见 §3 的决定 F8）。它**取代**不了 `task`，也**不表达成一条命令**。

**依据**（三条，逐条对上今天的形状）：

- **§268 的「执行前写入」对工具调用同样成立**（`docs/spec/05-normative.md:1309`：所有不可纯 Artifact
  回滚的 Effect MUST 写入 Journal，且执行前记录 intent / authorization / idempotency_key /
  planned_effect）。工具调用声明的那几条 `--effect` 就是它的 planned_effect，故第 4 步那套
  `PLANNED → AUTHORIZED → EXECUTING` 必须照写，且**必须提交在 `invoke` 之前**。这条要求与命令路径
  逐字相同——这正是两者共用一套机构、而不各写一套的理由。
- **CLI 今天表达不出工具调用**：`TaskArgs`（`cli.rs:90-113`）的必填项是 `base` / `intent` / `db` /
  `exec`，`--exec` 消费其后全部参数（`cli.rs:23-35`）。工具调用没有命令 argv，却有工具 id 与一段
  JSON 输入；把它硬塞进 `--exec` 会让 `--exec` 的约定（「其后全是 argv」）说谎。
- **工具调用不落在工作区上**：`--exec` 那条路建 Task 工作区是因为被跑的命令会改文件；工具调用的外部
  效应由 §316 的适配器执行，而 `invoke_tool` 交给适配器的那份**请求侧**里没有工作区
  这一项。把工具调用做成 `task` 的一个模式，会为了复用工作区而**凭空要求**一次工作区生命周期
  （建、落库、收尾），并且让 `--apply` 那道集成门对一条不改工作区的调用显得有意义——那是发明。

**被否掉的替代（逐条附理由）**：

| 替代 | 否掉的理由 |
|---|---|
| **取代**命令路径（工具调用成为唯一的执行形态） | 无规范依据，且 §316 的适配器今天**一个都没有**（`grep -rn 'impl ToolProvider' crates/` 只在 `continuum-provider/tests/fake_provider.rs` 命中）：取代之后驱动在生产里一步都跑不了。命令路径是 P2b 的全部交付，不由本子项目处置。 |
| **表达成一条命令**（驱动把工具调用交给沙箱里的子进程去做） | 两条都不成立：(a) 那会把 provider 句柄或凭据放进沙箱进程，与 §51 的「Agent 不直接获得密钥」（`docs/spec/01-concepts.md:1931`）反向；(b) 更坏的是，**强制点 (1) 会离开真正做副作用的那条路径**——效果由子进程做，而校验在驱动里，两者之间只剩一层没人保证的约定，正是 `p3a-followups.md` 第五节第 1 条点名的「忘了接线」形状。 |
| 把工具并进连接器（归 B） | 见文首的复核。 |

**被否掉的第四种**：让 `task` 同时接受 `--exec` 与 `--tool`（二选一）。否掉的理由与上面第三条同类
但更具体：`task` 的第 3 步（建工作区并落库）与第 8 步（按记录回收）在工具调用上**没有对象**，第 7 步
（`--apply` 集成）也没有；把两套生命周期放进一个函数，会让每一步都要问「本次是哪种形态」——
本仓对这类分支的既有判决是「写错的东西要拒绝，不要静默退化」（`cli.rs` 的模块文档），
而形态分支既不是错、也不能退化，它只是**不该存在**。

---

# 2. 命令行面

## 2.1 选项表

```
用法：
  continuum tool --db <路径> --tool <工具 id> [--input <JSON>]
                 [--effect <类型>:<目标>]... [--intent <id>] [--approve]
```

- `--db`（必填，只一次）：工具登记项与审计都要经库。与 `task` 同一条理由。
- `--tool`（必填，只一次）：工具 id，取值就是 `continuum_core::tool::ToolId`（**不另建第二个
  `ToolId`**，`p3a-followups.md` 第五节第 1 条）。字形不校验（`ToolId::new` 只收字符串）：
  未登记的工具由强制点 (1) 以 `UnknownTool` 拒掉，那比在解析期发明一套 id 语法更早失败也更准。
- `--input`（可选，只一次）：**请求侧承载的那段 `input` JSON**（F 把它连同 `&AuthorizedTool` 一起交给 `ProviderRegistry::invoke_tool`；请求类型本身由 `invoke_tool` 构造，F 不命名它）。
  **省略即 `{}`**。为什么允许省略：`ToolDescriptor.input_schema`（`tool.rs:18-23`）是自由 JSON、本层
  不解释它，一个无参数的工具写 `--input '{}'` 是纯噪声。为什么默认 `{}` 而不是 `null`：`input` 是
  对象形状的输入参数表，`{}` 是「没有参数」，`null` 是「没有输入」——后者是一个本层无从赋予含义的值。
  **取 `{}` 而不是编一个值，是本驱动已经立过的先例**，逐字同一条判据：`task` 对效应的 `parameters`
  填 `json!({})`，其处的原文是「本子项目不填 parameters（设计第 6.1 节未规定内容），故填空对象而不是
  编一个值：编出来的值会变成下游读得懂、而实际无意义的输入」（`task_cmd.rs:396-398`；**原稿写 `:395-397` 少算一行，已抽核改正**）。本处**照录那
  条判据**，不为 `--input` 另立一条。
  **这是一条决定**，规范未规定。非法 JSON 在解析期即 `Err`（不可解析的输入不该到运行期才失败，
  与 `EffectType::parse` 同一条判据）。
- `--effect <类型>:<目标>`（可零次、可多次）：**这条工具调用计划施加的外部效应**。与命令路径同一个
  `EffectSpec::parse`（`cli.rs:145`），不在本子项目另立一套声明语法。
- `--intent`（可选，只一次）：效应幂等键的第一段（`effect_key`，**锚点用函数名**）。
  **订正（2026-10-07）**：原稿在此引 `task_cmd.rs:512`——该函数随 F 的 Task 3 **搬进 lib 的
  `tool_call`**（工具路径的调用点在 lib、定义在 bin，前者调不到后者）；且**实测**它在搬家前的树上
  （**`dfefb54^`**）位于 `task_cmd.rs:446`，`:512` 这个读数对不上。
- `--approve`（可选，开关）：与命令路径同一个含义（第 2 级显式确认，`task_cmd.rs:728` 的
  `explicit_current_rule`）。
- **不接受** `--base` / `--exec` / `--apply` / `--sandbox`：解析期一律 `UnknownOption`
  （`cli.rs:338` 的既有臂）。前三个没有对象（见第 1.2 节），`--sandbox` 没有子进程可沙箱化——
  工具调用在驱动进程内进行，没有任何东西被放进沙箱。

## 2.2 决定 F6：`--effect` 一组的选项**同进同出**（两向都拒）

**决定**：`--intent` 与 `--approve` 只在给出至少一条 `--effect` 时才有含义。故：

- 给出 `--effect` 而**未**给出 `--intent` → `Err`（效应没有幂等键的来源）；
- 给出 `--intent` 或 `--approve` 而**未**给出任何 `--effect` → `Err`。

两侧都拒，新增一个 `CliError` 变体（`option: &'static str` 点名是哪一个），理由：

- **为什么 `--intent` 是条件必填而不是恒必填——理由是「可观察性」，不是「方便」**：`effect` 表
  **没有 intent 列**（`Effect` 的字段见 `crates/continuum-effect/src/effect.rs` 与
  `task_cmd.rs:391-404`）——intent 只经幂等键的第一段进入落库。零 `--effect` 时给一个恒必填的
  `--intent`，它**不落任何地方**：调用方被要求交出一个**读者无从验证**的取值，而「要求一个验证不了的
  东西」正是本仓一贯拒的形状（`CAPABILITY_LIFETIME_MS` 那条「不可观察的常量」是同一族，见
  `task_cmd.rs:174-188`）。本仓对「无消费方的声明」的判据是要么删、要么写明理由（`AuthorizedTool`
  的文档就是这么处理的），这里取前者。
  **本条的可观察性对照**：给出 `--effect` 时，`--intent` 经幂等键落库、**可观察**（P-2 读回
  `effect.idempotency_key` 并断言它以长度前缀形含该 intent——**这一读就是本条的照片**，
  原稿写「P-2 直接读它」时 P-2 其实没读，已补）；
  不给 `--effect` 时它不落任何地方、**不可观察**。故规则的两侧各由一条可观察的事实支撑，而不是由
  「对称好看」支撑。
- **为什么反向也要拒**：`--approve` 在这里唯一的作用是 `mints` 的开关（`task_cmd.rs:814`），零效应
  时它什么也不开关。**静默收下一个不生效的选项**正是 `cli.rs` 模块文档点名要拒的形状
  （「一个被静默忽略的选项会让驱动按它未被告知的行为跑」）。故不许静默；要拒。

**照片（两向各一）**：`--effect charge:x` 无 `--intent` → 退出码非零且点名 `--intent`；
`--intent i1` 无 `--effect` → 退出码非零且点名 `--effect`。零效应的**正常**侧另有一条：`--tool t1`
（无 `--effect`、无 `--intent`）应当跑得起来（对一条不声明任何能力的登记项）。

---

# 3. 步骤序（决定 F3 / F4 / F8）

`tool` 子命令的次序如下。**每一步都点明它与 `task` 哪一步同形或为何不同**——两条路径的次序要能被
逐句对照，否则「两条路径」迟早变成两个说法。

| # | 本路径 | 与命令路径的关系 |
|---|---|---|
| 1 | 打开库并应用迁移：`open_db`（`task_cmd.rs:323`，**私有**）＋ `runtime_migrations`（`main.rs:57`） | 与 `task` 共用**同一份迁移集合**（两处各写一份清单会让「注册的集合」有两个来源）。**但 `open_db` 今天不可直接复用**——见 §3.5 |
| 2 | 读策略表一次；对每条 `--effect` 查幂等键，任一已存在即拒整条 | 同 `task` 第 4 步的头两段（同一函数 `record_declared_effects` 的形状），**次序也照旧**：幂等键检查排在强制点之前 |
| 3 | **强制点 (2)**：逐条 `--effect` 用 `policy_context_for_effect` 裁决一次，据 `mints` 铸能力并配成 `AuthorizedEffect`；任一条铸不出即拒整条 | 调**提取出来的共享函数**（见 §3.2），**不是**照抄 `authorize_declared_effects` |
| 4 | **强制点 (1)**：`authorize(tx, &tool_id, &presented, now)` | 命令路径没有这一步（命令路径没有工具 id）。**排在写效应行之前**，理由见 §3.1 |
| 5 | 对每条效应写 `PLANNED → AUTHORIZED → EXECUTING`，**提交** | 同 `task` 第 4 步的尾段；`§268` 的「执行前写入」在此兑现 |
| 6 | 调 `ProviderRegistry::invoke_tool(&authorized, input)`（**C 的注册表**，§6.1）；失败（含「该 id 未登记」）→ 把各效应记 `FAILED` | 同 `task` 第 5 步的 `Sandbox::spawn`：真正做事的那一下。**门禁在注册表那一跳**（§6.2），驱动不自己开入口 |
| 7 | 按结果的 `is_error` 写 `COMMITTED` / `FAILED`，并把 `output` 打到 stdout | 同 `task` 第 6 步写终态；**打印结果这一条是本路径独有的**，见 §3.3 |

**没有**第 3 步（建工作区）、第 7 步（`--apply` 集成）、第 8 步（`discard`）：**决定 F8——工具调用不建
Task 工作区、不经 Integration Gate。** 依据是 §1.2 第三条（工具跑不到工作区上）。**这条有一条明确的
代价，记在第 14 节第 8 条**：若将来某个工具**确实需要**一个 Task 工作区（例如一个改文件的工具），
§316 的接口里没有把工作区交给适配器的通道——那时要改的是 §316 的调用面（C 的活），不是在本路径上
私自造一个工作区再把它藏起来。

**适配器在什么时候被装配**：不在本表里，**在组合根**（驱动的装配点，§10.2）——那与 `task` 装配
`Sandbox` 句柄同形，是「程序启动时把机制装好」，不是本路径的一步。本表第 6 步只是**用它**。

## 3.1 决定 F3：强制点 (1) 排在写效应行之前

**决定**：`authorize` 在**任何 `effect` 行落库之前**调用。

**依据**：与强制点 (2) 同一条，而且是本仓已经付过代价的那一条
（`task_cmd.rs:54-63`、P3A 设计 §10 第 12 条）。若把 `authorize` 排在写之后，一次拒绝会留下
`EXECUTING` 记录，而**全仓没有删除 `effect` 行的路径**（`grep 'DELETE FROM effect' crates/` 无命中；
本仓确有别的 `DELETE`，故引用必须限定到 `effect` 表才成立），于是同一个 Intent 会因幂等键再也创建
不了——正是第 4 步错误路径要防的那件事。**「写都不写」严格优于「写了再清」**，且不必发明删除机构。

**由此得到一条可观察的性质**：一次被强制点 (1) 拒掉的工具调用，库里**一条效应记录也没有**，
工具也**一次都没被调用**。这句话两侧都有照片（第 9 节）。

## 3.2 决定 F5：提取一个**无事务**的共享函数，本路径自己持有事务

**问题（交叉复审指出的两处）**：本路径既要「复用命令路径的铸币判定」（§5.1：不另设判定点），又要
`authorize` 与效应行**同一次提交**（下面的性质）；而今天的 `authorize_declared_effects`
（`task_cmd.rs:442-471`）**把 `arbitrate` 出来的 `Decision` 丢掉**（§7.2 要它去填 `authorization`），
且它是 `record_declared_effects`（`task_cmd.rs:364-413`）的**内部调用**，而后者**自己开事务、自己
commit**。故「原样复用」与这两条要求**三者不能同时成立**——原稿写「逐字复用」，那句话是**假的**。

**决定**：把该函数**提取并加宽**成一个**无事务**的共享函数，两条路径同调它：

```rust
/// 强制点 (2) 的铸币判定。**不收事务**——它**不碰库**：裁决读的是已取出的策略表、
/// `mint` 不读库（`task_cmd.rs:459-463`），故调用方自己决定在哪个事务里用它
/// （本路径要用它把结果与 `authorize`、效应行同事务提交）。
///
/// **它并不「纯」**：`mint` 的 `expiry` 取 `now_millis() + CAPABILITY_LIFETIME_MS`，
/// 而 `now_millis()` **读时钟**（`task_cmd.rs:867-872`）。要说准的是「不碰库」——
/// 写成绝对措辞「纯的」，它的反例就在同一个文件里。
fn mint_declared_effects(
    policies: &[Policy],
    effects: &[EffectSpec],
    approve: bool,
) -> Result<Vec<(AuthorizedEffect, Decision)>, TaskError>;
```

- **返回 `(AuthorizedEffect, Decision)` 成对**：`Decision` 不再被丢掉——本路径要用它填
  `effect.authorization`（§7.2 那条「填这条效应自己那次裁决」），而**本路径不第二次调 `arbitrate`**
  （那正是 §5.1 禁止的第二个判定点）。**这解决复审第 3 条**：把复用面写准，而不是声称「逐字复用」。
- **无事务**：`record_planned` / `advance` / `authorize` 才需要 `Tx`；裁决与铸币都不需要
  （`mint` 自己既不读时钟也不读库，见其文档——**但本函数要算 `expiry`，故它读时钟**，
  见上面那段文档注释）。**这解决复审第 4 条**：本路径自己 `db.begin()`，把步骤 4 与 5 放进
  这**一个**事务。
- **命令路径不受影响**：`record_declared_effects` 仍然自己开事务、自己 commit，其内部改调这个共享
  函数即可；它今天丢弃 `Decision` 的那一处照旧（它要的是集成那次裁决，另有来源）。

**它落在哪个 crate——`TaskError` 随之移进 lib（N-1 的处置）**：这个共享函数**必须在 lib 里**
（§6.4：工具调用路径是 lib 函数，lib 调不到 bin 的模块），而它的返回类型里有 `TaskError`——
`TaskError` 今天定义在 **bin 的** `task_cmd.rs:876`，`src/lib.rs` 只导出 `cli`。**三者不能同时成立**
（共享函数在 bin ⇒ lib 调不到；共享函数在 lib 而返回 `TaskError` ⇒ lib 叫不出这个名字）。
**决定：把 `TaskError` 从 `task_cmd.rs` 移进 lib**（新增一个 lib 模块，经 `lib.rs` 再导出），
`task_cmd.rs` 与 `main.rs` 改为 `use continuum_runtime::TaskError;`。
- **为什么不另建一个 lib 侧的 `ToolCallPathError`**（另一条可选处置）：那会给同一批失败造出**第二个
  错误类型**，两条路径各报一套，且 bin 还要写一层逐变体映射——正是本项目判为 Critical 的「同一件事
  两个类型」。**搬家是一步机械动作，造第二套词汇是一个长期的坑。**
- **变体一个不改**：§8 的失败面表**逐行照旧**（只是这些变体的定义位置从 bin 挪到 lib）。
- **`TaskError` 的 `#[from]` 目标**（`GateError` / `WorkspaceError` / `SandboxError` /
  `PersistError`）来自 lib 已依赖的 crate；**但 `SandboxSelectError` 不是**（plan-f 第 1 条）——它定义在 **bin 模块**
  `src/sandbox_select.rs`。故**该模块整体搬进 lib**（变体与 `Cargo.toml` 的依赖项不动）。
  **原稿在此处写「全部来自 lib 已依赖的 crate，故这一步不动 `Cargo.toml`」，那句话是假的，
  来历留此**（plan-f 第 1 条）。
- **`Cargo.toml` 并非「不动」**（plan-f 第 8 条）：库级夹具要实现 `#[async_trait]` 的 `ToolProvider`，
  故 `continuum-runtime` 需要 **`async-trait` 的 dev 边**。它是**外部 crate**，
  故 `ALLOWED`（内部边表）不受影响——**「不新增任何 crate 边」这句话的射程是
  `ALLOWED` 那张内部边表，不是 `Cargo.toml` 的全部**（§10.1 据此加限定）。
  **订正（2026-10-07，回灌 F Task 3 的实测）**：本行原写「`ALLOWED`（内部边表）与 **`Cargo.lock`**
  都不受影响」。**`Cargo.lock` 那一半是假的**——dev 边同样写进锁文件（`[[package]] continuum-runtime`
  的 `dependencies` 多一行 `"async-trait"`）；**实测** Task 3 的提交 `dfefb54` 对 `Cargo.lock` 的改动
  为 **`1 +`**。**旧话留此**，免得后来者照它断言「锁不变」。
- **随搬家一并进 lib 的判定小件，以及本设计里引它们的行号（2026-10-07 据实测补记）**：
  把共享函数搬进 lib **不止搬它自己**——它的**函数体**要调 `arbitrate` / `mints` / `decision_name` /
  `policy_context`（逐条效应那次用 `policy_context_for_effect`），而 bin 的 `task_cmd.rs` 仍在调
  `arbitrate` / `policy_context` / `mints` / `decision_name` / `explicit_current_rule`。**函数体在 lib、
  调用方分处两个 crate** ⇒ 这些定义必须跟着进 lib 并取 **`pub`**（bin 看不见 `pub(crate)`；唯一例外是
  `policy_context_for_effect`，bin 侧只在注释里提它，收窄为 `pub(crate)`）。**这是「行为不变」的必要条件，
  不是新增 API**（实现计划 Task 2 的 Produces 已记）。
  **由此，本设计里凡引 `task_cmd.rs:` 来定位 `mint` / `mints` / `arbitrate` / `policy_context` /
  `policy_context_for_effect` / `explicit_current_rule` 的，都是「搬家前」的读数**（§2.1 的 `:728`、
  §2.2 的 `:814`、§3.2 段内的 `:459-463`、§4.3 的 `:459-463`、§5.1 的 `:459`、§5.3 的 `:563`、
  §8.1 的 `:459-463`），**引用时以函数名为锚**；本文档不逐一改写这些历史读数，记此以免后来者照着
  去 `task_cmd.rs` 里找它们。

**由此得到一条「承重」性质**（原稿即有，现在才真的成立）：步骤 4 与 5 用**同一个事务**，一次 `commit`。
故「工具已获准」的审计行与「效应已在执行」的 `EXECUTING` 行**要么都在、要么都不在**：不存在
「审计说授权了、日志说没开始」的中间态。这条性质的照片是一条断言：被拒时 `audit_log` 的
`capability grants` 行数为 0（`authorize` 拒绝不写审计，P3A 设计 §3.4），且 `effect` 行数为 0。
**事务由谁开**：由 §6.4 那个 lib 函数自己开（它收 `&Db`），故「同一次提交」的范围完整地落在
lib 内的一处，不需要跨 crate 传递事务。

**这处提取与 `TaskError` 的搬家都是要落地的实现步骤**（写进实现计划），不是措辞调整：不提取而照旧
调 `authorize_declared_effects` 的话，上面那条承重性质**不成立**（两个事务），本设计就是错的。

## 3.3 决定 F7：工具调用的结果由驱动打到 stdout

**决定**：步骤 7 把 `ToolResult.output` 以 JSON 一行写到 stdout（无论 `is_error` 为何）。

**依据与代价**：命令路径里驱动**不在正常路径上写 stdout/stderr**（`task_cmd.rs:246-248`），因为被跑
的子进程自己的输出即是调用方看到的东西——**输出有一个不属于驱动的产生方**。工具调用**没有子进程**：
`ToolResult` 是驱动手里唯一一份结果，没有第二个东西会替它说话，**stdout 是调用方仅有的那条通道**；
不打印等于调用方付了一次调用却看不到结果。**这正是两条路径在此处必须不同的原因**，不是风格取舍
——把它按命令路径的规矩删掉，工具调用的结果就没有任何出口了。记在此以免后来者照 `task` 的样子删。

**这条决定的照片有一半拍不到（plan-f 第 4 条）**：「结果确实出现在 stdout 上」这一半，库级用例到不了那一跳（打印在 `tool_cmd` 的 bin 层）、且 `println!` 被测试框架捕获，故**据实记为无照片，不许为它造夹具**；能拍的是它的**上游**——`ToolResult` 被交回到调用处（P-11 那一半）。

## 3.4 决定 F4：工具调用（那一跳）排在强制点与效应行**之后**

**决定**：`invoke_tool`（步骤 6）排在强制点 (1)（步骤 4）与效应行的提交（步骤 5）之后。

**依据**：这是本驱动**已有的形状**——`task` 的第 4 步做强制点 (2)、第 5 步才**真正做事**：
机制在 `:252` 解析（`sandbox_select::select_for_this_machine`），`Sandbox::spawn` 在
`task_cmd.rs:588`（`run_in_sandbox` 内）。「做事的那一下」之前的次序正是 `§268` 要的「执行前写入」。
**订正**：本处原引 `task_cmd.rs:252` 作为 `spawn` 的位置，**那是机制解析那一行、不是 spawn**
（`:588` 才是）——行号已改正，两行分列（plan-f 计划评审抽核时发现）。
**注意这一条说的是「调用」的次序，不是「装配」的次序**：适配器由组合根在启动时装进注册表
（§10.2），与 `task` 装配 `Sandbox` 句柄同形，**不构成强制点前后的一步**。
**原稿在此处写的是「`ToolCaller` 的解析排在强制点之后」**，那是驱动自己开入口时的一个解析步骤；
随 §6.2 的改调它不存在了，**次序原则不变**（做事排在权限之后），只是落点从「解析」变成「那一跳」。

**由此，今天生产路径的终点位置是确定的**：只要步骤 2、3 不先拒（幂等键冲突、某条效应铸不出
能力），本路径就**会**走到强制点 (1)——**它不会因为「没有适配器」而停在强制点之前**（那是步骤 6 的
事）。故 `authorize` 有生产调用方，这正是 F 的兑现。步骤 6 因**注册表是空的**而失败
（`ToolCallError::Unregistered`），**它不挡**强制点 (1) 的接线（`authorize` 在步骤 4，早于那一跳）。
**订正（2026-10-07）**：本行原写「这不是遗留物——见第 14 节第 2 条，**它是 C 的交付缺口**」——
**「C 的交付缺口」是过期读数**：C 已交付注册表机制，而按中立性规则它不会在自己 `src/` 里提供
`impl`；缺的是**一个可登记的适配器实现**，收件人是**驱动自己／将来的适配器子项目**。
三条依据（C 设计 §4.1 表与末段、§7.1 末段、`tests/neutrality.rs` 的守卫）见 §10.2 那条订正。
**旧话留此。**

**调用失败时各效应记 `FAILED`**（而不是什么都不写）：与 `task` 第 5 步机制选择失败时的处置
一致（`task_cmd.rs:249-255`：注释在 `:249-251`、那两行走第 6 步记终态在 `:252-255`；
**原稿引 `:250-255` 少算了起行，已抽核改正**）。两条路径在「机制没拿到 / 做事那一下失败」这一格上
给同一个答案。

## 3.5 一处要顺带提取的私有函数：`open_db`

**问题**：`open_db`（`crates/continuum-runtime/src/task_cmd.rs:323`）是 **bin 目标里的私有函数**
（`src/lib.rs` 只导出 `cli`），两条子命令要共用它，今天**拿不到**。原稿把它写成「`open_db` /
`runtime_migrations`，`main.rs:57`」，那两个引用**都不对**：`runtime_migrations` 在 `main.rs:57`
（`pub(crate)`，可用），`open_db` 在 `task_cmd.rs`（私有）。

**决定**：把 `open_db` 提到 **`pub(crate)`**（它已经与 `runtime_migrations` 同在一份迁移集合上，
`task_cmd.rs:319-327` 的文档也写明「两处各写一份清单会让『注册的集合』有两个来源」）。两条子命令
都是 bin 的模块，`pub(crate)` 即够；**不必**把它搬进 lib（它做的事与 CLI 的库面无关）。
**这是实现计划里的一步**，写在此以免实现者照着原稿那句去 `main.rs` 里找一个不存在的函数。

**它与 §6.4 的分工**：工具调用路径的**库函数**收的是**已经打开**的 `&Db`；开库由 bin 侧的
`tool_cmd` 做（经上面那个 `pub(crate)` 的 `open_db`）。故「谁开库」只有一个产生点。

---

# 4. 强制点 (1) 的落点

## 4.1 四个入参逐一定

```rust
authorize(tx, tool_id, presented, now) -> Result<AuthorizedTool, CapabilityError>
```
（`crates/continuum-capability/src/registry.rs:102-107`）

| 入参 | 取值 | 依据 |
|---|---|---|
| `tx` | 步骤 1 打开的那个库的事务；与效应行同一次提交 | `authorize` 要读 `tool` 表并写审计（同函数文档） |
| `tool_id` | 命令行 `--tool` 给出的那个 `ToolId` | 唯一一个来源：调用方说要调哪个工具 |
| `presented` | **由步骤 3 铸出的 `AuthorizedEffect` 逐枚取 `capability().clone()` 而来**，按 `--effect` 的声明次序 | 见下 |
| `now` | [`now_millis`] 在**强制点这一步取的唯一一次** | `authorize` 不读时钟（P3A 设计 §2.3 第二条）；能力刚由步骤 3 铸出，故这一步的时效判定必然通过（见 §9.3） |

**`presented` 为什么取自 `AuthorizedEffect` 而不另建一个 `Vec<Capability>`**：本步骤的产物已经是一组
`(效应, 准它的能力)`（强制点 (2) 的载体，第 6 节）。若再建一个自由浮动的 `Vec<Capability>`，同一批
能力就有了**两个可以各自构造的容器**，而两者的构造分歧编译得过、也跑得过。取
`authorized.iter().map(|a| a.capability().clone())` 使「出示的」与「已获准做效应的」在构造上就是同一批
值。这条**没有独立的照片**（它是构造点的选择，不是可观察行为）；可由一条集成用例间接承重：一条
`--effect` 铸出的能力**恰好**是 `authorize` 收到的出示集——但注意那条用例钉的是「两者一致」，
不是「两者同源」，后者由评审维持。据实写明。

## 4.2 返回的 `AuthorizedTool` 怎么处置

- **持有**：它是 `run` 的局部变量，活到步骤 7 结束、随作用域析构。**它是调用工具的唯一凭据**
  （类型保证见 §6.3）。
- **它只经两处交出去，且两处交出去的都是「整枚它」**：**（i）** 交给 `invoke_tool`（§6.1）；
  **（ii）** 按裁决 2 **用它构造扩位后的请求类型**交给适配器（§6.5）——**不是拆出授权值再传一份**。
  **除此之外它不进任何地方**：
  不进**请求侧承载的 `input`**（那是自由 JSON，不是授权通道）、不进 stdout、不进命令行环境、
  不进 `effect.parameters`（后者本路径照 `task` 的先例写 `{}`，见 §7.2）。**这一条与 §51 的禁止同源**：
  命令路径上驱动不得把 `AuthorizedEffect` 或其内容塞进命令的环境（`task_cmd.rs:46-52`），
  工具路径上驱动同样不得把 `AuthorizedTool` 塞进工具的**输入**——**授权走请求侧的具名位置，
  不走 `input`**，这两件事在 §6.5 之后仍分得开。
  **原稿在此处写过「它不被序列化到任何地方」**，那句话在裁决 2 之后**不完全对**（请求侧那一处是刻意的
  交出），已按上句订正，来历留此。
- **它不进审计 payload**：审计行由 `authorize` 自己写（`registry.rs:133-140`），内容是工具 id 与每枚
  已获准能力的 kind / scope。驱动**不补写第二条**（§7.1）。
- **驱动**不调 `AuthorizedTool::granted()`：**取 `granted()` 的是适配器**（C 设计 §7.5）。
  F 这一侧只做一件事——**把整枚 `AuthorizedTool` 放进请求类型**，故「这次调用被授权了哪几枚能力」
  由**收到它的那一方**去读，F 不替它挑、也不把它拆成另一份值。
  **订正的来历（本节改了三次，全部留此，免得后来者按任一旧版改回去）**：
  ① 原稿写「本阶段那个位置在连接器侧（子项目 B）」——**那句没有依据**（三份设计的 grep 里 B 不含
  `granted`）；② 订正后记成「本路径只读 `tool_id()`、`granted()` 悬空」——**那句在裁决 2 之后也陈旧了**；
  ③ 现按 C 的 §7.5 落定：**F 不调 `granted()`，适配器调**。
  **「谁在哪一行调它」以 C §7.5 为准**（协调者已要 C 把那一句写死，本设计**等那句同步，不自行发挥**）。

## 4.3 本路径**不**判作用域（据实写明的限度）

`authorize` 的比对**只按 kind**，作用域由能力自带（P3A 设计 §3.4「比对按 kind，作用域由能力自带」）。
故在本路径上：调用方用 `--effect git.push:origin/main` 声明、铸出 `git.push:origin/main` 这枚能力；
若它真正想做的是 `git.push:origin/dev`，**本路径不会拦**——它拦不了，因为工具调用的输入
（即请求侧承载的那段 `input`，仍是一段自由 JSON）本层不解释它，也无从知道「输入里的分支」是哪一个。

这条**不是本子项目的缺陷，是本层边界的既有形状**：作用域的强制落在凭据签发（§51、P3A 设计 §5.2：
凭据的作用域不超出所给的能力）与执行点（子项目 B）。**本路径的作用是把这个 scope 原样带进
`AuthorizedTool`**，不是替下游判它够不够。

**照片要分成两半说，原稿把那两半混成了一句「没有照片」，那句话过强**：

- **作用域的「判定」没有照片，本路径也不该有**（本路径不判它）；
- **作用域的「原样带出」有照片，而且就在本路径手里**：步骤 3 用 `spec.target` 作 scope 铸能力
  （`task_cmd.rs:459-463`），这枚 scope 随已获准的能力进 `authorize` 写的审计 payload 的
  `capabilities[].scope`（`registry.rs:153-164`）。**一条端到端断言「审计 payload 里各
  `capabilities[].scope` 逐个等于对应的 `--effect` 目标」（P-18）就把本路径这半边钉住了。**
  原稿把这半边整份推给 capability 自己的用例，**那是不成立的**：那个 crate 的用例钉的是
  「`authorize` 把收到的 scope 原样写进 payload」，**钉不了「驱动传进去的是 `spec.target`」**。

记在第 14 节第 5 条，收件人 B（作用域的**判定**那一半）。

---

# 5. 与强制点 (2) 的关系

## 5.1 决定 F2：一次铸币，两处用

**问题**：一条 `effect_class` 为 `Some(t)` 的工具被调用时，是否需要一枚 `AuthorizedEffect`？

**答**：**需要，而且它与强制点 (1) 用的是同一批铸出来的能力，不是两套铸币。** 具体地：

1. 调用方用 `--effect` 声明本次调用计划施加的外部效应（与命令路径同一语法、同一解析）；
2. 每一条声明**恰好铸一枚**能力（`mint`，`task_cmd.rs:459`），经 `mints` 六格表判定——这是
   **六格表唯一的落点**，本子项目**不重写、也不新增**第二个判定点；
3. 这枚能力装进 `AuthorizedEffect`（强制点 (2) 的连接器侧载体，`crates/continuum-capability/src/effect.rs:55`）；
4. 同一批能力（逐枚 `clone`）作为 `presented` 交给 `authorize`（强制点 (1)）。

故**每次工具调用里，每个 `--effect` 只有一个 `mint` 调用点**，它的产物同时喂给 (1) 与 (2)。
这条是本设计里最要紧的一处防「两个词汇表」的措施：**两个强制点之间没有第二套铸币机构**。

**为什么 `presented` 不另走一条「按工具声明的 required_capabilities 逐枚铸」的路**：那条路会让
**同一枚能力有两个产生点**——一个是 `--effect` 逐条铸的，一个是按登记项铸的；两者可以各自漂移，
且「谁说了算」没有依据。更要紧的是，那样铸出的能力**不必对应任何声明过的效应**，于是
§268 的 Journal 会出现「有授权、无 planned_effect」的洞。

## 5.2 决定 F2b：驱动**不读** `Tool::effect_class`

**决定**：本路径不使用 `Tool::effect_class`（`crates/continuum-capability/src/tool.rs:95`）做任何判定。

**理由**：`effect_class` 与 `--effect` 若都参与「这条调用的外部效应是什么」，同一件事就有了两个
来源。本路径的处置是**只让声明（`--effect`）当来源**，并靠下面这条**登记项不变量**让 `effect_class`
不可能被绕过：

> **登记项不变量（本设计提出，由 `continuum-capability` 的 `save_tool` 强制）**：
> 登记一条 `effect_class == Some(t)` 的工具时，其 `required_capabilities` 必须包含
> `CapabilityKind::for_effect(t)`（`crates/continuum-capability/src/capability.rs:97`）。

**强制点定在 `save_tool`（协调者本轮拍板，原稿记的是「C 在登记时强制」，已订正）**：
`save_tool`（`crates/continuum-capability/src/persist.rs:64`）是**一条登记项进入 `tool` 表的唯一生产
写点**——`tool` 表只有它一个写入口（另一个读入口是 `load_tool` / `load_tools`），故把它放在这里，
这条不变量就在**工具定义进入存储的那一个点**上成立，而不必依赖任何调用方自觉。
**订正的来历**：本设计原稿把这条义务记给子项目 C（登记入口的实现方）；协调者裁定归
`continuum-capability`——理由是「谁写库谁把关」，C 那边根本不必知道这条规则。
**它同时把上文那条洞关在唯一入口上**：`save_tool` 是唯一的写点，故没有第二条路能让一条违反
不变量的登记项落库。

**这条不变量为什么够（射程限定在 `effect_class == Some(t)`；plan-f 第 6 条）**：若某工具有
`effect_class == Some(t)`，则它必然声明了 `for_effect(t)` 这枚能力；
`authorize` 的两向合取（出示集 ⊆ 声明集 且 声明集 ⊆ 出示集）于是**要求**出示集里有一枚
`for_effect(t)`；而本路径的出示集只可能来自 `--effect` 铸出的能力；故调用方**必须**声明一条能铸出
`for_effect(t)` 的 `--effect`，那条效应遂进 Journal。**故「工具做了外部效应却没有效应记录」这条洞
在 `Some(t)` 这一支上被堵死**，而驱动不必读 `effect_class`。
**原稿在此处写「这条洞由此被不变量堵死」，那是绝对措辞、过宽，来历留此。**
**`None` 那一支不开这个保证**：`effect_class == None` 的工具若其适配器仍做了外部效应，而这次调用
**零 `--effect`**，则**不留痕**——没有效应行、也没有别的记录（§7.3 与 §14 第 9 条：§312 的
trace 无落点）。这条不变量管的是「登记项与其声明一致」，**管不了适配器实际做了什么**。

**连带面（plan-f 第 5 条，实现时必须一并处置）**：在 `save_tool` 里加这条检查，会**打红 P3A 三处既有用例**
——`continuum-capability/tests/persist.rs` 的夹具 `tool()` 用了 `effect_class = Some(DeleteRemote)`
而能力表不含 `Git(DeleteRemote)`，涉及 `tool_round_trips` / `required_capabilities_round_trip` /
`saving_the_same_id_twice_is_rejected`。**这是「新增一道前置判定会改变既有用例命中的分支」那一类**，
不是本设计的缺陷；处置随实现计划的 Task 1 连带修正（把那三处夹具的能力表补齐或改 `effect_class`）。
**本设计只据实记下这条连带面，不在这里改那个 crate 的用例。**

**为什么这条不变量今天仍没有强制点**：`save_tool` 的生产调用方今天**不存在**（全仓只在测试里被调，
见第 14 节第 1 条），故**实现这一步（在 `save_tool` 里加这条检查）就是要落地的那件事**，
记在第 14 节第 7 条，**收件人改为 `continuum-capability`**（不再是 C）。**在该检查落地之前，
「工具做了外部效应却没有效应记录」这条洞是开着的**——本设计不假装它已经关上。

**被否掉的替代**：让驱动读 `effect_class` 并与 `--effect` 的集合比对。否掉的理由：那要么是
`authorize` 之外的第二道「这条调用准不准」的判定（第二个判定点），要么是一次冗余的重复读
（`authorize` 内部已经读过同一条登记项），而它换来的保证**可以由上面那条不变量以零运行时成本给出**。
**「用不变量换掉一次运行时判定」比「多一个判定点」更贴本项目对单一产生点的判据。**

## 5.3 今天能走通的工具是什么样的（据实写明的窄面）

`presented` 只可能含**六个由 `EffectType` 派生的 kind**（`CapabilityKind::for_effect` 的六个臂）。
词汇表另六个 kind——`Filesystem(Read|Write)`、`Git(Read|WorktreeWrite|CommitLocal)`、
`Github(CreatePr)`——**在 `EffectType` 里没有对应项**（`capability.rs:86-96` 的文档明说了这一点），
故策略的 `PolicyContext.effect_type`（`task_cmd.rs:563`）也没有它们的事实，**驱动无从为它们裁决、
也就无从铸出它们**。

**后果（可观察）**：一条 `required_capabilities` 含上述六种之一的工具，在本路径上**一律**得到
`MissingCapability`，**无论调用方怎么声明**——因为声明不出这种能力。故今天能走通的工具，其
`required_capabilities` 必须**恰好**是某组 `--effect` 声明经 `for_effect` 派生出的那些 kind，且两向
必须相等（多一枚 `UndeclaredCapability`、少一枚 `MissingCapability`，两向都由 `authorize` 判）。

这是一处**分阶段的限度**，不是本子项目的缺陷，也不是本子项目能关掉的东西：给这六个 kind 一条
策略事实，要动的是 `PolicyContext` 的事实集合（策略层）与「非效应类能力由谁判准」这条口径。
**收件人在第 14 节第 4 条**（策略层与子项目 D），本子项目**只据实写明它，不越权替它定**。

**对登记面的直接后果，要一并说明白**：今天**可以**登记一条 `required_capabilities` 含上述六种之一
的工具——`save_tool` / `ToolProfile` 对 kind 的取值不设这种约束（`required_capabilities` 收的是
`Vec<CapabilityKind>`，十二个 kind 都合法，`crates/continuum-capability/src/tool.rs:46`），
而这条登记项**在本路径上永远调不动**（每条调用都停在 `MissingCapability`）。

**本设计对这条后果的处置是「据实写明」，不是「在登记期拦住」**，理由是**登记入口不是本子项目的**：
它今天不存在于生产代码里（`save_tool` 生产零调用，第 14 节第 1 条），**其归属是 `continuum-capability`
的 `save_tool`**（协调者本轮把「登记项的合法性由谁把关」判给写库的那一方，见 §5.2 与 §11 的
`continuum-capability` 行；原稿在此处写「是 C 的活」，已订正）。**在别人的入口上装一道本层的闸，
会把这条限度变成一个只有本层知道的口径**，而那正是「同一件事两处各判一次」的形状。故本子项目**不**在
登记期拦，只在正文与第 14 节第 4 条把它写明；**将来若登记面要拦，那是 `continuum-capability` 的决定**，
且届时须一并处置「登记项与 `PolicyContext` 的事实集合谁先扩」这个次序问题。

---

# 6. 工具调用的唯一落点：C 注册表的受门禁入口

## 6.1 形状：驱动调 `ProviderRegistry::invoke_tool`

工具调用的**唯一入口是子项目 C 的注册表**，不是驱动自己开的一个函数：

```rust
// continuum-provider::registry（**子项目 C 建**，见 C 设计 §7.1）
ProviderRegistry::invoke_tool(&self, authorized: &AuthorizedTool, input: Value)
    -> Result<ToolResult, ToolCallError>;
    // 裁决 2（§6.5）：**请求侧**承载授权——按 C 设计 §7.5，请求类型装的是**整枚
    // `AuthorizedTool`**（＋ input），故工具 id 只由授权证明给出。**那个类型由 `invoke_tool`
    // 在内部构造，F 的生产路径不构造、不经它传值**（**F 的库级夹具会 `impl ToolProvider`，
    // 故签名里会写到它**——那是被实现的 trait 的形状，见 plan-f 第 3 条）；本处不写它的字段。

pub enum ToolCallError {                 // C 设计 §3.5（定义）与 §7.1（用于 invoke_tool）
    Unregistered { id: ToolId },         // 这个 id 没有适配器（路由）
    Provider(ProviderError),             // 适配器自己失败
}
```

**一处要提名的作者风险（跨设计接缝 2）**：`ToolCallError` 的**定义**在 C 设计 §3.5
（`…p3c-…:228-229`），用在 `invoke_tool` 上的**签名**在 §7.1（`…p3c-…:474`）；而 §3.1 一带
（`…p3c-…:139`，该节讲注册表的暴露面）只写了入口的名字、没写返回类型，**更早的 C 修订版**在那一处
曾把失败写成 `ProviderError`。**若读到的版本里 §3.1 与该入口的返回类型不一致，以 §3.5 + §7.1 为准**
（`ProviderError` 描述的是**一次 provider 调用**的失败，`ToolCallError::Unregistered` 描述的是
**路由未命中**，C 自己的 §5.2 把这两者分开的理由写得很清楚）。**本设计按 `ToolCallError` 写**
（§8 的 P-9 / P-10 逐臂断言它），并把这条不一致**摆出来**而不是静默取一个——实现者读到旧文本时
要知道该按哪一处。

**工具调用路径不持有适配器、也不自己开第二个调用入口**（见 §6.2）：它在步骤 4 经 `authorize` 拿到
`AuthorizedTool` 之后，把 `&authorized` 与输入交给注册表的 `invoke_tool`。工具 id 由 `invoke_tool`
**只从 `AuthorizedTool` 取**（C 设计 §7.1 末段：「工具 id 只有这一个来源」），故「授权 t1、调用 t2」
在这条入口上写不出来——这是**配对**，与 `AuthorizedEffect` 的保护落在配对上同一个手法
（P3A 设计 §4.2 的执行期订正：能保护的不是「外部构造不出来」，而是配对）。

**「不持有」的范围要说准（原稿在此处写的是「驱动**不持有**适配器」，那句话在本设计自己的另一条
路径上为假）**：**驱动**（组合根，§10.2）在装配期**确实**构造并登记适配器，它手里那一份是裸的
（C 设计 §7.1 末段记的同一限度）；「不持有」说的是**工具调用路径**——它只拿 `&ProviderRegistry`，
拿不到 `Arc<dyn ToolProvider>`。两句话不矛盾，但混成一句就是假的，故分开写。

## 6.2 为什么门禁在 C 的注册表、不在驱动（原稿在此处被推翻，来历留此）

**原稿（本轮并行起草时）写的是驱动自己的一套**：`pub struct ToolCaller { provider: Box<dyn ToolProvider> }`、
`pub async fn invoke_authorized(...)`、`pub fn select_tool_caller()`。**该形状已被协调者裁定废弃**
（本文档按裁定改写），改调 C 注册表的 `invoke_tool`。两处理由：

1. **门禁放在「最后一跳」，对每个持有者都成立。** 放在驱动的一个 crate 私有函数里，它只绑住那
   一个函数：**任何拿到注册表、或拿到某个适配器实例的代码**都可以绕开它去调 §316 的
   `invoke`（后者是 §316 的 trait，其请求参数由 `invoke_tool` 在内部构造后传入，C 设计 §7.1 / §7.5）。
   放在注册表里，注册表**不交出** `Arc<dyn ToolProvider>`（C 设计 §7.1：**不提供** `tool_for`、
   也**不提供** `tool_providers()` 枚举；只读的 `list_tools` / `describe_tool` 不在门禁内），
   故**任何持有注册表的代码都只能经 `invoke_tool`** 调到工具。
2. **原稿反对这条边的那句话已被推翻。** 原稿写：「加一条 `continuum-provider → continuum-capability`
   的边去收 `AuthorizedTool` 会改掉那条中立边界」。**那句是假的**——C 的设计 §4.1 的订正段正是
   加这条边，并给出**层内边**的论据（「接口 → 能力类型」，不是「接口 → 实现」；C 设计 §7.2）。
   **订正的来历**：本子项目的派发早于 C 的裁决，并行起草时无从知道它。**这不是本子项目的判断失误，
   是并行设计的产物**——按本仓的规矩，把它记在这里而不是抹掉。

**本项目规则：一个事实只能有一条生产路径、一个「唯一」的说法。** 若 F 也建一个入口，同一件事就有
两个产生点、两个各自自称「唯一」的入口，而两者**都编译得过、都能跑通**——那正是本项目判为
**Critical** 的形状（P2 为此出过一次）。故 F 这一侧**删掉** `ToolCaller` / `invoke_authorized` /
`select_tool_caller`，只保留「拿着 `AuthorizedTool` 去调注册表」这一步。

## 6.3 类型层说得出什么、说不出什么（据实分界）

**说得出**：`AuthorizedTool` 字段私有、无公开构造函数、**构造通道只有 `authorize` 一条**
（`crates/continuum-capability/src/registry.rs:37-57`；crate 外的构造通道由其编译失败样例
`crates/continuum-capability/tests/compile_fail/authorized_tool_cannot_be_built.rs` 钉住，
P3A 设计 §8）。注册表的 `invoke_tool` **只收它**，且注册表不交出适配器。故**「拿不到
`AuthorizedTool` 就调不了工具」对每个持有注册表的人都成立**——要绕开它，就得先造一枚
`AuthorizedTool`，那编译不过。

**说不出**（两条，据实写明）：

1. **`ToolProvider` 仍是 §316 的公开 trait**——但**「自己拼一个请求去调 `invoke`」这条旧路已经走不通**，
   因为**工具侧唯一的入口是 `ProviderRegistry::invoke_tool`**（C 设计 §7.1），它**在内部**用
   `&AuthorizedTool` 构造请求（C 设计 §7.5）。
   **订正（本条的论证方向反了，来历留此）**：原稿写的是「任何持有适配器实例的代码可以自己拼一个
   `ToolInvocation { tool: ToolId::new("随便"), input: ... }` 直接调 `invoke`，而这段代码**编译得过**」
   ——**那句在 `ToolInvocation` 被删、请求侧换成只装得下「已授权」的类型之后为假**（C 设计 §7.5）。
   故「拿一枚自己编的 id 去调 `invoke`」**在类型上写不出来**：那条路的请求只能由 `invoke_tool` 构造，
   而它**只收 `&AuthorizedTool` 与 `input`**（不另收 `ToolId`）。
   **剩下说得出的限度只剩一条**：**任何已经持有一枚真 `AuthorizedTool` 的代码**都能走到
   `invoke_tool`（那是 `authorize` 给的，属正当路径），而**装配者**在装配期手里那份裸适配器
   **也不再够用**——它同样要先有一枚 `AuthorizedTool`。C 设计 §7.5 的编译失败样例钉的正是这一条。
2. **配对的粒度是「工具 id」，不是「适配器真的按这个 id 做」**：`invoke_tool` 按
   `authorized.tool_id()` 路由，请求里的 id 也只从授权证明来，但一个实现多工具的适配器**仍然可以**
   无视它去做别的事（`ToolResult` 里没有回执字段可比对）。**类型层管住了「id 从哪来」，
   管不住「适配器照着它做」**——适配器是可信代码（C 的交付、在驱动进程内），本层不为它兜底。

**「唯一入口」这条保证的照片在 C 那边**（C 设计 §7.1 的编译失败 / 反依赖用例），**F 不重复**——
重复一遍就是同一件事两个产生点。**说得出与说不出的分界本身没有照片**：它是关于「别的 crate 里能
写出什么」的命题，本仓的用例只能证明「我没那么写」。据此写在正文而不当照片用（与 P3A 对 `Trust`
非 `Option` 的处置同法，`crates/continuum-capability/src/tool.rs:126-131`）。

## 6.4 决定：工具调用路径是一个**库**函数

**签名收 `&Db` 与 `&ProviderRegistry`**（子命令的参数解析、装配与失败映射留在 bin）。理由**只有一条**：
**用例必须能注入一个持有夹具适配器的注册表**——夹具只能由测试代码构造，而生产装配点是空的
（§10.2）。这是本子项目需要库目标的**唯一**原因。

**原稿进库的理由已作废**：原稿写「为了让 trybuild 钉住『不收 `&AuthorizedTool` 就调不了』」。
随 §6.2 的改调，「唯一入口」在 C 的 crate 里，那条编译失败样例归 C；F 的库级用例只钉
「**F 确实经注册表调用、且被拒时零调用**」（第 9 节 P-1 / P-7）。

**代价**：bin 侧的 `tool_cmd` 要调 lib 的公开面；**`TaskError` 随之从 bin 移进 lib**（§3.2 的
「N-1 的处置」：共享函数与这个函数都要返回它，而 lib 叫不出 bin 的名字）。**原稿在此处写
「`TaskError` 仍在 bin」，那句话与本设计自己的 lib 化要求冲突，已订正**——变体一个不改，
只是定义位置搬家。这条代价是装配上的，不是语义上的。

## 6.5 决定 F9：扩位后的**请求侧**，F 传什么、从哪来（裁决 2）

**背景（用户 2026-10-05 拍板，见 `docs/superpowers/specs/2026-10-05-p3-bcdf-set-decisions.md` §一.2）**：
取 **(a) 扩 §316 的请求面**——`ToolProvider::invoke` 的**请求侧**要能携带授权/凭据。**这是动 P1 冻结的
接口**（§315 / §316 同属 `docs/02-工程.md` §10.3 的「必须先冻结」三组），故变更后 §10.3 的接口清单
要同步（§316 的**四项方法名不变**，变的是 `invoke` 的**参数类型**）。分工按裁决原文：
**C 设计那个位置**（它拥有 §315 / §316 的落地），**F 传**（它是工具调用路径的调用方），
**B 不受影响**（连接器侧的凭据交付走它自己的「逐次调用的适配器」，不经 §316）。

**为什么是请求侧而不是响应侧**：响应在**调用完成之后**才存在，往那里加位置无法把授权交进去——
与 `cancel` 那条既有裁决同一判据（set-decisions §一.2 原文）。**这条判断记在此，以免后来者
「顺手」把它挪到 `ToolResult` 上。**

**F 传什么、从哪来（本子项目认领的那一半）——传的是「整枚 `AuthorizedTool`」**：

- **F 把整枚 `&AuthorizedTool`（连同 `input`）交给 `ProviderRegistry::invoke_tool`**（§6.1 那条唯一入口）。
  **请求类型由 `invoke_tool` 在内部构造——F 的**生产路径**既不构造它、也不经它传值**（C 设计 §7.5
  定的是那个类型**长什么样**，§7.1 定的是**谁构造、经哪条路调**；F 只走 §7.1 那条路）。
  **但 F 的库级夹具会在签名里写到它**（plan-f 第 3 条，裁决）：夹具要 `impl ToolProvider`，而那个 trait 的
  `invoke` 形参就是它——**那是「被实现的公开 trait 的形状」，不是「F 自己造的形状」**，
  两者不是一回事，别混成一句（协调者已把它记为派单纪律）。
  **F 也不调 `AuthorizedTool::granted()`**，也不把授权值拆出来单独传一份（**取 `granted()` 的是适配器**，
  C 设计 §7.5）。
  **订正的来历（本节改过两轮，都留此）**：① 初稿写「F 从 `granted()` 取授权值、经请求侧传过去」——
  与 C §7.5 相抵，是**上游派单措辞**造成的；② 上一稿改成「F 用整枚值**构造请求类型**、调
  `provider.invoke`」——**仍偏了**：构造者是 `invoke_tool`，不是 F（同样是上游派单措辞引入的，
  协调者已两次认领）。**判据一句话：§7.5 定类型长什么样，§7.1 定谁构造、经哪条路调。**
- **为什么必须是整枚，而不是「F 拆出授权值再传」——这是本条的要害，不是风格**：
  - **工具 id 只由授权证明给出，没有第二个来源**（C 设计 §7.1 / §7.5）：请求里那个工具 id
    **来自 `AuthorizedTool`**，故「没有授权就构造不出这次调用」是**类型层保证**；
  - 「F 拆出授权值、另带一个工具 id」的形状里，**那份授权**与**被调用的那个工具**是两个可各自漂移的
    值——**授权了 t1、却调用 t2** 在这条形状里写得出来，而它**编译得过**：这是 **fail-open 的形状**，
    与本项目「消除 fail-open」的判据相抵（与 §6.1 的配对、`AuthorizedEffect` 的配对同一条判据）。
    **注意这条论证与「谁构造」无关**：构造者换成 `invoke_tool` 之后，它**更**成立——拆开的那条路
    在 `invoke_tool` 的签名上就连写都写不出来（它只收 `&AuthorizedTool` 与 `input`）。
  - **订正的来历（我的派单引起的，记此）**：本设计上一稿写的是「F 从 `granted()` 取授权值、经请求侧
    传过去」——**那句话与 C 的 §7.5 相抵**，是**上游派单措辞**造成的（协调者已认领），**以 C 的 §7.5
    为准**，本稿按它改写。
- **凭据（若这条调用需要）**：来源**只有一处**——`continuum_secrets::issue(&cap, now)`
  （P3A 设计 §5.1），其中 `cap` 取自这次调用已获准的某一枚能力。**本设计不自己造凭据，也不把凭据
  放进 `--input`**（§4.2 那条禁止照旧：`input` 是自由 JSON，往里塞凭据既违反 §51 也绕开类型）。
  **诚实标注**：把这后半句接上要新增 `continuum-runtime → continuum-secrets` 的边并加一个调用点，
  而本设计**今天不登记那条边**（§10.1：零使用的边即假边；`p3a-followups.md` 第三节末尾把登记时机
  记给「B 把密钥运行时接上的那个 task」）。故**本节只指名来源，不落这次接线**，把它记在 §14 第 10 条。
- **失败面不变（若该值缺位）**：**值缺位不新增 F 这一侧的任何一种 `Err`**，§8 的表**逐行照旧**。
  适配器若认为缺了授权/凭据而拒绝，报的是 C 的第二个臂 `ToolCallError::Provider(ProviderError)`，
  F 原样带出——**不在 F 侧另立「凭据缺失」之类的变体**（同一件事两个词汇表）。

**F 不设计那个位置的形状**（不预先发明）：字段名、是否可选、与 `input` 的先后，都是 C 的接口面；
F 只负责**把整枚 `AuthorizedTool` 放进构造处**。故本节的措辞是「F 传整枚值」，**不是**「请求类型长什么样」。

**本节的照片今天写不出来，据实记为待照片**（写法照 P3A 对 `Trust` 非 `Option` 的处置）：
它应当是「库级用例的夹具适配器断言**收到的那个请求里的 `AuthorizedTool` 与 `authorize` 给出的那枚
一致**（同一个工具 id）」，**而那个位置的类型还不存在**（C 尚未设计它），故无从写实参、也无从断言。
**这不是遗漏**：C 定下位置后，这条用例连同 P-7 一起补；在那之前，本节的保证**只由正文承载**。

---

# 7. 审计与 Journal

## 7.1 决定：F **不新增**任何审计行

`§313`（`docs/spec/05-normative.md:2132`）的八项里，与本路径相关的只有两项，各自由既有产生方覆盖：

| §313 的项 | 产生方 | 本子项目 |
|---|---|---|
| `capability grants` | `authorize` 成功时写**恰一条**（`registry.rs:133-140`） | **不补写**。驱动不得因为「想留个痕」再写一条——那会让同一次授权有两个审计产生点 |
| `external effects` | `effect` 表的**两个**写入函数各写一条：`record_planned` 一条、每次 `advance` 一条（`crates/continuum-effect/src/journal.rs:12-13` 的模块文档、`:35-36` 的 `audit`、`:59` 的 `record_planned`） | **复用**。工具调用的 `--effect` 走与命令路径同一套状态机，故审计行由同一个产生方给出 |

**故本路径的审计行构成可以逐条数出来**：一次声明了 k 条 `--effect` 且全部通过的工具调用，
写入的是 `1` 条 `capability grants`（`authorize`）与 `4k` 条 `external effects`——
每条效应**四次**：`record_planned` 一次（`PLANNED`），`advance` 三次
（`AUTHORIZED` / `EXECUTING` / 终态）。**「三次 `advance`」不是「三条」**：登记那一步也有一条，
这一点单看 `advance` 的调用点数不出来，故按产生方（两个写入函数）数。
**这个「4」不是本设计新算出来的**：`crates/continuum-effect/tests/persist.rs:252` 的既有断言注释已经
把同一条次数写在库里——「4 = 1 次登记 + 3 次成功的推进」（该处 `assert_eq!(audit_rows(&tx).len(), 4)`）。
**订正的来历**：本设计原稿写的是「每条效应三次 `advance` / `3k` 条」，**那句是假的**（漏了
`record_planned` 那一条）；交叉复审按上面那条既有断言抓出，本处按它订正，并把来历留在这里。

**这条要有照片，且要数出总数**：只断言「有一条 `capability grants`」的实现，多写一条**也会过**。
故用例读 `audit_log` 的 `kind` **列**（不是枚举转一圈），断言 k=1 时**恰好**五条、且 multiset 为
`{capability grants × 1, external effects × 4}`；k=0 时**恰好**一条（`capability grants`）。
**P-15 的期望值由此定；写错这个数会让用例必然失败**，故它与上面那条计数同源。
零效应那条同时钉住「**没有**凭据捏造的『tool invoked』审计行」——本子项目不新增 `AuditKind`
（八项是 §313 的封闭清单，加一项就是发明）。

## 7.2 工具调用**属于** Effect Journal（§267–§269）

**决定**：一次工具调用的 `--effect` 走**与命令路径同一套** Journal 写入：
`Effect` 记录体（§267，`crates/continuum-effect/src/effect.rs`）＋ `PLANNED → AUTHORIZED → EXECUTING`
→ `COMMITTED` / `FAILED`（§269）。

- **`id` 与 `idempotency_key` 同源**：复用 `effect_key(intent, spec)`（**锚点用函数名**），
  **不在本路径另派一次**（理由与 `effect_key` 自己文档里那条逐字相同：不给「这条记录是谁」立第二个来源）。
  **订正（2026-10-07，回灌 F Task 3 的实测）**：原稿在此引 `task_cmd.rs:512`（以及 `:499-504` 的
  文档）。**该函数与它的文档、单元用例随 F 的 Task 3 一并搬进 lib 的 `tool_call`**——本路径在 lib、
  定义在 bin 时前者调不到后者，就地再写一份就是同一件事两个产生点；`task_cmd.rs` 改为 `use` 它。
  故**锚点用函数名**，不再引 bin 的行号（`dfefb54^` 上它在 `task_cmd.rs:446`）。
- **`authorization` 字段**：复用同一编码形状 `approve=<bool>;policy=<裁决名>`
  （`authorization_field`，**锚点用函数名**；**订正 2026-10-07**：原稿引 `task_cmd.rs:537`，该函数与
  `effect_key` 同批搬进 lib 的 `tool_call`，见上一条——`dfefb54^` 上它在 `task_cmd.rs:472`），
  但**填的是这条效应自己的那次裁决**——即 §3.2 那个共享
  函数**成对返回**的 `Decision`（`(AuthorizedEffect, Decision)` 的后一项），而不是像 `task` 那样填集成
  那次裁决：**工具调用没有集成裁决这一回事**，凭空造一次（`effect_type` 缺省的裁决）就是一次没有意义
  的判定。**它也不是本路径第二次 `arbitrate` 出来的**——那正是 §5.1 禁止的第二个判定点；
  本路径**只**用共享函数返回的那一个（原稿写「逐字复用 `authorize_declared_effects`」，而那个函数
  把 `Decision` 丢掉，**那句话是假的**，见 §3.2）。**这是一处与命令路径的刻意不同**，
  记在此处与第 14 节第 6 条：两条路径写同一个字段的口径由此分岔，而该字段**只记录、不校验**
  （`task_cmd.rs:81-85`），今天没有任何生产代码读回它。
- **`parameters` 照 `task` 的先例写 `{}`**：§6.1 只说它是 JSON，**未规定内容**，本子项目不发明。
  故**工具调用的输入（`--input`）今天不落库**——这是一处据实的缺口，记在第 14 节第 6 条。
- **没有声明的效应就不碰库**：零 `--effect` 时不写任何 `effect` 行（与 `finish_declared_effects`
  的空集早退同理，`task_cmd.rs:486-488`）。

## 7.3 `recover` 的既有语义照旧覆盖本路径

进程若在步骤 5 的提交之后、步骤 7 的终态之前被杀，那些效应停在 `EXECUTING`，由既有的恢复钩子
转 `UNKNOWN`（§269；`recover_cmd` 注册的两个钩子之一）。**工具路径不新增恢复逻辑**：
「这条外部效应到底发生了没有」正是 `UNKNOWN` 的含义，而它比任何臆测都对。

---

# 8. 失败面（逐条：哪一种 `Err`）

「哪一种」指**变体**，不只指出口。所有变体都断言到最内层。

**本表在裁决 2（§6.5，请求侧扩位）之后逐行不变**——按裁决原文，F 那一半是「传」，不是「判」：
请求侧那枚授权/凭据**缺位**时不新增 F 侧的任何一种 `Err`；适配器若因此拒绝，报的是本表倒数几行
那条既有的 `ToolCallError::Provider(ProviderError)`，F 原样带出。**故 F 侧不新增「凭据缺失」变体**。

| 失败 | 出来的东西 | 库里的状态 | 照片 |
|---|---|---|---|
| 某条 `--effect` 的策略裁决铸不出能力（`RequireApproval` 无 `--approve`、或 `Deny`） | `TaskError::EffectNotAuthorized { effect, target, decision }`（**复用**，`task_cmd.rs:901`），报**声明次序第一条**铸不出的 | 零效应行、零审计（事务未提交） | P-1 |
| 某条 `--effect` 的幂等键已存在 | `TaskError::EffectAlreadyRecorded { key }`（**复用**） | 零新行 | P-2 |
| 工具未登记 | `TaskError::Capability(CapabilityError::UnknownTool { id })`（**新包装变体** `#[from]`，内层是既有变体） | 零效应行；`authorize` 拒绝不写审计 | P-3 |
| 出示了工具未声明的能力 | 同上包装 → `CapabilityError::UndeclaredCapability { kind }` | 同上 | P-4 |
| 缺工具声明的能力 | 同上包装 → `CapabilityError::MissingCapability { kind }` | 同上 | P-5 |
| 读登记项或写审计失败 | 同上包装 → `CapabilityError::Persist(PersistError)`（**两个产生方**，消息只说「持久化失败」，见其文档） | 零效应行 | P-8 |
| 能力已失效 | 同上包装 → `CapabilityError::Expired { expiry, now }` | — | **没有照片，见 §8.1** |
| 注册表里没有这个 id 的适配器 | `TaskError::ToolCall(C)` → `ToolCallError::Unregistered { id }`（**新包装变体** `#[from]`，内层是 **C 的**错误类型） | 各效应已写并记 `FAILED` | P-9 |
| 适配器报错 | 同上包装 → `ToolCallError::Provider(ProviderError)`（`ProviderError` 见 `crates/continuum-core/src/error.rs:4-15`） | 各效应记 `FAILED` | P-10 |
| 适配器返回 `is_error == true` | `TaskError::ToolReportedError { tool }`（**新变体**）；`output` 仍打到 stdout | 各效应记 `FAILED` | P-11 |
| 终态写入失败 | `TaskError::Persist`（既有的 `#[from]`） | 效应停在 `EXECUTING`，由 `recover` 转 `UNKNOWN` | **没有照片**（同 `task` 的对应路径，`task_cmd.rs:266-267` 已据实记过：DB 写失败时未设分支） |
| `--input` 不是合法 JSON | `CliError::InvalidToolInput { value, reason }`（**新变体**），解析期 | 未开库 | P-12 |
| `--effect` 无 `--intent`、或 `--intent`/`--approve` 无 `--effect` | `CliError::OptionRequiresEffect { option }`（**新变体**） | 未开库 | P-13（两向） |
| `--base` / `--exec` / `--apply` / `--sandbox` 给了 `tool` | `CliError::UnknownOption { name }`（**复用**） | 未开库 | P-14 |

**「新变体」的判据**：只新增**包装**变体（把既有 crate 的错误原样带出去）与**本路径独有**的失败
（工具自报失败）。**不新增**任何重复既有判断的变体——尤其**不新增「裁决为 `Deny`」这类
变体**（那是 `mints` 的判断，`crates/continuum-capability/src/error.rs:20-24` 已把这条判据说死），
也**不为「没有适配器」另立一个变体**：那条失败已有 C 的 `ToolCallError::Unregistered`（C 设计 §3），
F **包装**它而不另起一个词汇——同一件事两个变体正是本设计 §6.2 在讲的同一类毛病。
**原稿在此处写过 `TaskError::NoToolProvider { tool }`，该变体随 §6.2 的改调删除**，来历留此。

## 8.1 哪条失败路径**没有**照片，为什么

- **`CapabilityError::Expired`：F 自己那次 `authorize` 上不可达——但「本路径上不可达」为假，已收口。**
  **原稿写的是「`Expired` 经由本路径不可达」**，那句是**全称**，**在裁决 2 之后为假**：
  能力确实会**离开 F**（§6.5：F 把**整枚 `AuthorizedTool`** 放进 §316 的请求类型交给适配器，
  `granted()` 由适配器读、凭据签发在下游），而**它在适配器手里被取用的时刻落在 `invoke` 之内**
  ——`invoke` 的时长 F **没有上界**（一次工具调用可以跑任意久）。故「铸出 → 取料」之间**有间隔**，
  **这个间隔由 F 自己的交出动作引入**（§11 的「F ↔ B」行即此事的收口，
  与 B 设计 §11 第 18 条对齐）。
  **订正后的口径（这才是本节要留的那句）**：**F 的 `authorize` 那一次调用上 `Expired` 不可达**——
  步骤 3 铸出与步骤 4 校验之间**没有 I/O**、而 `now` 是**两次读时钟**（§3.2 的共享函数签名里没有
  `now`，铸币那处自己取一次、`authorize` 这处再取一次，**两次相差微秒级**；plan-f 第 7 条），
  而 `expiry` 在 15 分钟之后
  （`CAPABILITY_LIFETIME_MS`，`task_cmd.rs:459-463`），故**F 这一次调用里那一行比较会跑、
  但永不触发**（除常量取 0）——**注意这里的「永不触发」只覆盖 F 那一次调用，不覆盖整条路径**。
  **下游**（适配器取用 / 凭据签发）**才是它第一次可能真的判出过期的地方**。
  **照片**：F 这一侧**没有照片，也不该有**——为它造一条用例只能靠改常量或改时钟，那是把被测对象换
  成夹具；照片在 `continuum-capability` 自己的
  `tests/authorize.rs::an_expired_presented_capability_is_rejected`。
  **下游那一侧的首次可执行比较由 B 负责**（B 设计 §4.3 那句「第一次进入一条**会被执行**的比较」
  **按本收口保持原样，不降格**）。
- **终态写入失败**没有照片，理由与 `task_cmd.rs:266-267` 记的同一条（DB 写失败时 `remove_workspace`
  多半也失败，两条路自然收敛；造不出「已打开的连接写不进去」而不改夹具）。

---

# 9. 测试策略（照片表）

**库级用例**（`continuum-runtime`，夹具适配器经 C 注册表的登记入口装进去）：夹具**实现 §316 的 `ToolProvider`**，故它的 `invoke` 签名里会写下 `AuthorizedToolInvocation<'_>`——**这是被实现 trait 的形状，不是 F 的生产路径造的形状**（plan-f 第 3 条）；夹具记录它收到的那份请求。这是唯一能观察「工具有没有被调用」的地方——**生产装配点今天是空的**（§10.2）。
夹具适配器的形状照 `crates/continuum-provider/tests/fake_provider.rs`（那已是本仓既有的写法）。

| 编号 | 验什么 | 怎么验 |
|---|---|---|
| P-1 | 铸不出能力 ⇒ **工具一次都没被调用**、零效应行、零审计 | 空策略表；夹具适配器断言调用次数为 0；读 `effect` 与 `audit_log` 行数 |
| P-2 | 幂等键已存在 ⇒ 拒整条、零新行；**且第一条调用的记录里读得回它给的 `--intent`** | 先跑一次成功调用（带 `--intent i1`），再跑同一条声明；**读回第一条的 `effect.idempotency_key`，断言它带有长度前缀 `<意图字节长>:<意图>:` 且该段就是 `i1`**（键形见 `effect_key`；原稿引 `task_cmd.rs:512-520`，该函数随 F 的 Task 3 搬进 lib 的 `tool_call`，见 §7.2）。**这一读是 §2.2 那条「`--intent` 可观察」的照片**：没有它，「条件必填」的理由就只是一句话 |
| P-3 | 未登记的工具 ⇒ `UnknownTool`，工具未被调用 | 库里有 `tool` 表但无该 id |
| P-4 | 出示了未声明的能力 ⇒ `UndeclaredCapability` | 登记项声明能力 A，声明 `--effect` 铸出能力 B |
| P-5 | 缺声明的能力 ⇒ `MissingCapability` | 登记项声明 A+B，只声明铸出 A 的那条 `--effect` |
| P-6 | **F 确实经注册表调用**（不是自己另开一条路） | 登记一个夹具适配器，放行后断言它**收到了一次调用**；与 P-1 互为对照臂（见末尾的变异说明）。**「唯一入口」那条类型层保证的照片归 C**（C 设计 §7.1），F 不重复 |
| P-7 | 工具 id **由 `AuthorizedTool` 定**（配对） | 授权 t1 后调用，夹具断言**它收到的那份请求里的工具 id 是 t1**（该 id 由 `invoke_tool` 从授权证明取出，C §7.1）；另一条断言请求里承载的 `input` 与 `--input` 逐字相同。**原稿按已删类型写「`call.tool`」，已随 §6.5 改**（plan-f 第 2 条） |
| P-8 | 登记项被写坏 ⇒ `CapabilityError::Persist` 原样带出 | 照 `tests/authorize.rs` 的 `insert_bad_capabilities` 写坏 `required_capabilities` 列 |
| P-9 | 注册表里没有该 id ⇒ 效应记 `FAILED`、报 `ToolCallError::Unregistered` | 注册表为空（不登记任何适配器） |
| P-10 | 适配器报错 ⇒ 效应记 `FAILED`、`ToolCallError::Provider` 原样带出 | 夹具返回 `Err(ProviderError::Transport)` |
| P-11 | 适配器自报失败（`is_error`）⇒ 效应记 `FAILED`；**「stdout 仍有 output」这一半没有照片**（plan-f 第 4 条） | 夹具返回 `is_error: true`，断言效应记 `FAILED` 与 `TaskError::ToolReportedError`。**「stdout 仍有 output」这一半拍不到**——库级用例到不了那一跳、库函数不交出该值、且 `println!` 被测试框架捕获；**据实记为无照片，不许为它造夹具** |
| P-12 | 非法 `--input` ⇒ 解析期 `Err` | 端到端，`--input '{'` |
| P-13 | 选项组两向都拒 | 两条端到端用例（见 §2.2） |
| P-14 | `tool` 不认识的选项 ⇒ `UnknownOption` | 逐条：`--base` / `--exec` / `--apply` / `--sandbox`（**四条各一**：四个是四条独立的分支，抽一个代表不算） |
| P-15 | **审计行的总数与构成** | 读 `audit_log.kind` 列，k=1 时 multiset 恰 `{capability grants×1, external effects×4}`（`record_planned` 一条 + `advance` 三条，同 `continuum-effect/tests/persist.rs:252` 的既有计数），共五条；k=0 时恰一条 |
| P-16 | 零 `--effect` 的工具调用跑得通、不碰 `effect` 表 | 登记项声明空能力表；断言 `effect` 零行、`audit_log` 一条 |
| P-17 | 零效应的正向对照：同一条调用在策略放行时**真的被调用** | 与 P-1 同一夹具，只差库里有没有那条 `Allow`（互为对照臂，与 `capability_gate.rs` 的既有手法同形） |
| P-18 | **作用域原样带出**（§4.3 的后半句） | 端到端：声明两条 `--effect`（目标各不相同），跑通后读 `audit_log` 的 `capability grants` 行，解析 payload，断言 `capabilities[].scope` 的 multiset **逐个等于**各 `--effect` 的目标。**把 `spec.target` 换成常量、或换成 `effect_type` 的字面串，本条即红** |
| P-19 | **射程边界**（§12.1）：命令路径不过强制点 (1) | **端到端，跑在 `task` 子命令上**：`task --effect charge:x --exec true`（策略放行、退出码 0）之后，断言 `SELECT COUNT(*) FROM audit_log WHERE kind = 'capability grants'` 为 **0**。**把强制点 (1) 接到命令路径上，本条即红**。这是本表唯一一条跑在**另一条子命令**上的用例——它钉的正是那条边界 |

**变异须真落到实现体**（本仓既有纪律）：P-1 的目标断言是**调用次数为 0**，故把「拒绝」的变异体
放在铸造之后、那一跳之前——若把变异体放在 `authorize` 之后（例如把 `presented` 换成空集），
P-1 仍绿而 P-5 会红：**两个变异体落在不同的用例上，故 P-1 与 P-5 都要在**，不能拿一条代表另一条。
**P-6 与 P-1 也互为对照**：一个「永不调用」的实现让 P-1 全绿而 P-6 红——这正是 §6.2 改调之后
「F 有没有真的把 `AuthorizedTool` 交到注册表那一跳」唯一剩下的可观察面。
P-15 的目标是**行数与构成**，等价变异体（改 payload 内容而总数不变）由 `continuum-capability` 自己的
payload 用例覆盖，本路径不重复。

---

# 10. 依赖边与装配

## 10.1 不新增任何 crate 边

`continuum-runtime` 的 `ALLOWED` 条目（`crates/continuum-runtime/tests/dependency_direction.rs:123-138`）
里**已经有**本子项目要用到的全部内部 crate：`continuum-capability`（`AuthorizedTool` / `authorize` /
`AuthorizedEffect` / `mint` / `CapabilityKind`）、`continuum-core`（`ToolId` / `ToolResult` /
`ProviderError`——**`ToolInvocation` 已由裁决 §五删除**）、`continuum-provider`（**`ProviderRegistry`**
——F 的生产路径只调它的 `invoke_tool`、**不构造也不传请求类型**；`ToolProvider` 与
`AuthorizedToolInvocation<'_>` 亦在此 crate，**后者由库级夹具 `impl ToolProvider` 的签名用到**）、`continuum-policy`（沿用）、
`continuum-effect`（沿用）、`continuum-persist`（沿用）。**故不改 `ALLOWED` 表**（那张表记的是**内部**
crate 边）。**`Cargo.toml` 的射程要限定**（plan-f 第 8 条）：内部依赖清单不动，但**要加 `async-trait` 的 dev 边**
（库级夹具实现 `#[async_trait]` 的 `ToolProvider` 需要它）——它是**外部** crate，故 **`ALLOWED` 不受影响**。
**原稿在此处写「也不改 `Cargo.toml`」，那是过宽的说法，已限定。**

**订正（2026-10-07，回灌 F Task 3 的实测）——本行原写「故 `ALLOWED` 与 `Cargo.lock` 都不受影响」，
「`Cargo.lock` 不受影响」那一半是假的**：**dev 边也写进锁文件**——`Cargo.lock` 里
`[[package]] continuum-runtime` 的 `dependencies` 列表**会多出一行 `"async-trait"`**（该**包**本身早已
在锁里，多的是这条**边**）。**实测**：Task 3 的提交 `dfefb54` 对 `Cargo.lock` 的改动是 **`1 +`**，
正是这一行；故实现计划的 Task 3 Step 11 **必须把 `Cargo.lock` 一并 `git add`**（否则锁与
`Cargo.toml` 不一致、锁过期）。**旧话留此。**（同一条假句子在设计 §3.2 与实现计划的 Global Constraints
里各有一处，已同批订正。）

**但有两处注释会变成假的，必须就地订正**（本仓的既有做法：订正时把错误说法的来历留在原地）：
`dependency_direction.rs:121` 写着「core / events / provider 在 runtime 内**至今无任何引用**」
——本子项目第一次真的引用 `continuum-core` 与 `continuum-provider`（`events` 仍无）。
**实现时必须把那半句改掉并注明是哪一次改的**，否则下一轮的扫查会照着它得出错的结论。

**`continuum-secrets` 的边不在这里加**：凭据签发是子项目 B 的事，`continuum-runtime` 今天对它零引用，
按「零使用的边即假边」（P2b 为此删过两条）不登记。`p3a-followups.md` 第三节末尾已点为「登记它的时机
是 B 把密钥运行时接上的那个 task」，本子项目**不提前登记**。

**`tokio`**：`continuum-runtime/Cargo.toml:27` 已声明它，而 `src/` 与 `tests/` 至今**零引用**
（`grep -rn tokio crates/continuum-runtime/` 只命中 Cargo.toml 那一行）。本子项目为
`block_on` 异步的 `invoke_tool`（C 的注册表方法，其内层是 `async_trait` 的
`ToolProvider::invoke`，`crates/continuum-provider/src/tool.rs:8`）**第一次真的用上它**：
单次调用用当前线程运行时（`Builder::new_current_thread().build()`），在那一跳前后存活。
**这一条要记**，否则后来者会以为这条依赖一直是活的（与 `CAPABILITY_LIFETIME_MS`
那条「不可观察的常量」同一类提醒）。**不新增 `futures` 之类的执行器**：多一个执行器就是同一件事的
第二个来源。

## 10.2 组合根：把适配器**登记进 C 的注册表**

**本子项目不建第二个装配点**（原稿的 `select_tool_caller()` 已随 §6.2 删除）。工具侧的装配是
**C 的注册表的登记入口**：组合根（就是驱动自己——C 设计 §7.1 末段点名的 composition root）
在启动时把适配器登记进去。

**登记入口的形状（照 C 的契约，不自己发明签名）**：C 设计 §3.1 定的是「**按 id 显式登记**，
装配期填装」——**id 由登记方给出，注册表不去问适配器**；注册表内部持
`ToolId → Arc<dyn ToolProvider>`（C 设计 §3.1）。故驱动侧的那一句形如
「**把这一组 id 登记给这个适配器**」，适配器以 `Arc` 交出。**本设计不写出具体函数名与实参表**：
C 尚未冻结那个方法的签名（C §12 第 9 条把它记为待定），本子项目**不预先发明**——写死的签名一旦与
C 不一致，就是同一件事的第二个形状。

**原稿在此处给过一个具体 sketch**（`registry.register_tool(Box::new(MyAdapter::new()))`），
**它有两处不符 C 的契约**，已删除并把来历留此：**(a)** 没有给出工具 id（C 要求 id 由登记方显式
给出，注册表不询问适配器）；**(b)** 用的是 `Box<dyn ToolProvider>`，而注册表内部持
`Arc<dyn ToolProvider>`。**今天的驱动没有这一句**（没有任何适配器可登记）。

- **今天没有任何适配器可登记**：注册表是空的，故步骤 6 那一跳返回
  `ToolCallError::Unregistered`。**它不挡强制点 (1)**：`authorize` 在步骤 4，早于那一跳（§3.4）。

  **订正（2026-10-07，F Task 3 评审翻出「设计与 brief 相抵」）**：本行原写「**这不是遗留物**——它是
  **C 的交付缺口**，记在第 14 节第 2 条」。**那句是过期读数，已按 C 的中立性规则订正。旧话留此。**
  **核过的事实（三条，逐条给出处）**：
  1. **C 已交付的是「注册表机制」（登记的入口），不是「适配器」**：注册表按 id 显式登记、装配期
     填装（C 设计 §3.1），入口存在且可用。
  2. **C 按中立性规则不会在自己 `src/` 里提供 `impl`**：C 设计 §4.1 的表把「实现（适配器）」一行的
     crate 记为「**未来的 `continuum-adapter-*`，本阶段不建**」；§4.1 末段与 §4.2 形态 4 把「把实现
     写进 `continuum-provider` 自己内部」列为**一片全绿的违规路径**，并有一条**可执行的模块面守卫**
     钉它——`crates/continuum-provider/tests/neutrality.rs` 的
     `the_neutral_crate_contains_no_implementation` 断言该 crate 的 `src/` 下不出现
     `impl ModelProvider for` / `impl ToolProvider for` / `impl Connector for`（带正控制
     `the_guard_sees_the_source_tree`，防「什么都没读到」的假绿）。
     **说准一层**：这**不是「类型上写不出来」**，而是**一条规则加一条可执行的守卫**；且该守卫是
     **下界**（按文本匹配，全限定路径 / `use … as` 别名 / `include!` / `macro_rules!` 的 `$trait`
     仍逃逸，见 C 设计 §4.1 末段与 `neutrality.rs` 文件头那张**非穷尽**的清单）。
  3. **装配者是驱动自己**：C 设计 §7.1 末段点名 composition root 是驱动，装配期由它构造并登记适配器。
  **故「生产装配点为空」的收件人不是 C，而是负责装配的那一条流**——**F（驱动）自己**，准确说是
  **将来的适配器子项目 / 下一轮驱动**：由它提供一个 `ToolProvider` 实现、经注册表的登记入口装进
  组合根。**F 本轮不做这一步**（本设计明写不建适配器），只把它记成具名未决（第 14 节第 2 条）。
- **F 不再声明「交出什么」**：注册表**不交出**适配器（C 设计 §7.1 的裁定），F 不需要、也不得
  再包一层。**原稿在此处写的「只交出 `ToolCaller`、不交出裸的 `Box<dyn ToolProvider>`」作废**，
  那句保证现在由 C 的注册表承担（来历见 §6.2）。

---

# 11. 与其它子项目的接口（各自的义务，一句话一条）

| 子项目 | 本设计要它做的 |
|---|---|
| **C** | ① **工具侧唯一的调用入口**（`ProviderRegistry::invoke_tool(&AuthorizedTool, input)`，且不交出适配器）由 C 维持——**F 调用它，不再自建入口**（§6.1、§6.2）；② **裁决 2**（`…set-decisions.md` §一.2）：**C 设计 §316 请求侧那个承载授权/凭据的位置**（F 只负责传，见 §6.5；**位置在请求侧、不在响应侧**）；③ **`Tool`（§252）与 `ToolDescriptor`（§316）并存，不合并**（C 设计 §8）——本路径**读 `tool` 表**、适配器另出 `ToolDescriptor`，两者靠共用的 `ToolId` 绑定；`input_schema` 有两个产生点一事按 C §8 的裁定处置（**调用的权威是适配器的 `ToolDescriptor.input_schema`**，`Tool.input_schema` 是规划快照），本路径**不消费**其中任何一个；④ 注册表的**登记入口**（组合根用它把适配器登记进去，§10.2）与 `ToolCallError` 的两个臂（§8 的 P-9 / P-10 依赖它们）。**第 5.2 节的登记项不变量**不在 C 这边——协调者本轮把它判给 `continuum-capability` 的 `save_tool`（「谁写库谁把关」），见 §5.2 与第 14 节第 7 条 |
| **B** | ① 按能力**逐枚**签发凭据（§51、P3A 设计 §5）：作用域的强制**在那一侧**，本路径只把 scope 原样带进 `AuthorizedTool`（§4.3）；② 说清它凭什么认为收到的 `AuthorizedEffect` 经过了校验（P3A 设计 §10 第 10 条）；③ **`granted()` 的调用方不是 B、也不是 F**——按 C 设计 §7.5，**取 `granted()` 的是适配器**；F 只把**整枚 `AuthorizedTool`** 放进 §316 的请求类型（§4.2、§6.5），B 消费的仍是 `AuthorizedEffect::capability()`。**B 不受裁决 2 影响**（连接器侧的凭据交付走它自己的「逐次调用的适配器」，不经 §316，见 `…set-decisions.md` §一.2） |
| **`continuum-capability`** | **登记项的合法性由它把关**：在 `save_tool`（`crates/continuum-capability/src/persist.rs:64`，`tool` 表**唯一**的生产写点）里强制第 5.2 节的不变量 `effect_class == Some(t)` ⇒ `for_effect(t)` ∈ `required_capabilities`。**协调者本轮拍板**（原稿把这条记在 C 名下，已订正）：谁写库谁把关——C 那边不必知道这条规则 |
| **G（模型调用路径）** | **不是本子项目，本表对它无要求**——此处只把名字钉住，免得再被混称。**F 是工具调用路径**（`authorize` → `ToolProvider::invoke`，过强制点 (1)）；**G 是模型调用路径**（`ModelProvider::invoke` / `stream`，**没有任何强制点**）。用户 2026-10-05 拍板新开 G（`…set-decisions.md` §一.1），并明令 **「G 不得借用 F 的名字」**。**故本设计与 F 的收件人一律写 F；凡模型侧的义务（取 `ProviderHealth` 快照、调 D 的 `rank`、消费 `RankedExecutionCandidates`、定义 `usage()` 语义、模型侧失败分类）收件人写 G，不写 F。** |
| **B ↔ F：「铸出 → 取料」之间有间隔，故 `Expired` 的首次可执行比较在下游**（对齐 B 设计 §11 第 18 条；B 要 F 给出这条事实并二选一收口，**F 据此选 (i)**） | **事实（F 这一侧的实况，B 看不到的那一半）**：**有间隔**，而且**由 F 自己的交出动作引入**——按裁决 2，F 在步骤 4 铸出并发给 `authorize`，随后把**整枚 `AuthorizedTool`** 放进 §316 的请求类型交给适配器（§6.5），而**取用它的是适配器**（`granted()`，C 设计 §7.5）；这次取用落在 `invoke` **之内**，而 `invoke` 的时长 **F 没有上界**（一次工具调用可以跑任意久，F 也没有超时规定）。凭据签发更是**下游**且**本设计未接线**（§14 第 10 条）。<br>**故选 (i)：F 原稿「`Expired` 经本路径不可达」那句全称为假，F 已改结论**——订正后的口径是 **「F 的 `authorize` 那一次调用上不可达」**（步骤 3 与 4 之间无 I/O、`now` 同一步、`expiry` 在 15 分钟后 ⇒ 那一行**比较会跑但永不触发**），**F 不再主张整条路径上不可达**（§8.1）。<br>**由此 B 那句不必降格**：B 设计 §4.3 的「第一次进入一条**会被执行**的比较」**保持原样**——它说的正是下游那一次，而它现在成立。 | 

**D 行已删（原稿在此处有一行，记 D 把驱动职责记到「子项目 F」名下）**：那条所指的 D 文本**已被 D
自己订正**——D 的 §1.2 现在题为「调用方**按角色**指名，**不记在子项目 F 名下**」，§7.1 同。
故本设计**不再对 D 提任何要求**（**订正的来历留此**，免得后来者按旧文本再提一次）。
**「执行侧」这个旧称已由裁决 1 落成具名子项目 G**（`…set-decisions.md` §一.1）：凡 D/C 把义务指向
「执行侧」「消费 `RankedExecutionCandidates` 的那个子项目」的，收件人一律写 **G**——**G 不是 F**
（F 走工具、G 走模型），本表已按此加 G 行。
`ToolProfile` 的 `cost` / `latency` / `trust` 仍服务 D 的候选排序（P3A 设计 §3.1），本路径**不读**。

**规范侧的订正已完成，本表不对它们提要求**（`…set-decisions.md` §二）：`docs/02-工程.md` §4.1 的
Connector 一行移入 §5.1、§9.1 写出箭头读法并补 `资源层 → 边界层` 的边、§4.3 补 Connector 与
`ToolProvider → 被工具调用路径调用` 两行、§10.3 标注 §316 请求面将扩。**这四处是规范与 B/C 的事**，
本设计只在 §6.5 认领「请求侧传值」那一半；**原稿若曾就它们提过要求，随本条一并关闭**。

---

# 12. 完成判据

《工程》§4.4 的第一条（`docs/02-工程.md:261`）：「工具调用前校验 Capability，且 Capability 不可与
裸字符串互换」。本子项目承担的是**前半句在生产路径上落地**：

- **「工具调用前校验」**：由步骤 4（强制点 (1)）与步骤 6（那一跳）之间的次序承担，
  照片是 P-1（拒时零调用）与 P-17（放行时真的调用）这对**对照臂**——单有 P-1 时，一个永不调用的
  实现全绿；单有 P-17 时，一个从不校验的实现全绿。
- **「Capability 不可与裸字符串互换」**：P3A 已交付（四份 `tests/compile_fail/` 样例），
  本子项目**不重复**。

**本子项目额外承担的一条**：强制点 (1) **有生产调用方**（`p3a-followups.md` 第一节那格的兑现）。
判据是 `grep` 全仓：`continuum-capability` 之外出现生产调用点（本 crate 的 `tool_cmd` / lib），
且该调用点**在正常路径上**（不是只有测试才走到）。**故 §3.4 那条次序（做事排在强制点之后）
是这条判据的前提**——若把那一跳排在 `authorize` 之前，今天这个「注册表是空的」世界里
`authorize` 永远不会被执行，判据就退化成一个只有测试走得到的调用点。这条逻辑链写在这里，
以免后来者顺手把两步对调。**这条判据的限度**：它说的是「步骤 2、3 不先拒时会走到」，
不是「每次调用都走到」——第 2、3 步各自的拒绝路径（P-1 / P-2）本就在强制点之前。

## 12.1 强制点 (1) 的**射程边界**（免得上面那段被读成全称）

上面说的是「**工具调用**这条路径上，调用前校验落地了」。**它不是「任何会造成外部效应的驱动行为
都要过强制点 (1)」**：`continuum task --exec git push …` 仍然是一条能造成**同一个**外部效应
（推一个分支）的路径，而它**只过强制点 (2)**（`task_cmd.rs` 第 4 步逐条 `--effect` 铸能力），
**不过强制点 (1)**——它根本没有工具 id，没有可交给 `authorize` 的东西。
**这不是缺陷**：命令不是工具调用（§87 管的是「Agent 调用工具」，而命令是调用方给的 argv），
两者由**不同的**强制点覆盖（§4.2 的三个强制点里，命令路径走 (2)，工具路径走 (1)+(2)）。
把这条边界写出来，是因为不写的话 §12 会被读成「所有外部效应都已过 (1)」，而那是假的。

**它有一张否定式照片（P-19），原稿写「没有照片」是错的**，订正来历留此：否定命题在本仓有可拍的
形态——**行数不变量**。`capability grants` 这个 `AuditKind` 的**唯一产生方是 `authorize`**
（§7.1 的表：本路径不补写，命令路径不写它），故「命令路径不经过强制点 (1)」可以拍成：
跑 `task --effect charge:x --exec true`（策略放行、命令成功），断言
`SELECT COUNT(*) FROM audit_log WHERE kind = 'capability grants'` 为 **0**。
**把强制点 (1) 接到命令路径上，这条即红。** B 的「不写审计」照片（`…p3b-…`）是同一形状
（行数不变量），本仓接受这种拍法。

---

# 13. 不能拍照片的东西（集中列出）

1. §6.3 的两条**限度**（装配者手里有裸适配器，可以直接调 `invoke`；适配器可以无视收到的 id）：
   关于「别处能写出什么」的命题，本仓用例证明不了。**「唯一入口」那条保证的照片在 C 那边**
   （C 设计 §7.1），F 不重复。
2. §4.3 的**作用域**：**判定**那一半本路径不产生任何可观察量，故没有照片；**原样带出**那一半
   **有**照片（P-18）——这条不许再被读成「§4.3 整节没有照片」。
3. §4.1 的「`presented` 与 `AuthorizedEffect` 同源」：构造点的选择，只由评审维持。
4. §8.1 的两条没有照片的失败路径（**`Expired` 只在 F 那次 `authorize` 上不可达**——它在**下游**
   （适配器取用 / 凭据签发）**可能真的触发**，那部分不归 F 拍；终态写入失败不可造）。
5. §5.2 的**登记项不变量**：它今天没有强制点（`save_tool` 里还没有这条检查、其生产调用方也不存在），
   故也没有照片；**它的落点已定为 `continuum-capability` 的 `save_tool`**（第 14 节第 7 条）。
6. **步骤 6 那一跳在生产路径上今天必然失败**（注册表为空 ⇒ `ToolCallError::Unregistered`），
   故「成功调用」在生产里不可达；它的照片全部来自库级用例里经注册表登记的夹具适配器。
7. §6.5（裁决 2 的**请求侧传值**）：**照片待 C 定下那个位置的类型**——今天写不出实参，
   故这条保证暂时只由正文承载（理由与替代处置写在 §6.5 末段）。
8. §3.3「结果打到 stdout」与 P-11 的「stdout 仍有 output」：**这一半拍不到**（plan-f 第 4 条）——
   库级用例到不了那一跳、库函数不交出该值、`println!` 又被测试框架捕获。**据实记为无照片，不许为它造夹具**。

---

# 14. 遗留与未决项

**收件人逐条给出**（P3A 的交接文件体例）。

1. **工具登记今天没有生产调用方**：全仓 `save_tool` 只在 `continuum-capability` 的测试里被调
   （`tests/authorize.rs`、`tests/persist.rs`），生产代码零调用。故 `tool` 表在真实库里是空的，
   `tool` 子命令对任何 id 都报 `UnknownTool`，**除非先经 `save_tool` 登记**。**登记项落 `tool` 表、
   `ToolDescriptor` 由适配器另出、两者靠共用的 `ToolId` 绑定**（C 设计 §8 判**不合并**——合并会把
   `required_capabilities` / `effect_class` 这些治理数据交给被治理方，那是强制点 (1) 的语义反转）。
   **原稿在此处写「登记要等 §316 的 `ToolDescriptor` 与 §252 的 `Tool` 合并」，那句话与 C 的裁定
   相反，已按 C 订正**，本子项目**不发明**一个登记用的 CLI。
   **收件人收紧（此处有两个「不同的登记」，原稿把它们混在一句里）**：
   - **`tool` 表的登记**（`save_tool` 写的一行）：**无生产调用方**，**收件人是 `continuum-capability`**
     ——它与第 7 条的「登记项合法性」是同一侧（谁写库谁把关）；**本设计不发明登记用的 CLI**，
     故 F 侧不缺东西。
   - **适配器登记**（C 注册表里 `ToolId → Arc<dyn ToolProvider>`）：**要分两件事**——**登记的
     *入口***（那张表与它的登记方法）是 **C 的交付**，**已交付**；C §7.4 接缝二说的正是这一条，
     **与上面那条不是一回事**。而**「谁来登记」**（提供一个 `ToolProvider` 实现、并调那个入口）
     **不是 C**——是负责装配的那一条流，见下面第 2 条与 §10.2。
     （**订正 2026-10-07**：原稿在此写「**收件人是子项目 C**」，把「入口归 C」与「实现归谁」混成
     一句；**旧的单句写法留此**。）
   旧稿把二者写成一句「收件人：子项目 C（登记入口）」，**那是含糊，已按本段拆开**。
   另：C §8 记的那处 `input_schema` 双产生点，**调用的权威是适配器的 `ToolDescriptor.input_schema`**，
   C 设计 §12 第 5 条把这条一致性缺口的收件人记为 F/D，**本路径不消费任何一份 `input_schema`**，
   故此处只登记、不认领。
2. **注册表里没有任何工具适配器**：`ToolProvider` 的实现在**生产代码里一个都没有**——
   `crates/continuum-provider/src/` 下零 `impl`，由中立性守卫
   （`tests/neutrality.rs::the_neutral_crate_contains_no_implementation`）钉住；**实现在 `tests/` 下**
   有几份夹具（**实测 2026-10-07**：`tests/common/mod.rs` 的 `FakeTool` / `RecordingTool` 两份、
   `tests/registry_tools.rs` 两份；`tests/fake_provider.rs` 自己只 `mod common;` 再引用它们）。
   故组合根（§10.2）登记不出东西，步骤 6 那一跳在生产里必然返回 `ToolCallError::Unregistered`
   （§3.4、第 13 节第 6 条）。
   **订正（2026-10-07，两处，旧话留此）**：
   - **收件人**：原写「**收件人：子项目 C。**」——**过期读数**。收件人是**负责装配的那一条流**：
     **F（驱动）自己**，准确说是**将来的适配器子项目 / 下一轮驱动**：由它提供一个 `ToolProvider`
     实现、经注册表的登记入口装进组合根。**它与 C 的中立性规则相抵**（C 不会在自己 `src/` 里提供
     `impl`——C 设计 §4.1 表把实现一行记为「未来的 `continuum-adapter-*`」，且
     `tests/neutrality.rs` 有守卫钉 `src/` 下零实现）；**C 已交付的是登记的入口**（§14 第 1 条）。
     依据逐条见 §10.2 那条订正。
   - **实现的份数**：原写「`ToolProvider` 的**唯一实现**是
     `crates/continuum-provider/tests/fake_provider.rs` 里的夹具」——**两处都不准**：实现**不止一份**，
     且 `fake_provider.rs` 里原本那两份**已移入 `tests/common/mod.rs`**（该文件今天只 `mod common;`）。
     **承重的那半句仍然成立**：**生产代码里一个都没有**。
3. **`AuthorizedTool::granted()` 的消费方：按 C 设计 §7.5，是「适配器」。**
   **三种口径的来历全部留此**（本节改了三次，免得后来者按任一旧版改回去）：
   ① 原稿把它指派给**子项目 B**——**那句没有依据**（B 设计全文不含 `granted`；
   B 消费的是 `AuthorizedEffect::capability()`，B 设计 `:234-244` 一带）；
   ② 订正后记为「**悬空**」（本路径只读 `tool_id()`、C 的 `invoke_tool` 也只读 `tool_id()` 去路由，
   C 设计 §7.1）；
   ③ 本稿按 **C 设计 §7.5** 落定：**取 `granted()` 的是适配器**——**F 不调它**，
   F 只把**整枚 `AuthorizedTool`** 放进 §316 的请求类型（§4.2、§6.5）。
   **「谁在哪一行调它」以 C §7.5 为准**：协调者已要 C 把那句写死，**本设计等那句同步，不自行发挥**。
   （P3A 那条既决「按出示顺序照录、保留重复项」**不受影响**，且**正是**适配器读它时不自行挑选的理由。）
4. **六个 kind 没有策略事实**（`Filesystem(Read|Write)`、`Git(Read|WorktreeWrite|CommitLocal)`、
   `Github(CreatePr)`）：`PolicyContext.effect_type` 只有 `EffectType`，故驱动**铸不出**这六种能力，
   含它们的工具在本路径上**一律** `MissingCapability`（§5.3）。要开这条路，须给 `PolicyContext` 加一条
   能力事实并定「非效应类能力由谁判准」——本子项目无权处置。
   **对登记面的后果**：今天**可以**登记一条这样的工具（`required_capabilities` 收十二个 kind 全合法），
   而它**永远调不动**；本设计**只在正文写明、不在登记期拦**（理由见 §5.3：本层不该在别人的入口上装闸
   ——而按第 7 条，那个入口现在是 `continuum-capability` 的 `save_tool`，「是否要拦」同样归它）。
   **收件人：策略层（`PolicyContext` 的事实集合）与子项目 D（能力需求侧的来源）；
   「登记期是否要拦」归 `continuum-capability`（第 7 条）。**
5. **本路径不判作用域**（§4.3）：`authorize` 只比 kind，作用域随能力原样带出（**带出**那一半有照片，
   P-18；**判定**那一半没有）。这是本层的既有边界，强制落在凭据签发（§51）与执行点。
   **收件人：子项目 B**（作用域的**判定**）。
6. **两处与命令路径的刻意分岔**，都记在此以免后来者按 `task` 的写法「顺手统一」：
   (a) `effect.authorization` 字段在本路径上写**这条效应自己那次裁决**，而 `task` 写的是**集成那次**
   裁决（§7.2）——因为工具调用没有集成裁决；(b) 本路径的**工具调用输入（`--input`）不落库**：
   §267 的 `parameters` 语义未规定，本子项目照 `task` 的先例写 `{}`（§7.2）。**收件人：后续阶段**
   （与 §312 Execution Trace 的落点一起定，见第 9 条）。
7. **登记项不变量没有强制点**（§5.2）：`effect_class == Some(t)` ⇒ `for_effect(t)` ∈
   `required_capabilities`。它是堵住「工具做了外部效应却无效应记录」这条洞的那块东西，
   **其落点已由协调者本轮拍板定为 `continuum-capability` 的 `save_tool`**（`tool` 表唯一的
   生产写点；原稿记的是 C，已订正）；而 `save_tool` 的生产调用方今天不存在（第 1 条），
   故**这一步的实现就是「在 `save_tool` 里加这条检查」**。**在该检查落地之前，那条洞是开着的。**
   **收件人：`continuum-capability`**（不再是子项目 C）。
8. **工具调用不建 Task 工作区**（§3 的决定 F8）：若某个工具**确实需要**一个 Task 工作区，
   §316 的**请求侧**（`invoke_tool` 交给适配器的那份）里没有把工作区交给适配器的通道。**收件人：子项目 C**（改 §316 的调用面
   是 C 的活），并与第 10 条同源。
9. **§312 的「tool invoked」没有落点**（`docs/spec/05-normative.md:2116`）：全仓没有 Execution Trace
   的设施（`grep -rni trace crates/` 只命中 `continuum-artifact` 的两个无关文件）。本子项目**不发明**
   一张 trace 表，也不新增 `AuditKind`（§313 的八项是封闭清单）。故今天「某次工具调用被发起」这件事
   只在**有 `--effect` 时**经效应记录可查，零效应的调用不留痕。**收件人：后续阶段**（与 §312 / §317 的
   落点一起定）。
10. **`AuthorizedEffect` / 授权的交付通道：已裁 (a)，不再是未决项。**
    **问题（留档；当时的类型现已不存在）**：`ToolProvider::invoke` 当时收的是 `ToolInvocation`
    （tool id + input），**没有携带授权或凭据的字段**——该类型已由裁决 §五删除，请求侧换成
    一个只装得下「已授权」的请求类型（C 设计 §7.5，由 `invoke_tool` 构造）。故若某工具的外部效应
    最终由 B 的连接器执行，**驱动无从把授权交给它**——「收下 `AuthorizedEffect` 才能做副作用」
    这条义务（共享面第四节第 2 条）在这条边界上**没有交付通道**。最吃紧的是**凭据类的工具**
    （`Email.Send`、`Payment.Charge` 这一类）。
    **裁决（用户 2026-10-05 拍板，`…set-decisions.md` §一.2）：取 (a)——扩 §316 的请求面。**
    - **被否掉的 (b)**（本阶段声明工具不接收凭据）**代价是实的**（需要凭据的工具工作不了），
      用户没取它；**原稿把 (a)/(b) 并列留待裁定，现已由上裁落定**，来历留此。
    - **这是一次**冻结面变更**，已记录在案**：§315/§316 同属 `docs/02-工程.md` §10.3 的「必须先冻结」
      三组，而 `docs/02-工程.md` §10.3 已据此**加了订正标注**（裁决 2 的配套订正）。变更范围是
      **`invoke` 的参数类型**（四项方法名不变）。
    - **本子项目的那一半**（传什么、从哪来、缺位时失败面不变）写在 **§6.5**；**位置由 C 设计**，
      F 不预先发明它的形状。
    - **一处仍未落的接线**（与本裁决配套、但不在裁决范围内）：**凭据**若这条调用需要，其唯一来源是
      `continuum_secrets::issue(&cap, now)`（`cap` 取自这次调用**已获准的**某一枚能力），
      而把它接上要新增
      `continuum-runtime → continuum-secrets` 的边与一个调用点。**本设计今天不登记那条边**
      （§10.1：零使用的边即假边）。**收件人：`continuum-runtime` 的下一轮**（工具调用路径自己），
      或按 `p3a-followups.md` 第三节末尾的原计划与 B 的密钥运行时接线一并做。
11. **`tokio` 在 `continuum-runtime` 里此前零使用**（`Cargo.toml:27` 声明、`src/` 与 `tests/` 无引用），
    本子项目第一次为 `block_on` 异步的 `invoke` 真的用它（§10.1）。**收件人：后续阶段**（若驱动整体
    转异步，这个「在同步路径里开一个当前线程运行时」的形状要重做）。
12. **`--input` 省略即 `{}`**（§2.1）与 **`--intent` / `--approve` 与 `--effect` 同进同出**（§2.2）
    都是本设计的**决定**，规范未规定。**两条由协调者本轮拍板接受**（拍板于本子项目的本轮派发，
    不是本子项目自评）：`{}` 的理由按 `task` 对 `parameters` 的既有先例写（§2.1 已引原文），
    条件必填 `--intent` 的理由按**可观察性**写（§2.2 已写明：零效应时该取值不落任何地方、
    读者无从验证），**该可观察性已由 P-2 的第二半拍下来**（读回 `idempotency_key` 断言含该 intent）。
    **不再悬置**；若用户日后要对称性（恒必填 `--intent`），改动只在这两处与 P-13。
13. **`AuthorizedTool` 派生 `Debug`**（P3A 设计 §10 第 10 条）：把整枚 `AuthorizedTool` 格式化进
    错误上下文会把已获准能力的**作用域**打出来。本路径**不这样做**——`TaskError::Capability` 的
    `Display` 只带内层 `CapabilityError` 的消息（那些消息里没有作用域；`MissingCapability` /
    `UndeclaredCapability` 只有 kind 的编码串）。**这一条没有照片**（关于「驱动没写某句格式化」的
    否定命题），记此以免后来者顺手加一句 `{:?}` 而没人发现。
