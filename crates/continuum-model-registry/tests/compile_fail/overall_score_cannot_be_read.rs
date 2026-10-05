// 应编译失败：`ModelProfile` 上没有 `overall_score` 这个方法——§248 禁止把九维折成一个
// 总分作为路由依据（《工程》§4.4 的完成判据「不依赖单一总分」；设计 §2.3 第 3 条 (a)）。
//
// 判据是编译失败，**且必须是 E0599（没有这个方法）**：若这里因为别的缘故失败
// （名字拼错、隐私错误、导入缺失），本样例就分不出它声称钉住的那件事。
//
// 取形说明：本样例**不构造画像**（crate 外构造不出来，见另两份样例），故只声明一个收
// `&ModelProfile` 的函数并调用该方法——这足以让编译器在 `ModelProfile` 上找这个名字。
use continuum_model_registry::ModelProfile;

fn read_total(profile: &ModelProfile) -> f64 {
    profile.overall_score()
}

fn main() {
    // 取一次函数项，免得 `read_total` 触发 `dead_code` 警告混进本样例的 `.stderr`；
    // 函数体照常被类型检查，故上面那句仍是本样例要的那次解析。
    let _ = read_total;
}
