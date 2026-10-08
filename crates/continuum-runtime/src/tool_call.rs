//! 工具调用路径的共享面（设计 §3.2、§6.4）。
//!
//! 本模块今天放五样东西：
//!
//! 1. **时钟**（[`now_millis`]）与**能力寿命**（[`CAPABILITY_LIFETIME_MS`]）；
//! 2. **强制点 (2) 的铸币判定**（[`mint_declared_effects`]）；
//! 3. 它带过来的那几个判定小件（[`explicit_current_rule`] / [`arbitrate`] / [`mints`] /
//!    [`decision_name`] / [`policy_context`] / `policy_context_for_effect`）；
//! 4. **工具调用路径本身**（[`run_tool_call`]，F 的 Task 3 落地）；
//! 5. 命令路径与工具路径**共用**的三个落库取值/推进函数（[`effect_key`] /
//!    [`authorization_field`] / [`finish_declared_effects`]，F 的 Task 3 从 bin 的
//!    `task_cmd.rs` 搬来——两条路径各写一份就是同一件事两个产生点）。
//!
//! # 为什么整批搬进 lib
//!
//! 工具调用路径是一个 **lib** 函数（设计 §6.4），而它与命令路径**共用同一个判定点**
//! （设计 §5.1：`mints` 六格表**只有一个落点**，工具路径不重写、也不新增第二个判定
//! 点）。「共用」在 Rust 里的含义是**只有一份定义**：铸币判定与它调用的裁决、上下文
//! 构造若留在 bin 的 `task_cmd.rs`，lib 侧的 [`mint_declared_effects`] 就调不到它们，
//! 而就地再写一份就是同一个判定有两个产生点。故这些函数整体移进 lib，
//! `task_cmd` 改为从这里取用——**这是搬家的代价，不是新立的一套词汇表**。
//!
//! # 公开面为什么比「应该」的大
//!
//! bin 是**另一个 crate**（`pub(crate)` 对它不可见），而 `task_cmd` 与它自己的单元用例
//! 还要用 [`arbitrate`] / [`mints`] / [`explicit_current_rule`] / [`effect_key`] /
//! [`authorization_field`] / [`finish_declared_effects`] 等，故这几个函数一律 `pub`。
//! 它们不是给 crate 外用的接口，是 bin 与 lib 之间的接缝。
//!
//! **可见性按「搬完之后谁还调用它」定，不按「搬之前谁调用过」**（F 的 Task 3 口径）：
//! [`run_tool_call`] 把**整条工具调用流程**搬进了 lib，那些调用点也随之进来；但
//! [`effect_key`] / [`authorization_field`] / [`finish_declared_effects`] 的
//! **命令路径调用点仍在 bin**（`task_cmd.rs` 的 `record_declared_effects` / `run` 第 6 步），
//! 故这三个仍必须是 `pub`。`policy_context_for_effect` 是唯一的例外，理由见它的文档。

use continuum_capability::{AuthorizedEffect, Capability, CapabilityKind, authorize, mint};
use continuum_effect::{
    Effect, EffectId, EffectState, EffectType, advance, find_by_idempotency_key, record_planned,
};
use continuum_persist::Db;
use continuum_policy::{
    Condition, Decision, ExplicitApproval, Level, Policy, PolicyContext, Scope, decide,
    load_policies,
};
use continuum_provider::ProviderRegistry;
use continuum_workspace::IntentId;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::TaskError;
use crate::cli::{EffectSpec, ToolArgs};

/// 当前时刻，Unix 毫秒。
///
/// 时钟早于 Unix 纪元时取 0 而非报错：`created_at` 只用于记录，回退的时钟不该让一条
/// 本来能跑的调用失败。
pub fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 强制点 (2) 铸出的能力活多久（Unix 毫秒），即签发时取 `now_millis() + 本值`。
///
/// §51 要求凭据「short-lived」，而**规范与设计都没有规定这个数**——它是本阶段的决定，
/// 集中在此一处、不散落。取值理由：
///
/// - 这枚能力只需覆盖**本条命令的执行**，以及由它签出的凭据（设计 §5.2：凭据的到期
///   不晚于能力的 `expiry`，故能力的寿命是那个上限）；
/// - 15 分钟对一条任务命令是宽的余量，又远短于长期令牌，符合「short-lived」的意；
/// - **驱动今天不自己校时**（[`mint`] 不读时钟，见其文档），故这个数在本次运行中不可
///   观察：一条跑得比它久的命令不会在运行中被拦下，而是让下游签出的凭据更早到期——
///   那是 fail-closed 的方向。
///
/// 连接器（子项目 B）真的消费 [`AuthorizedEffect`] 时，这个数应按那时能给出的依据
/// （命令的实际时长上界、凭据源的轮换周期）重新定；届时也只改这一处。
pub const CAPABILITY_LIFETIME_MS: i64 = 15 * 60 * 1000;

/// 第 2 级 `ExplicitCurrent` 的那条规则：`--approve` 给出时成立的 `Allow`。
///
/// **它是内建的、不落库的规则**（设计第 5.2 节：第 2 级在下篇由驱动的显式确认占位，
/// P4 就位后移交），由 [`arbitrate`] 在裁决时放进给 `decide` 的列表里，**不是从 `policy`
/// 表读来的**。
///
/// # `scope` 对它无意义
///
/// [`Scope`] 只有 `User | Project`，而本规则既不属于用户也不属于项目——它是内建的。
/// 此处填 [`Scope::User`] 只是因为该字段必填，**不是「这条规则是用户的」**。
/// `decide` **不读 `scope`**（`continuum_policy::engine` 的文档：裁决只按层级与决策取严），
/// 故这个取值不影响任何裁决结果——这一条由 `continuum-policy` 自己的用例
/// `crates/continuum-policy/tests/arbitration.rs::scope_does_not_participate_in_arbitration`
/// 从两个方向钉住（同层内对调 `Scope` 结果不变）。要给它一个名副其实的取值须给 `Scope` 加变体，那是改
/// 一个跨 crate 的公开类型（`policy` 表的 `scope` 列还有落库编码），须另行裁定，
/// 不在这里就地扩。
///
/// # 条件必须有，且必须是「`explicit_current` 成立」
///
/// 一条**无条件**的第 2 级 `Allow` 会在 `--approve` 未给出时照样获胜，把第 3–5 级的
/// `Deny` 一律推翻，即 fail-open（设计第 5.5 节的取舍只在 `--approve` 给出时成立）。
/// 本函数的用例 `the_explicit_current_rule_allows_only_when_the_flag_is_given`
/// 从两侧钉住它：给出时 `Allow`、未给出时**不成立**。
pub fn explicit_current_rule() -> Policy {
    let condition = Condition::parse(&serde_json::json!({
        "all": [{"fact": "explicit_current", "eq": true}]
    }))
    // 字面量条件必然合法：事实名与比较符都在各自的封闭集合内，取值是布尔。
    // 写成 `expect` 而不是在运行期兜底——这条规则是编译期就定死的常量，
    // 它若解析不了，那是本文件写错了，不该在用户的调用上表现为一条策略静默失效。
    .expect("内建的第 2 级规则条件是合法谓词");
    Policy {
        level: Level::ExplicitCurrent,
        condition,
        decision: Decision::Allow,
        scope: Scope::User,
    }
}

