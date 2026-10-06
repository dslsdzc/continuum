//! 工具调用路径的共享面（设计 §3.2、§6.4）。
//!
//! 本模块今天放三样东西：
//!
//! 1. **时钟**（[`now_millis`]）与**能力寿命**（[`CAPABILITY_LIFETIME_MS`]）；
//! 2. **强制点 (2) 的铸币判定**（[`mint_declared_effects`]）；
//! 3. 它带过来的那几个判定小件（[`explicit_current_rule`] / [`arbitrate`] / [`mints`] /
//!    [`decision_name`] / [`policy_context`] / `policy_context_for_effect`）。
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
//! 还要用 [`arbitrate`] / [`mints`] / [`explicit_current_rule`] 等，故这几个函数一律
//! `pub`。它们不是给 crate 外用的接口，是 bin 与 lib 之间的接缝。

use continuum_capability::{AuthorizedEffect, CapabilityKind, mint};
use continuum_effect::EffectType;
use continuum_policy::{
    Condition, Decision, ExplicitApproval, Level, Policy, PolicyContext, Scope, decide,
};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::TaskError;
use crate::cli::EffectSpec;

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
/// 唯一一条命令开关越不过的防线（第 5.5 节）。
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
    /// 两条都要：只测「都铸得出」的话，一个把 `Decision` 恒填成 `Allow` 的实现也过；
    /// 只测拒绝那条的话，`Allow` 与 `RequireApproval` 的区分没有照片。
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
    /// 对调次序再跑一次：报的随之换成新的第一条。**少了后半段，一个「恒报第一条声明」的
    /// 实现也能过前半段**；少了前半段，一个「恒报最后一条」的实现只在后半段露馅。
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
