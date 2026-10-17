# CLI 契约

本文档描述 `continuum-runtime` 三个子命令的参数、输出、退出码和典型调用序列。所有内容对应提交 `03e79c7`。

## 术语

- **Intent**：一次工作的目标、状态与关联标识，CLI 中以字符串形式给出。
- **Effect**：一次对外部世界的副作用记录，由 `--effect` 声明。
- **Workspace**：`task` 为某个 Intent 创建的隔离工作目录。
- **Tool**：登记在 `tool` 表中的工具登记项及其适配器。

## 1. 通用规则

### 1.1 可执行文件

二进制名：`continuum-runtime`。构建命令：

```bash
cargo build -p continuum-runtime
```

产物路径：`target/debug/continuum-runtime`。CLI 帮助文本把程序名写作 `continuum`（`crates/continuum-runtime/src/cli.rs:81`），但该名称不是构建产物名，仓库中也没有安装别名。

### 1.2 子命令集合

只有三个子命令：

| 子命令 | 用途 |
|---|---|
| `task` | 创建 Task 工作区，在沙箱内运行一条命令，按需集成。 |
| `tool` | 经工具注册表调用一个已登记的工具。 |
| `recover` | 打开数据库、应用迁移、执行五个恢复阶段。 |

没有 `--help`、`--version`、`list`、`show`、`status`、`serve` 等选项或子命令。`--help` 会按未知子命令或未知选项报错。

### 1.3 解析规则

| 规则 | 行为 |
|---|---|
| 子命令位置 | 必须是第一个参数。 |
| 选项顺序 | 除 `task` 的 `--exec` 外，选项可以任意顺序出现。 |
| `--exec` 位置 | 必须是 `task` 的最后一个选项；它消费其后的全部参数。 |
| 带值选项重复 | 报错。`--base`、`--intent`、`--db`、`--sandbox`、`--tool`、`--input` 各只接受一次。 |
| 开关选项重复 | 接受。`--apply`、`--approve` 重复出现结果不变。 |
| `--effect` 重复 | 接受，按出现次序记录。 |
| 缺失取值 | 选项后没有 token 时报错。 |
| 未知选项 | 报错，不忽略。 |
| 空参数列表 | 报错 `缺少子命令，可用：task、tool、recover`。 |
| 未知子命令 | 报错 `未知子命令：<name>，可用：task、tool、recover`。 |

解析失败时，stderr 输出两段：

```
参数错误：<CliError 的 Display 文本>
<USAGE 全文>
```

解析发生在打开数据库和装配密钥运行时之前（`crates/continuum-runtime/src/main.rs:23`）。解析失败不会创建数据库文件，也不会读取 `CONTINUUM_CREDENTIALS`。

### 1.4 错误输出前缀

| 阶段 | stderr 首行前缀 | 来源 |
|---|---|---|
| 参数解析 | `参数错误：` | `crates/continuum-runtime/src/main.rs:26` |
| 密钥运行时装配 | `启动失败: ` | `crates/continuum-runtime/src/main.rs:41` |
| `recover` 执行 | `启动失败: ` | `crates/continuum-runtime/src/main.rs:50` |
| `task` 执行 | `任务失败: ` | `crates/continuum-runtime/src/main.rs:57` |
| `tool` 执行 | `工具调用失败: ` | `crates/continuum-runtime/src/main.rs:72` |

前缀中的冒号有两套来源：`main.rs` 使用 ASCII 冒号，内层错误的 `Display` 文本可能使用全角冒号。界面不应解析冒号形式。

### 1.5 退出码

| 退出码 | 条件 |
|---|---|
| `0` | 子命令返回 `Ok(())`。 |
| `1` | 参数错误、启动失败、`task` 失败、`tool` 失败、`recover` 失败。 |

没有其它退出码。`task` 的 `--exec` 子进程退出码不会透传；它只出现在错误文本中。

## 2. `task`

### 2.1 签名

```text
continuum-runtime task
  --base <目录>
  --intent <id>
  --db <路径>
  [--apply]
  [--approve]
  [--sandbox landlock|bubblewrap]
  [--effect <类型>:<目标>]...
  --exec <命令> [参数...]
```

