# 事件与状态

本文档描述事件记录、事件类型、审计记录，以及 Intent、Node、Effect 三个状态机。界面可以用这些数据构建时间线和状态视图。

所有内容对应提交 `03e79c7`。

## 术语

- **Intent**：一次工作的目标、状态与关联标识。
- **Node**：图中的执行节点。
- **Effect**：一次对外部世界的副作用记录。
- **Artifact**：节点产出的内容对象的元数据。
- **Evidence**：验证过程产生的证据记录。
- **Event**：`events` 表中的一条事件记录。
- **Audit**：`audit_log` 表中的一条审计记录。

## 1. 事件记录

### 1.1 Rust 结构

```rust
pub struct Event {
    pub event_id: String,
    pub event_type: EventType,
    pub schema_version: u32,
    pub occurred_at: i64,
    pub intent_id: Option<String>,
    pub node_id: Option<String>,
    pub ignorable: bool,
    pub payload: serde_json::Value,
}
```

来源：`crates/continuum-events/src/event.rs:63`。

`Event::new` 把 `schema_version` 设为当前版本 `1`，把 `intent_id`、`node_id` 设为 `None`，把 `ignorable` 设为 `false`。调用方通过 `with_intent`、`with_node`、`with_ignorable` 补充可选字段（`crates/continuum-events/src/event.rs:81`）。

### 1.2 JSON 表示

事件被序列化时，`intent_id` 和 `node_id` 为 `None` 会省略，其余字段总是出现：

```json
{
  "event_id": "intent/demo-001/created",
  "event_type": "intent.created",
  "schema_version": 1,
  "occurred_at": 1760000000000,
  "intent_id": "demo-001",
  "ignorable": false,
  "payload": {
    "intent_id": "demo-001"
  }
}
```

字段类型：

| 字段 | 类型 | 说明 |
|---|---|---|
| `event_id` | string | 事件主键，全局唯一。 |
| `event_type` | string | 九个封闭取值之一，见 §2。 |
| `schema_version` | integer | 当前恒为 `1`。 |
| `occurred_at` | integer | Unix 毫秒。 |
| `intent_id` | string，可省略 | 事件所属 Intent；不是所有事件都有。 |
| `node_id` | string，可省略 | 事件所属节点；不是所有事件都有。 |
| `ignorable` | boolean | 未知类型在旧解码器中是否可跳过。九个既有类型恒为 `false`。 |
| `payload` | 任意 JSON | 事件类型决定其结构。 |

### 1.3 数据库列

`events` 表：

| 列 | 类型 | 可空 | 说明 |
|---|---|---|---|
| `event_id` | TEXT | 否 | 主键。 |
| `event_type` | TEXT | 否 | 事件类型字符串。 |
| `schema_version` | INTEGER | 否 | 版本号。 |
| `occurred_at` | INTEGER | 否 | Unix 毫秒。 |
| `intent_id` | TEXT | 是 | 可空。 |
| `node_id` | TEXT | 是 | 可空。 |
| `ignorable` | INTEGER | 否 | `0` 或 `1`。 |
| `payload` | TEXT | 否 | JSON 文本。 |

迁移定义：`crates/continuum-persist/src/db.rs:32`。

索引：`idx_events_type(event_type)`、`idx_events_intent(intent_id)`、`idx_events_occurred(occurred_at)`。

### 1.4 事件顺序

`events` 表没有自增序号列，也没有 `seq` 字段。`event_id` 是业务主键，不保证按时间递增。

库内扫描事件日志时使用 `ORDER BY rowid ASC`（`crates/continuum-persist/src/tx.rs:293`）。界面读取时间线时也应使用 `rowid` 排序：

```sql
SELECT rowid, event_id, event_type, occurred_at, intent_id, node_id, payload
FROM events
ORDER BY rowid ASC;
```

`rowid` 是 SQLite 的隐式行号，不是 schema 的一部分。当前代码不执行 `VACUUM`，但没有约束禁止外部工具执行。界面不应把 `rowid` 持久化为业务 ID。

`occurred_at` 由调用方提供，不是数据库自动生成。它可能出现相同值、回退值或 `0`。因此：

- 不要用 `occurred_at` 单独排序。
- 不要把 `occurred_at` 当作单调时钟。
- 排序使用 `rowid`；显示时间使用 `occurred_at`。

