//! 请求面与放置：§5.2 的 [`PlacementRequest`]／[`PlacementPolicy`]／[`place`]，
//! §5.3 的硬闸门（[`spec_floor`] ＋ [`strictest`]），
//! 以及 §5.4 (b) 的表 [`PlacementRules`]／[`TransferRule`] 与它的构造期判据。
//! 设计：`docs/superpowers/specs/2026-10-06-p3e-compute-placement-design.md`。
//!
//! **本模块是 `continuum-artifact` 这条 crate 边在本仓的第一个使用点**
//! （设计 §7.2 给本 crate 定死的唯一一条边；`ALLOWED` 条目由 Task 1 登记）。
//! 取用的是**已存在**的 [`PrivacyClass`]（§243 的五档）与 [`Artifact`]——本模块一个类型都不新建。
//! 故 `crates/continuum-runtime/tests/dependency_direction.rs` 的双向断言
//! **从 Task 4 起第一次真的被这条边用上**：登记在前、使用在后，两者今天才对得上号。
//!
//! # 走到哪为止
//!
//! [`place`] 今天只落**步骤 1–3**（判重 → 过闸门 → 判空）与**「取过滤后的第一枚」**；
//! **步骤 4（排序）与「兜底档」在 Task 6**。故今天的 `place` **不读**
//! [`PlacementPolicy::compare`]——这是**刻意的拆分**（各自的判据分开写清），
//! **不是 `place` 的最终形态**。由此还有一处可观察的后果：本 task 的每一条用例
//! 都构造成「过闸门后恰好剩一枚」或「次序无关」。
//!
//! [`BaselinePlacementPolicy`] 与它的 `compare` 一并落在本模块，是本 crate 对外交出的唯一一份
//! 策略实现——**它不对 §291 的任何一项因子作主张**（十一项里一项都算不出来，设计 §5.5／§5.6）。

use std::cmp::Ordering;

use continuum_artifact::{Artifact, PrivacyClass};

use crate::error::{PlacementError, RulesError};
use crate::node::{ComputeNode, NodeClass, NodeTrust};

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

/// 一次放置的请求。**纯数据，无 I/O**——与 D 的 `RoutingRequest` 同形（设计 §5.2）。
///
/// # 两个字段都是引用、都是必填（不是 `Option`）
///
/// 一个想忽略节点集或制品集的调用方，**必须先编出一个空切片**——
/// 那是一次**看得见的选择**，不是一次遗漏（同 D 把 `budget: BudgetView` 写成必填的理由）。
///
/// # 第三个字段（《工程》§4.3 的 Router 输出）**已按裁定删除**
///
/// 裁定出处：`docs/superpowers/2026-10-06-p3e-decisions.md` 第一节（**在 `docs/superpowers/` 下，
/// 不在 `specs/` 下**），取**形状甲**。**后果逐条**：
///
/// - **本 crate 对 `continuum-model-registry` 零依赖**（设计 §7.2）——故
///   `tests/placement.rs` 的 `a_placement_request_is_built_from_artifacts_and_nodes_alone`
///   在**类型上**就写不出 `rank`，那条用例只是「零读取」这件事的**下界**；
/// - **§4.3 那条边记为阻断**（不是「本层不接」，是**今天接不了**），进遗留、收件人按裁定原文是
///   **规范维护者 ＋ 复审者**（设计 §10 第 8 条）；
/// - **裁定的效力是有条件的**：依据是「**今天没有任何一行代码会读它**」，
///   **不是「那条边不存在」**。若将来放置真的开始读模型侧信息（例如节点能力与模型要求需比对）
///   ⇒ **那时加字段**，并**连同一个真会读它的用例一起加**（裁定不构成「永远不收」）。
///
/// # 为什么 `artifacts` 是 `&[Artifact]` 而不是一个新的 `(id, level)` 投影类型
///
/// §243 的输入就是制品自己的等级，而 [`Artifact`] 已经把它记在 `pub privacy_class` 上。
/// 另建一个投影类型是**同一份数据的第二个落点**。
/// **代价照实记**：本 crate 因此依赖 `continuum-artifact` 的 `Artifact` 与 `PrivacyClass`
/// 两个名字（设计 §7）。
pub struct PlacementRequest<'a> {
    /// §290 的 `required_artifacts` 那一面。§243 的隐私输入从这里读。
    pub artifacts: &'a [Artifact],
    /// §4.3 的「Compute Node 注册」。由 [`NodeRegistry::nodes()`](crate::NodeRegistry::nodes) 交出。
    pub nodes: &'a [ComputeNode],
}

