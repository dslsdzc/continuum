//! 规则形状、六级与受限条件（设计下篇第 5.1–5.4 节）。

use continuum_artifact::PrivacyClass;
use continuum_effect::EffectType;
use serde_json::Value;
use thiserror::Error;

use crate::context::PolicyContext;

/// 设计下篇第 5.2 节的六级。数字即顺序，**越大越弱**——裁决取最小的那一级。
///
/// 第 1 级是内建常量、不可配置；第 3、4 级来自本层存储；第 5 级内建；第 6 级
/// 下篇不产生（不采纳）。第 2 级在下篇由驱动的 `--approve` 占位，P4 就位后移交。
///
/// 层级是代码中的常量，不落库、无 setter：优先级不可学习这条约束由结构给出，
/// 不由文档约定（设计下篇第 5.4 节）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    SystemSafety = 1,
    ExplicitCurrent = 2,
    UserPersistent = 3,
    Project = 4,
    RuntimeDefault = 5,
    ModelSuggestion = 6,
}

/// 一条规则的裁决（设计下篇第 5.1 节）。
///
/// 本类型不派生 `Ord`：同层内的严苛次序（`Deny` > `RequireApproval` > `Allow`）
/// 是裁决规则的一部分，由裁决函数表达，不借枚举的声明次序暗中给出（第 5.3 节）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny,
    RequireApproval,
}

/// 规则的来源层（设计下篇第 5.1 节）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    User,
    Project,
}

/// 一条策略规则（设计下篇第 5.1 节）。
#[derive(Debug, Clone, PartialEq)]
pub struct Policy {
    pub level: Level,
    pub condition: Condition,
    pub decision: Decision,
    pub scope: Scope,
}

/// 受限的 JSON 谓词：事实名 / 比较 / 取值的合取（设计下篇第 5.1 节）。
///
/// 表示形式（顶层二选一）：
/// - `{"all": [谓词...]}`——合取；`{"all": []}` 是空合取，**恒真**；
/// - 单个谓词对象——一元合取，与 `{"all": [该谓词]}` 等价。
///
/// 谓词形如 `{"fact": "effect_type", "eq": "charge"}`，恰有一个比较符
/// （`eq` / `in` / `gte`）。事实名命中一个封闭集合（与 [`PolicyContext`] 的字段
/// 一一对应），取值须与该事实的类型相符。
///
/// **任何一项不符即整条条件被判为 `Err`，不是「不匹配」**：一条写错的条件若静默
/// 退化为「永不匹配」，会让人以为策略生效了而实际没有（设计下篇第 5.1 节）。
///
/// 求值不会失败，只有成立与不成立两种结果：写错在解析期就被挡下，求值时不成立的
/// 来源是取值不符或事实缺省（设计下篇第 5.3 节）。
#[derive(Debug, Clone, PartialEq)]
pub struct Condition(Vec<Predicate>);

impl Condition {
    /// 解析。任何一项不符即返回 `Err`——**不是**「不匹配」。
    pub fn parse(value: &Value) -> Result<Self, PolicyError> {
        let map = value.as_object().ok_or_else(|| PolicyError::Malformed {
            reason: format!("条件应为对象，实为{}", json_kind(value)),
        })?;

        let Some(all) = map.get("all") else {
            // 裸谓词：一元合取。与 `{"all": [该谓词]}` 同义，不增加表达力。
            return Ok(Self(vec![Predicate::parse(value)?]));
        };
        if map.len() != 1 {
            return Err(PolicyError::Malformed {
                reason: "`all` 不能与其它键并列".into(),
            });
        }
        let items = all.as_array().ok_or_else(|| PolicyError::Malformed {
            reason: format!("`all` 应为数组，实为{}", json_kind(all)),
        })?;

        let mut predicates = Vec::with_capacity(items.len());
        for item in items {
            predicates.push(Predicate::parse(item)?);
        }
        Ok(Self(predicates))
    }

    /// 求值。所引用的事实不在上下文中即该项不成立（设计下篇第 5.3 节）。
    ///
    /// 合取：每一项都成立才成立；空合取恒真。
    pub fn matches(&self, ctx: &PolicyContext) -> bool {
        self.0.iter().all(|p| p.matches(ctx))
    }
}

/// 合取中的一项。
#[derive(Debug, Clone, PartialEq)]
struct Predicate {
    fact: Fact,
    matcher: Matcher,
}

