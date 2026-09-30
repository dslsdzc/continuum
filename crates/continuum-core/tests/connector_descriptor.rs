use continuum_core::connector::{ConnectorDescriptor, ConnectorId, ConnectorOp};

#[test]
fn empty_operation_list_is_rejected() {
    let id = ConnectorId::new("GitHub");
    let err = ConnectorDescriptor::new(id.clone(), vec![])
        .expect_err("空操作列表必须被拒绝");
    assert_eq!(
        err.to_string(),
        "Connector GitHub 未声明任何操作，退回服务级授权（§125）"
    );
}

#[test]
fn operation_scoped_descriptor_is_accepted() {
    let id = ConnectorId::new("GitHub");
    let ops = vec![
        ConnectorOp::new("GitHub.read_repo"),
        ConnectorOp::new("GitHub.push_branch"),
    ];
    let d = ConnectorDescriptor::new(id, ops).expect("非空操作列表应被接受");
    assert_eq!(d.operations.len(), 2);
    assert_eq!(d.operations[0].as_str(), "GitHub.read_repo");
}
