# UI 接口契约：总览

本文档面向要为本仓库实现界面的前端开发者。它只描述界面可以调用的接口、可以读取的数据，以及这些接口和数据的约束。视觉风格、布局、交互模式不在本文档范围内。

文档基线：主仓库 `main` 分支，提交 `03e79c7`，workspace 版本 `0.1.0`。文中所有接口、字段和命令都以该提交的代码为准。

## 术语

- **Intent**：一次工作的目标、状态与关联标识，用 `IntentId` 字符串表示。
- **Artifact**：节点产出的内容对象的元数据；内容本身按 `content_hash` 存放。
- **Evidence**：验证过程产生的证据记录。
- **Node**：图中的执行节点。
- **Effect**：一次对外部世界的副作用记录。
- **Workspace**：某个 Intent 的隔离工作目录及其后端记录。
- **Event**：事件日志中的一条记录。
- **Audit**：审计日志中的一条记录。

## 1. 仓库当前提供什么

本仓库是一个 Rust workspace，对外只有两类可用接入点：

1. **CLI 子进程**：可执行文件 `continuum-runtime`，提供 `task`、`tool`、`recover` 三个子命令。
2. **SQLite 数据库**：由 CLI 通过 `--db <路径>` 打开并执行迁移；界面可以只读方式读取。

仓库中**没有** HTTP 服务、JSON-RPC 服务、WebSocket 服务、gRPC 服务或其它常驻进程接口。也没有用于查询 Intent、Artifact、Evidence、图或预算的 CLI 子命令。界面要显示这些数据，当前只能读取 SQLite。

可执行文件的实际名称是 `continuum-runtime`（`crates/continuum-runtime/Cargo.toml` 未声明 `[[bin]]`，二进制名取自包名）。CLI 的帮助文本首行写作 `continuum`（`crates/continuum-runtime/src/cli.rs:81`），这是文本中的程序名，不是安装后的文件名。

## 2. 进程模型

### 2.1 每次调用一个进程

界面每次操作都启动一个新的 `continuum-runtime` 进程，传入完整参数，等待进程结束，读取退出码、stdout 和 stderr。CLI 不维护跨调用的内存状态，所有持久状态都在 `--db` 指定的 SQLite 文件中。

三个子命令的进程行为：

| 子命令 | 是否启动子进程 | 说明 |
|---|---|---|
| `task` | 是 | 在沙箱内执行 `--exec` 指定的命令；overlay 后端下还可能先通过 `unshare -Urm` 重新执行自身。 |
| `tool` | 否 | 在当前进程内调用已经登记的 ToolProvider；当前生产装配为空注册表，见 §5。 |
| `recover` | 否 | 打开数据库、应用迁移、执行五个恢复阶段。 |

`task` 的重新执行发生在创建任何工作区之前：当 Base 不是 Git 仓库（overlay 后端）且当前进程不在用户命名空间内时，驱动会执行 `unshare -Urm <自身可执行文件> <原始参数>`，并设置环境变量 `CONTINUUM_IN_NAMESPACE=1` 防止递归（`crates/continuum-runtime/src/task_cmd.rs:187`、`:587`）。界面只需等待最外层进程结束；中间进程的 stdout 和 stderr 会继承到同一输出通道。

### 2.2 参数传递

界面应使用操作系统提供的进程启动接口，把参数作为独立的 argv 元素传入，不要拼接成一个字符串再交给 shell。CLI 按 argv 边界解析参数，不进行 shell 展开、通配符展开、引号去除或变量替换。

`task` 的 `--exec` 会消费其后的全部 argv，作为子进程的完整命令。`--exec` 之后的任何 token 都属于该命令，即使它与 CLI 选项同名。所有 CLI 选项必须写在 `--exec` 之前（`crates/continuum-runtime/src/cli.rs:43`）。

### 2.3 输出通道

