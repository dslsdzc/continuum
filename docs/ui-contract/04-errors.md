# 错误与失败分类

本文档描述错误的来源、错误文本、退出码，以及界面把错误映射为用户可读状态时应遵循的规则。所有内容对应提交 `03e79c7`。

## 术语

- **Intent**：一次工作的目标、状态与关联标识。
- **Effect**：一次对外部世界的副作用记录。
- **Workspace**：`task` 为某个 Intent 创建的隔离工作目录。
- **Tool**：登记在 `tool` 表中的工具登记项及其适配器。
- **Evidence**：验证过程产生的证据记录。

## 1. 总览

### 1.1 没有机器可读错误码

CLI 没有错误码字段，也没有 JSON 错误输出。退出码只有两个：

| 退出码 | 含义 |
|---|---|
| `0` | 成功。 |
| `1` | 任何失败。 |

一次失败只有：

- 退出码 `1`；
- stderr 上的一行或多行文本；
- 数据库中可能已经发生的状态变化。

因此，界面不能依赖稳定的错误码。可以依赖的是：

- 退出码 `0` 或 `1`；
- stderr 首行的固定前缀；
- 数据库中的实体状态；
- 错误文本中的标识（Intent、效应类型、目标、路径、退出码）。

### 1.2 错误分层

| 层 | 典型来源 | 是否在命令执行前 | 界面应显示为 |
|---|---|---|---|
| 参数解析 | `CliError` | 是 | 输入错误，修正参数后重试。 |
| 启动装配 | `SecretsError` | 是 | 环境配置错误。 |
| 工作区与沙箱 | `WorkspaceError`、`SandboxSelectError`、`SandboxError` | 通常在命令执行前 | 环境或路径错误。 |
| 策略与授权 | `TaskError::EffectNotAuthorized`、`CapabilityError` | 是 | 策略拒绝或需要批准。 |
| 命令执行 | `TaskError::CommandFailed`、`WaitFailed`、`ReExecFailed` | 执行中 | 执行失败。 |
| 集成 | `TaskError::IntegrationRefused`、`GateError` | 命令成功之后 | 集成失败，改动可能仍在。 |
| 存储 | `PersistError` | 任意阶段 | 数据库错误。 |
| 恢复 | `PersistError`、事件日志错误、审计链错误 | 恢复阶段 | 恢复失败。 |

### 1.3 错误文本前缀

| 前缀 | 来源 | 代码位置 |
|---|---|---|
| `参数错误：` | `cli::parse` 失败 | `crates/continuum-runtime/src/main.rs:26` |
| `启动失败: ` | 密钥运行时装配失败，或 `recover` 失败 | `crates/continuum-runtime/src/main.rs:41`、`:50` |
| `任务失败: ` | `task` 失败 | `crates/continuum-runtime/src/main.rs:57` |
| `工具调用失败: ` | `tool` 失败 | `crates/continuum-runtime/src/main.rs:72` |

前缀之后的内容来自各错误类型的 `Display` 实现。内层错误可能继续嵌套，形成多层前缀。例如：

```text
工具调用失败: 工具调用失败：工具 demo-tool 没有登记的适配器
```

界面应保留完整文本用于详情展示，但分类只使用首层前缀和已知字段。

## 2. 参数解析错误

### 2.1 `CliError`

`CliError` 共 12 个变体（`crates/continuum-runtime/src/cli.rs:255`）。这些错误都发生在打开数据库之前。

