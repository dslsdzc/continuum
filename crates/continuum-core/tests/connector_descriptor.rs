use continuum_core::connector::{ConnectorDescriptor, ConnectorId, ConnectorOp};

fn op(s: &str) -> ConnectorOp {
    ConnectorOp::new(s).expect("合法操作标识")
}

fn github() -> ConnectorId {
    ConnectorId::new("GitHub")
}

#[test]
fn empty_operation_list_is_rejected() {
    let err = ConnectorDescriptor::new(github(), vec![]).expect_err("空操作列表必须被拒绝");
    assert_eq!(
        err.to_string(),
        "Connector GitHub 未声明任何操作，退回服务级授权（§125）"
    );
}

#[test]
fn operation_scoped_descriptor_is_accepted() {
    let d = ConnectorDescriptor::new(github(), vec![op("GitHub.read_repo"), op("GitHub.push_branch")])
        .expect("非空操作列表应被接受");
    assert_eq!(d.id().as_str(), "GitHub");
    assert_eq!(d.operations().len(), 2);
    assert_eq!(d.operations()[0].as_str(), "GitHub.read_repo");
}

#[test]
fn malformed_operation_id_is_rejected() {
    assert!(ConnectorOp::new("").is_err(), "空串必须被拒绝");
    assert!(ConnectorOp::new("GitHub").is_err(), "不含 . 的标识必须被拒绝");
    assert_eq!(op("GitHub.push_branch").as_str(), "GitHub.push_branch");
}

#[test]
fn deserialization_cannot_bypass_the_operation_check() {
    let empty = r#"{"id":"GitHub","operations":[]}"#;
    let err = serde_json::from_str::<ConnectorDescriptor>(empty)
        .expect_err("反序列化不得绕过 §125 的空操作检查");
    assert!(err.to_string().contains("未声明任何操作"), "实际: {err}");

    let bad_op = r#"{"id":"GitHub","operations":["GitHub"]}"#;
    assert!(
        serde_json::from_str::<ConnectorDescriptor>(bad_op).is_err(),
        "反序列化不得接受形状非法的操作标识"
    );
}

#[test]
fn descriptor_round_trips_through_serde() {
    let d = ConnectorDescriptor::new(github(), vec![op("GitHub.push_branch")])
        .expect("非空操作列表应被接受");
    let text = serde_json::to_string(&d).expect("可序列化");
    let back: ConnectorDescriptor = serde_json::from_str(&text).expect("可反序列化");
    assert_eq!(back, d);
}
