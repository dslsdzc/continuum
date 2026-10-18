//! §4.5 的 Progress Gate：三值判定、调用方申报的 delta 与阈值。

mod support;

use continuum_canonical::Opaque;
use continuum_product::{
    MilestoneId, ProgressDelta, ProgressGateVerdict, ProgressRecord, ProgressUndetermined,
    ProductReadiness, ReadinessDimension, ReadinessDimensionName, evaluate_progress,
};
use continuum_semantics::ProductIdRef;
use continuum_verify::EvidenceId;
use support::DeltaByDimension;

fn readiness(documentation: Option<&str>) -> ProductReadiness {
    let base = ProductReadiness::unassessed();
    match documentation {
        None => base,
        Some(score) => base.with_dimension(
            ReadinessDimensionName::Documentation,
            ReadinessDimension::assessed(
                Opaque::from_str(score),
                vec![EvidenceId::from_str("e1")],
                100,
            )
            .unwrap(),
        ),
    }
}

fn record(
    previous: &ProductReadiness,
    current: &ProductReadiness,
    delta: ProgressDelta,
    at: i64,
) -> ProgressRecord {
    ProgressRecord {
        product_id: ProductIdRef::from_str("p1"),
        milestone_id: MilestoneId::from_str("m1"),
        previous_product_state: previous.clone(),
        current_product_state: current.clone(),
        progress_delta: delta,
        recorded_at: at,
    }
}

#[test]
fn no_policy_is_undetermined() {
    let readiness = readiness(Some("2"));
    let verdict = evaluate_progress(&readiness, &readiness, &[], None);
    assert!(matches!(
        verdict,
        ProgressGateVerdict::Undetermined {
            reason: ProgressUndetermined::NoPolicy
        }
    ));
}

#[test]
fn a_zero_threshold_is_invalid() {
    let readiness = readiness(Some("2"));
    let policy = DeltaByDimension { threshold: 0 };
    let verdict = evaluate_progress(&readiness, &readiness, &[], Some(&policy));
    assert!(matches!(
        verdict,
        ProgressGateVerdict::Undetermined {
            reason: ProgressUndetermined::InvalidThreshold
        }
    ));
}

#[test]
fn all_unassessed_readiness_is_undetermined() {
    let readiness = ProductReadiness::unassessed();
    let policy = DeltaByDimension { threshold: 3 };
    let verdict = evaluate_progress(&readiness, &readiness, &[], Some(&policy));
    assert!(matches!(
        verdict,
        ProgressGateVerdict::Undetermined {
            reason: ProgressUndetermined::ReadinessUnassessed
        }
    ));
}

#[test]
fn a_real_policy_reads_the_dimension_and_reports_improved_no_progress_regressed() {
    let policy = DeltaByDimension { threshold: 3 };

    let improved = evaluate_progress(
        &readiness(Some("2")),
        &readiness(Some("3")),
        &[],
        Some(&policy),
    );
    assert!(matches!(
        improved,
        ProgressGateVerdict::NotTriggered {
            consecutive_no_progress: 0,
            threshold: 3
        }
    ));

    let no_progress = evaluate_progress(
        &readiness(Some("3")),
        &readiness(Some("3")),
        &[],
        Some(&policy),
    );
    assert!(matches!(
        no_progress,
        ProgressGateVerdict::NotTriggered {
            consecutive_no_progress: 1,
            threshold: 3
        }
    ));

    let regressed = evaluate_progress(
        &readiness(Some("3")),
        &readiness(Some("2")),
        &[],
        Some(&policy),
    );
    assert!(matches!(
        regressed,
        ProgressGateVerdict::NotTriggered {
            consecutive_no_progress: 0,
            threshold: 3
        }
    ));
}

#[test]
fn a_current_improved_or_regressed_resets_the_run() {
    let policy = DeltaByDimension { threshold: 3 };
    let two = readiness(Some("2"));
    let three = readiness(Some("3"));
    let history = vec![
        record(&two, &two, ProgressDelta::NoProgress, 1),
        record(&two, &two, ProgressDelta::NoProgress, 2),
    ];

    let improved = evaluate_progress(&three, &readiness(Some("4")), &history, Some(&policy));
    assert!(matches!(
        improved,
        ProgressGateVerdict::NotTriggered {
            consecutive_no_progress: 0,
            threshold: 3
        }
    ));

    let regressed = evaluate_progress(&three, &two, &history, Some(&policy));
    assert!(matches!(
        regressed,
        ProgressGateVerdict::NotTriggered {
            consecutive_no_progress: 0,
            threshold: 3
        }
    ));
}

#[test]
fn a_no_progress_run_counts_up_to_the_threshold() {
    let policy = DeltaByDimension { threshold: 3 };
    let three = readiness(Some("3"));

    let first = evaluate_progress(&three, &three, &[], Some(&policy));
    assert!(matches!(
        first,
        ProgressGateVerdict::NotTriggered {
            consecutive_no_progress: 1,
            threshold: 3
        }
    ));

    let history = vec![
        record(&three, &three, ProgressDelta::NoProgress, 1),
        record(&three, &three, ProgressDelta::NoProgress, 2),
    ];
    let third = evaluate_progress(&three, &three, &history, Some(&policy));
    assert!(matches!(
        third,
        ProgressGateVerdict::Triggered {
            consecutive_no_progress: 3,
            threshold: 3
        }
    ));
}

#[test]
fn an_undetermined_history_before_the_threshold_is_undetermined() {
    let policy = DeltaByDimension { threshold: 3 };
    let three = readiness(Some("3"));
    let history = vec![record(&three, &three, ProgressDelta::Undetermined, 1)];
    let verdict = evaluate_progress(&three, &three, &history, Some(&policy));
    assert!(matches!(
        verdict,
        ProgressGateVerdict::Undetermined {
            reason: ProgressUndetermined::HistoryUndetermined
        }
    ));
}

#[test]
fn regressed_does_not_trigger_replan() {
    let policy = DeltaByDimension { threshold: 1 };
    let verdict = evaluate_progress(
        &readiness(Some("3")),
        &readiness(Some("2")),
        &[],
        Some(&policy),
    );
    assert!(matches!(
        verdict,
        ProgressGateVerdict::NotTriggered {
            consecutive_no_progress: 0,
            threshold: 1
        }
    ));
}
