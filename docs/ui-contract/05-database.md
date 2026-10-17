# 数据库接口

本文档描述界面直接读取 SQLite 时需要知道的表、列、迁移版本和只读约束。所有内容对应提交 `03e79c7`。

## 术语

- **Intent**：一次工作的目标、状态与关联标识。
- **Artifact**：节点产出的内容对象的元数据。
- **Evidence**：验证过程产生的证据记录。
- **Node**：图中的执行节点。
- **Effect**：一次对外部世界的副作用记录。
- **Workspace**：某个 Intent 的隔离工作目录及其后端记录。
- **Budget**：预算树节点与只追加的预算账本。

## 1. 连接

### 1.1 数据库位置

数据库路径由调用方通过 `--db <路径>` 指定。三个子命令都要求该参数：

- `task`：`crates/continuum-runtime/src/cli.rs:364`
- `tool`：`crates/continuum-runtime/src/cli.rs:450`
- `recover`：`crates/continuum-runtime/src/cli.rs:531`

驱动没有默认数据库路径，也不会从 Base 目录推导数据库位置。界面必须自己保存并传递同一个路径。`--db` 不要求绝对路径，但界面应传绝对路径，避免进程工作目录变化导致指向另一个文件。

### 1.2 打开方式

驱动使用 `Db::open_with` 打开数据库，并执行：

```sql
PRAGMA journal_mode=WAL;
PRAGMA synchronous=FULL;
PRAGMA foreign_keys=ON;
```

来源：`crates/continuum-persist/src/db.rs:72`。

随后创建 `schema_migrations` 表并应用迁移。数据库文件不存在时会被创建。

### 1.3 界面只读连接

界面应以只读方式打开数据库。SQLite URI 示例：

```text
file:/home/user/continuum.db?mode=ro
```

连接后设置：

```sql
PRAGMA query_only=ON;
PRAGMA busy_timeout=5000;
```

建议在一个读事务中完成一次界面刷新，以取得一致快照：

```sql
BEGIN DEFERRED;
-- 读取
COMMIT;
```

注意：

- 不要执行 `PRAGMA journal_mode=...`。该语句会写数据库。
- 不要执行迁移。迁移由 CLI 负责。
- 不要执行 `VACUUM`、`ANALYZE`、`REINDEX` 或任何写语句。
- 数据库处于 WAL 模式，目录中可能存在 `continuum.db-wal` 与 `continuum.db-shm`。只复制主文件可能丢失最近提交；备份应使用 SQLite 备份接口，或在写入方停止后一并复制三个文件。
- 在 WAL 模式下，如果数据库目录不可写且 `-shm` 文件不存在，只读连接可能无法打开。此时不要改成读写连接；应调整目录权限，或让用户停止写入方后复制数据库文件。
- 数据库不存在时，只读打开会失败。界面应提示用户先运行一次会创建数据库的 CLI 命令。`recover` 可以创建并迁移数据库，但它同时会执行恢复写入；不要仅为了建立只读连接而在用户不知情时调用它。

## 2. 迁移版本

迁移由 `Db::migrate()` 按版本升序应用。已记录在 `schema_migrations` 中的版本会跳过（`crates/continuum-persist/src/db.rs:96`）。

驱动装配的迁移集合见 `crates/continuum-runtime/src/main.rs:110`。

