//! §7.4 的两个输入面：`InitialVerifierInput` 与 `PostBlindVerifierInput`（设计 §7.4）。

use continuum_artifact::Artifact;
use continuum_graph::NodeId;

use crate::contract_view::ContractView;
use crate::error::VerifyError;
use crate::evidence::{Evidence, EvidenceProducer, WorkerCompletionDeclaration};
use crate::verdict::BlindVerdict;

/// §263 的初判输入面。**是一个独立的类型，不是 `Evidence` 的切片**——
/// 只有这样，「不共享 Worker 的完成声明与完整推理」才是结构上的。
///
/// **`TestValidity` 也不在输入面里**：它是 Test Validity Verifier 的**产出**，
/// 把它再喂进初判输入等于让上一轮的判定喂下一轮（§120 的「共享证据路径」的一种）。
///
/// # 「只有一个构造点」的照实说明
///
/// 字段照 §7.4 的形状开 `pub`（Task 10 的 `compile_fail` 样例 1 用结构体字面量写
/// 「多出第四项」），故字面量构造这条路径存在，且它绕过
/// [`InitialVerifierInput::assemble`] 的自产证据检查。设计 §7.4 把「只有一个构造点」
/// 写进了正文，而字段开 `pub` 使该句不成立——**这一处张力照实记录**
/// （与 Task 3 对 `TestValidity` 的处置同一裁定：设计字面维持，张力转终审）。
/// **§263 的结构性保证不依赖这一句**：字面量也装不下 `WorkerCompletionDeclaration`
/// （字段清单里没有那个位置）。
#[derive(Debug, Clone, PartialEq)]
pub struct InitialVerifierInput {
    pub contract: ContractView,
    pub artifacts: Vec<Artifact>,
    pub evidence: Vec<Evidence>,
}

impl InitialVerifierInput {
    /// 装配处做两件事：拒绝 `producer` 指向受验节点自己的证据（§263）；
    /// 输入面**只有** §263 点名的三类，不加第四类。**不接受
    /// `WorkerCompletionDeclaration`**——§263 与 §29 在本块的结构性兑现
    /// （照片在 Task 10 的 `compile_fail` 样例 1）。
    ///
    /// **第 1 条比 §263 的文本更严**（设计 §7.4 已记代价，方向是 fail-closed）：
    /// §263 逐字只点名「完整 Worker reasoning」与「Worker 自己的完成声明」，没说受验节点
    /// 自产的 `Evidence` 要排除。允许它进来等于让受验者自证；代价是节点的证据若只能自产，
    /// 该轮装不出初判输入（`Err` 冒泡、判定不得产出）——本块接受这个代价。
    ///
    /// **`occurred_at` 本轮不落任何字段**：§7.4 的输入面里没有时间戳的位置；
    /// 参数照签名保留，值本轮不参与任何判定。
    pub fn assemble(
        contract: ContractView,
        artifacts: Vec<Artifact>,
        evidence: Vec<Evidence>,
        verified_node: &NodeId,
        occurred_at: i64,
    ) -> Result<InitialVerifierInput, VerifyError> {
        // 本输入面没有承载时间戳的字段；照 §7.4 的签名保留参数。
        let _ = occurred_at;

        for item in &evidence {
            if let EvidenceProducer::Node { node, .. } = item.producer() {
                if node == verified_node {
                    return Err(VerifyError::EvidenceFromVerifiedNode {
                        evidence_id: item.id().clone(),
                        node: node.clone(),
                    });
                }
            }
        }

        Ok(InitialVerifierInput {
            contract,
            artifacts,
            evidence,
        })
    }

    /// §7.5：本输入面里有没有可读的制品（[`clamp_by_readability`] 的入参来源）。
    ///
    /// 判据取「`artifacts` 非空」：更细的「可读」判定（制品能否打开、`metadata` 的
    /// schema 是否认得）不在本块——`Artifact.metadata` 是 `serde_json::Value`，其 schema
    /// 规范未给（设计 §15 第 9 条），故本块只判有没有。
    ///
    /// [`clamp_by_readability`]: crate::verdict::clamp_by_readability
    pub fn has_readable_artifact(&self) -> bool {
        !self.artifacts.is_empty()
    }
}

/// §29 的「blind verification 之后才比较 Worker reasoning summary」。
/// **构造点的第一个参数是 `BlindVerdict`**：没有盲判结论，这一类输入写不出来。
#[derive(Debug, Clone)]
pub struct PostBlindVerifierInput {
    pub blind: BlindVerdict,
    pub worker_declaration: WorkerCompletionDeclaration,
}

impl PostBlindVerifierInput {
    /// 构造点：第一个参数是 [`BlindVerdict`]。没有盲判结论，这一类输入写不出来——
    /// 设计 §7.4 的照片 2（Task 10）以 `PostBlindVerifierInput::new(declaration)` 的
    /// 「缺第一个参数」形态写出它。字段开 `pub`，结构体字面量构造同样可行；
    /// 本入口是直通包装。
    ///
    /// **不派生 `PartialEq`**：`WorkerCompletionDeclaration` 上没有它（Task 2 的派生清单），
    /// 本型不因本 task 的比较需要而承压。
    pub fn new(blind: BlindVerdict, worker_declaration: WorkerCompletionDeclaration) -> Self {
        PostBlindVerifierInput {
            blind,
            worker_declaration,
        }
    }
}
