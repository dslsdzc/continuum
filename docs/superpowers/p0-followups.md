# P0 遗留项

本文件是 P0 实现过程中逐任务评审与全分支终审累积的未处理项，供 P1 分拣。
它记录的原始来源是各 task 的实现者报告与评审报告（重建于 worktree 的 `.superpowers/sdd/`，随工作区清理而失效）。

P0 最终状态：62 passed / 0 warning；终审判定 Ready to merge = Yes。
合并后必修项（三处 Important 与四处护栏）已在 Task 11、Task 12 落地，未列入下方清单。

---

# P0 基础设施实现进度

计划：docs/superpowers/plans/2026-09-30-p0-infrastructure.md
基线：230fef9

Task 1: complete (commits 8b2f85d..8391e98, review clean: spec OK / quality Approved)
  Minor 交最终评审分诊：
   1. dependency_direction.rs:23 两包共用一个 #[test]，core 失败会遮蔽 persist。brief 规定此形，未改。
   2. dependency_direction.rs:27 子串匹配 "continuum-provider"，未来同前缀 crate 会误报。
   3. dependency_direction.rs:9-13 测试内嵌套 cargo 调用，环境问题会表现为测试失败。
   4. 报告 :47 的 RED 是 workspace 尚未建立，不是断言变红；有效证据是变异测试。
   5. 报告 :91 称"未修改任何既有文件"，与实际 diff 含计划文档不符；来源是合并提交。
Task 2: complete (commits 4b88875..438a0c1, review clean after 1 Important fix)
  修复：ConnectorDescriptor/ConnectorOp 收紧，三条构造路径全受 §125 约束（裁定 C）
  Minor 交最终评审分诊：
   1. connector.rs:33 的 is_empty() 分支为死代码，!contains('.') 已覆盖空串；保留作自解释。
   2. ConnectorOp 前缀未与 ConnectorId 交叉校验；评审自评 Minor 且非本次引入，属规范层待定。
   3. ProviderError 的 Display、Role/ProviderHealth 的 snake_case 重命名仍无直接覆盖。
   4. ModelStream 未派生 Debug。
Task 3: complete (commit 119b381, review clean: spec OK / quality Approved)
  Minor 交最终评审分诊：
   1. fake_provider.rs:325-338 连接器断言只查 is_err/is_ok，未查 ProviderError::Protocol 变体；brief 原文。
   2. fake_provider.rs:296 FakeConnector::invoke 每次重建整个 ConnectorDescriptor。
   3. 测试用裸 assert 无自定义消息（未违反字面约束）。
  待办：OnceStream 从未被 poll，ModelStream 的可消费性在 P0 内无任何 task 验证；
        评审判定不属 Task 3 缺陷，但后续 task 也未必会调 stream()，需在最终评审确认是否接受该缺口。
