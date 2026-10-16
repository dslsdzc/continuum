//! 17 枚媒体算子（设计 §5.2）的清单、次序与注册入口的守卫。
//!
//! 语料（17 枚 id 清单）**手写**、逐行取自设计 §5.2 的表：若把期望值写成按
//! `all_operators()` 自己遍历，测的是「它等于它自己」——那是恒真的假照片
//! （设计 §12.2 末对 (f) 条的点名）。
//!
//! 本条文件**不**逐一断言 17 枚的全部各格（`input_schema` / `output_schema` /
//! `determinism` / backend 候选）：那属 Task 4 的 (h)（后端名单）与 (t)（端口边表）
//! 等用例；本文件钉的是 id 的次序、id 的互异、`version == 1`、注册入口的两侧，
//! 以及五枚具名算子的 `side_effect_class`。

use continuum_media::{MediaError, all_operators, register_media_operators};
use continuum_operator::{
    OperatorError, OperatorId, OperatorRegistry, OperatorVersion, SideEffectClass,
};

/// 设计 §5.2 表的行序（§5.1：「顺序即 §5.2 表的行序」），**手写照录**。
///
/// 这一份清单必须手写：它的对照物是设计原文的表，不是 `all_operators()` 自身
/// （否则次序断言变成恒真的假照片）。
const DESIGNED_IDS: [&str; 17] = [
    "media-import",
    "proxy-build",
    "shot-detect",
    "transcribe",
    "audio-analysis",
    "vision-analysis",
    "metadata-analysis",
    "narrative-plan",
    "timeline-compose",
    "effects-subtitle",
    "render",
    "generate-broll",
    "generate-voice",
    "generate-music",
    "frame-interpolation",
    "verify-deterministic",
    "verify-multimodal",
];

/// 设计 §5.1 的「顺序即 §5.2 表的行序」的照片：逐位相同。
///
/// 红条件（取反档）：把 `effects-subtitle` 与 `render` 两枚在数组里互换 ⇒ 红。
/// 这一条的用途：顺序不钉住，它就是一句**没有照片的绝对措辞**；且次序是
/// Task 4 的 (t) 边表的语料顺序。
#[test]
fn the_seventeen_ids_come_in_the_designed_order() {
    let operators = all_operators();
    let got: Vec<&str> = operators.iter().map(|op| op.id.as_str()).collect();
    assert_eq!(
        got, DESIGNED_IDS,
        "all_operators() 的 id 次序应为设计 §5.2 表的行序"
    );
}

/// 第 (f) 条：注册的正例——对手写的 17 枚 id **逐枚** `resolve` 成功。
///
/// 语料必须手写（设计 §12.2 末）：若写成按 `all_operators()` 自己遍历，
/// 就是恒真的假照片。
/// 红条件（放宽档）：把某一枚的 id 串改错（如 `shot-detect` → `shot_detect`）⇒
/// 该 id 的 `resolve` 返回 `NotFound`，红。（从数组里删一项 ⇒ 编译期红，
/// 不算运行期变红：返回类型是 `[Operator; 17]`。）
#[test]
fn the_seventeen_ids_are_hand_listed_and_each_resolves() {
    let mut registry = OperatorRegistry::new();
    register_media_operators(&mut registry, &all_operators()).expect("合法批次应全部注册成功");

    for id in DESIGNED_IDS {
        registry
            .resolve(&OperatorId::new(id), &OperatorVersion::new(1))
            .unwrap_or_else(|e| panic!("{id} 应在注册后可 resolve，实际 {e}"));
    }
}

/// 设计 §5.1 的不变量：id 逐枚不同、version 均为 1。
///
/// **照实记：`len() == 17` 这一半不承重**——`all_operators()` 的返回类型是
/// `[Operator; 17]`，在没有编译错误的前提下它**恒真**，是一个读数。
/// 本条真正承重的是**两两比较**与 `version == 1` 两条；「清单少了一枚」这一档的
/// 红由**返回类型**（编译期）与上一条的逐枚 `resolve` 承担，不由这一行承担。
/// 红条件（取反档）：把两枚算子的 id 写成同一个 ⇒ 两两比较红。（**注意**：那时
/// `register` 也会返回 `Duplicate`，两条用例会一起红——**这是刻意的**，
/// 它说明两处守卫的抓手不同。）
#[test]
fn all_operators_has_seventeen_distinct_ids_and_every_version_is_one() {
    let operators = all_operators();
    assert_eq!(operators.len(), 17, "清单的数目读数（并不承重，见注释）");

    for (i, earlier) in operators.iter().enumerate() {
        for later in &operators[i + 1..] {
            assert_ne!(
                earlier.id, later.id,
                "id 必须两两不同：{} 出现了两次",
                earlier.id
            );
        }
    }

    for op in &operators {
        assert_eq!(
            op.version,
            OperatorVersion::new(1),
            "{} 的 version 应为 1（设计 §5.1 的不变量）",
            op.id
        );
    }
}

