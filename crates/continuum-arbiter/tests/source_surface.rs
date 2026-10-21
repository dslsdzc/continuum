mod support;

use continuum_arbiter::{
    ArbiterPolicy, ResourceArbiter, ResourceDecision, ResourceObservation, SuspensionState,
    WorkloadDeclaration,
};

fn file<'a>(files: &'a [(String, String)], name: &str) -> &'a str {
    &files
        .iter()
        .find(|(file, _)| file.as_str() == name)
        .unwrap_or_else(|| panic!("src/ 缺少 {name}"))
        .1
}

#[test]
fn the_source_helper_strips_comments_and_strings() {
    let source = "// priority\nlet x = \"PriorityTier\"; /* pid */ let y = 1;";
    let code = support::code_only(source);
    assert!(!code.contains("priority"));
    assert!(!code.contains("PriorityTier"));
    assert!(!code.contains("pid"));
    assert!(code.contains("let y = 1;"));
}

#[test]
fn the_source_helper_handles_raw_strings_and_nested_braces() {
    let source = r##"let s = r#"PriorityTier } {"#; fn f() { let x = 1; }"##;
    let code = support::code_only(source);
    assert!(!code.contains("PriorityTier"), "raw string 内容必须被抹掉");
    assert!(code.contains("fn f()"), "raw string 之后的代码必须保留");

    let text = "pub struct S { a: u8, b: u8 }\n\
                impl S { pub fn a(&self) -> u8 { self.a } pub fn b(&self) -> u8 { self.b } }";
    let fields = support::struct_block(text, "pub struct S");
    assert_eq!(support::field_names(fields), vec!["a", "b"]);
    let block = support::impl_block(text, "impl S");
    assert_eq!(
        support::public_fn_names(block),
        vec!["a", "b"],
        "嵌套花括号（方法体）必须被正确配对"
    );
    let blocks = support::impl_blocks("impl S { } impl S { pub fn x() {} }", "impl S");
    assert_eq!(blocks.len(), 2, "impl_blocks 必须返回同一文件的全部 impl 块");
}

#[test]
fn src_files_reads_every_rust_file_in_src() {
    let names: Vec<String> = support::src_files()
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    let on_disk: Vec<String> = std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/src"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".rs"))
        .collect();
    for name in on_disk {
        assert!(
            names.contains(&name),
            "src_files 必须用 read_dir 全量读到 {name}"
        );
    }
}

#[test]
fn summary_has_exactly_two_public_accessors() {
    let files = support::code_files();
    let hits: Vec<_> = files
        .iter()
        .filter(|(_, text)| text.contains("impl ResourceSummary"))
        .collect();
    assert_eq!(hits.len(), 1, "impl ResourceSummary 必须恰一处");
    let block = support::impl_block(&hits[0].1, "impl ResourceSummary");
    assert_eq!(
        support::public_fn_names(block),
        vec!["resource_pressure", "workload_class"]
    );
    let summary = file(&files, "summary.rs");
    let fields = support::struct_block(summary, "pub struct ResourceSummary");
    assert_eq!(
        support::field_names(fields),
        vec!["resource_pressure", "workload_class"]
    );
    assert!(!fields.contains("pub "), "ResourceSummary 字段必须私有");
}

#[test]
fn summary_source_has_no_priority_or_process_terms() {
    let files = support::code_files();
    let summary = file(&files, "summary.rs");
    let fields = support::struct_block(summary, "pub struct ResourceSummary");
    let block = support::impl_block(summary, "impl ResourceSummary");
    for token in [
        "priority",
        "PriorityTier",
        "process_name",
        "process_id",
        "pid",
        "cpu_usage",
        "gpu_usage",
    ] {
        assert!(
            !support::contains_token(fields, token),
            "summary 字段出现 {token}"
        );
        assert!(
            !support::contains_token(block, token),
            "summary impl 出现 {token}"
        );
    }
}

#[test]
fn decision_has_exactly_three_public_accessors_and_no_priority() {
    let files = support::code_files();
    let hits: Vec<_> = files
        .iter()
        .filter(|(_, text)| text.contains("impl ResourceDecision"))
        .collect();
    assert_eq!(hits.len(), 1, "impl ResourceDecision 必须恰一处");
    let block = support::impl_block(&hits[0].1, "impl ResourceDecision");
    assert_eq!(
        support::public_fn_names(block),
        vec!["summary", "action", "phase"]
    );
    assert!(!block.contains("PriorityTier"));
    let arbiter = file(&files, "arbiter.rs");
    let fields = support::struct_block(arbiter, "pub struct ResourceDecision");
    assert_eq!(
        support::field_names(fields),
        vec!["summary", "action", "phase"]
    );
    assert!(!fields.contains("pub "), "ResourceDecision 字段必须私有");
    assert!(!fields.contains("priority") && !fields.contains("PriorityTier"));
}