/// 第 7 步的裁决：把第 2 级那条规则放进表里，**一次**裁决完。
///
/// 判定与 `decide` 分开写是刻意的（计划 Task 10 第 3 步）：`decide` 只裁决，而
/// 「`--approve` 能越过什么」这条规则在驱动这一侧，故「第 1 级不可越」有一个单独的落点，
/// 不会混进通用裁决里被顺手改掉。
///
/// **第 2 级那条规则必须留在表里**：把它排除在外、把「越过」留给 [`mints`] 去做，
/// 是设计第 5.7 节的另一种接法；本子项目在 Task 5 已裁定不这么做（理由见 [`mints`]，
/// 那边也是「照抄 §5.7 会 fail-open」的完整推导所在）。
pub fn arbitrate(policies: &[Policy], ctx: &PolicyContext) -> Decision {
    let mut table = policies.to_vec();
    table.push(explicit_current_rule());
    decide(&table, ctx)
}

/// 由**一次裁决的结果**决定是否铸造批准值（设计第 5.7 节的表，**本接法**）。
///
/// # 为什么不能照抄 §5.7 那张表
///
/// §5.7 的表对应的是另一种接法：裁决时**把第 2 级排除在外**，越过在第 5.7 节的映射里做。
/// 本子项目在 Task 5 已裁定采用**另一种接法**——第 2 级那条规则就留在传给 `decide` 的表里
/// （见 [`arbitrate`]）。两种接法在「铸造与否」上结果一致，但**返回的 `Decision` 不同**：
/// 例如第 3 级 `Deny` + `--approve`，接法 A 返回 `Deny`、本接法返回 `Allow`。
///
/// **照抄 §5.7 的 `Deny` 那一行（「有 `--approve` 才铸造」）是 fail-open**：本接法在
/// 「第 1 级 `Deny` + `--approve`」时也返回 `Deny`，照那行读就会铸造——而第 1 级是设计里
/// **那道**命令开关越不过的防线（第 5.5 节）。
/// **订正（2026-10-08，F 终审 m-6）**：此处原写「**唯一一条**……的防线」，与下方
/// `Deny + --approve` 的三种来源相抵（第 2 种「落库一条与第 2 级同层的 `Deny`」同样越不过，
/// 照片 `a_same_level_persisted_deny_survives_the_flag`）。那三种是 `save_policy`
/// 不校验层级来源带来的**可达状态**，全文一致地把「防线」这个说法留给第 1 级、
/// 把它们按来源枚举——这是措辞张力而非事实错误，同一段 30 行内即给全枚举；此处只去掉「唯一」。
///
/// # 本接法的映射
///
/// `--approve` 已给出时那条第 2 级内建 `Allow` 会参与夺冠（它是第 2 级），故裁决值只能
/// 来自比它更高（第 1 级）或与它同层更严的规则；未给出时它不成立，裁决值由落库规则或
/// 「无匹配默认 `Deny`」给出。下面逐种裁决**列全来源**——纪律要求「绝对措辞须有对应
/// 用例」，故每一支来源都要有照片，见各条末尾。
///
/// - **`Allow` → 铸造。** 夺冠的那条规则是 `Allow`，来源只有两类：
///   1. **落库的一条 `Allow`**——层级不限（[`explicit_current_rule`] 已说明 `save_policy`
///      不校验层级来源）。**无 `--approve` 时的主路径正是落库的第 3–5 级 `Allow`，
///      即设计第 5.7 节第一行**；照片：`a_runtime_default_allow_mints_without_the_flag`
///      与端到端的 `an_allowed_integration_is_applied_without_the_flag`。
///   2. **第 2 级内建的那条**（[`explicit_current_rule`]），即 `--approve` 已给出时；
///      照片：`with_no_rule_at_all_the_flag_still_decides`。
/// - **`RequireApproval` → 有 `--approve` 才铸造。** 夺冠的那条规则是 `RequireApproval`：
///   - `--approve` **未**给出：只能来自落库的某条 `RequireApproval`（层级不限，理由同上；
///     设计上落库的是第 3、4 级、第 5 级内建，但 `save_policy` 不拦第 1 级）；照片：
///     `a_require_approval_rule_needs_the_flag` 的前半段。
///   - `--approve` **已**给出：落库的第 3–6 级会被第 2 级的 `Allow` 越过（同一张照片的
///     后半段），但仍有两处返回 `RequireApproval`——**落库的第 1 级**，或**与第 2 级同层
///     的一条**（同层取更严，压过内建的 `Allow`）；两处各一张照片，见
///     `a_require_approval_verdict_survives_the_flag_at_the_first_two_levels`。故
///     `approved` 这个入参在这一支上可观察，六格表逐格钉住它。
/// - **`Deny` + `--approve` 未给出 → 不铸造。**（夺冠的是一条落库 `Deny`，或无任何规则
///   匹配而落到默认 `Deny`。）
/// - **`Deny` + `--approve` 已给出 → 不铸造。** 此时那条内建 `Allow` 在表里，却仍返回
///   `Deny`，来源只有三种：
///   1. **落库的第 1 级 `Deny`**（高于第 2 级）；照片：
///      `a_system_safety_deny_is_not_overridden_by_the_flag`。
///   2. **落库一条与第 2 级同层的 `Deny`**（`ExplicitCurrent`；同层取更严，压过内建的
///      `Allow`）——可达性论据同第 1 条来源：`save_policy` 不校验层级来源；照片：
///      `a_same_level_persisted_deny_survives_the_flag`。
///   3. **调用方没把第 2 级规则放进表里**（违反 [`decide`] 的前置条件）：此时 `Deny` 只是
///      「没有更高的规则放行」，`decide` 无从知道 `--approve` 的存在。**本驱动不走这条**
///      （[`arbitrate`] 恒把那条规则放进表里），故它没有照片——列在这里是为了让「`Deny`
///      的三种含义」在本层是完整的，三种的处置见 `engine.rs` 的说明。
///   **三种都不该铸造**：第 1 种是命令开关越不过的那条防线；后两种若铸造，就是把
///   「更严的同层规则」或「一条都没放行」读成了批准。
///
/// 「无任何规则匹配时默认 `Deny`」这一路（设计第 5.3 节）由此自动落到「给出 `--approve`
/// 才放行」：未给出即 `Deny` 不铸造，给出则那条第 2 级规则成立、裁决为 `Allow` 而铸造——
/// 与设计第 5.5 节的第三行一致。
pub fn mints(decision: Decision, approved: bool) -> bool {
    match decision {
        Decision::Allow => true,
        Decision::RequireApproval => approved,
        Decision::Deny => false,
    }
}

