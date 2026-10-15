// 应编译失败：`ResolutionConfidence` **不能由分数构造**（§4.4 第 3 条：
// `retrieval_score` 与 `ResolutionConfidence` 两个数**不同轴**）。
//
// `retrieval_score`（§272）是**排序**用的、取值域未给；`ResolutionConfidence`（§275）
// 是「是否允许自动执行」用的、要求**经过校准**。故前者不能转换成后者——
// 本样例钉的就是这两条路都走不通：**没有** `from_score` 这条构造口，
// 也**没有** `impl From<f64> for ResolutionConfidence`。
//
// 正控制：`tests/compile_pass/resolution_confidence_meets_is_usable.rs`
// ——公开的构造入口是 `ResolutionConfidence::new(Opaque)`，那条路编得过。
use continuum_canonical::ResolutionConfidence;

fn main() {
    // 应编译失败：没有 `from_score` 这个构造口。
    let _ = ResolutionConfidence::from_score(0.9);

    // 应编译失败：也没有 `From<f64>`。
    let _ = ResolutionConfidence::from(0.9f64);
}
