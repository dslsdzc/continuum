//! G 这一侧的失败面：分类表与 `ModelCallError` 的照片（设计 §6.1、§11）。
//!
//! **本文件本 task 只有分类那部分**：四段流程（取快照、调排序、`call`、`call_stream`）
//! 的用例由后续 task 追补。

use continuum_core::ProviderError;
use continuum_graph::failure::FailureClass;
use continuum_runtime::model_call::{classify, into_call_error};
use continuum_runtime::ModelCallError;

/// `ProviderError` 的五个变体**逐项**各得一个类别（设计 §6.1 的表）。
///
/// **逐项各写一次断言，不抽代表**：这张表的实现是手写的 `match`，
/// 每个臂能各自漂移，一条表驱动的循环会把「某一臂被改错」藏在别的臂后面。
/// 照片形态因此是**五条独立的断言**，而不是「一个循环里断言五次」。
#[test]
fn each_provider_error_variant_maps_to_its_class() {
    assert_eq!(
        classify(&ProviderError::Transport("连接被重置".into())),
        Some(FailureClass::Transient),
        "传输失败通常是连接抖动 → Transient"
    );
    assert_eq!(
        classify(&ProviderError::Unavailable("上游 503".into())),
        Some(FailureClass::Transient),
        "供应商侧暂时不可用 → Transient"
    );
    assert_eq!(
        classify(&ProviderError::Protocol("响应不合契约".into())),
        Some(FailureClass::Permanent),
        "契约不符、重试无益 → Permanent"
    );
    assert_eq!(
        classify(&ProviderError::UnknownModel("登记表里的 id 与适配器对不上".into())),
        Some(FailureClass::Permanent),
        "配置缺陷 → Permanent"
    );
    assert_eq!(
        classify(&ProviderError::Cancelled("调用方发起的取消".into())),
        None,
        "调用方自己发的取消不是失败，不进入分类（设计 §6.1）"
    );
}

/// **两侧对钉**：同一次转换里，`Cancelled` 得 `None`、`Transport` 得 `Some(..)`。
///
/// **为什么这条要单列**：只钉「四个失败臂各得类别」会漏掉另一侧——
/// 本条的 `None` 那一侧把「取消不是失败」这条判定放进了**体**里。
/// **两侧各挡一边，故两侧都要在**：一个只钉一侧的用例挡不住「某一侧恒真」的实现
/// （两个方向各举得出一个仍能编译的变异体，红分别落在本条的哪一侧，见下）。
///
/// **爆炸面据实写：不是「只有本条会红」，是跨用例三处。** 两个方向的变异各红三处：
///
/// - 把 `classify` 的 `Cancelled` 那一臂改成 `Some(FailureClass::Transient)` ⇒ 红在
///   `each_provider_error_variant_maps_to_its_class` 里 `Cancelled` 那条断言、
///   **本条的 `None` 那一侧**、以及 `the_error_type_carries_the_provider_error_verbatim`
///   里 `Cancelled` 那一臂的 `panic!`（它拿到的是 `Provider { class: Transient, … }`）；
/// - 把 `Transport` 那一臂改成 `None` ⇒ 红在 `each_…` 里 `Transport` 那条断言、
///   **本条的 `Some(..)` 那一侧**、以及 `the_error_type_…` 里 `Transport` 那一臂的 `panic!`。
///
/// **三处的成因**：`each_…` 的第五条断言与 `the_error_type_…` 的第五臂断的是同一件事，
/// 而 `into_call_error` 的类别与分支都取自 `classify`，故表一改、类型的搬运跟着改。
/// **故本条的价值不在「唯一会红的用例」**，而在把两侧放进**同一个体**——
/// 只有这样才能对「某一侧恒真」的实现在同一处同时立起两侧守卫。
///
/// **上文的「红在哪一处」用断言锚点写、不写行号**：行号会被本文件的下一次编辑平移
/// （本注释自己就在被引断言的上方），锚点不会。
#[test]
fn a_cancelled_call_is_not_a_failure() {
    let cancelled = classify(&ProviderError::Cancelled("调用方发起的取消".into()));
    let transport = classify(&ProviderError::Transport("连接被重置".into()));

    assert_eq!(
        cancelled, None,
        "Cancelled 不是失败：调用方自己发的中止不该被记成一次失败（设计 §6.1）"
    );
    assert_eq!(
        transport,
        Some(FailureClass::Transient),
        "Transport 仍是失败——这一侧挡住「一律返回 None」的实现"
    );
}