/// §42 的那一条例，逐字落在 [`PrivacyClass::LocalOnly`] 上；**其余四档规范未给**，
/// 故下限取 [`TransferRule::AnyNode`]（**这不是「允许」，是「规范没有说不允许」**，设计 §5.4、§10 第 2 条）。
///
/// # 两层结构里的「规范侧下限」这一层
///
/// 本函数**不读策略**（签名里就没有策略）——**这是闸门不可被策略放宽的全部机制的一半**，
/// 另一半是 [`strictest`] 让 `TrustedPersonalOnly` 一侧吸收一切。
///
/// # `match` 穷尽、无通配臂
///
/// 加档时**本函数编译不过**，故下限不会漏分支。
/// **这条性质的照片不存在**（它是编译期性质），据实记——
/// **不得为拍它去改 [`PrivacyClass`]**（设计 §8 第 5 条）。
/// 同形先例是 `PrivacyClass::as_str` 的注释（`crates/continuum-artifact/src/artifact.rs`）。
///
/// **两臂的排布不是随手写的**：`LocalOnly` 单独一臂、其余四档合写一臂并**逐个点名**，
/// 于是「规范后来给那四档中的某一档填了规则」这件事**只能**通过改这一条 `match` 落地，
/// 不会在某个 `_` 里被静默吞掉。
fn spec_floor(level: PrivacyClass) -> TransferRule {
    match level {
        // §42：「Private photo: LOCAL_ONLY …… Cloud Node 即使性能最好，也不能被选择」
        // （`docs/spec/01-concepts.md:1646-1672`）
        PrivacyClass::LocalOnly => TransferRule::TrustedPersonalOnly,
        // 其余四档：§243／§94 说「按表决定」，而**那张表规范没有给出**。本设计不替它填。
        PrivacyClass::Public
        | PrivacyClass::Personal
        | PrivacyClass::Private
        | PrivacyClass::Secret => TransferRule::AnyNode,
    }
}

/// 两层取更严者。**这是闸门不可被策略放宽的另一半机制**：
/// [`TransferRule::TrustedPersonalOnly`] 在**任一操作数**上出现即吸收一切
/// ——于是 [`spec_floor`] 给出的下限永远不会被调用方的表放宽。
///
/// **操作数无位置之分**（`strictest` 是对称的）：故「调用方那侧写更严」与「规范那侧写更严」
/// 两条路得到同一枚结果，闸门只有「宽严」这一个自由度，没有「谁赢」这第二个自由度。
///
/// **它今天只被 [`place`] 的一处调用**（步骤 2 里那一行）；`LocalOnly` 那一档上
/// `spec_floor` 恒为 `TrustedPersonalOnly`，故**调用方在那一格填什么都不改变结果**
/// （设计 §5.4 的 I2 段；照片在 `the_gate_cannot_be_widened_by_any_caller_table`）。
fn strictest(a: TransferRule, b: TransferRule) -> TransferRule {
    match (a, b) {
        (TransferRule::TrustedPersonalOnly, _) | (_, TransferRule::TrustedPersonalOnly) => {
            TransferRule::TrustedPersonalOnly
        }
        (TransferRule::AnyNode, TransferRule::AnyNode) => TransferRule::AnyNode,
    }
}

