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

impl Level {
    /// 全部六个层级。供全量遍历的用例使用（与 `continuum_effect::EffectType::ALL` 同形，
    /// 那里的说明同样适用，含它挡不住的那一种情形）。
    ///
    /// 完整性与数目由 `tests/persist.rs` 的
    /// `every_level_round_trips_through_its_encoding` 把关。
    pub const ALL: [Level; 6] = [
        Level::SystemSafety,
        Level::ExplicitCurrent,
        Level::UserPersistent,
        Level::Project,
        Level::RuntimeDefault,
        Level::ModelSuggestion,
    ];

    /// `level` 列的落库编码：小写、多词以 `_` 连接（设计下篇第 8 节）。
    ///
    /// **取名字而非判别值**，虽然本枚举的判别值恰好是 1–6 的序号：数字编码会让
    /// 「加一级」变成破坏性变更——在中间插入一层时，既有行的数字全部要改写（而
    /// 判别值本身也随之改变，两件事都得做对）；名字编码则不受插入影响。编码定义
    /// 放在这几行判别值旁边，正是为了让这条理由与它约束的东西在同一处。
    ///
    /// match 穷尽且无通配臂：加层级时本函数编译不过，编码不会漏分支。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SystemSafety => "system_safety",
            Self::ExplicitCurrent => "explicit_current",
            Self::UserPersistent => "user_persistent",
            Self::Project => "project",
            Self::RuntimeDefault => "runtime_default",
            Self::ModelSuggestion => "model_suggestion",
        }
    }

    /// [`Level::as_str`] 的**严格逆**：对 [`Level::ALL`] 里的每个层级都有
    /// `Level::parse(l.as_str()) == Some(l)`，表外字符串一律 `None`。
    ///
    /// `None` 而不是默认层级：层级是优先级的依据，取默认会让裁决判错
    /// （与 `crate::persist` 的解码同一条理由）。
    ///
    /// 解码侧没有穷尽 match 的保护——来源是 `&str` 而非枚举，编译器点不出漏掉的变体，
    /// 故由 `tests/persist.rs` 的 `every_level_round_trips_through_its_encoding`
    /// 遍历 [`Level::ALL`] 兜住。
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "system_safety" => Self::SystemSafety,
            "explicit_current" => Self::ExplicitCurrent,
            "user_persistent" => Self::UserPersistent,
            "project" => Self::Project,
            "runtime_default" => Self::RuntimeDefault,
            "model_suggestion" => Self::ModelSuggestion,
            _ => return None,
        })
    }
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

impl Decision {
    /// 全部三个决策。供全量遍历的用例使用（与 [`Level::ALL`] 同形）。
    ///
    /// 完整性与数目由 `tests/persist.rs` 的
    /// `every_decision_round_trips_through_its_encoding` 把关。
    pub const ALL: [Decision; 3] = [
        Decision::Allow,
        Decision::Deny,
        Decision::RequireApproval,
    ];

    /// `decision` 列的落库编码：小写、多词以 `_` 连接（设计下篇第 8 节）。
    ///
    /// match 穷尽且无通配臂：加决策时本函数编译不过，编码不会漏分支。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
            Self::RequireApproval => "require_approval",
        }
    }

    /// [`Decision::as_str`] 的**严格逆**；表外字符串一律 `None`，不取默认决策
    /// ——取默认会让裁决判错。解码侧的完整性由 `tests/persist.rs` 的
    /// `every_decision_round_trips_through_its_encoding` 遍历 [`Decision::ALL`] 兜住。
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "allow" => Self::Allow,
            "deny" => Self::Deny,
            "require_approval" => Self::RequireApproval,
            _ => return None,
        })
    }
}

/// 规则的来源层（设计下篇第 5.1 节）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    User,
    Project,
}

impl Scope {
    /// 全部两个作用域。供全量遍历的用例使用（与 [`Level::ALL`] 同形）。
    ///
    /// 完整性与数目由 `tests/persist.rs` 的
    /// `every_scope_round_trips_through_its_encoding` 把关。
    pub const ALL: [Scope; 2] = [Scope::User, Scope::Project];

