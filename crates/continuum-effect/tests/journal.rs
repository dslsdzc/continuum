use continuum_effect::{transition, EffectState, StateError};

/// 设计上篇第 7.2 节的迁移表：十三条合法边。
///
/// 本文件只用这一份数据——合法边由它逐对断言接受，其余三十六对逐对断言拒绝。
/// 表驱动而非「合法一份、非法一份」：加边漏改时两侧必有一侧变红。
const LEGAL: [(EffectState, EffectState); 13] = [
    (EffectState::Planned, EffectState::Authorized),
    (EffectState::Planned, EffectState::Failed),
    (EffectState::Planned, EffectState::RolledBack),
    (EffectState::Authorized, EffectState::Executing),
    (EffectState::Authorized, EffectState::Failed),
    (EffectState::Authorized, EffectState::RolledBack),
    (EffectState::Executing, EffectState::Committed),
    (EffectState::Executing, EffectState::Failed),
    (EffectState::Executing, EffectState::Unknown),
    (EffectState::Unknown, EffectState::Committed),
    (EffectState::Unknown, EffectState::Failed),
    (EffectState::Unknown, EffectState::RolledBack),
    (EffectState::Unknown, EffectState::Unknown),
];

/// 变体在 [`EffectState::ALL`] 中应处的下标。
///
/// 这个 match **穷尽且无通配臂**：给 `EffectState` 加变体而不同步更新它，
/// 整个测试目标**编译失败**，作者被迫回到本文件——而紧邻处就是数目、重复、
/// 覆盖三条断言。编译失败之所以必需：`ALL` 没同步时长度不变，
/// `assert_eq!(…len(), 7)` 单靠自己照过。
///
/// **残留缺口（据实记录）**：作者若只补上本函数的臂而不动 `ALL`，上一条用例
/// 仍会全绿——新变体不在 `ALL` 里，循环就永远走不到它。关掉这个缺口只能让
/// `ALL` 由变体清单生成（宏），或依赖 `std::mem::variant_count`（该函数在本
/// 工具链上仍是 unstable，见 rustc E0658）。当前实现选择编译期拦截 + 用例断言
/// 两层，而非改枚举的定义形态；`continuum-graph` 的 `NodeState` 连编译期拦截
/// 都没有，故本 crate 不比既有更弱。
fn ordinal(state: EffectState) -> usize {
    match state {
        EffectState::Planned => 0,
        EffectState::Authorized => 1,
        EffectState::Executing => 2,
        EffectState::Committed => 3,
        EffectState::Failed => 4,
        EffectState::RolledBack => 5,
        EffectState::Unknown => 6,
    }
}

#[test]
fn the_documented_transitions_are_accepted() {
    for (from, to) in LEGAL {
        assert_eq!(
            transition(from, to),
            Ok(to),
            "{from:?} → {to:?} 在迁移表内，应被接受"
        );
    }
}

#[test]
fn every_pair_is_accepted_iff_it_is_in_the_table() {
    // 七个状态的全部四十九对。表内十三对接受，表外三十六对拒绝；
    // 两侧由 `LEGAL` 同一处数据驱动，不存在只有一侧被覆盖的对。
    for from in EffectState::ALL {
        for to in EffectState::ALL {
            if LEGAL.contains(&(from, to)) {
                assert_eq!(
                    transition(from, to),
                    Ok(to),
                    "{from:?} → {to:?} 在表内，应被接受"
                );
            } else {
                assert_eq!(
                    transition(from, to),
                    Err(StateError::Illegal { from, to }),
                    "{from:?} → {to:?} 不在表内，应被拒绝"
                );
            }
        }
    }
}

#[test]
fn a_transition_outside_the_table_is_rejected() {
    let from = EffectState::Committed;
    let to = EffectState::Executing;
    let err = transition(from, to).unwrap_err();
    // 必须断 `from`/`to` 字段，不能只断变体：`StateError` 只有一个变体，
    // 只断变体等价于 `is_err()`，连 `Illegal { from: Planned, to: Planned }` 都通过。
    assert_eq!(err, StateError::Illegal { from, to }, "实际 {err:?}");
}

#[test]
fn the_three_terminal_states_have_no_outgoing_edges() {
    for from in [EffectState::Committed, EffectState::Failed, EffectState::RolledBack] {
        for to in EffectState::ALL {
            assert!(
                transition(from, to).is_err(),
                "{from:?} 是终态，不应有出边"
            );
        }
    }
}

#[test]
fn all_lists_every_variant_exactly_once() {
    // 与 P1 同法（`continuum-graph/tests/state_machine.rs` 的
    // `assert_eq!(NodeState::ALL.len(), 13)`），并多一层覆盖断言。
    //
    // `ALL` 是 `the_three_terminal_states_have_no_outgoing_edges` 的唯一驱动器：
    // 给 `EffectState` 加变体却漏加进 `ALL`，那条用例会静默少测而无人发现。
    // 本用例是第一道拦截——`ordinal` 的穷尽 match 先让漏改编译不过（见其注释，
    // 那里同时记着本拦截盖不住的那一种情形），作者回来后由下面三条断言判定
    // `ALL` 是否恰好是全部变体的一个排列。
    assert_eq!(EffectState::ALL.len(), 7, "ALL 与 EffectState 的变体数不符");

    let mut seen = vec![false; EffectState::ALL.len()];
    for state in EffectState::ALL {
        let i = ordinal(state);
        assert!(
            i < seen.len(),
            "{state:?} 的下标 {i} 越出 ALL 的长度 {}",
            seen.len()
        );
        assert!(!seen[i], "{state:?} 在 ALL 中出现了两次");
        seen[i] = true;
    }
    assert!(
        seen.iter().all(|s| *s),
        "ALL 漏了下标 {:?} 对应的变体",
        seen.iter().position(|s| !s)
    );
}