/// 裁决值的中文名，只用于错误信息（标识符仍是英文，见项目的语言口径）。
pub fn decision_name(decision: Decision) -> &'static str {
    match decision {
        Decision::Allow => "允许",
        Decision::RequireApproval => "要求批准",
        Decision::Deny => "禁止",
    }
}

/// 裁决用的上下文。`--approve` 给没给是唯一有来源的事实；其余五事实本项目暂无来源
/// （设计第 5.6 节：`task_class` 由驱动注入，但没规定注什么），填 `None` 而不是编一个
/// 值——编出来的值会让引用它的规则开始匹配，那正是第 5.6 节点名的变更风险。
///
/// **这是集成那次裁决的上下文**（第 4 步的 `decision`，第 7 步复用），`effect_type` 缺省。
/// 逐条效应那次用 `policy_context_for_effect`——两者问的不是同一件事，见 `task_cmd` 的
/// 模块文档「与『策略只查一次』不冲突」。
pub fn policy_context(approved: bool) -> PolicyContext {
    PolicyContext {
        explicit_current: approved.then_some(ExplicitApproval),
        ..PolicyContext::default()
    }
}

/// 逐条效应那次裁决的上下文（强制点 (2)）。
///
/// 与 [`policy_context`] 的唯一差别是 `effect_type` 填着这一条效应的类型——那是
/// [`PolicyContext::effect_type`] 这个字段存在的理由（其文档原文：「本次待记的效应
/// 类型」）。**其余四个事实同样缺省**，理由与 [`policy_context`] 相同。
///
/// 这个字段可观察：一条按 `{"fact":"effect_type","eq":"charge"}` 限定的规则只在
/// `--effect` 为 `charge` 时成立，而集成那次裁决（`effect_type` 缺省）不会被它匹配。
///
/// # 只对 crate 内公开（`pub(crate)`，不是 `pub`）
///
/// 本模块的公开面里其余几个判定小件都必须是 `pub`（bin 的生产代码或 bin 的用例在调它们，
/// 而 bin 是另一个 crate），**本函数是唯一例外**：`grep` 全量 bin 侧调用点，它只出现在
/// `task_cmd.rs` 的两条**文档注释**里，没有任何代码取用它——lib 内唯一的调用者是
/// [`mint_declared_effects`]。故它收窄到 `pub(crate)`，那个用例零成本。
///
/// 收窄的连带面：`mint_declared_effects` 与 [`policy_context`] 的文档原先以
/// `[`policy_context_for_effect`]` 链接它，而公开项的文档链到私有项会触发
/// `rustdoc::private_intra_doc_links`——那两处已按本仓处理 rustdoc 警告的既有办法
/// **降级为代码跨度**（名字仍是同一个，只是不再是链接）。
pub(crate) fn policy_context_for_effect(approved: bool, effect_type: EffectType) -> PolicyContext {
    PolicyContext {
        explicit_current: approved.then_some(ExplicitApproval),
        effect_type: Some(effect_type),
        ..PolicyContext::default()
    }
}

/// 强制点 (2) 的铸币判定（设计 §3.2）。**不收事务**——它不碰库。
///
/// **它并不「纯」**：`mint` 的 `expiry` 取 `now_millis() + CAPABILITY_LIFETIME_MS`，
/// 而 `now_millis()` **读时钟**。要说准的是「不碰库」。
///
/// 返回 `(AuthorizedEffect, Decision)` 成对：`Decision` 不再被丢掉——工具路径要用它
/// 填 `effect.authorization`（那条效应**自己**的那次裁决），而它不得第二次调 `arbitrate`
/// （那正是设计 §5.1 禁止的第二个判定点）。
///
/// # 逐条效应各问一次策略
///
/// 每条声明用**它自己**的 `policy_context_for_effect`（`effect_type` 已填）裁决一次；
/// 「铸不铸」复用 [`mints`]（六格表唯一的落点，见其文档），**不重写**。铸得出就用
/// [`CapabilityKind::for_effect`] 取 kind、以该效应的**目标**为作用域铸一枚能力
/// （§253 的 `git.push:origin/main` 即此形），并配成 [`AuthorizedEffect`]。
///
/// # 拒绝的是**整条**命令
///
/// 按声明次序逐条走，遇到第一条铸不出的即返回 [`TaskError::EffectNotAuthorized`]——
/// 不是只跳过那一条。调用方（`task_cmd` 的 `record_declared_effects` 开的事务）回滚，
/// 上层再清理工作区。
///
/// # 两个 `expect` 的理由
///
/// [`mint`] 唯一的失败是空作用域，而 `--effect` 的空目标在解析期即被拒
/// （`continuum_runtime::cli` 的 `EffectWithEmptyTarget`）；[`AuthorizedEffect::new`]
/// 唯一的失败是效应与能力的 kind 不对应，而这里的 kind 正是由同一条效应经
/// [`CapabilityKind::for_effect`] 派生的。两者都是本文件内部的不变量，写法与
/// `run_in_sandbox` 的「cli 已保证」同例，不在用户调用上兜底。
///
/// # 不读时钟
///
/// [`mint`] 不读时钟，`expiry` 由调用方给；这里取 [`now_millis`] 加
/// [`CAPABILITY_LIFETIME_MS`]。铸出的能力**不在这里做时效校验**——`
/// Capability::is_valid_at` 是时效判定的唯一产生点，由消费方（连接器 / 凭据签发）
/// 在用它之前判。
pub fn mint_declared_effects(
    policies: &[Policy],
    effects: &[EffectSpec],
    approve: bool,
) -> Result<Vec<(AuthorizedEffect, Decision)>, TaskError> {
    let mut authorized = Vec::with_capacity(effects.len());
    for spec in effects {
        let decision = arbitrate(
            policies,
            &policy_context_for_effect(approve, spec.effect_type),
        );
        if !mints(decision, approve) {
            return Err(TaskError::EffectNotAuthorized {
                effect: spec.effect_type.as_str(),
                target: spec.target.clone(),
                decision: decision_name(decision),
            });
        }
        let capability = mint(
            CapabilityKind::for_effect(spec.effect_type),
            spec.target.clone(),
            now_millis() + CAPABILITY_LIFETIME_MS,
        )
        .expect("cli 已保证 --effect 的目标非空，mint 不会失败");
        authorized.push((
            AuthorizedEffect::new(spec.effect_type, capability)
                .expect("kind 由 for_effect 从同一条效应派生，配对必然成立"),
            decision,
        ));
    }
    Ok(authorized)
}

