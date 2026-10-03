# P1 执行层：交接事项

本文件只记执行器就位前必须知道、而代码与设计正文里看不出来的东西。**收件阶段不作指定**——
执行器归哪个阶段尚未确定，故本文一律用「执行器就位后」「接入执行器的阶段」这类措辞，不写阶段名。
完整的遗留清单在
`docs/superpowers/plans/2026-10-01-p1-execution-layer.md` 的「遗留」一节；设计正文在
`docs/superpowers/specs/2026-10-01-p1-execution-layer-design.md`。

P1 交付：`continuum-port`（Port 类型系统）、`continuum-operator`（算子定义与注册表）、
`continuum-artifact`（Artifact 模型、内容寻址、磁盘存储、落库）、`continuum-graph`
（ADFIR 图、状态机、失效传播、增量重算、调度、失败分类与重试、持久化、执行接口）、
`continuum-runtime`（装配与恢复钩子）。合入 main 时 184 passed / 0 failed / 0 warning。

---

## 一、执行器必须遵守的接口约定

这三条已写进设计正文，但都是「P1 内无消费者、执行器就位后才兑现」的约定，容易被当成注释略过。

**1. 写入函数返回 `Err` 之后，调用方必须回滚，不得提交**（设计 §16）。
`apply_transition` 与 `save_artifact` 都可能在失败前已写入部分行——先写状态/元数据、再写事件，
事件写入失败（如事件 id 撞主键）时前一步已留在事务里。函数自身提交不了（它拿不到 `Db`），
所以「先收集错误、最后统一提交」的批量写法会写进一个没有对应事件的状态变迁。
端到端的 `§318` 原子性由调用方兑现。

**2. 事件 id 由调用方分配且必须唯一**（设计 §16）。
同一次执行中节点可以多次到达同一状态（`Running → Waiting → Ready → Queued → Running` 是合法路径，
会写两条 `NodeStarted`），故不能用可由节点与状态派生的可复用 id，要用单调序号。
唯一的豁免是 `ArtifactCreated`——一个 Artifact 恰一条事件、`artifact.id` 是主键，
故 `artifact/{id}` 安全。

**3. `node_attempt` 与 `execution_profile` 共用同一套 `attempt` 编号**（设计 §15）。
键为 `(graph_id, node_id)`，自 1 起，每次尝试取 `MAX(attempt) + 1`；
`node_attempt` 与 `execution_profile` **每次尝试必须同时写**——这是恢复钩子只从
`node_attempt` 取单表 `MAX` 仍然正确的前提。若只更新一张表，两表的同一个号会指向不同次执行
（不撞主键，故不会报错，只会静默错位）。

---

## 二、P1 建好但无生产调用方的机制（接线属执行器）

除测试外无调用点。这不是缺陷——P1 没有执行器，也就没有合法的「生产」位置——
但接入执行器时必须逐个接上，否则会重演「机制建好、运行路径不经过」。

`apply_transition`、`save_graph` / `load_graph`、`ArtifactStore::{persist, restore}`、
`commit_with_content`、`BlobStore` 的落盘路径、`select_runnable` / `apply_blocking` /
`apply_unblocking`、`propagate_invalidation`、`can_reuse` / `cache_key`、`decide_retry`、
`is_terminal`、`is_candidate_backend`、`OperatorRegistry`。

P1 内真正接通的生产链只有两条：迁移注册（`main.rs`）与恢复钩子（`recovery.rs`，在 `main.rs` 注册）。

---

## 三、执行器必须处理的设计遗留

**算子解析没有执行点。** 设计 §11.2 的 Failure condition 是「注册表中不存在
`(operator_id, operator_version)` 时节点不进入 RUNNING，返回 `OperatorNotFound`」，
但 P1 里 `Node::operator` 是 `OperatorRef`、`Node::new` 不查注册表、`transition` 也不收注册表。
**接入执行器后，`Queued → Running` 是它唯一的合法落点**，必须在彼处调 `OperatorRegistry::resolve`
并做前置判定。