### 1.5 解码规则

`continuum-events` 的解码器链当前只注册版本 `1`（`crates/continuum-events/src/codec.rs:124`）。库内扫描按以下规则判定（`crates/continuum-persist/src/tx.rs:293`）：

| 条件 | 结果 |
|---|---|
| `event_type` 未知且 `ignorable = 0` | 致命错误，扫描失败。 |
| `event_type` 未知且 `ignorable = 1` | 记为可跳过。 |
| `payload` 不是合法 JSON | 记为可跳过。 |
| `schema_version` 不在解码链中 | 致命错误。 |
| `schema_version` 不是 1 到 `u32::MAX` 的整数 | 致命错误。 |
| 任意可跳过记录之后出现 `intent.completed` | 致命错误。 |

界面直接读表时，不要求实现这套解码器。但如果界面要判断事件是否完整，应按同一规则处理：遇到未知类型时不要当作已知类型显示；遇到不可解析的 payload 时显示原始文本和“不可解码”。

## 2. 事件类型

`EventType` 共有九个变体（`crates/continuum-events/src/event.rs:13`）。截至提交 `03e79c7`，七个类型有写入点，两个类型没有写入点。

| 事件类型 | payload | 事件行的 `intent_id` | 事件行的 `node_id` | 写入点 | 状态 |
|---|---|---|---|---|---|
| `intent.created` | `{"intent_id": "<id>"}` | 有 | 无 | `save_intent` 首次插入 | 已实现 |
| `plan.review_required` | 无定义 | 无 | 无 | 无 | 仅声明 |
| `node.started` | `{"graph_id": "<id>", "node_id": "<id>"}` | 无 | 有 | `apply_transition`，目标状态 `RUNNING` | 已实现 |
| `node.completed` | `{"graph_id": "<id>", "node_id": "<id>"}` | 无 | 有 | `apply_transition`，目标状态 `COMPLETED` | 已实现 |
| `artifact.created` | `{"artifact_id": "<id>"}` | 无 | 无 | `save_artifact` | 已实现 |
| `verification.failed` | 见 §2.1 | 无 | 有 | `record_verification_round`，聚合为 `failed` 或 `adjudication_required` | 已实现 |
| `decision.required` | `{"decision_id": "<id>"}` | 有 | 无 | `create_decision` | 已实现 |
| `effect.committed` | 无定义 | 无 | 无 | 无 | 仅声明 |
| `intent.completed` | `{"intent_id": "<id>"}` | 有 | 无 | `apply_intent_transition`，目标状态 `COMPLETED` | 已实现 |

来源：`crates/continuum-events/src/event.rs:34`、`crates/continuum-semantics/src/persist.rs:403`、`crates/continuum-graph/src/transition_tx.rs:82`、`crates/continuum-artifact/src/persist.rs:80`、`crates/continuum-verify/src/persist.rs:328`、`crates/continuum-semantics/src/persist.rs:1189`、`crates/continuum-semantics/src/intent_tx.rs:106`。

### 2.1 `verification.failed` 的 payload

```json
{
  "round_id": "<id>",
  "graph_id": "<id>",
  "verified_node": "<id>",
  "aggregate": "failed",
  "independence": "not_independent",
  "verdicts": ["pass", "fail", "unknown"]
}
```

字段说明：

| 字段 | 类型 | 取值 |
|---|---|---|
| `round_id` | string | 验证轮次 ID。 |
| `graph_id` | string | 图 ID。 |
| `verified_node` | string | 被验证节点 ID。 |
| `aggregate` | string | `failed` 或 `adjudication_required`；事件只在这两种聚合下产生。 |
| `independence` | string | `independent`、`not_independent`、`not_established`。 |
| `verdicts` | string 数组 | 逐项判定，取值 `pass`、`fail`、`unknown`。 |

来源：`crates/continuum-verify/src/persist.rs:328`。

### 2.2 事件 ID 规则