/// 第 (g) 条：注册的反例——同一注册表注册两次同一批次 ⇒
/// `Err(MediaError::Registry(OperatorError::Duplicate { .. }))`（**不是**静默覆盖）。
///
/// 断言到**撞车的那一枚**的 `id` / `version`（不是「返回了 Err」）；并核
/// `#[error(transparent)]` 把 `OperatorError` 的文案原样透出。
/// 撞车的是批次第一枚 `media-import`：注册逐枚进行、任一失败即返回，
/// 第一枚就与首次注册留下的条目相撞。
/// 红条件（移除档）：把 `register_media_operators` 改成静默跳过已存在的算子 ⇒
/// 第二条返回 `Ok`，红。
#[test]
fn a_second_identical_registration_returns_the_registry_error() {
    let mut registry = OperatorRegistry::new();
    register_media_operators(&mut registry, &all_operators()).expect("首次注册应成功");

    let err = register_media_operators(&mut registry, &all_operators())
        .expect_err("同一批次注册两次必须被拒绝，不得静默跳过");

    assert_eq!(
        err.to_string(),
        "Operator media-import 版本 1 已注册",
        "transparent 应原样透出 OperatorError 的文案"
    );

    match err {
        MediaError::Registry(OperatorError::Duplicate { id, version }) => {
            assert_eq!(id.as_str(), "media-import", "撞车的应是批次第一枚");
            assert_eq!(version.as_u32(), 1);
        }
        // 本 task 的 MediaError 只有 Registry 一臂；内层非 Duplicate 即预期不成立。
        MediaError::Registry(other) => panic!("应为 OperatorError::Duplicate，实际 {other:?}"),
    }
}

/// 第 (n) 条：五枚的 `side_effect_class` **逐枚断言，不抽代表**
/// （设计 §5.2 的第 5、6 条理由逐枚给过判据）。
///
/// **本条守卫的射程**：它钉的是**这五枚**，不是「全部 17 枚的 `side_effect_class`
/// 都对」——其余 12 枚的取值没有任何用例钉得住（设计 §12.5 第 6 条；计划
/// `## 遗留` 第三节），本计划**不发明**那条全称。
/// 红条件（取反档）：把 `generate-broll` 改回 `NonIdempotent` ⇒ 该枚红。
#[test]
fn the_side_effect_class_of_the_five_named_operators_is_asserted_one_by_one() {
    let operators = all_operators();
    let class_of = |id: &str| {
        operators
            .iter()
            .find(|op| op.id.as_str() == id)
            .unwrap_or_else(|| panic!("清单里应有 {id}"))
            .side_effect_class
    };

    // 生成类的两枚是 Pure：产物是**一个值**，不改变本仓之外的状态
    //（「重跑得到另一段」是非确定性，不是副作用；设计 §5.2 第 5 条）。
    assert_eq!(
        class_of("generate-broll"),
        SideEffectClass::Pure,
        "generate-broll 应为 Pure（设计 §5.2 第 5 条）"
    );
    assert_eq!(
        class_of("frame-interpolation"),
        SideEffectClass::Pure,
        "frame-interpolation 应为 Pure（设计 §5.2 第 5 条）"
    );

    // 执行体自己有落盘动作的三枚是 Idempotent：改变本仓之外的状态，
    // 但重复执行得到同一状态（设计 §5.2 第 6 条）。
    assert_eq!(
        class_of("render"),
        SideEffectClass::Idempotent,
        "render 应为 Idempotent（设计 §5.2 第 6 条）"
    );
    assert_eq!(
        class_of("proxy-build"),
        SideEffectClass::Idempotent,
        "proxy-build 应为 Idempotent（设计 §5.2 第 6 条）"
    );
    assert_eq!(
        class_of("media-import"),
        SideEffectClass::Idempotent,
        "media-import 应为 Idempotent（设计 §5.2 第 6 条）"
    );
}
