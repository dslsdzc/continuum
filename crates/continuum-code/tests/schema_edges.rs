//! 设计 §3.4 的边表（§10.2 第 (j) 条）：§239「运行时 MUST 拒绝不兼容连接」的注册期一半。
//!
//! **形态：两组断言，各钉设计 §3.4 判据的一侧——这里不是「一份三列语料同时钉两侧」**：
//! 三列语料只覆盖它写到的那些型，**语料之外的取值两侧都是 fail-open**（给某枚下游算子
//! 加一个语料里没列过的型，三列语料全绿）。故把两半各自交给一条**独立**的断言：
//!
//! - **（A）下游覆盖**——手写「有上游算子的下游 → §3.4 表为该行列出的型集合」的表，
//!   逐行断言两件事：
//!   （A1）表里列出的**每个**型都在该下游的 `input_schema` 里；
//!   （A2）该下游的 `input_schema` 里**每个**型都在表里列出——两个方向都判，合起来是集合相等。
//!   **（A2）承担的就是「下游的每一个型都能找到上游」这一半**：下游若多出一个语料外的型，
//!   （A2）红，而在 (B) 里那一型根本没有对应的行——两处一起把「下游有孤儿型」逼出来。
//! - **（B）上游来源**——手写三列语料 `(下游算子 id, 上游算子 id, 型)`，逐行对应设计 §3.4
//!   的边表，逐行断言该上游的 `output_schema` 含该型。
//!
//! （A）＋（B）就是设计 §3.4 的判据：下游的每一型都在 (A) 的表里（(A2) 的后一个方向），
//! 表里的每一型在 (B) 里都有一枚上游的输出含它。**这不是把设计判据改写**——设计写的就是
//! 两半合起来的那一句，本用例只是**把两半各自交给一条独立的断言**：三列语料是 (B) 那一半，
//! 它**不能**同时充当 (A)，否则下游侧只剩「语料枚举到的那几个型」，正是 fail-open 的那一侧。
//!
//! **三条红条件，各指一个唯一锚点**（分别是 `src/operators.rs` 里 §3.2 表第 5、7、3 行的
//! 端口字段）：
//!
//! - **下游侧·删型**：把 `code-spec-review` 的 `input_schema` 改成 `[Patch]`（去掉 `Json`）
//!   （**取反档**）⇒（A1）红（表里列了 `Json` 而下游没有）。**这一条正是设计 §10.3 预告的
//!   那一行**；设计 §10.3 已按「两侧都钉」订正：该变异下「下游的每一型都要能找到上游」
//!   （本用例的 (A2)）**仍绿**（删掉一枚型只会让它更易满足），能让它红的只有 (A1)。
//! - **下游侧·增型**：给 `code-test-run` 的 `input_schema` 加一个 `ArtifactType::Json`
//!   （**取反档**）⇒（A2）红。**这一条是三列语料单独用时会漏掉的那一侧**——把它写进红条件，
//!   才说明 (A) 不是摆设。
//! - **上游侧**：把 `code-decompose` 的 `output_schema` 改成 `[Patch]`（**取反档**）
//!   ⇒（B）里 `(code-implement, code-decompose, Json)` 那一行红（该枚作为上游的其余两行
//!   ——`code-spec-review` 与 `code-quality-review` ← `code-decompose`（`Json`）——同样红）。
//!
//! **这条用例钉不住什么（写进注释）**：它只判**下游的每一型都有一枚上游的输出含它**，
//! **不判**「图上的边该不该存在」（那是 Planner 的构造），也**不判**「两侧的 schema 相等」；
//! **不替代** P1 的 `compatible`（`crates/continuum-port/src/port.rs:94`）——本块不写第二份
//! 那类判定（切分 §四第 7 条）。

use continuum_artifact::ArtifactType;
use continuum_code::all_operators;
use continuum_operator::Operator;

