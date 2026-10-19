//! §232 九维的构造面与源码面。
//!
//! 六条用例：字段名与顺序逐字、`iter()` 的规范序、三元组往返、空值／空证据／重复证据
//! 的拒绝、未评维的空三元组。

use continuum_canonical::Opaque;
use continuum_product::{
    ProductError, ProductReadiness, ReadinessDimension, ReadinessDimensionName, ReadinessValue,
};
use continuum_verify::EvidenceId;

fn field_names(src: &str, header: &str) -> Vec<String> {
    let start = src.find(header).expect("找不到结构体定义") + header.len();
    let rest = &src[start..];
    let end = rest.find("\n}").expect("结构体没有闭合");
    rest[..end]
        .lines()
        .filter_map(|line| {
            let t = line.trim();
            if t.is_empty() || t.starts_with("//") || t.starts_with("#[") {
                None
            } else {
                Some(t.split(':').next().expect("字段行没有冒号").trim().to_owned())
            }
        })
        .collect()
}

#[test]
fn the_nine_dimensions_are_verbatim_in_order() {
    let src = include_str!("../src/readiness.rs");
    assert_eq!(
        field_names(src, "pub struct ProductReadiness {"),
        [
            "functional",
            "correctness",
            "reliability",
            "performance",
            "security",
            "usability",
            "maintainability",
            "deployability",
            "documentation",
        ]
    );
}

#[test]
fn iter_yields_nine_names_in_normative_order() {
    let readiness = ProductReadiness::unassessed();
    let names: Vec<_> = readiness.iter().into_iter().map(|(name, _)| name).collect();
    assert_eq!(
        names,
        [
            ReadinessDimensionName::Functional,
            ReadinessDimensionName::Correctness,
            ReadinessDimensionName::Reliability,
            ReadinessDimensionName::Performance,
            ReadinessDimensionName::Security,
            ReadinessDimensionName::Usability,
            ReadinessDimensionName::Maintainability,
            ReadinessDimensionName::Deployability,
            ReadinessDimensionName::Documentation,
        ]
    );
    for (name, d) in readiness.iter() {
        assert!(d.value().is_unassessed(), "维度 {name:?} 不应已评估");
        assert!(d.evidence().is_empty());
        assert_eq!(d.last_verified(), None);
        assert!(std::ptr::eq(d, readiness.dimension(name)));
    }
}

#[test]
fn an_assessed_dimension_round_trips_its_three_parts() {
    let d = ReadinessDimension::assessed(
        Opaque::from_str("0.8"),
        vec![EvidenceId::from_str("e1")],
        100,
    )
    .unwrap();
    assert!(matches!(d.value(), ReadinessValue::Assessed(v) if v.as_str() == "0.8"));
    assert_eq!(d.evidence(), &[EvidenceId::from_str("e1")]);
    assert_eq!(d.last_verified(), Some(100));
}

#[test]
fn assessed_rejects_an_empty_value() {
    let err = ReadinessDimension::assessed(
        Opaque::from_str(""),
        vec![EvidenceId::from_str("e1")],
        100,
    )
    .unwrap_err();
    assert!(matches!(
        err,
        ProductError::EmptyReadinessValue {
            dimension: "<unbound>"
        }
    ));
}

#[test]
fn assessed_rejects_empty_or_duplicate_evidence() {
    let err = ReadinessDimension::assessed(Opaque::from_str("0.8"), vec![], 100).unwrap_err();
    assert!(matches!(
        err,
        ProductError::ReadinessValueWithoutEvidence {
            dimension: "<unbound>"
        }
    ));

    let e1 = EvidenceId::from_str("e1");
    let err = ReadinessDimension::assessed(
        Opaque::from_str("0.8"),
        vec![e1.clone(), e1],
        100,
    )
    .unwrap_err();
    assert!(matches!(
        err,
        ProductError::DuplicateEvidence { id } if id == "e1"
    ));

    assert!(
        ReadinessDimension::assessed(
            Opaque::from_str("0.8"),
            vec![EvidenceId::from_str("e1"), EvidenceId::from_str("e2")],
            100,
        )
        .is_ok()
    );
}

#[test]
fn unassessed_has_no_evidence_or_last_verified() {
    let d = ReadinessDimension::unassessed();
    assert!(d.value().is_unassessed());
    assert!(d.evidence().is_empty());
    assert_eq!(d.last_verified(), None);
    assert!(d.value().as_assessed().is_none());
}