#[test]
fn priority_tier_is_private_by_default_and_only_assembly_re_exports_it() {
    let files = support::src_files();
    let lib = file(&files, "lib.rs");
    let lines: Vec<&str> = lib.lines().map(str::trim).collect();
    assert!(lines.iter().any(|line| *line == "mod priority;"));
    assert!(!lines.iter().any(|line| *line == "pub mod priority;"));
    let assembly_line = lines
        .iter()
        .position(|line| *line == "pub mod assembly;")
        .expect("lib.rs 必须分行写 pub mod assembly;");
    let previous = lines[..assembly_line]
        .iter()
        .rev()
        .find(|line| !line.is_empty())
        .expect("assembly 的 cfg 属性必须在上一行");
    assert_eq!(
        *previous, "#[cfg(feature = \"assembly\")]",
        "assembly 模块必须由 cfg 属性门控（属性可与 pub mod 分行）"
    );
    let reexports: Vec<&str> = files
        .iter()
        .filter(|(_, text)| text.contains("pub use crate::priority::PriorityTier;"))
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(reexports, vec!["assembly.rs"]);
}

#[test]
fn priority_tier_is_confined_to_three_source_files() {
    let files = support::code_files();
    let hits: Vec<&str> = files
        .iter()
        .filter(|(_, text)| text.contains("PriorityTier"))
        .map(|(name, _)| name.as_str())
        .collect();
    assert!(
        hits.iter()
            .all(|name| matches!(*name, "priority.rs" | "assembly.rs" | "arbiter.rs")),
        "PriorityTier 出现在白名单外：{hits:?}"
    );
    let priority = file(&files, "priority.rs");
    assert_eq!(
        priority.matches("fn classify(").count(),
        1,
        "四档分类函数必须只有一个"
    );
}

#[test]
fn no_raw_process_information_in_public_types() {
    let files = support::code_files();
    let all: String = files.iter().map(|(_, text)| text.as_str()).collect();
    for token in [
        "process_name",
        "process_id",
        "cpu_usage",
        "gpu_usage",
        "command_line",
        "working_directory",
        "window_title",
    ] {
        assert!(!all.contains(token), "公开语料出现 {token}");
    }
    assert!(!support::contains_token(&all, "pid"), "公开语料出现 pid");
    assert_eq!(
        support::field_names(support::struct_block(
            file(&files, "pressure.rs"),
            "pub struct ResourceObservation"
        )),
        vec![
            "cpu",
            "ram",
            "gpu",
            "vram",
            "disk_io",
            "network",
            "workload_class"
        ]
    );
    assert_eq!(
        support::field_names(support::struct_block(
            file(&files, "arbiter.rs"),
            "pub struct ResourceDecision"
        )),
        vec!["summary", "action", "phase"]
    );
    assert_eq!(
        support::field_names(support::struct_block(
            file(&files, "state.rs"),
            "pub struct SuspensionState"
        )),
        vec![
            "phase",
            "attempts",
            "last_transition_millis",
            "checkpoint_deadline_millis"
        ]
    );
    let mut arbiter = ResourceArbiter::new(ArbiterPolicy::default()).unwrap();
    let summary = arbiter.observe(&ResourceObservation::default(), 0);
    assert!(!format!("{summary:?}").contains("Priority"));
    assert!(!format!("{summary:?}").contains("pid"));
    assert!(!format!("{summary:?}").contains("process"));
    let mut state = SuspensionState::new();
    let decision: ResourceDecision = arbiter
        .direct(
            WorkloadDeclaration {
                user_process: false,
                foreground: false,
                maintenance: false,
            },
            &mut state,
            0,
        )
        .unwrap();
    assert!(!format!("{decision:?}").contains("Priority"));
    assert!(!format!("{state:?}").contains("process"));
}

