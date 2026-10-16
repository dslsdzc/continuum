//! §6 的谱系照片（设计 §11.2 的 (m)、(n) 条）：**两处判据都在 P1，本块只重跑**。
//!
//! **这两条不是本块判据的照片**——本块的 crate 里**没有产出 Artifact 的代码**
//! （本块不实现 `OperatorImpl`，切分 §四第 4 条），故两条都是 **P1 写读路径的重跑**：
//! 它们钉「`input_artifacts` 与 `id` 经 `save_artifact` / `load_artifact` 往返可读」
//! 与「同 id 写第二次被拒、原行不变」，**钉不到**「本域的算子产出时确实这么填」。
//! 后者的承重者是 §327 在 P1 侧的那条**缺席**（`artifact.id` 是 `TEXT PRIMARY KEY`
//! ＋ 全仓没有任何 `UPDATE artifact`，切分 §四第 7 条第四处）——**它没有承重的函数**、
//! **不在本块**。**证据强度较低**：复审不宜按与本块别的用例同等的强度读它们
//! （计划 `## 遗留` 第四节）。
//!
//! **照片坐标**：`continuum-persist` 是本 crate 的 **dev 边**（生产代码对它零调用、
//! `src/` 里 `continuum_persist` 零命中）；这条边的使用点就是本文件。**这一条边不在
//! 设计 §2.1 的边表里、是本计划自定的一处**（取舍与理由见计划 `## 遗留` 第一节第 1 条，
//! 收件人：设计作者 ＋ 协调者）——`continuum-artifact` 只重导出 `save_artifact` /
//! `load_artifact` 与迁移，不重导出 `Db` / `Tx`，故「起真库」必须命名本 crate。
//!
//! 起库形状照本仓既有先例（`crates/continuum-artifact/tests/artifact_store.rs:109-110`）：
//! `tempfile::TempDir` ＋ `Db::open_with`（`builtin_migrations()` 加
//! `p1_artifact_migrations()`）＋ `db.migrate()` ＋ `db.begin()`。

use continuum_artifact::{
    Artifact, ArtifactId, ArtifactType, ContentHash, PrivacyClass, load_artifact,
    p1_artifact_migrations, save_artifact,
};
use continuum_persist::{Db, Migration, Value, builtin_migrations};

/// 起一个真库：P0 内建迁移（events / audit_log）＋ P1 的 artifact 迁移
/// （`artifact` / `artifact_input` 两张表，`artifact.id` 是主键）。
fn db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let mut migrations: Vec<Migration> = builtin_migrations();
    migrations.extend(p1_artifact_migrations());
    let db = Db::open_with(&dir.path().join("t.db"), migrations).unwrap();
    db.migrate().unwrap();
    (dir, db)
}

/// (m) 的夹具：一枚编辑产出的制品——`id = "img-v2"`、`input_artifacts = ["img-v1"]`、
/// `version = 2`、`artifact_type = Image`（P5e 的变体；设计 §6.1 的谱系形状）。
fn edited_artifact() -> Artifact {
    let bytes = b"edited-image-bytes";
    Artifact {
        id: ArtifactId::new("img-v2"),
        artifact_type: ArtifactType::Image,
        content_hash: ContentHash::of(bytes),
        size: bytes.len() as u64,
        producer_node: None,
        input_artifacts: vec![ArtifactId::new("img-v1")],
        // `metadata` / `provenance` 的字段约定未给（设计 §6.3）；本照片不涉它们，
        // 取 `Value` 的默认值（JSON null）。
        metadata: Default::default(),
        provenance: Default::default(),
        privacy_class: PrivacyClass::Personal,
        version: 2,
    }
}

/// (n) 的第二枚写入：同一个 id、内容与第一枚**可观察地不同**（`version` 与
/// `content_hash` 都不同）。
///
/// **为什么必须不同**：红条件（把 `INSERT INTO` 变异成 `INSERT OR REPLACE INTO`）
/// 若写的是逐字段相同的一份，REPLACE 之后 `before == after` 仍成立——那条红条件
/// 就成了恒真，判据也随之变假。故意让第二枚不同，红条件才有判别力。
fn overwriting_artifact() -> Artifact {
    let bytes = b"overwrite-attempt";
    Artifact {
        id: ArtifactId::new("img-v2"),
        artifact_type: ArtifactType::Image,
        content_hash: ContentHash::of(bytes),
        size: bytes.len() as u64,
        producer_node: None,
        input_artifacts: vec![ArtifactId::new("img-v1")],
        metadata: Default::default(),
        provenance: Default::default(),
        privacy_class: PrivacyClass::Personal,
        version: 3,
    }
}

