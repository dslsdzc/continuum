//! Task 14：源码面 J12／J13-d／J16–J21 与零表复核。

mod support;

use support::code_only;

const SRC: [(&str, &str); 7] = [
    ("lib.rs", include_str!("../src/lib.rs")),
    ("task.rs", include_str!("../src/task.rs")),
    ("supervisor.rs", include_str!("../src/supervisor.rs")),
    ("session.rs", include_str!("../src/session.rs")),
    ("resource.rs", include_str!("../src/resource.rs")),
    ("trigger.rs", include_str!("../src/trigger.rs")),
    ("error.rs", include_str!("../src/error.rs")),
];

const CARGO: &str = include_str!("../Cargo.toml");

fn src_corpus() -> String {
    SRC.iter()
        .map(|(_, source)| code_only(source))
        .collect::<Vec<_>>()
        .join("\n")
}

fn assert_absent(corpus: &str, banned: &[&str], context: &str) {
    for needle in banned {
        assert!(
            !corpus.contains(needle),
            "{context} 不应出现 {needle}"
        );
    }
}

fn tuple_range(body: &str, subject: &str) -> (usize, usize) {
    let quoted = format!("\"{subject}\"");
    let subject_start = body
        .find(&quoted)
        .unwrap_or_else(|| panic!("ALLOWED 缺少 subject {subject}"));
    let open = body[..subject_start]
        .rfind('(')
        .unwrap_or_else(|| panic!("{subject} 左侧没有元组起点"));
    let mut depth = 0usize;
    let mut in_string = false;
    for (offset, ch) in body[open..].char_indices() {
        match ch {
            '"' => in_string = !in_string,
            '(' if !in_string => depth += 1,
            ')' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return (open + 1, open + offset);
                }
            }
            _ => {}
        }
    }
    panic!("{subject} 的元组没有配对的右括号");
}

fn quoted_strings(fragment: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    for ch in fragment.chars() {
        if ch == '"' {
            if in_string {
                values.push(std::mem::take(&mut current));
            }
            in_string = !in_string;
        } else if in_string {
            current.push(ch);
        }
    }
    values
}

#[test]
fn lib_rs_defines_no_types() {
    let source = code_only(include_str!("../src/lib.rs"));
    assert_absent(
        &source,
        &["pub struct", "pub enum", "pub trait", "impl "],
        "lib.rs",
    );
}

#[test]
fn allowed_row_has_exactly_five_entries() {
    let source = code_only(include_str!(
        "../../../crates/continuum-runtime/tests/dependency_direction.rs"
    ));
    let marker = "const ALLOWED: &[(&str, &[&str])] = &[";
    let start = source.find(marker).expect("找不到 ALLOWED 表头") + marker.len();
    let end = start + source[start..].find("\n];").expect("ALLOWED 缺少结尾");
    let body = &source[start..end];

    let (open, close) = tuple_range(body, "continuum-supervisor");
    let mut entries = quoted_strings(&body[open..close]);
    assert_eq!(
        entries.first().map(String::as_str),
        Some("continuum-supervisor"),
        "元组第一项必须是 subject"
    );
    entries.remove(0);
    assert_eq!(
        entries,
        [
            "continuum-arbiter",
            "continuum-canonical",
            "continuum-intent-scheduler",
            "continuum-memory",
            "continuum-semantics",
        ],
        "continuum-supervisor 行必须恰五项且按字母序"
    );

    let (runtime_open, runtime_close) = tuple_range(body, "continuum-runtime");
    let runtime_entries = quoted_strings(&body[runtime_open..runtime_close]);
    assert!(
        !runtime_entries
            .iter()
            .any(|entry| entry == "continuum-supervisor"),
        "continuum-runtime 行不得出现 continuum-supervisor"
    );
}

#[test]
fn zero_persistence_and_no_intent_table() {
    assert!(!CARGO.contains("continuum-persist"));
    let corpus = src_corpus();
    assert_absent(
        &corpus,
        &[
            "Migration",
            "Tx",
            "INSERT",
            "UPDATE",
            "load_intent",
            "list_children",
            "save_intent",
            "apply_intent_transition",
            "Event",
            "EventType",
        ],
        "src",
    );
    for (name, source) in [
        (
            "main.rs",
            include_str!("../../../crates/continuum-runtime/src/main.rs"),
        ),
        (
            "migrations.rs",
            include_str!("../../../crates/continuum-runtime/tests/migrations.rs"),
        ),
        (
            "startup.rs",
            include_str!("../../../crates/continuum-runtime/tests/startup.rs"),
        ),
    ] {
        assert!(
            !source.contains("continuum_supervisor") && !source.contains("continuum-supervisor"),
            "{name} 不应引用本 crate"
        );
    }
}