| 事件 | 事件 ID | 来源 |
|---|---|---|
| `intent.created` | `intent/<intent_id>/created` | `crates/continuum-semantics/src/persist.rs:405` |
| `decision.required` | `decision/<decision_id>/required` | `crates/continuum-semantics/src/persist.rs:1191` |
| `node.started` / `node.completed` | 调用方提供，必须唯一 | `crates/continuum-graph/src/transition_tx.rs:53` |
| `artifact.created` | 调用方提供，必须唯一 | `crates/continuum-artifact/src/persist.rs:48` |
| `verification.failed` | 调用方提供，必须唯一 | `crates/continuum-verify/src/persist.rs:271` |
| `intent.completed` | 调用方提供，必须唯一 | `crates/continuum-semantics/src/intent_tx.rs:83` |

`events.event_id` 是主键。重复 ID 的写入会失败。调用方提供的 ID 不应由状态派生，因为同一节点或 Intent 可以多次进入同一状态。

## 3. 审计记录

### 3.1 结构

```rust
pub struct AuditRecord {
    pub seq: i64,
    pub kind: AuditKind,
    pub occurred_at: i64,
    pub payload: serde_json::Value,
    pub prev_hash: String,
    pub record_hash: String,
}
```

来源：`crates/continuum-events/src/audit.rs:64`。

数据库列：

| 列 | 类型 | 说明 |
|---|---|---|
| `seq` | INTEGER | 自增主键。 |
| `kind` | TEXT | 审计类型字符串。 |
| `occurred_at` | INTEGER | Unix 毫秒。 |
| `payload` | TEXT | JSON 文本。 |
| `prev_hash` | TEXT | 前一条记录的 `record_hash`。 |
| `record_hash` | TEXT | 本条记录的 SHA-256。 |

迁移定义：`crates/continuum-persist/src/db.rs:49`。

### 3.2 审计类型

`AuditKind` 共有八个变体（`crates/continuum-events/src/audit.rs:18`）：

| 类型字符串 | 当前写入点 | 状态 |
|---|---|---|
| `authority changes` | 无 | 仅声明 |
| `device join` | 无 | 仅声明 |
| `device revoke` | 无 | 仅声明 |
| `capability grants` | `authorize` 成功时 | 已实现 |
| `external effects` | 效应登记与状态推进；工作区放弃 | 已实现 |
| `contract changes` | `save_contract` | 已实现 |
| `user approvals` | 集成写入；决策结果写入 | 已实现 |
| `runtime self-update` | 无 | 仅声明 |

### 3.3 payload 形式

审计 payload 有两种形式：

1. **JSON 对象**：效应、能力授权、Contract、决策结果。
2. **一行文本**：工作区 Gate 的操作记录。写入时不是合法 JSON，因此 `append_audit` 把它存为 JSON 字符串。

界面解析 `payload` 时应先判断 JSON 类型：

- 对象：按字段读取。
- 字符串：直接显示字符串内容，不要再解析为对象。

#### 效应登记

```json
{
  "effect_id": "<id>",
  "effect_type": "publish",
  "target": "/releases/demo",
  "idempotency_key": "<key>",
  "state": "planned"
}
```

来源：`crates/continuum-effect/src/journal.rs:59`。

#### 效应状态推进

```json
{
  "effect_id": "<id>",
  "from": "planned",
  "to": "authorized"
}
```

来源：`crates/continuum-effect/src/journal.rs:117`。

一条正常完成的效应通常产生四条 `external effects` 审计：`planned` 登记、`authorized`、`executing`、`committed` 或 `failed`。进程在 `executing` 时崩溃的情况下，恢复钩子会再写一条 `executing -> unknown` 的审计。

#### 能力授权

```json
{
  "tool_id": "<id>",
  "capabilities": [
    {
      "kind": "registry_publish",
      "scope": "/releases/demo"
    }
  ]
}
```

来源：`crates/continuum-capability/src/registry.rs:153`。

#### Contract 变更

```json
{
  "contract_id": "<id>",
  "version": 2,
  "intent_id": "<id>"
}
```

来源：`crates/continuum-semantics/src/persist.rs:927`。

#### 决策结果

```json
{
  "decision_id": "<id>",
  "option_index": 0
}
```

来源：`crates/continuum-semantics/src/persist.rs:1312`。

#### 工作区操作

文本形式：

```text
<operation> intent=<intent_id> backend=<worktree|overlay> base=<base_path>[ <detail>]
```

`operation` 取值：

