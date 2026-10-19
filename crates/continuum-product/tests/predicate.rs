//! §4.6 的发布谓词：构造校验与访问器。

use continuum_canonical::Opaque;
use continuum_product::{
    ProductError, ReadinessDimensionName, ShippableClause, ShippableClauseId, ShippablePredicate,
};

fn clause(id: &str, statement: &str) -> ShippableClause {
    ShippableClause::new(
        ShippableClauseId::from_str(id),
        Opaque::from_str(statement),
    )
    .unwrap()
}

#[test]
fn a_predicate_requires_at_least_one_dimension_and_one_clause() {
    let err = ShippablePredicate::new(vec![], vec![clause("c1", "s1")]).unwrap_err();
    assert!(matches!(err, ProductError::EmptyShippablePredicate));

    let err = ShippablePredicate::new(vec![ReadinessDimensionName::Functional], vec![])
        .unwrap_err();
    assert!(matches!(err, ProductError::EmptyShippablePredicate));
}

#[test]
fn duplicate_dimensions_are_rejected() {
    let err = ShippablePredicate::new(
        vec![
            ReadinessDimensionName::Functional,
            ReadinessDimensionName::Functional,
        ],
        vec![clause("c1", "s1")],
    )
    .unwrap_err();
    assert!(matches!(
        err,
        ProductError::DuplicateDimension {
            dimension: "functional"
        }
    ));
}

#[test]
fn duplicate_clause_ids_are_rejected() {
    let err = ShippablePredicate::new(
        vec![ReadinessDimensionName::Functional],
        vec![clause("c1", "s1"), clause("c1", "s2")],
    )
    .unwrap_err();
    assert!(matches!(err, ProductError::DuplicateClause { id } if id == "c1"));
}

#[test]
fn an_empty_clause_id_or_statement_is_rejected() {
    assert_eq!(ShippableClauseId::from_str("").as_str(), "");

    let err = ShippableClause::new(
        ShippableClauseId::from_str(""),
        Opaque::from_str("statement"),
    )
    .unwrap_err();
    assert!(matches!(err, ProductError::EmptyClauseId));

    let err = ShippableClause::new(
        ShippableClauseId::from_str("c1"),
        Opaque::from_str(""),
    )
    .unwrap_err();
    assert!(matches!(
        err,
        ProductError::EmptyClauseStatement { id } if id == "c1"
    ));
}

#[test]
fn accessors_return_the_constructed_lists_in_order() {
    let c1 = clause("c1", "first");
    let c2 = clause("c2", "second");
    let c3 = clause("c3", "third");
    let predicate = ShippablePredicate::new(
        vec![
            ReadinessDimensionName::Reliability,
            ReadinessDimensionName::Security,
        ],
        vec![c1.clone(), c2.clone(), c3.clone()],
    )
    .unwrap();

    assert_eq!(
        predicate.required_dimensions(),
        &[
            ReadinessDimensionName::Reliability,
            ReadinessDimensionName::Security,
        ]
    );
    assert_eq!(predicate.clauses().len(), 3);
    for (got, want) in predicate.clauses().iter().zip([&c1, &c2, &c3]) {
        assert_eq!(got.id(), want.id());
        assert_eq!(got.statement().as_str(), want.statement().as_str());
    }
    assert_eq!(c1.id().as_str(), "c1");
    assert_eq!(c1.statement().as_str(), "first");
}
