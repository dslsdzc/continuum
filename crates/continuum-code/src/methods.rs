//! `bind_code_methods`：设计 §7.1 表里七条 `software/` 的 `realized_by` 的
//! **唯一填写入口**（设计 §7.3 的落点也在这里）。

use continuum_method::{MethodError, MethodId, MethodRegistry};

/// 把设计 §7.1 表里七条 `software/` 的 `realized_by` 填进方法库。
/// 调用形态：对**每一条已 seed 的 id** 调一次 `MethodRegistry::bind`（P5b 设计第六节），
/// 其中 `TDD` 与 `debugging` 两条按已关闭的裁定填**空**。
/// 这是本块调用 `MethodRegistry::bind` 的**唯一入口**。
/// **不做二次登记**：不 `register` 方法条目——七条 id 由 P5b 的 `seeded()` 给出。
///
/// # 七条取值逐行的理由（§7.1 表逐行照录）
///
/// - `planning` → `["code-plan", "code-decompose"]`：§186 `:62` 的 `Implementation Plan`
///   与 `:64` 的 `Engineering Tasks` 两步都是规划；
/// - `worktree` → `["code-workspace"]`：§16 `:790-794` 的 git worktree（算子见设计 §4.2）；
/// - `review` → `["code-spec-review", "code-quality-review"]`：§186 `:85` 逐字
///   「规格审查与代码质量审查分离」；
/// - `fuzz` → `["code-fuzz-run"]`：§193 `:368-369` 的 Fuzzing 与 Property-based Testing；
/// - `verification` → `["code-test-run", "code-verify"]`：§186 `:74` 的 Verification
///   （两枚的分工见设计 §3.3 第 5、11 条）。
///
/// # `TDD` 与 `debugging` 为什么是**空**（「不是还没填」）
///
/// - `TDD` 的全部内容是**次序**（先写测试、后写实现），而 `realized_by` 的类型
///   （`Vec<String>`）是算子 id 的一个**集合**，**表达不了次序**——填两枚 id 会让
///   `TDD` 与「先实现后补测试」在登记面上**不可区分**，而两者的差别正是这条方法的
///   全部内容（P5c 设计 §7.2 第 1 条；§188 同一取向）。
/// - `debugging` 在**链上没有对应的步骤**（《总纲》§8.3 的六步与 §186 那十行流程块
///   都没有调试），且规范没有一处给过「调试的输入与产物是什么」——故**不发明**
///   一枚 `code-debug`（P5c 设计 §7.2 第 2 条）。
///
/// 两条「无算子对家」由协调者 2026-10-10 裁定（P5b 设计第十节判据 6），以
/// [`continuum_method::UNREALIZED_BY_DESIGN`] 承载——**空不是「漏 bind」**，是裁定后
/// 的正确取值；这两条的取值范围与其余五条一样，都在本函数的唯一入口里落定。
///
/// # §7.3 的落点：§186 的流程名**不入目录**
///
/// 本函数的交付物里**没有任何一行 `register`**：只对 P5b `seeded()` 给出的七条 id 调
/// `MethodRegistry::bind`，**七个实参逐一取自 §3.2 的算子 id，没有一个是 §186 的步骤名**
/// （P5b 设计第六节禁止四个算子块自造 `MethodEntry`；设计 §7.3 的答复是「不入」）。
/// **这一条没有可拍的照片**（它是「代码里没有那一行」），据实记此；它的可观察近邻是
/// `tests/methods.rs` 的 (m)：§186 `Implement` 那一步落成的算子 id `code-implement`
/// 不被任何 `realized_by` 引用。
/// **射程注**：本判据在**登记面**（七个 `bind` 实参逐一核对），不是「源码零 grep 命中」
/// ——`spec-review` 字样作为算子 id `code-spec-review` 的后缀必然出现，本注释也以例样
/// 写到它（同 `dependency_direction.rs` 那条「零命中类判据先问本仓哪条规矩会往语料里
/// 加字」的教训）。
pub fn bind_code_methods(registry: &mut MethodRegistry) -> Result<(), MethodError> {
    // §7.1 表逐行（行序即表序）；每一行调一次 `bind`。
    // `planning`：§186 `:62`／`:64` 的两步都是规划。
    registry.bind(&MethodId::new("planning"), &["code-plan", "code-decompose"])?;
    // `TDD`：次序约束、无算子对家——空（裁定，理由见函数文档）。
    registry.bind(&MethodId::new("TDD"), &[])?;
    // `debugging`：链上无此步、无算子对家——空（裁定，理由见函数文档）。
    registry.bind(&MethodId::new("debugging"), &[])?;
    // `worktree`：§16 `:790-794` 的 git worktree。
    registry.bind(&MethodId::new("worktree"), &["code-workspace"])?;
    // `review`：§186 `:85` 逐字「规格审查与代码质量审查分离」。
    registry.bind(
        &MethodId::new("review"),
        &["code-spec-review", "code-quality-review"],
    )?;
    // `fuzz`：§193 `:368-369` 的 Fuzzing 与 Property-based Testing。
    registry.bind(&MethodId::new("fuzz"), &["code-fuzz-run"])?;
    // `verification`：§186 `:74` 的 Verification（两枚的分工见 §3.3 第 5、11 条）。
    registry.bind(&MethodId::new("verification"), &["code-test-run", "code-verify"])?;
    Ok(())
}
