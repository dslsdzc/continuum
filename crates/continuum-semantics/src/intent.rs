//! §222 的 Intent 数据模型与 §5.2 的版本语义。
//!
//! 计划「文件结构」一节把 `Intent、IntentKind、IntentStatus、Priority、ContractRef、
//! GraphIdRef、ProductIdRef、迁移表` 归在 `src/intent.rs`
//! （`docs/superpowers/plans/2026-10-06-p4-semantic-layer.md:301`）。
//! **本 task 把「迁移表」放在 `src/persist.rs`**（与 `continuum-canonical` 的
//! `p4_canonical_migrations` 同址，理由写在那边的文件头）——那一行是本计划的一处
//! 自相抵（`src/persist.rs` 那行也写着「七张表的迁移 110」），按计划自己的判据
//! 「实现期出现了更一致的归属，改的是计划那一行」（同文件 `:285-287`）办。

use continuum_canonical::{EntityId, Opaque};

use crate::ids::{ContractId, IntentId, PlanId};

/// §222 的六个 `kind`，**封闭六值**。
///
/// # 一处与规范原文的相抵（照实记）
///
/// §222（`docs/spec/05-normative.md:158`）的原话是「`kind` **至少**支持」，后接这六个词
/// ——「至少」字面上是一个**开放**取值域。设计 §5.1 把它取成**封闭六值**
/// （`design:836`：「`IntentKind`，六值封闭」），**这是一个设计决定，不是规范转录**。
///
/// 本层照设计办（封闭），代价写在下面：一枚「系统不认识的 kind」在库层读回时得到 `Err`，
/// 而不是被当成一个本层不解释的串存下来。这与 [`crate::GraphIdRef`] 那一类**不同**
/// ——那里的取值域规范也没给，而设计选了「不透明串」（§1.2.3 的规则 3 给了理由）；
/// 这里设计选了「封闭」，两者都是设计的决定。
///
/// # 匹配不带通配臂
///
/// [`IntentKind::as_str`] 加第七枚即**编译不过**——这是「六值落库编码**逐枚**有写」的
/// 编译期照片。（按本仓纪律，**编译不过不算变红**，故它是**类型保证**；运行期那一半
/// ——六枚各自的字面量——由 `tests/intent.rs` 的
/// `enum_columns_use_lowercase_underscore_and_reject_out_of_table_values` 逐枚读库断言。）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntentKind {
    /// §222 的 `TASK`。
    Task,
    /// §222 的 `PRODUCT`。
    Product,
    /// §222 的 `MAINTENANCE`。
    Maintenance,
    /// §222 的 `RESEARCH`。
    Research,
    /// §222 的 `BACKGROUND`。
    Background,
    /// §222 的 `RUNTIME_IMPROVEMENT`。
    RuntimeImprovement,
}

/// §223 的十三个状态。
///
/// **本 task 只定义词汇与落库编码**；合法迁移表与「哪些迁移合法」的判定归 Task 9
/// （`IntentStatusError` / `apply_intent_transition`）。这里**没有任何迁移判定**。
///
/// §223 给了两组（`docs/spec/05-normative.md:174-206`）：进入运行的五步链
/// `PROPOSED → SPECIFYING → PLANNING → REVIEW → READY → RUNNING`，
/// 运行后的六个态 `WAITING SUSPENDED VERIFYING COMPLETED FAILED CANCELLED`，
/// 另 `EVOLVING`（给 Product Intent）。**十三个名字，逐枚一个变体。**
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntentStatus {
    /// §223 的 `PROPOSED`。
    Proposed,
    /// §223 的 `SPECIFYING`。
    Specifying,
    /// §223 的 `PLANNING`。
    Planning,
    /// §223 的 `REVIEW`。
    Review,
    /// §223 的 `READY`。
    Ready,
    /// §223 的 `RUNNING`。
    Running,
    /// §223 的 `WAITING`。
    Waiting,
    /// §223 的 `SUSPENDED`。
    Suspended,
    /// §223 的 `VERIFYING`。
    Verifying,
    /// §223 的 `COMPLETED`。
    Completed,
    /// §223 的 `FAILED`。
    Failed,
    /// §223 的 `CANCELLED`。
    Cancelled,
    /// §223 的 `EVOLVING`（Product Intent 专用）。
    Evolving,
}