| 子命令 | stdout | stderr |
|---|---|---|
| `task` | 驱动自身不输出；`--exec` 子进程的 stdout 直接继承。 | 驱动失败时输出一行错误；子进程的 stderr 直接继承。 |
| `tool` | 工具成功返回时输出一行紧凑 JSON，即 `ToolResult.output`；工具自报失败时也先输出这一行。 | 驱动失败时输出一行错误。 |
| `recover` | 固定输出迁移数量、跳过记录数、标记 LOST 的节点数，以及五个恢复阶段的钩子数。 | 失败时输出一行错误。 |

`task` 和 `tool` 的 stdout 可能包含业务数据，也可能为空。界面不应假定 stdout 一定是 JSON；只有 `tool` 在工具实际被调用并返回时，stdout 才是 `ToolResult.output` 的 JSON 表示。

### 2.4 退出码

当前只有两个退出码：

| 退出码 | 含义 |
|---|---|
| `0` | 子命令成功。 |
| `1` | 任何失败：参数错误、启动失败、任务失败、工具调用失败、恢复失败。 |

`task` 的 `--exec` 子进程以非零码结束时，驱动会把它转换为自己的失败，最终退出码仍是 `1`。子进程的原始退出码只出现在 stderr 的错误文本中，不会作为驱动退出码返回（`crates/continuum-runtime/src/task_cmd.rs:468`、`crates/continuum-runtime/src/main.rs:54`）。

CLI 没有为不同错误分配不同退出码，也没有机器可读的错误码字段。界面分类错误时应优先依据 stderr 的固定前缀和数据库状态，不要依赖完整错误文本的逐字匹配。

### 2.5 环境变量

驱动读取以下环境变量：

| 变量 | 作用 | 来源 |
|---|---|---|
| `CONTINUUM_CREDENTIALS` | 设置后作为文件凭据源路径；未设置时使用环境变量凭据源，前缀为 `CONTINUUM_SECRET_`。 | `crates/continuum-runtime/src/secrets.rs:39`、`:45`、`:59` |
| `CONTINUUM_IN_NAMESPACE` | 由 `task` 的重新执行设置，值为 `1`；界面不需要设置。 | `crates/continuum-runtime/src/task_cmd.rs:173` |
| `CONTINUUM_OVERLAY_ROOT` | overlay 后端的根目录；未设置时使用 `$HOME/.local/share/continuum/overlays`。 | `crates/continuum-workspace/src/overlay.rs:22`、`:163` |

密钥运行时在参数解析成功之后装配，但当前没有消费者（`crates/continuum-runtime/src/main.rs:38`）。`CONTINUUM_CREDENTIALS` 指向的文件读不出来时，参数解析成功的任何子命令都会在启动阶段失败，stderr 前缀为 `启动失败:`；参数解析失败时先报参数错误，不读取该变量。

## 3. 数据流

### 3.1 `task`

```
界面
  -> 启动 continuum-runtime task --base ... --intent ... --db ... --exec ...
  -> 解析参数
  -> 判定 Base 后端（.git 是否存在）
  -> 必要时 unshare -Urm 重新执行自身
  -> 打开数据库并应用迁移
  -> 创建 Task 工作区
  -> 写 workspace 表
  -> 读取 policy 表，做集成裁决与逐条效应裁决
  -> 写 effect 表（PLANNED -> AUTHORIZED -> EXECUTING）及审计行
  -> 在沙箱内运行 --exec
  -> 写 effect 终态（COMMITTED 或 FAILED）
  -> 若给了 --apply 且命令成功：按裁决决定是否集成；集成时写审计行
  -> 否则放弃工作区并删除 workspace 记录
```

来源：`crates/continuum-runtime/src/task_cmd.rs:180`。

`task` 不创建 `intent`、`adfir_graph`、`artifact` 或 `evidence` 行。它只使用调用方给出的 Intent 字符串作为工作区标识和效应幂等键的一部分。因此界面可能看到 `workspace` 行，但 `intent` 表中没有对应行。

### 3.2 `tool`

