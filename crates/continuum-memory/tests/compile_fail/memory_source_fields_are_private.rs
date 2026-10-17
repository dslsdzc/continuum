// 应编译失败：`MemorySource` 五个字段全私有，构造点唯一。
// 样例：memory_source_fields_are_private（C5）。
use continuum_memory::{MemorySource, MemorySourceId, SourceKind};

fn main() {
    let _ = MemorySource {
        id: MemorySourceId::new("s"),
        kind: SourceKind::Files,
        external_ref: String::new(),
        observed_at: 0,
        removed_at: None,
    };
}