| 变体 | 触发条件 | `Display` 文本 |
|---|---|---|
| `MissingSubcommand` | 没有任何参数。 | `缺少子命令，可用：task、tool、recover` |
| `UnknownSubcommand { name }` | 第一个参数不是三个子命令之一。 | `未知子命令：<name>，可用：task、tool、recover` |
| `MissingValue { option }` | 带值选项后没有 token。 | `选项 <option> 缺少取值` |
| `MissingOption { option }` | 必填选项未给。 | `缺少必填选项：<option>` |
| `UnknownOption { name }` | 选项不属于当前子命令。 | `未知选项：<name>` |
| `DuplicateOption { option }` | 带值选项出现多次。 | `选项 <option> 出现了多次；本选项只接受一次` |
| `UnknownSandboxMechanism { name }` | `--sandbox` 取值不是 `landlock` 或 `bubblewrap`。 | `未知沙箱机制：<name>，可用：landlock、bubblewrap` |
| `EffectWithoutColon { value }` | `--effect` 值中没有冒号。 | `效应缺少冒号：<value>，应为 <类型>:<目标>` |
| `EffectWithEmptyTarget { value }` | `--effect` 的目标部分为空。 | `效应目标为空：<value>，应为 <类型>:<目标>` |
| `UnknownEffectType { name }` | `--effect` 类型不在六个取值内。 | `未知效应类型：<name>` |
| `InvalidToolInput { value, reason }` | `--input` 不是合法 JSON。 | `--input 不是合法 JSON（<value>）：<reason>` |
| `OptionRequiresEffect { option }` | `--effect` 与 `--intent` / `--approve` 不满足同进同出规则。 | `选项 <option> 与 --effect 必须同进同出：给出 --effect 时必须给 --intent；未给任何 --effect 时不得给 <option>` |

解析失败时 stderr 还会输出完整 `USAGE`。`USAGE` 中包含所有选项名，界面不要用整段 stderr 做关键词判断；应取第一行。

### 2.2 界面映射

| 条件 | 显示类别 | 是否可重试 |
|---|---|---|
| 缺少子命令、未知子命令、未知选项 | 调用方式错误 | 修正后重试。 |
| 缺少必填选项、选项缺少取值 | 参数不完整 | 补齐后重试。 |
| 重复选项 | 参数冲突 | 去掉重复项后重试。 |
| `--sandbox` 取值错误 | 参数取值错误 | 改为 `landlock` 或 `bubblewrap`。 |
| `--effect` 格式或类型错误 | 效应声明错误 | 修正声明后重试。 |
| `--input` 不是合法 JSON | 工具输入错误 | 修正 JSON 后重试。 |
| `OptionRequiresEffect` | 选项组合错误 | 按规则补齐或删除选项。 |

这些错误不会创建数据库，也不会创建工作区。界面不应在收到解析错误后去查询数据库。

## 3. `task` 错误

### 3.1 `TaskError`

`TaskError` 共 16 个变体（`crates/continuum-runtime/src/error.rs:49`）。

| 变体 | 触发条件 | `Display` 文本 | 数据库/文件系统结果 |
|---|---|---|---|
| `EffectAlreadyRecorded { key }` | 任意 `--effect` 的幂等键已存在。 | `幂等键 <key> 已有记录：拒绝运行整条命令（已存在的记录不静默跳过——跳过会让调用方以为命令执行了）` | 命令未执行；工作区与 `workspace` 记录被清理。 |
| `EffectNotAuthorized { effect, target, decision }` | 某条效应裁决不通过。 | `效应 <effect>:<target> 的策略裁决为 <decision>，铸不出能力：拒绝运行整条命令（命令一步都没跑）` | 命令未执行；工作区与记录被清理。 |
| `IntegrationRefused { decision, path }` | 命令成功，但集成裁决不铸造批准值。 | `策略裁决为 <decision>，拒绝集成；改动仍在 Task 工作区 <path>，未丢弃` | 效应为 `COMMITTED`；工作区与记录保留；未集成。 |
| `Gate(GateError)` | Integration Gate 失败。 | `Integration Gate 失败：<内层>` | 视失败点而定；工作区通常保留。 |
| `SandboxSelect(SandboxSelectError)` | 两种沙箱都不可用，或显式指定的不可用。 | `沙箱机制选择失败：<内层>` | 命令未执行；已声明效应记为 `FAILED`；工作区与记录被清理。 |
| `Workspace(WorkspaceError)` | Base、Intent 或工作区操作失败。 | `Workspace 操作失败：<内层>` | 视失败点而定；创建失败时无记录。 |
| `Sandbox(SandboxError)` | 施加隔离或启动子进程失败。 | `沙箱操作失败：<内层>` | 命令可能未启动；效应终态按失败处理。 |
| `Persist(PersistError)` | 数据库操作失败。 | `数据库操作失败：<内层>` | 视失败点而定；可能留下记录或工作区。 |
| `RecordMissing { intent }` | 刚写入的 `workspace` 记录读不回。 | `工作区记录读不回（Intent <intent>）：本次运行刚写过它，缺失说明记录被别处删掉了；工作区原样留着，不按猜出来的后端回收` | 工作区保留；记录缺失。 |
| `CommandFailed { code }` | `--exec` 子进程退出码非 `0`。 | `命令以退出码 <code> 失败` | 效应记为 `FAILED`；不集成；工作区与记录被清理。 |
| `WaitFailed { reason }` | 等待子进程结束失败。 | `等待子进程结束失败：<reason>` | 效应按失败处理；工作区与记录被清理。 |
| `ReExecFailed { reason }` | 通过 `unshare -Urm` 重新执行自身失败。 | `重新执行自身（unshare -Urm）失败：<reason>` | 工作区尚未创建。 |
| `Context { context, source }` | 在既有错误上附加“哪一步、还剩什么”。 | `<context>：<source>` | 取决于内层错误。 |
| `Capability(CapabilityError)` | 工具授权检查失败。 | `工具授权失败：<内层>` | 仅 `tool` 路径可能产生；不写 `effect` 行。 |
| `ToolCall(ToolCallError)` | 工具路由或适配器失败。 | `工具调用失败：<内层>` | 已声明的效应记为 `FAILED`。 |
| `ToolReportedError { tool }` | 工具返回 `is_error = true`。 | `工具 <tool> 自报失败` | 输出已打印；效应记为 `FAILED`。 |

