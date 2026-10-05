//! 强制点 (1) 的类型侧保证（设计 §3.4、§8）：`AuthorizedTool` 字段私有、无公开
//! 构造函数，故 crate 外造不出来——「忘了校验」这条路径在类型上不存在。
//!
//! 运行期用例只能证明「我没构造过」，证明不了「构造不出来」，故用本样例。判据是
//! 编译失败，且失败必须落在「字段私有」上（E0451），而不是拼错名字之类的别的错。
//!
//! 构造通道只有这一条：本类型是**具名字段**结构体，不存在元组构造函数那道门
//! （`AuthorizedTool(..)` 会另报 E0423）。

use continuum_capability::{AuthorizedTool, ToolId};

fn main() {
    let _ = AuthorizedTool {
        tool_id: ToolId::new("t"),
        granted: Vec::new(),
    };
}
