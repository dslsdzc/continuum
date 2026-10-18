# P6 设计/计划阶段台账

日期：2026-10-17（工作树 `.worktrees/p6-plan`，分支 `p6-plan`，基线 main@`03e79c7`；main 现为 `3625ee3`，仅多 UI 接口文档提交）

## 一、设计定稿状态

| 设计 | 文件 | 行数 | 复评结论 |
|---|---|---|---|
| P6A Context Compiler | `specs/2026-10-17-p6a-context-compiler-design.md` | 1283 | 4 轮复评，全部闭合；fix 5 收 3 条 Minor |
| P6B 七个信任标签 | `specs/2026-10-17-p6b-trust-labels-design.md` | 1004 | 3 轮复评，全部闭合，无新问题 |
| P6C 记忆与学习 | `specs/2026-10-17-p6c-memory-and-learning-design.md` | 1541 | 3 轮复评，全部闭合；fix 4 收 8 条 Minor |
| P6D Learning 与 Protected Core | `specs/2026-10-17-p6d-runtime-learning-and-protected-core-design.md` | 1382 | 2 轮复评，全部闭合；fix 2 收 6 条 Minor |

## 一·五、实现与合入状态（2026-10-18）

| 块 | 实现 | 终审 | 合入 main |
|---|---|---|---|
| P6C `continuum-memory` | 12 任务，干净单次全量门 **231 result / 1369 passed / 0 failed / 2 ignored / 0 warning / EXIT 0**（实现者曾报 1376/260，差异＝按 target 重复计数，终审已逐项解释） | **Approved**（0 Critical / 0 Important；4 Minor：`retrieval.rs` 两处仅 `is_ok`、`synthesis.rs` 三处 `expect`、A4/fail 的 `affected==1` 防御、全仓已无 `allow`） | **`e0adc69`（已推送）** |
| P6B `continuum-trust` | 6 任务 + 源码面 + 14 行变异（全部一致）；干净全量门 **196 Running / 224 result / 1281 passed / 0 failed / 2 ignored / 0 warning / EXIT 0** | **Approved**（0 Critical / 0 Important；Minor：source_surface 钉行号、fmt 差异、§9.3(w) 字面读数） | **`7198dfd`（已推送）** |
| P6D 两枚 crate | 8 任务（Task 8 仅核验无提交）；干净全量门 **198 Running / 227 result / 1272 passed / 0 failed / 2 ignored / 0 warning / EXIT 0**；P6D 专项 18/18；并集门 3/3；fixture 27 行、`P6_EXPECTED_ROWS` 5/5；5 条变异复核预期红 | **Approved**（0 Critical / 0 Important；Minor：`ProtectedCore::fingerprint` 的不可达 `expect`） | **`c55b03b`（已推送）** |
| P6A `continuum-context` | 实现中（10 任务，`p6a-dev`） | — | — |

P5 全量测试（main `e78eaec`）：**EXIT=0，216 result / 189 Running / 1253 passed / 0 failed / 0 warning**。

B+C+D 合并树全量门（main `e4a1ab2` 后）：**EXIT=0，250 result / 219 Running / 1416 passed / 0 failed / 0 warning**。

跨块修复：P6C 的 `wiring.rs` S2 断言原按「全文出现 2 次」冻结，P6D 的 `P6_EXPECTED_ROWS` 与 P6A 的 `context` 允许边会让它漂移——已改为「只在 `ALLOWED` 段内计数」（`e4a1ab2`），`wiring` 5/5 绿。P6A 接棒时用 `git checkout main -- crates/continuum-memory/tests/wiring.rs` 对齐。

## 二、计划状态

| 计划 | 文件 | 行数 | 任务数 | 评审结论 |
|---|---|---|---|---|
| P6A | `plans/2026-10-17-p6a-context-compiler.md` | 1820 | 10 | **定稿**（复评 2 全闭合；Minor 收口中） |
| P6B | `plans/2026-10-17-p6b-trust-labels.md` | 751 | 6 | **Approved**（7 条 Minor 收口中） |
| P6C | `plans/2026-10-17-p6c-memory-and-learning.md` | 1292 | 12 | **定稿**（4 轮 fix，全部闭合） |
| P6D | `plans/2026-10-17-p6d-runtime-learning-and-protected-core.md` | 1409 | 8 | **定稿**（1 条非阻塞 Minor 收口中） |

## 三、跨计划待裁定项（协调者）

1. **P6C 共写表漏两处测试断言**（P6C 计划评审 I1，已实读确认）：
   - `crates/continuum-runtime/tests/migrations.rs:35` 的 `expected_migrations()` 需随 190 加行；
   - `crates/continuum-runtime/tests/startup.rs:25/:92`（13 项）与 `:80`（11 项）的断言需改为 14／12。
   - 处置：P6C 计划 Task 12 已含同步动作；设计 §3.3 的共写表需协调者补两行（或明确把「测试断言同步」排除在「源码」之外）。
2. **P6A 的 `MemoryClaim` crate 外测试构造面**（P6A 计划遗留①，P6C 计划评审 I7）：
   - P6A 的假检索面需要 crate 外可造 `MemoryClaim`，而 P6C 冻结 `MemoryClaim::new` 为 `pub(crate)`（C2 照片）。
   - 两条不能同真。裁定选项：(i) P6C 加 feature-gated `#[doc(hidden)]` 测试构造器并改 C2 口径；(ii) P6A 走真库经 `apply_synthesis` 产 claim（与 `&dyn MemoryRetrieval` 假面冲突）。**P6A 开工前必须裁定。**
3. **P6B 设计 §9.3 的两处计数勘误**（P6B 计划评审 M1/M2）：
   - §9.3(t)「只在 `as_str`」的七串 → 实际应为本 crate 的 `as_str`/`parse` 编解码对（14 条），否则 §9.4 的 `as_str` 变异会失去敏感度；
   - §9.3(u)「三处」→ `TrustError` 落地后为四处。
4. **P6A 设计 §8.3 与 §8.1 J3 的相抵**（P6A 计划遗留②）：§8.3 残留「移除 J3 validate」一行，而 §8.1 J3 已改调 `validate`。待 P6A 计划评审裁决；若属实，以 §8.1 为准，§8.3 需订正。
5. **P6A 的 `compile` 参数序**（P6A 计划遗留③）：按设计 §5.1 取 `(request, budget, memory)`；P6A 计划已申报，计划评审核实中。

## 四、P6 不关闭的既有未决项（照设计 §11/§12 转记）

- 执行器（节点级 §279 通道）——P1；
- Conversation 存储/轮次身份、用户粘贴外部内容、artifact 类型化生产者、provider 身份——P1/P3B/P3F/P3G/协调者；
- `§70`「降低隐私暴露」无落点——P2/第 4 层/协调者；
- `§322` 的「独立验证」「用户批准」无类型/入口；authority/contract 两组无可引用规则集上游——规范维护者/长期层；
- `§74` TTL/decay/archived、`§171` 阈值、`§284` Session 冲突、`§283` 实例字段、`§9.2` 入度为零 vs persist——规范维护者/协调者/P7；
- P1 的 `continuum-graph` 若要命名标签类型需具名 `ALLOWED` 例外——协调者；
- P5a 的 `Evidence` 标签边——P5a 持有方。

## 五、P5 遗留

见 `docs/superpowers/p5-followups.md`（未动）。
