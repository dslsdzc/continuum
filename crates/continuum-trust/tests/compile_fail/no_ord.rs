//! §9.2 r：`ContextTrust` 没有 `PartialOrd`，本样例必须**编不过**。
//! 红形态是「样例从编不过变成编得过」，不是运行期断言。

use continuum_trust::ContextTrust;

fn main() {
    let _ = ContextTrust::UserInstruction < ContextTrust::WebUntrusted;
}
