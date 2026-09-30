# P0 基础设施与接口冻结 — 设计

本子项目建立持久化机制、事件通道与外围接口边界，是 P0→P7 建造顺序的第一个子项目。

范围依据 §343 中 Persistence 与 Event Stream 两项，以及 02 §10.3 要求冻结的三个外围接口。本子项目交付机制，不交付业务逻辑。

---

# 1. 范围排除

以下内容不在本子项目内：

```
Effect Journal 的语义与状态机（§268、§269）        属 P2 边界层
ADFIR 图、Node 状态机、调度（§235–§239、§303）      属 P1 执行层
模型与工具 Provider 的实现                          属 P3 资源层，本子项目只冻结 trait
业务表结构                                          §317 的十类状态由对应层建立
前端                                                §346 要求前端无关
```

# 2. 完成判据

```
1. 关键状态更新在单事务内提交，异常退出后不出现 §318 列举的不一致状态
2. 启动执行 §319 的五个阶段，各阶段调用已注册的钩子，且第一阶段完成事件日志解码与审计链校验
3. Event Stream 能构造并输出 §310 的九类事件，未知类型的处理符合第 5.2 节
4. Audit Log 覆盖 §313 的八类事件
5. §315、§316、§124 以 trait 冻结，且 core 与 persist 不引用任何 Provider 实现类型（§346）
```

各判据的验证方式见第 8 节。

# 3. crate 布局与依赖方向

```
crates/
  continuum-core      不变量类型；无 I/O
  continuum-events    事件类型、Event Stream 写入接口
  continuum-persist   迁移框架、事务 API、恢复钩子注册表
  continuum-provider  §315 / §316 / §124 的 trait 定义，无实现
  continuum-runtime   装配与二进制入口
```

依赖方向：

```
continuum-runtime  → continuum-provider → continuum-core
continuum-runtime  → continuum-persist  → continuum-events → continuum-core
continuum-runtime  → continuum-events
```

约束：禁止反向依赖。02 §9.1 的层间依赖方向由 crate 边界在编译期强制。若后续层需要反向边，应调整布局，不得新增反向依赖。

`continuum-persist` 依赖 `continuum-events` 的原因：状态提交与对应事件写入必须处于同一事务（§318）。

与 ENG-001 的关系：crate 边界是 ENG-001「Protected Core 的强制形态」的候选机制之一（02 §10.1 的 ENG-001 可选方向 a 为独立 crate）。本子项目定下的方向在 P6 只加固，不新增反向边。

# 4. 持久化机制

存储为 SQLite。依据 02 §1.4（持久化绑定 SQLite，性质为载荷）与 §318（要求事务语义）。

本子项目提供三项机制：

```
迁移框架           版本化迁移，启动时按序应用；迁移进度持久化
事务 API           单事务内提交状态更新与对应事件
恢复钩子注册表      各层注册恢复步骤，启动时按 §319 的五个阶段顺序调用
```

§319 五阶段与钩子归属：

```
load durable state             由本子项目实现
reconcile incomplete effects   P2 注册（§269）
reconcile running nodes        P1 注册（§237）
mark lost executions           P1 注册
resume eligible tasks          P1 注册
```

未注册钩子的阶段执行空操作，并在启动记录中标注该阶段无钩子。这表示在对应层完成前，§317 的恢复保证不成立。

## 4.1 第一阶段的判定规则

§319 只列出五个阶段的名称，未定义各阶段的判定规则。`load durable state` 由本子项目实现，其内容与判定规则在此定义。规则形态取自 DSH 的可恢复解码器（见 `2026-09-30-DSH-对照分析.md` 第 4.3 节）。

第一阶段执行三项：

```
1  打开数据库并按 version 升序应用未记录的迁移
2  解码事件日志，按第 5.2 节处理未知事件类型
3  重算审计链，按第 6 节校验哈希
```

判定规则：

```
可跳过     单条事件的 payload 无法反序列化
           跳过该事件并计入 RecoveryReport.skipped_records

致命       未知事件类型且 ignorable = false        §5.2
           可跳过事件之后存在 intent.completed     见下
           审计链任一哈希不匹配                     §6
           迁移失败                                第 4 节
```

「可跳过事件之后存在 `intent.completed`」升级为致命的原因：`intent.completed` 表示该 Intent 已按 §266 收口。其前驱事件不可读时，收口依据不成立，继续启动会得到无法解释的已完成状态。

致命判定触发时启动中止，不进入后续四个阶段。

`RecoveryReport` 增加 `skipped_records: usize` 字段，记录第 2 项与第 3 项判定中被跳过的事件数。该字段是「跳过发生了」的唯一可观测证据，不得省略。

约束：

- §318 列出的关键状态更新（node output commit、effect commit、contract update、authority update）必须经事务 API 提交。数据库连接对象在 `continuum-persist` 内私有，外部 crate 只能通过事务 API 访问，因此该约束在编译期成立。
- 迁移失败时启动中止，不进入部分迁移状态。

# 5. Event Stream

§310 规定 Runtime MUST 提供九类结构化事件。§135 规定前端只是事件的一个消费者。A12 / OPEN-006 未定义字段结构与版本兼容规则。

v0.1 取法如下。该取法不是 OPEN-006 的规范答案，OPEN-006 仍开放。

事件信封：

```
Event {
    event_id
    type
    schema_version
    occurred_at
    intent_id?
    node_id?
    ignorable
    payload
}
```

`payload` 为各事件类型各自定义的结构化字段，经 serde 序列化。

`type` 取 §310 的九个值：