| 操作 | `AuditKind` | 说明 |
|---|---|---|
| `discard` | `external effects` | 放弃 Task 工作区。 |
| `apply_patch` | `user approvals` | 把 Task 改动应用到 Base。 |
| `cherry_pick` | `user approvals` | 摘取指定提交；`detail` 为 `commits=<逗号分隔>`。 |
| `merge` | `user approvals` | 合并 Task 分支。 |

来源：`crates/continuum-workspace/src/gate.rs:396`、`:165`、`:219`、`:300`、`:345`。

### 3.4 哈希链

每条审计记录的 `record_hash` 由以下输入按固定顺序计算：

```text
域前缀 "continuum.audit.v1"
prev_hash
seq（大端 8 字节）
kind 字符串
occurred_at（大端 8 字节）
payload 的 JSON 序列化
```

来源：`crates/continuum-events/src/audit.rs:94`。

链首的 `prev_hash` 是 64 个 `0`（`crates/continuum-events/src/audit.rs:11`）。`recover` 会校验整条链；校验失败时命令以退出码 `1` 结束。

界面展示审计记录时不需要重算哈希。若需要显示“链是否完整”，应以 `recover` 的退出码为准，而不是自行比较部分记录。

## 4. Intent 状态机

### 4.1 状态集

`IntentStatus` 共有 13 个状态（`crates/continuum-semantics/src/intent.rs:61`）。数据库 `intent.status` 使用小写字符串（`crates/continuum-semantics/src/intent.rs:138`）：

| 枚举 | 数据库字符串 |
|---|---|
| `Proposed` | `proposed` |
| `Specifying` | `specifying` |
| `Planning` | `planning` |
| `Review` | `review` |
| `Ready` | `ready` |
| `Running` | `running` |
| `Waiting` | `waiting` |
| `Suspended` | `suspended` |
| `Verifying` | `verifying` |
| `Completed` | `completed` |
| `Failed` | `failed` |
| `Cancelled` | `cancelled` |
| `Evolving` | `evolving` |

### 4.2 迁移表

`IntentStatus::transition` 的合法迁移如下（`crates/continuum-semantics/src/intent.rs:217`）：

| 从 | 到 |
|---|---|
| `proposed` | `specifying`、`cancelled` |
| `specifying` | `planning`、`proposed`、`cancelled` |
| `planning` | `review`、`suspended`、`cancelled` |
| `review` | `ready`、`planning`、`cancelled` |
| `ready` | `running`、`cancelled` |
| `running` | `waiting`、`suspended`、`verifying`、`failed`、`cancelled`、`running` |
| `waiting` | `running`、`suspended`、`cancelled`、`waiting` |
| `suspended` | `running`、`cancelled` |
| `verifying` | `completed`、`failed`、`running` |
| `completed` | `evolving`，且仅当 `kind = product` |
| `evolving` | `running`、`cancelled` |

`failed` 与 `cancelled` 没有出边。

两条自环是合法的：`running -> running`、`waiting -> waiting`。其余自环一律非法。

### 4.3 可见性

- 当前状态：`intent.status`。
- 状态迁移到 `completed` 时写一条 `intent.completed` 事件。
- 其余状态迁移只更新 `intent.status` 与 `intent.updated_at`，不写事件。
- 首次落库写一条 `intent.created` 事件。

因此，`events` 表不能完整重建 Intent 的状态历史。界面应读取 `intent` 表的当前状态，并用事件补充创建与完成两个时点。

### 4.4 Intent 类型

`intent.kind` 的取值（`crates/continuum-semantics/src/intent.rs:36`、`:102`）：

| 枚举 | 数据库字符串 |
|---|---|
| `Task` | `task` |
| `Product` | `product` |
| `Maintenance` | `maintenance` |
| `Research` | `research` |
| `Background` | `background` |
| `RuntimeImprovement` | `runtime_improvement` |

`completed -> evolving` 只允许 `product`。其他类型执行该迁移会得到 `NotProduct { kind }`。

## 5. Node 状态机

### 5.1 状态集

`NodeState` 共有 13 个状态（`crates/continuum-graph/src/node.rs:56`）。数据库 `adfir_node.state` 使用小写字符串（`crates/continuum-graph/src/persist.rs:346`）：

