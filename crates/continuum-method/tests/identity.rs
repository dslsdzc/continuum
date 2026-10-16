//! Task 1 的照片：`MethodId` 与 `MethodError` 的形状（设计 §4.2／§4.4）。
//!
//! | 用例 | 钉的判据 | 红条件（档位） |
//! |---|---|---|
//! | `method_id_returns_the_string_it_was_built_from` | `as_str()` 逐字往返；**两枚夹具、各钉一侧**：`"TDD"`（含大写字母，照 §187 里 `software/TDD` 的真实形态）与 `"planning/规划"`（含非 ASCII 与斜杠） | 返回常量或对串做**大小写折叠**（**取反档**——`"TDD"` 那枚即红）；对串做**截断／按字节切**（**取反档**——非 ASCII 那枚即红） |
//! | `method_error_renders_its_id_in_the_message` | 两枚变体的 `to_string()` 各含其 id 子串——**钉的是 `MethodId: Display` 的必需性**（来历同 `crates/continuum-operator/src/definition.rs:36-37`） | 格式串改写成不带 `{id}` 的固定文案（**移除档**——两枚变体照旧编得过，只有本用例红） |
//! | `method_error_has_exactly_two_variants` | 两臂**穷尽解构、不带 `..`** | 加第三枚变体（或给两枚变体加字段）⇒ **编译失败**：**编译期照片**（纪律 1(c)：编译不过不算变红），**本条没有运行期红** |
//! | `method_error_equality_is_payload_sensitive` | 同一变体、不同 `id` 的两枚 `assert_ne!`；另一条（跨变体、同一 `id`）也 `assert_ne!`，但它是**恒真断言** | `PartialEq` 从派生改成手写、**只比变体判别式不比载荷**（**放宽档**——前一条断言红） |
//!
//! # 编译期照片的归属（纪律 1(c)）
//!
//! `method_error_has_exactly_two_variants` 的机制是**编译器强制穷尽的解构**：
//! 加第三枚 `MethodError` 变体时，那个不带 `..` 的两臂 `match` 编译失败——
//! **这不是运行期变红**，照实标注为编译期照片。本文件其余三条都有运行期红
//! （各自的档位见上表）。
//!
//! # `method_id_returns_the_string_it_was_built_from` 红条件的**射程**
//!
//! 到此为止：`"planning/规划"` 无大写字母、两枚首尾都无空白，故本用例
//! **抓不到 `trim()` 一类**——「对串做任何折叠」是过宽的说法，本文件不写。

use continuum_method::{MethodError, MethodId};

#[test]
fn method_id_returns_the_string_it_was_built_from() {
    // 两枚夹具、各钉一侧——两枚都不省：
    // 只留 "TDD" 漏掉截断那一面，只留非 ASCII 那枚漏掉大小写折叠那一面。

    // 含大写字母（§187 里 `software/TDD` 的真实形态）：`as_str` 若返回常量、
    // 或对串做大小写折叠（`to_lowercase` 一类）⇒ 本枚即红（"TDD" ≠ "tdd"）。
    let tdd = MethodId::new("TDD");
    assert_eq!(
        tdd.as_str(),
        "TDD",
        "含大写字母的 id 必须逐字往返（大小写折叠在这里即红）"
    );

    // 含非 ASCII 与斜杠：`as_str` 若对串做截断／按字节切（不用 `.chars()`、
    // 按字节下标切一类）⇒ 本枚即红。
    let planning = MethodId::new("planning/规划");
    assert_eq!(
        planning.as_str(),
        "planning/规划",
        "含非 ASCII 的 id 必须逐字往返（按字节切在这里即红）"
    );
}

#[test]
fn method_error_renders_its_id_in_the_message() {
    // 本条钉的是 `MethodId: Display` 的必需性：格式串以 `{id}` 引用该字段，
    // 缺 `Display` 时 `MethodError` 根本没有这段格式串可用。
    // 红条件（移除档）：把格式串改写成不带 `{id}` 的固定文案——两枚变体
    // 照旧编译得过，只有本用例红。
    let not_found = MethodError::NotFound {
        id: MethodId::new("nope"),
    }
    .to_string();
    assert!(
        not_found.contains("nope"),
        "NotFound 的文案必须渲染出它的 id（含子串 \"nope\"）：{not_found}"
    );

    let duplicate = MethodError::Duplicate {
        id: MethodId::new("dup"),
    }
    .to_string();
    assert!(
        duplicate.contains("dup"),
        "Duplicate 的文案必须渲染出它的 id（含子串 \"dup\"）：{duplicate}"
    );
}

#[test]
fn method_error_has_exactly_two_variants() {
    // **编译期照片**（纪律 1(c)：编译不过不算变红；本条没有运行期红）：
    // 下面两臂穷尽解构、`..` 一个都不许写——加第三枚变体即编译失败，
    // 给现有变体加字段同样编译失败。
    let err = MethodError::NotFound {
        id: MethodId::new("photo"),
    };
    match &err {
        MethodError::NotFound { id } => {
            assert_eq!(id.as_str(), "photo", "解构出的载荷就是构造时那一枚");
        }
        MethodError::Duplicate { id: _ } => {
            // 夹具是 NotFound，本臂运行期到不了；它在此的作用是让 match 穷尽
            // （带上完整的字段清单），把「恰两枚变体」变成编译期照片。
        }
    }
}

#[test]
fn method_error_equality_is_payload_sensitive() {
    // 前一条：同一变体、不同 `id` 的两枚必须不相等——本条钉的是 `PartialEq`
    // **对载荷敏感**（上一条只有编译期照片，给不出运行期可观察面）。
    // 红条件（放宽档）：把 `PartialEq` 从派生改成手写、只比变体判别式不比载荷
    // （`NotFound` 之间恒相等）⇒ 前一条断言红。
    assert_ne!(
        MethodError::NotFound {
            id: MethodId::new("a")
        },
        MethodError::NotFound {
            id: MethodId::new("b")
        },
        "同一变体、不同 id 的两枚必须不相等"
    );

    // 后一条（跨变体、同一 `id`）是**类型上的恒真断言**：Rust 里两个不同变体
    // 恒不相等，**它的红条件不存在**；保留只为把「跨变体不相等」这一事实写下，
    // 不计入本块的判据。（原写的「把两枚变体折叠成同一个值」不是可编译的变异体
    // ——合成一枚＝删变体 ⇒ 编译失败，正是纪律 1(c) 排除的形态。）
    assert_ne!(
        MethodError::NotFound {
            id: MethodId::new("same")
        },
        MethodError::Duplicate {
            id: MethodId::new("same")
        },
        "不同变体、同一 id 的两枚必须不相等"
    );
}
