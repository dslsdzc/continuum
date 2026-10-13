//! §224 的 Contract 数据模型与 §7.2 的 `verification_requirement` 禁读。
//!
//! 计划「文件结构」一节把 `TaskContract、Requirement、RequirementClass、
//! VerificationRequirement` 归在 `src/contract.rs`
//! （`docs/superpowers/plans/2026-10-06-p4-semantic-layer.md:306`）；本文件与
//! `persist.rs` 的分工照 Task 8 的 `intent.rs` / `persist.rs` 与 Task 10 的
//! `spec.rs` / `persist.rs`：**本文件是 §224 的词汇与记录体**，`persist.rs` 是
//! `contract` / `contract_requirement` 两张表与 I/O（[`crate::save_contract`] /
//! [`crate::load_contract`]）。
//!
//! # `verification_requirement`：OPEN-001 未决期间只存不判（§7.2）
//!
//! 《工程》§2.5 明写该字段**可以存储**、但在 OPEN-001 解决前**无法判定其是否被满足**
//! （「这不阻断本层建设，阻断的是第 8 层的验证收口」）。故 [`VerificationRequirement`]
//! 是**结构性禁读**的落点：公开方法集只有 `as_str` / `from_str`——没有 `is_satisfied` /
//! `check` / `evaluate` 这类方法。OPEN-001 的三个可选方向（a/b/c）**一个都不选**：
//! 补任何一个都是发明判据。照片机制见类型文档（trybuild 三份反例 ＋ 一份正控制 ＋
//! `src/` 的出现点清单）。
//!
//! # 版本化（§226）：落库主键 `(id, version)`，无 UPDATE 路径
//!
//! §226：用户改要求时 MUST 创建 `Contract vN+1`、**MUST NOT 原地修改**。
//! [`TaskContract::version`] 由调用方给（「新版本」是一个显式动作），
//! [`crate::save_contract`] **只 INSERT**——同 `(id, version)` 的二次写被复合主键拒绝
//! （照片：`tests/contract.rs` 的 `a_second_write_of_the_same_id_and_version_hits_the_primary_key`）。
//! **本文件不含 I/O**（表与读写都在 `persist.rs`，同 Task 8／10 的分工）。

use continuum_canonical::Opaque;

use crate::ids::{ContractId, IntentId, RequirementId};

/// §224 的四个 class，**封闭四值**。
///
/// §224（`docs/spec/05-normative.md:209-254`）给出四个词：`REQUIRED` / `PREFERRED` /
/// `FLEXIBLE` / `UNSPECIFIED`。**四值各一条照片**（`tests/contract.rs` 的
/// `every_requirement_class_has_a_photo`：四值各写一次、读回断言是**哪一枚**，
/// 落库字面量逐枚 raw 读列）。
///
/// # 编码（小写、多词接 `_`）与 `as_str` / `parse` 同址
///
/// [`RequirementClass::as_str`] **无通配臂**：加第五枚即**编译不过**（编码逐枚有写的
/// 编译期照片）；解码走带通配臂的 [`RequirementClass::parse`]（输入是开放集），
/// 表外取值由 `persist.rs` 的解码处转成 [`crate::ContractError::UnknownClass`]
/// （带库里那一个原始串、不取默认值）。
///
/// # 为什么是 `pub(crate)` 的编码对
///
/// 可观察的用处只有两处：`contract_requirement` 表的 `class` 列（落库走
/// [`crate::save_contract`]、读回走 [`crate::load_contract`]），以及本 crate 内的判定
/// （Task 14 的 Constraint Validator 判 `REQUIRED`——同 crate）。故先按 `pub(crate)` 收着，
/// 加宽是一字之改；同 [`crate::IntentKind::as_str`] 的收法。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequirementClass {
    /// §224 的 `REQUIRED`。
    Required,
    /// §224 的 `PREFERRED`。
    Preferred,
    /// §224 的 `FLEXIBLE`。
    Flexible,
    /// §224 的 `UNSPECIFIED`。
    Unspecified,
}

