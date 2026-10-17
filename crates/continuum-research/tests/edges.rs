//! §4.5 的边表与端口类型的可解析性（§9.2 的 (l)(m)）。
//!
//! (l) 的**边表手写**（设计 §4.5 的表逐行照录），端口取值才从被测的
//! [`all_operators`] 读；(m) 的语料就是七枚的端口取值本身。
//! 两条的**作用域都是枚举式的**：各条的红条件、锚点与「照实」说明见下方
//! 该用例的文档注释——把任一条读成全称都是误读（纪律 4）。
//!
//! 两条用例所测的实现体（`src/operators.rs` 的七行端口取值）**已在 Task 1 落地**
//! ⇒ 本文件首跑**预期绿**，不是「先跑红再实现」那一步；两条红条件的兑现
//! **不在本 task**，在 Task 7 Step 5 的变异轮：放宽档是那一轮的 (iv)、
//! 收紧档是 (v)，(m) 的跨 crate 取反档按跨 crate 承重那一档另跑。

use continuum_artifact::ArtifactType;
use continuum_research::all_operators;

/// §9.2 第 (l) 条：§4.5 边表**逐行**——下游 `input_schema` 的每一型都应由
/// 该行列出的某枚上游的 `output_schema` 供应。
///
/// **边表手写**（设计 §4.5 的表，七行逐行照录）：逐行给「下游 id → 来源 id 的列表」。
/// 断言**逐行**：下游 `input_schema` 的**每一个型**都能在该行列出的上游里找到，
/// 且所引的那一枚的 `output_schema` 含该型。
/// **断言的对象是「交集中的型」，不是「两侧相等」**——第 7 行的 `input_schema` 是
/// `[Report, Json]`，两个来源各供一枚，两侧不相等。
///
/// 红条件（**放宽档**）：把第 1 行 `research-question` 的 `input_schema` 由 `[]` 改成
/// `[Text]`（锚点：`research-question` 那一枚的 `input_schema` 字段值——空数组 `[]`
/// 在七枚里只出现**一次**，故唯一）⇒ 该行「无上游」而下游要一个型 ⇒ 本用例红。
/// 红条件（**收紧档**）：把第 7 行 `research-citation-verification` 的 `input_schema`
/// 里的 `Report` 删掉（只留 `Json`）（锚点：**`research-citation-verification` 那一枚
/// 的 `input_schema` 行**里那一枚 `Report`——`Report` 在七枚里出现**两处**，另一处是
/// 第 6 行 `research-synthesis` 的 `output_schema`，故要靠算子 id 定位）
/// ⇒ Task 1 的 (b) 红，而**本用例的第 7 行变绿**（`Json` 在上游找得到）
/// ⇒ **故 (l) 单独不足以钉住这一格，(b) 与 (l) 要一起看**（设计 §9.3 已列此条）。
///
/// **并写明本条的作用域（照实，不许写成全称）**：
/// **它只钉住枚举到的这七行**；其中**第 2–6 行（五行）落在 `Json`↔`Json` 上、
/// 无区分力**（§3.2 的代价：`SourceSet` 未收成一型），它只能拒掉「把 `Report`
/// 接到只收 `Json` 的端口」这一类错。**「§330 的七步之间的顺序」本块表达不出**
/// （`Operator` 无前驱字段，设计 §4.3 末）——本用例的断言**弱于顺序**：
/// `Json → Json` 的三条边在类型上无法与「把链倒过来接」相区分。
#[test]
fn every_downstream_input_type_is_supplied_by_a_listed_upstream() {
    // 设计 §4.5 的边表，七行逐行照录：(下游 id, 该行来源 id 的列表)。
    // 第 1 行**无来源**——问题来自 `Intent`，不是 `Artifact`（设计 §4.4 第 1 条）。
    // 语料手写：边表不从 `all_operators()` 推出（端口取值才从被测清单读）。
    let edges: [(&str, &[&str]); 7] = [
        // 设计 §4.5 第 1 行
        ("research-question", &[]),
        // 设计 §4.5 第 2 行
        ("research-search", &["research-question"]),
        // 设计 §4.5 第 3 行
        ("research-source-set", &["research-search"]),
        // 设计 §4.5 第 4 行
        ("research-evidence-extraction", &["research-source-set"]),
        // 设计 §4.5 第 5 行
        (
            "research-contradiction-check",
            &["research-evidence-extraction"],
        ),
        // 设计 §4.5 第 6 行
        (
            "research-synthesis",
            &["research-evidence-extraction", "research-contradiction-check"],
        ),
        // 设计 §4.5 第 7 行
        (
            "research-citation-verification",
            &["research-synthesis", "research-evidence-extraction"],
        ),
    ];

    let operators = all_operators();

    for (i, (downstream_id, upstream_ids)) in edges.into_iter().enumerate() {
        let row = i + 1;
        let downstream = operators
            .iter()
            .find(|op| op.id.as_str() == downstream_id)
            .unwrap_or_else(|| panic!("第 {row} 行的下游 {downstream_id} 不在七枚清单里"));

        for ty in &downstream.input_schema {
            // 该型必须在这一行列出的**某一枚**上游的 output_schema 里（交集中的型）：
            // 两侧不相等不算红——第 7 行就是两个来源各供一枚。
            let supplied = upstream_ids.iter().any(|upstream_id| {
                let upstream = operators
                    .iter()
                    .find(|op| op.id.as_str() == *upstream_id)
                    .unwrap_or_else(|| panic!("第 {row} 行的来源 {upstream_id} 不在七枚清单里"));
                upstream.output_schema.contains(ty)
            });
            assert!(
                supplied,
                "第 {row} 行（{downstream_id}）：`input_schema` 里的 {ty:?} 应由该行列出的 \
                 来源之一的 `output_schema` 供应；该行的来源是 {upstream_ids:?}"
            );
        }
    }
}

