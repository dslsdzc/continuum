//! §287 的 `ComputeNode` 与它的三个取值类型（设计 §3）。
//!
//! §287（`docs/spec/05-normative.md:1649-1671`）给出六个字段：
//! `ComputeNode { id  class  capabilities  resources  trust  availability }`。
//! **六个里只有 `class` 的取值域是规范给的**（`PERSONAL | TEMPORARY`，§3.2）；
//! `trust` 规范只给字段名（§3.3），`capabilities` / `resources` / `availability` 只给名字
//! （§3.4）。故本模块里哪些形状来自规范、哪些是本设计取的，逐处写在各类型的文档里——
//! 不写清的话，后来者会把本设计取的两值信任域与 `Vec<String>` 读成规范的形状。

/// §287 的 `id`：注册表的键，也是放置结果的身份。
///
/// **不新建第二个 `NodeId`**：`continuum_graph::NodeId` 已存在且指 **ADFIR 节点**
/// （设计 §3.6）。§287 的 `id` 指的是**计算节点**，两者不是同一个概念——故这里**改名**
/// （`ComputeNodeId`）而不是沿用同一个词。这与 P3A 当年「`ToolId` 复用而不新建第二个」
/// 是同一条纪律的**反面用法**：那里两者是同一个概念、故复用；这里不是、故改名。
///
/// `Ord` 是必需的，不是顺手派生的：放置的确定性格（设计 §5.9）用它做排序的**最后一道**
/// ——`sort_by(|a, b| policy.compare(a, b).then_with(|| a.id().cmp(b.id())))`，
/// 即无论策略给出什么样的比较，`ComputeNodeId` 升序都是兜底档。它的消费方是 Task 6。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ComputeNodeId(String);

impl ComputeNodeId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// §287 的 "class"。**两枚，逐字照录规范**——`PERSONAL | TEMPORARY`，
/// 这是 §287 六个字段里**唯一**由规范给出取值域的一个（设计 §3.2）。
///
/// `Personal` 对应 §288 的 Personal Node，`Temporary` 对应 §289 的 Temporary Node。
/// **本设计不给它加第三枚。** 特别地，§4.4 的完成判据说的是「**云端**节点」，
/// 而 §287 里**没有「云端／本地」这个轴**——那里的处置是取 `class != Personal` 作超集
/// （更严，多排除了朋友的电脑一类，设计 §5.3 末段）。
///
/// `Copy` 与 `Eq` 是访问器返回面要的（`class()` 按值返回）；它今天没有比较大小的消费方，
/// 故**不派生** `Ord`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeClass {
    Personal,
    Temporary,
}

/// 节点在信任域中的位置。
///
/// **取值域是本设计定的，不是规范的形状**（设计 §3.3）：§287 只给字段名，§94
/// （`docs/spec/02-positioning.md:1054-1072`）只说 Scheduler 按「Artifact privacy
/// **× Node trust class**」决定是否允许传输，**也没有给这个 class 的取值域**。
/// 规范里能读出的最小区分只有下面两处，两个值各照一处取。
///
/// **若规范后来给出更细的信任分级，`NodeTrust` 要重取，且放行方向会变**（设计 §3.3 限度 1、
/// §10 第 3 条）：今天这两个值只在「能不能放 `LocalOnly` 的制品」这一处被读，闸门对它的用法是
/// **逐值归类**（§5.4 通道 (a)）——故**加值时必须把新值重新逐值归类**，
/// 不能假定它默认落在放行侧或拒绝侧。这条约定的另一头是：**信任的产生方不存在**
/// （按 §292 是 Authority Host，不在 E 的范围），本层**照抄调用方给的标签**，
/// 不做推断、也不校验它与 `class` 的关系（§3.3 限度 2）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeTrust {
    /// 来源：§293 的终态（`docs/spec/05-normative.md:1777`，Device Join 流程走完之后的
    /// `Trusted Personal Node`）。
    TrustedPersonal,
    /// 来源：§342.7（`docs/spec/05-normative.md:2697`）——`Temporary Node 不得**自动**进入
    /// Personal Trust Domain`。注意它的措辞是「不得**自动**进入」，**不是「不得进入」**。
    OutsidePersonalTrustDomain,
}

