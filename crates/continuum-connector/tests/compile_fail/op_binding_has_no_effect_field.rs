// 应编译失败：`OpBinding` 上没有 `effect` 字段——效应是**推出来的**（由
// `CapabilityKind::effect` 给出），不是声明的（设计 §3.2 第 4 条、§9）。
//
// **形状是刻意的，判别力全靠它**：`OpBinding` 的字段是**私有**的，故任何外部结构体
// 字面量（`OpBinding { op, kind }`）都编译不过，而 rustc 报的是**隐私错误**
// （E0451 / E0603 一类），不是「没有 `effect` 字段」——那样这条样例就分不出
// 「效应是推出来的」与「外部构造不出 `OpBinding`」，**声称钉住的东西其实没钉住**。
// 故本样例先经**公开构造入口** `OpBinding::new(op, kind)` 造出绑定，再对它做**字段
// 赋值**：这条路径只碰字段名，不碰可见性，失败因而落在
// **E0609 / `no field \`effect\` on type \`OpBinding\``** 上。

use continuum_capability::{CapabilityKind, GitAction};
use continuum_connector::OpBinding;
use continuum_core::connector::ConnectorOp;

fn main() {
    let mut binding = OpBinding::new(
        ConnectorOp::new("git.push").expect("用例里的操作串合法（非空且含 `.`）"),
        CapabilityKind::Git(GitAction::Push),
    );
    binding.effect = None;
}
