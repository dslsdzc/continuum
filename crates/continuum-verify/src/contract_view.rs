//! §6.1 的 `ContractView` 一侧的类型：Contract 的视图（[`ContractView`]）、视图里的
//! 一条 requirement（[`ContractRequirement`]）、以及本块要读的两项身份承载
//! （[`RequirementId`] 与 [`RequirementClass`]）。Task 2 落 [`RequirementId`]——
//! 它是 `EvidenceSubject::Requirement` 的载荷；Task 5 追加其余三枚。

/// §224 的 `Requirement.id`（本块的承载；设计 §4.3.1 的订正段）。
///
/// # 与 P4 的 `continuum_semantics::RequirementId` 是两个类型
///
/// 同名、不同型，不共享 `From` / `Deref`，`rustc` 不会把一枚当另一枚。本块**不登记**
/// 指向 `continuum-semantics` 的边（设计 §6.1：以值承载是无条件的），故这枚第二份是
/// 该处置的代价；**两者之间的对应由装配方维持**——装配方从 P4 的 Contract 数据里取出
/// `Requirement.id`，构造本块这一枚，再填进 `EvidenceSubject::Requirement`。
///
/// # 派生 `PartialEq`
///
/// §6.3 第 1 条的判据是 `e.subject == EvidenceSubject::Requirement(r)`，归属判定按它
/// 比较；这与 P4 那枚（不派生 `PartialEq`、断言一律经 `as_str` 比较）的处置不同——
/// 那边没有按它比较的判定，本边有。`PartialEq` 不是构造路径（本类型的构造只有
/// [`RequirementId::from_str`]）。
#[derive(Debug, Clone, PartialEq)]
pub struct RequirementId(String);

impl RequirementId {
    /// 取里子串。**不改写、不裁剪**。
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 从任意串构造。**无校验**——§224 只给了字段名，没给取值域。
    ///
    /// 名字用 `from_str` 而没有实现 `std::str::FromStr`：后者要求一个 `Err` 关联类型，
    /// 而本类型**不失败**（同 P4 的 id 类类型与本 crate 的 `EvidenceId`）。
    pub fn from_str(s: &str) -> Self {
        RequirementId(s.to_owned())
    }
}

/// §224 的 `class` 四值（`REQUIRED` / `PREFERRED` / `FLEXIBLE` / `UNSPECIFIED`）。
/// 本块只做 `match`，**不在本块里判 `class` 的语义**（§6.1）。
///
/// # 与 P4 的 `continuum_semantics::RequirementClass` 是两个类型
///
/// 同 [`RequirementId`] 的处境（同名、不同型；设计 §4.3.1 的订正段）：本块**不登记**
/// 指向 `continuum-semantics` 的边，而设计对 `ContractView.class` 的**字段类型**全文
/// 没有写出（`:109`／`:463`／`:1039`，设计自己记为 §14 第 1 条 (ii) 与 §15 第 19 条），
/// 故本块自定这一枚（申报见计划的 `## 遗留` 第二节）。**跨层只转值、不转序**：P4 那枚
/// 带声明序（`UNSPECIFIED < FLEXIBLE < PREFERRED < REQUIRED`），本块的**没有**——
/// 本块不给 `Ord`（判定只做相等比较与 `match`，没有一处按序比较）。
///
/// # 编码（小写、多词接 `_`）与 `as_str` / `parse` 同址
///
/// 四值的编码取 §224 四个规范名的小写形，与 P4 那枚的编码逐字相同。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequirementClass {
    /// §224 的 `REQUIRED`——**四值里唯一受 §190 证据约束的那一值**（§6.1）。
    Required,
    /// §224 的 `PREFERRED`。
    Preferred,
    /// §224 的 `FLEXIBLE`。
    Flexible,
    /// §224 的 `UNSPECIFIED`。
    Unspecified,
}

impl RequirementClass {
    /// §224 的四个 class，逐值照录。**本常量带数目字面量**，与
    /// [`EvidenceType::ALL`](crate::EvidenceType::ALL) 同形。
    ///
    /// 顺序照 §224 的四个规范名；完整性与数目由 `tests/coverage.rs` 的
    /// `the_four_class_values_each_round_trip` 把关。
    pub const ALL: [RequirementClass; 4] = [
        RequirementClass::Required,
        RequirementClass::Preferred,
        RequirementClass::Flexible,
        RequirementClass::Unspecified,
    ];

    /// 四值的字符串名：小写、多词以 `_` 连接。
    ///
    /// **本函数是该编码唯一的产生点**。match 穷尽且无通配臂：加臂时本函数编译不过，
    /// 编码不会漏分支。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Required => "required",
            Self::Preferred => "preferred",
            Self::Flexible => "flexible",
            Self::Unspecified => "unspecified",
        }
    }

    /// [`RequirementClass::as_str`] 的**严格逆**：对 [`RequirementClass::ALL`] 里的每臂
    /// 都有 `RequirementClass::parse(t.as_str()) == Some(t)`，表外字符串一律 `None`
    /// （理由同 [`EvidenceType::parse`](crate::EvidenceType::parse)：取默认会把一份别的
    /// 东西当成本值处理）。
    ///
    /// 解码侧没有穷尽 match 的保护（来源是 `&str` 而非枚举），故由
    /// `tests/coverage.rs` 的 `the_four_class_values_each_round_trip` 遍历
    /// [`RequirementClass::ALL`] 兜住。
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "required" => Self::Required,
            "preferred" => Self::Preferred,
            "flexible" => Self::Flexible,
            "unspecified" => Self::Unspecified,
            _ => return None,
        })
    }
}

/// 视图里的一条 requirement：只带本块要读的两项（`id` 与 `class`），
/// 加 `verification_requirement` 一项**只读不判**（P4 设计 §7.2 的「只存不判」在本块延续）。
///
/// # 字段为 `pub` 与本类型是自定的
///
/// 设计给了「读哪几个字段」这条读法（§6.1）、**没给承载类型**（申报见计划的
/// `## 遗留` 第二节），本块取一枚三字段结构体。字段开 `pub`：这是**值传递的读数口**，
/// 没有「唯一产生点」之类的形状约束（与 [`Evidence`](crate::Evidence) 的处置不同）。
#[derive(Debug, Clone, PartialEq)]
pub struct ContractRequirement {
    /// §224 的 `Requirement.id`——本块承载的那一枚（同 [`RequirementId`] 的处置）。
    pub id: RequirementId,
    /// §224 的 `class`。判定只认 [`RequirementClass::Required`]（§6.1）。
    pub class: RequirementClass,
    /// §224 的 `verification_requirement`。**只读不判**：本块不解析它
    /// （P4 设计 §7.2 的「只存不判」延续为「只读不判」）。
    pub verification_requirement: Option<String>,
}

/// §6.1 的 **Contract 视图**：本块对 Contract 只读 `requirements[].{id, class}` 与
/// `verification_requirement`，故取一个视图、其字段以**值**传入——**不给
/// `continuum-semantics` 登记边，也不造第二个 `TaskContract`**（设计 §6.1、§2.1）。
///
/// # 字段类型是本计划自定的
///
/// 设计全文**没有写出**本类型的字段类型（`:109`／`:463`／`:1039`），本计划取
/// [`ContractRequirement`] 的列表（申报见计划的 `## 遗留` 第二节）。
#[derive(Debug, Clone, PartialEq)]
pub struct ContractView {
    /// Contract 的 `requirements`——逐项照传；顺序即
    /// [`coverage`](crate::coverage::coverage) 返回行的顺序。
    pub requirements: Vec<ContractRequirement>,
}