| 版本 | 名称 | 创建的表 | 定义位置 |
|---|---|---|---|
| `1` | `events` | `events` | `crates/continuum-persist/src/db.rs:27` |
| `2` | `audit_log` | `audit_log` | `crates/continuum-persist/src/db.rs:46` |
| `10` | `p1_artifact` | `artifact`、`artifact_input` | `crates/continuum-artifact/src/persist.rs:7` |
| `20` | `p1_graph` | `adfir_graph`、`adfir_node`、`adfir_port`、`adfir_edge`、`execution_profile`、`node_attempt` | `crates/continuum-graph/src/persist.rs:13` |
| `30` | `p2_workspace` | `workspace` | `crates/continuum-workspace/src/persist.rs:20` |
| `40` | `p2_effect` | `effect` | `crates/continuum-effect/src/persist.rs:26` |
| `41` | `p2_policy` | `policy` | `crates/continuum-policy/src/persist.rs:30` |
| `50` | `p3_capability` | `tool` | `crates/continuum-capability/src/persist.rs:52` |
| `80` | `p3d_model_registry` | `model_registry`、`model_profile`、`model_skill_score` | `crates/continuum-model-registry/src/persist.rs:106` |
| `100` | `p4_alias` | `alias` | `crates/continuum-canonical/src/persist.rs:83` |
| `110` | `p4_intent` | `intent`、`specification`、`contract`、`contract_requirement`、`plan`、`decision`、`decision_result` | `crates/continuum-semantics/src/persist.rs:206` |
| `120` | `p4_budget` | `budget_node`、`budget_ledger` | `crates/continuum-budget/src/persist.rs:59` |
| `130` | `p5a_verify` | `evidence`、`evidence_artifact`、`test_validity`、`test_validity_check`、`verification_round` | `crates/continuum-verify/src/persist.rs:63` |

共 13 个迁移项，32 张业务表，另有 `schema_migrations`。

### 2.1 `schema_migrations`

| 列 | 类型 | 说明 |
|---|---|---|
| `version` | INTEGER | 主键。 |
| `name` | TEXT | 迁移名称。 |
| `applied_at` | INTEGER | Unix 毫秒。 |

迁移只增不减。没有降级迁移，也没有数据库级版本号字段。

## 3. 表清单

### 3.1 `events`

事件记录。详细语义见 `02-events-and-state.md`。

| 列 | 类型 | 可空 | 说明 |
|---|---|---|---|
| `event_id` | TEXT | 否 | 主键。 |
| `event_type` | TEXT | 否 | 九个类型之一。 |
| `schema_version` | INTEGER | 否 | 当前为 `1`。 |
| `occurred_at` | INTEGER | 否 | Unix 毫秒。 |
| `intent_id` | TEXT | 是 | 可空。 |
| `node_id` | TEXT | 是 | 可空。 |
| `ignorable` | INTEGER | 否 | `0` 或 `1`。 |
| `payload` | TEXT | 否 | JSON。 |

索引：`idx_events_type(event_type)`、`idx_events_intent(intent_id)`、`idx_events_occurred(occurred_at)`。

### 3.2 `audit_log`

审计记录。详细语义见 `02-events-and-state.md`。

| 列 | 类型 | 说明 |
|---|---|---|
| `seq` | INTEGER | 自增主键。 |
| `kind` | TEXT | 八个类型之一。 |
| `occurred_at` | INTEGER | Unix 毫秒。 |
| `payload` | TEXT | JSON；可能是对象或字符串。 |
| `prev_hash` | TEXT | 前一条的 `record_hash`。 |
| `record_hash` | TEXT | 本条 SHA-256。 |

索引：`idx_audit_kind(kind)`。

### 3.3 `artifact`、`artifact_input`

```sql
CREATE TABLE artifact (
    id TEXT PRIMARY KEY,
    artifact_type TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    size INTEGER NOT NULL,
    producer_node TEXT,
    privacy_class TEXT NOT NULL,
    version INTEGER NOT NULL,
    metadata TEXT NOT NULL,
    provenance TEXT NOT NULL
);
CREATE TABLE artifact_input (
    artifact_id TEXT NOT NULL,
    input_artifact_id TEXT NOT NULL,
    PRIMARY KEY (artifact_id, input_artifact_id)
);
```

索引：`idx_artifact_hash(content_hash)`。

`metadata`、`provenance` 是 JSON。`producer_node` 可空。`content_hash` 是 64 位小写十六进制。

### 3.4 `adfir_graph`

| 列 | 类型 | 说明 |
|---|---|---|
| `id` | TEXT | 主键。 |
| `version` | INTEGER | 图版本。 |
| `contract_id` | TEXT | Contract ID。 |
| `entry_nodes` | TEXT | JSON 字符串数组。 |
| `terminal_nodes` | TEXT | JSON 字符串数组。 |

### 3.5 `adfir_node`