/// 调用方给放置的两样东西：§94 的表，以及排序用的比较。
///
/// **收 `&dyn` 而不是泛型参数**：[`place`] 不需要 `Sized`，且 E 今天只有一份实现
/// （[`BaselinePlacementPolicy`]），泛型化只会把对象安全性换掉而不换来任何东西。
pub trait PlacementPolicy {
    /// §94 的表。见 [`PlacementRules`]。
    ///
    /// **必填、无默认**：一个「忘了传表」的调用方今天编不过，
    /// 而不是拿到一张全身是「放行」的默认表（同 [`PlacementRules`] 不派生 `Default` 的理由）。
    fn rules(&self) -> &PlacementRules;

    /// 全序比较：`Ordering::Less` 表示 `a` 排在 `b` 前面。
    ///
    /// **操作数就是 [`ComputeNode`]**——不另立一个「已打分节点」类型（同 D 的
    /// `compare`，设计 §5.5）。
    ///
    /// **它给出的是「哪一枚更合适」，不是「哪一枚能不能」**：能不能由 §5.3 的硬闸门单独决定，
    /// 策略**无权**放宽它（[`place`] 的步骤 2 在 [`compare`](PlacementPolicy::compare) 之前）。
    ///
    /// **今天零生产消费方**：[`place`] 的步骤 4（排序）在 Task 6，本 task 调用的
    /// `place` 不读本方法。它的唯一实现体是 [`BaselinePlacementPolicy`]。
    fn compare(&self, a: &ComputeNode, b: &ComputeNode) -> Ordering;
}

/// 具名基线策略。**它不对 §291 的任何一项因子作主张**——十一项里一项都算不出来
/// （设计 §5.5／§5.6、§10 第 7 条）。
///
/// [`compare`](PlacementPolicy::compare) 对任何一对返回 [`Ordering::Equal`]；
/// 次序完全由 [`place`] 的**兜底档**决定（§5.9：`ComputeNodeId` 升序，Task 6 落地）。
///
/// **它唯一的「主张」是 [`rules`](PlacementPolicy::rules) 交出的那张表**，而那张表也是
/// **调用方给的**——本类型一个值都不填（同上，§10 第 2 条：那四档的宽严完全取决于调用方）。
pub struct BaselinePlacementPolicy {
    /// 与 [`PlacementRules`] 同一种处置：由调用方给，本设计不替它填。
    rules: PlacementRules,
}

impl BaselinePlacementPolicy {
    pub fn new(rules: PlacementRules) -> Self {
        Self { rules }
    }
}

impl PlacementPolicy for BaselinePlacementPolicy {
    fn rules(&self) -> &PlacementRules {
        &self.rules
    }

    fn compare(&self, _a: &ComputeNode, _b: &ComputeNode) -> Ordering {
        // §10 第 7 条：排序没有基线，也没有可写的基线——十一项因子一项都算不出来。
        // 返回 `Equal` 不是「两枚一样好」这个主张，是「本层今天没有资格给出主张」。
        Ordering::Equal
    }
}

