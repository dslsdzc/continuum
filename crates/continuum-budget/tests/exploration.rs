//! Task 18 的探索预算（§334 的落点、§12.5）的照片。
//!
//! | 用例 | 钉的判据 | 红条件（档位） |
//! |---|---|---|
//! | `each_of_the_four_default_criteria_has_a_pass_photo_and_a_gate_photo` | **四个默认判据各一条放行与各一条门控（共八条）**：`wall_time` / `money` / `network_transfer` / `gpu_time`（§15.2 第 10 条的「四个阈值」）；放行条取**恰等于阈值**的点（比较是 `≤`）；两个规范级默认（60 / 0）当场断言 | 某一格只实现一半（**取反档**——那一格的两条之中必有一条红）；或比较写成 `<`（放行条红） |
//! | `the_external_effect_gate_carries_no_dimension` | `has_external_effect = true` 那一格 `dimension` **为 `None`**（「外部副作用」是**一个种类，不是一个量**——判定属边界层）；枚举断言**逐项**，不靠「四格里三格顺带覆盖」 | 给它安一个量纲（**取反档**）；或顺带覆盖（逐枚断言处红） |
//! | `an_irreversible_action_is_gated_without_a_dimension` | `reversible = false` 同样 `dimension = None`（§334 的「可逆」未给量纲——必填入参） | 同上 |
//! | `a_tightened_policy_gates_an_otherwise_allowed_action` | 同一动作、两套阈值：`wall_time` 阈值调小 ⇒ 从 `AutoAllowed` 变 `DecisionRequired { dimension: Some(WallTime) }` | 判定不读阈值（**取反档**） |
//!
//! # 第 5 条（「必填入参」）不在本文件
//!
//! §7 末两行的 `network_transfer <= threshold` 与 `compute_cost <= threshold`
//! **原文只写 `threshold`、没有数值**（§16 第 21 条）——故它们是**必填入参**、
//! 本层**不发明这两个数**。这一条是**编译期**命题（「没有 `Default`」），照片落在
//! `tests/type_level.rs` 驱动的两份 trybuild 样例：反例
//! `exploration_thresholds_has_no_default` ＋ 正控制
//! `exploration_thresholds_needs_all_four_thresholds`。
//!
//! # 「策略真的收紧了」在本层不可观察（§15.2 第 10 条）
//!
//! 四个阈值可由策略**收紧**，而**策略的来源（谁配、配在哪）规范未给**——故
//! 「策略真的收紧了」这件事在本层**不可观察**，只有「**给定阈值时的判定**」可拍。
//! `a_tightened_policy_gates_an_otherwise_allowed_action` 拍的就是后者。
//!
//! # 本文件不需要库
//!
//! `check_exploration` 是纯函数（四个判据的输入全在参数里：`dims` ＋ `thresholds`
//! ＋两个必填 `bool`），故**不建 `Db`**——这是它与 `tests/validator.rs` 的形状差别。

use continuum_budget::{
    DimensionKind, Dimensions, ExplorationThresholds, ExplorationVerdict, check_exploration,
};

/// 五维的固定顺序（与 `src/dimensions.rs` 的枚举序一致）。
const FIVE: [DimensionKind; 5] = [
    DimensionKind::Money,
    DimensionKind::WallTime,
    DimensionKind::Token,
    DimensionKind::GpuTime,
    DimensionKind::NetworkTransfer,
];

/// 只有 `d` 这一维取值 `v`；其余四维从 `all(0)` 出发——即全 `Some(0)`
/// （可读、且在阈值内，不构成约束）。
fn only(d: DimensionKind, v: i64) -> Dimensions {
    let mut out = all(0);
    match d {
        DimensionKind::Money => out.money = Some(v),
        DimensionKind::WallTime => out.wall_time = Some(v),
        DimensionKind::Token => out.token = Some(v),
        DimensionKind::GpuTime => out.gpu_time = Some(v),
        DimensionKind::NetworkTransfer => out.network_transfer = Some(v),
    }
    out
}

/// 五维同值。
fn all(v: i64) -> Dimensions {
    Dimensions {
        money: Some(v),
        wall_time: Some(v),
        token: Some(v),
        gpu_time: Some(v),
        network_transfer: Some(v),
    }
}