/// §9.2 第 (m) 条：七枚的每个端口类型都能被 `ArtifactType` 解回——对每个
/// `input_schema` / `output_schema` 里的型 `t`，断言
/// `ArtifactType::parse(t.as_str()).is_some()`。
///
/// 红条件（**取反档，跨 crate**）：删掉 `crates/continuum-artifact/src/artifact.rs`
/// 的 `parse` 里 `"report"` 那一臂（锚点：该文件 `parse` 本体里 `"report"` 那一臂——
/// **是 `parse` 里那条，不是 `as_str` 里那条**；**按内容指，不写行号范围**：
/// P5e 在本块之前改这个文件）⇒ `Report` 的 `as_str` 取回 `"report"`、
/// `parse` 走通配臂返回 `None` ⇒ 本用例红。**兑现不在本轮**：那一档在 Task 7
/// Step 5 的变异轮里按跨 crate 承重那一档另跑。
///
/// **并写明本条的实际强度（据实，见实现计划 `## 遗留` 第一节第 5 条）**：
/// **它不是恒真的，但与 P1 的往返用例高度重叠**——端口类型的取值必在
/// `ArtifactType::ALL` 里，而 `parse ∘ as_str == id` 由 P1 的
/// `every_artifact_type_round_trips_through_its_encoding` 把守；
/// **它独立的那一格**恰是 P1 遍历不到的：`parse` 带通配臂、`ALL` 的完整性无守卫，
/// 故「新变体进了枚举与 `as_str`、未进 `parse`」这一状态本 crate 编得过、
/// P1 的门也绿，而本条红（这也说明它**不是**「本 crate 能编译」的推论）。
/// 故**不许**把它写成「本块声明了两枚正确的型」的全称。
/// **这一处的标签曾写作「恒真」，2026-10-10 复核订正为上述，原文与理由留在
/// 实现计划 `## 遗留` 第一节第 5 条。**
#[test]
fn every_declared_port_type_is_a_known_artifact_type() {
    for (i, op) in all_operators().iter().enumerate() {
        for ty in op.input_schema.iter().chain(op.output_schema.iter()) {
            assert!(
                ArtifactType::parse(ty.as_str()).is_some(),
                "第 {} 枚（{}）的端口类型 {} 应能被 ArtifactType::parse 解析（§9.2 第 (m) 条）",
                i + 1,
                op.id,
                ty.as_str()
            );
        }
    }
}
