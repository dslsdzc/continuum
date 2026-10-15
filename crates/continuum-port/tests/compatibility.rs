use continuum_artifact::ArtifactType;
use continuum_port::{compatible, Direction, Port, PortError, PortId};

fn port(id: &str, direction: Direction, t: ArtifactType) -> Port {
    Port::new(PortId::new(id), direction, id, t)
}

#[test]
fn same_type_is_compatible() {
    let out = port("o1", Direction::Output, ArtifactType::Patch);
    let inp = port("i1", Direction::Input, ArtifactType::Patch);
    compatible(&out, &inp).expect("同类型应兼容");
}

#[test]
fn different_type_is_rejected_with_both_sides_named() {
    let out = port("o1", Direction::Output, ArtifactType::SourceTree);
    let inp = port("i1", Direction::Input, ArtifactType::Patch);
    match compatible(&out, &inp) {
        Err(PortError::TypeMismatch {
            from,
            from_type,
            to,
            to_type,
        }) => {
            assert_eq!(from.as_str(), "o1");
            assert_eq!(from_type, ArtifactType::SourceTree);
            assert_eq!(to.as_str(), "i1");
            assert_eq!(to_type, ArtifactType::Patch);
        }
        other => panic!("类型不同必须被拒绝，实际 {other:?}"),
    }
}

#[test]
fn two_outputs_are_rejected() {
    // 连接的两端必须一进一出
    let a = port("o1", Direction::Output, ArtifactType::Text);
    let b = port("o2", Direction::Output, ArtifactType::Text);
    match compatible(&a, &b) {
        Err(PortError::DirectionMismatch { .. }) => {}
        other => panic!("同向端口不得连接，实际 {other:?}"),
    }
}

#[test]
fn every_artifact_type_round_trips_through_serde() {
    for t in ArtifactType::ALL {
        let text = serde_json::to_string(&t).expect("可序列化");
        let back: ArtifactType = serde_json::from_str(&text).expect("可反序列化");
        assert_eq!(back, t, "类型 {t:?} 往返不一致");

        // 除往返外逐型再钉一件事：serde 串与 `as_str()` 逐字相同。
        //
        // 只断往返时这里有等价变异体：删掉 `artifact.rs` 的
        // `#[serde(rename_all = "snake_case")]` 不红——`"SubtitleTrack"`
        // 序列化后仍能解回 `SubtitleTrack`，往返照旧成立；能把两版分开的
        // 输入就是这两个串本身。它同时也是 `artifact.rs` 里「`rename_all =
        // "snake_case"` 给出的字符串与 `as_str` 逐型相同」那条注记的照片。
        let serde_key = text.trim_matches('"');
        assert_eq!(
            serde_key,
            t.as_str(),
            "类型 {t:?} 的 serde 串应与 as_str() 逐字相同"
        );
    }
}
