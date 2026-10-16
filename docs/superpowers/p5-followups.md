# P5 收尾台账（跨块）

> 用途：P5 六块（P5a–P5f）的**跨块留存项**与**已合入记录**汇总。
> 各块详细的 deferred minors 在各自 worktree 的 `.superpowers/sdd/*/progress.md`（gitignore 区，随 worktree 保留）。
> 本文件随进展更新；P4 的对应台账见 `docs/superpowers/p4-followups.md`。

## 已合入 main

| 块 | 合并点 | 日期 | 摘要 |
|---|---|---|---|
| QA 质量摸底修复 | `17e464a` | 2026-10-16 | 性能/混沌/对抗三维摸底 + 全部跟进修复（graph 线性化 10–16×、budget 聚合 2.3–2.6×、写入模式文档、policy 写策略侧自查、落档 `docs/2026-10-16-qa-质量摸底.md`）。复评 All addressed。 |
| P5b 执行方法库 | `c7bcb07` | 2026-10-16 | `continuum-method` 全量 5 任务（`MethodDomain`/`MethodId`/`MethodError`/`MethodEntry`/`MethodRegistry`/`seeded()`/`UNREALIZED_BY_DESIGN`）。终审 **Approved**（0 Critical / 0 Important / 5 Minor）。 |

## P5b 终审 Minor（待顺手修，注释/文本层）

- **M1** `crates/continuum-method/tests/domain.rs:114` 与 `:20-21`：「`"3D"` 的大小写变体（四条里唯一有字母可大写）」措辞失准（四个目录名都有字母）——建议改或然性表述（「数字后接字母、按常见写法也作 `3D`，是本清单里唯一会被顺手大写的组合」）。
- **M2** `crates/continuum-method/tests/roster.rs`（约 `:341-351`）：名册「多一条」红条件注释未点名机制——补「由上面的 `len` 断言抓住；`zip` 只比到短者」。
- **M3** `crates/continuum-method/src/catalog.rs:40` 与 `tests/registry.rs:18`：「只在这条路径上可达」/「不可达」表述并存——加衔接半句。
- M4（`Eq` 派生无判据）、M5（`select` 排序弱变异体）——终审裁定**维持现状**。

## P5b 任务级 deferred minors（终审已逐条裁定，0 条阻塞）

详见 `.worktrees/p5b/.superpowers/sdd/2026-10-10-p5b-execution-method-library/final-review.md` 与各 `task-N-report.md`。
重点条目：T2「3D」措辞（=M1）、T2 报告 §2.1「全文」标注、T3 `Eq` 无判据（=M4）、T3 排序弱变异体（=M5）、T4 `len` 注释（=M2）、T4「可达/不可达」（=M3）、T5「合理连带」分档读法、设计 §8 R1–R9 待对账项（收件人：设计作者）。

## 进行中

- **P5a**（`continuum-verify`）：T1–T10 完成（门 1111/0/0）；**T11 装配与收尾在跑**（runtime 接迁移 130 + 12 档变异复核，长跑）。
  - 待办结转：Task 10 的 `CompletionVerdict::Complete` 编译失败样例已落；T11 的 `## 遗留` 是否并入 followups 由协调者定。
- **P5e**（媒体领域算子）：T1–T7 完成（门 166/1053/0/0）；**T8 `bind_media_methods` 收口在跑**；剩 T9/T10/T11。
  - 已执行跨块合并：`2319763 Merge branch 'p5b'`（硬前置）。

## 待启动

- **P5c**（代码领域算子）——等 P5a、P5b 合入。
- **P5d**（研究领域算子）——等 P5a、P5b、P5e。
- **P5f**（图像领域算子）——等 P5a、P5e。

## 环境备注

- 本机 btrfs 97% 满 + 并发门负载：全量门实测贴 2400s 上限（T7/T8 各一次 EXIT=124）——**全量门统一用 3600**；性能数字受环境放大（见 QA 落档 §环境）。
- 服务器重启频繁：恢复执行者与接棒者可能双跑——先到者提交、后到者核对接受的处置已多次生效。

## P5a 终审 M1（随合入并档——设计同步待办；2026-10-17）

- **半同独立性读法**（P5a T6）：`independence_of` 对「model/provider 一项同、一项不同」判 `NotEstablished`（fail-closed 侧）——该读法目前只存在于代码文档与任务报告；**设计 §7.3 未写**。收件人：设计作者（建议补一句「半同 ⇒ 判不出」）。
- **`UndecidableItem`**（P5a T7，协调者已追认）：`undecidable` 元素类型取两臂枚举（合取项 / `DeclaredCompletionPredicate`）——**设计 §8.2 仍写 `Vec<Conjunct>`**、§12.1 首段枚举清单亦未列；照 §8.2 字面读会得出错结论。收件人：设计作者（§8.2 与 §12.1 同步）。

## 已合入 main（续）

| 块 | 合并点 | 日期 | 摘要 |
|---|---|---|---|
| P5a 验证与证据判定 | `07353ef` | 2026-10-17 | `continuum-verify` 全量 11 任务 + runtime 装配（迁移 130）。终审 **Approved**（0 Critical/0 Important；6 组 Minor、29 条 deferred 全裁定接受）。门（提交前树）168/1111/0/0/EXIT=0；13 轮变异复核。 |