/// 落库编码的写法（计划「全局约束」逐字：小写、多词以 `_` 连接、不依赖 serde、
/// 不用 `Debug`）。
///
/// # 为什么是 `pub(crate)`
///
/// 这一对今天只有**一个**外部可观察的用处：`intent` 表的那一列。crate 外没有第二个
/// 消费者要自己编码一枚 `IntentKind`（要落库就调 [`crate::save_intent`]；要读就调
/// [`crate::load_intent`]）。故先按 `pub(crate)` 收着——**加宽是一字之改**，
/// 而一枚没有外部照片的公开表达力正是本仓判为「先不加」的那一类（同
/// `continuum_canonical::AliasScope::as_str`，它同样是 `pub(crate)`）。
impl IntentKind {
    /// 六枚之一 → 落库字面量。**全函数**（闭集六枚，没有第七种输入）。
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            IntentKind::Task => "task",
            IntentKind::Product => "product",
            IntentKind::Maintenance => "maintenance",
            IntentKind::Research => "research",
            IntentKind::Background => "background",
            IntentKind::RuntimeImprovement => "runtime_improvement",
        }
    }

    /// [`IntentKind::as_str`] 的逆。**定义域是所有串**，认不出就是 `None`。
    ///
    /// 返回 `Option` 而不返回一枚「未知」变体：本函数没有失败原因可给（它不读库、不写库）。
    /// 把认不出的串猜成某一枚会让库里一个被改坏的值静默变成一条有 kind 的记录——
    /// 那正是枚举列纪律要拦的。`None` 转成哪个错误由调用方的解码处决定
    /// （`continuum-semantics` 的 `persist.rs` 转成 [`crate::IntentError::UnknownKind`]，
    /// 带原始取值）。
    ///
    /// **带通配臂是刻意的**（与 [`IntentKind::as_str`] 的无通配相反）：这一边的输入是
    /// **开放集**（任意串），逐枚列出永远列不完。
    pub(crate) fn parse(s: &str) -> Option<IntentKind> {
        match s {
            "task" => Some(IntentKind::Task),
            "product" => Some(IntentKind::Product),
            "maintenance" => Some(IntentKind::Maintenance),
            "research" => Some(IntentKind::Research),
            "background" => Some(IntentKind::Background),
            "runtime_improvement" => Some(IntentKind::RuntimeImprovement),
            _ => None,
        }
    }
}

impl IntentStatus {
    /// 十三枚之一 → 落库字面量。**全函数**（闭集十三枚）。
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            IntentStatus::Proposed => "proposed",
            IntentStatus::Specifying => "specifying",
            IntentStatus::Planning => "planning",
            IntentStatus::Review => "review",
            IntentStatus::Ready => "ready",
            IntentStatus::Running => "running",
            IntentStatus::Waiting => "waiting",
            IntentStatus::Suspended => "suspended",
            IntentStatus::Verifying => "verifying",
            IntentStatus::Completed => "completed",
            IntentStatus::Failed => "failed",
            IntentStatus::Cancelled => "cancelled",
            IntentStatus::Evolving => "evolving",
        }
    }

    /// [`IntentStatus::as_str`] 的逆。**定义域是所有串**，认不出就是 `None`
    /// （理由同 [`IntentKind::parse`]）。
    pub(crate) fn parse(s: &str) -> Option<IntentStatus> {
        match s {
            "proposed" => Some(IntentStatus::Proposed),
            "specifying" => Some(IntentStatus::Specifying),
            "planning" => Some(IntentStatus::Planning),
            "review" => Some(IntentStatus::Review),
            "ready" => Some(IntentStatus::Ready),
            "running" => Some(IntentStatus::Running),
            "waiting" => Some(IntentStatus::Waiting),
            "suspended" => Some(IntentStatus::Suspended),
            "verifying" => Some(IntentStatus::Verifying),
            "completed" => Some(IntentStatus::Completed),
            "failed" => Some(IntentStatus::Failed),
            "cancelled" => Some(IntentStatus::Cancelled),
            "evolving" => Some(IntentStatus::Evolving),
            _ => None,
        }
    }
}

/// §222 的 `priority`。**§222 未给取值域**（设计 §16 第 8 条）——本层取不透明表示、
/// **不排序**。
///
/// # 「不改写、不裁剪」「无校验」有照片，「不排序」没有（按纪律 3 明写）
///
/// 「不改写、不裁剪」「无校验」由 `tests/intent.rs` 的
/// `opaque_strings_are_not_rewritten_trimmed_or_rejected` 直接钉住：带空白／特殊字符／
/// 非 ASCII 的串**逐字节往返**、空串也被接受。
/// **「不排序」没有运行期照片**——它是「本类型没有比较 API」这件事本身：`Priority`
/// 不实现 `Ord` / `PartialOrd`、也没有比较方法，故没有一条会红的用例可写。
/// 按纪律 3「要么有可写出的照片，要么明写为什么没有」，这一句留在原处。
///
/// # 为什么不直接用 [`Opaque`]
///
/// `Priority` 与 `completion_predicate` 今天都是不透明串，但它们是**两个不同的字段**。
/// 合并成一个类型会让「把 priority 填进 completion_predicate」在类型上可写。
/// 这一处比 [`crate::IntentId`] 那一处更值得分开：那三个是**同一种东西的三个位置**
/// （都是记录键），而这两个是**两个语义不同的字段**，将来其中一个先有了取值域，
/// 收窄它不该连带收窄另一个（与 `continuum_canonical::MentionKind` 另立类型同一条理由）。
#[derive(Debug, Clone)]
pub struct Priority(Opaque);

