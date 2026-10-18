use std::collections::HashSet;

use continuum_memory::{MemoryError, MemoryRetrieval, RetrievalQuery};

use crate::{ContextBudget, ContextPackage, ContextRequest, Reference};

/// §279 的「节点执行前必经」通道。**本 crate 唯一的 `ContextPackage` 构造点**（设计 §4.3）。
///
/// 调用方是 P1 的节点执行器（未建）：节点进入 `Running` 之前调用一次。本函数不读节点
/// 状态、不读时钟／环境／文件系统，不写事件、不调度、不缓存。
///
/// 失败面只有 P6C 的 `MemoryError`，原样透传；本块不定义第二枚错误类型（设计 §5.6）。
pub fn compile(
    request: ContextRequest,
    budget: ContextBudget,
    memory: &dyn MemoryRetrieval,
) -> Result<ContextPackage, MemoryError> {
    let ContextRequest {
        instructions,
        references,
        artifacts,
        scopes,
        anchor,
        memory_query,
    } = request;

    let mut instructions_out: Vec<String> = Vec::new();
    let mut references_out: Vec<Reference> = Vec::new();
    let mut artifacts_out = Vec::new();
    let mut provenance: Vec<continuum_trust::ContextProvenance> = Vec::new();
    let mut trust_labels: Vec<continuum_trust::ContextTrust> = Vec::new();

    // instructions：丢空串，其余按输入序全收；不受预算约束（设计 §5.3 第 1／2 条）。
    for candidate in instructions {
        if candidate.content.is_empty() {
            continue;
        }
        let label_value = continuum_trust::label(&candidate.provenance);
        instructions_out.push(candidate.content);
        provenance.push(candidate.provenance);
        trust_labels.push(label_value);
    }

    // references：只丢空 `Text`，纯空白保留；本 task 暂不截断（设计 §5.3 第 1／3 条）。
    for candidate in references {
        if let Reference::Text(text) = &candidate.content
            && text.is_empty()
        {
            continue;
        }
        let label_value = continuum_trust::label(&candidate.provenance);
        references_out.push(candidate.content);
        provenance.push(candidate.provenance);
        trust_labels.push(label_value);
    }

    // artifacts：按输入序装 id；本 task 暂不去重、暂不截断（设计 §5.3 第 4 条）。
    for candidate in artifacts {
        let label_value = continuum_trust::label(&candidate.provenance);
        artifacts_out.push(candidate.content);
        provenance.push(candidate.provenance);
        trust_labels.push(label_value);
    }

    // memories：只在 scopes 非空且 max_memories > 0 时构造查询并调用检索面。
    // `?` 直接透传 P6C 的 `MemoryError`（设计 §5.3 第 5 条、§5.6）。
    let mut memories = Vec::new();
    if !scopes.is_empty() && budget.max_memories > 0 {
        let query = RetrievalQuery::new(
            scopes,
            anchor,
            memory_query,
            budget.max_memories,
            false,
        )?;
        let claims = memory.retrieve(&query)?;
        let mut seen: HashSet<String> = HashSet::new();
        for claim in claims {
            if memories.len() >= budget.max_memories {
                break;
            }
            if !seen.insert(claim.id().as_str().to_string()) {
                continue;
            }
            // provenance 必须在移动 claim 之前构造（借 `id()`）。
            let claim_provenance =
                continuum_trust::ContextProvenance::memory(claim.id().as_str());
            let label_value = continuum_trust::label(&claim_provenance);
            memories.push(claim);
            provenance.push(claim_provenance);
            trust_labels.push(label_value);
        }
    }

    Ok(ContextPackage {
        instructions: instructions_out,
        references: references_out,
        artifacts: artifacts_out,
        memories,
        provenance,
        trust_labels,
    })
}