```
intent.created
plan.review_required
node.started
node.completed
artifact.created
verification.failed
decision.required
effect.committed
intent.completed
```

`schema_version` 按事件类型独立编号，初值为 1。

约束：向后兼容为 MUST。新增字段必须可选；已有字段不得删除、改名或改变语义。

## 5.1 版本迁移链

规则取自 DSH 的会话格式世代管理（见 `2026-09-30-DSH-对照分析.md` 第 4.2 节），按 §310 的事件流改写。

```
相邻版本迁移     每个 schema_version 对应一个冻结的解码器；
                 版本号之间不得有缺口，装配时校验
已提交世代       不得移动、覆盖或删除；兼容性变更只能追加后继版本
结构性变更       只有改变记录结构才递增 schema_version；
                 新增可选字段不递增
```

Failure condition：解码器链存在缺口时装配失败，不得以「最近的可用版本」替代。

## 5.2 未知事件类型

事件类型的成员默认为读时必需。遇到不在解码器 `type` 集合中的事件时：

```
ignorable = false（默认）   拒绝整份事件日志
ignorable = true            跳过该事件，计入恢复报告的跳过计数
```

`ignorable` 只在事件类型新增时为 `true`，既有九类事件的 `ignorable` 恒为 `false`。

Failure condition：无法兼容的变更必须新建事件类型，不得修改既有类型的语义。新建类型若未标记 `ignorable`，旧构建拒绝整份日志，这是预期行为而非故障。

# 6. Audit Log

§313 规定八类事件 MUST 记录：

```
authority changes
device join
device revoke
capability grants
external effects
contract changes
user approvals
runtime self-update
```

§114 规定 Audit Log 不可轻易篡改。该项属设计说明，不构成规范级约束（01 §9.3 的 B4）。

v0.1 取法：

```
append-only 表，不提供更新与删除接口
每条记录含 prev_hash 与 record_hash，构成哈希链
保留期不设上限
不做外部时间戳，不做签名
```

Failure condition：单条记录被改写后，该记录之后的所有记录哈希校验失败。校验在读取时执行。本地攻击者可重写整条链，该项防护不在 v0.1 范围内，对应 02 §10.1 的可推迟项。

# 7. 接口冻结

三个接口以 trait 定义在 `continuum-provider`，无实现。trait 只依赖 `continuum-core` 的类型。

```
ModelProvider   list_models / describe_model / invoke / stream / cancel / usage / health   §315
ToolProvider    list_tools / describe_tool / invoke / cancel                               §316
Connector       按操作细分，不得退化为服务级授权                                            §124、§125
```

## 7.1 规范内部冲突的裁决

§80 与 §315 对同一接口给出不同方法集：

```
§80   discover / capabilities / invoke / stream / cancel / usage
§315  list_models / describe_model / invoke / stream / cancel / usage / health
```

裁决：以 §315 为准。§80 位于设计段（§62b–§148），§315 位于规范段（§219–§348）。§80 的方法集视为非规范设计意图，对应关系为 `discover` → `list_models`、`capabilities` → `describe_model`，`health` 无对应项。该冲突未出现在 01 §9.3 的 C 类清单（C1–C4）内。

§135 与 §310 的事件名存在同类差异：§135 的示例含 `task.completed`，§310 的九类事件中无此项，对应项为 `intent.completed`。裁决同为以 §310 为准，§135 的示例视为非规范。

## 7.2 中立性约束

`continuum-core` 与 `continuum-persist` 不得引用任何 Provider 实现类型，对应 §346 的模型无关、Provider 无关、工具无关。

# 8. 测试策略

```
判据 1  事务原子性
        测试进程在事务内派生一个子进程执行写事务，
        对子进程发送 SIGKILL，重启后断言不存在 §318 列举的不一致状态
判据 2  §319 恢复
        构造 incomplete effect 与 RUNNING 节点状态，执行启动流程，
        断言五个阶段按序执行且已注册钩子被调用；
        另按第 4.1 节构造四类输入：可跳过 payload、未知类型且非 ignorable、
        未知类型且 ignorable、可跳过 payload 后接 intent.completed，
        断言前两者与第四者致命、第三者可跳过且计入 skipped_records
判据 3  Event Stream
        断言九类事件可构造、可序列化、可反序列化；
        断言新增可选字段不改变既有反序列化结果；
        断言九类事件的 ignorable 恒为 false
判据 4  Audit Log
        断言八类事件可写入；改写一条记录后断言哈希校验失败
判据 5  接口中立
        以一个假 ModelProvider 实现替换 trait；
        断言 continuum-core 与 continuum-persist 的依赖列表中不含 Provider 实现 crate
```

判据 5 的断言方式为 `cargo tree` 输出检查。

# 9. 遗留风险与未决项

```
crate 方向       ENG-001 的候选机制；方向定下后 P6 只加固，不新增反向依赖
A12 / OPEN-006   Event Stream 的字段结构与版本规则仍开放；
                 schema_version 一旦对外，后续改动受第 5 节约束。
                 第 5.1 节的迁移链与第 4.1 节的跳过计数取自 DSH 的做法，
                 是本子项目引入的机制，规范层未要求
B4               防篡改在 v0.1 只覆盖单条改写，不覆盖整链重写
§317             本子项目只建立机制，十类状态的表结构由对应层建立；
                 在对应层完成前，§317 的恢复保证不成立
事务 API 强制    依赖数据库连接的 crate 私有性；
                 若出现跨 crate 的直接数据库访问需求，该约束被削弱
```
