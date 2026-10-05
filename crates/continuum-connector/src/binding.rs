//! 一条「操作 → 授权」的绑定（设计 §3.2.1）。

use continuum_capability::CapabilityKind;
use continuum_core::connector::ConnectorOp;

/// 一条「操作 → 授权」的绑定。**两个字段都是既有类型，本 crate 不新增词汇表**（设计 §3.2）。
///
/// 字段**只有这两个**：没有 `EffectType`——效应是**推出来的**（由 `CapabilityKind::effect`
/// 给出），不是声明的（设计 §3.2 第 4 条）。这条「写不出来」由 Task 6 的编译失败样例钉。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpBinding {
    op: ConnectorOp,
    kind: CapabilityKind,
}

impl OpBinding {
    /// **必须有这个公开构造入口**：连接器作者（以及本 crate 的集成测试，它们在 crate 外）
    /// 要在 `bindings()` 里造出绑定，而字段私有、无构造函数会让 `ConnectorImpl` 实现不了。
    /// 没有可校验的东西（两个入参各自的构造期校验已在它们自己的类型上做过），故不返回 `Result`。
    pub fn new(op: ConnectorOp, kind: CapabilityKind) -> Self {
        Self { op, kind }
    }

    /// 绑定的操作。供注册期核对与入口解析使用（本 crate 内部）。
    pub(crate) fn op(&self) -> &ConnectorOp {
        &self.op
    }

    /// 绑定到的能力类别。供注册期一一核对与入口第 4 步使用（本 crate 内部）。
    pub(crate) fn kind(&self) -> CapabilityKind {
        self.kind
    }
}
