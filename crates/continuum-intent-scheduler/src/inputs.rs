use continuum_semantics::{Intent, IntentId, Priority};

#[derive(Debug)]
pub struct SchedulerInputs<'a> {
    pub candidates: &'a [Intent],
    pub priority: &'a [PrioritySignal],
    pub deadline: &'a [Deadline],
    pub blocking: &'a [Blocking],
    pub user_focus: UserFocus,
    pub background_suitability: BackgroundSuitability,
}

#[derive(Debug, Clone)]
pub struct PrioritySignal {
    pub intent: IntentId,
    pub priority: Priority,
}

#[derive(Debug, Clone)]
pub struct Deadline {
    pub intent: IntentId,
    pub at_millis: i64,
}

#[derive(Debug, Clone)]
pub struct Blocking {
    pub intent: IntentId,
    pub blocks: Vec<IntentId>,
}

#[derive(Debug, Clone)]
pub enum UserFocus {
    None,
    Intent(IntentId),
    Area(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundSuitability(String);

impl BackgroundSuitability {
    pub fn from_projection(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn projection(&self) -> &str {
        &self.0
    }
}