impl Predicate {
    fn parse(value: &Value) -> Result<Self, PolicyError> {
        let map = value.as_object().ok_or_else(|| PolicyError::Malformed {
            reason: format!("谓词应为对象，实为{}", json_kind(value)),
        })?;

        let fact_name = match map.get("fact") {
            Some(Value::String(name)) => name.as_str(),
            Some(other) => {
                return Err(PolicyError::Malformed {
                    reason: format!("`fact` 应为字符串，实为{}", json_kind(other)),
                });
            }
            None => {
                return Err(PolicyError::Malformed {
                    reason: "谓词缺 `fact`".into(),
                });
            }
        };
        let fact = Fact::parse(fact_name).ok_or_else(|| PolicyError::UnknownFact {
            name: fact_name.to_string(),
        })?;

        // 除 `fact` 外的键必须恰有一个，且须是已知比较符。`gt` 之类的错拼落在这里。
        let mut operators = map.iter().filter(|(key, _)| key.as_str() != "fact");
        let Some((op_name, op_value)) = operators.next() else {
            return Err(PolicyError::Malformed {
                reason: "谓词缺比较符".into(),
            });
        };
        if operators.next().is_some() {
            return Err(PolicyError::Malformed {
                reason: "谓词有多个比较符".into(),
            });
        }
        let op = Op::parse(op_name).ok_or_else(|| PolicyError::UnknownOperator {
            name: op_name.clone(),
        })?;

        // `gte` 只对 `duration_ms` 有意义：给字符串或布尔事实排序没有定义。
        // 拒绝而不是「不匹配」，理由同第 5.1 节。
        if op == Op::Gte && fact != Fact::DurationMs {
            return Err(PolicyError::OperatorNotApplicable {
                fact: fact.name(),
                op: op.name(),
            });
        }

        let matcher = match op {
            Op::Eq => Matcher::Eq(parse_fact_value(fact, op_value)?),
            // 上面的适用性检查保证走到这里的只有 `duration_ms`，而
            // `parse_fact_value` 对该事实只产出 `FactValue::Number`。
            Op::Gte => Matcher::Gte(parse_fact_value(fact, op_value)?),
            Op::In => {
                let items = op_value.as_array().ok_or_else(|| PolicyError::ValueTypeMismatch {
                    fact: fact.name(),
                    expected: "数组",
                })?;
                let mut values = Vec::with_capacity(items.len());
                for item in items {
                    values.push(parse_fact_value(fact, item)?);
                }
                Matcher::In(values)
            }
        };
        Ok(Self { fact, matcher })
    }

    fn matches(&self, ctx: &PolicyContext) -> bool {
        let Some(observed) = self.fact.observe(ctx) else {
            // 设计下篇第 5.3 节：所引用的事实不在上下文中时，该规则不匹配。
            // 这不等于放行——放行必须由某条明确的 `Allow` 给出。
            return false;
        };
        match &self.matcher {
            Matcher::Eq(expected) => *expected == observed,
            // 空列表恒不成立。它是结构合法的条件（三项「不符」都不占），
            // 故解析期接受，不拒。
            Matcher::In(list) => list.contains(&observed),
            Matcher::Gte(bound) => observed.reaches(bound),
        }
    }
}

/// 一项比较。
#[derive(Debug, Clone, PartialEq)]
enum Matcher {
    /// 取值相等。
    Eq(FactValue),
    /// 取值命中列表中的一项。
    In(Vec<FactValue>),
    /// 数值不小于下界。解析期已保证它只落在 `duration_ms` 上，故其值必是数值。
    Gte(FactValue),
}

/// 一个比较符。封闭集合，与设计下篇第 5.1 节一致。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Eq,
    In,
    Gte,
}

impl Op {
    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "eq" => Self::Eq,
            "in" => Self::In,
            "gte" => Self::Gte,
            _ => return None,
        })
    }

    fn name(self) -> &'static str {
        match self {
            Self::Eq => "eq",
            Self::In => "in",
            Self::Gte => "gte",
        }
    }
}

/// 谓词可引用的事实名。封闭集合，与 [`PolicyContext`] 的字段一一对应
/// （设计下篇第 5.1 节：事实名必须命中 `PolicyContext` 的字段）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fact {
    ExplicitCurrent,
    PrivacyClass,
    EffectType,
    TaskClass,
    DurationMs,
}

impl Fact {
    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "explicit_current" => Self::ExplicitCurrent,
            "privacy_class" => Self::PrivacyClass,
            "effect_type" => Self::EffectType,
            "task_class" => Self::TaskClass,
            "duration_ms" => Self::DurationMs,
            _ => return None,
        })
    }

    fn name(self) -> &'static str {
        match self {
            Self::ExplicitCurrent => "explicit_current",
            Self::PrivacyClass => "privacy_class",
            Self::EffectType => "effect_type",
            Self::TaskClass => "task_class",
            Self::DurationMs => "duration_ms",
        }
    }

    /// 该事实在上下文中的当前取值；`None` 即事实不在上下文中。
    ///
    /// 只有 `explicit_current` 恒有取值——见 [`PolicyContext::explicit_current`]。
    fn observe(self, ctx: &PolicyContext) -> Option<FactValue> {
        match self {
            Self::ExplicitCurrent => Some(FactValue::Bool(ctx.explicit_current.is_some())),
            Self::PrivacyClass => ctx.privacy_class.map(FactValue::Privacy),
            Self::EffectType => ctx.effect_type.map(FactValue::Effect),
            Self::TaskClass => ctx.task_class.clone().map(FactValue::Text),
            Self::DurationMs => ctx.duration_ms.map(FactValue::Number),
        }
    }
}

