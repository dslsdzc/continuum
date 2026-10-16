//! §308 的第一枚 `Checkpointable` 实现（`generate-broll` 与 `render`）的守卫
//! （设计 §4.5 的三条判据，逐条两侧）。
//!
//! 用例的语料（算子 id、后端名、版本号）**手写**、逐条取自设计 §5.2 第 11、12 行与 §4.5：
//! 若从实现侧取语料，测的是「它等于它自己」——那是恒真的假照片
//! （设计 §12.2 末对 (a)(f) 的点名同形；`tests/permission.rs` 同纪律）。
//! **两处刻意的例外**：`checkpointing_without_progress_is_rejected` 与
//! `the_two_implementations_are_the_two_named_operators`——它们**就是**核「实现与清单（`all_operators()`）
//! 的纯数据对应」的用例（设计 §4.5：这条对应编译期拦不住，须由用例逐枚核），
//! 故对照物取清单，而实现侧的 id 是硬编码的（`GenerateBroll::new()` / `Render::new()` 各一处）。
//!
//! 各条与设计条目的对应：(o) 判据 1 正例、(p) 判据 1 反例、(q) 判据 2 两侧、
//! 后端两侧 = 判据 3、末条 = 「实现是那两枚算子」的补件（计划 `## 遗留` 第三节申报）。

use std::sync::Mutex;

use continuum_media::{GenerateBroll, Progress, Render, all_operators};
use continuum_operator::definition::CheckpointOwner;
use continuum_operator::{
    BackendId, CheckpointError, Checkpointable, OperatorId, OperatorVersion,
};

/// 判据 1／2 的「逐字段相同」：对 `Progress` 与 `CheckpointOwner` 各做**穷尽解构**
/// （不带 `..`——任一枚类型加字段则本函数编译失败，提醒把新字段纳入比较），逐字段断言。
///
/// `Mutex` 字段的比较：锁（`lock()`）后比内部值；两把锁互不相干，无交错持有。
fn assert_progress_matches(context: &str, actual: &Progress, expected: &Progress) {
    let Progress {
        owner,
        completed_segments,
        selected_backend,
    } = actual;
    let Progress {
        owner: expected_owner,
        completed_segments: expected_completed,
        selected_backend: expected_backend,
    } = expected;

    // 身份也逐字段（`CheckpointOwner` 没有 `PartialEq`，且「逐字段」要两层都写得出）。
    let CheckpointOwner { id, version } = owner;
    let CheckpointOwner {
        id: expected_id,
        version: expected_version,
    } = expected_owner;

    assert_eq!(id, expected_id, "{context}：owner.id 应与存前相同");
    assert_eq!(version, expected_version, "{context}：owner.version 应与存前相同");
    assert_eq!(
        *completed_segments.lock().expect("Progress 的锁不应中毒"),
        *expected_completed.lock().expect("Progress 的锁不应中毒"),
        "{context}：已完成的片段应与存前相同"
    );
    assert_eq!(
        *selected_backend.lock().expect("Progress 的锁不应中毒"),
        *expected_backend.lock().expect("Progress 的锁不应中毒"),
        "{context}：选中的 backend 应与存前相同"
    );
}

/// 判据 1 正例 (o)：`checkpoint()` 在**有进度**时返回 `Ok`，且 `restore` 该检查点后
/// 算子的进度与**存前**逐字段相同（以 `progress()` 取存前的值，穷尽解构见上面的 helper）。
///
/// **中间刻意造成偏离**（再推进一个片段 ＋ 把 backend 换成候选外的 `nvenc`）——
/// 「恢复」因此有可观察的内容，而不是与「从来没变过」混同；
/// **偏离的 backend 取候选外的 `nvenc` 是刻意的**：`restore` 只认身份、不重跑
/// `checkpoint()` 的 backend 判定（判据 3 的判定在 `checkpoint()` 一侧）。
///
/// **红条件（移除档）**：把 `restore` 的写回删掉（只校验不写回——`no-op` 的最强形态）
/// ⇒ 恢复后进度停在偏离后的状态、与存前不同，红。
#[test]
fn a_checkpoint_restores_the_progress_it_recorded() {
    let mut broll = GenerateBroll::new();
    broll.record_completed_segment(0);
    broll.record_completed_segment(3);
    broll.select_backend(BackendId::new("video-gen-backend"));

    // 存前的值：以 progress() 取。
    let before = broll.progress().clone();
    assert!(
        !before
            .completed_segments
            .lock()
            .expect("Progress 的锁不应中毒")
            .is_empty(),
        "前置条件：record_completed_segment 之后进度应非空"
    );

    let checkpoint = broll
        .checkpoint()
        .expect("有进度时 checkpoint() 应返回 Ok（判据 1 正例）");

    // 偏离：推进更多片段、把 backend 选择换掉——恢复要能把两者都带回存前。
    broll.record_completed_segment(9);
    broll.select_backend(BackendId::new("nvenc"));

    broll
        .restore(&checkpoint)
        .expect("本实例自己产出的检查点应被接受（判据 1 正例的恢复侧）");

    assert_progress_matches("恢复后", broll.progress(), &before);
}

