//! Task 3 的照片集：§3.4 的边表（§11.2 第 (s) 条），即 §239 的**注册期一半**
//! （运行时那一半是 `crates/continuum-port/src/port.rs:94` 的 `compatible`）。
//!
//! **形态：两组断言，各钉 §3.4 判据的一侧**——**这里不是「一份语料同时钉两侧」**：
//! 语料只覆盖它写到的那些型，**语料之外的取值两侧都是 fail-open**（给某枚下游算子加一个
//! 语料里没列过的型，两侧都照绿）。故把两半各自交给一条**独立**的断言
//! （设计那一条用例名 `every_declared_edge_has_the_type_on_both_sides` 在此拆成两条）：
//!
//! - （A）`every_listed_type_is_on_its_downstream_and_each_downstream_type_is_listed`：
//!   **下游覆盖**——手写一份「有上游算子的下游 → 该下游在 §3.4 表里列出的型集合」的表
//!   （设计 `:288-292` 的边表，下游侧），**逐行断言两件事**：
//!   （A1）表里列出的**每个**型都在该下游的 `input_schema` 里；
//!   （A2）该下游的 `input_schema` 里**每个**型都在表里列出——**两个方向都判，合起来是集合相等**。
//! - （B）`every_edge_row_has_an_upstream_that_outputs_its_type`：**上游来源**——手写三列语料
//!   `(下游算子 id, 上游算子 id, 型)`，逐行对应设计 §3.4 的边表，**逐行断言该上游的
//!   `output_schema` 含该型**。
//!
//! **（A）＋（B）就是设计 §3.4 的判据**：下游的每一型都在（A）的表里（即（A2）），
//! 表里的每一型在（B）里都有一枚上游的输出含它。**这不是把设计判据改写**——设计写的就是
//! 两半合起来的那一句（「上游算子的 `output_schema` 与下游算子的 `input_schema` **相交非空**」，
//! 设计 `:282`）。
//!
//! **期望值手写**（设计 §11.2 的「一条预防」，设计 `:770`）——两份语料都**不从
//! `all_operators()` 派生**：语料从被测清单里取就是恒真的假照片。
//!
//! **本块不建端口兼容判定函数**：§239 的判定已由 `crates/continuum-port/src/port.rs:94` 的
//! `compatible` 落地（切分 §四第 7 条）。**不许**为了让用例通过而在 `src/` 里加一个
//! 「端口兼容判定」函数。
//!
//! **首次运行即绿，步名不叫「确认失败」**：本 task 只新建测试、不交付生产代码
//! （计划 Task 3 的 Step 3 明写「无生产代码」），断言的是 Task 2 已交付的声明；
//! 它的红由 Task 9 Step 5 的变异复核取（改 `operators.rs` 的某一列 → 对应的
//! （A1）／（A2）／（B）红）。

use continuum_artifact::ArtifactType;
use continuum_image::all_operators;

/// （A）的语料：设计 §3.4 边表的下游侧一行，**逐字手写**（期望值的唯一来源，
/// 不从 `all_operators()` 派生）。
struct DownstreamRow {
    id: &'static str,
    /// 该下游在 §3.4 表里列出的型集合（设计 `:284` 的括号：「该来源的 `output_schema`
    /// 与下游 `input_schema` **交集中的型**」）。
    types: &'static [ArtifactType],
}

/// 设计 §3.4 边表（设计 `:288-292`）的五枚下游，**按表行序逐行手写**。
/// 派生的加强（（A1），本计划新增）见（A）测试函数的注释与计划 `## 遗留` 第一节第 6 条、第五节。
const DOWNSTREAM_TABLE: [DownstreamRow; 5] = [
    DownstreamRow {
        id: "generate-image",
        types: &[ArtifactType::Image],
    },
    DownstreamRow {
        id: "detect-edit-region",
        types: &[ArtifactType::Image],
    },
    DownstreamRow {
        id: "local-generative-edit",
        types: &[ArtifactType::Image, ArtifactType::Blob],
    },
    DownstreamRow {
        id: "hard-composite",
        types: &[ArtifactType::Image, ArtifactType::Blob],
    },
    DownstreamRow {
        id: "verify-outside-mask",
        types: &[ArtifactType::Image, ArtifactType::Blob],
    },
];

/// （B）的语料一行：`(下游算子 id, 上游算子 id, 型)` 三列，**逐字手写**
/// （期望值的唯一来源，不从 `all_operators()` 派生）。
struct EdgeRow {
    downstream: &'static str,
    upstream: &'static str,
    artifact_type: ArtifactType,
}

