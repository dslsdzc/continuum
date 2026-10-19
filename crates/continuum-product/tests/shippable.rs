//! §335 `ShippableVerdict` 的 fail-closed 判定（集成面）。
//!
//! 需要「已评估 readiness」的用例在 `src/shippable.rs` 的 crate 内测试里：`Product` 的
//! readiness 只能经 `record_milestone_progress` 更新（Task 12，C2 后），集成测试拿不到
//! 已评估的 Product 值。

mod support;

use continuum_canonical::Opaque;
use continuum_product::{
    ClauseAssessment, ClauseVerdict, NotShippableReason, Product, ShippableAssessment,
    ShippableClause, ShippableClauseId, ShippablePredicate, ShippableUndetermined,
    ShippableVerdict, evaluate_shippable,
};
use continuum_verify::CompletionVerdict;
use support::{FixedBaselineGenerator, anchor_intent, usable_baseline};

fn request() -> continuum_product::BaselineRequest {
    continuum_product::BaselineRequest {
        idea: Opaque::from_str("an idea"),
        constraints: vec![],
    }
}

async fn product(predicate: ShippablePredicate) -> Product {
    continuum_product::establish_baseline(
        &anchor_intent("p1", continuum_semantics::IntentStatus::Ready),
        &request(),
        predicate,
        &FixedBaselineGenerator {
            result: Ok(usable_baseline()),
        },
    )
    .await
    .unwrap()
}

async fn default_product() -> (Product, ShippableClauseId) {
    let id = ShippableClauseId::from_str("c1");
    let predicate = ShippablePredicate::new(
        vec![continuum_product::ReadinessDimensionName::Functional],
        vec![
            ShippableClause::new(id.clone(), Opaque::from_str("works")).unwrap(),
        ],
    )
    .unwrap();
    (product(predicate).await, id)
}

fn assessment(id: &ShippableClauseId, verdict: ClauseVerdict) -> ShippableAssessment {
    ShippableAssessment {
        clauses: vec![ClauseAssessment {
            clause_id: id.clone(),
            verdict,
        }],
    }
}

fn undetermined_completion() -> CompletionVerdict {
    CompletionVerdict::Undetermined {
        undecidable: vec![],
    }
}

fn blocked_completion() -> CompletionVerdict {
    CompletionVerdict::Blocked { failing: vec![] }
}

#[tokio::test]
async fn no_product_is_undetermined_even_when_completion_is_blocked() {
    let (_p, id) = default_product().await;
    let verdict = evaluate_shippable(
        None,
        &assessment(&id, ClauseVerdict::Satisfied),
        &blocked_completion(),
    );
    assert!(matches!(
        verdict,
        ShippableVerdict::Undetermined {
            reason: ShippableUndetermined::MissingProduct
        }
    ));
}

#[tokio::test]
async fn completion_blocked_wins_over_a_mismatched_assessment() {
    let (p, _id) = default_product().await;
    let verdict = evaluate_shippable(
        Some(&p),
        &ShippableAssessment { clauses: vec![] },
        &blocked_completion(),
    );
    assert!(matches!(
        verdict,
        ShippableVerdict::NotShippable {
            reason: NotShippableReason::CompletionBlocked
        }
    ));
}

#[tokio::test]
async fn a_not_satisfied_clause_wins_over_an_extra_assessment() {
    let (p, id) = default_product().await;
    let assessment = ShippableAssessment {
        clauses: vec![
            ClauseAssessment {
                clause_id: id.clone(),
                verdict: ClauseVerdict::NotSatisfied,
            },
            ClauseAssessment {
                clause_id: ShippableClauseId::from_str("extra"),
                verdict: ClauseVerdict::Satisfied,
            },
        ],
    };
    let verdict = evaluate_shippable(Some(&p), &assessment, &undetermined_completion());
    assert!(matches!(
        verdict,
        ShippableVerdict::NotShippable {
            reason: NotShippableReason::ClauseNotSatisfied(got)
        } if got == id
    ));
}