/// 在 `all_operators()` 里按 `id` **线性查找**；**找不到即 `panic!` 并打印那个串**。
///
/// 形态写死，**不许写成「找不到就跳过」**：那样 Task 1 的 id 变异（把 §3.2 表第 5、6 行的
/// id 改成同一个）会让本用例**静默通过**；写成 panic 之后，那一处变异在本用例的红
/// 是「找不到 `code-quality-review`」这一条 panic——**红的位置可解释**。
/// Task 7 的 `l`／`m` 也按**串**指认算子，用**同形**的查找。
fn operator_by_id(id: &str) -> Operator {
    all_operators()
        .into_iter()
        .find(|op| op.id.as_str() == id)
        .unwrap_or_else(|| panic!("all_operators() 里找不到 id 为 `{id}` 的算子"))
}

/// 设计 §10.2 第 (j) 条。
///
/// **（A2）比的是「下游的 `input_schema`」与「设计 §3.4 表为该行列出的型」，不是「下游的
/// 输入与上游的输出两侧相等」**——后者是设计 §3.4 末与 P5e 设计 §5.3 已否掉的那条读法
/// （例如 `code-spec-review` 的输入是 `{Json, Patch}`，而它的两枚上游的输出分别是 `{Json}`
/// 与 `{Patch}`，两侧并不相等）。本用例断言的始终是「交集中的型」，即 (B) 的逐行含型判定。
#[test]
fn every_declared_edge_has_the_type_on_both_sides() {
    // （A）下游覆盖：语料＝「该下游在设计 §3.4 表里列出的型集合」。**手写**，逐行取自
    // §3.4 的边表；行序即 §3.4 表的行序（`code-plan` 一行除外，理由见下）。
    //
    // **依据栏的读法照设计 §3.4**：语料里写「§186 的 `Implementation Plan`→`Engineering
    // Tasks` 两步」这一类的是规范正文的步骤序；写「§3.2 第 N 行」的是**该行的 `input_schema`
    // 与全表 `output_schema` 的对应**（本设计的判定，规范未给边）。
    //
    // **`code-plan` 那一行不入本用例**：它的输入由装配方以 `Json` 值投射、**不由任何算子的
    // `output_schema` 提供**，故它**没有上游算子可指**（设计 §3.4 的「一处例外」）。
    // **若不写明这一点，本用例会被读成判据的反例。**它的上游是 §234 的 `Milestone` 与
    // §228 的 `Plan`（都是 P4 的对象），**以值**由装配方投射。
    let downstream_rows: [(&str, &[ArtifactType]); 8] = [
        // §3.4 表第 1 行：无输入（Base 不在端口上，§3.3 第 2 条）；依据：§3.2 第 1 行。
        // **三元组数为零——明写为零，而不是「不在表里」**：它在本表里、型集合为空集，
        // 且它的输入确实是空的（Task 1 的 (h) 钉后者）；「不在表里」则是漏写。
        ("code-workspace", &[]),
        // §3.4 表第 3 行：上游 `code-plan`（`Json`）；依据：§186 `:62`→`:64`（步骤序）。
        ("code-decompose", &[ArtifactType::Json]),
        // §3.4 表第 4 行：上游 `code-decompose`（`Json`）；依据：§186 `:64`→`:68`。
        ("code-implement", &[ArtifactType::Json]),
        // §3.4 表第 5 行：上游 `code-decompose`（`Json`）、`code-implement`（`Patch`）；
        // 依据：§186 `:68`→`:70`。
        (
            "code-spec-review",
            &[ArtifactType::Json, ArtifactType::Patch],
        ),
        // §3.4 表第 6 行：上游 `code-decompose`（`Json`）、`code-implement`（`Patch`）；
        // 依据：§186 `:68`→`:72`。
        (
            "code-quality-review",
            &[ArtifactType::Json, ArtifactType::Patch],
        ),
        // §3.4 表第 7 行：上游 `code-workspace`（`SourceTree`）、`code-implement`（`Patch`）；
        // 依据：§186 `:74`；§3.2 第 7 行。
        (
            "code-test-run",
            &[ArtifactType::SourceTree, ArtifactType::Patch],
        ),
        // §3.4 表第 8 行：上游 `code-workspace`（`SourceTree`）、`code-implement`（`Patch`）；
        // 依据：§193 `:361`。
        (
            "code-fuzz-run",
            &[ArtifactType::SourceTree, ArtifactType::Patch],
        ),
        // §3.4 表第 9 行：上游 `code-test-run`（`TestResult`）、`code-implement`（`Patch`）；
        // 依据：§186 `:74`；§3.2 第 9 行。
        (
            "code-verify",
            &[ArtifactType::TestResult, ArtifactType::Patch],
        ),
    ];

    for (downstream_id, listed_types) in downstream_rows {
        let downstream = operator_by_id(downstream_id);

        // （A1）表里列出的每个型都在该下游的 `input_schema` 里
        // ——「下游丢掉了表里列出的型」在这里红。
        for t in listed_types {
            assert!(
                downstream.input_schema.contains(t),
                "（A1）{downstream_id} 的 input_schema 应含 §3.4 表为该行列出的型 {t:?}，\
                 实际为 {:?}",
                downstream.input_schema
            );
        }

        // （A2）该下游的 `input_schema` 里每个型都在表里列出
        // ——「下游新增了一枚表外的型」在这里红（该型在 (B) 里也没有对应的行）。
        for t in &downstream.input_schema {
            assert!(
                listed_types.contains(t),
                "（A2）{downstream_id} 的 input_schema 里的型 {t:?} 不在 §3.4 表为该行列出的\
                 集合里——下游多出了一个语料外的型（它在 (B) 里没有对应的行）"
            );
        }
    }

    // （B）上游来源：手写三列语料 `(下游算子 id, 上游算子 id, 型)`，逐行对应设计 §3.4
    // 的边表（行序即表内书写序），逐行断言该上游的 `output_schema` 含该型。
    // 依据栏的读法同 (A) 上方：§186／§193 的是规范正文的步骤序；「§3.2 第 N 行」的是
    // 该行 `input_schema` 与全表 `output_schema` 的对应（本设计的判定，规范未给边）。
    let upstream_rows: [(&str, &str, ArtifactType); 12] = [
        // §186 `:62`→`:64`（`code-plan` 产 `Implementation Plan`）。
        ("code-decompose", "code-plan", ArtifactType::Json),
        // §186 `:64`→`:68`（`code-decompose` 产 `Engineering Tasks`）。
        ("code-implement", "code-decompose", ArtifactType::Json),
        // §186 `:68`→`:70`（两枚上游：实现任务的拆解与补丁本身）。
        ("code-spec-review", "code-decompose", ArtifactType::Json),
        ("code-spec-review", "code-implement", ArtifactType::Patch),
        // §186 `:68`→`:72`（同两枚上游）。
        ("code-quality-review", "code-decompose", ArtifactType::Json),
        ("code-quality-review", "code-implement", ArtifactType::Patch),
        // §186 `:74`；§3.2 第 7 行（工作区提供 `SourceTree`，补丁由实现来）。
        ("code-test-run", "code-workspace", ArtifactType::SourceTree),
        ("code-test-run", "code-implement", ArtifactType::Patch),
        // §193 `:361`（Fuzzing／Property 与跑测试同形：工作区 ＋ 补丁）。
        ("code-fuzz-run", "code-workspace", ArtifactType::SourceTree),
        ("code-fuzz-run", "code-implement", ArtifactType::Patch),
        // §186 `:74`；§3.2 第 9 行（判定器吃 `TestResult`，补丁由实现来）。
        ("code-verify", "code-test-run", ArtifactType::TestResult),
        ("code-verify", "code-implement", ArtifactType::Patch),
    ];

    for (downstream_id, upstream_id, t) in upstream_rows {
        let upstream = operator_by_id(upstream_id);
        assert!(
            upstream.output_schema.contains(&t),
            "（B）语料行（{downstream_id} ← {upstream_id}，型 {t:?}）不成立：\
             {upstream_id} 的 output_schema 应含该型，实际为 {:?}",
            upstream.output_schema
        );
    }
}