/// 工具调用路径（F 设计 §6.4）。**收已经打开的 `&Db` 与 `&ProviderRegistry`**：
/// 开库与装配留在 bin（[`crate::cli`] 那一侧），本函数只做路径本身。
///
/// # 入库的**唯一**理由
///
/// 用例必须能注入一个持有**夹具适配器**的注册表——生产装配点今天是空的
/// （设计 §10.2：**没有任何适配器可登记**，注册表空 ⇒ 第 6 步那一跳返回
/// `Unregistered`）。**空装配点不是遗留物**：缺的是**一个可登记的适配器实现**，
/// 而装配者＝驱动自己、收件人是驱动自己／将来的适配器子项目。
///
/// # 七步（与设计 §3 的表逐条对齐）
///
/// 1. **幂等键预检**：逐条 `--effect` 用 [`effect_key`] 查
///    `find_by_idempotency_key`，任一已存在即 [`TaskError::EffectAlreadyRecorded`] 拒
///    **整条**。次序照命令路径：幂等键检查排在**强制点之前**。
/// 2. **读策略表一次**：`load_policies(&tx)`。
/// 3. **强制点 (2)**：[`mint_declared_effects`] → `Vec<(AuthorizedEffect, Decision)>`。
///    **不重写裁决、不新增判定点**；每条效应的 `Decision` 在这里被留下（下面写
///    `authorization` 要用），不重调 `arbitrate`。
/// 4. **强制点 (1)**：`presented` **由步骤 3 的产物逐枚 `capability().clone()` 而来，
///    按 `--effect` 的声明次序**——不另建一个自由浮动的 `Vec<Capability>`（那会让同一批
///    能力有两个可以各自构造的容器）。`now` 在这一步**取一次**，交给 `authorize` 与
///    下面各条效应的 `planned_at` / `updated_at` 复用。失败 →
///    [`TaskError::Capability`] 原样带出（`#[from]`）。
/// 5. **写效应行并提交**：逐条 `PLANNED → AUTHORIZED → EXECUTING`，`authorization` 填
///    **这条效应自己那次裁决**（[`authorization_field`]），`parameters` 填 `json!({})`
///    （照 `task` 的先例）。**步骤 4 与 5 用同一个事务**——设计 §3.2 那条承重性质
///    （审计行与 `EXECUTING` 行要么都在、要么都不在）的**来源**就是这一条：一次
///    `commit`，中途不提交。
/// 6. **那一跳**：[`ProviderRegistry::invoke_tool`]。**F 不构造也不命名请求类型**
///    （那由 `invoke_tool` 在内部构造）；这里只把 `&AuthorizedTool` 与 `input` 交出去。
///    失败（含 `Unregistered`）→ **各效应记 `FAILED`**（与 `task` 第 5 步机制选择失败
///    时的处置一致），再把 [`TaskError::ToolCall`] 报出去。
/// 7. **终态与 stdout**：按 `ToolResult.is_error` 写 `COMMITTED` / `FAILED`；**无论
///    `is_error` 为何，先把 `output` 以 JSON 一行打到 stdout**（设计 §3.3：工具调用
///    **没有子进程**，stdout 是调用方仅有的通道）。`is_error == true` 时打完再返回
///    [`TaskError::ToolReportedError`]（Task 6；那一支的细节见本函数下面那一节）。
///
/// # `is_error == true` 那一支（Task 6 落地）
///
/// 工具跑起来了、但**自报失败**时：各效应记 `FAILED`（与 `Err` 那一支同一条终态判定）、
/// `output` 照样先打印、然后返回 [`TaskError::ToolReportedError`]（带 `--tool` 的那个 id）。
/// **它不包成 [`TaskError::ToolCall`]**——C 设计 §7.1 的三分法把「适配器没跑成」（`Err`）
/// 与「跑成了、结果是错误」（`Ok(is_error: true)`）放在两条通道上，合并即抹掉这个区别。
///
/// **本段先前写的是「这一支只写终态、只打印，不返回 `Err`——变体在 Task 6 落地」**：
/// 那句在 Task 6 之前是真的，现在已被上面这段取代。来历留此，免得后来者照旧句以为
/// 自报失败的调用今天仍以退出码 0 结束（它今天以 `Err` 结束）。
///
/// # 打印与终态落库的先后
///
/// **先打印 `output`，再落终态，再返回变体**（计划 Task 6 第 3 步的次序）。两件事都得
/// 赶在 `result` 被消费之前做完，而打印只对 `Ok` 那一支有意义、终态落库两支都有：
/// 故打印写在终态那个 `match` 的 `Ok` 臂里，**两个 `Ok` 分支共用同一句**，`Err` 那一支
/// 直接取 `Failed`。**落库不能挪到 `result?` 之后**——`?` 在 `Err` 那一支就早退了，
/// `FAILED` 会不落（P-10 的终态断言即红）。打印在前的后果只有一处：终态那次写库失败时，
/// 调用方仍拿得到工具的输出。
///
/// # 运行时：当前线程，不 `enable_all`
///
/// `invoke_tool` 是异步的（内层是 `async_trait` 的 `ToolProvider::invoke`，适配器在
/// 调用里不做 I/O 的话不需要反应堆）。用
/// `Builder::new_current_thread().build()`——**不 `enable_all()`**：workspace 的 tokio
/// features 是 `["rt-multi-thread", "macros"]`，`enable_all` 按 `net` / `time` 等 feature
/// 门控，可能不可用，而换 feature 会动 `Cargo.toml` 的依赖清单。构造失败**没有**承载它的
/// 错误变体（那是资源耗尽，且本层不为此新立一个只在这一点可达的变体），故 `expect`。
pub fn run_tool_call(
    db: &Db,
    registry: &ProviderRegistry,
    args: &ToolArgs,
) -> Result<(), TaskError> {
    let tx = db.begin()?;

    // 步骤 1：幂等键预检——先全查，再写任何一条。
    for spec in &args.effects {
        let key = effect_key(declared_intent(args), spec);
        if find_by_idempotency_key(&tx, &key)?.is_some() {
            return Err(TaskError::EffectAlreadyRecorded { key });
        }
    }

    // 步骤 2：读策略表一次。工具路径**没有集成裁决**那一回事，故这一步只取表。
    let policies = load_policies(&tx)?;

    // 步骤 3：强制点 (2)。铸不出即拒整条（`Err` 时 `tx` 随作用域回滚，一条都没写）。
    let authorized = mint_declared_effects(&policies, &args.effects, args.approve)?;

    // 步骤 4：强制点 (1)。出示集由上面那批 `AuthorizedEffect` 逐枚取来——**不另建容器**。
    let presented: Vec<Capability> = authorized
        .iter()
        .map(|(authorized_effect, _decision)| authorized_effect.capability().clone())
        .collect();
    let now = now_millis();
    let authorized_tool = authorize(&tx, &args.tool, &presented, now)?;

    // 步骤 5：写效应行。**与步骤 4 同一个事务**，一次提交（见本函数的文档）。
    for (spec, (_authorized_effect, decision)) in args.effects.iter().zip(&authorized) {
        let key = effect_key(declared_intent(args), spec);
        let effect = Effect {
            // `id` 与幂等键同源：同一个三元组、同一个函数 [`effect_key`]。
            id: EffectId::new(key.clone()),
            effect_type: spec.effect_type,
            target: spec.target.clone(),
            // 本子项目不填 parameters（设计 §6.1 只说它是 JSON、未规定内容）：
            // **工具调用的 `--input` 因此今天不落库**，那是一处据实的缺口（设计 §14 第 6 条）。
            parameters: serde_json::json!({}),
            // 这条效应**自己**那次裁决（步骤 3 的产物），不是集成裁决——工具路径没有集成裁决。
            authorization: authorization_field(args.approve, *decision),
            idempotency_key: key,
            state: EffectState::Planned,
            planned_at: now,
            updated_at: now,
        };
        record_planned(&tx, &effect)?;
        // 三态一次写完（设计 §6.3）。每次 `advance` 在同一事务内追加一条审计。
        advance(&tx, &effect.id, EffectState::Authorized, now)?;
        advance(&tx, &effect.id, EffectState::Executing, now)?;
    }

    tx.commit()?;

    // 步骤 6：那一跳。**唯一的工具调用入口**（设计 §6.1）——F 不自己开第二条路。
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("当前线程运行时构造失败（资源耗尽）：本层没有承载它的错误变体");
    let result = runtime.block_on(registry.invoke_tool(&authorized_tool, args.input.clone()));

    // 步骤 7：终态与 stdout。失败那一格按本函数文档的说明处置（写 FAILED 后带出错误）。
    //
    // **先判空、再取 intent**（顺序是要紧的）：零 `--effect` 时 `--intent` 按设计**必须不给**
    // （`cli::parse_tool` 的同进同出），故那时 `declared_intent(args)` 会 panic 在一条**合法**的
    // 调用上。**这不是假想**：修复轮 1 ③ 的第一次合并正是无条件求值，被 P-9 / P-17 两条零效应
    // 用例当场抓到（`panicked at …: 有 --effect 必有 --intent`）。`finish_declared_effects` 自己
    // 也有一次早退，但**早退救不了实参求值**。
    //
    // **打印排在终态落库之前**（计划 Task 6 第 3 步的次序）。两件都得赶在 `result` 被消费
    // 之前做完，但约束不同：打印取的是 `Ok` 里那个 `ToolResult`（`Err` 那一支没有可打的
    // 东西），而终态落库**不能**挪到 `result?` 之后——`?` 会在 `Err` 那一支早退，`FAILED`
    // 就不落了（P-10 的终态断言即红）。故打印写在这一支里、由两个 `Ok` 分支共用**同一句**。
    //
    // 打印在前的可观察后果只有一处，而那正是设计 §3.3 的理由所在：终态那次写库若失败，
    // 调用方**仍拿得到**工具的输出（工具调用没有子进程，stdout 是它仅有的那条通道）。
    let terminal = match &result {
        Ok(tool_result) => {
            // `Value` 的 JSON 是一行紧凑文本（`Display` 即 `serde_json::to_string`）；
            // 无论 `is_error` 为何都打印——这是调用方仅有的那条通道（设计 §3.3）。
            println!("{}", tool_result.output);
            if tool_result.is_error {
                EffectState::Failed
            } else {
                EffectState::Committed
            }
        }
        Err(_) => EffectState::Failed,
    };
    if !args.effects.is_empty() {
        finish_declared_effects(db, declared_intent(args), &args.effects, terminal)?;
    }

    // 自报失败：终态落库之后报 [`TaskError::ToolReportedError`]，带的 `tool` 是 `--tool`
    // 给的那个 id。
    //
    // **`is_error` 不是 provider 失败**（C 设计 §7.1 的三分法：`Err` 与
    // `Ok(is_error: true)` 是两条通道），故这里**不能**包成 [`TaskError::ToolCall`]，
    // 也不另造一个重复既有判断的变体——报本路径独有的那一枚。
    match result {
        Ok(tool_result) if tool_result.is_error => Err(TaskError::ToolReportedError {
            tool: args.tool.clone(),
        }),
        Ok(_) => Ok(()),
        Err(error) => Err(TaskError::ToolCall(error)),
    }
}

