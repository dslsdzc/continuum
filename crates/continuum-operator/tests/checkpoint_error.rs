//! `Checkpointable` 错误类型的照片（P5e Task 2）。
//!
//! `CheckpointError` **没有** `as_str` / `parse` / `ALL`——它的编码就是 thiserror 的
//! `Display`（本仓 `OperatorError`（`src/registry.rs:6-12`）同形：一个带载荷的错误
//! 枚举没有字符串往返这一说）。设计 §12.1 订正前要求它有 as_str/parse 与数目断言，
//! 与本设计 §4.3 的类型相抵；该处已按本计划的结论订正（§12.1 现文与本节同）——
//! 见计划 `## 遗留` 第一节第 2 条。

use continuum_operator::definition::CheckpointOwner;
use continuum_operator::{BackendId, CheckpointError, Checkpointable, OperatorId, OperatorVersion};

fn owner(id: &str, version: u32) -> CheckpointOwner {
    CheckpointOwner {
        id: OperatorId::new(id),
        version: OperatorVersion::new(version),
    }
}

#[test]
fn each_of_the_three_arms_renders_its_own_discriminating_text() {
    // 三臂各一条，不抽代表。
    //
    // 取反档（把 CheckpointOwner 的 Display 写成只打印 id、丢掉版本）⇒ 第（2）（3）条红。
    // 这一条钉的是「检查点的身份是 id ＋ version 两字段」这件事在**输出上**可见——
    // 设计 §4.3 明写 CheckpointOwner 的两个字段与 OperatorRef 逐字相同而**不合并**，
    // 故它就是那个「两个字段都要有人读」的地方。

    // （1）UnsupportedByBackend 的文案含 nvenc。
    // 取反档（把 BackendId 的 Display 实现成 Debug 形态 BackendId("nvenc")）⇒ 本条红：
    // 文案里会带引号与括号——与 backend_id_display_names_the_backend 一起红（同一个
    // Display 被两处消费，这是刻意）。
    let err = CheckpointError::UnsupportedByBackend {
        backend: BackendId::new("nvenc"),
    };
    assert_eq!(err.to_string(), "backend nvenc 不支持检查点（§308）");

    // （2）NothingToCheckpoint 的文案含该算子的 id 与版本两者。
    let err = CheckpointError::NothingToCheckpoint {
        owner: owner("generate-broll", 7),
    };
    assert_eq!(err.to_string(), "算子 generate-broll@7 尚无可检查点的状态");

    // （3）RestoreRejected 的文案同时含两个 owner 的 id 与版本。
    let err = CheckpointError::RestoreRejected {
        expected: owner("render", 3),
        found: owner("generate-broll", 7),
    };
    assert_eq!(
        err.to_string(),
        "检查点属于 generate-broll@7，本实例是 render@3，恢复被拒"
    );
}

#[test]
fn backend_id_display_names_the_backend() {
    // 取反档（实现成 Debug 形态 BackendId("ffmpeg")）⇒ 本条红；
    // 且 UnsupportedByBackend 那条文案里会带引号与括号——两条用例会一起红
    //（这是刻意：同一个 Display 被两处消费）。
    assert_eq!(BackendId::new("ffmpeg").to_string(), "ffmpeg");
}

/// 编译期照片，照实写：trait 的两枚方法签名收的是 `CheckpointError`。
///
/// 两条腿的差别：
/// （i）「签名收的是 CheckpointError」是**编译期照片**——trait 若仍是 `Result<_, String>`，
/// 这个实现体编不过（那是 `could not compile`，不算变红）；
/// （ii）**运行期可观察的那一半**是「取回的错误能按三臂之一匹配」。
///
/// 红条件（移除档）：把 NothingToCheckpoint 与 RestoreRejected 合成一臂 ⇒ 实现体里的
/// match 臂对不上、编译失败——故本条的失败路径守卫是第一条用例那三臂的文案断言，
/// 不是这一条。
struct Minimal;

impl Checkpointable for Minimal {
    type Checkpoint = u32;

    fn checkpoint(&self) -> Result<u32, CheckpointError> {
        Err(CheckpointError::NothingToCheckpoint {
            owner: owner("minimal", 1),
        })
    }

    fn restore(&self, _c: &u32) -> Result<(), CheckpointError> {
        Ok(())
    }
}

#[test]
fn the_two_trait_methods_are_typed_by_checkpoint_error() {
    match Minimal.checkpoint() {
        Err(CheckpointError::NothingToCheckpoint { .. }) => {}
        other => panic!("Minimal 的检查点必须按三臂之一取回，实际 {other:?}"),
    }
    assert!(Minimal.restore(&0).is_ok(), "restore 必须成功");
}

#[test]
fn the_two_names_are_reachable_from_the_crate_root() {
    // re-export 的照片：文件顶部 `use continuum_operator::{Checkpointable, CheckpointError};`
    //（不用全路径）。红条件（移除档）：删掉 lib.rs 的那两枚 re-export ⇒ E0432 未解析
    // 导入，即编译不过、不算变红；故本条的运行期照片只有一条：本文件能被 `cargo test`
    // 收住并跑起来。
    //
    // P1 计划里记的「Checkpointable 未 re-export」是遗留项（按内容指，不写行号），
    // 本 task 一并清掉。
    let err = CheckpointError::UnsupportedByBackend {
        backend: BackendId::new("vulkan"),
    };
    assert!(
        matches!(err, CheckpointError::UnsupportedByBackend { .. }),
        "构造的臂必须与匹配的臂一致，实际 {err:?}"
    );
}