/// 判据 1 反例 (p)：**尚无进度**时 `checkpoint()` ⇒ `Err(NothingToCheckpoint { owner })`，
/// 且 `owner` 的 id 与 version **是这一枚算子**（对照物取自清单 `all_operators()`：
/// 本条核的就是「实现与清单的纯数据对应」，见文件头的纪律说明）。
///
/// **红条件（取反档）**：尚无进度时返回一个「空进度」的检查点并给 `Ok` ⇒ 红。
#[test]
fn checkpointing_without_progress_is_rejected() {
    let broll = GenerateBroll::new();

    let err = broll
        .checkpoint()
        .expect_err("尚无进度时 checkpoint() 必须被拒（判据 1 反例）");

    match err {
        CheckpointError::NothingToCheckpoint { owner } => {
            let CheckpointOwner { id, version } = &owner;
            let declared = all_operators()
                .into_iter()
                .find(|op| op.id == OperatorId::new("generate-broll"))
                .expect("清单 all_operators() 里应有 generate-broll");
            assert_eq!(id, &declared.id, "owner.id 应是这一枚算子（generate-broll）");
            assert_eq!(
                version, &declared.version,
                "owner.version 应与清单里那一枚一致"
            );
        }
        other => panic!("应为 CheckpointError::NothingToCheckpoint，实际 {other:?}"),
    }
}

/// 判据 2 (q)，两侧：用**另一版本**（`OperatorVersion` 不同）的**同一算子**的检查点去
/// `restore` ⇒ `Err(RestoreRejected { .. })`，且**恢复失败后算子状态与调用前逐字段相同**
/// （半恢复比不恢复更坏——失败的恢复必须是零写入的）。
/// **两侧的另一半**：同一版本、同一算子的检查点 `restore` ⇒ `Ok`
/// （否则「一律 `Err`」也能满足前一半）。
///
/// **本用例用 `Render` 那一枚实现**（Task 10 Step 5 第 4 轮的变异据此落到同一枚上；
/// 两枚实现的恢复判定走同一个私有 helper，故「变异只落未被 (q) 用到的另一枚」的
/// 失效形态在结构上不可能成立——见实现侧的注释）。
///
/// **红条件（移除档）**：删掉身份比对（一律 `Ok`）⇒ 第一个子例红（返回 `Ok`）。
#[test]
fn restoring_another_operators_checkpoint_is_rejected_and_leaves_state_unchanged() {
    let mut render = Render::new();
    render.record_completed_segment(5);
    render.select_backend(BackendId::new("ffmpeg"));
    let before = render.progress().clone();

    // 另一版本、同一算子的检查点：**直接构造**（`Progress` 字段全 pub，无需绕过路径）。
    // **载荷刻意取与 `before` 不同的值**（`[7]` / `nvenc`）：若实现变成「先写回、后校验」
    // （半恢复形态），状态会被写成这里的载荷、下面 `assert_progress_matches` 当场红；
    // 载荷若与 `before` 相同，那一腿就没有判别力（写回结果恰好等于存前值、静默绿）。
    let alien = Progress {
        owner: CheckpointOwner {
            id: OperatorId::new("render"),
            version: OperatorVersion::new(2),
        },
        completed_segments: Mutex::new(vec![7]),
        selected_backend: Mutex::new(Some(BackendId::new("nvenc"))),
    };

    let err = render
        .restore(&alien)
        .expect_err("另一版本的检查点必须被拒（判据 2）");

    match err {
        CheckpointError::RestoreRejected { expected, found } => {
            let CheckpointOwner {
                id: expected_id,
                version: expected_version,
            } = &expected;
            let CheckpointOwner {
                id: found_id,
                version: found_version,
            } = &found;
            assert_eq!(
                expected_id,
                &OperatorId::new("render"),
                "expected 应是本实例的算子（render）"
            );
            assert_eq!(
                expected_version,
                &OperatorVersion::new(1),
                "expected 的版本应是本实例的（1）"
            );
            assert_eq!(
                found_id,
                &OperatorId::new("render"),
                "found 应是检查点携带的算子（同一算子）"
            );
            assert_eq!(
                found_version,
                &OperatorVersion::new(2),
                "found 的版本应是检查点携带的（另一版本）"
            );
        }
        other => panic!("应为 CheckpointError::RestoreRejected，实际 {other:?}"),
    }

    // 半恢复比不恢复更坏：失败的恢复不允许动状态。
    assert_progress_matches("恢复失败后", render.progress(), &before);

    // 两侧的另一半：同一版本、同一算子的检查点 ⇒ Ok。
    let mine = render
        .checkpoint()
        .expect("失败恢复之后状态未变（仍有进度），checkpoint() 应返回 Ok");
    render
        .restore(&mine)
        .expect("同一版本、同一算子的检查点应被接受");
}

