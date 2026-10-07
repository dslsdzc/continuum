//! §5.2–§5.4、§5.5、§5.8、§5.9 的运行期照片：请求面、硬闸门、`place` 的全部五步与它的两枚失败变体。
//!
//! 设计：`docs/superpowers/specs/2026-10-06-p3e-compute-placement-design.md`。
//!
//! # 本文件的两段式射程（**Task 5 的夹具刻意收窄，Task 6 的夹具才有次序**）
//!
//! Task 5 落 `place` 的**步骤 1–3**（判重 → 过闸门 → 判空），那些用例的每一份夹具都构造成
//! 「**过闸门后恰好剩一枚**」或「**次序无关**」；Task 6 落**步骤 4–5**（排序 → 取头），
//! 本文件下半部分才第一次让**多枚可比节点**同时存活——那是 **Task 6 落地的三条**（其中一条含两条路径）。
//!
//! **两段之间有两条刻意的口径差，别读成不一致**：
//!
//! - `the_gate_cannot_be_widened_by_any_caller_table` 的 (b) 臂里两枚节点都过闸门，
//!   那一臂**只断言「返回的 id 在两枚之内」**，不断言具体是哪一枚。**Task 6 落地后
//!   `place` 确实会排序**，那一臂本可收紧成「必是 `"a"`」——**不收紧是刻意的**：
//!   收紧会把 Task 5 对「`spec_floor` 对那四档也改成 `TrustedPersonalOnly`」那条变异体的
//!   预测一并改掉（那条预测说本臂不红）。理由写在该用例的注释里。
//! - Task 5 的 `RecordingPolicy` 的 `compare` 恒 `Equal`；Task 6 另加一份
//!   [`ByCapabilityCount`]（按 `capabilities()` 的标签数**降序**）——两份都留着，
//!   前者的「恒 `Equal`」本身就是兜底档那条用例要的前提。
//!
//! # 本文件里 `:NNN:CC` 那类行号的来历（读之前先看这一节）
//!
//! 几处「红在哪条断言」的注释里带着 `tests/placement.rs:NNN:CC` 形式的位置。**它们是实测值，
//! 但它绑的是量它那一刻的字节**：Task 5 量过一次，Task 6 在本文件前面加了约 100 行
//! （文件头、三枚节点的夹具、`ByCapabilityCount`），**行号因此整体后移**。
//! 故 **Task 6 在最终字节上把这几处重测了一遍**，本文件写的是**重测值**；
//! Task 5 的原值在报告里留档（`.superpowers/sdd-p3e-impl/task-5-report.md`）。
//! **Task 5 那一轮的变异体字节没有留档**，重测用的是按同一机制重造的等价变异体
//! ——逐枚的源码改写与 sha256 见 `.superpowers/sdd-p3e-impl/task-6-report.md`。
//! （**这一类引用天生会漂**：本文件再插行，它们就会再次失真。）

//! # 夹具的 id 取法是承重的
//!
//! 两个节点工厂都收 `id` 参数、**不写死**（设计 §9 的判据 §4.4 那一格与判重那一格的 id 要求互相冲突：
//! 前者要 `[cloud, desktop]` 两枚可比，判重要两枚**同 id**，四种组合那一格要**四枚 id 各不相同**）。
//!
//! **判据 §4.4 的正面用例里，云节点取两枚中较小的那个 id**（`cloud("a")` ＋ `desktop("b")`）：
//! 这样**无论 `place` 排不排序**（Task 6 起它会排序），只要云节点没被滤掉，返回的就必是云节点
//! ——于是「闸门真的过滤了」与「它只是恰好排在前面」两件事才分得开；
//! 该用例因此是**承重的**：过滤若失效，返回的必是云节点。

use std::cmp::Ordering;

use continuum_artifact::{Artifact, ArtifactId, ArtifactType, ContentHash, PrivacyClass};
use continuum_node::{
    place, BaselinePlacementPolicy, ComputeNode, ComputeNodeId, NodeClass, NodeTrust,
    PlacementError, PlacementPolicy, PlacementRequest, PlacementRules, TransferRule,
};
use serde_json::json;

/// 一枚制品。**十个字段全是字面量**，`privacy_class` 是**唯一**随参数变动的字段。
///
/// 其余九项取固定值，理由：本节要分辨的唯一一维是「闸门的判据读的是不是 `privacy_class`」，
/// 让别的字段跟着动只会引入与判据无关的自由度。
///
/// `metadata` / `provenance` 取 `serde_json::json!({})`——两个字段的类型是 `serde_json::Value`，
/// 而 `continuum-artifact` **不 re-export 它**（`crates/continuum-artifact/src/lib.rs` 的 `pub use`
/// 清单里没有 `serde_json`），故本文件必须自己登记依赖（见 `Cargo.toml` 的 `[dev-dependencies]`）。
fn artifact(level: PrivacyClass) -> Artifact {
    Artifact {
        id: ArtifactId::new("artifact-1"),
        artifact_type: ArtifactType::Text,
        content_hash: ContentHash::of(b"placement-fixture"),
        size: 0,
        producer_node: None,
        input_artifacts: Vec::new(),
        metadata: json!({}),
        provenance: json!({}),
        privacy_class: level,
        version: 1,
    }
}

/// 任意 `class` × `trust` 组合、任意 `capabilities` 的节点。
/// **三个具名工厂都建在它上面**——`four_class_trust_combinations_and_only_one_passes`
/// 要的正是四种 `class` × `trust` 组合，而具名工厂只覆盖其中两种。
fn node(id: &str, class: NodeClass, trust: NodeTrust, capabilities: Vec<String>) -> ComputeNode {
    ComputeNode::new(
        ComputeNodeId::new(id),
        class,
        trust,
        capabilities,
        Vec::new(),
        Vec::new(),
    )
}

/// 云节点（§5.3 的取法：`class ≠ Personal` 是「云端节点」的超集）。
fn cloud(id: &str) -> ComputeNode {
    node(
        id,
        NodeClass::Temporary,
        NodeTrust::OutsidePersonalTrustDomain,
        Vec::new(),
    )
}