表中的“被清理”指清理成功。清理失败时，错误会以 `Context` 形式带出，工作区或 `workspace` 记录可能保留；界面应以数据库实际状态为准。

### 3.2 `Context` 的嵌套

`TaskError::Context` 可能把两个失败叠在一起：

```text
任务失败: <外层上下文>：<内层错误>
```

常见形式：

| 外层上下文 | 内层 | 含义 |
|---|---|---|
| `落库失败（...）之后，放弃本次建出的工作区也失败` | 清理错误 | 工作区记录未写入，工作区也未清理。 |
| `第 4 步（效应登记）失败（...）之后，清理工作区也失败` | 清理错误 | 效应登记失败，清理也失败。 |
| `效应记录的终态写入失败（...）之后，清理工作区也失败` | 清理错误 | 终端状态未写入，清理也失败。 |
| `第 7 步（策略裁决与集成）失败（Intent <id>）` | 集成错误 | 命令成功，集成失败。 |
| `命令已执行，但随后的清理失败` | 清理错误 | 命令成功，工作区清理失败。 |
| `执行失败（...）之后，清理也失败` | 清理错误 | 命令失败，清理也失败。 |

界面应同时显示外层和内层信息。只显示最外层会丢掉具体原因；只显示最内层会丢掉“哪一步失败”。

## 4. `tool` 错误

`tool` 复用 `TaskError`，但只会产生其中一部分变体。可达变体：

| 变体 | 触发条件 | 结果 |
|---|---|---|
| `EffectAlreadyRecorded` | 幂等键已存在。 | 不写 `effect` 行。 |
| `EffectNotAuthorized` | 逐条效应裁决不通过。 | 不写 `effect` 行。 |
| `Capability` | 工具未登记、缺少能力、出示未声明能力、能力过期。 | 不写 `effect` 行。 |
| `ToolCall` | 没有适配器，或适配器返回错误。 | 已声明效应记为 `FAILED`。 |
| `ToolReportedError` | 工具返回 `is_error = true`。 | 输出已打印；效应记为 `FAILED`。 |
| `Persist` | 数据库读写失败。 | 视失败点而定。 |

`tool` 不会产生 `IntegrationRefused`、`Gate`、`CommandFailed`、`ReExecFailed` 等命令路径变体。

### 4.1 `CapabilityError`

`CapabilityError` 共 7 个变体（`crates/continuum-capability/src/error.rs:26`）：

