//! §94 的「privacy × trust class」表与它的构造期判据（设计 §5.4 通道 (b)）。
//!
//! **本模块是 `continuum-artifact` 这条 crate 边在本仓的第一个使用点**
//! （设计 §7.2 给本 crate 定死的唯一一条边；`ALLOWED` 条目由 Task 1 登记）。
//! 取用的是**已存在**的 [`PrivacyClass`]（§243 的五档）——本模块一个类型都不新建。
//! 故 `crates/continuum-runtime/tests/dependency_direction.rs` 的双向断言
//! **从本 task 起第一次真的被这条边用上**：登记在前、使用在后，
//! 两者今天才对得上号。
//!
//! 本模块今天只有表与查询面；`place` 本身（硬闸门、`spec_floor`、`strictest`、
//! 排序）由 Task 5／Task 6 落在本模块，那时本文件的模块文档要重写。
//!
//! 本 task 的判据面只取 [`PrivacyClass`] 一个既有的取值类型：
//! `Artifact`（作为 `PlacementRequest` 的字段）是 Task 5 才进本模块的。

use continuum_artifact::PrivacyClass;

use crate::error::RulesError;

/// 一档隐私等级允许被放置在什么样的节点上。
///
/// **取值域里没有「`LocalOnly` 可以上任何节点」这一项**——见 `spec_floor`（Task 5）：
/// 那一档由规范钉死（设计 §5.3、§5.4 的 I2 段），
/// 若在这里给它一枚「不受限」的取值，闸门就多了一条可被调用方走通的路。
///
/// 两枚都是**本设计定的形状**，不是规范给的取值域（同 [`crate::NodeTrust`] 的处置）：
/// §94（`docs/spec/02-positioning.md:1054-1072`）只说 Scheduler 按
/// 「Artifact privacy × Node trust class」决定是否允许传输，没有给这张表的值域。
///
/// `Copy` 与 `Eq` 是访问器 [`PlacementRules::rule_for`] 的返回面要的；
/// 今天没有比较大小的消费方，故**不派生** `Ord`（同 [`crate::NodeClass`]）。
/// `Debug` 是断言红时读值要的。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferRule {
    /// 只允许放在 `class == Personal` **且** `trust == TrustedPersonal` 的节点上。
    TrustedPersonalOnly,
    /// 可以放在任何已登记的节点上。
    AnyNode,
}

/// §94 的「privacy × trust class」表。**值由调用方给，本设计一个都不填。**
///
/// # 必填、无默认
///
/// **不派生 `Default`**（纪律：必填、无默认）：一个默认表就是 fail-open 的入口
/// ——它会让「忘了传表」这件事在一张全身是「放行」的表上静默通过。
/// 构造的唯一入口是 [`PlacementRules::try_new`]，它**只接受覆盖全档的表**。
///
/// **不派生任何东西**（连 `Debug` 也不）：字段私有、今天零消费方
/// ——与 D 的 `BudgetView` 同一种处置。
/// **这条有可观察的后果**：用例里 `try_new` 的 `Ok` 分支不能 `{:?}` 打印它，
/// 故 `tests/rules.rs` 的 `Ok(_)` 臂只报「Ok」两字、不报内部值。
///
/// # `LocalOnly` 那一格是装饰性的，但 `try_new` 仍强制它存在
///
/// 设计 §5.4 的 I2 段（纪律：绝对措辞须有用例）：Task 5 的
/// `strictest(rules.rule_for(LocalOnly), spec_floor(LocalOnly))` 里右侧恒为
/// [`TransferRule::TrustedPersonalOnly`]，它在取更严者时永远被吞掉。
/// 故**「五格都有意义」是错的读法**——第一格是给 [`PrivacyClass::ALL`]
/// 的完整性用的，不是给判据用的。
///
/// **它是接口上的坑，不是安全问题**：设计 §5.4 的 I2 段把它写成「一个以为把
/// `LocalOnly` 填成 `AnyNode` 就放开了的调用方会发现放不开」，
/// 并把这条判断挂到设计 §9 表里标题为「**闸门不可被策略放宽**」的那一格上
/// （那里的话是：五档全写 `AnyNode` 时 `LocalOnly` 的制品仍只落在
/// `TrustedPersonalOnly` 那一类节点上，且**逐档遍历**）。
///
/// **那张照片今天还不存在**——它属于 Task 5（`place` 与 `spec_floor`）。
/// 本节到「§5.3 的 `strictest` 右臂恒为 `TrustedPersonalOnly`、
/// 故取更严者时这一格被吞掉」为止是**设计文本可核的**（§5.3、§5.4 的 I2 段，
/// 两处的原句都在设计文件里）；**再往后（「填错这一格的代价上限是调用方白填」）
/// 要等 `place` 落地才可核，本节不写。**
///
/// # 覆盖判据的来源
///
/// [`PlacementRules::try_new`] 逐档比对的来源**逐字是** [`PrivacyClass::ALL`]，
/// **不是**本模块手写的一张五档清单：一张手抄清单在规范加第六档时**不会失败**，
/// 而 `ALL` 会（设计 §5.4）。**即使今天两者不可区分，实现的取法也照此写。**
///
/// **这一条今天没有运行期照片**，据实记：手抄清单与 `ALL` 逐项相同，
/// 是**等价变异体**，只有加第六档才能区分。
/// `tests/rules.rs` 的 `the_coverage_criterion_reads_the_five_levels_from_the_type`
/// 钉的是**输入侧的事实**（`ALL` 是五枚、两两不同），**它拍不到本段的取法**；
/// 该用例的注释里写着这条射程。
pub struct PlacementRules {
    /// 键是 [`PrivacyClass`]，**不是** `HashMap`：
    ///
    /// 1. 五个键的规模下，线性查找与哈希查找没有值得换的差别；
    /// 2. `PrivacyClass` 的派生面今天不含 `Ord`，有序 `Vec` 也无法用来保证遍历序，
    ///    故这里不假装有顺序契约——**查询面只有 [`PlacementRules::rule_for`] 一个**。
    ///
    /// 字段私有：构造期的不变式（覆盖全档、无重复）由 [`PlacementRules::try_new`]
    /// 单独维持，没有第二条写入口。
    entries: Vec<(PrivacyClass, TransferRule)>,
}