/// 设计 §3.4 边表（设计 `:288-292`）的语料，**按表行序逐行手写**。
///
/// **第 1 行（`generate-image`）的来源在 §3.4 表里写的是「上游任何一枚出的 `Image`」**
/// （设计 `:288`）——语料为它写一条 `generate-image ← generate-image : Image`
/// （本域至少有一枚算子输出 `Image`，而 `generate-image` 自身即满足），
/// **这一条与其余各行的强度不同**：其余各行指认了一枚**具体**的上游，这一行只指认
/// 「**存在**一枚」。
///
/// **另有三行的来源是同形的「上游源图（`Image`）」**：设计 §3.4 表给 `local-generative-edit`、
/// `hard-composite`、`verify-outside-mask` 各列了这一来源（设计 `:290-292`），它与
/// `generate-image` 那一行的「上游任何一枚出的 `Image`」是**同一个命题**：存在一枚上游其
/// `output_schema` 含 `Image`。**语料不为它们各写一行，理由在此写明**：那三行加上
/// `generate-image` 那一行是同一个断言的四次出现，逐行抄写会把「任一枚」读成「某四枚具体的
/// 上游」；**它们的承重与 `generate-image` 那一行相同**（都由
/// `generate-image ← generate-image : Image` 这一条钉住）。
///
/// **射程据实**：这一条钉的是「本域**存在**一枚上游输出 `Image`」，**不钉**「那三行的源图
/// 就是 `generate-image` 的产物」——后者的上游是哪一枚，规范与该表都写作「任一枚」。
///
/// **与（A）的语料无关**：（B）逐行只读**上游**的 `output_schema`（下游 id 只做语料自洽的
/// 存在性检查，**不读它的端口**）——故（A）一侧的任一取反不会动到（B）。
const EDGE_TABLE: [EdgeRow; 7] = [
    // 设计表第 1 行（设计 `:288`）：来源是「上游任一枚出的 Image」——「存在一枚」的对照件
    // （强度与其余各行不同，理由见上）。
    EdgeRow {
        downstream: "generate-image",
        upstream: "generate-image",
        artifact_type: ArtifactType::Image,
    },
    // 设计表第 2 行（设计 `:289`）：「上游任一枚出的 Image（generate-image 或素材导入）」
    // 在本语料里的具体化。
    EdgeRow {
        downstream: "detect-edit-region",
        upstream: "generate-image",
        artifact_type: ArtifactType::Image,
    },
    // 设计表第 3 行（设计 `:290`）：具体上游 `detect-edit-region` 的 `Blob`。
    EdgeRow {
        downstream: "local-generative-edit",
        upstream: "detect-edit-region",
        artifact_type: ArtifactType::Blob,
    },
    // 设计表第 4 行（设计 `:291`）：具体上游 `local-generative-edit` 的 `Image`。
    EdgeRow {
        downstream: "hard-composite",
        upstream: "local-generative-edit",
        artifact_type: ArtifactType::Image,
    },
    // 设计表第 4 行（设计 `:291`）：具体上游 `detect-edit-region` 的 `Blob`。
    EdgeRow {
        downstream: "hard-composite",
        upstream: "detect-edit-region",
        artifact_type: ArtifactType::Blob,
    },
    // 设计表第 5 行（设计 `:292`）：具体上游 `hard-composite` 的 `Image`。
    EdgeRow {
        downstream: "verify-outside-mask",
        upstream: "hard-composite",
        artifact_type: ArtifactType::Image,
    },
    // 设计表第 5 行（设计 `:292`）：具体上游 `detect-edit-region` 的 `Blob`。
    EdgeRow {
        downstream: "verify-outside-mask",
        upstream: "detect-edit-region",
        artifact_type: ArtifactType::Blob,
    },
];