/// 一次放置。**同步纯函数**：收请求与策略，不接 `Tx`、不做 I/O、不读时钟
/// （设计 §5.2；这也是 [`PlacementError`] 里**不收** `Persist` 一类变体的唯一依据）。
///
/// 返回**单枚**节点——不是候选序列：§291 要的是「放在哪」，
/// `ExecutionProfile.compute_node` 也只要一枚。序列是本函数内部的中间物（§5.9）。
///
/// # 步骤（本 task 落 1–3，第 4、5 步见 Task 6）
///
/// 1. **判重**：`request.nodes` 里同一个 [`ComputeNodeId`](crate::ComputeNodeId) 出现两次即
///    `Err(`[`PlacementError::DuplicateNode`]`)`。放在第一步，因为它是 §5.9 那条「确定」断言的
///    **前提**，不是结尾的卫生检查。
/// 2. **过闸门**：对**每一枚** `request.artifacts` 读它的 `privacy_class`，算
///    `strictest(policy.rules().rule_for(level), spec_floor(level))`，据此把节点集滤到剩下的那一批。
///    [`TransferRule::TrustedPersonalOnly`] 的判据逐字是
///    `class == Personal && trust == TrustedPersonal`（设计 §5.3）。
/// 3. **判空**：第 2 步之后一枚不剩即 `Err(`[`PlacementError::NoPlaceableNode`]`)`。
/// 4. **排序**：`sort_by(|a, b| policy.compare(a, b).then_with(|| a.id().cmp(b.id())))`（§5.9 的兜底档）。**Task 6。**
/// 5. **取头**：返回排在第一的那一枚的借用。**本 task 在此取的是过滤后的第一枚，不排序。**
///
/// # 返回的生命周期
///
/// `&'a ComputeNode` 借用自 `request.nodes`，其生命周期由签名钉住——返回的那一枚节点
/// **活不过请求里的那一段切片**，故调用方不可能拿到一枚悬空的节点。
///
/// # 今天的唯一生产调用方是测试（纪律 6，照实记）
///
/// 本函数在 `src/` 里**零调用点**：唯一的调用面是 `tests/placement.rs`。
/// 下游的装配点（把放置结果写进 `ExecutionProfile.compute_node` 的那一步）
/// 在本仓**不存在**——`ExecutionProfile.compute_node` 是**裸 `Option<String>`**
/// （`crates/continuum-graph/src/execution.rs`），
/// [`ComputeNodeId`](crate::ComputeNodeId) 塞不进去（它是 `String` 的 newtype、不是 `String`）。
/// **本设计不擅自收紧那个字段**：改它是 G 那一侧的形状，且今天没有第二处会读它。
pub fn place<'a>(
    request: &PlacementRequest<'a>,
    policy: &dyn PlacementPolicy,
) -> Result<&'a ComputeNode, PlacementError> {
    // ── 步骤 1：判重 ────────────────────────────────────────────────
    // 两两比较，报出的是**第二次出现的那一枚**的 id（与 `RulesError::DuplicateLevel`
    // 报「第二次出现的那一档」同一取法：调用方要看的正是那个「撞上来了」的位置）。
    for (i, node) in request.nodes.iter().enumerate() {
        if request.nodes[..i].iter().any(|other| other.id() == node.id()) {
            return Err(PlacementError::DuplicateNode {
                id: node.id().clone(),
            });
        }
    }

    // ── 步骤 2：过闸门 ──────────────────────────────────────────────
    // 对每一枚制品算一次规则，规则更严的那一枚胜出（取交：一枚制品说不行就是不行）。
    //
    // **这里有本层唯一的 fail-closed 机制**：`strictest` 的右操作数是 `spec_floor(level)`，
    // 而 `spec_floor` **不读 `policy`**——故调用方的表只能收紧、放不宽。
    // `LocalOnly` 那一档上右臂恒为 `TrustedPersonalOnly`，于是那一格填什么都一样。
    let mut placeable: Vec<&ComputeNode> = Vec::with_capacity(request.nodes.len());
    for node in request.nodes.iter() {
        let mut allowed = true;
        for artifact in request.artifacts.iter() {
            let level = artifact.privacy_class;
            let rule = strictest(policy.rules().rule_for(level), spec_floor(level));
            if rule == TransferRule::TrustedPersonalOnly
                && !(node.class() == NodeClass::Personal
                    && node.trust() == NodeTrust::TrustedPersonal)
            {
                allowed = false;
                break;
            }
        }
        if allowed {
            placeable.push(node);
        }
    }

    // ── 步骤 3：判空 ＋ 步骤 5：取头（步骤 4 的排序在 Task 6）────────
    // 空制品集时 `placeable` 就是 `request.nodes` 全体——`LocalOnly` 的判据不适用，
    // 因为没有制品要保护（§9 的「无制品」那一格）。
    placeable
        .first()
        .copied()
        .ok_or(PlacementError::NoPlaceableNode)
}