    /// `scope` 列的落库编码：小写、多词以 `_` 连接（设计下篇第 8 节）。
    ///
    /// match 穷尽且无通配臂：加作用域时本函数编译不过，编码不会漏分支。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Project => "project",
        }
    }

    /// [`Scope::as_str`] 的**严格逆**；表外字符串一律 `None`。解码侧的完整性由
    /// `tests/persist.rs` 的 `every_scope_round_trips_through_its_encoding`
    /// 遍历 [`Scope::ALL`] 兜住。
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "user" => Self::User,
            "project" => Self::Project,
            _ => return None,
        })
    }
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

    /// 写回 [`Value`]，供落库（`crate::persist` 的 `condition` 列）。
    ///
    /// **与 [`Condition::parse`] 严格互逆**：对任何 `parse` 得出来的条件，
    /// `Condition::parse(&c.to_json()) == Ok(c)`。由 `tests/condition.rs` 的
    /// `every_condition_shape_round_trips_through_to_json` 遍历全部形状钉住。
    ///
    /// 形状一律取设计下篇第 5.1 节的合取形式（`{"all": [...]}`）。**故写回的是
    /// 归一化后的形状**：裸谓词输入经 `parse` 已是一元合取，写回不会还原成裸谓词。
    /// 这不是缺陷——两种写法本就同义（`a_bare_predicate_equals_a_one_term_conjunction`），
    /// 故比较两处条件时要比 [`Condition`] 的相等，不比 JSON 字节。
    ///
    /// 枚举取值经枚举自己的 `as_str` 写出，与 [`Condition::parse`] 的取值解析同源
    /// （见 `parse_fact_value` 的说明），本 crate 不自建第二张字符串表。
    pub fn to_json(&self) -> Value {
        let mut map = serde_json::Map::new();
        map.insert(
            "all".to_owned(),
            Value::Array(self.0.iter().map(Predicate::to_json).collect()),
        );
        Value::Object(map)
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

    /// [`Predicate::parse`] 的逆：写出 `fact` 与恰一个比较符的键值对。
    ///
    /// 比较符的键名取 [`Op::name`]，与解析侧的 [`Op::parse`] 同源。
    fn to_json(&self) -> Value {
        let mut map = serde_json::Map::new();
        map.insert("fact".to_owned(), Value::String(self.fact.name().to_owned()));
        match &self.matcher {
            Matcher::Eq(value) => {
                map.insert(Op::Eq.name().to_owned(), value.to_json());
            }
            Matcher::Gte(value) => {
                map.insert(Op::Gte.name().to_owned(), value.to_json());
            }
            Matcher::In(values) => {
                map.insert(
                    Op::In.name().to_owned(),
                    Value::Array(values.iter().map(FactValue::to_json).collect()),
                );
            }
        }
        Value::Object(map)
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
    ///
    /// 列表为空时恒不成立，且解析期**不拒**它：设计下篇第 5.1 节列的「不符」只有
    /// 三类（事实名不命中、比较符不在封闭集合、取值类型不符），空列表不占任何一类，
    /// 多立一类拒绝规则就是发明规范。但它确实属于该节要防的「永不匹配」形态，
    /// 所以不靠注释保证它可见——由 `tests/condition.rs` 的
    /// `an_empty_in_list_never_matches` 明文钉住：它不静默。
    In(Vec<FactValue>),
    /// 数值不小于下界。解析期已保证它只落在 `duration_ms` 上，故其值必是数值。
    Gte(FactValue),
}

/// 一个比较符。封闭集合，与设计下篇第 5.1 节一致。
///
/// 三个比较符各有自己的适用规则，都在解析期判定：
/// - `eq` / `in`：适用于全部事实，取值类型须与该事实相符；`in` 的空列表被接受
///   （见 [`Matcher::In`]）；
/// - `gte`：只适用于 `duration_ms`，对别的事实报
///   [`PolicyError::OperatorNotApplicable`]——给字符串或布尔事实排序没有定义。
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
///
/// **可观察性分两类**，逐条标明如下——设计下篇第 5.3 节的「事实不在上下文中即该条
/// 不成立」只适用于第一类：
///
/// | 事实 | 观察性 | 上下文缺该字段时 |
/// |---|---|---|
/// | `privacy_class` | 缺省即不成立 | 该条不成立 |
/// | `effect_type` | 缺省即不成立 | 该条不成立 |
/// | `task_class` | 缺省即不成立 | 该条不成立 |
/// | `duration_ms` | 缺省即不成立 | 该条不成立 |
/// | `explicit_current` | **总可观察** | 取值为 `false`（`--approve` 未给） |
///
/// 非对称的理由：其余四个事实是「驱动可能不知道」，缺省等于未知，取更严的一侧
/// （不成立）；而 `--approve` 给没给**总是已知**的，`Option` 的 `None` 在这里意为
/// 「未给」而非「未知」。若把它也当缺省，`{"fact":"explicit_current","eq":false}`
/// 将永不成立：一条第 5 级 `Deny` 配这个条件会静默失效，而一条更高层的 `Allow`
/// 就会获胜——那是 fail-open，与第 5.3 节的 fail-closed 相反。完整论证见
/// [`PolicyContext::explicit_current`]。
///
/// 两个方向各有用例：`tests/condition.rs` 的
/// `explicit_current_is_an_always_observable_boolean_fact` 断言 `eq: false` 在未给
/// `--approve` 时**匹配**、在给了时**不匹配**。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fact {
    /// 总可观察：恒有取值，见本枚举的说明。
    ExplicitCurrent,
    /// 缺省即不成立。
    PrivacyClass,
    /// 缺省即不成立。
    EffectType,
    /// 缺省即不成立。
    TaskClass,
    /// 缺省即不成立。
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
    /// [`parse_fact_value`] 的逆：按事实的类型写出一个字面量。
    ///
    /// 两个枚举事实经枚举自己的 `as_str` 写出——与解析侧的 `parse` 是同一对函数，
    /// 故往返成立的前提（编码互逆）由那些 crate 自己的用例给出，不在这里另立一份。
    fn to_json(&self) -> Value {
        match self {
            Self::Bool(value) => Value::Bool(*value),
            Self::Privacy(class) => Value::String(class.as_str().to_owned()),
            Self::Effect(effect_type) => Value::String(effect_type.as_str().to_owned()),
            Self::Text(text) => Value::String(text.clone()),
            Self::Number(number) => Value::Number((*number).into()),
        }
    }

    /// 数值是否不小于 `bound`。任一侧不是数值即取 `false`：解析期已拒绝
    /// 「`gte` 用于非数值事实」，故该分支不出现；万一出现，判为不成立是更严的一侧。
    fn reaches(&self, bound: &Self) -> bool {
        matches!((self, bound), (Self::Number(n), Self::Number(b)) if n >= b)
    }
}

/// 按事实的类型解析一个字面量。类型不符即返回 `Err`（设计下篇第 5.1 节）。
///
/// 两个枚举事实（`privacy_class`、`effect_type`）的取值经枚举自己的 `parse` 解码，
/// 即与落库编码、驱动 `--effect` 解析**同一个来源**（`PrivacyClass::parse` /
/// `EffectType::parse`）。故本 crate 不自建第二张字符串表：写错的条件在这里报
/// `UnknownValue` 而不是静默不匹配，写错的编码在那些 crate 自己的用例里报红。
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
            PrivacyClass::parse(name)
                .map(FactValue::Privacy)
                .ok_or_else(|| PolicyError::UnknownValue {
                    fact: fact.name(),
                    value: name.to_string(),
                })
        }
        Fact::EffectType => {
            let name = value.as_str().ok_or_else(|| mismatch("字符串"))?;
            EffectType::parse(name)
                .map(FactValue::Effect)
                .ok_or_else(|| PolicyError::UnknownValue {
                    fact: fact.name(),
                    value: name.to_string(),
                })
        }
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