/// 桌面节点：§288 的 Personal Node，且已在 Personal Trust Domain 内（§293 的终态）。
fn desktop(id: &str) -> ComputeNode {
    node(
        id,
        NodeClass::Personal,
        NodeTrust::TrustedPersonal,
        Vec::new(),
    )
}

/// 带 `n` 个 `capabilities` 标签的桌面节点（`class` / `trust` 与 [`desktop`] 同）。
///
/// **标签的内容不带任何语义**：§3.4 明写 `capabilities` 本层「只搬运，不解释、不比较、不排序」，
/// 故这里给的是 `"cap-1"` 这样的占位串——**承重的只有条数**，
/// [`ByCapabilityCount::compare`] 读的也正是条数。
fn desktop_with_tags(id: &str, n: usize) -> ComputeNode {
    node(
        id,
        NodeClass::Personal,
        NodeTrust::TrustedPersonal,
        (1..=n).map(|i| format!("cap-{i}")).collect(),
    )
}

/// 本 task 共用的三枚节点里 `id` 为 `id` 的那一枚。**表写死如下**（计划 Task 6 的那张表）：
///
/// | id | 工厂 | `class` | `trust` | 标签数 |
/// |---|---|---|---|---|
/// | `"a"` | [`desktop`] | `Personal` | `TrustedPersonal` | 0 |
/// | `"b"` | [`desktop_with_tags`]`(_, 1)` | `Personal` | `TrustedPersonal` | 1 |
/// | `"c"` | [`desktop_with_tags`]`(_, 3)` | `Personal` | `TrustedPersonal` | 3 |
///
/// **两处是承重的**：
///
/// 1. **`class` / `trust` 取 `desktop`**（`Personal` / `TrustedPersonal`）⇒ 三枚**都过闸门**
///    （本 task 的制品是 `Public` ＋ [`rules_all_any`]，而 §4.4 的闸门只对 `LocalOnly` 收紧）；
/// 2. **id 与标签数反向**：`"a"` 的 id 最小而标签最少、`"c"` 的 id 最大而标签最多——
///    于是降序策略的赢家（`"c"`）与 id 升序的赢家（`"a"`）**是两个不同的节点**。
///    这正是「确定性」与「兜底档」两条各自可观察的前提（否则是**等价变异体**：
///    「读不读策略」「排不排序」两件事分不开）。
fn fixture_node(id: &str) -> ComputeNode {
    match id {
        "a" => desktop("a"),
        "b" => desktop_with_tags("b", 1),
        "c" => desktop_with_tags("c", 3),
        other => panic!("本 task 的夹具只有 a / b / c 三枚节点，收到 {other:?}"),
    }
}

/// **同一组**三枚节点（见 [`fixture_node`]），**按 `ids` 给的顺序**放进 `Vec`。
///
/// 每次调用都**现造**：`ComputeNode` 不派生 `Clone`（设计 §3.5：零消费方），
/// 故「同一组节点换顺序」只能靠重新构造，不能靠克隆。
fn three_nodes_in_order(ids: [&str; 3]) -> Vec<ComputeNode> {
    ids.into_iter().map(fixture_node).collect()
}

/// 五档全 `AnyNode` 的表。**它是最宽的一张表**——
/// 「闸门不可被策略放宽」那一格要用它当反例。
fn rules_all_any() -> PlacementRules {
    PlacementRules::try_new(
        PrivacyClass::ALL
            .iter()
            .map(|&level| (level, TransferRule::AnyNode))
            .collect(),
    )
    .expect("五档全 AnyNode 的表覆盖 PrivacyClass::ALL 每一档，必须被接受")
}

/// 只把 `level` 那一档写成 `rule`、其余四档 `AnyNode` 的表。
///
/// 判据：`PlacementRules` 的查询面是逐档的（设计 §5.4 (b)），
/// 故「某一档被收紧」这件事只能用**只动那一档**的表来拍。
fn rules_with(level: PrivacyClass, rule: TransferRule) -> PlacementRules {
    PlacementRules::try_new(
        PrivacyClass::ALL
            .iter()
            .map(|&l| (l, if l == level { rule } else { TransferRule::AnyNode }))
            .collect(),
    )
    .expect("五档齐全的表必须被接受")
}

/// 测试侧的调用方策略：`rules()` 交出给定的表，`compare` 对任何一对返回 `Ordering::Equal`。
///
/// **`compare` 恒 `Equal` 是刻意的**：本策略服务的用例要分辨的是「闸门读不读 `privacy_class`」，
/// 一份「有主张的比较」会在那些用例里引入与判据无关的次序自由度。
/// **这不等于「`place` 不读策略」**——`place` 自 Task 6 起真的读 `compare`，
/// 而恒 `Equal` 的那一份读出来的结果由兜底档接手。
struct RecordingPolicy {
    rules: PlacementRules,
}

impl RecordingPolicy {
    fn new(rules: PlacementRules) -> Self {
        Self { rules }
    }
}

impl PlacementPolicy for RecordingPolicy {
    fn rules(&self) -> &PlacementRules {
        &self.rules
    }

    fn compare(&self, _a: &ComputeNode, _b: &ComputeNode) -> Ordering {
        Ordering::Equal
    }
}

/// 测试侧的调用方策略：`compare` 按 [`ComputeNode::capabilities`] 的标签数**降序**
/// （标签多者在前）。
///
/// # 方向是承重的
///
/// 本 task 的夹具（[`fixture_node`]）里 `"c"` 的 id 最大而标签最多，
/// 故**降序策略的赢家（`"c"`）与 id 升序的赢家（`"a"`）是两个不同的节点**。
/// 若把它改成升序（或把夹具的标签数配反），本策略会与兜底档指向同一枚，
/// 「排序真的读了策略」与「排序只是按 id」就再也分不开——那是**等价变异体**。
///
/// # 它钉的是「`place` 读策略」，不钉「哪个策略对」
///
/// 标签数多者更合适**不是本层的主张**：§291 的十一项因子里九项连量纲都没有
/// （设计 §5.6），本层给不出数值基线。这份实现体只是**一个可替换的样例**
/// （设计 §9 的「策略可替换」那一格的口径逐字如此）。
struct ByCapabilityCount {
    rules: PlacementRules,
}