/// 判据 3，两侧：以 `select_backend` 指定 backend 之后——
/// 选中的 backend 在**该算子清单枚的 `backend_candidates` 内** ⇒ `checkpoint()` 为 `Ok`；
/// 否则 ⇒ `Err(UnsupportedByBackend { backend })`，**且 `backend` 正是选中的那一枚**。
/// **两侧各一条，两枚实现各一轮**（判据 3 是每枚实现的义务）。
///
/// 取值的出处（设计 §5.2 第 11、12 行）：`render` 的候选是 `ffmpeg`、
/// `generate-broll` 的候选是 `video-gen-backend`；`nvenc` 不在 17 枚的任何候选里
/// （下面有一条语料自检把它钉住）。
///
/// **红条件（取反档）**：把不支持的后端也放行为 `Ok` ⇒ 两条不支持侧红
/// （这正是「假装检查点成功」的形态）。
#[test]
fn an_unsupported_backend_is_rejected_and_a_supported_one_succeeds() {
    // 语料自检：`nvenc` 必须真的不在任何算子的候选里——否则「不支持侧」测的就不是
    // 「候选外的一枚」。将来某枚算子收了它，这里红，提醒重选语料。
    assert!(
        all_operators()
            .iter()
            .all(|op| !op.backend_candidates.contains(&BackendId::new("nvenc"))),
        "语料自检：nvenc 不应出现在任何算子的候选里"
    );

    // 支持侧：候选内的一枚 ⇒ Ok。
    let mut broll = GenerateBroll::new();
    broll.record_completed_segment(0);
    broll.select_backend(BackendId::new("video-gen-backend"));
    broll
        .checkpoint()
        .expect("video-gen-backend 是 generate-broll 的候选，应返回 Ok");

    let mut render = Render::new();
    render.record_completed_segment(0);
    render.select_backend(BackendId::new("ffmpeg"));
    render
        .checkpoint()
        .expect("ffmpeg 是 render 的候选，应返回 Ok");

    // 不支持侧：候选外的一枚 ⇒ UnsupportedByBackend，且 backend 正是选中的那一枚。
    let mut broll = GenerateBroll::new();
    broll.record_completed_segment(0);
    broll.select_backend(BackendId::new("nvenc"));
    let err = broll
        .checkpoint()
        .expect_err("nvenc 不在 generate-broll 的候选内，必须被拒");
    match err {
        CheckpointError::UnsupportedByBackend { backend } => {
            assert_eq!(
                backend,
                BackendId::new("nvenc"),
                "backend 正是选中的那一枚"
            );
        }
        other => panic!("应为 CheckpointError::UnsupportedByBackend，实际 {other:?}"),
    }

    let mut render = Render::new();
    render.record_completed_segment(0);
    render.select_backend(BackendId::new("nvenc"));
    let err = render
        .checkpoint()
        .expect_err("nvenc 不在 render 的候选内，必须被拒");
    match err {
        CheckpointError::UnsupportedByBackend { backend } => {
            assert_eq!(
                backend,
                BackendId::new("nvenc"),
                "backend 正是选中的那一枚"
            );
        }
        other => panic!("应为 CheckpointError::UnsupportedByBackend，实际 {other:?}"),
    }
}

/// 两枚实现是那两枚算子：`operator_id()` 与 `all_operators()` 里 `generate-broll` /
/// `render` 的 `id` 相同（**逐枚**），**且它们确实实现了 `Checkpointable`**
/// （照片机制：以泛型 `T: Checkpointable<Checkpoint = Progress>` 调一次两个方法——
/// 关联类型也一并钉住；若某枚没实现，这里是 `E0277`、编译不过）。
///
/// **红条件（取反档）**：把 `render` 的 `operator_id()` 改成另一枚算子的 id ⇒ 红。
#[test]
fn the_two_implementations_are_the_two_named_operators() {
    let declared = all_operators();
    let declared_id_of = |id: &str| {
        declared
            .iter()
            .find(|op| op.id == OperatorId::new(id))
            .expect("清单 all_operators() 里应有该枚算子")
            .id
            .clone()
    };

    assert_eq!(
        GenerateBroll::new().operator_id(),
        declared_id_of("generate-broll"),
        "GenerateBroll::operator_id() 应是清单里 generate-broll 的 id"
    );
    assert_eq!(
        Render::new().operator_id(),
        declared_id_of("render"),
        "Render::operator_id() 应是清单里 render 的 id"
    );

    // 照片机制：只经 `Checkpointable` 的接口调两方法各一次
    // （不用 `///`：函数体内的嵌套 item 上不挂文档注释，避免 unused_doc_comments）。
    fn exercise<T: Checkpointable<Checkpoint = Progress>>(op: &T) -> Result<(), CheckpointError> {
        let checkpoint = op.checkpoint()?;
        op.restore(&checkpoint)
    }

    let mut broll = GenerateBroll::new();
    broll.record_completed_segment(0);
    exercise(&broll).expect("GenerateBroll 应实现 Checkpointable，且两方法可用");

    let mut render = Render::new();
    render.record_completed_segment(0);
    exercise(&render).expect("Render 应实现 Checkpointable，且两方法可用");
}
