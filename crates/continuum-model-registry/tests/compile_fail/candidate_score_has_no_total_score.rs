// 应编译失败：`CandidateScore` 上没有 `overall` 这个字段——§248 禁止把九维折成一个总分
// 作为路由依据（《工程》§4.4 的完成判据「不依赖单一总分」；设计 §5.3）。
//
// 判据是编译失败，**且必须是 E0609（no field on struct）**：若这里因为别的缘故失败
// （名字拼错、字段私有、导入缺失、缺 `main`），本样例就分不出它声称钉住的那件事。
//
// 与 `overall_score_cannot_be_read.rs` 的分工：那一份钉 `ModelProfile` 上读不到总分
// （E0599，**方法**名不存在）；本份钉 **`CandidateScore` 这个接口面**上没有总分子段
// （E0609，**字段**不存在）。§248 的禁令落在两个类型上，故**两份样例各钉一侧、不合并**
// （一份样例只钉一条通道，见 `tests/type_level.rs` 的文件头）。
//
// 取形说明：`CandidateScore` 的三个字段都是 `pub`（设计 §5.3 要的是「策略在 crate 外也能构造它」），
// 故本样例**不必**构造一枚实例——声明一个收 `&CandidateScore` 的函数并读那个不存在的字段，
// 编译器就会在该类型上找这个名字。构造它需要 `RoutingReason`，那会引入与本题无关的依赖。
use continuum_model_registry::CandidateScore;

fn read_total(score: &CandidateScore) -> f64 {
    score.overall
}

fn main() {
    // 取一次函数项，免得 `read_total` 触发 `dead_code` 警告混进本样例的 `.stderr`；
    // 函数体照常被类型检查，故上面那句仍是本样例要的那次字段解析。
    let _ = read_total;
}