/// 事实的取值。同一类型既装字面量也装观察值，故比较不需要第二套表示。
#[derive(Debug, Clone, PartialEq)]
enum FactValue {
    Bool(bool),
    Privacy(PrivacyClass),
    Effect(EffectType),
    Text(String),
    Number(u64),
}

impl FactValue {
    /// 数值是否不小于 `bound`。任一侧不是数值即取 `false`：解析期已拒绝
    /// 「`gte` 用于非数值事实」，故该分支不出现；万一出现，判为不成立是更严的一侧。
    fn reaches(&self, bound: &Self) -> bool {
        matches!((self, bound), (Self::Number(n), Self::Number(b)) if n >= b)
    }
}

/// 按事实的类型解析一个字面量。类型不符即返回 `Err`（设计下篇第 5.1 节）。
fn parse_fact_value(fact: Fact, value: &Value) -> Result<FactValue, PolicyError> {
    let mismatch = |expected: &'static str| PolicyError::ValueTypeMismatch {
        fact: fact.name(),
        expected,
    };
    match fact {
        Fact::ExplicitCurrent => value
            .as_bool()
            .map(FactValue::Bool)
            .ok_or_else(|| mismatch("布尔")),
        Fact::TaskClass => value
            .as_str()
            .map(|s| FactValue::Text(s.to_string()))
            .ok_or_else(|| mismatch("字符串")),
        Fact::DurationMs => value
            .as_u64()
            .map(FactValue::Number)
            .ok_or_else(|| mismatch("非负整数")),
        Fact::PrivacyClass => {
            let name = value.as_str().ok_or_else(|| mismatch("字符串"))?;
            privacy_from_name(name)
                .map(FactValue::Privacy)
                .ok_or_else(|| PolicyError::UnknownValue {
                    fact: fact.name(),
                    value: name.to_string(),
                })
        }
        Fact::EffectType => {
            let name = value.as_str().ok_or_else(|| mismatch("字符串"))?;
            effect_from_name(name)
                .map(FactValue::Effect)
                .ok_or_else(|| PolicyError::UnknownValue {
                    fact: fact.name(),
                    value: name.to_string(),
                })
        }
    }
}

/// `EffectType` 的字符串名。编码与 `continuum-effect` 的落库编码同名同形。
///
/// 该编码的权威表在 `continuum_effect::persist` 内且不可达（`effect_type_str` 是
/// `pub(crate)`、`parse_effect_type` 是模块私有），故此处自建一份。两个方向分开写：
/// `effect_name` 的 match **穷尽且无通配臂**——给 `EffectType` 加变体时它编译失败，
/// 作者被迫回到本文件；本函数从字符串出发，编译器帮不上忙，由本文件的 `tests` 模块
/// 的往返断言兜住。往返断言盖不住的那一种情形记在那里。
///
/// `effect_name` 只在测试里用（它的存在就是为了让上面那次编译失败发生），故随之
/// `cfg(test)`：它不参与任何生产路径。
fn effect_from_name(name: &str) -> Option<EffectType> {
    Some(match name {
        "send_email" => EffectType::SendEmail,
        "push_branch" => EffectType::PushBranch,
        "publish" => EffectType::Publish,
        "delete_remote" => EffectType::DeleteRemote,
        "charge" => EffectType::Charge,
        "deploy" => EffectType::Deploy,
        _ => return None,
    })
}

#[cfg(test)]
fn effect_name(effect: EffectType) -> &'static str {
    match effect {
        EffectType::SendEmail => "send_email",
        EffectType::PushBranch => "push_branch",
        EffectType::Publish => "publish",
        EffectType::DeleteRemote => "delete_remote",
        EffectType::Charge => "charge",
        EffectType::Deploy => "deploy",
    }
}

/// `PrivacyClass` 的字符串名。理由与 `effect_name` 同：权威表在
/// `continuum_artifact::persist` 内且不可达（`privacy_str` 与 `parse_privacy` 都是
/// 模块私有），且本函数同样只为测试而存在（见 `effect_name` 的说明）。
fn privacy_from_name(name: &str) -> Option<PrivacyClass> {
    Some(match name {
        "public" => PrivacyClass::Public,
        "personal" => PrivacyClass::Personal,
        "private" => PrivacyClass::Private,
        "secret" => PrivacyClass::Secret,
        "local_only" => PrivacyClass::LocalOnly,
        _ => return None,
    })
}