| 变体 | 触发条件 | `Display` 文本 |
|---|---|---|
| `EmptyScope` | 能力作用域为空。 | `能力的作用域不能为空` |
| `Expired { expiry, now }` | 能力已过期。 | `能力已于 <expiry> 失效（Unix 毫秒），当前 <now>` |
| `UnknownTool { id }` | `tool` 表没有该工具。 | `工具 <id> 不在登记表中` |
| `Persist(PersistError)` | 读登记项或写审计失败。 | `持久化失败: <内层>` |
| `MissingCapability { kind }` | 工具声明了某能力，调用方未出示。 | `工具声明了能力 <kind>，调用方未出示` |
| `UndeclaredCapability { kind }` | 调用方出示了工具未声明的能力。 | `调用方出示了工具未声明的能力 <kind>` |
| `EffectCapabilityMismatch { effect, expected, actual }` | 效应与能力不匹配。 | `效应 <effect> 需要能力 <expected>，而给出的能力是 <actual>` |

### 4.2 `ToolCallError`

`ToolCallError` 共 2 个变体（`crates/continuum-provider/src/registry.rs:53`）：

| 变体 | 触发条件 | `Display` 文本 |
|---|---|---|
| `Unregistered { id }` | 注册表没有该工具 id 的适配器。 | `工具 <id> 没有登记的适配器` |
| `Provider(ProviderError)` | 适配器返回错误。 | `适配器调用失败: <内层>` |

`ProviderError` 共 5 个变体（`crates/continuum-core/src/error.rs:4`）：

| 变体 | `Display` 文本 |
|---|---|
| `Unavailable` | `provider 不可用: <text>` |
| `UnknownModel` | `模型不存在: <text>` |
| `Cancelled` | `调用已取消: <text>` |
| `Transport` | `传输失败: <text>` |
| `Protocol` | `协议错误: <text>` |

## 5. 工作区与沙箱错误

### 5.1 `WorkspaceError`

`WorkspaceError` 是 `#[non_exhaustive]` 枚举（`crates/continuum-workspace/src/error.rs:25`）。主要变体：

| 变体 | 触发条件 | `Display` 文本 |
|---|---|---|
| `MissingBase { path }` | Base 路径不存在。 | `Base Workspace 路径不存在：<path>` |
| `NotAbsolute { path }` | Base 路径不是绝对路径。 | `Base Workspace 路径必须是绝对路径：<path>` |
| `NotADirectory { path }` | Base 路径不是目录。 | `Base Workspace 路径不是目录：<path>` |
| `EscapesRoot { path }` | 目标路径越出 Task 根。 | `路径越出 Task Workspace 根：<path>` |
| `Overlaps { root, base }` | Task 根与 Base 重叠。 | `Task Workspace 的根 <root> 与 Base <base> 重叠：...` |
| `BackendUnavailable { reason }` | 后端无法建立或挂载。 | `Workspace 后端不可用：<reason>` |
| `IoFailed { path, reason }` | 文件读写失败。 | `路径 <path> 的读写失败：<reason>` |
| `NotInNamespace` | 当前进程或命令不在所需命名空间内。 | `不在本 Workspace 所需的命名空间内` |
| `GitFailed { code, stderr }` | git 命令失败。 | `git 命令失败（退出码 <code>）：<stderr>` |
| `CommandFailed { command, code, stderr }` | 非 git 外部命令失败。 | `命令 <command> 失败（退出码 <code>）：<stderr>` |
| `InvalidIntent { intent }` | Intent 不能作为路径分量或分支名。 | `Intent 标识不能用作 Task Workspace 路径分量：<intent>` |
| `Context { context, source }` | 附加清理上下文。 | `<context>：<source>` |
| `GateRefused { reason }` | Integration Gate 拒绝。 | `Integration Gate 拒绝：<reason>` |

### 5.2 `GateError`

`GateError` 共 4 个变体（`crates/continuum-workspace/src/gate.rs:606`）：

| 变体 | 触发条件 | `Display` 文本 |
|---|---|---|
| `Workspace(WorkspaceError)` | 底层工作区操作失败。 | 透传内层文本。 |
| `Persist(PersistError)` | 写审计失败。 | 透传内层文本；Base 改动可能已经生效。 |
| `ApprovalMismatch { reason }` | 批准值与本次集成不符。 | `批准值与本次集成不符：<reason>` |
| `Malformed { reason }` | 后端结果无法解释。 | `Integration Gate 无法解释后端给出的结果：<reason>` |