| 列 | 类型 | 说明 |
|---|---|---|
| `graph_id` | TEXT | 图 ID。 |
| `node_id` | TEXT | 节点 ID。 |
| `operator_id` | TEXT | 算子 ID。 |
| `operator_version` | INTEGER | 算子版本。 |
| `state` | TEXT | 小写状态字符串。 |
| `execution_policy` | TEXT | JSON。 |
| `verification_policy` | TEXT | JSON。 |
| `constraints` | TEXT | JSON 字符串数组。 |
| `capabilities` | TEXT | JSON 字符串数组。 |

主键 `(graph_id, node_id)`。

### 3.6 `adfir_port`

| 列 | 类型 | 说明 |
|---|---|---|
| `graph_id` | TEXT | 图 ID。 |
| `node_id` | TEXT | 节点 ID。 |
| `port_id` | TEXT | 端口 ID。 |
| `direction` | TEXT | `input` 或 `output`。 |
| `name` | TEXT | 端口名。 |
| `artifact_type` | TEXT | 17 个 Artifact 类型之一。 |

主键 `(graph_id, port_id)`。

### 3.7 `adfir_edge`

| 列 | 类型 |
|---|---|
| `graph_id` | TEXT |
| `from_node` | TEXT |
| `from_port` | TEXT |
| `to_node` | TEXT |
| `to_port` | TEXT |
| `kind` | TEXT |

没有主键。重复边由图层拦截，不由数据库拦截。

### 3.8 `execution_profile`

| 列 | 类型 | 可空 |
|---|---|---|
| `graph_id` | TEXT | 否 |
| `node_id` | TEXT | 否 |
| `attempt` | INTEGER | 否 |
| `backend` | TEXT | 是 |
| `timeout_ms` | INTEGER | 是 |
| `retry_policy` | TEXT | 否 |
| `cost_budget` | TEXT | 是 |

主键 `(graph_id, node_id, attempt)`。当前没有生产读写路径。

### 3.9 `node_attempt`

| 列 | 类型 | 可空 |
|---|---|---|
| `graph_id` | TEXT | 否 |
| `node_id` | TEXT | 否 |
| `attempt` | INTEGER | 否 |
| `state` | TEXT | 否 |
| `failure_class` | TEXT | 是 |

主键 `(graph_id, node_id, attempt)`。恢复钩子会追加行。

### 3.10 `workspace`

| 列 | 类型 | 说明 |
|---|---|---|
| `intent_id` | TEXT | 主键。 |
| `backend` | TEXT | `worktree` 或 `overlay`。 |
| `path` | TEXT | Task 根目录，绝对路径。 |
| `base_path` | TEXT | Base 根目录，绝对路径。 |
| `created_at` | INTEGER | Unix 毫秒。 |

一个 Intent 最多一行。`save_workspace` 是裸 `INSERT`，不覆盖。

### 3.11 `effect`

| 列 | 类型 | 说明 |
|---|---|---|
| `id` | TEXT | 主键。 |
| `effect_type` | TEXT | 六个类型之一。 |
| `target` | TEXT | 作用对象。 |
| `parameters` | TEXT | JSON；当前生产路径写 `{}`。 |
| `authorization` | TEXT | 不透明字符串。 |
| `idempotency_key` | TEXT | 唯一索引。 |
| `state` | TEXT | 七个状态之一。 |
| `planned_at` | INTEGER | Unix 毫秒。 |
| `updated_at` | INTEGER | Unix 毫秒。 |

唯一索引：`idx_effect_idempotency(idempotency_key)`。

没有 `intent_id` 列。Intent 信息在 `id` / `idempotency_key` 前缀中，格式为 `<意图字节长度>:<意图>:<类型>:<目标>`。

### 3.12 `policy`

| 列 | 类型 | 说明 |
|---|---|---|
| `id` | TEXT | 主键。 |
| `level` | TEXT | 六个层级之一。 |
| `scope` | TEXT | `user` 或 `project`。 |
| `condition` | TEXT | JSON。 |
| `decision` | TEXT | `allow`、`deny`、`require_approval`。 |

`load_policies` 按 `id` 升序读取。条件解析失败会让读取返回错误。

