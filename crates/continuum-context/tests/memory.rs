//! Task 4：记忆检索、查询构造与 `MemoryError` 透传。
//!
//! 十二条用例：七条非 claim ＋ 五条 claim（claim 经 P6C 的 `test-support`
//! 构造面 `MemoryClaim::for_test` 造出）。

mod support;

use continuum_context::{ContextBudget, ContextRequest, compile};
use continuum_memory::{MemoryError, MemoryScope};
use continuum_trust::{ContextProvenance, ContextTrust};
use support::{CountingMemory, test_claim};

fn memory_request(
    scopes: Vec<MemoryScope>,
    anchor: Option<String>,
    memory_query: Option<String>,
) -> ContextRequest {
    ContextRequest {
        instructions: Vec::new(),
        references: Vec::new(),
        artifacts: Vec::new(),
        scopes,
        anchor,
        memory_query,
    }
}

fn budget(max_memories: usize) -> ContextBudget {
    ContextBudget {
        max_references: 0,
        max_artifacts: 0,
        max_memories,
    }
}

#[test]
fn the_retrieval_surface_is_not_called_when_scopes_are_empty() {
    let memory = CountingMemory::new(Vec::new());
    let request = memory_request(Vec::new(), None, Some("x".into()));

    let package = compile(request, budget(1), &memory).expect("空 scope 不构造查询，不是错误");

    assert_eq!(memory.calls(), 0, "scopes 为空时不得调检索面");
    assert!(package.memories().is_empty());
}

#[test]
fn the_retrieval_surface_is_not_called_when_max_memories_is_zero() {
    let memory = CountingMemory::new(Vec::new());
    let request = memory_request(
        vec![MemoryScope::Personal],
        None,
        Some("x".into()),
    );

    let package = compile(request, budget(0), &memory).expect("max_memories = 0 不是错误");

    assert_eq!(memory.calls(), 0, "max_memories = 0 时不得调检索面");
    assert!(package.memories().is_empty());
}

#[test]
fn the_query_carries_scopes_anchor_text_limit_and_terminal_false() {
    let memory = CountingMemory::new(Vec::new());
    let request = memory_request(
        vec![MemoryScope::Project],
        Some("p1".into()),
        Some("x".into()),
    );

    let package = compile(request, budget(2), &memory).expect("查询构造应成功");
    assert!(package.memories().is_empty(), "假实现返回空集");

    let query = memory.last_query().expect("检索面应收到查询");
    assert_eq!(query.scopes, vec![MemoryScope::Project]);
    assert_eq!(query.anchor.as_deref(), Some("p1"));
    assert_eq!(query.text.as_deref(), Some("x"));
    assert_eq!(query.limit, 2);
    assert!(!query.include_terminal, "include_terminal 必须写死 false");
}

#[test]
fn a_query_construction_rejection_is_passed_through() {
    let memory = CountingMemory::new(Vec::new());
    let request = memory_request(vec![MemoryScope::Project], None, None);

    let result = compile(request, budget(1), &memory);

    assert!(
        matches!(result, Err(MemoryError::AnchorRequired)),
        "anchored scope 缺 anchor 必须原样透传 AnchorRequired"
    );
    assert_eq!(memory.calls(), 0, "查询构造失败时不得调检索面");
}

#[test]
fn an_anchor_on_an_unanchored_scope_is_rejected() {
    let memory = CountingMemory::new(Vec::new());
    let request = memory_request(
        vec![MemoryScope::Personal],
        Some("p".into()),
        None,
    );

    let result = compile(request, budget(1), &memory);

    assert!(
        matches!(result, Err(MemoryError::AnchorNotAllowed)),
        "Personal 带 anchor 必须原样透传 AnchorNotAllowed"
    );
}

#[test]
fn a_retrieval_failure_is_passed_through() {
    let memory = CountingMemory::failing_with_empty_scope_set();
    let request = memory_request(vec![MemoryScope::Personal], None, None);

    let result = compile(request, budget(1), &memory);

    assert!(
        matches!(result, Err(MemoryError::EmptyScopeSet)),
        "检索失败必须原样透传，不得包装成第二枚错误类型"
    );
    assert_eq!(memory.calls(), 1);
}