impl ByCapabilityCount {
    fn new(rules: PlacementRules) -> Self {
        Self { rules }
    }
}

impl PlacementPolicy for ByCapabilityCount {
    fn rules(&self) -> &PlacementRules {
        &self.rules
    }

    fn compare(&self, a: &ComputeNode, b: &ComputeNode) -> Ordering {
        // 降序：`b.cmp(a)` 而不是 `a.cmp(b)`。
        b.capabilities().len().cmp(&a.capabilities().len())
    }
}

/// **判据 §4.4 的正面**（《工程》§4.4 的第三条：`LOCAL_ONLY` 的制品不会被放置到云端节点）。
///
/// 夹具：`nodes = [cloud("a"), desktop("b")]`，云节点**在前且 id 更小**（见文件头的 id 取法），
/// 制品是 `LocalOnly`，策略表五档全 `AnyNode`。
///
/// **红在哪条断言**：`assert_eq!(n.class(), NodeClass::Personal)` ——
/// **红的条件（档位：移除）**：把过闸门那一步（步骤 2）整段删掉 → 返回的必是云节点，
/// 于是 `class()` 与 `trust()` **两条断言同时红**（这一条与下一条是成对的，
/// 见 `a_local_only_artifact_with_no_trusted_personal_node_is_unplaceable`）。
#[test]
fn a_local_only_artifact_lands_only_on_a_trusted_personal_node() {
    let nodes = vec![cloud("a"), desktop("b")];
    let artifacts = vec![artifact(PrivacyClass::LocalOnly)];
    let request = PlacementRequest {
        artifacts: &artifacts,
        nodes: &nodes,
    };
    let policy = RecordingPolicy::new(rules_all_any());

    let placed = place(&request, &policy).expect("个人且被信任的桌面节点在场，LocalOnly 该放得下");

    assert_eq!(
        placed.class(),
        NodeClass::Personal,
        "LocalOnly 落在了非 personal 的节点上"
    );
    assert_eq!(
        placed.trust(),
        NodeTrust::TrustedPersonal,
        "LocalOnly 落在了未被信任的节点上"
    );
}

/// **判据 §4.4 的否定面**。节点集只有云节点，同一个 `LocalOnly` 制品 → `Err(NoPlaceableNode)`。
///
/// **它与上一条是成对的**：单看任一条分不清「被闸门滤掉」与「本来就没节点」——
/// `NoPlaceableNode` 不区分两者是**刻意的**（设计 §5.8：§243／§291 没有要求放置输出一个理由）。
///
/// **红在哪条断言**（实测 M3 落在 `:294:40`）：`unwrap_err()`——**该用例的第一条断言就是「必须返回 `Err`」**。
/// **红的条件（档位：放宽）**：把闸门改成「滤掉之后若为空就退回未过滤的集合」→ 红。
#[test]
fn a_local_only_artifact_with_no_trusted_personal_node_is_unplaceable() {
    let nodes = vec![cloud("a")];
    let artifacts = vec![artifact(PrivacyClass::LocalOnly)];
    let request = PlacementRequest {
        artifacts: &artifacts,
        nodes: &nodes,
    };
    let policy = RecordingPolicy::new(rules_all_any());

    let err = place(&request, &policy).unwrap_err();
    assert!(
        matches!(err, PlacementError::NoPlaceableNode),
        "只有云节点时 LocalOnly 该判 NoPlaceableNode，实际是 {err:?}"
    );
}

/// **放行侧**（设计 §9 的 I1 格）。`Public` ＋ 表里 `Public` 那档 `AnyNode` ＋ 只有云节点 → `Ok`。
///
/// **这一格是必需的**：没有它，一个「把所有制品都当 `LocalOnly` 一律拒掉」的实现
/// 能通过本文件其余每一行——那正是 fail-closed 的反面（闸门静默拒绝一切）。
/// 上面两条钉「该拒的拒」，这一条钉「该放的放」。
///
/// **红在哪条断言**（实测 M4 落在 `:330:43`）：`expect(…)`——**该用例的第一条断言就是「必须返回 `Ok`」**。
/// **红的条件（档位：收紧）**：让闸门对所有等级都返回 `Err`——或只把 `spec_floor` 对那四档
/// 也改成 `TrustedPersonalOnly`。**同一条变异体还会红下面这两处，逐条点名**：
/// `the_gate_cannot_be_widened_by_any_caller_table` 的 **(a) 臂的四个 `AnyNode` 档**
/// （同一情形：`nodes = [cloud("a")]`）；**该用例的 (b) 臂不红**（它是 `[cloud, desktop]`，
/// 变异后 `desktop` 仍在，故那四档仍得 `Ok`）。**不会红的**（免得去查不存在的问题）：
/// `a_caller_supplied_rule_can_tighten_the_gate`（本来就期望 `Err`）、
/// `four_class_trust_combinations_and_only_one_passes`（全走 `LocalOnly`，`spec_floor` 没动）、
/// 两条空制品集的用例（闸门连一档都不看）、两条期望 `Err` 的用例（与该变异体同解），
/// 以及 **Task 6 的三条**（`the_same_node_set_in_any_order_yields_the_same_node` ／
/// `the_id_breaks_ties_when_the_policy_says_equal` ／ `swapping_the_policy_changes_the_result`；
/// 它们的节点都是 `desktop(...)`，正是变异后仍放行的那一类）。
/// **这三条这一句是 Task 6 的复核读数**（不是预测）：见报告「Task 5 预言的复核」一节。
#[test]
fn a_public_artifact_may_land_on_a_cloud_node() {
    let nodes = vec![cloud("a")];
    let artifacts = vec![artifact(PrivacyClass::Public)];
    let request = PlacementRequest {
        artifacts: &artifacts,
        nodes: &nodes,
    };
    let policy = RecordingPolicy::new(rules_with(PrivacyClass::Public, TransferRule::AnyNode));

    let placed = place(&request, &policy).expect("Public 且表里放行，云节点该放得下");

    assert_eq!(
        placed.id(),
        &ComputeNodeId::new("a"),
        "返回的不是那枚云节点"
    );
    assert_ne!(
        placed.class(),
        NodeClass::Personal,
        "夹具自证：这枚节点本就该是云节点（class ≠ Personal）"
    );
}

