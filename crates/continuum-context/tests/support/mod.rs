//! 记忆用例共用的假检索面。**不写进 `src/`**；用到它的 integration test 顶部写
//! `mod support;`。`test_claim` 夹具在 P6C 的 `test-support` 预检通过后加入。

#![allow(dead_code)] // 各集成测试各自 `mod support;`，不是每个 helper 都被每个目标用到。

use std::cell::{Cell, RefCell};

use continuum_memory::{
    Confidence, MemoryClaim, MemoryClaimId, MemoryError, MemoryRetrieval, MemoryScope,
    MemorySourceId, RetrievalQuery, Validity,
};

pub struct CountingMemory {
    claims: Vec<MemoryClaim>,
    fail_with_empty_scope_set: bool,
    calls: Cell<usize>,
    last_query: RefCell<Option<QuerySnapshot>>,
}

#[derive(Clone)]
pub struct QuerySnapshot {
    pub scopes: Vec<MemoryScope>,
    pub anchor: Option<String>,
    pub text: Option<String>,
    pub limit: usize,
    pub include_terminal: bool,
}

impl CountingMemory {
    pub fn new(claims: Vec<MemoryClaim>) -> Self {
        CountingMemory {
            claims,
            fail_with_empty_scope_set: false,
            calls: Cell::new(0),
            last_query: RefCell::new(None),
        }
    }

    pub fn failing_with_empty_scope_set() -> Self {
        CountingMemory {
            claims: Vec::new(),
            fail_with_empty_scope_set: true,
            calls: Cell::new(0),
            last_query: RefCell::new(None),
        }
    }

    pub fn calls(&self) -> usize {
        self.calls.get()
    }

    pub fn last_query(&self) -> Option<QuerySnapshot> {
        self.last_query.borrow().clone()
    }
}

impl MemoryRetrieval for CountingMemory {
    fn retrieve(&self, query: &RetrievalQuery) -> Result<Vec<MemoryClaim>, MemoryError> {
        self.calls.set(self.calls.get() + 1);
        *self.last_query.borrow_mut() = Some(QuerySnapshot {
            scopes: query.scopes().to_vec(),
            anchor: query.anchor().map(str::to_string),
            text: query.text().map(str::to_string),
            limit: query.limit(),
            include_terminal: query.include_terminal(),
        });
        if self.fail_with_empty_scope_set {
            return Err(MemoryError::EmptyScopeSet);
        }
        Ok(self.claims.clone())
    }
}

/// 跨 crate 的 claim 夹具：七字段按 P6C §5.1 填，经 `test-support` 的
/// `for_test` 构造（不伪造、不写私有字段）。
pub fn test_claim(id: &str, statement: &str) -> MemoryClaim {
    MemoryClaim::for_test(
        MemoryClaimId::new(id),
        MemoryScope::Personal,
        statement,
        vec![MemorySourceId::new(format!("src-{id}"))],
        Validity::Active,
        Confidence::from_value(serde_json::json!(1.0)),
        0,
    )
}
