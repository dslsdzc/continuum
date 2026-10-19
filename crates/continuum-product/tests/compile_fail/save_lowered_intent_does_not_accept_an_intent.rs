//! J10 的类型面：调用方不能把拼好的 `Intent`（尤其 READY）传给写路径。

use continuum_persist::Tx;
use continuum_product::{Milestone, Product, save_lowered_intent};
use continuum_semantics::{ContractRef, Intent, Priority};

fn call(
    tx: &Tx<'_>,
    product: &Product,
    milestone: &Milestone,
    contract: ContractRef,
    priority: Priority,
    intent: Intent,
) {
    let _ = save_lowered_intent(tx, product, milestone, contract, priority, 0, 0, intent);
}

fn main() {}
