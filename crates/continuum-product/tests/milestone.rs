//! §4.4 的 Milestone 七字段、七状态与有界批次惰性生成。

mod support;

use continuum_canonical::Opaque;
use continuum_product::{
    BatchBoundary, Milestone, MilestoneBatchDraft, MilestoneDependency, MilestoneDraft,
    MilestoneId, MilestoneState, Product, ProductError, propose_next_batch,
};
use continuum_semantics::ProductIdRef;
use support::{FixedBaselineGenerator, FixedMilestoneGenerator, anchor_intent, predicate, usable_baseline};

fn request() -> continuum_product::BaselineRequest {
    continuum_product::BaselineRequest {
        idea: Opaque::from_str("an idea"),
        constraints: vec![],
    }
}

async fn product() -> Product {
    continuum_product::establish_baseline(
        &anchor_intent("p1", continuum_semantics::IntentStatus::Ready),
        &request(),
        predicate(),
        &FixedBaselineGenerator {
            result: Ok(usable_baseline()),
        },
    )
    .await
    .unwrap()
}

fn draft(
    objective: &str,
    dependencies: Vec<MilestoneDependency>,
    entry: Vec<&str>,
    exit: Vec<&str>,
) -> MilestoneDraft {
    MilestoneDraft {
        objective: Opaque::from_str(objective),
        dependencies,
        entry_conditions: entry.into_iter().map(Opaque::from_str).collect(),
        exit_conditions: exit.into_iter().map(Opaque::from_str).collect(),
    }
}

fn plain(objective: &str) -> MilestoneDraft {
    draft(objective, vec![], vec![], vec![])
}

fn batch(milestones: Vec<MilestoneDraft>) -> MilestoneBatchDraft {
    MilestoneBatchDraft { milestones }
}

fn boundary(max_milestones: usize, max_depth: u32) -> BatchBoundary {
    BatchBoundary::new(max_milestones, max_depth).unwrap()
}

fn ids(milestones: &[Milestone]) -> Vec<&str> {
    milestones.iter().map(|m| m.id().as_str()).collect()
}

#[test]
fn the_seven_milestone_fields_are_verbatim_in_order() {
    let source = include_str!("../src/milestone.rs");
    let start = source.find("pub struct Milestone {").unwrap();
    let rest = &source[start + "pub struct Milestone {".len()..];
    let end = rest.find("\n}").unwrap();
    let fields: Vec<&str> = rest[..end]
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
        .map(|line| line.trim_end_matches(',').split(':').next().unwrap().trim())
        .collect();
    assert_eq!(
        fields,
        vec![
            "id",
            "product_id",
            "objective",
            "dependencies",
            "entry_conditions",
            "exit_conditions",
            "state",
        ]
    );
}

#[test]
fn the_milestone_has_seven_read_only_accessors() {
    let _: fn(&Milestone) -> &MilestoneId = Milestone::id;
    let _: fn(&Milestone) -> &ProductIdRef = Milestone::product_id;
    let _: fn(&Milestone) -> &Opaque = Milestone::objective;
    let _: fn(&Milestone) -> &[MilestoneId] = Milestone::dependencies;
    let _: fn(&Milestone) -> &[Opaque] = Milestone::entry_conditions;
    let _: fn(&Milestone) -> &[Opaque] = Milestone::exit_conditions;
    let _: fn(&Milestone) -> MilestoneState = Milestone::state;

    let source = include_str!("../src/milestone.rs");
    assert!(!source.contains("&mut self"));
    assert!(!source.contains("fn into_parts"));
    let start = source.find("pub struct Milestone {").unwrap();
    let rest = &source[start + "pub struct Milestone {".len()..];
    let end = rest.find("\n}").unwrap();
    assert!(!rest[..end].contains("pub "));
}

#[test]
fn the_seven_states_are_listed_in_the_normative_order() {
    let source = include_str!("../src/milestone.rs");
    let start = source.find("pub enum MilestoneState {").unwrap();
    let rest = &source[start + "pub enum MilestoneState {".len()..];
    let end = rest.find("\n}").unwrap();
    let states: Vec<&str> = rest[..end]
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
        .map(|line| line.trim_end_matches(','))
        .collect();
    assert_eq!(
        states,
        vec![
            "Proposed",
            "Approved",
            "Active",
            "Verifying",
            "Completed",
            "Failed",
            "Superseded",
        ]
    );
}