/// **放行侧的收紧面；裁定义务 1 的落点**（设计 §9 的「调用方收紧」格、§10 第 2 条的照片）。
///
/// `Secret` ＋ 表里 `Secret` 那档写 `TrustedPersonalOnly` ＋ 只有云节点 → `Err(NoPlaceableNode)`。
///
/// **这一条钉的是「调用方的表真的被读」**：`spec_floor(Secret)` 是 `AnyNode`（规范只钉了 `LocalOnly`），
/// 故一个**完全不读 `policy.rules()`、只算 `spec_floor(level)`** 的实现会返 `Ok` ⇒ 在这一格红。
/// **同时它是 §10 第 2 条那句「那四档的宽严完全取决于调用方」的照片**——没有它，那句话既无照片、
/// 又在「不读表」的实现下为假。
///
/// **红在哪条断言**（实测 M1 落在 `:371:40`）：`unwrap_err()`——**该用例的第一条断言就是「必须返回 `Err`」**。
/// **红的条件（档位：移除）**：把 `strictest` 的两个操作数之一换成常量 `AnyNode`
/// （即丢掉调用方那一侧）→ 红。
/// **本条与 `the_gate_cannot_be_widened_by_any_caller_table` 是同一对的两个方向，缺一不可**：
/// 只钉后者（全 `AnyNode`）时，「不读表」的实现照样全绿——它与「五档全 `AnyNode`」同解。
#[test]
fn a_caller_supplied_rule_can_tighten_the_gate() {
    let nodes = vec![cloud("a")];
    let artifacts = vec![artifact(PrivacyClass::Secret)];
    let request = PlacementRequest {
        artifacts: &artifacts,
        nodes: &nodes,
    };
    let policy = RecordingPolicy::new(rules_with(
        PrivacyClass::Secret,
        TransferRule::TrustedPersonalOnly,
    ));

    let err = place(&request, &policy).unwrap_err();
    assert!(
        matches!(err, PlacementError::NoPlaceableNode),
        "调用方把 Secret 收紧成 TrustedPersonalOnly 后该判 NoPlaceableNode，实际是 {err:?}"
    );
}

/// **闸门不可被策略放宽**（设计 §5.3 的绝对措辞、§9 的同名格）：逐档遍历，**不抽代表**。
///
/// 策略的表**五档全写 `AnyNode`**，对 [`PrivacyClass::ALL`] 的每一档各跑两次：
///
/// - **(a)** `nodes = [cloud("a")]`：`LocalOnly` → `Err(NoPlaceableNode)`；其余四档 → `Ok(云节点)`；
/// - **(b)** `nodes = [cloud("a"), desktop("b")]`：`LocalOnly` → `Ok`，且 `class == Personal` 与
///   `trust == TrustedPersonal` **逐项断言**；其余四档 → `Ok`，且返回的 id **在两枚之内**。
///
/// **(b) 臂那四档为什么不收紧成「必是 `"a"`」**（Task 6 落地后 `place` 确实排序，
/// 按步骤 4 这两枚里 id 较小的是 `"a"`）：**收紧会连带改掉下面那条变异体的预测**——
/// 「`spec_floor` 对那四档也改成 `TrustedPersonalOnly`」时云节点被滤掉、只剩 `desktop("b")`，
/// 本臂仍该是 `Ok`；若这里断言「必是 `"a"`」，那条变异体会把本臂一起带红，
/// 本用例就不再是「只有 `a_public_artifact_may_land_on_a_cloud_node` 那一处红」。
/// **两枚都允许**这个口径是本用例**刻意的下界**，不是 Task 6 的遗漏。
///
/// **绝对措辞与照片**：设计 §5.3 写「`LocalOnly` **一概**上不了非 personal、或未被信任的节点
/// ——包括调用方把五档全写成 `AnyNode` 的情形」。**这条绝对措辞的照片就是 (a) 与 (b) 的
/// `LocalOnly` 两条臂**，不是靠读代码。
///
/// **为什么逐档遍历而不是只测 `LocalOnly`**：(b) 的「其余四档 → `Ok`」那四臂钉的是
/// **放行侧不能跟着一起收紧**；只留 `LocalOnly` 一格时，一个「对一切等级都过闸门」的实现
/// 与一个「对一切等级都拒」的实现各能骗过一半。
///
/// **红在哪条断言**：末尾那一条 `assert!(failures.is_empty(), …)`（两条臂的失败都累积进 `failures`，
/// 故「红了几处」在报错信息里可读）。
/// **红的条件（档位：放宽）**：把 `strictest` 改成返回第一个操作数（即调用方的规则胜出）→
/// 本条的 `LocalOnly` 两臂红（而 `a_caller_supplied_rule_can_tighten_the_gate` 仍绿——
/// 两条各自独立）。**实测**：该变异体下本用例的报错信息里是 **3** 条——
/// (a) 臂 1 条，(b) 臂 2 条（`class` 与 `trust` 是各自入 `failures` 的两条失败）。
///
/// **顺带钉住另一句**（不另写用例）：`LocalOnly` 那一格填什么都一样（`spec_floor` 覆盖它）——
/// 把 `rules_all_any()` 换成一份 `LocalOnly → TrustedPersonalOnly` 的表，**本用例的结果一字不变**。
#[test]
fn the_gate_cannot_be_widened_by_any_caller_table() {
    let policy = RecordingPolicy::new(rules_all_any());
    let mut failures: Vec<String> = Vec::new();

    // (a) 只有云节点。
    let cloud_only = vec![cloud("a")];
    for &level in PrivacyClass::ALL.iter() {
        let artifacts = vec![artifact(level)];
        let request = PlacementRequest {
            artifacts: &artifacts,
            nodes: &cloud_only,
        };
        match (level, place(&request, &policy)) {
            (PrivacyClass::LocalOnly, Err(PlacementError::NoPlaceableNode)) => {}
            (PrivacyClass::LocalOnly, Err(PlacementError::DuplicateNode { id })) => {
                failures.push(format!("(a) LocalOnly：判成了 DuplicateNode {{ id: {id:?} }}"))
            }
            (PrivacyClass::LocalOnly, Ok(n)) => failures.push(format!(
                "(a) LocalOnly：过了闸门（返回 {:?}）—— 五档全 AnyNode 也不该放宽它",
                n.id()
            )),
            (_, Ok(n)) if n.id() == &ComputeNodeId::new("a") => {}
            (_, Ok(n)) => failures.push(format!(
                "(a) {level:?}：Ok 但返回的是 {:?}，不是那枚云节点",
                n.id()
            )),
            (_, Err(err)) => failures.push(format!(
                "(a) {level:?}：Err({err:?})—— 五档全 AnyNode 下这一档该放行"
            )),
        }
    }

    // (b) 云 ＋ 桌面。
    let mixed = vec![cloud("a"), desktop("b")];
    for &level in PrivacyClass::ALL.iter() {
        let artifacts = vec![artifact(level)];
        let request = PlacementRequest {
            artifacts: &artifacts,
            nodes: &mixed,
        };
        let got = place(&request, &policy);
        if level == PrivacyClass::LocalOnly {
            match got {
                Ok(n) => {
                    if n.class() != NodeClass::Personal {
                        failures.push(format!("(b) LocalOnly：返回的节点 class 是 {:?}", n.class()));
                    }
                    if n.trust() != NodeTrust::TrustedPersonal {
                        failures.push(format!("(b) LocalOnly：返回的节点 trust 是 {:?}", n.trust()));
                    }
                }
                Err(err) => failures.push(format!(
                    "(b) LocalOnly：Err({err:?})—— 桌面节点在场，该放得下"
                )),
            }
        } else {
            match got {
                Ok(n) => {
                    let id = n.id();
                    if id != &ComputeNodeId::new("a") && id != &ComputeNodeId::new("b") {
                        failures.push(format!("(b) {level:?}：返回的 id {id:?} 不在两枚之内"));
                    }
                }
                Err(err) => failures.push(format!(
                    "(b) {level:?}：Err({err:?})—— 五档全 AnyNode 下这一档该放行"
                )),
            }
        }
    }

    assert!(
        failures.is_empty(),
        "闸门被五档全 AnyNode 的表放宽了（{} 处）：{failures:?}",
        failures.len()
    );
}

