use std::path::Path;

const LIB: &str = include_str!("../src/lib.rs");
const DECISION: &str = include_str!("../src/decision.rs");
const INPUTS: &str = include_str!("../src/inputs.rs");
const GRAPH: &str = include_str!("../src/graph.rs");
const RESOURCE: &str = include_str!("../src/resource.rs");

fn src_texts() -> [(&'static str, &'static str); 5] {
    [
        ("lib.rs", LIB),
        ("decision.rs", DECISION),
        ("inputs.rs", INPUTS),
        ("graph.rs", GRAPH),
        ("resource.rs", RESOURCE),
    ]
}

/// 设计 §7.2 的源码面判据允许「只命中文档注释」；这里逐行去掉 `//` 开头的注释行。
/// grep 按行匹配，跨行折行会漏；**结论仍以人工通读为准**，测试只负责把已知词根钉住。
fn code_without_comments(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 取一条函数／trait 方法的签名：有函数体时截到第一个 `{`，trait 方法截到第一个 `;`。
/// `fn direct` 是 trait 方法，没有函数体；只按 `{` 截断会越过它、切到后面的 item。
fn signature<'a>(source: &'a str, needle: &str) -> &'a str {
    let start = source.find(needle).expect("签名应存在");
    let rest = &source[start..];
    let end = match (rest.find('{'), rest.find(';')) {
        (Some(brace), Some(semi)) => brace.min(semi),
        (Some(brace), None) => brace,
        (None, Some(semi)) => semi,
        (None, None) => panic!("签名后应有 `{{` 或 `;`"),
    };
    &rest[..end]
}

/// 把签名里的换行／多空格折叠成单空格，便于做子串断言。
fn flat(signature: &str) -> String {
    signature.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn assert_absent(source: &str, forbidden: &[&str]) {
    let code = code_without_comments(source);
    for needle in forbidden {
        assert!(!code.contains(needle), "源码面不得出现 {needle:?}");
    }
}

#[test]
fn lib_rs_only_declares_modules_and_reexports() {
    assert_absent(
        LIB,
        &["struct ", "enum ", "impl ", "fn ", "const ", "static "],
    );
    for module in ["decision", "graph", "inputs", "resource"] {
        assert!(LIB.contains(&format!("pub mod {module};")), "lib.rs 应声明 {module}");
    }
}

#[test]
fn the_source_tree_has_exactly_the_five_design_modules() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files: Vec<String> = std::fs::read_dir(dir)
        .expect("src 目录应可读")
        .map(|entry| {
            entry
                .expect("目录项应可读")
                .file_name()
                .to_str()
                .expect("文件名应是 UTF-8")
                .to_owned()
        })
        .collect();
    files.sort();
    assert_eq!(
        files,
        ["decision.rs", "graph.rs", "inputs.rs", "lib.rs", "resource.rs"]
    );
    for module in ["decision", "graph", "inputs", "resource"] {
        assert!(LIB.contains(&format!("pub mod {module};")), "lib.rs 应声明 {module}");
    }
}

#[test]
fn no_empty_intent_id_sentinel_in_source() {
    for (name, source) in src_texts() {
        let code = code_without_comments(source);
        assert!(
            !code.contains("IntentId::from_str(\"\")"),
            "{name} 不得用空串 IntentId 当哨兵"
        );
    }
}

#[test]
fn priority_is_not_parsed_in_source() {
    for (name, source) in src_texts() {
        let code = code_without_comments(source);
        for needle in ["intent.priority", "candidate.priority", "Ord for Priority"] {
            assert!(!code.contains(needle), "{name} 不得解析 priority：{needle}");
        }
        for line in code.lines() {
            if line.contains("priority") {
                assert!(!line.contains("sort_by"), "{name} 不得按 priority 排序：{line}");
                assert!(!line.contains("cmp"), "{name} 不得比较 priority：{line}");
            }
            if line.contains("Priority") {
                assert!(
                    !line.contains("PartialOrd"),
                    "{name} 不得为 Priority 派生比较能力：{line}"
                );
            }
        }
    }
}