/// §287 的 `ComputeNode`：一枚计算节点在注册表里的样子。
///
/// **六个字段私有，唯一构造入口是 `new`**（设计 §3.5）。判据是**具体的一处**：
/// `ComputeNode` 要进放置的硬闸门，而闸门只读 `class` 与 `trust`——六个 `pub` 字段会让
/// 「闸门读到的 `class`」与「构造时给的 `class`」之间没有任何一处可挂不变量；
/// 私有字段加访问器把这两处收成一条路径。**代价照实记**：今天没有任何不变量可挂，
/// 故这一处私有性的收益是**结构性的**（将来加字段不许外部直接写字面量），
/// 不是今天可观察的。这条通道由 `tests/compile_fail/compute_node_fields_are_private.rs` 钉。
///
/// **只派生 `Debug`，`Clone` 与 `PartialEq` 都不得派生**：
///
/// - `Debug` 的判据是具体的一处——`place` 返回 `Result<&ComputeNode, _>`，而
///   `Result::unwrap_err` 的签名带 `T: Debug`，故测试里最自然的失败路径写法
///   （`let err = place(…).unwrap_err();`）要求它。
/// - **`Clone`**：今天零消费方。
/// - **`PartialEq`**：今天零消费方，且派生它会把「两枚节点相等」变成一条**本层没有判据**的
///   命题——放置只比 `id`（设计 §5.9），本层从不比较两枚节点整体。`ComputeNodeId` 与
///   `NodeClass` / `NodeTrust` 该有的派生在各自的接口处已列明，不靠这里顺手带上。
#[derive(Debug)]
pub struct ComputeNode {
    /// 注册表的键（设计 §4）与放置结果的身份；也是确定性排序的兜底档（§5.9）。
    id: ComputeNodeId,
    /// §287 的 `class`；硬闸门读它（§5.3）。
    class: NodeClass,
    /// §287 的 `trust`；硬闸门读它（§5.3）。**照抄调用方给的标签**：信任的产生方按 §292
    /// 是 Authority Host，不在 E 的范围，故本层不做任何推断、也不校验它与 `class` 的关系
    /// （§342.7 是 Authority 的义务，设计 §3.3 限度 2）。
    trust: NodeTrust,
    /// **只搬运，不解释、不比较、不排序**（设计 §3.4）。
    ///
    /// **代价，据实记**：§291 的十一项因子里有九项挂在 `capabilities` / `resources` /
    /// `availability` 与 `latency`／`money` 上，**本层一项都算不出来**（§3.4、§5.6、§10 第 4 条）。
    /// 一旦要比较两个标签，就得给出一套词表与一个序——两者规范都没有给。
    capabilities: Vec<String>,
    /// **只搬运，不解释、不比较、不排序**（同 `capabilities`，设计 §3.4）。
    resources: Vec<String>,
    /// **只搬运，不解释、不比较、不排序**（同 `capabilities`，设计 §3.4）。
    ///
    /// 注：因注册表没有刷新入口，它连「有一份当前状态」都谈不上（设计 §10 第 10 条）。
    availability: Vec<String>,
}

impl ComputeNode {
    pub fn new(
        id: ComputeNodeId,
        class: NodeClass,
        trust: NodeTrust,
        capabilities: Vec<String>,
        resources: Vec<String>,
        availability: Vec<String>,
    ) -> Self {
        Self {
            id,
            class,
            trust,
            capabilities,
            resources,
            availability,
        }
    }

    /// 消费方：§5.3 的硬闸门、§5.9 的兜底档、§5.8 的错误变体。
    pub fn id(&self) -> &ComputeNodeId {
        &self.id
    }

    /// 消费方：§5.3 的硬闸门。
    pub fn class(&self) -> NodeClass {
        self.class
    }

    /// 消费方：§5.3 的硬闸门。
    pub fn trust(&self) -> NodeTrust {
        self.trust
    }

    /// 消费方：**调用方自己实现的策略**（`PlacementPolicy` 的实现体）——**本 task 落地时
    /// E 内部零读取**（设计 §3.5 末段）。
    ///
    /// **这一句带时间与位置的限定，Task 5／6 的策略实现落地时要重取它**：今天本 crate 里
    /// 除本文件外**没有第二个模块**，故这句话的观察对象只有 `tests/node.rs` 的三条访问器断言。
    /// **不补用例**（已裁）：没有观察对象，写一条「本 crate 内不读它」的正则守卫是对**缺席**设守卫，
    /// 还得自己自证两侧，成本与收益不成比例。
    pub fn capabilities(&self) -> &[String] {
        &self.capabilities
    }

    /// 消费方：调用方自己实现的策略——**本 task 落地时 E 内部零读取**
    /// （同 `capabilities`，含那里关于时间限定与「不补用例」的说明）。
    pub fn resources(&self) -> &[String] {
        &self.resources
    }

    /// 消费方：调用方自己实现的策略——**本 task 落地时 E 内部零读取**
    /// （同 `capabilities`，含那里关于时间限定与「不补用例」的说明）。
    pub fn availability(&self) -> &[String] {
        &self.availability
    }
}