#[cfg(test)]
fn privacy_name(privacy: PrivacyClass) -> &'static str {
    match privacy {
        PrivacyClass::Public => "public",
        PrivacyClass::Personal => "personal",
        PrivacyClass::Private => "private",
        PrivacyClass::Secret => "secret",
        PrivacyClass::LocalOnly => "local_only",
    }
}

/// JSON 取值的种类，只用于错误信息。
fn json_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "空值",
        Value::Bool(_) => "布尔",
        Value::Number(_) => "数值",
        Value::String(_) => "字符串",
        Value::Array(_) => "数组",
        Value::Object(_) => "对象",
    }
}

/// 条件解析的失败。每一种「写错」各有一个可区分的变体，使调用方能分辨
/// 「这条规则为什么被拒」，而不是只知道被拒。
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PolicyError {
    /// 结构不符：顶层不是对象、`all` 不是数组或与其它键并列、谓词缺键或多键、
    /// `fact` 不是字符串。
    #[error("条件结构不符：{reason}")]
    Malformed { reason: String },
    /// 事实名不在封闭集合内。
    #[error("未知事实名：{name}")]
    UnknownFact { name: String },
    /// 比较符不在封闭集合内。
    #[error("未知比较符：{name}")]
    UnknownOperator { name: String },
    /// 比较符不适用于该事实的类型（如对字符串事实用 `gte`）。
    #[error("比较符 {op} 不适用于事实 {fact}")]
    OperatorNotApplicable {
        fact: &'static str,
        op: &'static str,
    },
    /// 取值的 JSON 类型与事实不符。
    #[error("事实 {fact} 的取值类型不符，期望{expected}")]
    ValueTypeMismatch {
        fact: &'static str,
        expected: &'static str,
    },
    /// 取值是字符串，但不是该事实的封闭取值之一。
    #[error("事实 {fact} 的取值不在封闭集合内：{value}")]
    UnknownValue { fact: &'static str, value: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `EffectType` 的全部变体。
    ///
    /// 与 `EffectState::ALL` 同形。数目断言的必要性也在同一处：漏加新变体时
    /// 长度不变，只遍历名单的断言照过。此处比那处多一层——
    /// [`effect_name`] 的 match 穷尽，加变体时编译失败会把作者带到这里。
    /// 仍盖不住的情形：作者补了 [`effect_name`] 的臂而不补本名单，
    /// 此时往返断言走不到新变体，但 `effect_from_name` 若也漏补，
    /// 任何用到新变体名的条件都会在解析期报 `UnknownValue`，不会静默错判。
    const ALL_EFFECTS: [EffectType; 6] = [
        EffectType::SendEmail,
        EffectType::PushBranch,
        EffectType::Publish,
        EffectType::DeleteRemote,
        EffectType::Charge,
        EffectType::Deploy,
    ];

    /// `PrivacyClass` 的全部变体。理由同上。
    const ALL_PRIVACY: [PrivacyClass; 5] = [
        PrivacyClass::Public,
        PrivacyClass::Personal,
        PrivacyClass::Private,
        PrivacyClass::Secret,
        PrivacyClass::LocalOnly,
    ];

    /// 两张表都必须与其枚举互逆，且没有两个变体共用一个名字。
    #[test]
    fn both_name_tables_are_bijective() {
        assert_eq!(ALL_EFFECTS.len(), 6, "EffectType 的名单与变体数不符");
        assert_eq!(ALL_PRIVACY.len(), 5, "PrivacyClass 的名单与变体数不符");

        for effect in ALL_EFFECTS {
            let name = effect_name(effect);
            assert_eq!(
                effect_from_name(name),
                Some(effect),
                "{name} 不能解回 {effect:?}"
            );
        }
        for privacy in ALL_PRIVACY {
            let name = privacy_name(privacy);
            assert_eq!(
                privacy_from_name(name),
                Some(privacy),
                "{name} 不能解回 {privacy:?}"
            );
        }

        let mut effect_names: Vec<&str> = ALL_EFFECTS.into_iter().map(effect_name).collect();
        effect_names.sort_unstable();
        effect_names.dedup();
        assert_eq!(effect_names.len(), ALL_EFFECTS.len(), "EffectType 有两个变体重名");

        let mut privacy_names: Vec<&str> = ALL_PRIVACY.into_iter().map(privacy_name).collect();
        privacy_names.sort_unstable();
        privacy_names.dedup();
        assert_eq!(privacy_names.len(), ALL_PRIVACY.len(), "PrivacyClass 有两个变体重名");
    }
}