Task 4: complete (commits 428274e..c9a231e, review clean: spec OK / quality Approved)
  控制器追加断言：serde_names_match_as_str_for_all_nine_types（跨来源比对，覆盖九个类型）
  Minor 交最终评审分诊：
   1. codec.rs:91 非对象输入得 type_str="" → Err(UnknownEventType{"")，错误信息空名；fail-closed 侧。
   2. EventType::ALL 是第三份清单且被 codec 用作已知/未知判定；新增变体漏加进 ALL 则全测试仍绿但运行期致命。
   3. with_ignorable/with_intent/with_node 零覆盖、全仓无调用点。
   4. validate_contiguous 对空链返回 Ok，语义可争议，无测试。
   5. 三处非 rustfmt 干净（仓库级断言未验证）。
Task 5: complete (commits 39c2c97..fa4ac7e, review clean: spec OK / quality Approved)
  控制器追加两条断言：AuditKind 的 serde rename↔as_str 一致性；record_hash 黄金向量。
  评审独立复算黄金向量与调序后的失败值，两者均与实现者报值逐字节一致。
  Minor 交最终评审分诊：
   1. audit.rs:111 文档称「校验顺序」，但 verify_chain 不校验 seq 严格递增；无键哈希下影响有限，但注释强于代码。
   2. tests/audit_chain.rs:13 同一函数两种写法（全路径 vs 导入名）。
   3. 黄金向量只钉正数 seq/occurred_at，负数补码编码未锁定。
   4. 【重要】若启用 serde_json 的 preserve_order，键序变为插入序依赖，所有已存审计哈希静默失效；
      黄金向量的两个键字面序恰好等于排序序，故现有测试覆盖不到。建议补一条「键序无关」断言。
      已核实当前 workspace 无 crate 启用该 feature，属潜在风险。
   5. audit.rs:105 对不可序列化 payload 用 .expect 会 panic；对 Value 不可达。
Task 6: complete (commits 43008bb..7a53ce7, review clean: spec OK / quality Approved)
  控制器追加：pragmas_are_set_as_required（三条 PRAGMA 一并断言，原测试只覆盖 journal_mode）
  评审独立核对三项：PRAGMA 断言承重、连接编译期私有、错误类型不外泄 rusqlite，全部成立。
  Minor 交最终评审分诊：
   1. migrations.rs:16 对内置迁移条数的断言自反（:57、:66 同形）；表集合约束由 :38 承担。
   2. Db::begin 文档称同线程重入「阻塞」，实为死锁（std Mutex 不可重入）。已并入 Task 7 一并修正。
   3. value.rs:31 from_ref 对 TEXT 用 from_utf8_lossy，非法 UTF-8 静默替换为 U+FFFD。
   4. as_text/kind_name/ParamType/TransactionDropped 全仓无调用点，公开面宽于 Produces 清单。
   5. 全仓不合 rustfmt（既有状态，非本 task 引入；无 rustfmt.toml 与 CI）。
Task 7: complete (commit f678f1a, review clean: spec OK / quality Approved)
  两处计划错误由执行方在 GREEN 阶段定位：append_event 漏写 ignorable 列 + json_at 把 INTEGER 读成 JSON number；
  以及 tamper 测试写裸文本进 payload 列导致解析先于校验失败。
  评审独立核对：ignorable 往返闭合、Drop 回滚且覆盖「可再次 begin」、审计 kind 写读重算三处同源。
  Minor 交最终评审分诊：
   1. with_ignorable(true) 写路径无测试保护——本 task 第一轮缺陷正是它，修复后仍无回归保护。
      补法：append 一条 with_ignorable(true) 的事件后直接查 ignorable 列断言为 1（公开 API 无事件读回路径）。
   2. tx.rs:103 的 ParamType{index:2} 参数号不正确（应为 payload 位）；仅影响错误文案，只在 Blob payload 时可达。
   3. bool_at 对非 Int 值静默取 false；SQLite 类型亲和允许 TEXT 落进 INTEGER 列。
   4. tests/transaction.rs:222 第 11 条测试未断言 decoded。
   5. audit_records 六段近似重复的 match 臂（约 50 行），可收敛为辅助函数。
   6. Drop 吞掉 ROLLBACK 错误；PersistError::TransactionDropped 全仓无使用点。
Task 8: complete (commit fbdc7fc, review clean: spec OK / quality Approved)
  本 task 未撞到计划错误；lib.rs 为纯追加，既有导出全保留。
  评审独立核对四项：§319 顺序逐字一致、阶段事务边界正确（失败阶段回滚且已提交阶段不受影响）、
  第一阶段内建工作在钩子循环之前且致命即短路、同阶段内钩子按注册顺序执行。
  Minor 交最终评审分诊：
   1. 六条测试未覆盖「失败阶段之前的阶段写入存活」的复合交错；评审判定非缺陷，结构上由 commit 先于下一个 begin 保证。
   2. tests/recovery.rs:328、:394 两处 expect_err 对错误变体不敏感。
   3. 公开面宽于 brief 的 Produces 清单：is_empty()、derive(Default)、report 类型的 Debug/Clone/PartialEq/Eq。
   4. RecoveryHook: Send + Sync 超 trait 未在 brief 签名中列出。
   5. 钩子失败向上传播时不带阶段上下文。
   6. Vec<&Box<dyn RecoveryHook>> 可简化为 Vec<&dyn RecoveryHook>。
Task 9: complete (commit fb54ae3, review clean: spec OK / quality Approved)
  计划错误 10：wait_ready 用 take() 取走读端，函数返回即关闭，夹具提交后的 println 撞断管 panic，
  对照组以退出码 101 假失败。裁定采纳执行方提出的 as_mut 变体（不依赖调用方绑定返回值）。
  评审核对：杀进程用例的结论由 READY 打印位置 + 对照组共同承载；
  外部探针（第二连接 BEGIN IMMEDIATE 得 database is locked）为补充证据，不可复现，不计入交付物。
  Minor 交最终评审分诊：
   1. crash_atomicity.rs:127-139 count 用 unwrap_or(0) 把「无行」映射为 0，而负向断言恰好期望 0；实际不可达，建议改 expect。
   2. wait_ready 无超时，夹具挂起会永久阻塞而非失败。
   3. kill/wait 前无 panic 守卫，失败时夹具残留最多 60s。
   4. stderr inherit() 绕过 harness 捕获，略噪。
   5. crash-writer.rs:66 的 COMMITTED 无消费者。
Task 10: complete (commit f1af2a7, review clean: spec OK / quality Approved)
  控制器追加：startup_reports_skipped_records（空库上「跳过记录 0 条」恒为 0，区分不了真读出与写死）
  评审独立核对：该用例确为承重，损坏行走可跳过路径得 skipped_records=1，且失败字段确为 schema_version 而非 payload。
  Minor 交最终评审分诊：
   1. startup.rs:60 五阶段用裸 contains，不断言 :N 钩子 后缀与行数（顺序由 Task 8 覆盖）。
   2. startup.rs:70-71 裸 INSERT 复制了 events 的物理列清单，未来迁移改表会在运行时失败且无编译期信号。
   3. main.rs:12 tokio/thiserror 未被引用，属 P6 清理项。
   4. 提交卫生：f1af2a7 复用了 bf4fefb 的提交信息。
   5. 失败路径（坏路径 → 退出码 1）无自动化断言。
Task 11: complete (commits 34f0713..04a1f4a, 终审修复)
  三处 Important：I1 payload 静默降级、I2 迁移链悬空、I3 迁移不在恢复内；四处护栏。
Task 12: complete (commit e1dd41f, 复核回归修复)
  复核发现 Task 11 引入 fail-open：未知类型判定在 chain.decode 内，
  「未知类型 + ignorable=0 + 坏 payload」从致命降级为可跳过。已前置判定并补用例。
  另修：schema_version 可读但非法（0/负数/超 u32）与 99 同判致命。

终审判定：Ready to merge = Yes（final-review, commit e1dd41f）
  I1/I2/I3、四处护栏、fail-open 回归均闭合；每条都有能杀掉对应变异体的用例支撑。
  终审新增两条 P1 Minor：未知类型规则现有两份实现靠注释同步；非法版本号的错误文案不含列值。

最终计数：core 5、provider 2、events 21、persist 30、runtime 4，共 62 passed，0 warning。