/// （A）下游覆盖——§3.4 判据的下游侧：手写表「下游 → 该下游在 §3.4 表里列出的型集合」，
/// **逐行断言两件事**：
///
/// - （A1）表里列出的**每个**型都在该下游的 `input_schema` 里；
/// - （A2）该下游的 `input_schema` 里**每个**型都在表里列出——两个方向都判，
///   **合起来是集合相等**。
///
/// **（A1）是本计划新增的一半**：设计 §11.2 第 (s) 条写的「下游 `input_schema` 的每一型都能
/// 在该行列出的上游算子里找到」**正是**本计划的（A2）那一个方向；**表中列了而下游没有的型
/// 无人过问**——那一侧（取值 fail-open）才是新补的（A1），理由与依据见计划 `## 遗留`
/// 第一节第 6 条与第五节。
///
/// **（A2）承担的就是「下游的每一个型都能找到上游」这一半**：下游若多出一个语料外的型，
/// （A2）红，而在（B）里那一型根本没有对应的行——两处一起把「下游有孤儿型」逼出来。
///
/// **红条件**（各指唯一锚点）：
/// - 从 `local-generative-edit` 的 `input_schema` 里删掉 `Blob`（**取反档**）→（A1）红、
///   （B）里那一行（`local-generative-edit ← detect-edit-region : Blob`）**仍绿**
///   （它读的是上游的输出）；
/// - 给 `verify-outside-mask` 的 `input_schema` 加一个 `ArtifactType::Text`（**取反档**）→
///   （A2）红，（A1）仍绿。
///
/// Task 9 Step 5 另有一条对称的取反（从 `verify-outside-mask` 的 `input_schema` 里删掉
/// `Blob`）：它红（A1）而不红（A2）（剩下的 `input_schema = {Image}` 仍**是**表的子集）、
/// 也不红（B）（（B）读的是上游的 `output_schema`，该变异不动它）——（A1）／（A2）／（B）
/// 三半各自的唯一锚点因此都有独立的红来源。
#[test]
fn every_listed_type_is_on_its_downstream_and_each_downstream_type_is_listed() {
    let ops = all_operators();

    for row in DOWNSTREAM_TABLE {
        // 「按 id 取行」（与 tests/operators.rs 同口径：不按下标——行序重排是等价变异体）。
        let op = ops
            .iter()
            .find(|op| op.id.as_str() == row.id)
            .unwrap_or_else(|| panic!("all_operators() 里没有 id = {}（（A）表列出）", row.id));

        // （A1）表里列出的每个型都在该下游的 `input_schema` 里。
        for t in row.types {
            assert!(
                op.input_schema.contains(t),
                "（A1）{} 的 input_schema 缺 §3.4 表列出的型 {}",
                row.id,
                t.as_str()
            );
        }

        // （A2）该下游 `input_schema` 里的每个型都在表里列出。
        for t in &op.input_schema {
            assert!(
                row.types.contains(t),
                "（A2）{} 的 input_schema 含 §3.4 表外的型 {}（（B）里没有它的行）",
                row.id,
                t.as_str()
            );
        }
    }
}

/// （B）上游来源——§3.4 判据的上游侧：手写三列语料 `(下游算子 id, 上游算子 id, 型)`，
/// 逐行对应设计 §3.4 的边表（设计 `:288-292`），**逐行断言该上游的 `output_schema` 含该型**。
///
/// **交集中的型，不是「两侧相等」**：第 1 行（`generate-image`）的上游是「**任一枚**」
/// （设计 `:288`），故该行**不**要求上游的 `output_schema` 与下游的 `input_schema` 相等；
/// 这里断言的是「**交集非空**」这一半的可检查形式——（B）只读上游的 `output_schema`，
/// 不读下游的 `input_schema`（设计 `:284` 的括号：「交集中的型」）。
///
/// **红条件**：把 `hard-composite` 的 `output_schema` 里的 `Image` 删掉（**取反档**）→
/// （B）里**只红一行**（`verify-outside-mask ← hard-composite : Image`）；
/// `hard-composite ← local-generative-edit : Image`**仍绿**——它读的是**上游**
/// `local-generative-edit` 的输出，删 `hard-composite` 的输出与它无关。
/// （A1）与（A2）也不受影响（`hard-composite` 自己的 `input_schema` 一字未动）。
#[test]
fn every_edge_row_has_an_upstream_that_outputs_its_type() {
    let ops = all_operators();

    for row in EDGE_TABLE {
        // 语料自洽：下游这一栏应能取到行。**本行不读它的端口**——（B）只读上游的输出。
        assert!(
            ops.iter().any(|op| op.id.as_str() == row.downstream),
            "语料的下游 id = {} 在 all_operators() 里找不到",
            row.downstream
        );

        let upstream = ops
            .iter()
            .find(|op| op.id.as_str() == row.upstream)
            .unwrap_or_else(|| {
                panic!(
                    "语料的上游 id = {} 在 all_operators() 里找不到",
                    row.upstream
                )
            });

        assert!(
            upstream.output_schema.contains(&row.artifact_type),
            "（B）{} ← {} : {}——上游的 output_schema 不含该型（§3.4 边表）",
            row.downstream,
            row.upstream,
            row.artifact_type.as_str()
        );
    }
}
