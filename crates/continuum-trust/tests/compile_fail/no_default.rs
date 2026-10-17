//! §9.2 q：`ContextTrust` 没有 `Default`，本样例必须**编不过**。
//! 红形态是「样例从编不过变成编得过」，不是运行期断言。

fn main() {
    let _ = continuum_trust::ContextTrust::default();
}
