//! `Tool`（§252 的定义）与 `ToolProfile`（§87 的登记项，含定义）的取值与形状。
//!
//! 「登记项含定义、不与定义并列」是**结构**命题：`ToolProfile` 的字段只有
//! `tool` / `cost` / `latency` / `trust`，没有第二份 `id` / `version`——这件事由类型
//! 本身承载，运行期用例只能钉「取回来的定义与构造时一致」这一半。
//!
//! **`trust` 没有用例，也不该有**：它由类型表达（`ToolProfile` 没有收
//! `Option<Trust>` 的入口），而「某入口不存在」只有 `trybuild` 的编译失败样例能钉，
//! 本 task 不为它造。同一段理由留在 `ToolProfile` 的文档注释里，两处都留是因为
//! 后来者更可能先读到那一处。

use continuum_capability::{
    CapabilityKind, Cost, FsAction, GitAction, Latency, Tool, ToolId, ToolProfile, Trust,
};
use continuum_effect::EffectType;
use serde_json::{Value, json};

/// 样例工具的输入 schema。与 [`output_schema`] 不同：两个字段都是
/// `serde_json::Value`，构造时把二者互换在类型上完全合法，故取值必须分得开。
fn input_schema() -> Value {
    json!({
        "type": "object",
        "properties": { "message": { "type": "string" } },
        "required": ["message"]
    })
}

/// 样例工具的输出 schema。与 [`input_schema`] 不同，理由见那里。
fn output_schema() -> Value {
    json!({
        "type": "object",
        "properties": { "sha": { "type": "string" } },
        "required": ["sha"]
    })
}

/// 样例工具：除 `deterministic` 外，其余六个字段由 [`sample_tool`] 定死。
///
/// 七个字段里只有 `input_schema` / `output_schema` 是**同型**的一对（都是
/// `serde_json::Value`），把构造参数写反在类型上合法，故这一对取值必须可分——
/// `a_tool_carries_its_definition_fields` 为此另加一条互不相等断言。余下六个字段
/// 两两异型（`ToolId` / `String` / `Vec<CapabilityKind>` / `Option<EffectType>` /
/// `bool`），位置写反即编译不过，**给不出也不需要照片**。
fn sample_tool_with_deterministic(deterministic: bool) -> Tool {
    Tool::new(
        ToolId::new("git.push"),
        String::from("1.4.0"),
        input_schema(),
        output_schema(),
        vec![
            CapabilityKind::Filesystem(FsAction::Read),
            CapabilityKind::Git(GitAction::Push),
        ],
        Some(EffectType::PushBranch),
        deterministic,
    )
}

/// 样例工具，`deterministic = true`。
fn sample_tool() -> Tool {
    sample_tool_with_deterministic(true)
}

/// `Tool` 的七个字段原样带回（§252）。
///
/// 逐字段断言而不是抽一两个代表：七个访问器各读各的字段，手写映射能各自漂移。
/// `input_schema` / `output_schema` 那两条另加一条**互不相等**的断言——两个字段
/// 同型，把构造参数写反（或把访问器接错字段）在类型上合法，只有取值不同才看得见。
///
/// `deterministic` **两侧都钉**：两个工具**只在该字段上不同**（其余六个字段同源，
/// 见 [`sample_tool_with_deterministic`]），故断言可归因到这一个字段。只钉 `true`
/// 那一侧的话，一个恒返回 `true` 的实现照过——而 `false` 恰是出 bug 时会翻过去的那侧。
#[test]
fn a_tool_carries_its_definition_fields() {
    let tool = sample_tool();

    assert_eq!(tool.id().as_str(), "git.push", "id");
    assert_eq!(tool.version(), "1.4.0", "version");
    assert_eq!(tool.input_schema(), &input_schema(), "input_schema");
    assert_eq!(tool.output_schema(), &output_schema(), "output_schema");
    assert_ne!(
        tool.input_schema(),
        tool.output_schema(),
        "两个 schema 取值必须不同，否则接错字段也看不出来"
    );
    assert_eq!(
        tool.required_capabilities(),
        [
            CapabilityKind::Filesystem(FsAction::Read),
            CapabilityKind::Git(GitAction::Push),
        ],
        "required_capabilities（顺序与内容都在内）"
    );
    assert_eq!(
        tool.effect_class(),
        Some(EffectType::PushBranch),
        "effect_class"
    );

    assert!(tool.deterministic(), "deterministic = true 应原样带回");
    let nondeterministic = sample_tool_with_deterministic(false);
    assert!(
        !nondeterministic.deterministic(),
        "deterministic = false 应原样带回（只钉 true 一侧的话，恒返回 true 的实现照过）"
    );
}