/// 步骤 6/7 用的终态写入（**命令路径第 6 步与工具调用路径第 7 步共用**）。
///
/// `to` 由调用方**一处**判定：命令路径按 `--exec` 的退出形态（退出码 0 → `Committed`，
/// 非 0 → `Failed`）、工具调用路径按 `ToolResult.is_error`。崩溃那一支不走这里：进程都没了，
/// 记录停在 `EXECUTING`，由恢复钩子转 `UNKNOWN`（设计第 6.3、6.4 节）。
///
/// 零效应时不碰库（与 `task_cmd` 原先那份同一条早退理由：一个空事务没有意义，也会让
/// 无效应的一次调用凭空多一次写锁）。
///
/// # 每条效应**各取一次**时钟（两条路径在此**语义相同**）
///
/// `updated_at` 说的是「这条记录**在那一刻**被推到了新状态」，故循环内逐条取
/// [`now_millis`]。这一点与第 5 步那次写入**不同**：那一次是同一瞬间写下的若干行
/// （工具调用路径的 `planned_at` / `updated_at` 取同一个 `now`，那是刻意的，见
/// [`run_tool_call`] 的步骤 5），而这里是**若干次独立的推进**。
///
/// **写准这一条是有代价换来的**：本函数原先在两条路径上各有一份（bin 的 `task_cmd.rs`
/// 与 lib 的 `tool_call.rs`），而两份的差别**只有**这一点——bin 那份在循环内每条各取一次
/// [`now_millis`]，lib 那份复用第 4 步取的那**一个** `now`。**那个差别是语义差别，
/// 不是形状差别**：复用旧 `now` 会让 `COMMITTED` 的 `updated_at` 早于它实际发生的时刻
/// （工具调用可以跑任意久），而命令路径从来不是这么写的。故这里按**命令路径已有的语义**
/// 合并成一份：每次 `advance` 各取一次时钟。
///
/// **初稿在这里写过一句假话，来历留此**：那句话把「不合并」的理由说成「合并要求先给两条
/// 路径造一个共同形状，而那正是『同一件事两个类型』的反方向」。**评审判它不是真障碍，判得对**
/// ——共同形状用现成参数即可（`&IntentId` ＋ `&[EffectSpec]` ＋ `EffectState`），不必发明任何类型；
/// 而那句话会让后来者**照着它去避免合并**，从而把上面这处**真实的**时间戳差别永久留在两份实现里。
/// 错不在代码，在一句会让后来者做错事的话。
///
/// # 两条路径共用一份定义
///
/// 与 [`effect_key`] / [`authorization_field`] 同一条理由：命令路径调它（`task_cmd` 的第 6 步），
/// 工具调用路径也调它（[`run_tool_call`] 的第 7 步）。**它是 `pub`**：搬完之后 bin 侧的调用点
/// 还在（命令路径那一处）。
pub fn finish_declared_effects(
    db: &Db,
    intent: &IntentId,
    effects: &[EffectSpec],
    to: EffectState,
) -> Result<(), TaskError> {
    if effects.is_empty() {
        return Ok(());
    }
    let tx = db.begin()?;
    for spec in effects {
        // 记录的 `id` 与幂等键同源（[`effect_key`]），故这里由同一次派生取回 id。
        let id = EffectId::new(effect_key(intent, spec));
        advance(&tx, &id, to, now_millis())?;
    }
    tx.commit()?;
    Ok(())
}