#[tokio::test]
async fn a_missing_clause_assessment_is_undetermined() {
    let (p, _id) = default_product().await;
    let verdict = evaluate_shippable(
        Some(&p),
        &ShippableAssessment { clauses: vec![] },
        &undetermined_completion(),
    );
    assert!(matches!(
        verdict,
        ShippableVerdict::Undetermined {
            reason: ShippableUndetermined::AssessmentMismatch
        }
    ));
}

#[tokio::test]
async fn a_duplicate_clause_assessment_is_undetermined() {
    let (p, id) = default_product().await;
    let assessment = ShippableAssessment {
        clauses: vec![
            ClauseAssessment {
                clause_id: id.clone(),
                verdict: ClauseVerdict::Satisfied,
            },
            ClauseAssessment {
                clause_id: id,
                verdict: ClauseVerdict::Satisfied,
            },
        ],
    };
    let verdict = evaluate_shippable(Some(&p), &assessment, &undetermined_completion());
    assert!(matches!(
        verdict,
        ShippableVerdict::Undetermined {
            reason: ShippableUndetermined::AssessmentMismatch
        }
    ));
}

#[tokio::test]
async fn an_extra_clause_assessment_is_undetermined() {
    let (p, id) = default_product().await;
    let assessment = ShippableAssessment {
        clauses: vec![
            ClauseAssessment {
                clause_id: id,
                verdict: ClauseVerdict::Satisfied,
            },
            ClauseAssessment {
                clause_id: ShippableClauseId::from_str("extra"),
                verdict: ClauseVerdict::Satisfied,
            },
        ],
    };
    let verdict = evaluate_shippable(Some(&p), &assessment, &undetermined_completion());
    assert!(matches!(
        verdict,
        ShippableVerdict::Undetermined {
            reason: ShippableUndetermined::AssessmentMismatch
        }
    ));
}

#[tokio::test]
async fn a_not_satisfied_clause_is_not_shippable() {
    let (p, id) = default_product().await;
    let verdict = evaluate_shippable(
        Some(&p),
        &assessment(&id, ClauseVerdict::NotSatisfied),
        &undetermined_completion(),
    );
    assert!(matches!(
        verdict,
        ShippableVerdict::NotShippable {
            reason: NotShippableReason::ClauseNotSatisfied(got)
        } if got == id
    ));
}

#[tokio::test]
async fn an_unassessed_required_dimension_is_undetermined() {
    let (p, id) = default_product().await;
    let verdict = evaluate_shippable(
        Some(&p),
        &assessment(&id, ClauseVerdict::Satisfied),
        &undetermined_completion(),
    );
    assert!(matches!(
        verdict,
        ShippableVerdict::Undetermined {
            reason: ShippableUndetermined::RequiredDimensionUnassessed(
                continuum_product::ReadinessDimensionName::Functional
            )
        }
    ));
}

#[tokio::test]
async fn the_verdict_does_not_use_clause_statements() {
    let id = ShippableClauseId::from_str("c1");
    let predicate_with = |statement: &str| {
        ShippablePredicate::new(
            vec![continuum_product::ReadinessDimensionName::Functional],
            vec![
                ShippableClause::new(id.clone(), Opaque::from_str(statement)).unwrap(),
            ],
        )
        .unwrap()
    };
    let first = product(predicate_with("works")).await;
    let second = product(predicate_with("an entirely different statement")).await;
    let assessment = assessment(&id, ClauseVerdict::Satisfied);
    let verdict = evaluate_shippable(
        Some(&first),
        &assessment,
        &undetermined_completion(),
    );
    let other = evaluate_shippable(
        Some(&second),
        &assessment,
        &undetermined_completion(),
    );
    assert_eq!(verdict, other);
}

#[tokio::test]
async fn neither_completion_arm_ever_produces_shippable() {
    let (p, id) = default_product().await;
    for completion in CompletionVerdict::ALL {
        let verdict = evaluate_shippable(
            Some(&p),
            &assessment(&id, ClauseVerdict::Satisfied),
            &completion,
        );
        assert!(!matches!(verdict, ShippableVerdict::Shippable));
    }
}
