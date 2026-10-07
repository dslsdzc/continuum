//! §5.2–§5.4、§5.8 的运行期照片：请求面、硬闸门、`place` 的前三步与它的两枚失败变体。
//!
//! 设计：`docs/superpowers/specs/2026-10-06-p3e-compute-placement-design.md`。
//!
//! # 本文件的射程（**刻意收窄，不是漏测**）
//!
//! 本 task 只落 `place` 的**步骤 1–3**（判重 → 过闸门 → 判空）；**步骤 4（排序）与兜底档在 Task 6**。
//! 故本文件的每一份夹具都构造成「**过闸门后恰好剩一枚**」或「**次序无关**」：
//!
//! - 凡是需要**多枚可比节点**的断言（打乱顺序三次同解、`compare` 真的改变结果）一律在 Task 6；
//! - `the_gate_cannot_be_widened_by_any_caller_table` 的 (b) 臂里两枚节点都过闸门，
//!   那一臂因此**只断言「返回的 id 在两枚之内」**，不断言具体是哪一枚——「取头」的次序判据属于 Task 6。
//!
//! # 夹具的 id 取法是承重的
//!
//! 两个节点工厂都收 `id` 参数、**不写死**（设计 §9 的判据 §4.4 那一格与判重那一格的 id 要求互相冲突：
//! 前者要 `[cloud, desktop]` 两枚可比，判重要两枚**同 id**，四种组合那一格要**四枚 id 各不相同**）。
//!
//! **判据 §4.4 的正面用例里，云节点取两枚中较小的那个 id**（`cloud("a")` ＋ `desktop("b")`）：
//! 这样**即使 `place` 不排序**，返回的也是云节点，于是「闸门真的过滤了」与「它只是恰好排在前面」
//! 两件事才分得开——该用例因此是**承重的**：过滤若失效，返回的必是云节点。

use std::cmp::Ordering;

use continuum_artifact::{Artifact, ArtifactId, ArtifactType, ContentHash, PrivacyClass};
use continuum_node::{
    place, ComputeNode, ComputeNodeId, NodeClass, NodeTrust, PlacementError, PlacementPolicy,
    PlacementRequest, PlacementRules, TransferRule,
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

/// 任意 `class` × `trust` 组合的节点。**两个具名工厂都建在它上面**——
/// `four_class_trust_combinations_and_only_one_passes` 要的正是四种组合，
/// 而两个具名工厂只覆盖其中两种。
fn node(id: &str, class: NodeClass, trust: NodeTrust) -> ComputeNode {
    ComputeNode::new(
        ComputeNodeId::new(id),
        class,
        trust,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
}

/// 云节点（§5.3 的取法：`class ≠ Personal` 是「云端节点」的超集）。
fn cloud(id: &str) -> ComputeNode {
    node(id, NodeClass::Temporary, NodeTrust::OutsidePersonalTrustDomain)
}

/// 桌面节点：§288 的 Personal Node，且已在 Personal Trust Domain 内（§293 的终态）。
fn desktop(id: &str) -> ComputeNode {
    node(id, NodeClass::Personal, NodeTrust::TrustedPersonal)
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
/// **`compare` 恒 `Equal` 是刻意的**：本 task 没有排序（步骤 4 在 Task 6），
/// 一份「有主张的比较」会引入本文件拍不到的自由度。
/// Task 6 会另加一份真比较的策略。
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
/// **红在哪条断言**（实测 M3 落在 `:180:40`）：`unwrap_err()`——**该用例的第一条断言就是「必须返回 `Err`」**。
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
/// **红在哪条断言**（实测 M4 落在 `:213:43`）：`expect(…)`——**该用例的第一条断言就是「必须返回 `Ok`」**。
/// **红的条件（档位：收紧）**：让闸门对所有等级都返回 `Err`——或只把 `spec_floor` 对那四档
/// 也改成 `TrustedPersonalOnly`。**同一条变异体还会红下面这两处，逐条点名**：
/// `the_gate_cannot_be_widened_by_any_caller_table` 的 **(a) 臂的四个 `AnyNode` 档**
/// （同一情形：`nodes = [cloud("a")]`）；**该用例的 (b) 臂不红**（它是 `[cloud, desktop]`，
/// 变异后 `desktop` 仍在，故那四档仍得 `Ok`）。**不会红的**（免得去查不存在的问题）：
/// `a_caller_supplied_rule_can_tighten_the_gate`（本来就期望 `Err`）、
/// `four_class_trust_combinations_and_only_one_passes`（全走 `LocalOnly`，`spec_floor` 没动）、
/// 两条空制品集的用例（闸门连一档都不看）、两条期望 `Err` 的用例（与该变异体同解），
/// 以及 **Task 6 的三条**（它们的节点是 `desktop(...)`，正是变异后仍放行的那一类）。
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
/// **红在哪条断言**（实测 M1 落在 `:254:40`）：`unwrap_err()`——**该用例的第一条断言就是「必须返回 `Err`」**。
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
///   `trust == TrustedPersonal` **逐项断言**；其余四档 → `Ok`，且返回的 id **在两枚之内**
///   （两枚都允许，故此处不断言具体哪一枚——「取头」的次序判据在 Task 6）。
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
/// （设计 §9 那一格把数目写成过「五种」，**已在该格内订正为四种**；见报告「派单缺陷」一节。）
///
/// **四次断言分开写**，因为两处宽松变异**各自只红一格**：
/// **红的条件（档位：放宽）**——把闸门的判据改成 `class == Personal`（不看 `trust`）→
/// `(Personal, Outside)` 那一格红；改成只看 `trust` → `(Temporary, TrustedPersonal)` 那一格红。
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

    // 另外三格，逐格一条断言。
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
        let nodes = vec![node(id, class, trust)];
        let request = PlacementRequest {
            artifacts: &artifacts,
            nodes: &nodes,
        };
        let err = match place(&request, &policy) {
            Ok(n) => panic!(
                "({class:?}, {trust:?}) 不该过 LocalOnly 的闸门，却返回了 {:?}",
                n.id()
            ),
            Err(err) => err,
        };
        assert!(
            matches!(err, PlacementError::NoPlaceableNode),
            "({class:?}, {trust:?}) 该判 NoPlaceableNode，实际是 {err:?}"
        );
    }
}

/// `nodes` 为空 → `Err(NoPlaceableNode)`（设计与 §5.8 的变体文档：**含 `nodes` 为空的情形**）。
///
/// **红在哪条断言**（实测 M8 落在 `src/placement.rs:388:35`）：**被调方**的 `Option::unwrap()` panic。
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
/// **多枚节点下「返回兜底档选中的那个」这一半在 Task 6**（本 task 没有排序），
/// 故这里只有一枚节点，断言落在它身上。
///
/// **红在哪条断言**（实测 M10 落在 `:503:43`）：`expect(…)`——**该用例的第一条断言就是「必须返回 `Ok`」**。
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