`GateError::Persist` 表示 Base 改动已经生效但审计行写入失败。界面不能因为看到这个错误就认为 Base 没有变化。

### 5.3 `SandboxSelectError`

两个变体（`crates/continuum-runtime/src/sandbox_select.rs:41`）：

| 变体 | 触发条件 | `Display` 文本 |
|---|---|---|
| `ExplicitUnavailable { mechanism, reason }` | 显式指定的机制不可用。 | `显式指定的沙箱机制 <mechanism> 在本机不可用（<reason>）；不自动改选别的机制` |
| `NoneAvailable { landlock, bubblewrap }` | 两种机制都不可用。 | `两种沙箱机制在本机都不可用：landlock（<landlock>）、bubblewrap（<bubblewrap>）；拒绝在无文件系统隔离下运行` |

### 5.4 `SandboxError`

三个变体（`crates/continuum-sandbox/src/error.rs`）：

| 变体 | 触发条件 | `Display` 文本 |
|---|---|---|
| `MechanismUnavailable { mechanism, reason }` | 机制在本机不存在。 | `沙箱机制 <mechanism> 不可用：<reason>` |
| `IsolationFailed { mechanism, reason }` | 施加隔离失败。 | `沙箱机制 <mechanism> 施加隔离失败：<reason>` |
| `SpawnFailed { reason }` | 子进程启动失败。 | `子进程启动失败：<reason>` |

`IsolationFailed` 与 `SpawnFailed` 在调用方一侧不总是可分。界面不应据此断言子进程是否已经存在。

## 6. 存储错误

`PersistError` 共 5 个变体（`crates/continuum-persist/src/error.rs:5`）：

| 变体 | 触发条件 | `Display` 文本 |
|---|---|---|
| `Database(String)` | SQLite 或 SQL 错误。 | `数据库错误: <text>` |
| `Migration { version, message }` | 迁移失败。 | `迁移 <version> 失败: <message>` |
| `ParamType { index, actual }` | 参数类型不匹配。 | `参数类型不匹配: 第 <index> 个参数为 <actual>` |
| `ColumnType { index, actual }` | 列类型不匹配。 | `结果类型不匹配: 第 <index> 列为 <actual>` |
| `TransactionDropped` | 事务未提交即被丢弃。 | `事务未提交即被丢弃` |

`PersistError::Database` 也承载唯一索引冲突、主键冲突、表不存在等 SQLite 错误。它没有独立的“冲突”变体，因此界面不能从变体名区分“重复写入”和“数据库损坏”；需要结合操作上下文判断。

## 7. 状态机错误

图状态与效应状态各有一个 `StateError`，文本形式相同，类型不同：

| 来源 | `Display` 文本 |
|---|---|
| `continuum-graph` 的 `StateError::Illegal { from, to }` | `迁移 <from> → <to> 不在允许的迁移表内` |
| `continuum-effect` 的 `StateError::Illegal { from, to }` | `迁移 <from> → <to> 不在允许的迁移表内` |

来源：`crates/continuum-graph/src/state.rs:7`、`crates/continuum-effect/src/effect.rs:147`。

这些错误当前只在库层产生，CLI 不直接暴露。界面读数据库时通常看不到它们，除非数据被手工改写。

## 8. 失败分类

### 8.1 `FailureClass`

`FailureClass` 共 7 个取值（`crates/continuum-graph/src/failure.rs:8`）。它用于节点失败分类，当前存储在 `node_attempt.failure_class`，数据库编码为小写。

| 枚举 | 数据库字符串 | 含义 | 重试判定 |
|---|---|---|---|
| `Transient` | `transient` | 暂时性失败。 | 在白名单内且有剩余尝试时重试。 |
| `Resource` | `resource` | 资源不足。 | 在白名单内且有剩余尝试时重试。 |
| `Constraint` | `constraint` | 约束失败。 | 升级，不自动重试。 |
| `Authorization` | `authorization` | 授权失败。 | 升级，不自动重试。 |
| `Permanent` | `permanent` | 永久性失败。 | 直接失败。 |
| `Verification` | `verification` | 验证失败。 | 直接失败。 |
| `Unknown` | `unknown` | 未知失败。 | 直接失败。 |