### 3.13 `tool`

| 列 | 类型 | 可空 | 说明 |
|---|---|---|---|
| `id` | TEXT | 否 | 主键。 |
| `version` | TEXT | 否 | 版本字符串。 |
| `input_schema` | TEXT | 否 | JSON。 |
| `output_schema` | TEXT | 否 | JSON。 |
| `required_capabilities` | TEXT | 否 | JSON 字符串数组。 |
| `effect_class` | TEXT | 是 | 六个 EffectType 之一或 `NULL`。 |
| `deterministic` | INTEGER | 否 | `0` 或 `1`。 |
| `cost` | TEXT | 是 | 文本。 |
| `latency` | TEXT | 是 | 文本。 |
| `trust` | TEXT | 否 | 文本。 |

`required_capabilities` 的元素是能力字符串，例如 `git_push`、`registry_publish`。

### 3.14 `model_registry`

| 列 | 类型 | 说明 |
|---|---|---|
| `id` | TEXT | 主键。 |
| `lifecycle_state` | TEXT | 十个状态之一。 |

### 3.15 `model_profile`

| 列 | 类型 | 可空 | 说明 |
|---|---|---|---|
| `id` | TEXT | 否 | 主键，外键到 `model_registry.id`。 |
| `version` | TEXT | 否 | 版本字符串。 |
| `provider` | TEXT | 否 | 提供方。 |
| `model_revision` | TEXT | 否 | 模型修订。 |
| `modalities` | TEXT | 否 | JSON 字符串数组。 |
| `tools` | TEXT | 否 | JSON 字符串数组。 |
| `failure_modes` | TEXT | 否 | JSON 字符串数组。 |
| `cost_profile` | TEXT | 是 | `NULL` 表示未登记。 |
| `latency_profile` | TEXT | 是 | `NULL` 表示未登记。 |
| `evidence_count` | INTEGER | 否 | 证据数量。 |
| `confidence` | TEXT | 否 | 十进制字符串。 |

### 3.15.1 `model_skill_score`

| 列 | 类型 | 说明 |
|---|---|---|
| `model_id` | TEXT | 外键到 `model_profile.id`。 |
| `dimension` | TEXT | 九个维度之一。 |
| `score_version` | INTEGER | 分数版本。 |
| `score` | TEXT | 分数。 |
| `confidence` | TEXT | 置信度。 |
| `sample_count` | INTEGER | 样本数。 |
| `time_range_start` | INTEGER | Unix 毫秒。 |
| `time_range_end` | INTEGER | Unix 毫秒。 |

主键 `(model_id, dimension, score_version)`。

### 3.16 `alias`

| 列 | 类型 | 说明 |
|---|---|---|
| `alias` | TEXT | 主键。 |
| `target` | TEXT | 目标。 |
| `scope` | TEXT | `turn`、`session`、`project`、`user_persistent`。 |

### 3.17 `intent`

| 列 | 类型 | 可空 | 说明 |
|---|---|---|---|
| `id` | TEXT | 否 | 主键。 |
| `version` | INTEGER | 否 | 版本号。 |
| `kind` | TEXT | 否 | 六个类型之一。 |
| `goal` | TEXT | 否 | 规范化目标。 |
| `status` | TEXT | 否 | 十三个状态之一。 |
| `priority` | TEXT | 否 | 不透明字符串。 |
| `created_at` | INTEGER | 否 | Unix 毫秒。 |
| `updated_at` | INTEGER | 否 | Unix 毫秒。 |
| `parent_intent` | TEXT | 是 | 父 Intent。 |
| `contract_id` | TEXT | 否 | Contract ID。 |
| `contract_version` | INTEGER | 否 | Contract 版本。 |
| `active_plan_id` | TEXT | 是 | 当前 Plan。 |
| `graph_id` | TEXT | 是 | 图 ID。 |
| `product_id` | TEXT | 是 | 产品 ID。 |
| `completion_predicate` | TEXT | 否 | 只存不判。 |

索引：`idx_intent_parent(parent_intent)`。

