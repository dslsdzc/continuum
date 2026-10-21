//! J14 的类型面：调用方不能把伪造的 `ProgressGateVerdict` 传进 `emit_replan_decision`。

use continuum_persist::Tx;
use continuum_product::{
    Product, ProgressGateVerdict, ProgressRecord, ReplanDraft, emit_replan_decision,
};

fn call(
    tx: &Tx<'_>,
    product: &Product,
    record: &ProgressRecord,
    history: &[ProgressRecord],
    draft: ReplanDraft,
    verdict: ProgressGateVerdict,
) {
    let _ = emit_replan_decision(tx, product, record, history, None, draft, verdict, 0);
}

fn main() {}