/// **四种 `class` × `trust` 组合逐项，不抽代表**。
///
/// `NodeClass::{Personal, Temporary}` × `NodeTrust::{TrustedPersonal, OutsidePersonalTrustDomain}`
/// 四枚节点，**四枚的 id 各不相同**（若四枚同 id，会先撞上步骤 1 的判重），
/// 逐个作为**唯一的**节点（每次 `nodes` 只有一枚），配 `rules_all_any()` 与 `[artifact(LocalOnly)]`：
/// 只有 `(Personal, TrustedPersonal)` 那一次 `Ok`，**另外三次各断一次 `Err(NoPlaceableNode)`**。
///
/// **判据**：这四枚就是二值 × 二值的**全部**组合（2 × 2 = 4）。
/// （设计 §9 那一格把数目写成过「五种」，**已在该格内订正为四种**、订正注记原话照留；该处的来历见 Task 5 报告「计划／派单缺陷」第 2 条。）
///
/// **四次断言分开写**，因为两处宽松变异**各自只红一格**：
/// **红的条件（档位：放宽）**——把闸门的判据改成 `class == Personal`（不看 `trust`）→
/// `(Personal, Outside)` 那一格红；改成只看 `trust` → `(Temporary, TrustedPersonal)` 那一格红。
///
/// # 另外三格为什么**累积**进 `Vec<String>` 再一次性断言（Task 6 改）
///
/// 初稿是「每格 `panic!` 一次」。**那个写法让第三格（`Temporary, Outside`）永远拿不到照片**：
/// 同一族的变异体都只放过第 1 格或只放过第 2 格，而 `panic!` 在第 1 格就把用例打断了
/// ——于是「第三格自己会红吗」这件事**在本文件里从来没有被观测过**。
/// 改成累积后，一次跑就能读出**三格各自的**结果（与 `the_gate_cannot_be_widened_by_any_caller_table`
/// 的 `failures: Vec<String>` 同体例）。
///
/// **第三格单独进 `failures` 的那枚变异体**：把闸门判据里那个合取号 **`&&` 改成 `==`**
/// （单 token；`bool == bool` 编得过）：
///
/// ```text
/// !(node.class() == NodeClass::Personal && node.trust() == NodeTrust::TrustedPersonal)
/// !(node.class() == NodeClass::Personal == (node.trust() == NodeTrust::TrustedPersonal))
/// ```
///
/// 判据变成「两个 `==` 同真同假」，于是**放行集是 `(Personal, Trusted)` 与 `(Temporary, Outside)`**
/// ——`(Personal, Outside)` 与 `(Temporary, Trusted)` 仍被拒，正向那一格也仍放行
/// （故那条 `expect` 不会先把用例打断）。**第三格因此第一次单独进 `failures`**。
///
/// **实测的报文**（`failures.len() == 1`，逐字）：
///
/// ```text
/// LocalOnly 的闸门放过了不该放的组合（1 格）：
/// ["(Temporary, OutsidePersonalTrustDomain) 不该过 LocalOnly 的闸门，却返回了 ComputeNodeId(\"combo-temporary-outside\")"]
/// ```
///
/// **为什么非这一枚不可**：本用例要的是第三格**单独**进 `failures`。上面那两条「只看 `class`」／
/// 「只看 `trust`」的变异体（M6／M7）**实测各自只让一格进 `failures`——但分别是第 1 格与第 2 格**
/// （报文里的 id 是 `combo-personal-outside` 与 `combo-temporary-trusted`），
/// **第三格在它们下面仍被掩蔽**。把合取号改成 `==` 是本轮**找到的那一枚**让第 3 格单独进 `failures` 的
/// ——**不是「唯一可能存在的一枚」**（本文件没有做穷举）。
/// **逐枚的源码改写与 sha256 见 `.superpowers/sdd-p3e-impl/task-6-report.md`。**
#[test]
fn four_class_trust_combinations_and_only_one_passes() {
    let policy = RecordingPolicy::new(rules_all_any());
    let artifacts = vec![artifact(PrivacyClass::LocalOnly)];

    // 正向那一格：唯一该放行的组合。
    let trusted_personal = vec![desktop("combo-personal-trusted")];
    let request = PlacementRequest {
        artifacts: &artifacts,
        nodes: &trusted_personal,
    };
    let placed = place(&request, &policy)
        .expect("(Personal, TrustedPersonal) 是唯一该过 LocalOnly 闸门的组合");
    assert_eq!(placed.id(), &ComputeNodeId::new("combo-personal-trusted"));
    assert_eq!(placed.class(), NodeClass::Personal);
    assert_eq!(placed.trust(), NodeTrust::TrustedPersonal);

    // 另外三格，逐格一条断言——**三格的失败都累积进 `failures`，最后一次性断言**，
    // 故「哪几格红了」在一次跑里全部可读（理由见本用例的文档注释）。
    let mut failures: Vec<String> = Vec::new();
    for (id, class, trust) in [
        (
            "combo-personal-outside",
            NodeClass::Personal,
            NodeTrust::OutsidePersonalTrustDomain,
        ),
        (
            "combo-temporary-trusted",
            NodeClass::Temporary,
            NodeTrust::TrustedPersonal,
        ),
        (
            "combo-temporary-outside",
            NodeClass::Temporary,
            NodeTrust::OutsidePersonalTrustDomain,
        ),
    ] {
        let nodes = vec![node(id, class, trust, Vec::new())];
        let request = PlacementRequest {
            artifacts: &artifacts,
            nodes: &nodes,
        };
        match place(&request, &policy) {
            Ok(n) => failures.push(format!(
                "({class:?}, {trust:?}) 不该过 LocalOnly 的闸门，却返回了 {:?}",
                n.id()
            )),
            Err(PlacementError::NoPlaceableNode) => {}
            Err(err) => failures.push(format!(
                "({class:?}, {trust:?}) 该判 NoPlaceableNode，实际是 {err:?}"
            )),
        }
    }
    assert!(
        failures.is_empty(),
        "LocalOnly 的闸门放过了不该放的组合（{} 格）：{failures:?}",
        failures.len()
    );
}