### 2.2 参数

| 参数 | 必填 | 默认值 | 说明 |
|---|---|---|---|
| `--base <目录>` | 是 | 无 | Base Workspace 根目录。必须是绝对路径、存在、且是目录。 |
| `--intent <id>` | 是 | 无 | Intent 标识。必须能作为单个路径分量：非空、不是 `.` 或 `..`、不含 `/` 或 `\`。 |
| `--db <路径>` | 是 | 无 | SQLite 数据库路径。文件不存在时创建；打开后应用迁移。 |
| `--exec <命令> [参数...]` | 是 | 无 | 要执行的命令。消费其后的全部参数，至少需要一个 token。 |
| `--apply` | 否 | `false` | 命令成功后请求把 Task 工作区的改动集成进 Base。 |
| `--approve` | 否 | `false` | 给出第 2 级显式批准。影响集成裁决，也影响每条 `--effect` 的裁决。 |
| `--sandbox <机制>` | 否 | 自动选择 | 取值 `landlock` 或 `bubblewrap`。未指定时优先 Landlock，不可用再选 bubblewrap；两者都不可用则失败。显式指定的机制不可用时直接失败，不改选另一个。 |
| `--effect <类型>:<目标>` | 否 | 空 | 可重复。按第一个冒号切分；目标不能为空。类型见 §2.3。 |

`--effect` 与 `--approve` 在 `task` 上**不要求**成对出现。零效应时给 `--approve` 是合法的，它只影响集成裁决。

`--exec` 之后的所有 token 都属于命令。若要在命令后写 `--apply`，必须把 `--apply` 放在 `--exec` 之前；放在后面会被当作命令的参数。

### 2.3 `--effect` 类型

类型是封闭集合，取值如下（`crates/continuum-effect/src/effect.rs:16`、`:44`）：

| 类型字符串 | 含义 |
|---|---|
| `send_email` | 发送邮件。 |
| `push_branch` | 推送分支。 |
| `publish` | 发布制品。 |
| `delete_remote` | 删除远端资源。 |
| `charge` | 发起扣费。 |
| `deploy` | 部署。 |

格式示例：

```text
--effect publish:https://example.com/releases/1
--effect push_branch:origin/main
--effect charge:acct_123
```

解析规则：

- 按第一个 `:` 切分，因此目标中可以包含 `:`。
- 类型不在上表内：解析期失败，错误文本 `未知效应类型：<name>`。
- 没有冒号：解析期失败，错误文本 `效应缺少冒号：<value>，应为 <类型>:<目标>`。
- 目标为空：解析期失败，错误文本 `效应目标为空：<value>，应为 <类型>:<目标>`。

### 2.4 执行流程

`task` 按以下次序执行（`crates/continuum-runtime/src/task_cmd.rs:180`）：

1. 校验 Base 路径，判定后端。Base 中存在 `.git` 时使用 worktree 后端，否则使用 overlay 后端。
2. 若使用 overlay 后端、当前进程不在用户命名空间内，则通过 `unshare -Urm` 重新执行自身。重新执行发生在创建任何工作区之前。
3. 打开数据库并应用迁移。
4. 创建 Task 工作区，写 `workspace` 表。
5. 读取 `policy` 表，做一次集成裁决。
6. 对每条 `--effect`：检查幂等键、逐条执行策略裁决并铸造能力。任意一条失败则拒绝整条命令，命令不会执行。
7. 对每条 `--effect` 写 `effect` 行，状态依次为 `PLANNED`、`AUTHORIZED`、`EXECUTING`。三条状态写入和审计行在同一个事务内提交。
8. 在沙箱内启动 `--exec`，工作目录为 Task 工作区根。
9. 命令退出码为 `0` 时，把所有效应写为 `COMMITTED`；非 `0` 时写为 `FAILED`。
10. 若命令成功且给了 `--apply`：按第 5 步的裁决决定是否集成。集成分支不清理工作区。
11. 否则：放弃工作区，成功之后删除 `workspace` 记录。

### 2.5 stdout

`task` 自身在成功路径上不写 stdout。`--exec` 子进程继承驱动的 stdout 和 stderr，因此界面看到的内容就是子进程的输出。

若 `--exec` 子进程没有输出，`task` 成功时 stdout 为空。

### 2.6 stderr 与退出码

| 情况 | stderr | 退出码 | 数据库与文件系统结果 |
|---|---|---|---|
| 参数错误 | `参数错误：...` + USAGE | `1` | 不创建数据库，不创建工作区。 |
| Base 路径无效 | `任务失败: Workspace 操作失败：...` | `1` | 不创建数据库，不创建工作区。 |
| 沙箱不可用 | `任务失败: 沙箱机制选择失败：...` | `1` | 效应记为 `FAILED`；工作区与 `workspace` 记录被清理。 |
| 效应幂等键已存在 | `任务失败: 幂等键 <key> 已有记录：...` | `1` | 命令未执行；工作区与 `workspace` 记录被清理。 |
| 某条效应裁决不通过 | `任务失败: 效应 <type>:<target> 的策略裁决为 <decision>，铸不出能力：...` | `1` | 命令未执行；工作区与 `workspace` 记录被清理。 |
| `--exec` 退出码非 `0` | `任务失败: 命令以退出码 <code> 失败` | `1` | 效应记为 `FAILED`；不集成；工作区与 `workspace` 记录被清理。 |
| 命令成功，未给 `--apply` | 无驱动错误 | `0` | 效应记为 `COMMITTED`；工作区被放弃，`workspace` 记录被删除。 |
| 命令成功，`--apply` 被策略拒绝 | `任务失败: 第 7 步（策略裁决与集成）失败（Intent <id>）：策略裁决为 <decision>，拒绝集成；改动仍在 Task 工作区 <path>，未丢弃` | `1` | 效应记为 `COMMITTED`；工作区与 `workspace` 记录保留；改动未集成。 |
| 命令成功，集成成功 | 无驱动错误 | `0` | 效应记为 `COMMITTED`；改动进入 Base；工作区与 `workspace` 记录保留。 |

表中的 `<decision>` 取值为中文：`允许`、`要求批准`、`禁止`（`crates/continuum-runtime/src/tool_call.rs:202`）。

表中的“被清理”指清理成功。若放弃工作区或删除记录失败，驱动会在错误文本中附加 `Context`，工作区或 `workspace` 记录可能保留。界面应以数据库中的实际记录为准，不要只根据错误类型推断清理结果。

### 2.7 典型调用序列

#### 2.7.1 运行一条无副作用的命令

```bash
continuum-runtime task \
  --base /home/user/project \
  --intent demo-001 \
  --db /home/user/continuum.db \
  --exec /bin/sh -c 'printf "hello\n"'