/// 登记项含定义：`ToolProfile` 取回的 `Tool` 与构造时一致（设计 §3.1）。
///
/// 「含定义而非并列两份」的结构面由 `tool()` 的返回类型 `&Tool` 承载——并列两份
/// 写法的定义面本用例照不出来（那是类型层面的形状）；本用例钉住的是运行期那一半：
/// 登记项给出的定义就是放进去的那一个，`id` / `version` / `required_capabilities`
/// 三项逐项对齐。
#[test]
fn a_profile_contains_its_tool() {
    let profile = ToolProfile::new(sample_tool(), None, None, Trust);

    let registered = profile.tool();
    assert_eq!(registered.id().as_str(), "git.push", "登记项的 id");
    assert_eq!(registered.version(), "1.4.0", "登记项的 version");
    assert_eq!(
        registered.required_capabilities(),
        [
            CapabilityKind::Filesystem(FsAction::Read),
            CapabilityKind::Git(GitAction::Push),
        ],
        "登记项的能力清单"
    );
}

/// `effect_class: None` 即「不携带任何 `EffectType`」（设计 §3.2）。
///
/// 「任何」是**对整个封闭枚举**的绝对措辞，故逐项有照片：遍历 [`EffectType::ALL`]
/// 的六个类型，逐个断言 `None` 的工具不等于它。
///
/// 另一侧是承重的一格：带外部副作用的工具确实给出它的类型。没有这一格，一个恒返回
/// `None` 的实现也照过。
#[test]
fn effect_class_none_means_no_external_effect() {
    let pure = Tool::new(
        ToolId::new("text.count"),
        String::from("0.1.0"),
        input_schema(),
        output_schema(),
        vec![CapabilityKind::Filesystem(FsAction::Read)],
        None,
        true,
    );

    assert_eq!(pure.effect_class(), None, "无外部副作用的工具");
    for effect in EffectType::ALL {
        assert_ne!(
            pure.effect_class(),
            Some(effect),
            "None 的工具不该携带 {effect:?}"
        );
    }

    // 承重的另一侧：有副作用的工具给出它的类型。
    let pushing = Tool::new(
        ToolId::new("git.push"),
        String::from("1.4.0"),
        input_schema(),
        output_schema(),
        vec![CapabilityKind::Git(GitAction::Push)],
        Some(EffectType::PushBranch),
        true,
    );
    assert_eq!(
        pushing.effect_class(),
        Some(EffectType::PushBranch),
        "有外部副作用的工具应给出其 EffectType"
    );
}

/// `cost` / `latency` 的两种状态都可达、都照相（设计 §10 第 9 条走「给出可达 `Some`
/// 的路径」那一条）。
///
/// 可达路径是 [`ToolProfile::new`] 的 `cost` / `latency` 两个入参（`Some(Cost)` /
/// `Some(Latency)`），故「尚未登记画像」（`None`）与「已登记、取值域尚空」
/// （`Some(Cost)`，`Cost` 是单位结构体）在类型上分得开——这正是这两个字段取
/// `Option` 的理由（设计 §3.1）。
///
/// `Tool` 侧不参与：画像字段在登记项上，不在定义上（§87 与 §252 的分工）。
#[test]
fn cost_and_latency_record_whether_a_metric_was_registered() {
    let registered = ToolProfile::new(sample_tool(), Some(Cost), Some(Latency), Trust);
    let unregistered = ToolProfile::new(sample_tool(), None, None, Trust);

    assert_eq!(registered.cost(), Some(Cost), "已登记画像的 cost");
    assert_eq!(registered.latency(), Some(Latency), "已登记画像的 latency");
    assert_eq!(unregistered.cost(), None, "尚未登记画像的 cost");
    assert_eq!(unregistered.latency(), None, "尚未登记画像的 latency");

    // 两种状态可分：没有这两条，一个把画像字段恒置于 `None`（或恒置于 `Some`）的
    // 实现也照过——上面四条会各自只看一侧。
    assert_ne!(
        registered.cost(),
        unregistered.cost(),
        "「已登记」与「尚未登记」必须是两个可观察的状态"
    );
    assert_ne!(
        registered.latency(),
        unregistered.latency(),
        "「已登记」与「尚未登记」必须是两个可观察的状态"
    );
}
