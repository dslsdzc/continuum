//! §249 十态的编码与 §4.1 的迁移表（设计 `docs/superpowers/specs/2026-10-05-p3d-model-registry-router-design.md`）。
//!
//! **本文件只放不碰 `ModelProfile` 的那些用例**（十态编码、迁移表）。**闸门**那一组
//! （`RoutableModel::try_new` 的十态逐项、`stale` 的两侧对钉、`Degraded` 放行）住在
//! `src/lifecycle.rs` 的 `#[cfg(test)] mod tests`——它们各要一份画像，而 `ModelProfile` 的
//! 构造是 crate 内的（`pub(crate) fn try_new`，设计 §2.1），集成测试构造不出来（计划第 84 行
//! 已点明这一后果，Task 3 亦照此办理）。**这不是漏项，是那条保证的直接后果**；
//! 拆开的理由与各处住址写在这里，免得后来者以为闸门那组用例被我漏掉了。
//!
//! 每条断言的「红的条件」见各用例的注释——这些注释不是说明，是变异时的靶子。

use continuum_model_registry::{transition, LifecycleError, LifecycleState};

/// §249 的十个状态，**逐项列出**（不抽代表）。
///
/// 上面那两句断言（`as_str` / `parse`）都遍历本表，故加第十一个状态时本表必须手工同步：
/// `as_str` / `parse` 侧漏了臂会编译不过，**但这张表漏了不会**——这个点不设防，
/// 靠后来者自己记得（与 `tests/profile.rs` 的 `ALL_DIMENSIONS` 同一处不设防的点）。
const ALL_STATES: [LifecycleState; 10] = [
    LifecycleState::Discovered,
    LifecycleState::Unprofiled,
    LifecycleState::Researched,
    LifecycleState::Probed,
    LifecycleState::Verified,
    LifecycleState::Active,
    LifecycleState::Stale,
    LifecycleState::Degraded,
    LifecycleState::Quarantined,
    LifecycleState::Disabled,
];

/// §249 照录的十个状态名，与本 crate 的落库编码**逐字相同**：小写、多词以 `_` 连接。
///
/// **这张表是手写的字面量**（钉的是格式，不是路径）：十态恰好都是单词，故本表里
/// 一个 `_` 都不出现——若将来 `as_str` 被改成大写（`"ACTIVE"`）或多加下划线
/// （`"un_profiled"`），本表逐项比对即红。
const ALL_NAMES: [&str; 10] = [
    "discovered",
    "unprofiled",
    "researched",
    "probed",
    "verified",
    "active",
    "stale",
    "degraded",
    "quarantined",
    "disabled",
];

/// 十态**逐项**：`as_str` 的小写字面量 + `parse` 的往返。
///
/// 红的条件：任一枚 `as_str` 改成大写或多词加下划线、或 `parse` 漏掉任一枚即红
/// （两个方向都在本用例里：编码一次、解码一次）。
#[test]
fn the_ten_lifecycle_states_are_recorded_verbatim() {
    assert_eq!(ALL_STATES.len(), ALL_NAMES.len(), "十态与十个名字须一一对应");

    for (state, name) in ALL_STATES.iter().zip(ALL_NAMES) {
        assert_eq!(state.as_str(), name, "{state:?} 的编码应是 {name:?}");
        assert_eq!(
            LifecycleState::parse(name),
            Some(*state),
            "{name:?} 应解析回 {state:?}"
        );
    }
}

/// `parse` 是 `as_str` 的**严格逆**：表外串一律 `None`，**不取默认值**。
///
/// 红的条件：`parse` 的 `_` 臂被改成某一枚默认状态（例如 `Some(Self::Discovered)`）即红。
/// 覆盖三种表外形状：大写、未知词、空串，以及一处「看起来像但多一个下划线」的串。
#[test]
fn an_unknown_lifecycle_state_name_is_rejected() {
    assert_eq!(LifecycleState::parse("ACTIVE"), None, "大写不是本编码");
    assert_eq!(LifecycleState::parse("un_profiled"), None, "多词须按原词连写");
    assert_eq!(LifecycleState::parse("unknown"), None);
    assert_eq!(LifecycleState::parse(""), None);
}

/// §4.1 的合法对**逐条** `Ok`，且返回值是**迁移前的旧态**（`from`）。
///
/// 逐条写死的理由：这些对由手写 `matches!` 分支承载，各分支能各自漂移；
/// 抽一个数组遍历会让「漏掉其中一条」在同一个循环里静默。
/// 每条的 `from` 与 `to` **互不相同**——否则「返回值是 `from` 还是 `to`」就分不出来。
#[test]
fn the_transition_table_accepts_every_listed_pair() {
    use LifecycleState::*;

    // 正常阶梯（§21 §22）：六态线性、单调。
    assert_eq!(transition(Discovered, Unprofiled), Ok(Discovered));
    assert_eq!(transition(Unprofiled, Researched), Ok(Unprofiled));
    assert_eq!(transition(Researched, Probed), Ok(Researched));
    assert_eq!(transition(Probed, Verified), Ok(Probed));
    // `verified → active`：上线（同一对，不另设分支）。
    assert_eq!(transition(Verified, Active), Ok(Verified));

    // §82 的漂移与重新 profiling。
    assert_eq!(transition(Active, Stale), Ok(Active));
    assert_eq!(transition(Stale, Researched), Ok(Stale));

    // §249 的异常态往返。
    assert_eq!(transition(Active, Degraded), Ok(Active));
    assert_eq!(transition(Degraded, Active), Ok(Degraded));

    // 事故与人工下线后的两条出口。
    assert_eq!(transition(Quarantined, Disabled), Ok(Quarantined));
    assert_eq!(transition(Disabled, Unprofiled), Ok(Disabled));
}

