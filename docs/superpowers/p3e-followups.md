# P3 子项目 E：终审后的交接事项

本文件只记**代码与设计正文里看不出来、而后继者必须知道**的东西。
设计与计划在 `docs/superpowers/specs/2026-10-06-p3e-compute-placement-design.md` 与
`docs/superpowers/plans/2026-10-06-p3e-compute-placement.md`；
裁定记录在 `docs/superpowers/2026-10-06-p3e-decisions.md`。

**终审结论（`ce335dd..e6fec3d`，报告在 `.worktrees/p3e/.superpowers/sdd/p3e-final-review.md`）：可以合入 main。**
无 Critical；一条 Important（已修）；九条 Minor；两条计划强制项（已裁）。

---

## 一、已修

- **I1（唯一 Important）**：判据 §4.4 在**制品 ≥ 2 枚**时没有照片。变异体把步骤 2 的循环改成
  `take(1)` ⇒ 一枚 `LOCAL_ONLY` 制品被放到云节点上，而**全量门 exit=0、0 FAILED**。
  根因：`tests/placement.rs` 的夹具都只有 0 或 1 枚制品。**设计 §5.3 要求「对每一枚制品」** ⇒ 补用例。
- **计划强制项 1**：`tests/registry.rs:92` 的用例名与体内不符（计划 `:720` 已改名，代码没跟）。
- **M4～M8（仓里已有的失真陈述）**：`error.rs` 的两处 `unwrap_err` 界（实为 `T: Debug` 而非 `E: Debug`）、
  `Cargo.toml` 的两处使用点、`node.rs` 的 `capabilities()` 读者数、`placement.rs:380` 的
  「唯一 fail-closed 机制」（设计 §5.4 是三条通道）。

## 二、**据实记为无照片、本轮不补**（每条附「能区分的那枚变异体」，将来要补时直接用）

| # | 属性 | 现状 | 能区分它的变异体 |
|---|---|---|---|
| **M1** | 「判重是第一步」 | 无照片。终审的第一枚变异体（判重挪到闸门之后、仍在取头之前）**实测全绿，但它是等价变异体**，已据实作废 | **M-B2**：判重挪到**判空 return 之后**，输入 `[cloud a, cloud a]` ＋ `LocalOnly` ⇒ 交付版 `DuplicateNode`、变异版 `NoPlaceableNode`，全量 0 FAILED |
| **M2** | `try_new`「先覆盖、再重复」的次序 | 无照片 | 区分两版的入参 ＝ **4 条且重复 `Public` 且缺 `LocalOnly`** 的表 |
| **M3** | 注册表 → 放置这条**接缝** | **无用例**：`registry.rs:99-106` 与 `placement.rs:205-206` 的文档互指，而 `tests/placement.rs` 全文不出现 `NodeRegistry`、`tests/registry.rs` 全文不出现 `place`。否定面（无旁路）只有 grep 人证：`src/` 里 `place` 调用点 0 处 | 未给（要先定义这条接缝该断言什么） |
| **M9** | 报告 §4 的红行列**绑在 `287c6c1` 的字节**（`f7f6639b…`，849 行）上却称「交付字节」；交付是 `bced591f…`（862 行） | 报告措辞。**交付文件自身的 `:NNN:CC` 已逐条核过全对** | — |

## 三、承重归属（终审逐处判过，记在此免得后人重复判）

- **`ALL` 两两不同**那条断言的**承重方是 `continuum-artifact`**（外键 ＋ crate 边界），
  E 这一侧是**回归护栏**，不是守卫。已在 `p3bcdf-followups.md` 记明。
- `cargo tree -p continuum-node --depth 1 --edges all` **直接边四条**：`continuum-artifact` 与
  `thiserror`（`[dependencies]`）、`trybuild` 与 `serde_json`（`[dev-dependencies]`）。
  **`Cargo.toml` 里「`serde_json` 只在测试目标里用」这句全称断言没有判据钉住**——
  `ALLOWED` 的内层循环只遍历 workspace 成员之间的边，外部 crate 不在射程（设计 §7.3），
  故把它挪进 `[dependencies]` 不会让任何断言变红。**据实记为无照片。**

## 四、共享文件（与别的分支共写，合入时注意）

`Cargo.toml` 的 `members` 与 `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED`
**由 E、F、P4 三份计划共写**。终审已核：**E 这两处各自只登记了自己那一条**（`Cargo.toml` 一行、
`ALLOWED` 一条 ＋ 注释），**别的条目一字未动**。