`task` 子命令不写这张表。`workspace.intent_id` 可能没有对应的 `intent` 行。

### 3.18 `specification`

| 列 | 类型 | 说明 |
|---|---|---|
| `intent_id` | TEXT | 复合主键第一列。 |
| `version` | INTEGER | 复合主键第二列。 |
| `goals` | TEXT | JSON 字符串数组。 |
| `invariants` | TEXT | JSON 字符串数组。 |
| `acceptance_predicates` | TEXT | JSON 字符串数组。 |
| `forbidden_states` | TEXT | JSON 字符串数组。 |
| `preferences` | TEXT | JSON 字符串数组。 |
| `unknowns_kind` | TEXT | `none_found` 或 `enumerated`。 |
| `unknowns` | TEXT | JSON 数组。 |

主键 `(intent_id, version)`。

### 3.19 `contract`

| 列 | 类型 | 说明 |
|---|---|---|
| `id` | TEXT | 复合主键第一列。 |
| `version` | INTEGER | 复合主键第二列。 |
| `intent_id` | TEXT | 所属 Intent。 |
| `budget` | TEXT | 不透明字符串。 |
| `authority_limit` | TEXT | 不透明字符串。 |
| `data_policy` | TEXT | 不透明字符串。 |
| `model_policy` | TEXT | 不透明字符串。 |
| `node_policy` | TEXT | 不透明字符串。 |
| `acceptance_predicate` | TEXT | 不透明字符串。 |

主键 `(id, version)`。

### 3.20 `contract_requirement`

| 列 | 类型 | 说明 |
|---|---|---|
| `contract_id` | TEXT | 复合主键第一列。 |
| `contract_version` | INTEGER | 复合主键第二列。 |
| `id` | TEXT | 复合主键第三列。 |
| `class` | TEXT | `required`、`preferred`、`flexible`、`unspecified`。 |
| `expression` | TEXT | 自由文本。 |
| `source` | TEXT | 自由文本。 |
| `verification_requirement` | TEXT | 自由文本，只存不判。 |

主键 `(contract_id, contract_version, id)`。

### 3.21 `plan`

| 列 | 类型 | 说明 |
|---|---|---|
| `id` | TEXT | 复合主键第一列。 |
| `major` | INTEGER | 复合主键第二列。 |
| `minor` | INTEGER | 复合主键第三列。 |
| `intent_id` | TEXT | 所属 Intent。 |
| `type` | TEXT | `strategic` 或 `execution`。 |
| `objectives` | TEXT | JSON 字符串数组。 |
| `milestones` | TEXT | JSON 字符串数组。 |
| `assumptions` | TEXT | JSON 字符串数组。 |
| `risks` | TEXT | JSON 字符串数组。 |
| `acceptance_conditions` | TEXT | JSON 字符串数组。 |
| `review_state` | TEXT | 六个状态之一。 |

主键 `(id, major, minor)`。

### 3.22 `decision`

| 列 | 类型 | 说明 |
|---|---|---|
| `id` | TEXT | 主键。 |
| `intent_id` | TEXT | 所属 Intent。 |
| `issue` | TEXT | 不透明字符串。 |
| `options` | TEXT | JSON 数组。 |
| `blocking` | INTEGER | `0` 或 `1`。 |

`options` 的元素是 `{"option": "...", "consequence": "..."}`。

### 3.23 `decision_result`

| 列 | 类型 | 可空 | 说明 |
|---|---|---|---|
| `decision_id` | TEXT | 否 | 主键。 |
| `option_index` | INTEGER | 否 | 选中项下标。 |
| `decided_at` | INTEGER | 否 | Unix 毫秒。 |
| `contract_update` | TEXT | 是 | JSON 或 `NULL`。 |
| `plan_update` | TEXT | 是 | JSON 或 `NULL`。 |

当前只写不读。

### 3.24 `budget_node`

| 列 | 类型 | 可空 | 说明 |
|---|---|---|---|
| `owner` | TEXT | 否 | 主键，预算所有者键。 |
| `parent` | TEXT | 是 | 父节点键；根为 `NULL`。 |
| `money` | INTEGER | 是 | 金额额度。 |
| `wall_time` | INTEGER | 是 | 墙钟时间额度。 |
| `token` | INTEGER | 是 | Token 额度。 |
| `gpu_time` | INTEGER | 是 | GPU 时间额度。 |
| `network_transfer` | INTEGER | 是 | 网络传输额度。 |