```
界面
  -> 启动 continuum-runtime tool --db ... --tool ... [--input ...] [--effect ...]...
  -> 解析参数
  -> 打开数据库并应用迁移
  -> 幂等键预检
  -> 读取 policy 表
  -> 逐条效应裁决并铸造能力
  -> 读 tool 表，执行授权检查
  -> 写 effect 表（PLANNED -> AUTHORIZED -> EXECUTING）及审计行
  -> 调用 ProviderRegistry::invoke_tool
  -> 输出 ToolResult.output
  -> 写 effect 终态（COMMITTED 或 FAILED）
```

来源：`crates/continuum-runtime/src/tool_call.rs:387`。

当前生产装配使用空注册表（`crates/continuum-runtime/src/tool_cmd.rs:44`）。授权检查通过时，真实调用会在最后一步之前以 `ToolCallError::Unregistered` 失败；若工具声明了必需能力而本次调用没有出示，则会更早以能力错误失败。前一种情况下，已声明的效应已经写入并处于 `EXECUTING`，随后被写为 `FAILED`。没有声明效应时，不会写 `effect` 行。

### 3.3 `recover`

```
界面
  -> 启动 continuum-runtime recover --db ...
  -> 打开数据库并应用迁移
  -> 打印“迁移应用 N 项”
  -> 执行五个恢复阶段：
       1. load durable state
       2. reconcile incomplete effects
       3. reconcile running nodes
       4. mark lost executions
       5. resume eligible tasks
  -> 打印跳过记录数、标记 LOST 的节点数、各阶段钩子数
```

来源：`crates/continuum-runtime/src/recover_cmd.rs:38`、`crates/continuum-persist/src/recovery.rs:86`。

### 3.4 事件

`task`、`tool`、`recover` 三个 CLI 路径**都不写 `events` 表**。事件由库函数写入，例如 `save_intent`、`apply_intent_transition`、`apply_transition`、`save_artifact`、`record_verification_round`、`create_decision`。这些函数当前没有 CLI 入口。

因此，界面通过 CLI 操作后，`events` 表可能为空。事件时间线只反映已经被库层写入的事件，不能单独用来重建全部状态。当前状态的权威来源是各实体表，事件用于补充时间线。

## 4. 当前可用的接入方式

| 能力 | 状态 | 接口 |
|---|---|---|
| 运行一次任务命令 | 可用 | `continuum-runtime task` |
| 调用一个已登记工具 | 接口存在，当前不可成功 | `continuum-runtime tool`；生产注册表为空。 |
| 执行启动恢复 | 可用 | `continuum-runtime recover` |
| 读取工作区记录 | 可用 | 只读 SQLite `workspace` 表。 |
| 读取效应记录 | 可用 | 只读 SQLite `effect` 表。 |
| 读取策略、工具登记、模型登记 | 可用 | 只读对应表。 |
| 读取事件与审计 | 可用 | 只读 `events`、`audit_log` 表。 |
| 读取 Artifact、Evidence、图、Intent、预算 | 可用 | 只读对应表；但这些表当前没有 CLI 写入路径，内容取决于是否有其它库调用方写入。 |
| 查询类 CLI 命令 | 不可用 | 没有 `list`、`show`、`status` 等子命令。 |
| HTTP / RPC 服务 | 不可用 | 仓库中不存在服务端实现。 |
| 事件订阅或推送 | 不可用 | 没有长连接、轮询接口或游标接口。 |
| 取消正在运行的 `task` | 不可用 | 没有取消子命令；界面只能终止自己启动的进程。 |
| 进度上报 | 不可用 | CLI 不输出结构化进度。 |
| 版本查询 | 不可用 | 没有 `--version`；`--help` 也不是已实现的选项。 |

## 5. 未实现范围

以下内容在本仓库中没有实现。界面不得按“将来会有”的方式调用。