### 8.2 重试判定

`decide_retry` 的规则（`crates/continuum-graph/src/failure.rs:106`）：

1. 非幂等副作用直接升级，白名单不改变这一条。
2. 按类别取固有归属：
   - `constraint`、`authorization` 升级；
   - `permanent`、`verification`、`unknown` 失败；
   - `transient`、`resource` 继续判断。
3. 类别不在 `retryable_errors` 白名单内时失败。
4. 仍有尝试余量时重试。
5. 否则升级。

`RetryDecision` 三个取值（`crates/continuum-graph/src/failure.rs:86`）：

| 枚举 | 含义 |
|---|---|
| `Retry { next_attempt }` | 重试，下一次尝试编号为 `next_attempt`。 |
| `Escalate { to }` | 升级到 `EscalationPolicy`。 |
| `Fail` | 终止。 |

`EscalationPolicy` 三个取值：`none`、`decision`、`manual`。`none` 时“升级”退化为 `Fail`。

这套判定是库 API，当前没有 CLI 入口。界面若显示 `node_attempt.failure_class`，可以按上表显示“可重试”“需升级”“终止”，但不要声称系统会自动重试；是否自动重试取决于调用方是否存在。

## 9. 库层错误（CLI 不可达）

以下错误类型存在，但当前没有 CLI 子命令能把它们暴露给界面。列出它们是为了避免界面把库层错误误认为 CLI 错误。

### 9.1 `ModelCallError`

5 个变体（`crates/continuum-runtime/src/error.rs:184`）：

| 变体 | `Display` 文本 |
|---|---|
| `Routing(RoutingError)` | `模型排序失败：<内层>` |
| `Storage(PersistError)` | `模型登记表读取失败：<内层>` |
| `Deadline { elapsed_ms }` | `模型调用在 <elapsed_ms> ms 内没有完成` |
| `Provider { class, source }` | `模型适配器调用失败（<class>）：<source>` |
| `Cancelled { source }` | `模型调用被调用方取消：<source>` |

`ModelCallError` 没有 CLI 入口。界面不能通过 `task` 或 `tool` 触发模型调用。

### 9.2 `VerifyError`

11 个变体（`crates/continuum-verify/src/error.rs:21`）：

`EvidenceWithoutArtifact`、`HumanConfirmationFromNonHuman`、`IncompleteTestValidityChecks`、`UnknownVerificationPolicy`、`ProfileMissing`、`NoAvailableVerifier`、`NoIndependentVerifier`、`EvidenceFromVerifiedNode`、`MissingEvidenceEdge`、`CounterexampleWithPassingAggregate`、`Persist`。

这些错误只从 `continuum-verify` 的库函数返回。当前没有验证 CLI。

### 9.3 语义层错误

`continuum-semantics` 有 `IntentError`、`SpecError`、`ContractError`、`PlanError`、`DecisionError`。它们只从库函数返回，当前没有语义层 CLI。

## 10. 界面展示建议

### 10.1 错误对象

界面可以按以下字段组织一条错误：

| 字段 | 来源 | 说明 |
|---|---|---|
| `stage` | stderr 前缀 + 调用上下文 | 参数、启动、任务、工具、恢复。 |
| `summary` | 按 §10.2 的规则生成 | 一句短说明。 |
| `detail` | 完整 stderr | 原样展示，保留嵌套信息。 |
| `retryable` | 按 §10.3 的规则生成 | 是、否、条件性。 |
| `dataRefs` | 从错误文本或数据库提取 | Intent、效应、路径、退出码。 |

### 10.2 摘要映射