`NULL` 表示该维不构成约束；`0` 表示额度为零。

### 3.25 `budget_ledger`

| 列 | 类型 | 说明 |
|---|---|---|
| `owner` | TEXT | 预算所有者键。 |
| `dimension` | TEXT | 五维之一。 |
| `kind` | TEXT | `allocate`、`reserve`、`release`、`settle`。 |
| `amount` | INTEGER | 名义量。 |
| `at` | INTEGER | Unix 毫秒。 |

没有主键。只追加，不 UPDATE，不 DELETE。

### 3.26 `evidence`

| 列 | 类型 | 可空 | 说明 |
|---|---|---|---|
| `evidence_id` | TEXT | 否 | 主键。 |
| `graph_id` | TEXT | 否 | 图 ID。 |
| `verified_node` | TEXT | 否 | 被验证节点。 |
| `producer_node` | TEXT | 是 | 产生节点；`Human` 来源为 `NULL`。 |
| `subject_kind` | TEXT | 否 | `requirement` 或 `unattached`。 |
| `subject_requirement` | TEXT | 是 | Requirement ID 或 `NULL`。 |
| `evidence_type` | TEXT | 否 | 14 个类型之一。 |
| `claim` | TEXT | 否 | 自由文本。 |
| `strength` | TEXT | 否 | JSON。 |
| `scope` | TEXT | 否 | JSON。 |
| `occurred_at` | INTEGER | 否 | Unix 毫秒。 |

### 3.27 `evidence_artifact`

| 列 | 类型 | 说明 |
|---|---|---|
| `evidence_id` | TEXT | 证据 ID。 |
| `artifact_id` | TEXT | Artifact ID。 |

主键 `(evidence_id, artifact_id)`。

### 3.28 `test_validity`

| 列 | 类型 | 说明 |
|---|---|---|
| `evidence_id` | TEXT | 主键。 |
| `verdict` | TEXT | `valid`、`invalid`、`unknown`。 |

### 3.29 `test_validity_check`

| 列 | 类型 | 说明 |
|---|---|---|
| `evidence_id` | TEXT | 复合主键第一列。 |
| `check_name` | TEXT | 五个检查之一。 |
| `verdict` | TEXT | `pass`、`fail`、`unknown`。 |

主键 `(evidence_id, check_name)`。

### 3.30 `verification_round`

| 列 | 类型 | 说明 |
|---|---|---|
| `round_id` | TEXT | 主键。 |
| `graph_id` | TEXT | 图 ID。 |
| `verified_node` | TEXT | 被验证节点。 |
| `selection` | TEXT | JSON。 |
| `aggregate` | TEXT | 五个聚合判定之一。 |
| `independence` | TEXT | 三个独立性判定之一。 |
| `strategy_verdict` | TEXT | `no_counterexample` 或 `counterexample_found`。 |
| `occurred_at` | INTEGER | Unix 毫秒。 |

## 4. 枚举编码速查