impl PlacementRules {
    /// 必须覆盖 [`PrivacyClass::ALL`] 的每一档，**多一条、少一条、重复一条都是 `Err`**。
    ///
    /// 两条判据**顺序**是承重的：先查覆盖、再查重复。
    /// 一张「四条不重复」的表两条判据都能命中，先报哪一条决定了
    /// 调用方看到的诊断——本实现报 [`RulesError::MissingLevel`]，
    /// 因为「少表态了一档」比「某档被说了两次」更接近 fail-closed 要挡的形状。
    ///
    /// **「多一条」没有独立的错误变体**：[`PrivacyClass`] 只有五枚，
    /// 故一张条数超过五的表必然在某一档上重复，由重复判据兜住；
    /// 条数少于五且不重复的表由覆盖判据兜住（见 [`RulesError`] 的文档）。
    pub fn try_new(entries: Vec<(PrivacyClass, TransferRule)>) -> Result<Self, RulesError> {
        // 覆盖判据的来源是 PrivacyClass::ALL（见类型文档「覆盖判据的来源」一段）。
        // 本循环逐档问「表里有没有它」，故缺哪一档就报哪一档。
        for &level in PrivacyClass::ALL.iter() {
            if !entries.iter().any(|(l, _)| *l == level) {
                return Err(RulesError::MissingLevel { level });
            }
        }

        // 重复判据。扫描顺序是**表的顺序**（不是 ALL 的顺序），
        // 故报出来的 `level` 是「第二次出现的那一档」——这正是调用方要看的那个。
        let mut seen: Vec<PrivacyClass> = Vec::new();
        for &(level, _) in entries.iter() {
            if seen.contains(&level) {
                return Err(RulesError::DuplicateLevel { level });
            }
            seen.push(level);
        }

        Ok(Self { entries })
    }

    /// 构造期已保证覆盖，故这里不含 `Option`。查询落空时的返回值是
    /// [`TransferRule::TrustedPersonalOnly`] ——**不可达**，但若真到达，它是 fail-closed 的那一侧。
    ///
    /// # 「不可达」是逻辑蕴含，不是没测过
    ///
    /// 入参 `level` 的类型是 [`PrivacyClass`]，它的取值恰好是五枚，
    /// 而这五枚就是 [`PrivacyClass::ALL`] 的内容；[`PlacementRules::try_new`]
    /// 又保证每一枚都在 `entries` 里。
    /// 故「查不到」在构造出 `PlacementRules` 之后**在类型与不变式上都不成立**。
    ///
    /// **上面那句「`ALL` 的内容就是这五枚」的两个半句各有出处，出处不同**：
    ///
    /// - **枚数**（`ALL.len() == 5`）由 `continuum-artifact` 的
    ///   `tests/privacy_class.rs` 的 `every_privacy_class_round_trips_through_its_encoding`
    ///   把关（那里第一句就是 `assert_eq!(PrivacyClass::ALL.len(), 5, …)`）。
    /// - **两两不同**（`ALL` 里没有重复档）**不在那边**——那份用例只钉长度与往返编码，
    ///   一份「`Public` 写两遍、`Personal` 缺席」的 `ALL` 长度仍是 5、仍能往返。
    ///   这一半由**本 crate** 的 `tests/rules.rs` 的
    ///   `the_coverage_criterion_reads_the_five_levels_from_the_type` 钉住。
    ///   **故「完整性由 artifact 那边把关」是错的读法**，据实分写在这里。
    ///
    /// **故本分支没有、也不该有专门的用例**：写一条「构造一张合法表、再查一个查不到的档」
    /// 的用例，在今天的取值域里**构造不出来**；能构造出来的只有「绕过 `try_new` 的
    /// `PlacementRules`」，而那种值在本 crate 里没有产生点。
    /// 该分支留着是因为 `find` 的返回类型是 `Option`，而 `unwrap` 会把一个
    /// **不可达**的分支变成 panic——返回 fail-closed 的那一枚更便宜也更安全。
    pub fn rule_for(&self, level: PrivacyClass) -> TransferRule {
        match self.entries.iter().find(|(l, _)| *l == level) {
            Some((_, rule)) => *rule,
            None => TransferRule::TrustedPersonalOnly,
        }
    }
}
