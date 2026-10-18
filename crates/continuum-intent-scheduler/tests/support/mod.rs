#![allow(dead_code)]

use std::cell::Cell;

use continuum_canonical::{EntityId, Opaque};
use continuum_intent_scheduler::{
    BackgroundSuitability, DecisionBasis, IntentDecision, IntentSelectionPolicy, SchedulerInputs,
    UserFocus,
};
use continuum_semantics::{
    ContractId, ContractRef, Intent, IntentId, IntentKind, IntentStatus, Priority,
};

pub fn intent(id: &str, status: IntentStatus) -> Intent {
    Intent {
        id: IntentId::from_str(id),
        version: 1,
        kind: IntentKind::Task,
        goal: EntityId::from_str("goal"),
        status,
        priority: Priority::from_str("p0"),
        created_at: 0,
        updated_at: 0,
        parent_intent: None,
        contract_id: ContractRef { id: ContractId::from_str("contract"), version: 1 },
        active_plan_id: None,
        graph_id: None,
        product_id: None,
        completion_predicate: Opaque::from_str("done"),
    }
}

pub fn inputs<'a>(candidates: &'a [Intent]) -> SchedulerInputs<'a> {
    SchedulerInputs {
        candidates,
        priority: &[],
        deadline: &[],
        blocking: &[],
        user_focus: UserFocus::None,
        background_suitability: BackgroundSuitability::from_projection("background"),
    }
}

pub fn advance(id: &str, basis: &str) -> IntentDecision {
    IntentDecision::Advance {
        intent: IntentId::from_str(id),
        basis: DecisionBasis::new(basis),
    }
}

pub fn no_intent(basis: &str) -> IntentDecision {
    IntentDecision::NoIntent { basis: DecisionBasis::new(basis) }
}

/// 去掉整行 `//` 注释；源码面用例共用（Task 5／6／7）。
pub fn code_without_comments(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 取函数体，按**花括号平衡**收尾。`directive_for` 开头的 `let ... else { ... }`
/// 不能被第一个 `}` 截断；trait 方法没有函数体，不能用于本函数。
pub fn function_body<'a>(source: &'a str, signature: &str) -> &'a str {
    let start = source.find(signature).expect("函数签名应存在");
    let open = start + source[start..].find('{').expect("函数应有函数体");
    let mut depth = 0usize;
    for (offset, ch) in source[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return &source[open + 1..open + offset];
                }
            }
            _ => {}
        }
    }
    panic!("函数体未闭合");
}

pub struct FixedPolicy {
    decision: Result<IntentDecision, DecisionBasis>,
    calls: Cell<usize>,
}

impl FixedPolicy {
    pub fn returning(decision: IntentDecision) -> Self {
        Self { decision: Ok(decision), calls: Cell::new(0) }
    }

    pub fn failing(basis: &str) -> Self {
        Self { decision: Err(DecisionBasis::new(basis)), calls: Cell::new(0) }
    }

    pub fn calls(&self) -> usize {
        self.calls.get()
    }
}

impl IntentSelectionPolicy for FixedPolicy {
    fn choose(&self, _inputs: &SchedulerInputs<'_>) -> Result<IntentDecision, DecisionBasis> {
        self.calls.set(self.calls.get() + 1);
        self.decision.clone()
    }
}
