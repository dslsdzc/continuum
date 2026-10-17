// 应编译失败：`MemoryClaim::new` 是 `pub(crate)`，crate 外唯一入口是
// `apply_synthesis`（测试面另有 `for_test`，本样例不碰它）。
// 样例：claim_constructor_is_crate_private（C2）。
use continuum_memory::{
    Confidence, MemoryClaim, MemoryClaimId, MemoryScope, MemorySourceId,
};

fn main() {
    let _ = MemoryClaim::new(
        MemoryClaimId::new("c"),
        MemoryScope::Personal,
        "陈述".to_string(),
        vec![MemorySourceId::new("s")],
        Confidence::from_value(serde_json::json!(null)),
        0,
    );
}