#[test]
fn batch_boundary_rejects_zero_milestones_and_allows_zero_depth() {
    assert!(matches!(
        BatchBoundary::new(0, 0),
        Err(ProductError::InvalidBatchBoundary)
    ));
    let boundary = BatchBoundary::new(2, 0).unwrap();
    assert_eq!(boundary.max_milestones(), 2);
    assert_eq!(boundary.max_dependency_depth(), 0);
}

#[tokio::test]
async fn propose_next_batch_assigns_global_ordinals_across_two_batches() {
    let p = product().await;
    let generator = FixedMilestoneGenerator::new(vec![
        Ok(batch(vec![plain("one"), plain("two")])),
        Ok(batch(vec![plain("three"), plain("four")])),
    ]);
    let first = propose_next_batch(&p, &[], boundary(2, 0), &generator)
        .await
        .unwrap();
    assert_eq!(ids(&first), vec!["milestone/p1/0", "milestone/p1/1"]);

    let second = propose_next_batch(&p, &first, boundary(2, 0), &generator)
        .await
        .unwrap();
    assert_eq!(ids(&second), vec!["milestone/p1/2", "milestone/p1/3"]);

    let mut all: Vec<&str> = ids(&first);
    all.extend(ids(&second));
    all.sort_unstable();
    all.dedup();
    assert_eq!(all.len(), 4);
}

#[tokio::test]
async fn propose_next_batch_rejects_a_batch_larger_than_the_boundary() {
    let p = product().await;
    let generator = FixedMilestoneGenerator::new(vec![Ok(batch(vec![
        plain("one"),
        plain("two"),
        plain("three"),
    ]))]);
    let result = propose_next_batch(&p, &[], boundary(2, 0), &generator).await;
    assert!(matches!(
        result,
        Err(ProductError::BatchTooLarge {
            limit: 2,
            actual: 3
        })
    ));
}

#[tokio::test]
async fn propose_next_batch_rejects_an_empty_batch() {
    let p = product().await;
    let generator = FixedMilestoneGenerator::new(vec![Ok(batch(vec![]))]);
    let result = propose_next_batch(&p, &[], boundary(2, 0), &generator).await;
    assert!(matches!(result, Err(ProductError::EmptyBatch)));
}

#[tokio::test]
async fn propose_next_batch_rejects_empty_objectives_and_conditions() {
    let p = product().await;

    let generator = FixedMilestoneGenerator::new(vec![Ok(batch(vec![plain("")]))]);
    assert!(matches!(
        propose_next_batch(&p, &[], boundary(1, 0), &generator).await,
        Err(ProductError::EmptyMilestoneObjective)
    ));

    let generator = FixedMilestoneGenerator::new(vec![Ok(batch(vec![draft(
        "one",
        vec![],
        vec!["ok", ""],
        vec![],
    )]))]);
    assert!(matches!(
        propose_next_batch(&p, &[], boundary(1, 0), &generator).await,
        Err(ProductError::EmptyMilestoneCondition {
            field: "entry_conditions",
            index: 1
        })
    ));

    let generator = FixedMilestoneGenerator::new(vec![Ok(batch(vec![draft(
        "one",
        vec![],
        vec![],
        vec![""],
    )]))]);
    assert!(matches!(
        propose_next_batch(&p, &[], boundary(1, 0), &generator).await,
        Err(ProductError::EmptyMilestoneCondition {
            field: "exit_conditions",
            index: 0
        })
    ));
}

#[tokio::test]
async fn in_batch_dependencies_may_only_point_backwards() {
    let p = product().await;

    let generator = FixedMilestoneGenerator::new(vec![Ok(batch(vec![draft(
        "self",
        vec![MilestoneDependency::InBatch(0)],
        vec![],
        vec![],
    )]))]);
    assert!(matches!(
        propose_next_batch(&p, &[], boundary(1, 0), &generator).await,
        Err(ProductError::DependencyOutOfRange { index: 0 })
    ));

    let generator = FixedMilestoneGenerator::new(vec![Ok(batch(vec![
        plain("first"),
        draft(
            "forward",
            vec![MilestoneDependency::InBatch(2)],
            vec![],
            vec![],
        ),
    ]))]);
    assert!(matches!(
        propose_next_batch(&p, &[], boundary(2, 0), &generator).await,
        Err(ProductError::DependencyOutOfRange { index: 2 })
    ));
}