// ===== 四个判据各一条放行、各一条门控（八条） =====

/// **逐项，共八条**：`wall_time` / `money` / `network_transfer` / `gpu_time`
/// 四个判据各一条放行与各一条门控，**不抽代表**。
///
/// # 放行条为什么取「恰等于阈值」的点
///
/// §7 的判据是 `≤`（`wall_time <= 60s`）；「恰等」同时钉住**包含侧**
/// （`<` 的实现会在放行条上红）与**该判据真的在用这个阈值**（不看阈值的实现
/// 会在门控条上红）。
///
/// # 两个规范级默认的来历
///
/// §7 的示例数值里给了三个：`wall_time ≤ 60s`、`money_cost = 0`、
/// `external_side_effect = none`；ENG-005 §三.1 采**前两个数**为**规范级默认**
/// （「且它是规范里**已有**的数字，不是新发明的」）。后两个阈值（network / gpu）
/// §7 只写 `threshold`——**必填入参**，由调用方以值给出（本用例给 1000）。
#[test]
fn each_of_the_four_default_criteria_has_a_pass_photo_and_a_gate_photo() {
    // 规范级默认在前两个字段上当场断言（其余两个必填）。
    let t = ExplorationThresholds::spec_defaults(1000, 1000);
    assert_eq!(
        t.wall_time, 60,
        "`wall_time` 的规范级默认是 §7 的 60（示例数值，ENG-005 采为规范级默认）"
    );
    assert_eq!(
        t.money, 0,
        "`money` 的规范级默认是 §7 的 0（\"= 0\" 就是 `≤ 0`）"
    );
    assert_eq!(t.network_transfer, 1000, "后两条阈值由调用方给（必填入参）");
    assert_eq!(t.gpu_time, 1000, "同上");

    // 放行四条（恰等于阈值；其余判据全合规：无外部副作用、可逆）。
    let pass_cases: [(DimensionKind, i64, &str); 4] = [
        (DimensionKind::WallTime, 60, "wall_time = 60：恰等于默认阈值"),
        (DimensionKind::Money, 0, "money = 0：恰等于默认阈值（\"= 0\"）"),
        (
            DimensionKind::NetworkTransfer,
            1000,
            "network_transfer = 1000：恰等于调用方给的阈值",
        ),
        (DimensionKind::GpuTime, 1000, "gpu_time = 1000：恰等于调用方给的阈值"),
    ];
    for (kind, value, why) in pass_cases {
        match check_exploration(&only(kind, value), &t, false, true) {
            ExplorationVerdict::AutoAllowed => {}
            other => panic!("（{kind:?}，{why}）应放行：{other:?}"),
        }
    }

    // 门控四条（恰好越界一格；断言门控带着**那一维**）。
    let gate_cases: [(DimensionKind, i64); 4] = [
        (DimensionKind::WallTime, 61),
        (DimensionKind::Money, 1),
        (DimensionKind::NetworkTransfer, 1001),
        (DimensionKind::GpuTime, 1001),
    ];
    for (kind, value) in gate_cases {
        match check_exploration(&only(kind, value), &t, false, true) {
            ExplorationVerdict::DecisionRequired { dimension } => {
                assert_eq!(
                    dimension,
                    Some(kind),
                    "（{kind:?} = {value}）门控带着那一维"
                );
            }
            other => panic!("（{kind:?} = {value}）应门控：{other:?}"),
        }
    }
}

// ===== 两格 `dimension = None`：枚举断言逐项 =====