/// `nodes` 为空 → `Err(NoPlaceableNode)`（设计与 §5.8 的变体文档：**含 `nodes` 为空的情形**）。
///
/// **红在哪条断言**（实测 M8 落在 `src/placement.rs:419:10`）：**被调方**的 `Option::unwrap()` panic。
/// **红的条件（档位：移除）**：删掉判空那一步 → 该条以 **panic** 失败（对空切片取头）。
/// **那不是干净的红，但确实是红**，照实记（本项目的先例：G 的截止用例也是「红形态就是 panic」）。
#[test]
fn an_empty_node_set_is_unplaceable() {
    let nodes: Vec<ComputeNode> = Vec::new();
    let artifacts = vec![artifact(PrivacyClass::Public)];
    let request = PlacementRequest {
        artifacts: &artifacts,
        nodes: &nodes,
    };
    let policy = RecordingPolicy::new(rules_all_any());

    let err = place(&request, &policy).unwrap_err();
    assert!(
        matches!(err, PlacementError::NoPlaceableNode),
        "空节点集该判 NoPlaceableNode，实际是 {err:?}"
    );
}

/// 同一个 `ComputeNodeId` 出现两次 → `Err(DuplicateNode { id })`，**并断言是哪一枚**。
///
/// **夹具里两枚同 id、两枚不同 `class`**：这样「判重」与「过滤」两件事分得开
/// （若两枚完全一样，`Err` 也可能来自别处）。
///
/// **判重的次序是承重的**：它是 §5.9「确定」那条断言的前提，不是结尾的卫生检查（设计 §5.2）。
///
/// **红在哪条断言**：`assert_eq!(id, ComputeNodeId::new("a"))` 那一条
/// （若实现返回的是 `Ok`，则先在 `unwrap_err` 上 panic——那也仍是这一条用例红）。
/// **红的条件（档位：移除）**：删掉步骤 1 → 返回 `Ok`（或返回其中一枚），红。
#[test]
fn duplicate_node_ids_are_a_named_error() {
    let nodes = vec![desktop("a"), cloud("a")];
    let artifacts = vec![artifact(PrivacyClass::LocalOnly)];
    let request = PlacementRequest {
        artifacts: &artifacts,
        nodes: &nodes,
    };
    let policy = RecordingPolicy::new(rules_all_any());

    let err = place(&request, &policy).unwrap_err();
    let id = match err {
        PlacementError::DuplicateNode { id } => id,
        other => panic!("两枚同 id 该判 DuplicateNode，实际是 {other:?}"),
    };
    assert_eq!(
        id,
        ComputeNodeId::new("a"),
        "报出来的该是重复的那一枚 id"
    );
}