impl Priority {
    /// 取里子串。**不改写、不裁剪**。
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// 从任意串构造。**无校验**（同 `Opaque::from_str`）。
    pub fn from_str(s: &str) -> Self {
        Priority(Opaque::from_str(s))
    }
}

/// §222 的 `contract_id`，**补上版本**。
///
/// # 为什么有 `version`（本设计的补，不是规范转录）
///
/// §222 的 `Intent.contract_id` 与 §235 的 `ADFIRGraph.contract_id` **都只有 id、
/// 没有版本**（`docs/spec/05-normative.md:148`、`:250`），而 §226 的 `ContractDiff`
/// 消费**必须知道基准版本**（否则「哪些节点受影响」没有比较基准）。
/// 故本层的 `Intent.contract_id` 取 `ContractRef { id, version }`（设计 §7.3）。
///
/// **这是本设计补的字段**：读它的人不该以为 §222 里有 `version` 这个名字。
/// 执行层的 `adfir_graph` 也需要版本，那是对已建之物的一处请求（设计 §13.2），
/// **不在本 crate 落地**。
///
/// # 字段是 `pub` 的
///
/// 同 [`Intent`]：本类型是 §222 的一个**记录字段**，没有比 §222 更强的约束可挂。
#[derive(Debug, Clone)]
pub struct ContractRef {
    /// §222 的 `contract_id`（`id` 那一半）。
    pub id: ContractId,
    /// 基准版本。**§222 未给这个字段名**（见类型文档）。
    pub version: u32,
}

/// §222 的 `graph_id`。**不透明串**。
///
/// # 为什么不是执行层那个 `GraphId`（`continuum-graph` 的类型）
///
/// 取它会写出一条指向执行层的 `Cargo.toml` 边（设计 §1.2.3 的规则 3）——
/// 而本层**不登记任何指向执行层的边**。故这里取不透明串，代价是「本层这个串」与
/// 「执行层那个 `GraphId`」之间的对应由装配方维持（同 `EntityId` 那一处的代价，
/// 见 `continuum-canonical` 的 `candidate.rs`）。**第二侧守卫是 Task 21 的 grep**；
/// 本 crate 内的零命中由 `tests/intent.rs` 的
/// `graph_id_and_product_id_are_opaque_strings` 钉住。
#[derive(Debug, Clone)]
pub struct GraphIdRef(String);

impl GraphIdRef {
    /// 取里子串。**不改写、不裁剪**。
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 从任意串构造。**无校验**（理由同 [`crate::GraphIdRef`] 的文档：取值域规范未给）。
    ///
    /// **表外取值是被接受的**：本类型没有「合法取值」这个概念，
    /// `tests/intent.rs` 的同名用例断言一个表外取值往返。
    pub fn from_str(s: &str) -> Self {
        GraphIdRef(s.to_owned())
    }
}

/// §222 的 `product_id`。**不透明串**，理由与 [`GraphIdRef`] 逐字相同
/// （设计 §5.1 把它与 `graph_id` 列在同一格：「同上，不透明 `String`」）。
///
/// 设计另记了一处：§4.3 的第 1 条守卫里，`product_id` 是 Intent 上一个**没入枚的目标**
/// （`design:569-570`）——它今天不与 `CandidateFacts` 相交。
#[derive(Debug, Clone)]
pub struct ProductIdRef(String);

impl ProductIdRef {
    /// 取里子串。**不改写、不裁剪**。
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 从任意串构造。**无校验**。
    pub fn from_str(s: &str) -> Self {
        ProductIdRef(s.to_owned())
    }
}