/// 未列出的对一律 `Err(Illegal { from, to })`，且**两端正是给的那一对**。
///
/// 红的条件：把 `Illegal` 的字段写反（`from` / `to` 对调）即红——**这正是本用例存在的理由**：
/// 只断言「返回了 `Err`」的话，字段写反不会有任何用例变红。
/// 另含三条**回退**方向（`→ active`）与一条跨级（`Discovered → Active`），
/// 它们是「看起来合理但表里没有」的对，专治「顺手放行」。
#[test]
fn an_unlisted_transition_pair_is_rejected_with_both_ends() {
    use LifecycleState::*;

    assert_eq!(
        transition(Stale, Active),
        Err(LifecycleError::Illegal {
            from: Stale,
            to: Active
        }),
        "§82 的漂移态回到 `active` 是绕开画像流水线，表里没有这条"
    );
    assert_eq!(
        transition(Disabled, Active),
        Err(LifecycleError::Illegal {
            from: Disabled,
            to: Active
        }),
        "重新启用须重走 onboarding（出口是 `unprofiled`）"
    );
    assert_eq!(
        transition(Quarantined, Active),
        Err(LifecycleError::Illegal {
            from: Quarantined,
            to: Active
        }),
        "隔离态的出口只有 `disabled`"
    );
    assert_eq!(
        transition(Discovered, Active),
        Err(LifecycleError::Illegal {
            from: Discovered,
            to: Active
        }),
        "跨过整条画像流水线"
    );
    assert_eq!(
        transition(Active, Verified),
        Err(LifecycleError::Illegal {
            from: Active,
            to: Verified
        }),
        "顺着阶梯往回退不是本表的边"
    );
    assert_eq!(
        transition(Probed, Unprofiled),
        Err(LifecycleError::Illegal {
            from: Probed,
            to: Unprofiled
        }),
        "顺着阶梯往回退不是本表的边"
    );
}

/// §4.1：`* → quarantined` / `* → disabled` **可由任意状态进入**——十态**逐项**×两目标，
/// 共 20 条，每条 `Ok`（事故与人工下线不挑时机，与 P1 的 `INVALIDATED` / `CANCELLED` 同判据）。
///
/// 这 20 条**含两个自环**（`Quarantined → Quarantined` 与 `Disabled → Disabled`）——
/// 它们是这张矩阵顺带覆盖到的，**自环另有专案**（`a_self_transition_follows_the_any_state_rule`）。
///
/// 红的条件：把 `* → quarantined` 或 `* → disabled` 收窄成白名单（去掉某几个来源）即红。
#[test]
fn quarantined_and_disabled_are_reachable_from_every_state() {
    for &state in ALL_STATES.iter() {
        assert_eq!(
            transition(state, LifecycleState::Quarantined),
            Ok(state),
            "{state:?} → quarantined 应可进入"
        );
        assert_eq!(
            transition(state, LifecycleState::Disabled),
            Ok(state),
            "{state:?} → disabled 应可进入"
        );
    }
}

/// 自环的**专案照片，不靠上面的矩阵顺带**：十态逐项 `x → x`，
/// 目标是 `Quarantined` / `Disabled` 的两条 `Ok`，其余八条 `Err(Illegal { from, to })`。
///
/// **两侧都钉**：只写「自环一律非法」会与上面那 20 条直接打架；只写「自环一律合法」
/// 则会把 `Active → Active` 这种未列出的对放行。故两个方向各在一条专案断言里。
/// 依据是执行期裁定（计划 Task 4）：自环的合法性**由那张表决定**，目标是
/// `Quarantined` / `Disabled` 时合法（`*` 含它自己），其余是未列出的对。
///
/// 红的条件：实现里加一条「`from == to` 一律拒绝」即前两条红；把自环一律放行即后八条红。
#[test]
fn a_self_transition_follows_the_any_state_rule() {
    use LifecycleState::*;

    // 表里列到了的：`* → quarantined` / `* → disabled` 的 `*` 含它自己。
    assert_eq!(transition(Quarantined, Quarantined), Ok(Quarantined));
    assert_eq!(transition(Disabled, Disabled), Ok(Disabled));

    // 其余八态的自环没有列进表，一律 `Illegal`——两端正是给的那一对。
    for &state in ALL_STATES.iter() {
        if matches!(state, Quarantined | Disabled) {
            continue;
        }
        assert_eq!(
            transition(state, state),
            Err(LifecycleError::Illegal {
                from: state,
                to: state
            }),
            "{state:?} → 自身 未列进表，应被拒"
        );
    }
}

/// §21：**重新启用须重走画像流水线**，故 `disabled` 的出口是 `unprofiled` 而不是 `active`。
///
/// **两侧对钉**：只写「`→ unprofiled` 可以」的话，把出口改成 `active` 不会有任何用例变红
/// （另一侧正是 fail-open 的那一侧）。这里两个方向都在。
#[test]
fn disabling_returns_a_model_to_unprofiled_not_active() {
    use LifecycleState::*;

    assert_eq!(
        transition(Disabled, Unprofiled),
        Ok(Disabled),
        "重新启用走的是 onboarding 入口"
    );
    assert_eq!(
        transition(Disabled, Active),
        Err(LifecycleError::Illegal {
            from: Disabled,
            to: Active
        }),
        "直接上线会跳过整条画像流水线"
    );
}