| 条件 | 摘要 |
|---|---|
| 首行以 `参数错误：` 开头 | 调用参数有误。 |
| 首行以 `启动失败: ` 开头且子命令是 `recover` | 恢复启动失败。 |
| 首行以 `启动失败: ` 开头且子命令不是 `recover` | 运行时启动失败。 |
| 首行以 `任务失败: ` 开头 | 任务未完成。 |
| 首行以 `工具调用失败: ` 开头 | 工具调用未完成。 |
| 文本包含 `幂等键` 与 `已有记录` | 该效应已经执行过，拒绝重复执行。 |
| 文本包含 `策略裁决为` 与 `拒绝集成` | 命令已成功，但改动未集成。 |
| 文本包含 `策略裁决为` 与 `铸不出能力` | 命令未执行，策略拒绝。 |
| 文本包含 `命令以退出码` | 子进程失败。 |
| 文本包含 `没有登记的适配器` | 没有可用的工具适配器。 |
| 文本包含 `不在登记表中` | 工具未登记。 |
| 文本包含 `工作区记录读不回` | 工作区记录缺失，工作区保留。 |

这些匹配只用于生成摘要，不用于决定业务状态。业务状态以数据库为准。

### 10.3 可重试性

| 情况 | 可重试性 |
|---|---|
| 参数错误 | 修正参数后可重试。 |
| 启动配置错误 | 修正环境后可重试。 |
| 沙箱不可用 | 修正环境后可重试；同一环境重复调用结果相同。 |
| 策略拒绝，裁决为“要求批准” | 可加 `--approve` 重试。若拒绝发生在命令执行前的效应检查，工作区与记录已被清理，同一 Intent 通常可以重试；若拒绝来自集成阶段，见“集成被拒”。 |
| 策略拒绝，裁决为“禁止” | 不应通过 `--approve` 重试；需要修改策略或操作内容。 |
| 幂等键已存在 | 不应重试同一 Intent + 效应；需要新的 Intent，或人工处理数据库。 |
| 命令退出码非 `0` | 可以用新的 Intent 重试；同一 Intent 的工作区已被清理。 |
| 集成被拒 | 不应直接重试同一 Intent；工作区与记录保留，需要人工处理。 |
| 集成 Gate 失败 | 视具体原因；同一 Intent 通常不能直接重跑。 |
| 工具未登记或没有适配器 | 当前环境不可重试；需要先登记工具并装配适配器。 |
| 工具自报失败 | 可以重试，但应使用新的 Intent 或新的效应声明。 |
| 数据库错误 | 不应自动重试；先确认数据库可用。 |
| 恢复失败 | 不应自动重试；先检查数据库、事件日志和审计链。 |

### 10.4 状态与错误的对应

| 数据状态 | 界面含义 |
|---|---|
| `effect.state = planned` | 已登记，尚未授权。 |
| `effect.state = authorized` | 已授权，尚未开始执行。 |
| `effect.state = executing` | 已开始执行；若进程已退出，说明记录可能停在崩溃前。 |
| `effect.state = committed` | 命令或工具返回成功。这不等于改动已经集成。 |
| `effect.state = failed` | 命令或工具返回失败。这不等于外部副作用没有发生。 |
| `effect.state = rolled_back` | 已回滚。 |
| `effect.state = unknown` | 外部结果未知，需要人工核对。不要显示为成功或失败。 |
| `workspace` 行存在 | 工作区保留；可能是集成成功，也可能是集成被拒。 |
| `workspace` 行不存在 | 工作区已被清理；可能是成功后的清理，也可能是失败后的清理。 |
| `intent.status = completed` | Intent 完成；不等于所有验证通过。 |
| `intent.status = failed` | Intent 失败。 |
| `node.state = lost` | 节点在崩溃恢复中被标记为丢失。 |
| `node.state = invalidated` | 节点因输入变化失效；可能有后继迁移。 |

### 10.5 不要做的事

- 不要把 `effect.state = committed` 显示为“已集成”。
- 不要把 `effect.state = failed` 显示为“没有产生副作用”。
- 不要把 `effect.state = unknown` 显示为“失败”或“成功”。
- 不要从退出码 `1` 推断命令没有运行；`task` 在命令成功但集成失败时也返回 `1`。
- 不要从 `workspace` 行缺失推断任务从未运行。
- 不要解析完整错误文本的标点或本地化文案；只使用固定前缀和标识字段。
- 不要把 `tool` 的 stdout 当作错误信息；错误在 stderr。
