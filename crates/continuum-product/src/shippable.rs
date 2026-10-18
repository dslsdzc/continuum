//! §4.6 的发布判定：三值 fail-closed。
//!
//! `Shippable` 今天**不可达**：`CompletionVerdict` 只有 `Blocked`／`Undetermined` 两臂，
//! 入口的穷尽 match 没有正臂。P5a 若加正臂，这里编译不过——刻意留的 tripwire。

use continuum_verify::CompletionVerdict;

use crate::predicate::{ShippableClauseId, ShippablePredicate};
use crate::product::Product;
use crate::readiness::{ProductReadiness, ReadinessDimensionName};

/// 调用方对每条子句的判定；**P7A 不评估子句内容**。
#[derive(Debug, Clone, PartialEq)]
pub struct ShippableAssessment {
    pub clauses: Vec<ClauseAssessment>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClauseAssessment {
    pub clause_id: ShippableClauseId,
    pub verdict: ClauseVerdict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClauseVerdict {
    Satisfied,
    NotSatisfied,
    Undetermined,
}

/// 三值 fail-closed。`Undetermined` 不是许可。
#[derive(Debug, Clone, PartialEq)]
pub enum ShippableVerdict {
    Shippable,
    NotShippable { reason: NotShippableReason },
    Undetermined { reason: ShippableUndetermined },
}

#[derive(Debug, Clone, PartialEq)]
pub enum NotShippableReason {
    ClauseNotSatisfied(ShippableClauseId),
    CompletionBlocked,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ShippableUndetermined {
    MissingProduct,
    /// Product 在但 baseline 不可用（product_goal／minimum_complete_system 空）。
    /// 正常读路径由 `load_product` 先行判 `CorruptRow`；这是 defense-in-depth。
    BaselineUnusable,
    AssessmentMismatch,
    RequiredDimensionUnassessed(ReadinessDimensionName),
    ClauseUndetermined(ShippableClauseId),
    CompletionUndetermined,
}

/// 判定顺序逐字照设计 §4.6：
/// 1. 无 Product ⇒ `Undetermined(MissingProduct)`（前置）；
/// 2. completion `Blocked` ⇒ `NotShippable(CompletionBlocked)`；
/// 3. 任一子句 `NotSatisfied` ⇒ `NotShippable(ClauseNotSatisfied)`（按谓词顺序取第一枚）；
/// 4. baseline 不可用 ⇒ `Undetermined(BaselineUnusable)`；
/// 5. assessment 与子句不是一一对应 ⇒ `Undetermined(AssessmentMismatch)`；
/// 6. 任一 required dimension 未评估 ⇒ `Undetermined(RequiredDimensionUnassessed)`；
/// 7. 任一子句 `Undetermined` ⇒ `Undetermined(ClauseUndetermined)`；
/// 8. completion `Undetermined` ⇒ `Undetermined(CompletionUndetermined)`。
///
/// **没有任何一行返回 `Shippable`。**
pub fn evaluate_shippable(
    product: Option<&Product>,
    assessment: &ShippableAssessment,
    completion: &CompletionVerdict,
) -> ShippableVerdict {
    let product = match product {
        Some(product) => product,
        None => {
            return ShippableVerdict::Undetermined {
                reason: ShippableUndetermined::MissingProduct,
            }
        }
    };

    // 穷尽 match：`Blocked` 立即返回；`Undetermined` 只记下，留到第 8 步。
    // P5a 加正臂时这里编译不过。
    let completion_undetermined = match completion {
        CompletionVerdict::Blocked { .. } => {
            return ShippableVerdict::NotShippable {
                reason: NotShippableReason::CompletionBlocked,
            }
        }
        CompletionVerdict::Undetermined { .. } => true,
    };

    let predicate = product.shippable_predicate();

    // 3. 明确不满足先于结构错误。同 id 多条时任一 NotSatisfied 即按 NotSatisfied。
    for clause in predicate.clauses() {
        if assessment
            .clauses
            .iter()
            .any(|a| a.clause_id == *clause.id() && a.verdict == ClauseVerdict::NotSatisfied)
        {
            return ShippableVerdict::NotShippable {
                reason: NotShippableReason::ClauseNotSatisfied(clause.id().clone()),
            };
        }
    }

    // 4. defense-in-depth：正常读路径到不了这里。
    if product.baseline().is_usable().is_err() {
        return ShippableVerdict::Undetermined {
            reason: ShippableUndetermined::BaselineUnusable,
        };
    }

    // 5. 一一对应：每条子句恰有一条 assessment，且没有多出的 id。
    if assessment.clauses.len() != predicate.clauses().len()
        || predicate.clauses().iter().any(|clause| {
            assessment
                .clauses
                .iter()
                .filter(|a| a.clause_id == *clause.id())
                .count()
                != 1
        })
        || assessment
            .clauses
            .iter()
            .any(|a| !predicate.clauses().iter().any(|c| c.id() == &a.clause_id))
    {
        return ShippableVerdict::Undetermined {
            reason: ShippableUndetermined::AssessmentMismatch,
        };
    }

    // 6. 按 required_dimensions 的顺序取第一枚未评估。
    for name in predicate.required_dimensions() {
        if product.readiness().dimension(*name).value().is_unassessed() {
            return ShippableVerdict::Undetermined {
                reason: ShippableUndetermined::RequiredDimensionUnassessed(*name),
            };
        }
    }

    // 7. 按 clauses 顺序取第一枚 Undetermined。
    for clause in predicate.clauses() {
        if assessment
            .clauses
            .iter()
            .any(|a| a.clause_id == *clause.id() && a.verdict == ClauseVerdict::Undetermined)
        {
            return ShippableVerdict::Undetermined {
                reason: ShippableUndetermined::ClauseUndetermined(clause.id().clone()),
            };
        }
    }

    // 8. 没有第 9 步：今天没有正臂。
    if completion_undetermined {
        return ShippableVerdict::Undetermined {
            reason: ShippableUndetermined::CompletionUndetermined,
        };
    }

    // 不可达：`completion` 只有两臂，`Blocked` 已在上面返回、`Undetermined`
    // 已在上面返回。保留这一行是为了让「没有正臂」在类型层面成立，而不是
    // 靠 unreachable! 掩盖。
    match completion {
        CompletionVerdict::Blocked { .. } | CompletionVerdict::Undetermined { .. } => {
            ShippableVerdict::Undetermined {
                reason: ShippableUndetermined::CompletionUndetermined,
            }
        }
    }
}

/// `ShippablePredicate` 只读访问器在本模块无额外要求；这里仅确认类型可达。
#[allow(dead_code)]
fn _predicate_type_is_used(_p: &ShippablePredicate, _r: &ProductReadiness) {}

#[cfg(test)]
mod tests {
    use super::*;
    use continuum_canonical::Opaque;
    use continuum_semantics::{IntentId, ProductIdRef};

    use crate::baseline::EngineeringBaseline;
    use crate::predicate::ShippableClause;
    use crate::readiness::ReadinessDimension;

    fn usable_baseline() -> EngineeringBaseline {
        EngineeringBaseline {
            product_goal: Opaque::from_str("ship a product"),
            target_users: vec![Opaque::from_str("developers")],
            required_capabilities: vec![Opaque::from_str("build")],
            hard_constraints: vec![],
            architecture: Opaque::from_str("library"),
            technology_choices: vec![Opaque::from_str("rust")],
            minimum_complete_system: vec![Opaque::from_str("cli")],
            known_unknowns: vec![],
            engineering_risks: vec![Opaque::from_str("scope")],
            acceptance_criteria: vec![Opaque::from_str("tests pass")],
        }
    }

    fn predicate() -> ShippablePredicate {
        ShippablePredicate::new(
            vec![ReadinessDimensionName::Functional],
            vec![
                ShippableClause::new(
                    ShippableClauseId::from_str("c1"),
                    Opaque::from_str("works"),
                )
                .unwrap(),
            ],
        )
        .unwrap()
    }

    fn assessed(functional: &str) -> ProductReadiness {
        ProductReadiness::unassessed().with_dimension(
            ReadinessDimensionName::Functional,
            ReadinessDimension::assessed(
                Opaque::from_str(functional),
                vec![continuum_verify::EvidenceId::from_str("e1")],
                100,
            )
            .unwrap(),
        )
    }

    fn product_with(readiness: ProductReadiness) -> Product {
        Product::from_verified_parts(
            ProductIdRef::from_str("p1"),
            IntentId::from_str("intent/root"),
            usable_baseline(),
            readiness,
            None,
            Vec::new(),
            predicate(),
        )
        .unwrap()
    }

    fn assessment(verdict: ClauseVerdict) -> ShippableAssessment {
        ShippableAssessment {
            clauses: vec![ClauseAssessment {
                clause_id: ShippableClauseId::from_str("c1"),
                verdict,
            }],
        }
    }

    fn undetermined() -> CompletionVerdict {
        CompletionVerdict::Undetermined {
            undecidable: vec![],
        }
    }

    #[test]
    fn an_unusable_baseline_is_undetermined() {
        let product = Product::for_test_with_unusable_baseline(predicate());
        let verdict = evaluate_shippable(
            Some(&product),
            &assessment(ClauseVerdict::Satisfied),
            &undetermined(),
        );
        assert!(matches!(
            verdict,
            ShippableVerdict::Undetermined {
                reason: ShippableUndetermined::BaselineUnusable
            }
        ));
    }

    #[test]
    fn a_clause_undetermined_is_undetermined() {
        let product = product_with(assessed("1"));
        let verdict = evaluate_shippable(
            Some(&product),
            &assessment(ClauseVerdict::Undetermined),
            &undetermined(),
        );
        assert!(matches!(
            verdict,
            ShippableVerdict::Undetermined {
                reason: ShippableUndetermined::ClauseUndetermined(id)
            } if id.as_str() == "c1"
        ));
    }

    #[test]
    fn completion_undetermined_is_undetermined() {
        let product = product_with(assessed("1"));
        let verdict = evaluate_shippable(
            Some(&product),
            &assessment(ClauseVerdict::Satisfied),
            &undetermined(),
        );
        assert!(matches!(
            verdict,
            ShippableVerdict::Undetermined {
                reason: ShippableUndetermined::CompletionUndetermined
            }
        ));
    }

    #[test]
    fn the_verdict_does_not_use_clause_statements() {
        let product_with_statement = |statement: &str| {
            let predicate = ShippablePredicate::new(
                vec![ReadinessDimensionName::Functional],
                vec![
                    ShippableClause::new(
                        ShippableClauseId::from_str("c1"),
                        Opaque::from_str(statement),
                    )
                    .unwrap(),
                ],
            )
            .unwrap();
            Product::from_verified_parts(
                ProductIdRef::from_str("p1"),
                IntentId::from_str("intent/root"),
                usable_baseline(),
                assessed("1"),
                None,
                Vec::new(),
                predicate,
            )
            .unwrap()
        };
        let first = product_with_statement("works");
        let second = product_with_statement("different text, same id");
        let verdict = evaluate_shippable(
            Some(&first),
            &assessment(ClauseVerdict::Satisfied),
            &undetermined(),
        );
        let other = evaluate_shippable(
            Some(&second),
            &assessment(ClauseVerdict::Satisfied),
            &undetermined(),
        );
        assert_eq!(verdict, other);
    }
}