| 列 | 取值 |
|---|---|
| `events.event_type` | `intent.created`、`plan.review_required`、`node.started`、`node.completed`、`artifact.created`、`verification.failed`、`decision.required`、`effect.committed`、`intent.completed` |
| `audit_log.kind` | `authority changes`、`device join`、`device revoke`、`capability grants`、`external effects`、`contract changes`、`user approvals`、`runtime self-update` |
| `artifact.artifact_type` | 见 `03-data-model.md` §2.2 |
| `artifact.privacy_class` | `public`、`personal`、`private`、`secret`、`local_only` |
| `adfir_node.state` | `pending`、`ready`、`queued`、`running`、`waiting`、`blocked`、`suspended`、`verifying`、`completed`、`failed`、`cancelled`、`invalidated`、`lost` |
| `node_attempt.failure_class` | `transient`、`permanent`、`constraint`、`authorization`、`resource`、`verification`、`unknown` |
| `workspace.backend` | `worktree`、`overlay` |
| `effect.effect_type` | `send_email`、`push_branch`、`publish`、`delete_remote`、`charge`、`deploy` |
| `effect.state` | `planned`、`authorized`、`executing`、`committed`、`failed`、`rolled_back`、`unknown` |
| `policy.level` | `system_safety`、`explicit_current`、`user_persistent`、`project`、`runtime_default`、`model_suggestion` |
| `policy.scope` | `user`、`project` |
| `policy.decision` | `allow`、`deny`、`require_approval` |
| `tool.effect_class` | 六个 EffectType 之一或 `NULL` |
| `model_registry.lifecycle_state` | `discovered`、`unprofiled`、`researched`、`probed`、`verified`、`active`、`stale`、`degraded`、`quarantined`、`disabled` |
| `alias.scope` | `turn`、`session`、`project`、`user_persistent` |
| `intent.kind` | `task`、`product`、`maintenance`、`research`、`background`、`runtime_improvement` |
| `intent.status` | `proposed`、`specifying`、`planning`、`review`、`ready`、`running`、`waiting`、`suspended`、`verifying`、`completed`、`failed`、`cancelled`、`evolving` |
| `specification.unknowns_kind` | `none_found`、`enumerated` |
| `contract_requirement.class` | `required`、`preferred`、`flexible`、`unspecified` |
| `plan.type` | `strategic`、`execution` |
| `plan.review_state` | `draft`、`self_reviewed`、`ai_reviewed`、`user_approved`、`rejected`、`superseded` |
| `budget_ledger.dimension` | `money`、`wall_time`、`token`、`gpu_time`、`network_transfer` |
| `budget_ledger.kind` | `allocate`、`reserve`、`release`、`settle` |
| `evidence.subject_kind` | `requirement`、`unattached` |
| `evidence.evidence_type` | 见 `03-data-model.md` §3.2 |
| `test_validity.verdict` | `valid`、`invalid`、`unknown` |
| `test_validity_check.check_name` | `requirement_mapping`、`assertion_strength`、`error_impl_would_pass`、`mock_bypasses_real_path`、`missing_boundary` |
| `test_validity_check.verdict` | `pass`、`fail`、`unknown` |
| `verification_round.aggregate` | `adjudication_required`、`failed`、`unknown`、`passed_with_unknown`、`passed` |
| `verification_round.independence` | `independent`、`not_independent`、`not_established` |
| `verification_round.strategy_verdict` | `no_counterexample`、`counterexample_found` |

## 5. 常用查询

### 5.1 工作区

```sql
SELECT intent_id, backend, path, base_path, created_at
FROM workspace
ORDER BY created_at DESC;
```

### 5.2 效应

```sql
SELECT id, effect_type, target, state, planned_at, updated_at
FROM effect
ORDER BY updated_at DESC;
```

### 5.3 事件

```sql
SELECT rowid, event_id, event_type, occurred_at, intent_id, node_id, payload
FROM events
ORDER BY rowid ASC;
```

### 5.4 审计

```sql
SELECT seq, kind, occurred_at, payload
FROM audit_log
ORDER BY seq ASC;
```

### 5.5 节点

```sql
SELECT node_id, operator_id, operator_version, state
FROM adfir_node
WHERE graph_id = ?
ORDER BY node_id;
```

### 5.6 证据

```sql
SELECT evidence_id, graph_id, verified_node, producer_node,
       subject_kind, subject_requirement, evidence_type,
       claim, strength, scope, occurred_at
FROM evidence
ORDER BY occurred_at DESC;
```

### 5.7 预算余额

余额不在表中，需要按账本现算。简化查询：

```sql
SELECT owner, dimension, kind, SUM(amount) AS total
FROM budget_ledger
GROUP BY owner, dimension, kind;
```

`reserved` 需要配对 `reserve` 与 `settle` / `release` 行；不能只按 `kind` 求和当作余额。完整算法见 `crates/continuum-budget/src/ledger.rs`。

## 6. 数据完整性

### 6.1 数据库约束

