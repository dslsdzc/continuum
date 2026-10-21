mod support;

use continuum_graph::{AdfirGraph, ContractIdRef, GraphId, SchedulerConfig, select_runnable};

const GRAPH_SRC: &str = include_str!("../src/graph.rs");

#[test]
fn an_empty_graph_yields_no_runnable_nodes() {
    let graph = AdfirGraph::new(GraphId::new("g"), ContractIdRef::new("c"));
    let config = SchedulerConfig::default();
    let nodes = continuum_intent_scheduler::runnable_nodes(&graph, &config);
    assert!(nodes.is_empty());
    assert_eq!(nodes, select_runnable(&graph, &config));
}

#[test]
fn the_graph_entry_is_a_zero_logic_transfer() {
    let code = support::code_without_comments(GRAPH_SRC);
    let body = support::function_body(&code, "pub fn runnable_nodes");
    assert_eq!(body.trim(), "continuum_graph::select_runnable(graph, config)");
    // 局部禁止词表；设计 J11-b 的 `src/` 全体扫描在 Task 7（本用例只守这一个函数体）。
    for forbidden in [
        "if ", "for ", "while ", "match ", "NodeState", "edges_to", "predecessors",
        "apply_blocking", "apply_unblocking", "transition(", "max_parallel",
    ] {
        assert!(!body.contains(forbidden), "runnable_nodes 的函数体不得包含 {forbidden:?}");
    }
}

#[test]
fn graph_source_mentions_neither_intent_nor_resource() {
    let code = support::code_without_comments(GRAPH_SRC);
    for forbidden in ["IntentDecision", "ResourceDirective", "DirectiveForError"] {
        assert!(!code.contains(forbidden), "src/graph.rs 不得出现 {forbidden}");
    }
}