/// §222 的十五个字段（下表十三行）。
///
/// # 十五个规范字段 → 十四个 Rust 成员
///
/// §222 逐字列了**十五个**字段（`docs/spec/05-normative.md:132-155`）：
/// `id`、`version`、`kind`、`goal`、`status`、`priority`、`created_at`、`updated_at`、
/// `parent_intent?`、`child_intents[]`、`contract_id`、`active_plan_id?`、`graph_id?`、
/// `product_id?`、`completion_predicate`。
///
/// **`child_intents[]` 不是本结构体的成员**：§5.1 的「子列表不落列」——只存父指针，
/// 子列表由 [`crate::list_children`] 派生（两份真值会漂移，本仓一贯判为缺陷的那一类）。
/// 故本结构体有**十四个**成员，第一节四处逐一列在
/// `tests/intent.rs` 的 `intent_carries_exactly_the_fifteen_fields_of_222` 的文档注释里
/// （那一条的穷尽解构绑的正是这十四个）。
///
/// # `goal` 取 `EntityId`——这里**不是** `CanonicalRepresentation`（因为下面两条）
///
/// 计划 Step 4 的代码速写把 `goal` 写成 `CanonicalRepresentation`，设计 §5.1 那一格
/// 也写「规范化后的表示（§3.2）」。**本 task 取 `EntityId`**，两条理由：
///
/// 1. **设计 §1.3 的读法甲（本设计采纳的那一支）逐字说类型边是 `EntityId`**：
///    「Intent 表的**外键面依赖 `EntityId` 这一个类型**，而 Intent 的任何判定都不查
///    Canonicalization」（`design:184-186`）。`continuum-runtime` 的 `ALLOWED` 注释与
///    计划「全局约束」也是这么写的（「`Intent.goal` 取 `EntityId` 类型，设计 §1.3 的读法甲」）。
/// 2. **`CanonicalRepresentation` 在 crate 外造不出来**：它的字段私有、唯一构造入口
///    `new` 是 `pub(crate)`（`continuum-canonical/src/canonical.rs`），
///    crate 外**只有一个**取得它的入口 `finish`。故它是**一次性产物**，不能是
///    [`crate::load_intent`] 要从库里**读回**的记录字段——读回来就得在 crate 外把它拼出来，
///    而那是写不出来的（同 [`crate::GraphIdRef`] 那一类「本层把跨层输入当真值存下来」的形状）。
///
/// 代价照实写：`goal` 因此**不是**一份「哪些 mention 解到了哪个实体」的记录，
/// 只是那一个不透明串——与 `EntityId` 在 `candidate.rs` 里自陈的代价同源。
///
/// # 版本语义（§5.2 的设计决定）
///
/// `kind` / `goal` / `parent_intent` / `completion_predicate` / `contract_id` **五个
/// 语义字段**之一变化时 `version` +1；`status` 迁移**不** +1。
/// **这两件事的实现落点在 [`crate::save_intent`]**，不在本结构体上——版本是**写路径**
/// 算出来的，不是这个记录自带的不变量。规则的理由（§64 要的是「用户要求」的版本，
/// 不是事件计数）写在 `design:848-863`。
///
/// # 字段是 `pub` 的（同 `Candidate`，而 `CanonicalRepresentation` 的不是）
///
/// 区别仍在于**有没有不变量可挂**：本类型是 §222 逐字给出的记录，本层没有比 §222
/// 更强的约束可挂（版本那条规则挂在写路径上，见上）。锁起来只会让调用方多写一个构造函数，
/// 而不变量一条也不会多。**这条选择本身有照片**：`tests/intent.rs` 的
/// `intent_carries_exactly_the_fifteen_fields_of_222` 用**不带 `..`** 的穷尽解构钉住字段清单。
#[derive(Debug, Clone)]
pub struct Intent {
    /// §222 的 `id`。落库主键。
    pub id: IntentId,
    /// §222 的 `version`。**由 [`crate::save_intent`] 算**（见类型文档）。
    pub version: u32,
    /// §222 的 `kind`。
    pub kind: IntentKind,
    /// §222 的 `goal`（见类型文档：本层取 `EntityId`）。
    pub goal: EntityId,
    /// §222 的 `status`。合法迁移的判定归 Task 9。
    pub status: IntentStatus,
    /// §222 的 `priority`（不透明）。
    pub priority: Priority,
    /// §222 的 `created_at`（`i64` 毫秒）。**由调用方给**：本层不取时钟
    /// （同 `continuum-workspace` 的 `occurred_at` 处置），故它是记录的第一个字段而不是
    /// 库自己填的一列。
    pub created_at: i64,
    /// §222 的 `updated_at`（`i64` 毫秒）。**由调用方给**，理由同 `created_at`；
    /// 状态迁移改它（§5.2）。
    pub updated_at: i64,
    /// §222 的 `parent_intent?`。**子列表不落列**，由 [`crate::list_children`] 派生。
    pub parent_intent: Option<IntentId>,
    /// §222 的 `contract_id`，补上了版本（见 [`ContractRef`]）。
    pub contract_id: ContractRef,
    /// §222 的 `active_plan_id?`。
    pub active_plan_id: Option<PlanId>,
    /// §222 的 `graph_id?`（见 [`GraphIdRef`]）。
    pub graph_id: Option<GraphIdRef>,
    /// §222 的 `product_id?`（见 [`ProductIdRef`]）。
    pub product_id: Option<ProductIdRef>,
    /// §222 的 `completion_predicate`。**§222 只给字段名**：判定属第 8 层
    /// （§266 Completion Predicate），本层只存（设计 §16 第 9 条）。
    pub completion_predicate: Opaque,
}