/// `has_external_effect = true` 那一格 `dimension` 为 **`None`**。
///
/// **依据**：「外部副作用」是**一个种类、不是一个量**——它的判定属边界层
/// （Effect Journal / `EffectType`）。故它是**必填入参** `has_external_effect: bool`，
/// 由调用方以值给出。
///
/// # 枚举断言逐项，不靠「四格里三格顺带覆盖」
///
/// 四个**数值格**（`Some(Money)` / `Some(WallTime)` / `Some(GpuTime)` /
/// `Some(NetworkTransfer)`）各有自己的门控照片（上一条）；这一格的 `None`
/// **单独断言**：先断言它恰是 `DecisionRequired`，再断言 `dimension` 恰为 `None`，
/// 最后**逐枚**断言四个数值维都不出现在这一格（不靠别的格顺带覆盖）。
#[test]
fn the_external_effect_gate_carries_no_dimension() {
    let t = ExplorationThresholds::spec_defaults(1000, 1000);

    // 数值全合规（全 0）也挡不住：external 的门控是「种类」问题、不是「量」问题。
    match check_exploration(&all(0), &t, true, true) {
        ExplorationVerdict::DecisionRequired { dimension } => {
            assert_eq!(dimension, None, "external 门控**不带量纲**");
            for kind in FIVE {
                assert_ne!(
                    dimension,
                    Some(kind),
                    "（{kind:?}）这一格不得带任何量纲——逐项断言，不靠顺带覆盖"
                );
            }
        }
        other => panic!("有外部副作用必须门控：{other:?}"),
    }

    // 对照（这一格的反面）：`has_external_effect = false` 不是门控的理由。
    match check_exploration(&all(0), &t, false, true) {
        ExplorationVerdict::AutoAllowed => {}
        other => panic!("无外部副作用不是门控理由：{other:?}"),
    }
}

/// `reversible = false` 同样 `dimension = None`。
///
/// **依据**：§334 的「可逆」未给量纲（§12.5 的表末行：不在 `Dimensions` 里），
/// 故是**必填入参** `reversible: bool`。它与 external 门控共用 `None` 形状——
/// 设计写死的（`None` = 因「不可逆 / 有外部副作用」而门控）。
#[test]
fn an_irreversible_action_is_gated_without_a_dimension() {
    let t = ExplorationThresholds::spec_defaults(1000, 1000);

    match check_exploration(&all(0), &t, false, false) {
        ExplorationVerdict::DecisionRequired { dimension } => {
            assert_eq!(dimension, None, "不可逆门控**不带量纲**");
            for kind in FIVE {
                assert_ne!(
                    dimension,
                    Some(kind),
                    "（{kind:?}）这一格不得带任何量纲——逐项断言，不靠顺带覆盖"
                );
            }
        }
        other => panic!("不可逆动作必须门控：{other:?}"),
    }

    // 对照：`reversible = true` 不是门控的理由。
    match check_exploration(&all(0), &t, false, true) {
        ExplorationVerdict::AutoAllowed => {}
        other => panic!("可逆不是门控理由：{other:?}"),
    }
}

// ===== 策略收紧：同一动作、两个裁决 =====

/// 把 `wall_time` 阈值调小，**同一个动作**从 `AutoAllowed` 变
/// `DecisionRequired { dimension: Some(WallTime) }`。
///
/// # 「策略真的收紧了」在本层不可观察（§15.2 第 10 条）
///
/// 四个阈值可由策略收紧，而**策略的来源（谁配、配在哪）规范未给**——故本层能拍的
/// 只有「**给定阈值时的判定**」：本条用例把同一个动作喂给两套阈值，两个裁决都必须
/// 出来。**不发明**「策略怎么收紧」的机制（那不是本层的落点）。
#[test]
fn a_tightened_policy_gates_an_otherwise_allowed_action() {
    // 同一个动作：wall_time = 45（其余维不构成约束）。
    let action = only(DimensionKind::WallTime, 45);

    // 规范级默认（60）：45 ≤ 60 → 放行。
    let loose = ExplorationThresholds::spec_defaults(1000, 1000);
    match check_exploration(&action, &loose, false, true) {
        ExplorationVerdict::AutoAllowed => {}
        other => panic!("45 ≤ 60（规范级默认）应放行：{other:?}"),
    }

    // 策略收紧（wall_time 阈值 30）：45 > 30 → 门控（且带着那一维）。
    let tight = ExplorationThresholds {
        wall_time: 30,
        money: 0,
        network_transfer: 1000,
        gpu_time: 1000,
    };
    match check_exploration(&action, &tight, false, true) {
        ExplorationVerdict::DecisionRequired { dimension } => {
            assert_eq!(
                dimension,
                Some(DimensionKind::WallTime),
                "收紧后由 `wall_time` 那一维门控"
            );
        }
        other => panic!("45 > 30（收紧了）应门控：{other:?}"),
    }
}