#[test]
fn workload_class_and_pressure_are_not_parsed_in_source() {
    for (name, source) in src_texts() {
        let code = code_without_comments(source);
        for needle in [
            "workload_class",
            "resource_pressure",
            "INTERACTIVE_GAME",
            "COMPUTE_WORKLOAD",
            "COMPILER",
            "VIDEO_RENDER",
        ] {
            assert!(!code.contains(needle), "{name} 不得解析 {needle}");
        }
    }
}

#[test]
fn no_intent_storage_or_transition_surface_in_source() {
    for (name, source) in src_texts() {
        let code = code_without_comments(source);
        for needle in [
            "load_intent",
            "list_children",
            "save_intent",
            "apply_intent_transition",
            "continuum_persist",
        ] {
            assert!(!code.contains(needle), "{name} 不得出现 {needle}");
        }
    }
}

#[test]
fn the_three_decision_faces_have_distinct_signatures() {
    let decide = flat(signature(&code_without_comments(DECISION), "pub fn decide"));
    assert!(decide.contains("IntentDecision"), "decide 返回 IntentDecision");
    assert!(decide.contains("IntentDecisionError"), "decide 返回 IntentDecisionError");
    assert!(!decide.contains("NodeId"), "decide 不碰执行层 NodeId");
    assert!(!decide.contains("ResourceDirective"), "decide 不碰资源指令");

    let runnable = flat(signature(&code_without_comments(GRAPH), "pub fn runnable_nodes"));
    assert!(runnable.contains("Vec<NodeId>"), "runnable_nodes 返回 Vec<NodeId>");
    assert!(!runnable.contains("IntentDecision"), "runnable_nodes 不碰 Intent 决策");
    assert!(!runnable.contains("ResourceDirective"), "runnable_nodes 不碰资源指令");

    let direct = flat(signature(&code_without_comments(RESOURCE), "fn direct("));
    assert!(direct.contains("ResourceDirective"), "direct 返回 ResourceDirective");
    assert!(
        direct.contains("ResourceDirectiveError"),
        "direct 返回 ResourceDirectiveError"
    );
    assert!(!direct.contains("IntentDecision"), "direct 不碰 Intent 决策");
    assert!(!direct.contains("Vec<NodeId>"), "direct 不返回图节点");

    let directive_for = flat(signature(&code_without_comments(RESOURCE), "pub fn directive_for"));
    assert!(
        directive_for.contains("Option<ResourceDirective>"),
        "directive_for 返回 Option<ResourceDirective>"
    );
    assert!(
        directive_for.contains("DirectiveForError"),
        "directive_for 返回 DirectiveForError"
    );
    // directive_for 是转交入口，明确排除在三决策面的返回类型比较之外：
    // 它的合法签名同时含 IntentDecision 参数与 ResourceDirective 返回。
}

#[test]
fn no_model_call_surface_in_source() {
    for (name, source) in src_texts() {
        let code = code_without_comments(source);
        for needle in [
            "ModelProvider",
            "InvokeRequest",
            "continuum_provider",
            "continuum_runtime",
        ] {
            assert!(!code.contains(needle), "{name} 不得出现模型调用面 {needle}");
        }
    }
}

#[test]
fn no_resource_scheduler_type_in_source() {
    for (name, source) in src_texts() {
        let code = code_without_comments(source);
        assert!(!code.contains("ResourceScheduler"), "{name} 不得出现 ResourceScheduler");
    }
}

#[test]
fn decision_source_names_no_execution_ids_or_resource_directives() {
    assert_absent(
        DECISION,
        &["NodeId", "ResourceDirective", "DirectiveForError"],
    );
}

#[test]
fn no_graph_logic_vocabulary_anywhere_in_source() {
    let mut all = String::new();
    for (_, source) in src_texts() {
        all.push_str(&code_without_comments(source));
        all.push('\n');
    }
    for needle in [
        "NodeState",
        "edges_to",
        "predecessors",
        "apply_blocking",
        "apply_unblocking",
        "transition(",
        "max_parallel",
    ] {
        assert!(!all.contains(needle), "src/ 全体不得出现图逻辑词汇 {needle}");
    }
}
