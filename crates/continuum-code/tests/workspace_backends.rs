//! 设计 §4.2 的照片（§10.2 第 (p) 条）：`code-workspace` 的两个 backend 候选名
//! 与 P2 已落地的 `WorkspaceBackend` 两臂的对应。
//!
//! **两半合起来才算钉住**（纪律 7）：
//!
//! - **(p1) 运行期侧**：第 1 行（`crates/continuum-code/src/operators.rs` 里
//!   `code-workspace` 那一行）的 `backend_candidates` 与**手写的**
//!   `{"worktree", "overlay"}` **集合相等**（个数也判，故「删一项」红）。
//!   **不判次序**：候选列表在本块没有任何断言读它的次序，「两项换序」是一枚
//!   **等价变异体**（设计 §10.3 末行已写明）；**要打红它必须换变异体：删掉一项**
//!   ——那时本条的 (p1) 红（第 1 行与手写期望的集合不等）。**实跑**：删一项 ⇒
//!   本条红；**Task 2 的名单条 `the_list_is_exactly_these_five_names` 读的是
//!   `REPRODUCIBLE_BACKENDS` 常量本身、不随之红**（它不看第 1 行的候选列）。
//!   **这一句要写进注释**，否则后来者会拿换序去验守卫。
//! - **(p2) 编译期侧**：对 `WorkspaceBackend` 做**穷尽 `match`、不带 `..`**，
//!   每臂返回期望的候选串，断言 `match` 出来的集合等于**测试里同一份手写期望**。
//!   **这是一张编译期照片，照实写成编译期照片**：P2 若按 §255 的非 Git 项目那五种
//!   形态给 `WorkspaceBackend` 加第三臂，`crates/continuum-code/tests/workspace_backends.rs`
//!   **编译不过**（实跑错误码：`error[E0004]`，完整文案见 `declared_name` 的注记），
//!   它**不是**「用例变红」。**它成立的前提是 `match` 的入参是枚举自身**——故这里的
//!   `match` 收 `WorkspaceBackend`，而**不是**收 `&str`（收 `&str` 的解析函数一定有
//!   通配臂，做不成这张照片）。
//!
//! **(p2) 为什么不受 (p1) 的红条件影响**：它读的是 `WorkspaceBackend` 与**测试里
//! 手写的**期望串，**不读算子的 `backend_candidates`**，故改算子的那一列不动它。
//! **若本意是让 (p2) 也红**，那要把它写成「`match` 的结果等于**算子的候选集合**」
//! ——那时它变成运行期断言，与它自陈的「编译期照片」混在一处，本计划**不取**
//! 那条写法（(p2) 的用途只有一个：「P2 给枚举加一臂即编译不过」）。
//!
//! **红条件（取反档，锚点唯一）**：把 `operators.rs` 里第 1 行的 `overlay` 写成
//! §255 的散文名 `overlayfs` ⇒ **(p1) 红而 (p2) 仍绿**（理由见上一条）。
//! **锚点唯一**：`crates/continuum-code/src/operators.rs` 里第 1 行的 `backend_candidates`。
//!
//! **这条照片的限度（照实写）**：它**不钉** `workspace` 列的落库编码——
//! 若 P2 只改 `backend_str` 的两个字面量而两臂名不变，**本用例不红**。
//! 该处的权威是 `backend_str` 本身（`crates/continuum-workspace/src/persist.rs:166-176`
//! 的注记写明这两个编码**显式给出、不由 serde 表示或 `Debug` 推出**），故本块
//! **不拿 serde 表示替代它**：两者今天恰好同串，而它们是两件事（设计 §3.5 末）。
//! 原命题「候选名等于 `workspace` 列的落库编码」今天拍不出：`backend_str` 是
//! `pub(crate) fn`（`persist.rs:177`），本 crate 跨 crate 调不到（设计 §10.2 第 (p) 条
//! 的订正段）；补它的两条路（放开可见性 / 加一条 `continuum-persist` 的 dev 边去读
//! `workspace` 列的原文）都超出设计 §2.1 的六项边表——据实报出、不在本块就地补
//! （计划 `## 遗留`）。
//!
//! **候选名取的是 P2 的编码，不是 §255 的散文形**（§4.2、§3.5 末）：§255 `:1026`
//! 写的是 `git worktree`、`:1032` 写的是 `OverlayFS`，而 P2 已落地的编码是 `"worktree"`
//! 与 `"overlay"`——`backend_candidates` 的元素是 Router 拿去与 P2 的后端名比对的
//! 东西，故**以代码里的编码为准**。

