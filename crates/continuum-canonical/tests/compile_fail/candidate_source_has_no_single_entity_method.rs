// 应编译失败：`CandidateSource` 上**没有任何返回单一实体的方法**
// ——`source.pick_one(&m)` 这条路走不通（§197 的分工：候选生成负责高召回，不直接决定最终语义；
// 见设计 §4.3 末段与 §197 原文）。
//
// 它是简报里 `a_candidate_source_can_only_return_a_set` 的**反例那半边**
// （正例在 `tests/compile_pass/a_candidate_source_with_only_retrieve.rs`）。
//
// **本样例钉的限度（简报要求写明）**：它钉的是「**本层**不为单实体定义方法」，
// **不钉**「实现者内部返回唯一候选」——后者不是本层能观察的（§197 的高召回是**手段**层面的事），
// 由 Task 5 的四条断言（`§4.2` 的 B / D）在**解析侧**钉。
//
// 调用的入参取一枚**真切的** mention（走 `normalize` / `detect`），而不是手拼一个 `Mention`：
// 这样「`retrieve` 的入参形状」也不会成为本样例失败的旁因。
use continuum_canonical::{Candidate, Mention, CandidateSource, detect, normalize};

struct ABlindSource;

impl CandidateSource for ABlindSource {
    fn retrieve(&self, _m: &Mention) -> Vec<Candidate> {
        Vec::new()
    }
}

fn main() {
    let source = ABlindSource;
    let surface = normalize("请把它改掉");
    let mentions = detect(&surface);
    let m = &mentions[0];

    // 应编译失败：`CandidateSource` 没有这个方法。
    let _ = source.pick_one(m);
}