impl RequirementClass {
    /// 四枚之一 → 落库字面量。**全函数**（闭集四枚，没有第五种输入）。**无通配臂**。
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            RequirementClass::Required => "required",
            RequirementClass::Preferred => "preferred",
            RequirementClass::Flexible => "flexible",
            RequirementClass::Unspecified => "unspecified",
        }
    }

    /// [`RequirementClass::as_str`] 的逆。**定义域是所有串**，认不出就是 `None`。
    ///
    /// 形状照 [`crate::IntentKind::parse`]：返回 `Option` 而不返回一枚「未知」变体——
    /// 本函数没有失败原因可给（它不读库、不写库）；把认不出的串猜成某一枚会让库里
    /// 一个被改坏的值静默变成一条有 class 的记录。`None` 转成哪个错误由调用方的解码处
    /// 决定（`continuum-semantics` 的 `persist.rs` 转成 [`crate::ContractError::UnknownClass`]，
    /// 带原始取值）。**带通配臂是刻意的**（与 [`RequirementClass::as_str`] 的无通配相反）：
    /// 这一边的输入是**开放集**（任意串），逐枚列出永远列不完。
    pub(crate) fn parse(s: &str) -> Option<RequirementClass> {
        match s {
            "required" => Some(RequirementClass::Required),
            "preferred" => Some(RequirementClass::Preferred),
            "flexible" => Some(RequirementClass::Flexible),
            "unspecified" => Some(RequirementClass::Unspecified),
            _ => None,
        }
    }
}

/// §224 的 `verification_requirement`。**只有 `as_str` / `from_str`**
/// ——没有 `is_satisfied` / `check` / `evaluate`（OPEN-001 未决期间只存不判，§7.2）。
///
/// # 为什么是 newtype（而不是裸 `String`）
///
/// 它是 `Requirement` 的一个**有名字的字段**：合并进 `source` / `expression` 的裸
/// `String` 会让「把验证要求填进来源」在类型上可写（同 [`crate::Priority`] 与
/// [`crate::spec::Goal`] 一类的处置）。且「公开面只有两个方法」这句话需要一个
/// **类型**来承载——`String` 的公开面是 `str` 的全部方法。
///
/// # 公开面清单（逐条）
///
/// - [`VerificationRequirement::as_str`]——取里子串，不改写、不裁剪；
/// - [`VerificationRequirement::from_str`]——从任意串构造，无校验；
/// - `Debug` / `Clone` 两个派生（**非语义能力**：断言失败时要打得出来、
///   要能装进 [`TaskContract`] 并随值传递）。
///
/// **没有第三个语义方法**——这是「结构性禁读」的落点。照片：
///
/// 1. `tests/compile_fail/verification_requirement_has_no_{is_satisfied,check,evaluate}.rs`
///    三份反例（`.stderr` 实跑取值、逐字比对，`tests/type_level.rs` 驱动）；
/// 2. `tests/compile_pass/verification_requirement_as_str_is_usable.rs` 正控制
///    （两个公开方法确实可用——禁读是「没有判定方法」，不是「没有公开面」）；
/// 3. `tests/contract.rs` 的
///    `the_layer_has_no_function_that_takes_a_verification_requirement_as_a_judging_input`
///    （`src/` 的出现点逐条登记：没有以它为输入做判定的函数）。
#[derive(Debug, Clone)]
pub struct VerificationRequirement(String);

impl VerificationRequirement {
    /// 取里子串。**不改写、不裁剪**。
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 从任意串构造。**无校验**——本类型没有「合法取值」这个概念
    /// （OPEN-001 未给取值域；本层只保存）。
    ///
    /// 名字用 `from_str` 而没有实现 `std::str::FromStr`：后者要求一个 `Err` 关联类型，
    /// 而本类型**不失败**（同 `EntityId::from_str` / [`crate::Priority::from_str`]）。
    pub fn from_str(s: &str) -> Self {
        VerificationRequirement(s.to_owned())
    }
}

/// §224 的 `Requirement`：五个字段**逐字照 §224**（`id` / `class` / `expression` /
/// `source` / `verification_requirement`）。
///
/// # `expression` / `source` 取 [`Opaque`]（§7.1）
///
/// §224 未给表达式的语法、`source` 也未给取值域——本层**不解析、不发明**，
/// 只把串存下来。「不改写、不裁剪」「无校验」的照片在 `tests/contract.rs` 的
/// 编码往返用例里（经 `contract_requirement` 表逐字节进出）。
///
/// # 字段是 `pub` 的
///
/// 同 [`crate::Intent`] / [`crate::spec::SpecDraft`]：本类型是 §224 逐字给出的记录，
/// 本层没有比 §224 更强的约束可挂，锁起来只会让调用方多写一个构造函数，
/// 而不变量一条也不会多。
#[derive(Debug, Clone)]
pub struct Requirement {
    /// §224 的 `id`。
    pub id: RequirementId,
    /// §224 的 `class`（四值封闭，见 [`RequirementClass`]）。
    pub class: RequirementClass,
    /// §224 的 `expression`。**§224 未给表达式的语法**，本层不解析它（§7.1）。
    pub expression: Opaque,
    /// §224 的 `source`。**未给取值域**。
    pub source: Opaque,
    /// §224 的 `verification_requirement`（只存不判，见 [`VerificationRequirement`]）。
    pub verification_requirement: VerificationRequirement,
}