| 约束 | 范围 |
|---|---|
| 主键 | 大多数表有主键。 |
| 唯一索引 | `effect.idempotency_key`。 |
| 外键 | 仅 `model_profile.id` 与 `model_skill_score.model_id`。 |
| 非空 | 各表迁移中标注 `NOT NULL` 的列。 |

没有数据库约束保证以下关系：

- `workspace.intent_id` 对应 `intent.id`。
- `intent.graph_id` 对应 `adfir_graph.id`。
- `adfir_node.graph_id` 对应 `adfir_graph.id`。
- `artifact.producer_node` 对应 `adfir_node.node_id`。
- `evidence.producer_node` 对应 `adfir_node.node_id`。
- `evidence_artifact.artifact_id` 对应 `artifact.id`。
- `contract_requirement.contract_id` 对应 `contract.id`。
- `decision_result.decision_id` 对应 `decision.id`。

界面做关联时应允许“找不到对应行”，并显示为数据缺口，而不是报错。

### 6.2 追加型表

以下表只追加，不应被界面修改：

| 表 | 规则 |
|---|---|
| `events` | 只插入。 |
| `audit_log` | 只插入；哈希链依赖顺序。 |
| `artifact`、`artifact_input` | Artifact 不可变；同 ID 二次写入被拒。 |
| `evidence`、`evidence_artifact` | 只插入。 |
| `test_validity`、`test_validity_check` | 只插入。 |
| `verification_round` | 只插入。 |
| `budget_ledger` | 只追加。 |
| `contract`、`contract_requirement` | 每个版本只插入。 |

### 6.3 可更新表

以下表的指定列会被运行时更新：

| 表 | 可更新列 |
|---|---|
| `workspace` | 行会被插入或删除，不更新。 |
| `effect` | `state`、`updated_at`。 |
| `adfir_node` | `state`。 |
| `intent` | `status`、`updated_at`、`version` 等。 |
| `tool` | 无更新路径；只插入。 |
| `policy` | 无更新路径；只插入。 |

界面不应直接更新这些列。

## 7. 迁移与兼容

### 7.1 升级

新二进制打开旧数据库时，`migrate()` 会应用旧库缺少的迁移。迁移按版本升序执行，每个迁移单独一个事务。某个迁移失败时，该迁移回滚，后续迁移不执行（`crates/continuum-persist/src/db.rs:96`）。

### 7.2 降级

没有降级迁移。旧二进制打开新数据库时，不会识别新增的迁移版本，也不会删除新增的表或列。行为未定义，不应依赖。

### 7.3 Schema 稳定性

数据库 schema 没有独立的版本号字段，也没有对外承诺的兼容策略。`schema_migrations.version` 是迁移编号，不是 API 版本。

界面应把本文档的列清单作为部署时校验依据。读取前可以检查 `schema_migrations` 是否包含预期版本；缺少版本时提示先运行 CLI。

### 7.4 只读检查

界面在启动时可以执行以下只读检查：

```sql
SELECT version, name FROM schema_migrations ORDER BY version;
```

如果缺少 `130`，说明数据库尚未由当前版本的 CLI 迁移。此时不应假设 `evidence`、`verification_round` 等表存在。

## 8. 已知缺口

| 缺口 | 影响 |
|---|---|
| 没有查询 CLI | 界面必须直接读 SQLite。 |
| `effect` 表没有 `intent_id` | 按 Intent 过滤需要解码 `id` / `idempotency_key`。 |
| `workspace` 行可能在成功后保留 | 不能仅凭行存在判断任务正在运行。 |
| 没有“已集成”列 | 集成成功与否只能从 CLI 结果和审计行推断。 |
| `evidence` 没有读取 API | 只能直接读表。 |
| `decision_result` 只写不读 | 只能直接读表。 |
| Artifact blob 根目录不在库中 | 界面无法仅凭数据库定位内容文件。 |
| `execution_profile` 无读写路径 | 表存在但当前没有生产者。 |
| `adfir_edge` 无主键 | 可能存储重复边，取决于写入方。 |
| `budget_ledger` 无主键 | 同一行内容可以重复追加。 |
