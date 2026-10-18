//! Task 9 的源码面断言：J1-d（唯一构造点）、J9（不定义 P6B／P6C 类型）、
//! Review Focus 4 的第二半（无隐藏来源）与 Review Focus 5 的第二半（无投影面）。
//!
//! 零命中类断言只针对本文件写死的词根／模式，不做「整份文件没有任何…」的泛化；
//! 语料限定 `src/` 的六个源文件，不含 `tests/`（测试要用 `ContextTrust::` 比较、
//! 要用 `serde_json`）。命中行由 Task 10 Step 3 逐处通读复核。

/// 六个源文件，按声明依赖的书写序。`src_text()` 与逐文件断言共用这一份清单。
fn src_files() -> [(&'static str, &'static str); 6] {
    [
        ("src/lib.rs", include_str!("../src/lib.rs")),
        ("src/package.rs", include_str!("../src/package.rs")),
        ("src/reference.rs", include_str!("../src/reference.rs")),
        ("src/request.rs", include_str!("../src/request.rs")),
        ("src/candidate.rs", include_str!("../src/candidate.rs")),
        ("src/compile.rs", include_str!("../src/compile.rs")),
    ]
}

/// 六份源码拼成的语料。只用于跨文件的子串断言。
fn src_text() -> String {
    src_files()
        .iter()
        .map(|(_, text)| *text)
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn context_package_is_constructed_only_in_compile() {
    // ① 定义恰一处。
    assert_eq!(
        src_text().matches("pub struct ContextPackage {").count(),
        1,
        "`ContextPackage` 的定义必须恰一处"
    );

    // ② `compile.rs` 内的结构体字面量恰一处——唯一构造点。
    assert_eq!(
        include_str!("../src/compile.rs")
            .matches("ContextPackage {")
            .count(),
        1,
        "`src/compile.rs` 里的 ContextPackage 构造必须恰一处"
    );

    // ③ 全部源码行里，含 `ContextPackage {` 且不是 impl 头的行恰两处：定义＋构造。
    // `impl ContextPackage {` 是 impl 头，不是构造，必须先过滤掉。
    let construction_lines: Vec<(&str, &str)> = src_files()
        .iter()
        .flat_map(|(name, text)| text.lines().map(move |line| (*name, line)))
        .filter(|(_, line)| line.contains("ContextPackage {"))
        .filter(|(_, line)| !line.trim_start().starts_with("impl "))
        .collect();
    assert_eq!(
        construction_lines.len(),
        2,
        "定义＋唯一构造之外不得出现第三个 ContextPackage 字面量：{construction_lines:?}"
    );

    // ④ impl 块只允许在 package.rs 一处；compile.rs 零个 impl（J1-d 的明确一半）。
    for (name, text) in src_files() {
        let expected = usize::from(name == "src/package.rs");
        assert_eq!(
            text.matches("impl ContextPackage {").count(),
            expected,
            "{name} 的 impl ContextPackage 块数不对"
        );
    }

    // ⑤ 三个禁止构造入口扫六文件（与 Task 1 的同一断言在终态复跑）。
    for (name, text) in src_files() {
        for forbidden in ["pub fn new", "from_parts", "from_fields"] {
            assert!(
                !text.contains(forbidden),
                "{name} 不应出现禁止构造入口：{forbidden:?}"
            );
        }
    }

    // 无 `Default`、无任何 trait impl（含 `impl<'de> Deserialize<'de> for ContextPackage`）、
    // 无内嵌测试模块——测试全在 `tests/`，`src/` 语料里没有第二个构造点。
    let all = src_text();
    assert!(
        !all.contains("impl Default for ContextPackage"),
        "不得为 ContextPackage 实现 Default"
    );
    assert!(
        !all.contains("for ContextPackage"),
        "不得为 ContextPackage 实现任何 trait"
    );
    assert!(
        !all.contains("#[cfg(test)]"),
        "单元测试不得放进 src/：那会引入测试侧的第二个构造点"
    );
}

#[test]
fn the_crate_defines_none_of_p6b_or_p6c_types() {
    let all = src_text();
    for forbidden in [
        "enum ContextTrust",
        "enum ContextProvenance",
        "struct MemoryClaim",
        "enum MemoryScope",
        "impl ContextTrust",
        "impl ContextProvenance",
        "impl MemoryClaim",
        "NodeTrust",
        "PrivacyClass",
    ] {
        assert!(
            !all.contains(forbidden),
            "本 crate 不得定义／引用 P6B／P6C 的类型或同名异物：{forbidden:?}"
        );
    }
    // 本块不定义自己的错误类型（设计 §2.2、§5.6）：失败面原样透传 MemoryError。
    assert!(
        !all.contains("ContextError"),
        "本 crate 不得定义自己的错误类型"
    );
}

#[test]
fn labels_are_not_constructed_in_this_crate() {
    let all = src_text();
    assert!(
        !all.contains("ContextTrust::"),
        "标签构造字面量只属于 P6B；本 crate 只经 label() 取值"
    );
    assert!(
        all.contains("use continuum_trust"),
        "标签／来源类型的来源必须是 P6B 的 continuum_trust"
    );
}

#[test]
fn there_is_no_projection_serialization_or_persistence_surface() {
    let all = src_text();
    for forbidden in [
        "to_messages",
        "into_invoke_request",
        "InvokeRequest",
        "impl Serialize",
        "impl Deserialize",
        "Serialize for ContextPackage",
        "Deserialize for ContextPackage",
        "serde::Serialize",
        "serde::Deserialize",
        "continuum_persist",
        "continuum_events",
    ] {
        assert!(
            !all.contains(forbidden),
            "本 crate 不得出现投影／序列化／持久化面：{forbidden:?}"
        );
    }
}

#[test]
fn compile_reads_no_hidden_source() {
    let all = src_text();
    for forbidden in [
        "std::env",
        "env::var",
        "SystemTime",
        "Instant::now",
        "current_dir",
        "std::fs",
        "File::open",
    ] {
        assert!(
            !all.contains(forbidden),
            "compile 不得读时钟／环境／文件系统：{forbidden:?}"
        );
    }
}