use continuum_code::all_operators;
use continuum_workspace::WorkspaceBackend;

/// 设计 §10.2 第 (p) 条。
#[test]
fn the_workspace_candidates_are_the_two_declared_backends() {
    // 行序读法：「第 1 行」= §3.2 表的第 1 行（与 Task 1/4 同一种读法），
    // 不是 `operators.rs` 的物理行号。
    let operators = all_operators();
    let workspace = &operators[0];

    // **同一份手写期望**，两半都读它。**必须手写**，不从 `WorkspaceBackend`
    // 或算子的候选列遍历派生——从被测源派生期望值，测的是「它等于它自己」，
    // 那是恒真的假照片（纪律 1(b)(ii)；设计 §10.2 的「语料从被测清单里取就是
    // 恒真的假照片」）。
    let mut sorted_expected = ["worktree", "overlay"];

    // ===== (p1) 运行期侧 =====
    // 集合相等（个数也判）：两侧各自排序，排序后逐位比较即「集合相等+个数相等」，
    // 而列表的次序不进比较。**「两项换序」是一枚等价变异体**（设计 §10.3 末行：
    // 候选列表无序，本块没有任何断言读它的次序）；**要打红它必须换变异体：
    // 删掉一项**——那时本条红（集合与个数都不等）。
    let mut actual: Vec<&str> = workspace
        .backend_candidates
        .iter()
        .map(|b| b.as_str())
        .collect();
    actual.sort_unstable();
    sorted_expected.sort_unstable();
    assert_eq!(
        actual, sorted_expected,
        "第 1 行（code-workspace）的 backend_candidates 应恰为手写的两枚：集合相等且个数相等（§10.2 第 (p) 条）"
    );

    // ===== (p2) 编译期侧 =====
    // 穷尽 `match`、不带 `..`、**入参是枚举自身**（不是 `&str`——收 `&str` 的
    // 解析函数一定有通配臂）：P2 给 `WorkspaceBackend` 加第三臂时，本文件
    // **编译不过**（不是「用例变红」）。判定落在 `declared_name` 的 `match` 上。
    let mut declared: Vec<&str> = [WorkspaceBackend::Worktree, WorkspaceBackend::Overlay]
        .into_iter()
        .map(declared_name)
        .collect();
    declared.sort_unstable();
    assert_eq!(
        declared, sorted_expected,
        "(p2)：WorkspaceBackend 两臂的期望串集合应等于 (p1) 的同一份手写期望（§10.2 第 (p) 条）"
    );
}

/// 每臂返回期望的候选串。**穷尽 `match`、不带 `..`**——这张照片的判定就在本函数：
/// P2 若按 §255 的非 Git 项目那五种形态给 `WorkspaceBackend` 加第三臂，
/// 这里的 `match` 缺臂，**本文件编译不过**。实跑错误码（加第三臂的实验，
/// 实验臂名 `Snapshot`）：`error[E0004]`、文案
/// `non-exhaustive patterns: WorkspaceBackend::Snapshot not covered`，
/// 指向本函数的 `match` 行（完整输出与实验日志见报告与 `.tmp/`）。
///
/// **入参是 `WorkspaceBackend` 自身，不是 `&str`**：收 `&str` 的解析函数**一定有
/// 通配臂**（认不出的串要有去处），做不成这张照片——这是本照片成立的前提。
///
/// **它不读算子的 `backend_candidates`**：改第 1 行那一列（如 `overlay` →
/// `overlayfs`）时本文件照旧编译、本半照旧绿——(p2) 与 (p1) 各读各的来源。
fn declared_name(backend: WorkspaceBackend) -> &'static str {
    match backend {
        WorkspaceBackend::Worktree => "worktree",
        WorkspaceBackend::Overlay => "overlay",
    }
}