/// §224 的 `TaskContract`。
///
/// # 字段清单（§224 的 12 个规范字段 → 本结构体的 10 个成员）
///
/// | §224 字段 | 本结构体 | 说明 |
/// |---|---|---|
/// | `id` | [`ContractId`] | 落库主键的第一半 |
/// | `version` | `u32` | **由调用方给**：「新版本」是显式动作（§226），[`crate::save_contract`] 不算版本 |
/// | `intent_id` | [`IntentId`] | 不建外键（同 `intent.contract_id` 的处置：跨记录对应关系由写路径的调用方保证） |
/// | `requirements[]` | `Vec<Requirement>` | 落 `contract_requirement` 表 |
/// | `budget` / `authority_limit` / `data_policy` / `model_policy` / `node_policy` / `acceptance_predicate` | [`Opaque`] | **规范只给名字**，一律不透明值（§7.1） |
/// | `preferences[]` / `prohibitions[]` | **未建**（见下） | |
///
/// **差额申报**：§224 逐字列了 `preferences[]` / `prohibitions[]` 两个列表字段，
/// 设计 §7.1 与任务简报均**未处置**它们（元素结构、落库形态、消费者三处都没有），
/// 故本 task 按「范围与设计一致、不发明判据」**未建**——要建的话先要有它们的元素
/// 结构与这两列的去处。收件人：协调者／规范维护者。
///
/// # `budget` 与 §12 的预算**不是同一个东西**（§16 第 11 条）
///
/// 本字段是 Contract 里的**声明字段**（只保存）；§333 的 `Budget` 是**记账对象**。
/// 两者的关系规范没给——缺的是「Contract 的 `budget` 怎么变成树上的一次分配」这一步。
/// 本层不建那个桥，也不把它当 `continuum-budget` 的输入（本 crate 连那条依赖边都没有，
/// §11.5 的第 1 条守卫）。
///
/// # 版本化（§226）
///
/// 落库主键 `(id, version)`，**无 UPDATE 路径**：同 `(id, version)` 的二次写被主键拒绝
/// （照片：`tests/contract.rs` 的
/// `a_second_write_of_the_same_id_and_version_hits_the_primary_key`）。
/// 每个新版本在同一事务内写一条 `AuditKind::ContractChanges`（§5.4，在
/// [`crate::save_contract`] 里）。
///
/// # 字段是 `pub` 的（同 [`crate::Intent`]）
///
/// 版本那条规则挂在**写路径**上（[`crate::save_contract`] 只 INSERT、撞键即拒），
/// 不是本结构体的不变量；本层没有比 §224 更强的约束可挂。字段清单**逐项**的
/// 照片是 `tests/contract.rs` 的 `the_other_fields_of_224_are_opaque`
/// （穷尽解构、**不带 `..`**——加／少／改名即编译不过）。
#[derive(Debug, Clone)]
pub struct TaskContract {
    /// §224 的 `id`。落库主键的第一半。
    pub id: ContractId,
    /// §224 的 `version`。落库主键的第二半；**由调用方给**（见类型文档）。
    pub version: u32,
    /// §224 的 `intent_id`。
    pub intent_id: IntentId,
    /// §224 的 `requirements[]`（读回按写入顺序，见 [`crate::load_contract`]）。
    pub requirements: Vec<Requirement>,
    /// §224 的 `budget`（不透明；见类型文档：与 §12 的预算不是同一个东西）。
    pub budget: Opaque,
    /// §224 的 `authority_limit`（不透明）。
    pub authority_limit: Opaque,
    /// §224 的 `data_policy`（不透明）。
    pub data_policy: Opaque,
    /// §224 的 `model_policy`（不透明）。
    pub model_policy: Opaque,
    /// §224 的 `node_policy`（不透明）。
    pub node_policy: Opaque,
    /// §224 的 `acceptance_predicate`（不透明）。**判定属第 8 层**
    /// （同 `Intent.completion_predicate`：本层只存，见设计 §16 第 9 条）。
    pub acceptance_predicate: Opaque,
}