/// 制品集为空 → **不过滤任何节点**（`LocalOnly` 的判据不适用：没有制品要保护）。
///
/// **它只钉「不过滤」这一半**，故夹具里只有一枚节点、Ordering 无关
/// ——设计 §9 的「无制品」那一格的后一半（**返回兜底档选中的那个**）由
/// [`the_id_breaks_ties_when_the_policy_says_equal`] 的**第二个路径**承载
/// （多枚节点；那一条原先是独立的一条用例，Task 6 因两处变异等价而把它合了进去，见那里的说明）。
///
/// **红在哪条断言**（实测 M10 落在 `:668:43`）：`expect(…)`——**该用例的第一条断言就是「必须返回 `Ok`」**。
/// **红的条件（档位：收紧）**：把空制品集当成「一律拒」（或当成 `LocalOnly`）→ 红。
#[test]
fn an_empty_artifact_set_filters_nothing() {
    let nodes = vec![cloud("a")];
    let artifacts: Vec<Artifact> = Vec::new();
    let request = PlacementRequest {
        artifacts: &artifacts,
        nodes: &nodes,
    };
    let policy = RecordingPolicy::new(rules_all_any());

    let placed = place(&request, &policy).expect("空制品集不该过滤掉任何节点");
    assert_eq!(
        placed.id(),
        &ComputeNodeId::new("a"),
        "空制品集下该返回唯一那枚节点"
    );
}

/// **反侧照片**（设计 §6.3；`docs/superpowers/2026-10-06-p3e-decisions.md` 第一节的配套要求 3）：
/// **只给 `artifacts` 与 `nodes`** 就把 `PlacementRequest` 建出来并调用 `place`
/// ——**不调 `rank`、不构造任何候选集**。
///
/// # 证明力边界（写进用例注释，照实）
///
/// 本 crate 对 `continuum-model-registry` **零依赖**，故这份样例**在类型上就写不出 `rank`**
/// ——它是**下界**，不是封闭判定。**封闭的那一层是 Task 1 的 `ALLOWED` 逐对断言**
/// （裁定义务 5 的同一条）。**本用例不重复那些闸门断言**：它只断言「请求建得出来、`place` 跑得通」。
///
/// 判据出处：`p3bcdf-followups.md` §四.4 判「**否定式照片在本仓可接受**」。
#[test]
fn a_placement_request_is_built_from_artifacts_and_nodes_alone() {
    let nodes = vec![desktop("b")];
    let artifacts = vec![artifact(PrivacyClass::LocalOnly)];

    let request = PlacementRequest {
        artifacts: &artifacts,
        nodes: &nodes,
    };
    let policy = RecordingPolicy::new(rules_all_any());

    let placed = place(&request, &policy).expect("只给制品与节点建出的请求该跑得通");
    assert_eq!(placed.id(), &ComputeNodeId::new("b"));
}

// ── 以下三条是 Task 6 的（`place` 的步骤 4–5：排序与取头）──────────────────────
//
// 夹具统一是 [`three_nodes_in_order`]／[`fixture_node`] 那一组三枚节点
// （`"a"` 0 个标签、`"b"` 1 个、`"c"` 3 个，`class` / `trust` 都是 `desktop`），
// 制品统一是 `Public`（三枚都过闸门）＋ `rules_all_any()`。**id 与标签数反向**是这组的承重点。

/// **确定性**（设计 §5.9；§9 的「确定性」那一格）：同一组节点按**三种不同顺序**放进 `nodes`，
/// 三次 `place` 返回**同一枚**。
///
/// 策略取 [`ByCapabilityCount`]（**降序**，标签多者在前），故正确实现的赢家是 `"c"`
/// ——它的 id **最大**而排在**第一**。**这正是本用例的承重点**：策略的序与 id 序相反，
/// 于是「排序读了策略」与「只是按 id」两件事才分得开。
///
/// **红在哪条断言**：`winners[0] != winners[1]` 那一条（第一条判三次同解）。
/// **红的条件（档位：移除）**：删掉步骤 4 的 `sort_by` 那一行 → `placeable` 保留输入序，
/// 三次分别返回 `"a"` / `"c"` / `"b"`，第 1 次与第 2 次就分了家，红。
///
/// **期望值 `"c"` 与 [`ByCapabilityCount::compare`] 的方向是一对**：
/// 把方向改成升序（或把夹具的标签数配反）必须**同时**改这一处，
/// 否则 [`swapping_the_policy_changes_the_result`] 的「两次不同」也会一并失去意义。
#[test]
fn the_same_node_set_in_any_order_yields_the_same_node() {
    let policy = ByCapabilityCount::new(rules_all_any());
    let artifacts = vec![artifact(PrivacyClass::Public)];

    let mut winners: Vec<ComputeNodeId> = Vec::new();
    for ids in [["a", "b", "c"], ["c", "b", "a"], ["b", "c", "a"]] {
        let nodes = three_nodes_in_order(ids);
        let request = PlacementRequest {
            artifacts: &artifacts,
            nodes: &nodes,
        };
        let placed = place(&request, &policy).expect("三枚都是 desktop 且制品是 Public，该放得下");
        winners.push(placed.id().clone());
    }

    assert_eq!(winners.len(), 3, "三次调用都该有结果");
    assert_eq!(
        winners[0], winners[1],
        "同一组节点换了顺序，返回的却不是同一枚（第 1 次 vs 第 2 次）"
    );
    assert_eq!(
        winners[1], winners[2],
        "同一组节点换了顺序，返回的却不是同一枚（第 2 次 vs 第 3 次）"
    );
    assert_eq!(
        winners[0],
        ComputeNodeId::new("c"),
        "该返回策略的序所指的那一枚（标签最多者）"
    );
}

