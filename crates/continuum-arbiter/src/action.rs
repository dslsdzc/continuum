/// 四档动作（切分 §2.3.2 第 2 条 `:158-159`）。恰四变体；没有 `Kill`。
///
/// `ResourceDecision.action` 是 `Option<ResourceAction>`；`None` 只表示「当前相位
/// 已经是所需状态，没有转移」，**不是第五动作**。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceAction {
    Run,
    Throttle,
    Suspend,
    Resume,
}

impl ResourceAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Run => "run",
            Self::Throttle => "throttle",
            Self::Suspend => "suspend",
            Self::Resume => "resume",
        }
    }

    /// 精确小写解析；未知文本返回 `None`。
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "run" => Some(Self::Run),
            "throttle" => Some(Self::Throttle),
            "suspend" => Some(Self::Suspend),
            "resume" => Some(Self::Resume),
            _ => None,
        }
    }
}