```

结果：命令在 Task 工作区中运行，stdout 为 `hello`；成功后工作区被放弃，`workspace` 记录被删除，`effect` 表无记录。

#### 2.7.2 运行命令并集成改动

```bash
continuum-runtime task \
  --base /home/user/project \
  --intent demo-002 \
  --db /home/user/continuum.db \
  --apply --approve \
  --exec /bin/sh -c 'printf "hello\n" > result.txt'
```

结果：命令成功后，`--approve` 使第 2 级规则成立，集成被执行，`result.txt` 进入 Base；Task 工作区与 `workspace` 记录保留。

#### 2.7.3 声明效应并执行

```bash
continuum-runtime task \
  --base /home/user/project \
  --intent demo-003 \
  --db /home/user/continuum.db \
  --effect publish:/releases/demo-003 \
  --approve \
  --exec /bin/sh -c 'printf "published\n"'
```

结果：`publish:/releases/demo-003` 先被裁决；`--approve` 使裁决通过；命令执行；效应最终状态为 `COMMITTED` 或 `FAILED`，取决于命令退出码。

#### 2.7.4 首次使用前创建 Base

```bash
mkdir -p /home/user/project
continuum-runtime task \
  --base /home/user/project \
  --intent demo-000 \
  --db /home/user/continuum.db \
  --exec /bin/true
```

Base 不是 Git 仓库时使用 overlay 后端；在容器或受限环境中，驱动可能需要 `unshare` 权限。

## 3. `tool`

### 3.1 签名

```text
continuum-runtime tool
  --db <路径>
  --tool <工具 id>
  [--input <JSON>]
  [--effect <类型>:<目标>]...
  [--intent <id>]
  [--approve]