| 项目 | 状态 | 说明 |
|---|---|---|
| Memory（P6） | 未实现 | 仓库中没有 Memory crate、表或接口。 |
| HTTP / API 服务层 | 未实现 | 没有服务端二进制或路由。 |
| CLI 查询命令 | 未实现 | 只有 `task`、`tool`、`recover`。 |
| 生产工具适配器 | 未实现 | `ProviderRegistry::new()` 为空；`tool` 无法成功调用任何工具。 |
| `plan.review_required` 事件生产方 | 未实现 | 事件类型已声明，代码中没有写入点。 |
| `effect.committed` 事件生产方 | 未实现 | 事件类型已声明，代码中没有写入点。 |
| 模型调用 CLI | 未实现 | `continuum-runtime::model_call` 是库 API，没有子命令。 |
| 验证 CLI | 未实现 | `continuum-verify` 是库，没有子命令。 |
| 预算 CLI | 未实现 | `continuum-budget` 是库，没有子命令。 |
| 证据读取 API | 未实现 | 没有 `load_evidence`；只能读表。 |
| 决策结果读取 API | 未实现 | `decision_result` 表只写不读。 |
| Artifact 内容读取 CLI | 未实现 | 有 `BlobStore` 库类型，但没有 CLI；数据库不记录 blob 根目录。 |
| `execution_profile` 表读写 | 未实现 | 表由迁移创建，当前没有生产读写路径。 |

## 6. 版本与兼容

### 6.1 CLI

CLI 没有版本协商，也没有 `--version`。界面应把可执行文件路径作为部署配置，并在启动时通过一次无害调用确认它可执行。参数或行为变化只能通过重新阅读本目录文档确认。

### 6.2 事件

事件记录的 `schema_version` 当前为 `1`（`crates/continuum-events/src/event.rs:9`）。解码器链只注册了版本 `1`（`crates/continuum-events/src/codec.rs:124`）。版本 `1` 的记录可以直接按本目录 `02-events-and-state.md` 的字段读取。

### 6.3 数据库

数据库迁移版本从 `1` 到 `130`，共 13 个迁移项。`Db::migrate()` 按版本升序应用尚未记录在 `schema_migrations` 中的迁移；已应用的迁移不会重复执行（`crates/continuum-persist/src/db.rs:96`）。支持的升级方向是旧库由新二进制打开；没有降级保证，也没有数据库版本号字段。

数据库 schema 不是版本化的公开 API。本文档记录的是提交 `03e79c7` 的实际 schema。界面应把 schema 差异当作部署错误处理，而不是在运行时猜测列的含义。

## 7. 文档索引

| 文档 | 内容 |
|---|---|
| `01-cli.md` | 三个子命令的参数、输出、退出码和调用序列。 |
| `02-events-and-state.md` | 事件类型、事件结构、Intent/Node/Effect 状态机。 |
| `03-data-model.md` | 界面可能显示的实体、字段、枚举和关系。 |
| `04-errors.md` | 错误来源、错误文本、失败分类和展示映射。 |
| `05-database.md` | 表清单、关键列、只读约束和迁移版本。 |

## 8. 来源索引

| 主题 | 代码位置 |
|---|---|
| CLI 用法与参数解析 | `crates/continuum-runtime/src/cli.rs` |
| 子命令分派与退出码 | `crates/continuum-runtime/src/main.rs` |
| `task` 流程 | `crates/continuum-runtime/src/task_cmd.rs` |
| `tool` 流程 | `crates/continuum-runtime/src/tool_call.rs`、`crates/continuum-runtime/src/tool_cmd.rs` |
| `recover` 流程 | `crates/continuum-runtime/src/recover_cmd.rs` |
| 错误类型 | `crates/continuum-runtime/src/error.rs` |
| 事件与审计 | `crates/continuum-events/src/` |
| 事务、迁移、恢复 | `crates/continuum-persist/src/` |
| 工作区 | `crates/continuum-workspace/src/` |
| 效应 | `crates/continuum-effect/src/` |
| 图与节点状态 | `crates/continuum-graph/src/` |
| Artifact | `crates/continuum-artifact/src/` |
| Evidence 与验证 | `crates/continuum-verify/src/` |
| 预算 | `crates/continuum-budget/src/` |
| 语义层 | `crates/continuum-semantics/src/` |