/// 声明里那个意图 id。`--effect` 非空 ⇒ `--intent` 必为 `Some`（`cli::parse_tool` 的
/// 同进同出判定的两个方向之一）。
///
/// 写成函数而不是在 [`run_tool_call`] 顶上取一次：零效应时那两个循环都不执行，本值也
/// 用不到，而在顶上一取就会让一条**合法**的零效应调用 panic。故 `expect` 只在真的要用
/// 它时触发，那时代码路径已在「有 --effect」这一侧。
fn declared_intent(args: &ToolArgs) -> &IntentId {
    args.intent
        .as_ref()
        .expect("cli 已保证：有 --effect 必有 --intent")
}

/// 效应的身份与幂等键（设计第 6.1、6.5 节）：由 意图 id / 类型 / 目标 派生。
///
/// **`Effect.id` 与 `idempotency_key` 取同一个值**——同一三元组、同一个函数，
/// 不是一个字段各派一次。设计第 6.5 节只规定幂等键由该三元组派生；`id` 是表的主键
/// （第 8 节），本子项目同样用它派生，以免给「这条记录是谁」再立第二个来源。
///
/// # 两条路径共用一个定义（F 的 Task 3 从 bin 搬来）
///
/// 命令路径（`task_cmd` 的 `record_declared_effects` / `finish_declared_effects`）与
/// 工具调用路径（[`run_tool_call`]）都调它。**搬家的理由**：工具调用路径是 lib 函数，
/// 而 bin 是另一个 crate——定义留一份在 bin，lib 就调不到，就地再写一份就是同一件事
/// 两个产生点。**它仍是 `pub`**：搬完之后**命令路径的三个调用点仍在 bin**，故
/// 「搬完之后谁还调用它」的答案是「两侧都有」。
///
/// # 为什么带长度前缀
///
/// `intent` 与 `target` 都是自由文本（`IntentId::new` 只收字符串，不做校验），只靠
/// 分隔符拼接不是单射：`("a", publish, "b:publish:c")` 与 `("a:publish:b", publish, "c")`
/// 会拼出同一个串，两条不同的声明被当成同一条，第二条被静默拒绝。长度前缀让三段的
/// 分界可判定——键形如 `<意图字节长>:<意图>:<类型>:<目标>`，类型取自封闭枚举、不含
/// 冒号，故目标即最后一段的全部。
pub fn effect_key(intent: &IntentId, spec: &EffectSpec) -> String {
    format!(
        "{}:{}:{}:{}",
        intent.as_str().len(),
        intent.as_str(),
        spec.effect_type.as_str(),
        spec.target
    )
}