| 枚举 | 数据库字符串 |
|---|---|
| `Pending` | `pending` |
| `Ready` | `ready` |
| `Queued` | `queued` |
| `Running` | `running` |
| `Waiting` | `waiting` |
| `Blocked` | `blocked` |
| `Suspended` | `suspended` |
| `Verifying` | `verifying` |
| `Completed` | `completed` |
| `Failed` | `failed` |
| `Cancelled` | `cancelled` |
| `Invalidated` | `invalidated` |
| `Lost` | `lost` |

注意：`NodeState` 的 serde 表示是 `SCREAMING_SNAKE_CASE`，但数据库列不是。界面读数据库时按上表的小写值处理。

### 5.2 迁移表

`transition` 的规则（`crates/continuum-graph/src/state.rs:13`）：

| 从 | 到 |
|---|---|
| 任意状态 | `invalidated`、`cancelled` |
| `pending` | `ready`、`blocked` |
| `ready` | `blocked`、`queued` |
| `queued` | `running` |
| `running` | `verifying`、`waiting`、`blocked`、`suspended`、`failed`、`lost` |
| `waiting` | `ready`、`suspended` |
| `blocked` | `ready`、`suspended` |
| `suspended` | `ready` |
| `verifying` | `completed`、`failed`、`lost` |

`invalidated` 与 `cancelled` 可以由任意状态到达，包括彼此和终态。因此“终态”不等于“没有出边”。

终态集合：`completed`、`failed`、`cancelled`、`invalidated`、`lost`（`crates/continuum-graph/src/state.rs:60`）。

### 5.3 可见性

- 当前状态：`adfir_node.state`。
- 迁移到 `running` 时写 `node.started`。
- 迁移到 `completed` 时写 `node.completed`。
- 其余迁移只更新 `adfir_node.state`，不写事件。

节点事件没有 `intent_id`，只有 `node_id` 和 payload 中的 `graph_id`。要按 Intent 关联节点事件，需要经 `intent.graph_id` 匹配 `graph_id`，再按 `node_id` 匹配。这个关联是文本匹配，没有数据库外键。

### 5.4 失败分类

`node_attempt.failure_class` 的取值（`crates/continuum-graph/src/failure.rs:8`、`crates/continuum-graph/src/persist.rs:370`）：

| 枚举 | 数据库字符串 | 含义 |
|---|---|---|
| `Transient` | `transient` | 暂时性失败。 |
| `Permanent` | `permanent` | 永久性失败。 |
| `Constraint` | `constraint` | 约束失败。 |
| `Authorization` | `authorization` | 授权失败。 |
| `Resource` | `resource` | 资源不足。 |
| `Verification` | `verification` | 验证失败。 |
| `Unknown` | `unknown` | 未知。 |

`FailureClass` 的 serde 表示是 `SCREAMING_SNAKE_CASE`，数据库列是小写。两者不要混用。

## 6. Effect 状态机

### 6.1 状态集

`EffectState` 共有 7 个状态（`crates/continuum-effect/src/effect.rs:85`）。数据库 `effect.state` 使用小写字符串（`crates/continuum-effect/src/persist.rs:50`）：

| 枚举 | 数据库字符串 |
|---|---|
| `Planned` | `planned` |
| `Authorized` | `authorized` |
| `Executing` | `executing` |
| `Committed` | `committed` |
| `Failed` | `failed` |
| `RolledBack` | `rolled_back` |
| `Unknown` | `unknown` |

### 6.2 迁移表

合法迁移（`crates/continuum-effect/src/effect.rs:157`）：

| 从 | 到 |
|---|---|
| `planned` | `authorized`、`failed`、`rolled_back` |
| `authorized` | `executing`、`failed`、`rolled_back` |
| `executing` | `committed`、`failed`、`unknown` |
| `unknown` | `committed`、`failed`、`rolled_back`、`unknown` |

`committed`、`failed`、`rolled_back` 没有出边。`unknown` 有出边，允许对账后继续推进，也允许 `unknown -> unknown`。

### 6.3 可见性

- 当前状态：`effect.state`。
- 状态迁移不写 `events` 表。
- 每次状态推进写一条 `audit_log`，`kind = external effects`。
- `effect` 行没有 `intent_id` 列。Intent 信息编码在 `id` 与 `idempotency_key` 中，格式为：

```text
<意图字节长度>:<意图>:<类型>:<目标>
```

示例：Intent `demo`，类型 `publish`，目标 `/site`，键为：

```text
4:demo:publish:/site
```