/// **兜底档**（设计 §5.9 的 `ComputeNodeId` 升序）：策略恒 `Equal` 时由 id 决定次序。
///
/// 夹具：[`three_nodes_in_order`]`(["c", "a", "b"])`——**输入的第一个元素不是 id 最小的那一枚**
/// （这是本用例的承重点，否则「排了序」与「没排序」不可区分），配 [`BaselinePlacementPolicy`]。
///
/// **红在哪条断言**：循环里那一条 `assert_eq!(placed.id(), &ComputeNodeId::new("a"))`。
/// **红的条件**：两条，各自一击即红——
///
/// - **（档位：移除）** 删掉 `.then_with(|| a.id().cmp(b.id()))` → `sort_by` 是**稳定**排序，
///   全 `Equal` 时保留输入序，返回 `"c"`；
/// - **（档位：取反）** 把兜底档反过来（`b.id().cmp(a.id())`）→ 返回 `"c"`。
///
/// # 两条路径合进这一条用例（Task 6 合并，理由与证据照实记）
///
/// 计划给的另一条 `an_empty_artifact_set_returns_the_tiebreak_winner`（**§9「无制品」那一行的后一半**）
/// 与这一条**只有制品集不同**：一条走**经闸门**的路径（制品 `Public`，三枚都过），
/// 一条走**不经闸门**的路径（`artifacts = []`，步骤 2 一枚都不滤）。
/// 计划预先写明：「若实现下来发现两处的变异完全等价（**同一行代码、同一组入参**），
/// 据实合并成一条并记在报告里」。
///
/// **实测正是如此**：删掉 `.then_with(|| a.id().cmp(b.id()))` 那一枚变异体下，两条用例**同时红**，
/// 且红在同一行 `tests/placement.rs:811`；两者交给 `sort_by` 的 `placeable` **是同一个数组**
/// （三枚 `desktop` 按 `"c"` / `"a"` / `"b"` 的顺序，过滤路径在这组夹具上一枚都不滤）。
/// 此外删掉整个 `sort_by`、把兜底档取反、以及把 `Ord` 改成按长度比，
/// **三枚变异体也都让两条一起红**。
///
/// # 「两条从未分开过」这句过宽（2026-10-07 复核订正，**旧说法原话照留**）
///
/// 初稿在这里写的是「……三枚变异体也都让两条一起红——**两条从未分开过，独立证据为零**」。
/// **那句为假**：它把「排序那一行上的变异体分不开这两条」读成了「任何变异体都分不开」。
/// 反例是 `M10`（把空制品集当成「一律拒」，档位**收紧**）：在交付字节上把被合并的那条按计划
/// 原文复原成独立 `#[test]` 后实跑，**被合并的那条红、本条绿**——两条**分得开**。
/// **诚实的分界**：分开它们的那枚变异体落在**空制品集那一侧**，不在排序那一侧；
/// **排序那一行**确实分不开（同一行代码、同一组入参，两条交给 `sort_by` 的是同一个数组）
/// ——后面这句才是当年合并的依据。
/// 另记：被合并的那条**没有被吞掉**（它作为本条的第二个路径逐字执行并断言），
/// 且空制品集那一侧的观测点另有 [`an_empty_artifact_set_filters_nothing`]（`M10` 的红集里就有它）。
///
/// **故合并**：下面那个循环的两个元素就是那两条路径，§9「无制品」那一行的后一半由此条承载。
/// **合并的代价照实记**：`an_empty_artifact_set_returns_the_tiebreak_winner` 这个名字从本文件消失，
/// 计划 Task 7 Step 5 的表按名字引的是本条，不受影响；设计 §9「无制品」那一格则改为由
/// 本条 ＋ [`an_empty_artifact_set_filters_nothing`] 两条合起来覆盖。
#[test]
fn the_id_breaks_ties_when_the_policy_says_equal() {
    let policy = BaselinePlacementPolicy::new(rules_all_any());
    let public = vec![artifact(PrivacyClass::Public)];
    let empty: Vec<Artifact> = Vec::new();

    for (label, artifacts) in [("制品非空（Public）", &public), ("制品集为空", &empty)] {
        let nodes = three_nodes_in_order(["c", "a", "b"]);
        let request = PlacementRequest {
            artifacts: artifacts.as_slice(),
            nodes: &nodes,
        };

        let placed = place(&request, &policy)
            .unwrap_or_else(|err| panic!("{label}：三枚都是 desktop，该放得下，实际是 {err:?}"));
        assert_eq!(
            placed.id(),
            &ComputeNodeId::new("a"),
            "{label}：策略全 Equal 时，兜底档该给出 ComputeNodeId 升序最小的那一枚"
        );
    }
}

/// **策略可替换**（设计 §5.5、§9 的「策略可替换」那一格）：同一组节点、同一组制品，
/// 换一份策略 → 换一个结果。
///
/// 两次调用只差 `policy` 这一个实参：[`BaselinePlacementPolicy`]（全 `Equal`，
/// 由兜底档给出 id 最小者 `"a"`）与 [`ByCapabilityCount`]（降序，标签最多者 `"c"`）。
///
/// **它钉的是「排序真的读策略」，不钉「哪个策略对」**（§9 那一行的口径逐字如此）：
/// 断言里没有一句说「标签多者更该被选中」。
///
/// **红在哪条断言**：`assert_ne!(…)` 那一条。
/// **红的条件（档位：移除）**：把 `place` 里的 `policy.compare(a, b)` 换成 `Ordering::Equal`
/// （即不读策略）→ 两次都返回 `"a"`，红。
#[test]
fn swapping_the_policy_changes_the_result() {
    let nodes = three_nodes_in_order(["a", "b", "c"]);
    let artifacts = vec![artifact(PrivacyClass::Public)];
    let request = PlacementRequest {
        artifacts: &artifacts,
        nodes: &nodes,
    };
    let baseline = BaselinePlacementPolicy::new(rules_all_any());
    let by_capability_count = ByCapabilityCount::new(rules_all_any());

    let with_baseline =
        place(&request, &baseline).expect("三枚都是 desktop 且制品是 Public，该放得下");
    let with_capability_count =
        place(&request, &by_capability_count).expect("同上，换一份策略不该改变可放置性");

    assert_ne!(
        with_baseline.id(),
        with_capability_count.id(),
        "换一份策略后结果该变——不变说明 place 没读策略的 compare"
    );
    assert_eq!(
        with_baseline.id(),
        &ComputeNodeId::new("a"),
        "基线策略全 Equal，该由兜底档给出 id 最小的那一枚"
    );
    assert_eq!(
        with_capability_count.id(),
        &ComputeNodeId::new("c"),
        "标签最多者该胜出"
    );
}