/// §11.2 第 (m) 条：编辑产出制品的 `input_artifacts == [source_id]` 且
/// `id != source_id`（钉的是 §6.1 的谱系边）。
///
/// **这一条不是本块判据的照片——它是 P1 写读路径的重跑**：本块的 crate 里没有
/// 产出 Artifact 的代码（不实现 `OperatorImpl`），故它钉的是「`input_artifacts` 与
/// `id` 经 `save_artifact` / `load_artifact` 往返可读」，**钉不到**「本域的算子产出时
/// 确实这么填」。后者的承重者是 §327 在 P1 侧的**缺席**（`artifact.id` 是
/// `TEXT PRIMARY KEY` ＋ 全仓没有任何 `UPDATE artifact`，切分 §四第 7 条第四处）——
/// 它没有承重的函数、不在本块。
///
/// **红条件（靶子不在本块的交付物里，只做一轮本地变异）**：把 `continuum-artifact`
/// 的 `save_artifact` 里写 `artifact_input` 的那个循环删掉（**移除档**）→ 本条红
/// （读回的 `input_artifacts` 变空、裸查查不到那一行）。
/// **那一轮只能在本地工作树上做、必须还原，不许把那个改动提交**（计划纪律 2）。
#[test]
fn an_edited_artifact_carries_the_source_in_its_inputs_and_a_new_id() {
    let (_dir, db) = db();

    // 写一枚并提交（形状照先例：写事务显式 commit、读事务直接结束）。
    let tx = db.begin().unwrap();
    save_artifact(&tx, &edited_artifact(), "event-img-v2", 1_000).unwrap();
    tx.commit().unwrap();

    // 读回（`load_artifact` 反查 `artifact_input` 还出谱系边）。
    let tx = db.begin().unwrap();
    let back = load_artifact(&tx, &ArtifactId::new("img-v2"))
        .unwrap()
        .expect("写入已提交，应能读回");

    assert_eq!(back.id, ArtifactId::new("img-v2"), "读回的 id 应是 img-v2");
    assert_eq!(
        back.input_artifacts,
        vec![ArtifactId::new("img-v1")],
        "编辑产出制品的 input_artifacts 应恰为 [源图]（§6.1 的谱系边）"
    );
    assert_ne!(
        back.id,
        ArtifactId::new("img-v1"),
        "编辑产出的制品必须带新 id（§241：不得覆盖源图）"
    );

    // **裸查 `artifact_input` 表**：上面读的是还原结果，这一条直接查落库的行——
    // 确认写路径把边写进了表，且表里恰有 `(artifact_id, input_artifact_id) ==
    // ("img-v2", "img-v1")` 这一行（不多不少）。
    let rows = tx
        .query(
            "SELECT artifact_id, input_artifact_id FROM artifact_input",
            &[],
        )
        .unwrap();
    assert_eq!(
        rows,
        vec![vec![Value::text("img-v2"), Value::text("img-v1")]],
        "artifact_input 表里应恰有 (img-v2, img-v1) 这一行"
    );
}

/// §11.2 第 (n) 条：用同一个 id 经 `save_artifact` 写第二次 ⇒ `Err`，且原行不变。
/// **这是 P1 判据的重跑**（设计 §6.2 逐字：「用同一个 id 写第二次 ⇒ `Err`（**这是
/// P1 判据的照片，本块只是重跑它**）」）——本块**不建第二份「源图未被覆盖」的检查**。
///
/// **判据的作用域**：本条钉的是**「行没被改」**，**不是**「行还在」——只判「行还在」
/// 的话，一个 `INSERT OR REPLACE` 的实现照样绿。故本条断两件事：第二次写入返回
/// `Err`，且第三个事务读回的 `after` 与写前读到的 `before` **逐字段相同**
/// （`Artifact` 有 `PartialEq`）。
///
/// **红条件（靶子同样不在本块）**：把 `save_artifact` 的 `INSERT INTO artifact` 改成
/// `INSERT OR REPLACE INTO artifact`（**移除档**）→ 本条红（`after != before`）。
/// **那一轮只能在本地工作树上做、必须还原，不许把那个改动提交**（计划纪律 2）。
///
/// **具体是哪一种 `PersistError` 不在本计划预设**：`Err` 的 Debug 读数打在日志里
/// （见下方 `println!`），报告里照实记录。
#[test]
fn a_second_write_of_the_same_artifact_id_is_rejected_and_the_row_is_unchanged() {
    let (_dir, db) = db();
    let id = ArtifactId::new("img-v2");

    // 事务一：写第一枚并提交。
    let tx = db.begin().unwrap();
    save_artifact(&tx, &edited_artifact(), "event-img-v2-a", 1_000).unwrap();
    tx.commit().unwrap();

    // 事务二（读）：写前读数 before。
    let before = {
        let tx = db.begin().unwrap();
        load_artifact(&tx, &id)
            .unwrap()
            .expect("第一次写入后应能读回")
    };

    // 事务三（写第二次）：同一个 id ⇒ Err。**不提交**——照 `save_artifact` 的文档注释
    // （返回 `Err` 之后调用方必须回滚该事务，不得提交），此处由 `Tx` 结束作用域时的
    // Drop（`ROLLBACK`）落回滚。
    {
        let tx = db.begin().unwrap();
        let err = save_artifact(&tx, &overwriting_artifact(), "event-img-v2-b", 2_000)
            .expect_err("用同一个 id 写第二次必须被拒绝（§241：不覆盖）");
        // 不预设变体名（brief 如此）：把实际读数打进日志，供 Task 7 报告引用
        // （`cargo test … -- --nocapture`）。
        println!("同 id 第二次写入返回的 Err：{err:?}");
    }

    // 事务四（读）：写后读数 after ⇒ 与 before 逐字段相同。
    let after = {
        let tx = db.begin().unwrap();
        load_artifact(&tx, &id)
            .unwrap()
            .expect("被拒的写入不得破坏原行")
    };

    assert_eq!(
        before, after,
        "同 id 的第二次写入必须不改变原行（§241：不覆盖）"
    );
}
