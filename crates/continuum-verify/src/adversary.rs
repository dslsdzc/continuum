//! §9 的 Verifier Adversary：类型与消费规则（设计 §9.1）。
//!
//! §9.2 的 mutation 插槽在本文件**没有类型落点**（mutation 的产物是一条
//! `EvidenceType::Test` 证据，不新增臂）——三条源码面核对的说明在
//! `tests/adversary.rs` 末的注释块里（Task 11 钉，无运行期照片）。

use continuum_operator::BackendId;

use crate::evidence::Claim;
use crate::verdict::VerdictAggregate;

/// §264 / §215 的产物。**「高价值任务」本轮不定义**（`B1`：触发条件与成本归属规范未给），
/// 故本类型本轮无触发——**它只冻结类型与消费规则**。
///
/// 派生 `Debug` / `Clone` / `PartialEq`：它们不产生构造路径（同
/// [`VerifierVerdict`](crate::verdict::VerifierVerdict) 的派生清单）。
#[derive(Debug, Clone, PartialEq)]
pub enum AdversaryOutcome {
    /// 本轮 adversary 未找到反例（设计 §9.1 第 2 条的第一臂）。
    NoCounterexample,
    /// §264：找到反例 ⇒ `verification_strategy = insufficient`。
    CounterexampleFound {
        /// 被证伪的那个 verifier（它「验过」但反例在场）。
        passed_verifier: BackendId,
        /// 反例本身（本 crate 不解析 `Claim` 的内容）。
        counterexample: Claim,
    },
}

impl AdversaryOutcome {
    /// 两臂，逐臂照录（设计 §9.1 第 2 条）。
    ///
    /// **本清单是函数不是 `const`**：`CounterexampleFound` 的载荷含 `BackendId`
    /// （`continuum-operator`：字段私有、`new` 不是 `const fn`），本 crate 在常量上下文里
    /// **构造不出** `AdversaryOutcome` 的值；本仓带 `ALL` 常量的各处成立的前提是那些类型
    /// 可在常量上下文里完整构造（照 P5e `2543aeb` 的订正与 P5c 的 `all_operators()` 形状）。
    /// 数目 `2` 写在**返回类型**里，使清单个数的改动仍**编译期可见**——删一臂必须同步改
    /// 返回类型，而同步那一处后 `tests/adversary.rs` 的数目断言会在**运行期**红（点名走
    /// 切片比较，不靠越界索引；纪律 1(a) 的锚点）。
    ///
    /// 载荷取样例值（空串），不承诺代表性：清单回答「有哪些臂」，载荷不参与这件事
    /// （同 [`VerifierVerdict::ALL`](crate::verdict::VerifierVerdict::ALL) 的处置）。
    pub fn all() -> [AdversaryOutcome; 2] {
        [
            AdversaryOutcome::NoCounterexample,
            AdversaryOutcome::CounterexampleFound {
                passed_verifier: BackendId::new(""),
                counterexample: Claim::from_str(""),
            },
        ]
    }

    /// 两臂判别串的编码：小写、多词以 `_` 连接。它落
    /// `verification_round.strategy_verdict` 列（Task 9）。**本函数是该编码唯一的产生点**，
    /// match 穷尽且无通配臂：加臂时本函数编译不过，编码不会漏分支。
    ///
    /// **串名没有规范逐字出处**（§9.1 只给臂名与消费规则）：两枚串名是本块的命名，
    /// 照 §4.2 的口径（同
    /// [`VerdictAggregate::as_str`](crate::verdict::VerdictAggregate::as_str)）。
    /// **编码只覆盖判别串**：`CounterexampleFound` 的载荷（`passed_verifier` /
    /// `counterexample`）不进这一列——取舍见计划 `## 遗留` 第二节。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NoCounterexample => "no_counterexample",
            Self::CounterexampleFound { .. } => "counterexample_found",
        }
    }

    /// [`AdversaryOutcome::as_str`] 的**严格逆**（在判别串的意义上）：对
    /// [`AdversaryOutcome::all`] 里的每臂都有 `parse(t.as_str()) == Some(t)`，
    /// 表外字符串一律 `None`。
    ///
    /// **解码只重建判别串**：`CounterexampleFound` 的载荷从串里读不回来，`parse` 重建的
    /// 载荷取空串样本（取舍同上）。`None` 而不是默认臂：从值里读出表外串说明写着本版本
    /// 不认得的取值，取默认会把一份别的东西当成本臂处理（理由同
    /// [`VerdictAggregate::parse`](crate::verdict::VerdictAggregate::parse)）。
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "no_counterexample" => Self::NoCounterexample,
            "counterexample_found" => Self::CounterexampleFound {
                passed_verifier: BackendId::new(""),
                counterexample: Claim::from_str(""),
            },
            _ => return None,
        })
    }
}

/// §9.1 第 3 条的消费规则：`CounterexampleFound` 时该轮的聚合**不得**取
/// `Passed` / `PassedWithUnknown`。**只拒绝那一对组合，不改写聚合值**。
///
/// 由 Task 9 的 `record_verification_round` 在写行前施加（§9.1 第 4 条：本轮它
/// **不是判定反转**，唯一的可见形态是 finding 记录 + `strategy_verdict` 的标注）。
/// 四条组合的边界（两条拒绝、两条放行）由 `tests/adversary.rs` 逐条钉住。
pub fn adversary_permits_passing(aggregate: &VerdictAggregate, outcome: &AdversaryOutcome) -> bool {
    match outcome {
        AdversaryOutcome::NoCounterexample => true,
        AdversaryOutcome::CounterexampleFound { .. } => !matches!(
            aggregate,
            VerdictAggregate::Passed | VerdictAggregate::PassedWithUnknown
        ),
    }
}
