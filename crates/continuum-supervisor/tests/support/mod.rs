#![allow(dead_code)]

//! 共享夹具与源码语料工具。只放测试用的构造与文本处理，不放生产逻辑。

use continuum_canonical::{EntityId, Opaque};
use continuum_semantics::{
    ContractId, ContractRef, Intent, IntentId, IntentKind, IntentStatus, Priority, ProductIdRef,
};

/// 逐行丢弃整行注释后拼接；用于源码面的零命中断言。
pub fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 构造一枚最小的 Intent；所有字段由夹具写死，测试按需覆盖。
pub fn intent(kind: IntentKind, status: IntentStatus) -> Intent {
    Intent {
        id: IntentId::from_str("intent-1"),
        version: 1,
        kind,
        goal: EntityId::from_str("goal"),
        status,
        priority: Priority::from_str("normal"),
        created_at: 0,
        updated_at: 0,
        parent_intent: None,
        contract_id: ContractRef {
            id: ContractId::from_str("contract-1"),
            version: 1,
        },
        active_plan_id: None,
        graph_id: None,
        product_id: None,
        completion_predicate: Opaque::from_str("done"),
    }
}

/// 构造一枚 Product Intent（`product_id` 有效）。
pub fn product_intent(status: IntentStatus) -> Intent {
    let mut value = intent(IntentKind::Product, status);
    value.product_id = Some(ProductIdRef::from_str("product-1"));
    value
}