/// `authorization` 字段的内容（设计第 6.7 节）：**只记录、不校验**的不透明串。
///
/// 写入两件事：`--approve` 是否给出、以及策略的裁决结果。裁决取 [`Decision::as_str`]
/// 的编码——与 `policy` 表同一份，不在这里手抄字面量。
///
/// # 两条路径填的是**不同的那次裁决**（F 设计 §7.2，一处刻意的不同）
///
/// 命令路径填的是**集成那次裁决**（`policy_context`，`effect_type` 缺省）；工具调用路径
/// 填的是**这条效应自己那次裁决**（[`mint_declared_effects`] 成对返回的后一项）——
/// 工具调用**没有集成裁决这一回事**，凭空造一次就是一次没有意义的判定。**编码形状
/// 共用本函数**，这正是两条路径在此不各写一份的理由。
///
/// # 本子项目到此为止
///
/// **生产代码**不读回、不校验本字段（测试会读它，以钉住写入的内容与格式）。它是留给
/// 对账与审计的记录，不是一道强制。**P3 起驱动确实有了 Capability 的输入**（强制点 (2)
/// 已接上），但那条路径**不读回本字段**、也不靠它——本字段本身仍不被任何生产代码校验。
/// 校验属 Capability（P3）与 Authority（长期）的职责，其中 Capability 那一半落在
/// [`mint_declared_effects`]。
/// **不要把本函数或这个字段读成「此处已强制」**（设计第 6.7 节要求显式声明此边界）。
///
/// **它仍是 `pub`**：与 [`effect_key`] 同一条——搬完之后命令路径的调用点还在 bin。
pub fn authorization_field(approved: bool, decision: Decision) -> String {
    format!("approve={approved};policy={}", decision.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 一条**只对某一种效应类型**成立的规则。
    ///
    /// 条件形如 `{"fact": "effect_type", "eq": "<类型>"}`（取值取自
    /// [`EffectType::as_str`]，与 `capability_gate.rs` 的同名夹具同法）。
    fn only_for(level: Level, decision: Decision, effect_type: EffectType) -> Policy {
        Policy {
            level,
            condition: Condition::parse(&serde_json::json!({
                "all": [{"fact": "effect_type", "eq": effect_type.as_str()}]
            }))
            .expect("effect_type 是封闭事实，取值取自枚举"),
            decision,
            scope: Scope::User,
        }
    }

    fn spec(effect_type: EffectType, target: &str) -> EffectSpec {
        EffectSpec {
            effect_type,
            target: target.to_owned(),
        }
    }

    /// 幂等键对三段是**单射**：设计第 6.5 节的键由 意图 id / 类型 / 目标 派生，
    /// 若拼接有歧义，两条不同的声明会被当成同一条，第二条被静默拒绝。
    ///
    /// 用例给出一对**真的会撞**的三元组（`a` + `publish` + `b:publish:c` 与
    /// `a:publish:b` + `publish` + `c`；`IntentId::new` 不校验，含冒号的意图是收下的）。
    /// 带长度前缀时两者分得开；去掉长度前缀、改用普通分隔符拼接时本用例变红——
    /// 这就是 [`effect_key`] 那条「长度前缀使拼接是单射」论据的对照片。
    ///
    /// **本用例随 [`effect_key`] 从 bin 的 `task_cmd.rs` 搬来**（F 的 Task 3）：函数的
    /// 单元用例跟着函数走，留在 bin 会让「lib 里的函数由 bin 的用例覆盖」这一层错位
    /// （同 Task 2 搬 `mint_declared_effects` 时连同两条用例一起搬的先例）。
    #[test]
    fn the_effect_key_separates_the_intent_from_the_target() {
        let first = effect_key(&IntentId::new("a"), &spec(EffectType::Publish, "b:publish:c"));
        let second = effect_key(&IntentId::new("a:publish:b"), &spec(EffectType::Publish, "c"));
        assert_ne!(
            first, second,
            "两条不同的效应声明派生出同一个幂等键：第二条会被当成已登记而静默拒绝"
        );
    }

    /// **每一条效应带的是它自己那次裁决**，不是第一条的、也不是集成那次的。
    ///
    /// 两张表，两个方向：
    ///
    /// - **两条都铸得出、但裁决不同**（`--approve` 已给出 + 一条与第 2 级同层的
    ///   `RequireApproval` 限定 `charge`）：返回的两项各带本条的裁决
    ///   （`publish` → `Allow`，`charge` → `RequireApproval`），且各配对自己的能力
    ///   （目标是本条声明的目标）。**这就是「`Decision` 没被丢掉」的照片**：两条类型
    ///   不同、裁决不同，把两项都填成第一条的 `Decision` 时下面第二条断言即红。
    /// - **只有一条铸得出**：另一条报 [`TaskError::EffectNotAuthorized`]，且报的是
    ///   **铸不出的那一条**、`decision` 是那一条自己的裁决。
    ///
    /// 两个方向都要，但**它们钉的不是同一件事**（这一句原先写成「只测『都铸得出』的话，
    /// 一个把 `Decision` 恒填成 `Allow` 的实现也过」——**那句是假的**，来历见下）：
    ///
    /// - **方向一钉的是「两条各自带自己那次裁决」**。把两项都填成第一条的裁决，就是本用例
    ///   第二条断言的变异体（实测：`*first_decision.get_or_insert(decision)` 把
    ///   `pairs[1].1` 从 `RequireApproval` 变成 `Allow` ⇒ 红在那一行）。故「恒填 `Allow`
    ///   的实现也能过方向一」不成立——**它单独就红在方向一上**。
    /// - **方向二钉的是错误路径**：报的是**哪一条**（铸不出的那条，不是铸得出的那条）、
    ///   带的是**哪一枚**裁决（那一条自己的「禁止」，不是第一条的「允许」）。
    ///   **它不是 `Allow` 与 `RequireApproval` 的区分者**——那条路在 `Err` 上返回，
    ///   返回值的裁决根本不可观察。
    ///
    /// 于是那半句仍然成立、且是方向一**不可替代**的理由：**只测拒绝那条的话，
    /// `Allow` 与 `RequireApproval` 的区分没有照片**。
    ///
    /// # 这两组断言的「守卫三件套」第三件不可得（据实写明）
    ///
    /// 变异 A（两项都填第一条的裁决）红在**本测试体中部**（第二条裁决断言那一行）、
    /// 变异 C（返回次序反转）同样红在中部（`pairs[0].0.effect()` 那一行）。断言在测试体
    /// 中途炸掉，**其后的断言根本不执行**，故「同体内之前的断言全过、而它独红」那种
    /// **独立承重**的证明在这种写法下构造不出来——**它们的照片是与其后断言共有的**。
    ///
    /// 两个变异体的强度还不一样，一并写准：
    ///
    /// - **C 会让其后的每一条配对断言都红**（次序一反，后面的 `pairs[i]` 全对不上）⇒
    ///   后半段断言对 C 的区分力为零；
    /// - **A 不影响方向二**（改的是返回项的裁决，方向二根本不读返回值）⇒ 后半段对 A
    ///   **无区分力是因为它不观察这件事**，不是因为「也红」。**两者别混成一句。**
    #[test]
    fn the_shared_minting_function_returns_each_effect_own_decision() {
        // 方向一：两条都铸得出，裁决不同。
        let same_level_require_approval =
            vec![only_for(Level::ExplicitCurrent, Decision::RequireApproval, EffectType::Charge)];
        let effects = vec![
            spec(EffectType::Publish, "p1"),
            spec(EffectType::Charge, "c1"),
        ];
        let pairs = mint_declared_effects(
            &same_level_require_approval,
            &effects,
            // `--approve` 已给出：`charge` 那条与第 2 级内建 `Allow` 同层更严，故裁决为
            // `RequireApproval`（`mints` 该格为「铸造」）；`publish` 无更严的规则，裁决为
            // 内建那条的 `Allow`。
            true,
        )
        .expect("两条都铸得出");

        assert_eq!(pairs.len(), 2, "两条声明各铸一枚");
        assert_eq!(pairs[0].0.effect(), EffectType::Publish);
        assert_eq!(pairs[0].0.capability().scope(), "p1", "能力的作用域是本条的目标");
        assert_eq!(
            pairs[0].1,
            Decision::Allow,
            "publish 那条带的是它自己那次裁决 Allow"
        );
        assert_eq!(pairs[1].0.effect(), EffectType::Charge);
        assert_eq!(pairs[1].0.capability().scope(), "c1", "能力的作用域是本条的目标");
        assert_eq!(
            pairs[1].1,
            Decision::RequireApproval,
            "charge 那条带的是它自己那次裁决 RequireApproval——\n\
             把两项都填成第一条的 Decision 时，红在这一行"
        );

        // 方向二：只有 publish 铸得出（`--approve` 未给出，charge 无规则匹配 → 默认 Deny）。
        let only_publish_allowed =
            vec![only_for(Level::RuntimeDefault, Decision::Allow, EffectType::Publish)];
        match mint_declared_effects(&only_publish_allowed, &effects, false) {
            Err(TaskError::EffectNotAuthorized {
                effect,
                target,
                decision,
            }) => {
                assert_eq!(effect, "charge", "报的是铸不出的那一条（publish 铸得出）");
                assert_eq!(target, "c1");
                assert_eq!(
                    decision, "禁止",
                    "decision 是那一条自己的裁决（无匹配默认 Deny），不是第一条的「允许」"
                );
            }
            other => panic!("应报 EffectNotAuthorized{{charge,c1}}，实际 {other:?}"),
        }
    }

    /// 两条都铸不出时，报的是**声明次序第一条**，不是最后一条。
    ///
    /// 与端到端同名用例（`capability_gate.rs` 的
    /// `the_first_unmintable_effect_in_declaration_order_is_reported`）同法，但在 lib 里
    /// **无门控地跑**——不建工作区、不起沙箱。两条都铸不出是刻意的：只有这样，「报第一条」
    /// 与「报最后一条」才是两个不同的可观察结果（若头一条能铸出，两种实现报的是同一条）。
    ///
    /// 对调次序再跑一次：报的随之换成新的第一条。
    ///
    /// **后半段杀的是「与声明次序无关、只是恰好在半段一上撞对」的实现**——例如**恒报
    /// 「目标字典序最小者」**：半段一 `[c1, d1]` 报 `c1`（对），半段二 `[d1, c1]` **仍报
    /// `c1`**（错，应为 `d1`）⇒ 只有后半段能抓它。
    ///
    /// （这一句原先写成「**少了后半段，一个『恒报第一条声明』的实现也能过前半段**」——
    /// **那句是假的**：「恒报声明次序第一条」**就是正确行为**，它在两半上都对。
    /// 来历留此，免得后来者再照那句话去设计半段。另一句「少了前半段，一个『恒报最后一条』
    /// 的实现只在后半段露馅」成立，保留。）
    ///
    /// # 这组断言的「守卫三件套」第三件不可得（据实写明）
    ///
    /// 变异 B（铸不出的记下、循环走完报**最后**一条）编得过、也红在**本用例自己的断言上**，
    /// 但红在**前半段的第一条断言**（`charge_first` 那一半里 `("charge", "c1")` 那条
    /// `assert_eq!`）⇒ **同测试体内其后全部断言（整个「对调次序」半段）根本不执行**，故第三件
    /// （「之前的断言全过而它独红」）在这种写法下构造不出来——**它的照片同样是与其后断言
    /// 共有的**。
    ///
    /// （**原写「`:420` 那一行」**，2026-10-07 改为结构性定位：那是**落笔即过期**的写法
    /// ——写它的那次提交自己就把它作废了（同一提交在 `mod tests` 里加了行），`:420` 在最终
    /// 字节上是**测试 A 的结束括号**，不是任何断言。而这个数所数的语料**包含它自己所在的
    /// 文件**，故本文件里不写裸行号。）
    ///
    /// **更强的反证（别把它读成「后半段白写了」）**：该变异体下**后半段自己也是红的**
    /// （它会报 `charge:c1` 而非 `deploy:d1`）⇒ **后半段的区分力不落在 B 上**，而落在它
    /// 独有的那些实现上（上面那个「字典序最小者」）。**「它是守卫」与「它可独立承重」
    /// 是两件事**：前者成立，后者在这种「一个测试体内跑两半」的写法下不成立。
    #[test]
    fn the_shared_minting_function_reports_the_first_unmintable_in_declaration_order() {
        let no_rules: Vec<Policy> = Vec::new();

        let charge_first = vec![
            spec(EffectType::Charge, "c1"),
            spec(EffectType::Deploy, "d1"),
        ];
        match mint_declared_effects(&no_rules, &charge_first, false) {
            Err(TaskError::EffectNotAuthorized { effect, target, .. }) => {
                assert_eq!(
                    (effect, target.as_str()),
                    ("charge", "c1"),
                    "应报声明次序第一条 charge:c1"
                );
            }
            other => panic!("应报声明次序第一条，实际 {other:?}"),
        }

        // 对调次序：报的换成新的第一条。
        let deploy_first = vec![
            spec(EffectType::Deploy, "d1"),
            spec(EffectType::Charge, "c1"),
        ];
        match mint_declared_effects(&no_rules, &deploy_first, false) {
            Err(TaskError::EffectNotAuthorized { effect, target, .. }) => {
                assert_eq!(
                    (effect, target.as_str()),
                    ("deploy", "d1"),
                    "对调次序后应报新的第一条 deploy:d1（报最后一条的实现会在这里露馅）"
                );
            }
            other => panic!("对调次序后应报新的第一条，实际 {other:?}"),
        }
    }
}
