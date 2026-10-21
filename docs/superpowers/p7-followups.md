# P7 设计与实现台账

日期：2026-10-22

## 一、设计

- 切分：`specs/2026-10-18-p7-scope-and-split.md`（494 行，3 轮评审收敛；C2 裁定已补记于 `3e33d64`）。
- P7A 产品循环：`specs/2026-10-18-p7a-product-loop-design.md`（1974 行，复评 4 全部闭合）。
- P7B Intent Scheduler：`specs/2026-10-18-p7b-intent-scheduler-design.md`（1451 行，复评 3 收尾）。
- P7C Resource Arbiter：`specs/2026-10-19-p7c-resource-arbiter-design.md`（1535 行，复评全部闭合）。
- P7D Supervisor + Session：`specs/2026-10-19-p7d-background-supervisor-and-session-design.md`（1927 行，复评全部闭合）。
- 计划：`plans/2026-10-19-p7{a,b,c,d}-*.md`（2333 / 1752 / 2537 / 1671 行）。

## 二、实现与合入

| 块 | crate | 终审 | 终审门 |
|---|---|---|---|
| P7A | `continuum-product`（17 任务） | Approved（`6cd19d7` + `eda889f`） | 1643 passed / 1 failed（P6D 已知红） |
| P7B | `continuum-intent-scheduler`（8 任务） | Approved（`22782c0`） | 1462 passed / 0 failed / 0 warning |
| P7C | `continuum-arbiter`（8 任务） | Approved（`6ee81cc`） | 1594 passed / 1 failed（P6D 已知红） |
| P7D | `continuum-supervisor`（14 任务） | Approved（`eda889f`） | **1802 passed / 0 failed / 0 warning** |

C2 裁定：`ContinuousProductDrive` 取 **(b) 独立 `ProductDriveState` + `product_drive` 表**（迁移 200 四表）。

## 三、P7 终极门

main `eda889f`（四块齐）：**287 Running / 323 result / 1802 passed / 0 failed / 0 warning / EXIT=0**。

## 四、P6D rebaseline（由 P7D 负责）

- `p7_pre_allowed.txt` 31 行（等于 `c89d839` 的 `ALLOWED`）；`P7_EXPECTED_ROWS` 四行；解析器与断言 1 的 `27`→`31`；断言 2 的 `5`→`4`；旧 `p6_pre_allowed.txt` 删除；测试改名 `p7_union_adds_only_its_own_rows`。
- 遗留口径：31 行基线早于 P6A 合入，断言 5/8 的余量由 `POST_BASELINE_SUBJECTS`（`continuum-context`/`memory`/`protected`）承接——单一余量口径，不重开 P6 fixture。
- P7B 在 `dependency_direction.rs` 留的 `POST_P6_SUBJECTS = ["continuum-intent-scheduler"]` 为 interim 扩展，P7D 的 rebaseline 已统一。

## 五、Minor 与遗留（不阻塞）

- **P7A**：`baseline.rs:43`/`readiness.rs:155,171`/`milestone.rs:145,159` 的 `#[allow(dead_code)]` 过期；`shippable.rs:177` 的 `_predicate_type_is_used` 死代码；`persist.rs:1203,1213,1223` 的 `serde_json::to_string(...).expect(...)`；J19 照片解析 `Cargo.toml` 而非 `cargo tree`；构造点隐私照片（`from_verified_parts` 无 trybuild）；`cargo fmt` 非零。
- **P7B**：`POST_P6_SUBJECTS` 的 4 行扩展需在 P8 切分中确认（P8 切分已按 P7D 的 rebaseline 口径处理）；J1-a 的 `NoIntent` 结构未用源码面钉住。
- **P7C**：`policy.rs:35` 的合法策略只断言 `is_ok()`；J3 的钳位在公开面不可观察（设计约束）。
- **P7D**：`authorized_intent` 缺非 READY 负例（附加变异未红）；`c2d9f35` 修改了计划共写清单外的测试文件（必要跟进）；`TaskKey::for_task` 的 `expect` 在公开调用链；`tests/compile_fail.rs` 的 glob 过宽；两处 `is_ok()` 偏弱；`7126bb6` 的空引用检查冗余。
- **P7 的设计措辞**：P7D 的 4 条（§3.3 五字段、`NewConversation` 归属、§9.1「P6C 函数」、J12 `Evidence` grep）已由实现照实登记，未改设计正文。

## 六、待协调（跨批次）

- **§14**：`Running → Throttled → Checkpointing` 是否必经——P7C 按「可跳过」实现（设计/计划的当前读法），规范维护者尚未确认；若改「必经」，Task 4/5 需重做。
- **P7D 的内存态**：`HashMap<TaskKey, SuspensionState>` 跨重启丢失——已知缺口，若需持久化走 P7 新表（未计划）。
- **P7D 的 `SynthesisIo` 端口**：装配方持 `Tx`；切分 §2.3.1 第 4 条已同步。
- **P8 的 P7 门 rebaseline**：P8A 第一提交（`p8_pre_allowed.txt` 36 行、`P8_EXPECTED_ROWS` 3 行、`P8_RUNTIME_ADDITIONS`）。