```

### 3.2 参数

| 参数 | 必填 | 默认值 | 说明 |
|---|---|---|---|
| `--db <路径>` | 是 | 无 | SQLite 数据库路径。 |
| `--tool <工具 id>` | 是 | 无 | 工具标识。字形不校验；未登记时由授权检查拒绝。 |
| `--input <JSON>` | 否 | `{}` | 工具输入，必须是合法 JSON。解析期校验；不落库。 |
| `--effect <类型>:<目标>` | 否 | 空 | 可重复。格式与 `task` 相同。 |
| `--intent <id>` | 条件必填 | 无 | 有任意 `--effect` 时必填；零效应时必须不给。 |
| `--approve` | 条件允许 | `false` | 有任意 `--effect` 时才能给；零效应时必须不给。 |

`--intent` 与 `--approve` 的规则是双向的（`crates/continuum-runtime/src/cli.rs:503`）：

- 有 `--effect` 而没有 `--intent`：解析失败。
- 有 `--effect` 但 `--approve` 未给：合法，由策略裁决决定是否执行。
- 零 `--effect` 而给了 `--intent`：解析失败。
- 零 `--effect` 而给了 `--approve`：解析失败。
- 零 `--effect` 且两个都给了：先报 `--intent` 的错误。

`task` 专属的 `--base`、`--exec`、`--apply`、`--sandbox` 在 `tool` 上是未知选项，解析失败。

### 3.3 stdout

工具被实际调用并返回 `ToolResult` 时，stdout 输出一行紧凑 JSON：

```text
<ToolResult.output 的 JSON 表示>
```

规则：

- 输出使用 `serde_json::Value` 的 `Display` 形式，即紧凑 JSON，无缩进。
- 若 `output` 是字符串，输出包含双引号。
- `ToolResult.is_error` 为 `false` 或 `true` 时都会先输出这一行。
- 工具没有被调用（未授权、未登记、适配器失败）时，stdout 为空。
- 零效应调用如果成功，也会输出这一行。

来源：`crates/continuum-runtime/src/tool_call.rs:467`。

### 3.4 执行流程

1. 打开数据库并应用迁移。
2. 对每条 `--effect` 检查幂等键；任意一条已存在即拒绝整条调用。
3. 读取 `policy` 表一次。
4. 对每条 `--effect` 逐条裁决并铸造能力；任意一条失败即拒绝整条调用。
5. 读 `tool` 表，执行强制点 (1)：工具必须已登记，出示的能力集合必须与工具声明一致。
6. 写 `effect` 行（`PLANNED -> AUTHORIZED -> EXECUTING`）与审计行。
7. 调用 `ProviderRegistry::invoke_tool`。
8. 输出 `ToolResult.output`。
9. 按 `is_error` 写终态：`false` 为 `COMMITTED`，`true` 为 `FAILED`。
10. 若 `is_error` 为 `true`，返回 `TaskError::ToolReportedError`，进程退出码 `1`。

来源：`crates/continuum-runtime/src/tool_call.rs:387`。

### 3.5 当前装配状态

生产装配使用空注册表：

```rust
let registry = ProviderRegistry::new();
```

来源：`crates/continuum-runtime/src/tool_cmd.rs:44`。

因此，即使 `tool` 表中有登记项，授权检查通过后也会在步骤 7 以 `ToolCallError::Unregistered` 失败。若工具声明了必需能力而本次调用没有出示对应能力，则会更早以 `CapabilityError` 失败，不写 `effect` 行。走到步骤 7 时，已声明的效应已经处于 `EXECUTING`；失败后会被写为 `FAILED`。

CLI 能出示的能力只来自 `--effect` 的六种类型：`send_email`、`push_branch`、`publish`、`delete_remote`、`charge`、`deploy`。工具登记项中的 `filesystem_read`、`filesystem_write`、`git_read`、`git_worktree_write`、`git_commit_local`、`github_create_pr` 等能力没有对应的 `--effect` 类型，当前无法通过 CLI 出示。

### 3.6 stderr 与退出码

| 情况 | stderr | 退出码 | 数据库结果 |
|---|---|---|---|
| 参数错误 | `参数错误：...` + USAGE | `1` | 不打开数据库。 |
| 工具未登记 | `工具调用失败: 工具授权失败：工具 <id> 不在登记表中` | `1` | 不写 `effect` 行。 |
| 出示能力缺失或多余 | `工具调用失败: 工具授权失败：...` | `1` | 不写 `effect` 行。 |
| 没有适配器 | `工具调用失败: 工具调用失败：工具 <id> 没有登记的适配器` | `1` | 已声明的效应先写为 `EXECUTING`，再写为 `FAILED`。 |
| 适配器返回错误 | `工具调用失败: 工具调用失败：适配器调用失败: ...` | `1` | 同上。 |
| 工具自报失败 | `工具调用失败: 工具 <id> 自报失败` | `1` | 输出先打印；效应写为 `FAILED`。 |
| 成功 | 无驱动错误 | `0` | 效应写为 `COMMITTED`；无效应时不写 `effect` 行。 |

### 3.7 `--input` 不落库

`--input` 解析后只传给工具适配器；`effect.parameters` 固定写 `{}`（`crates/continuum-runtime/src/tool_call.rs:426`）。界面不能从数据库读回工具输入。

### 3.8 典型调用序列

#### 3.8.1 零效应调用

```bash
continuum-runtime tool \
  --db /home/user/continuum.db \
  --tool demo-tool \
  --input '{"query":"hello"}'
