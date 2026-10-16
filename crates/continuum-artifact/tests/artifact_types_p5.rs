//! **P5 的 11 枚新变体**（设计 §3.2）的编码与落库往返守卫。
//!
//! 与 `artifact_type.rs` 分开的两个理由：
//!
//! - 后者的语料从 `ArtifactType::ALL` 取，只覆盖「名单里的型都解得回去」；
//!   「11 枚新变体在不在名单里」与它们的**串名对不对**落不到它——本文件
//!   两条用例的语料都是**手写的 11 对**（变体, 串），不从 `ALL` 取。
//! - 后者的既有用例只改了数目字面量即可单独跑（运行期红点）；本文件在实现
//!   落地前会**编译失败**（`ArtifactType::Timeline` 等 11 枚变体还不存在），
//!   照实记账为编译期红点。
//!
//! 第 (b) 条（六个表外取值仍 `None`）由 `artifact_type.rs` 的既有断言覆盖，
//! 本文件不重复；11 个新串与那六个无一相同（本 task 的核对项）。

use continuum_artifact::{
    Artifact, ArtifactId, ArtifactStore, ArtifactType, ContentHash, PrivacyClass,
    p1_artifact_migrations,
};
use serde_json::json;

/// 11 枚 P5 新变体与其**手写的期望串**（设计 §3.2 的表，逐条照录）。
///
/// 这份清单必须手写：若写成遍历 `ArtifactType::ALL`，测的是「`ALL` 与 `parse`
/// 一致」——那正是 `ALL` 的既有守卫；而「11 枚在不在 `ALL` 里」不会被它测到
/// （设计 §12.2 末）。
fn p5_types() -> [(ArtifactType, &'static str); 11] {
    [
        (ArtifactType::Image, "image"),
        (ArtifactType::Audio, "audio"),
        (ArtifactType::Video, "video"),
        (ArtifactType::Timeline, "timeline"),
        (ArtifactType::Mesh, "mesh"),
        (ArtifactType::Scene, "scene"),
        (ArtifactType::Transcript, "transcript"),
        (ArtifactType::Report, "report"),
        (ArtifactType::ShotSet, "shot_set"),
        (ArtifactType::SubtitleTrack, "subtitle_track"),
        (ArtifactType::Render, "render"),
    ]
}

/// 设计 §12.2 的 (a) 与 (e)：11 枚新变体逐条断言**手写的期望串**与往返。
///
/// 承重的一半是 (i) `ty.as_str() == 期望串`：只断往返时，把 `shot_set` 写成
/// `shotset`（`as_str` 与 `parse` 两处一起改）仍全绿——那是一枚等价变异体
/// （设计 §12.3）；能把两版分开的输入就是期望串本身。多词型只有两条
/// （`shot_set` / `subtitle_track`），各在自己的那一对里。
///
/// (ii) `parse(期望串) == Some(ty)` 钉解码侧的对应臂。解码侧没有「加型即
/// 编译不过」的照片——`parse` 从 `&str` 出发，多一枚变体照样编译，漏臂只在
/// 运行时落到 `_ => return None`；那张照片是 `as_str` 的穷尽 match。
#[test]
fn every_p5_artifact_type_has_exactly_its_declared_encoding() {
    for (ty, expected) in p5_types() {
        assert_eq!(ty.as_str(), expected, "{ty:?} 的落库串应为 {expected:?}");
        assert_eq!(
            ArtifactType::parse(expected),
            Some(ty),
            "{expected} 应解回 {ty:?}"
        );
    }
}

/// 设计 §12.2 的 (c)：11 枚新变体逐枚写入 `artifact_type` 列、从库读回同一型。
///
/// 钉的是 `crates/continuum-artifact/src/persist.rs` 的 `parse_type` 委托链：
/// `load_artifact` 从 `artifact_type` 列读出串后委托到 `ArtifactType::parse`
/// （该文件不含自己的表）。若 `parse` 漏掉某一臂（如 `timeline`），该枚读回时
/// 不会解出型，`restore` 报「未知 ArtifactType」而不是静默换型。
///
/// 语料与上一条同为**手写的 11 枚**（不抽代表）：落库路径共享，但编码是逐型
/// 产生的，一枚代表不了另外十枚。
#[test]
fn a_new_type_survives_a_commit_and_read_back() {
    let dir = tempfile::tempdir().unwrap();
    let db = continuum_persist::Db::open_with(&dir.path().join("t.db"), migrations()).unwrap();
    db.migrate().unwrap();

    let tx = db.begin().unwrap();
    let mut store = ArtifactStore::new();
    for (i, (ty, _)) in p5_types().iter().enumerate() {
        store
            .commit(artifact_of_type(&format!("p5-{i}"), *ty, (i + 1) as u8))
            .unwrap();
    }
    store.persist(&tx, 1_724_000_000_000).unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let restored = ArtifactStore::restore(&tx).unwrap();
    for (i, (ty, _)) in p5_types().iter().enumerate() {
        let id = ArtifactId::new(format!("p5-{i}"));
        let got = restored
            .get(&id)
            .unwrap_or_else(|| panic!("{id} 应能从库中读回"));
        assert_eq!(
            got.artifact_type, *ty,
            "{id} 读回的 artifact_type 应与提交时相同"
        );
    }
}

/// 落库往返用的夹具：与 `artifact_store.rs` 的 `artifact(...)` 同形，
/// 只是类型可指定、内容按传入字节。不改后者——它的夹具固定 `Text`，
/// 动它会影响既有四条用例的语料。
fn artifact_of_type(id: &str, ty: ArtifactType, byte: u8) -> Artifact {
    let bytes = [byte];
    Artifact {
        id: ArtifactId::new(id),
        artifact_type: ty,
        content_hash: ContentHash::of(&bytes),
        size: bytes.len() as u64,
        producer_node: Some("n1".to_owned()),
        input_artifacts: vec![],
        metadata: json!({}),
        provenance: json!({}),
        privacy_class: PrivacyClass::Personal,
        version: 1,
    }
}

fn migrations() -> Vec<continuum_persist::Migration> {
    let mut m = continuum_persist::builtin_migrations();
    m.extend(p1_artifact_migrations());
    m
}