#[test]
fn an_empty_memory_query_still_retrieves_and_normalizes_to_none() {
    // 两个假实现各自从 0 起计数；不共用一个计数器。
    let memory_none = CountingMemory::new(Vec::new());
    let package_none = compile(
        memory_request(vec![MemoryScope::Personal], None, None),
        budget(3),
        &memory_none,
    )
    .expect("None 查询应检索");
    assert_eq!(memory_none.calls(), 1);
    assert!(package_none.memories().is_empty());
    assert_eq!(
        memory_none.last_query().expect("应有查询").text,
        None,
        "None 原样传递"
    );

    let memory_empty = CountingMemory::new(Vec::new());
    let package_empty = compile(
        memory_request(
            vec![MemoryScope::Personal],
            None,
            Some(String::new()),
        ),
        budget(3),
        &memory_empty,
    )
    .expect("Some(\"\") 不是不检索的信号");
    assert_eq!(memory_empty.calls(), 1);
    assert!(package_empty.memories().is_empty());
    assert_eq!(
        memory_empty.last_query().expect("应有查询").text,
        None,
        "Some(\"\") 由 P6C 的 new 归一为 None"
    );
}

#[test]
fn a_claim_from_the_retrieval_surface_lands_in_memories() {
    let claim = test_claim("c1", "记忆内容");
    let memory = CountingMemory::new(vec![claim.clone()]);
    let request = memory_request(vec![MemoryScope::Personal], None, None);

    let package = compile(request, budget(3), &memory).expect("检索成功");

    assert_eq!(package.memories(), [claim]);
    assert!(package.instructions().is_empty());
    assert!(package.references().is_empty());
    assert!(package.artifacts().is_empty());
    assert_eq!(
        package.provenance(),
        [ContextProvenance::memory("c1")],
        "记忆片段的 provenance 只经 memory(claim_id) 构造"
    );
    assert_eq!(package.trust_labels(), [ContextTrust::Memory]);
}

#[test]
fn max_memories_one_calls_the_surface_once() {
    let claim = test_claim("c1", "唯一一条");
    let memory = CountingMemory::new(vec![claim]);
    let request = memory_request(vec![MemoryScope::Personal], None, None);

    let package = compile(request, budget(1), &memory).expect("检索成功");

    assert_eq!(memory.calls(), 1, "max_memories = 1 应调检索面一次");
    assert_eq!(package.memories().len(), 1);
}

#[test]
fn duplicate_claim_ids_keep_the_first() {
    let memory = CountingMemory::new(vec![
        test_claim("c1", "first"),
        test_claim("c1", "second"),
        test_claim("c2", "other"),
    ]);
    let request = memory_request(vec![MemoryScope::Personal], None, None);

    let package = compile(request, budget(3), &memory).expect("重复 id 静默丢弃，不是错误");

    assert_eq!(package.memories().len(), 2);
    let first = package
        .memories()
        .iter()
        .find(|claim| claim.id().as_str() == "c1")
        .expect("c1 应保留");
    assert_eq!(first.statement(), "first", "同一 id 保留首个");
}

#[test]
fn memories_are_truncated_to_max_memories() {
    let memory = CountingMemory::new(vec![
        test_claim("c1", "一"),
        test_claim("c2", "二"),
        test_claim("c3", "三"),
    ]);
    let request = memory_request(vec![MemoryScope::Personal], None, None);

    let package = compile(request, budget(2), &memory).expect("检索成功");

    assert_eq!(package.memories().len(), 2);
    assert!(
        package
            .memories()
            .iter()
            .all(|claim| claim.id().as_str() != "c3"),
        "被截断的第 3 条不得出现在包中"
    );
}

#[test]
fn memories_come_only_from_the_retrieval_surface() {
    // 请求里没有任何 claim 输入；检索面返回 C 时包里有 C。
    let claim = test_claim("c1", "来自检索面");
    let memory = CountingMemory::new(vec![claim.clone()]);
    let package = compile(
        memory_request(vec![MemoryScope::Personal], None, None),
        budget(2),
        &memory,
    )
    .expect("检索成功");
    assert_eq!(memory.calls(), 1);
    assert_eq!(package.memories(), [claim]);

    // 不调检索面时 memories 为空；计数与返回逐次对应。
    let unused_claim = test_claim("c2", "不应出现");
    let memory_not_called = CountingMemory::new(vec![unused_claim]);
    let package = compile(
        memory_request(Vec::new(), None, None),
        budget(2),
        &memory_not_called,
    )
    .expect("空 scope 不检索");
    assert_eq!(memory_not_called.calls(), 0);
    assert!(package.memories().is_empty());
}