#[tokio::test]
async fn existing_dependencies_must_be_in_history() {
    let p = product().await;
    let generator = FixedMilestoneGenerator::new(vec![Ok(batch(vec![draft(
        "depends on unknown",
        vec![MilestoneDependency::Existing(MilestoneId::from_str("m-unknown"))],
        vec![],
        vec![],
    )]))]);
    let result = propose_next_batch(&p, &[], boundary(1, 0), &generator).await;
    assert!(matches!(
        result,
        Err(ProductError::UnknownMilestone { id }) if id == "m-unknown"
    ));
}

#[tokio::test]
async fn a_diamond_dag_is_accepted_and_fan_out_fan_in_is_preserved() {
    let p = product().await;
    let generator = FixedMilestoneGenerator::new(vec![Ok(batch(vec![
        plain("m1"),
        draft("m2", vec![MilestoneDependency::InBatch(0)], vec![], vec![]),
        draft("m3", vec![MilestoneDependency::InBatch(0)], vec![], vec![]),
        draft(
            "m4",
            vec![
                MilestoneDependency::InBatch(1),
                MilestoneDependency::InBatch(2),
            ],
            vec![],
            vec![],
        ),
    ]))]);
    let milestones = propose_next_batch(&p, &[], boundary(4, 2), &generator)
        .await
        .unwrap();
    let dependencies: Vec<&str> = milestones[3]
        .dependencies()
        .iter()
        .map(|d| d.as_str())
        .collect();
    assert_eq!(
        dependencies,
        vec![milestones[1].id().as_str(), milestones[2].id().as_str()]
    );
}

#[tokio::test]
async fn the_dependency_depth_limit_is_enforced() {
    let p = product().await;
    let chain = || {
        batch(vec![
            plain("m1"),
            draft("m2", vec![MilestoneDependency::InBatch(0)], vec![], vec![]),
            draft("m3", vec![MilestoneDependency::InBatch(1)], vec![], vec![]),
        ])
    };

    let generator = FixedMilestoneGenerator::new(vec![Ok(chain())]);
    assert!(matches!(
        propose_next_batch(&p, &[], boundary(3, 1), &generator).await,
        Err(ProductError::DependencyDepthExceeded {
            limit: 1,
            actual: 2
        })
    ));

    let generator = FixedMilestoneGenerator::new(vec![Ok(chain())]);
    let history = propose_next_batch(&p, &[], boundary(3, 2), &generator)
        .await
        .unwrap();

    let generator = FixedMilestoneGenerator::new(vec![Ok(batch(vec![draft(
        "depends on history only",
        vec![MilestoneDependency::Existing(history[0].id().clone())],
        vec![],
        vec![],
    )]))]);
    assert!(
        propose_next_batch(&p, &history, boundary(1, 0), &generator)
            .await
            .is_ok(),
        "Existing 依赖不计入批内深度"
    );
}

#[tokio::test]
async fn the_generator_sees_only_the_generated_history() {
    let p = product().await;
    let generator = FixedMilestoneGenerator::new(vec![
        Ok(batch(vec![plain("one"), plain("two")])),
        Ok(batch(vec![plain("three")])),
    ]);
    let first = propose_next_batch(&p, &[], boundary(2, 0), &generator)
        .await
        .unwrap();
    let second = propose_next_batch(&p, &first, boundary(1, 0), &generator)
        .await
        .unwrap();

    assert_eq!(generator.seen_history_lens(), vec![0, 2]);
    assert_eq!(generator.seen_boundaries(), vec![boundary(2, 0), boundary(1, 0)]);
    assert_eq!(second[0].id().as_str(), "milestone/p1/2");
    assert_eq!(first.len(), 2);
}

#[test]
fn there_is_no_api_that_returns_the_whole_future() {
    for source in [
        include_str!("../src/milestone.rs"),
        include_str!("../src/product.rs"),
        include_str!("../src/drive.rs"),
    ] {
        assert!(!source.contains("fn all_milestones"));
        assert!(!source.contains("fn full_plan"));
    }
}