**`RESOURCE` 的固有策略不能取 `RetryPolicy::default()`。** 设计 §13.1 给 RESOURCE 的是
「退避后仍失败则升级」，而默认策略 `max_attempts = 1` 会在首次失败即失败，连一次退避都没有。
默认值是「未配置」的表示，不是任何类别的固有策略——P3 的 Router 必须为 RESOURCE 显式给出
`max_attempts >= 2` 与退避参数。

**`EscalationPolicy::None` 把 `CONSTRAINT` / `AUTHORIZATION` 压成 `Fail`。** 设计 §13.1 要求这两类
「立即升级为决策」，而 P1 内没有决策对象，`escalate()` 把它压成 `Fail`，语义是「无处可升级」
而非「不该升级」。引入决策落点时要一并处理，不要当成缺陷去改。

**`connect` 拒绝重复边，但 `adfir_edge` 表没有主键。** 构造层已挡住同一对端口的重复连边
（`GraphError::DuplicateEdge`），落库侧没有约束。执行器若绕过构造层直接写表需自知。

---

## 四、已知缺口与陷阱

**`ArtifactStore::persist` 是非幂等的裸 INSERT。** 与 `save_graph` 同类：同一个 id 再落一次必撞主键。
执行器每次状态变化都要落库，届时需要 UPSERT 路径。

**孤儿字节**：`commit_with_content` 落盘先于登记，登记失败时字节可能已留在盘上，
调用方无法从错误区分「字节未落盘」与「字节已落盘但未登记」。内容寻址存储是幂等的，
孤儿可被复用，故不回收，也无计数。

**`restore` 里 `commit(...).map_err` 那条分支不可达**（`id TEXT PRIMARY KEY` 排除
`AlreadyCommitted`，进入分支的守卫排除 `UnresolvedInput`）。保留为防御，属零覆盖代码。

**`invalidation` 的 EFFECT 边不参与传播，目前只由否定式间接保证**——用例名声称覆盖
但从未构造 EFFECT 边。后续若改动边集判定，这里没有守卫。

**编码约定**：枚举列（`adfir_node.state`、`node_attempt.state`/`failure_class`、
`adfir_edge.kind`、`adfir_port.direction`、`artifact.artifact_type`、`artifact.privacy_class`）
一律小写、经显式辅助函数，**不依赖 serde**（`NodeState`/`FailureClass` 的 serde 是
`SCREAMING_SNAKE_CASE`，直接反序列化会失败）。同一张表的读写函数与表定义放在同一 crate；
`continuum-runtime` 不直接对这些列写 SQL 字面量。这条是构造性质，**没有行为守卫**
（把 WHERE 的参数换回取值正确的字面量，全部用例仍绿）。

---

## 五、流程上值得沿用的做法

**计划里的代码块是历史记录，改对了要与正文分开记。** P1 的计划文本累计出过 20+ 处事实错误，
实现方在 RED 阶段先撞上并回报，没有一处流向产物。已实现 task 的段落若留有错误代码块，
要加订正注记（说明错在哪、正确的是什么），否则后续实现者照抄会复发——本次那处 Critical
正是两个 task 的代码块对同一列给了两个编码。

**变异验证要确认红的*位置*，不只看红没红。** 本波次出过一次：给提交路径新增一道 size 校验后，
另一条用例的变异不再命中它原本要验的断言，第一次跑红了但红在错的地方。**新增前置判定会改变
既有用例命中的分支**，每加一道都要回头确认受影响的用例仍钉在该钉的地方。

**变异要按整个包跑，不要只跑单个 test binary。** 终审有一次只跑了一个 test，全绿，一度疑为缺口，
改跑整包才红在正确位置。

**外部依赖的边集被 `dependency_direction` 冻结，dev-dependencies 也算直接边。**
测试若需要某个 crate，先看它是否已在直接依赖里；加 dev-dep 会改动那份被冻结的边集。

**`.superpowers/sdd/` 是 gitignore 的 scratch，随时会丢。** 跨阶段要保留的东西必须写进
`docs/`——`p0-followups.md` 的存在就是它被清掉一次的实证。
