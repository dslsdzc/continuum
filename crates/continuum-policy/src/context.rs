//! 策略求值的上下文（设计下篇第 5.6 节）。

use continuum_artifact::PrivacyClass;
use continuum_effect::EffectType;

/// `--approve` 是否给出（设计下篇第 5.5 节）。
///
/// 单元结构体而非 `bool`：它表示的是「驱动的显式确认存在」这一件事，不携带内容，
/// 也不可伪造出第二种取值。它占第 2 级 `ExplicitCurrent`，第 1 级之外的规则
/// 都可被它越过——定级见第 5.5 节，本类型只回答「是否给出」。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExplicitApproval;

/// 求值条件时可见的事实集合（设计下篇第 5.6 节）。
///
/// 每个字段都可能缺省（`None`）。缺省表示「该事实不在上下文中」，引用它的条件项
/// 不成立（设计下篇第 5.3 节）——**不是**报错，也**不是**恒真。唯一的例外是
/// [`PolicyContext::explicit_current`]：见该字段的说明。
///
/// 本类型是本层唯一会随其他阶段长大的类型：P3 的 `model` 字段就位后新增。新增
/// 字段会让原本不匹配的规则开始匹配，故每次扩充都要重新审视既有规则的语义
/// （设计下篇第 5.6 节）。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PolicyContext {
    /// `--approve` 是否给出。`Some` 即给出。
    ///
    /// 与其余四个字段不同，本事实**恒可观察**：上下文总是知道 `--approve` 给没给，
    /// 故 `None` 不是「事实不在上下文中」，而是「事实为假」。若按 `Option` 字面处理，
    /// 写成 `{"fact": "explicit_current", "eq": false}` 的规则将永不成立——一条
    /// `Deny` 规则静默失效即 fail-open，正是第 5.1 节要排除的形态。
    pub explicit_current: Option<ExplicitApproval>,
    /// 相关制品中最高的一档隐私等级。
    pub privacy_class: Option<PrivacyClass>,
    /// 本次待记的效应类型。
    pub effect_type: Option<EffectType>,
    /// 任务类别，由驱动注入。
    pub task_class: Option<String>,
    /// 任务已持续的时间。
    pub duration_ms: Option<u64>,
    // `model` 字段在 P3 就位后加；设计下篇第 5.6 节已记该类型的扩充风险。
}