当前两个生产写入点都把 `effect.id` 与 `idempotency_key` 写成同一个值（`crates/continuum-runtime/src/tool_call.rs:581`、`crates/continuum-runtime/src/task_cmd.rs:389`）。数据库没有约束要求二者相同；界面读取时应以实际列为准。

界面按 Intent 过滤效应时，需要解码这个前缀，或者按相同的格式构造前缀。解码次序是：先读第一个冒号前的长度，再按该长度取出 Intent，然后读下一个冒号前的类型，剩余部分全部是目标。目标本身可以包含冒号。

## 7. 从事件重建状态

### 7.1 结论

**不能只靠事件重建当前状态。** 原因是事件覆盖不完整：

| 实体 | 当前状态来源 | 事件覆盖 |
|---|---|---|
| Intent | `intent.status` | 仅 `intent.created`、`intent.completed` |
| Node | `adfir_node.state` | 仅 `node.started`、`node.completed` |
| Effect | `effect.state` | 无事件；只有审计行 |
| Workspace | `workspace` 表 | 无事件；放弃与集成写审计行 |
| Artifact | `artifact` 表 | 仅 `artifact.created` |
| Evidence | `evidence` 表 | 仅失败时的 `verification.failed` |
| Decision | `decision` 表 | 仅 `decision.required` |
| Budget | `budget_node`、`budget_ledger` | 无事件、无审计 |
| Tool 登记 | `tool` 表 | 无事件；授权成功写审计 |

### 7.2 推荐读取顺序

1. 在一个只读事务中打开数据库，取得一致快照。
2. 读取实体表，得到当前状态。
3. 读取 `events`，按 `rowid` 排序，构建时间线。
4. 读取 `audit_log`，按 `seq` 排序，补充效应、工作区、授权、Contract、决策结果的操作记录。
5. 对 `effect.state = 'unknown'` 的记录显示“结果未知，需要人工核对”。
6. 对缺失的 `workspace` 记录，不要推断“任务从未运行”；可能是任务成功后被清理。用 `effect` 行与审计行区分。
7. 不要用 `events.occurred_at` 或 `audit_log.occurred_at` 做跨表排序；两张表没有共同的序列。

### 7.3 常用查询

列出全部工作区：

```sql
SELECT intent_id, backend, path, base_path, created_at
FROM workspace
ORDER BY created_at DESC;
```

列出全部效应：

```sql
SELECT id, effect_type, target, state, planned_at, updated_at
FROM effect
ORDER BY updated_at DESC;
```

列出事件时间线：

```sql
SELECT rowid, event_type, occurred_at, intent_id, node_id, payload
FROM events
ORDER BY rowid ASC;
```

列出审计时间线：

```sql
SELECT seq, kind, occurred_at, payload
FROM audit_log
ORDER BY seq ASC;
```

列出某个 Intent 的效应：

```sql
SELECT id, effect_type, target, state, planned_at, updated_at
FROM effect
WHERE idempotency_key LIKE '4:demo:%'
ORDER BY updated_at DESC;
```

上面的 `4:demo:%` 只适用于 Intent 字符串长度为 4 的情况。界面应动态计算长度前缀，或读取全部效应后在应用层解码。

### 7.4 关联限制

| 关联 | 可用字段 | 限制 |
|---|---|---|
| Intent 到 Node | `intent.graph_id` 与事件 payload 的 `graph_id` | 文本匹配，无外键；`graph_id` 可空。 |
| Intent 到 Effect | `effect.idempotency_key` 前缀 | 需要解码；`effect` 表没有 `intent_id` 列。 |
| Intent 到 Artifact | 无直接字段 | `artifact.created` 事件不带 `intent_id`；Artifact 表没有 Intent 列。 |
| Node 到 Artifact | `artifact.producer_node` | 文本匹配，无外键；可为空。 |
| Node 到 Evidence | `evidence.producer_node`、`evidence.verified_node` | 文本匹配，无外键；`producer_node` 可为空。 |
| Artifact 到 Evidence | `evidence_artifact` | 有复合主键；没有外键约束。 |
| Workspace 到 Intent | `workspace.intent_id` | 有记录；但 `intent` 表可能没有对应行。 |

界面应把这些关联显示为“按标识匹配”，不要显示为数据库保证的引用关系。