/// `ModelCallError` **不压平** `ProviderError`：`Provider { class, source }` 的 `source`
/// 是给进去的那一枚（**逐变体各断一次**）；`Cancelled` 走 `ModelCallError::Cancelled`
/// 那一枚，**且那一枚不带 `class` 字段**。
///
/// 设计 §6.1 明写 `Routing` 要**带出 D 的错**、`Provider` 要**带出类别**，
/// 两者都不是「一枚同名的变体」——本条就是这条判据的照片。
///
/// **「`Cancelled` 不带 `class`」为什么值得单列**：它不是字段的取舍，它是
/// 「一次正常中止不会被记成一次失败」这条判据**在类型上的落点**——若 `Cancelled`
/// 也带 `class`，调用方会在一个没有失败的地方读到 `FailureClass`。
#[test]
fn the_error_type_carries_the_provider_error_verbatim() {
    // 一、Transport：`source` 逐字回读，`class` 是设计 §6.1 的那一格。
    match into_call_error(ProviderError::Transport("连接被重置".into())) {
        ModelCallError::Provider { class, source } => {
            assert_eq!(class, FailureClass::Transient, "Transport 的类别该是 Transient");
            assert_eq!(
                source,
                ProviderError::Transport("连接被重置".into()),
                "source 该是给进去的那一枚，逐字回读"
            );
        }
        other => panic!("Transport 该走 Provider 那一枚，实际 {other:?}"),
    }

    // 二、Unavailable。
    match into_call_error(ProviderError::Unavailable("上游 503".into())) {
        ModelCallError::Provider { class, source } => {
            assert_eq!(class, FailureClass::Transient, "Unavailable 的类别该是 Transient");
            assert_eq!(
                source,
                ProviderError::Unavailable("上游 503".into()),
                "source 该是给进去的那一枚，逐字回读"
            );
        }
        other => panic!("Unavailable 该走 Provider 那一枚，实际 {other:?}"),
    }

    // 三、Protocol。
    match into_call_error(ProviderError::Protocol("响应不合契约".into())) {
        ModelCallError::Provider { class, source } => {
            assert_eq!(class, FailureClass::Permanent, "Protocol 的类别该是 Permanent");
            assert_eq!(
                source,
                ProviderError::Protocol("响应不合契约".into()),
                "source 该是给进去的那一枚，逐字回读"
            );
        }
        other => panic!("Protocol 该走 Provider 那一枚，实际 {other:?}"),
    }

    // 四、UnknownModel。
    match into_call_error(ProviderError::UnknownModel("登记表里的 id 对不上".into())) {
        ModelCallError::Provider { class, source } => {
            assert_eq!(class, FailureClass::Permanent, "UnknownModel 的类别该是 Permanent");
            assert_eq!(
                source,
                ProviderError::UnknownModel("登记表里的 id 对不上".into()),
                "source 该是给进去的那一枚，逐字回读"
            );
        }
        other => panic!("UnknownModel 该走 Provider 那一枚，实际 {other:?}"),
    }

    // 五、Cancelled：走 `Cancelled` 那一枚，不是 `Provider` 那一枚。
    match into_call_error(ProviderError::Cancelled("调用方发起的取消".into())) {
        ModelCallError::Cancelled { source } => {
            assert_eq!(
                source,
                ProviderError::Cancelled("调用方发起的取消".into()),
                "source 该是给进去的那一枚，逐字回读"
            );
        }
        other => panic!("Cancelled 该走 Cancelled 那一枚，实际 {other:?}"),
    }

    // **「那一枚不带 `class` 字段」是本条里唯一的一张编译期照片**：
    // 下面这枚字面量只给 `source`。若 `Cancelled` 也长出一个 `class` 字段，
    // 这一行**编不过**（少给一个字段），故「不带 class」由编译通过本身钉住。
    // 它不能写成运行期断言——「一个字段不存在」不是运行期可观察的性质。
    let _cancelled_has_no_class_field = ModelCallError::Cancelled {
        source: ProviderError::Cancelled("调用方发起的取消".into()),
    };
}