#[test]
fn no_resource_scheduler_budget_or_persistence_surface() {
    let files = support::code_files();
    let all: String = files.iter().map(|(_, text)| text.as_str()).collect();
    for token in [
        "continuum_provider",
        "continuum_model_registry",
        "continuum_node",
        "continuum_graph",
        "continuum_budget",
        "continuum_verify",
        "RoutingRequest",
        "PlacementRequest",
        "ModelProvider",
        "select_runnable",
        "Migration",
        "Tx",
        "persist",
    ] {
        assert!(!all.contains(token), "src/ 出现禁用符号 {token}");
    }
    for token in ["Evidence", "CompletionVerdict"] {
        assert!(
            !support::contains_token(&all, token),
            "src/ 出现证据面符号 {token}"
        );
    }
    let arbiter_blocks: Vec<&str> = files
        .iter()
        .flat_map(|(_, text)| support::impl_blocks(text, "impl ResourceArbiter"))
        .collect();
    assert!(!arbiter_blocks.is_empty(), "必须找到 impl ResourceArbiter 块");
    for block in arbiter_blocks {
        for method in ["place", "rank", "model", "tool", "parallelism"] {
            assert!(
                !support::public_fn_names(block).contains(&method),
                "ResourceArbiter 出现决策方法 {method}"
            );
        }
    }
}

#[test]
fn no_same_named_foreign_types() {
    let all: String = support::code_files()
        .iter()
        .map(|(_, text)| text.as_str())
        .collect();
    for token in [
        "NodeState",
        "IntentStatus",
        "TaskSource",
        "Decision",
        "ResourceDirective",
        "ResourceContext",
        "Budget",
    ] {
        assert!(
            !support::contains_token(&all, token),
            "src/ 出现同名异物 {token}"
        );
    }
}

#[test]
fn summary_from_parts_is_only_called_by_the_arbiter() {
    let all: String = support::code_files()
        .iter()
        .map(|(_, text)| text.as_str())
        .collect();
    assert!(
        all.contains("pub(crate) fn from_parts"),
        "from_parts 必须是 crate-private"
    );
    let files = support::code_files();
    let hits: Vec<&str> = files
        .iter()
        .filter(|(_, text)| text.contains("ResourceSummary::from_parts"))
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(hits, vec!["arbiter.rs"], "from_parts 只允许 arbiter.rs 调用");
}

#[test]
fn no_clock_random_process_filesystem_network_environment_or_unsafe() {
    let all: String = support::code_files()
        .iter()
        .map(|(_, text)| text.as_str())
        .collect();
    for token in [
        "SystemTime::now",
        "Instant::now",
        "UNIX_EPOCH",
        "thread_rng",
        "rand::",
        "std::process",
        "Command::new",
        "kill(",
        "std::fs",
        "std::net",
        "std::env",
        "unsafe",
        "Kill",
    ] {
        assert!(!all.contains(token), "src/ 出现禁用符号 {token}");
    }
}

#[test]
fn serialization_display_and_default_are_absent_from_conclusion_types() {
    let all: String = support::code_files()
        .iter()
        .map(|(_, text)| text.as_str())
        .collect();
    for token in ["Serialize", "Deserialize", "serde"] {
        assert!(!all.contains(token), "src/ 出现序列化面 {token}");
    }
    for ty in [
        "ResourceSummary",
        "ResourceDecision",
        "SuspensionState",
        "ResourceArbiter",
        "AssemblyView",
        "PriorityTier",
    ] {
        assert!(
            !all.contains(&format!("impl Display for {ty}")),
            "{ty} 不应实现 Display"
        );
    }

    fn assert_no_default(all: &str, ty: &str, decl: &str) {
        assert!(
            !all.contains(&format!("impl Default for {ty}")),
            "{ty} 不应 impl Default"
        );
        let start = all.find(decl).unwrap_or_else(|| panic!("找不到 {decl}"));
        let attrs: Vec<&str> = all[..start]
            .lines()
            .rev()
            .skip_while(|line| line.trim().is_empty())
            .take_while(|line| line.trim_start().starts_with("#["))
            .collect();
        assert!(
            !attrs.iter().any(|line| line.contains("Default")),
            "{ty} 的 derive 列表不得包含 Default：{attrs:?}"
        );
    }

    for (ty, decl) in [
        ("ResourceSummary", "pub struct ResourceSummary"),
        ("ResourceDecision", "pub struct ResourceDecision"),
        ("SuspensionState", "pub struct SuspensionState"),
        ("ResourceArbiter", "pub struct ResourceArbiter"),
        ("AssemblyView", "pub struct AssemblyView"),
        ("PriorityTier", "pub enum PriorityTier"),
        ("WorkloadDeclaration", "pub struct WorkloadDeclaration"),
    ] {
        assert_no_default(&all, ty, decl);
    }
}