#[test]
fn no_user_behavior_or_model_paths() {
    let corpus = src_corpus();
    assert_absent(
        &corpus,
        &[
            "std::process",
            "Command::new",
            "window",
            "keyboard",
            "screen",
            "open files",
            "process_name",
            "pid",
            "continuum_provider",
            "ModelProvider",
            "InvokeRequest",
            "continuum_runtime",
        ],
        "src",
    );
    assert_absent(
        CARGO,
        &[
            "continuum-provider",
            "continuum-runtime",
            "continuum-model-registry",
            "continuum-node",
        ],
        "Cargo.toml",
    );
}

#[test]
fn no_second_intent_store_or_product_record() {
    let corpus = src_corpus();
    assert_absent(
        &corpus,
        &[
            "load_intent",
            "list_children",
            "save_intent",
            "apply_intent_transition",
            "continuum_product",
            "Product::",
            "Milestone",
            "load_product",
        ],
        "src",
    );
    assert!(!CARGO.contains("continuum-product"));
}

#[test]
fn no_shippable_or_readiness_types() {
    let corpus = src_corpus();
    assert_absent(
        &corpus,
        &["Shippable", "ProductReadiness", "CompletionVerdict"],
        "src",
    );
}

#[test]
fn no_second_resource_scheduler() {
    let corpus = src_corpus();
    assert_absent(
        &corpus,
        &[
            "rank(",
            "place(",
            "RoutingRequest",
            "PlacementRequest",
            "parallelism",
            "ResourceDirective",
        ],
        "src",
    );
}

#[test]
fn no_priority_tier_or_assembly_surface() {
    let corpus = src_corpus();
    assert_absent(
        &corpus,
        &["PriorityTier", "AssemblyView", "ResourceObservation", "assembly"],
        "src",
    );
    assert_absent(
        CARGO,
        &["PriorityTier", "AssemblyView", "ResourceObservation", "assembly"],
        "Cargo.toml",
    );
    assert_absent(
        &corpus,
        &[
            "impl CheckpointPort",
            "enum ResourceAction",
            "struct ResourceDecision",
            "struct SuspensionState",
            "struct ResourceArbiter",
        ],
        "src",
    );
}

#[test]
fn background_is_not_evidence() {
    let corpus = src_corpus();
    let has_standalone = corpus
        .split(|ch: char| !(ch.is_alphanumeric() || ch == '_'))
        .any(|token| token == "Evidence");
    assert!(!has_standalone, "src 不应出现独立的 Evidence 标识符");
    assert_absent(&corpus, &["CompletionVerdict", "continuum_verify"], "src");
    assert!(!CARGO.contains("continuum-verify"));
}

#[test]
fn suspension_state_is_owned_not_a_parameter() {
    let source = code_only(include_str!("../src/supervisor.rs"));
    assert!(
        source.contains("states: HashMap<TaskKey, SuspensionState>"),
        "supervisor 必须持有 states 内存表"
    );

    let mut search_from = 0;
    while let Some(offset) = source[search_from..].find("pub fn") {
        let start = search_from + offset;
        let rest = &source[start..];
        let end = rest.find('{').expect("pub fn 必须有函数体");
        let signature = &rest[..end];
        assert!(
            !signature.contains("SuspensionState"),
            "公开函数的参数不得出现 SuspensionState：{signature}"
        );
        search_from = start + end + 1;
    }

    let body = support::fn_body(&source, "checkpoint_timed_out");
    assert_eq!(
        body.matches(".checkpoint_timed_out(").count(),
        1,
        "checkpoint_timed_out 必须恰调一次 P7C 的同名方法：{body}"
    );
}

#[test]
fn src_uses_only_full_line_comments() {
    for (name, source) in SRC {
        assert!(
            !source.contains("/*") && !source.contains("*/"),
            "{name} 不得使用块注释"
        );
        for (index, line) in source.lines().enumerate() {
            if line.contains("//") {
                assert!(
                    line.trim_start().starts_with("//"),
                    "{name}:{} 出现行尾注释：{line}",
                    index + 1
                );
            }
        }
    }
}
