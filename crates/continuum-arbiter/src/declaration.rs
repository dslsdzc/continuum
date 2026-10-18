/// 四档优先级的分类输入。**不是**任务来源枚举；三个布尔描述的是「这份工作是谁的」，
/// 不是「它从哪个授权来源来」。分类规则见设计 §5.3。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkloadDeclaration {
    /// 用户直接在机器上操作的外部工作负载（不是 runtime 任务）。
    pub user_process: bool,
    /// 用户在等待其结果的 runtime 任务。
    pub foreground: bool,
    /// 维护计划中的任务。
    pub maintenance: bool,
}
