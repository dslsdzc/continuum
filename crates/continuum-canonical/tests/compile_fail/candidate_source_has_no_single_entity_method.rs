// 应编译失败：`source.pick_one(&m)` 这条路走不通——`CandidateSource` 上没有 `pick_one`
// 这个方法（§197 的分工：候选生成负责高召回，不直接决定最终语义；见设计 §4.3 末段与 §197 原文）。
// **注意钉的是哪个名字**：本样例的判据是那份同名 `.stderr`，逐字比对的正是
// `E0599: no method named \`pick_one\``——即它钉的是**这个方法名**，
// 不是「本层没有任何单实体方法」那句更宽的话。
//
// 它是简报里 `a_candidate_source_can_only_return_a_set` 的**反例那半边**
// （正例在 `tests/compile_pass/a_candidate_source_with_only_retrieve.rs`）。
//
// **本样例钉的限度（简报要求写明）**：它钉的是「**本层没有 `pick_one` 这个口子**」，**不钉**两件事：
// 1. 「实现者内部返回唯一候选」——后者不是本层能观察的（§197 的高召回是**手段**层面的事），
//    由 Task 5 的四条断言（`§4.2` 的 B / D）在**解析侧**钉。
// 2. 「别的名字的单实体方法也不存在」——方法名是**开放集**，`trybuild` 枚举不完；
//    一个叫别的名字、又带默认实现的单实体方法加进来，本样例仍绿。详见 `src/candidate.rs`
//    的「这条守卫的限度」一节。
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
