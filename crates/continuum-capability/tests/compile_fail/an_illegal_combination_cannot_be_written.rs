// 应编译失败：半封闭词汇表 —— `Filesystem` 没有 `Push` 这个动作，
// 「`Filesystem.Push`」这类非法组合在类型层面拼不出（设计 §2.1）。
use continuum_capability::{CapabilityKind, FsAction};

fn main() {
    let _ = CapabilityKind::Filesystem(FsAction::Push);
}