```

结果：解析通过；若 `demo-tool` 未登记，退出码 `1`，stderr 指向未登记工具。

#### 3.8.2 带效应调用

```bash
continuum-runtime tool \
  --db /home/user/continuum.db \
  --tool demo-tool \
  --intent demo-004 \
  --effect publish:/releases/demo-004 \
  --approve \
  --input '{"artifact":"demo"}'
```

结果：效应先裁决并写入；工具调用成功后效应为 `COMMITTED`，失败或自报失败为 `FAILED`。

## 4. `recover`

### 4.1 签名

```text
continuum-runtime recover --db <路径>
```

### 4.2 参数

| 参数 | 必填 | 默认值 | 说明 |
|---|---|---|---|
| `--db <路径>` | 是 | 无 | SQLite 数据库路径。文件不存在时创建；打开后应用迁移。 |

`recover` 不接受其它选项。重复给 `--db` 报错。

### 4.3 执行流程

1. 打开数据库，应用全部迁移。
2. 打印 `迁移应用 <N> 项`。
3. 建事件解码链并校验版本连续性。
4. 扫描事件日志，统计解码数与跳过数。
5. 校验审计哈希链。
6. 按固定顺序执行五个恢复阶段。每个阶段在独立事务内运行。
7. 打印恢复报告。

来源：`crates/continuum-runtime/src/recover_cmd.rs:38`、`crates/continuum-persist/src/recovery.rs:86`。

五个阶段的名称与顺序固定（`crates/continuum-persist/src/recovery.rs:9`）：

| 顺序 | 阶段字符串 | 当前注册的钩子 |
|---|---|---|
| 1 | `load durable state` | 无；该阶段的内建工作由 `run_recovery` 执行。 |
| 2 | `reconcile incomplete effects` | `MarkExecutingAsUnknown`：把 `EXECUTING` 的效应改为 `UNKNOWN`。 |
| 3 | `reconcile running nodes` | `MarkRunningNodesLost`：把 `RUNNING` / `VERIFYING` 的节点改为 `LOST`。 |
| 4 | `mark lost executions` | `BudgetReservationRecovery`：结算悬空的预算预留。 |
| 5 | `resume eligible tasks` | 无。 |

### 4.4 stdout 格式

成功时 stdout 固定为以下行序：

```text
迁移应用 <N> 项
跳过记录 <S> 条
标记 LOST <L> 个节点
load durable state: <h1> 钩子
reconcile incomplete effects: <h2> 钩子
reconcile running nodes: <h3> 钩子
mark lost executions: <h4> 钩子
resume eligible tasks: <h5> 钩子
```

一次已迁移数据库上的典型输出：

```text
迁移应用 0 项
跳过记录 0 条
标记 LOST 0 个节点
load durable state: 0 钩子
reconcile incomplete effects: 1 钩子
reconcile running nodes: 1 钩子
mark lost executions: 1 钩子
resume eligible tasks: 0 钩子
```

字段说明：

| 字段 | 含义 |
|---|---|
| `<N>` | 本次 `db.migrate()` 应用的迁移数量。 |
| `<S>` | 事件日志扫描中判定为可跳过的记录数。 |
| `<L>` | 本次被标记为 `LOST` 的节点数。 |
| `<h1>`–`<h5>` | 对应阶段实际运行的钩子数；没有钩子的阶段为 `0`。 |

注意：`run_recovery` 内部会再次调用 `migrate()`，此时返回 `0`；不会再次打印。`<N>` 是第一次调用的结果。

### 4.5 stderr 与退出码

| 情况 | stderr | 退出码 | stdout |
|---|---|---|---|
| 成功 | 无 | `0` | 完整八行。 |
| 数据库打不开 | `启动失败: 数据库错误: ...` | `1` | 可能为空。 |
| 迁移失败 | `启动失败: 迁移 <version> 失败: ...` | `1` | 可能没有第一行。 |
| 事件日志或审计链校验失败 | `启动失败: 数据库错误: ...` | `1` | 可能只有第一行。 |
| 某个恢复阶段失败 | `启动失败: ...` | `1` | 已打印的行保留；后续阶段不执行。 |

界面应把 stdout 当作“已完成的步骤”，而不是“全部成功”。退出码非 `0` 时，已输出的行仍可能有效。

### 4.6 恢复写入的内容

`recover` 可能修改以下数据：

| 数据 | 变化 | 来源 |
|---|---|---|
| `effect.state` | `EXECUTING` → `UNKNOWN`；同时追加审计行。 | `crates/continuum-effect/src/recovery.rs:51` |
| `adfir_node.state` | `RUNNING` / `VERIFYING` → `LOST`。 | `crates/continuum-graph/src/persist.rs:188` |
| `node_attempt` | 为每个被标记的节点追加一行，`failure_class` 为 `unknown`。 | `crates/continuum-graph/src/persist.rs:156` |
| `budget_ledger` | 为悬空预扣追加 `settle` 行。 | `crates/continuum-budget/src/recovery.rs` |

`recover` 不创建新的 `workspace` 记录，也不删除 `effect` 行。

## 5. 调用序列

### 5.1 首次打开数据库

```bash
continuum-runtime recover --db /home/user/continuum.db
```

`recover` 会创建数据库并应用 13 个迁移项。之后 `task` 和 `tool` 可以直接使用同一个 `--db`。

### 5.2 任务加恢复

```bash
# 运行任务
continuum-runtime task \
  --base /home/user/project \
  --intent demo-005 \
  --db /home/user/continuum.db \
  --exec /bin/sh -c 'sleep 1; printf "done\n"'

# 若进程在任务执行期间被杀，之后运行恢复
continuum-runtime recover --db /home/user/continuum.db
```

恢复后，停在 `EXECUTING` 的效应变为 `UNKNOWN`；界面应把 `UNKNOWN` 显示为“结果未知，需要人工核对”，不要映射为成功或失败。

### 5.3 任务加集成

```bash
continuum-runtime task \
  --base /home/user/project \
  --intent demo-006 \
  --db /home/user/continuum.db \
  --apply --approve \
  --exec /bin/sh -c 'printf "change\n" >> README.md'
```

集成成功后，`workspace` 记录保留。界面可以用该记录的 `path` 显示工作区位置。

## 6. 不是接口的内容

以下内容不属于 CLI 契约，界面不得依赖：

- `USAGE` 文本中的程序名 `continuum`。
- 错误文本的完整措辞、标点、全角/半角形式。
- 子进程的退出码作为驱动退出码。
- stdout 中除 `tool` 输出行之外的任何结构。
- 未列出的环境变量。
- 数据库表的物理行顺序，唯一例外是 `events` 的 `rowid` 顺序和显式 `ORDER BY` 的查询。
