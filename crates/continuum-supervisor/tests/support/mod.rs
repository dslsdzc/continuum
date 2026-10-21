#![allow(dead_code)]

//! 共享夹具与源码语料工具。只放测试用的构造与文本处理，不放生产逻辑。

use continuum_canonical::{EntityId, Opaque};
use continuum_semantics::{
    ContractId, ContractRef, Intent, IntentId, IntentKind, IntentStatus, Priority, ProductIdRef,
};
use continuum_supervisor::AuthorizedTask;

/// 逐行丢弃整行注释后拼接；用于源码面的零命中断言。
pub fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 从 `fn <name>` 起，截取配对的 `{}` 之间的函数体；先丢弃整行注释，避免注释里的花括号干扰。
pub fn fn_body(source: &str, name: &str) -> String {
    let code = code_only(source);
    let marker = format!("fn {name}");
    let start = code
        .find(&marker)
        .unwrap_or_else(|| panic!("源码里找不到 {marker}"));
    let rest = &code[start..];
    let mut depth = 0usize;
    let mut started = false;
    let mut body = String::new();
    for ch in rest.chars() {
        if ch == '{' {
            depth += 1;
            if depth == 1 {
                started = true;
                continue;
            }
        }
        if ch == '}' {
            if depth == 0 {
                continue;
            }
            depth -= 1;
            if depth == 0 {
                break;
            }
        }
        if started {
            body.push(ch);
        }
    }
    assert!(depth == 0, "{marker} 的函数体没有配对的右花括号");
    body
}

/// 构造一枚最小的 Intent；所有字段由夹具写死，测试按需覆盖。
pub fn intent(kind: IntentKind, status: IntentStatus) -> Intent {
    named_intent("intent-1", kind, status)
}

/// 构造一枚指定 id 的最小 Intent。
pub fn named_intent(id: &str, kind: IntentKind, status: IntentStatus) -> Intent {
    Intent {
        id: IntentId::from_str(id),
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

/// 构造一枚已授权的非 Product 任务。
pub fn task(intent: Intent) -> AuthorizedTask {
    AuthorizedTask::authorized_intent(intent).expect("夹具 Intent 必须是合法的授权任务")
}

/// 构造一枚已授权的 Product 任务。
pub fn product_task(status: IntentStatus) -> AuthorizedTask {
    AuthorizedTask::product_drive(product_intent(status)).expect("夹具 Product Intent 必须已 READY")
}
