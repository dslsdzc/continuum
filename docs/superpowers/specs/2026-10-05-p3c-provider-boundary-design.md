# P3 资源层（子项目 C）设计：Provider / ToolProvider 中立边界

**范围**：《工程》§4.1 的十一个组件中，本子项目建 **Provider Adapter（§315 §80）** 与
**ToolProvider（§316）** 这条中立边界的**注册与发现机制**，并把它对**中立性**的要求落成可断言的形式。

前提是共享面 `docs/superpowers/specs/2026-10-05-p3-bcdf-ownership-and-interfaces.md`（下称「共享面」）：
路径归属、已冻结的接口（§315/§316，含方法清单与类型位置）、四条横切约束。**本文不与共享面冲突的地方
以共享面为准**；有冲突的地方在正文中点名。

次序为 P3 分解 A → B → C → D → E 中的 **C**。A（Capability）是 B/C/D 的公共前置（共享面 §五），
B（连接器）与 C 在同一条「外围中立边界」上但**对象不同**：B 管 `Connector`（§124/§125，外部服务），
C 管 `ModelProvider` / `ToolProvider`（§315/§316，模型与工具）。D 的 Router 是 `ModelProvider` 的
消费方，F（驱动侧工具调用路径）是 `ToolProvider` 的消费方，两者今天都尚不存在。

---

# 1. 范围

## 1.1 建什么、不建什么

| 组件 | 规范依据 | 本子项目 |
|---|---|---|
| Provider 的**注册**（谁把一个适配器登记进来）与**发现**（调用方按什么找到它） | §315 §316 §80 | 建 |
| **中立性强制**：核心不得持有适配器实现细节，只能经接口调用 | §315 §316 §80 §10.3、02-工程 §4.2 §1.2 §1.5 | 建（断言形式） |
| Provider 的**错误 / 超时 / 取消契约** | §315 §316 | 建（契约文字 + 一处类型缺口的上报） |
| `continuum-provider` 里 trait 与 `continuum-core` 里接口类型 | §315 §316 | **已在，不重建**（共享面 §二） |
| 具体适配器（DeepSeek / OpenAI / 本地模型……） | §80 | **不建**（本仓只有 DeepSeek 单一端点；且无消费方） |
| `Router` 与候选排序、ModelProfile / Model Registry | §247–§251 | 不建（子项目 D） |
| `Connector`、权限细分到能力、凭据怎么取 | §124 §125 | 不建（子项目 B） |
| 驱动侧 `authorize` → `ToolProvider::invoke` 的调用路径 | §4.2 §252 §253 | 不建（子项目 F） |

**不建的部分不留桩。** 具体适配器是**唯一**能让本子项目产出「生产调用方」的东西（见 §9），它不在本轮
范围内，故本轮建的是一套**当前无生产调用方**的机制——这一点在 §9 与 §12 逐条记明，不掩盖。

## 1.2 与 §80 的关系

§80 给的 `ModelProvider` 方法集（`discover` / `capabilities` / `invoke` / `stream` / `cancel` / `usage`）
与 §315 不同。该冲突已由 P0 设计第 7.1 节裁决**以 §315 为准**，对应关系为 `discover` → `list_models`、
`capabilities` → `describe_model`（`docs/superpowers/specs/2026-09-30-p0-infrastructure-design.md:248-252`）。
本子项目照此，**不新增任何 §80 独有的方法**。

---

# 2. 现状清点：trait 已在，缺的是什么

## 2.1 已存在的东西（照录，不改）

**§315 ModelProvider**，七项方法，`crates/continuum-provider/src/model.rs:14-21`：

```
list_models / describe_model / invoke / stream / cancel / usage / health
```

接口类型在 `crates/continuum-core/src/model.rs`：`ModelId`(8)、`ModelDescriptor`(20)、`Role`(29)、
`Message`(37)、`InvokeRequest`(43)、`Usage`(50)、`InvokeResponse`(56)、`StreamChunk`(63)、
`CallId`(69)、`ProviderHealth`(81)、`ModelStream`(93)。

**§316 ToolProvider**，四项方法，`crates/continuum-provider/src/tool.rs:9-13`：

```
list_tools / describe_tool / invoke / cancel
```

接口类型在 `crates/continuum-core/src/tool.rs`：`ToolId`(6)、`ToolDescriptor`(18)、`ToolInvocation`(25)、
`ToolResult`(31)。`ToolId` **同时**是 §252 的 `Tool.id`，P3A 判为同一个类型并复用
（`crates/continuum-capability/src/lib.rs:38-41`）；本子项目**不另建第二个**。

错误类型 `ProviderError` 在 `crates/continuum-core/src/error.rs:3-15`，五个变体：
`Unavailable` / `UnknownModel` / `Cancelled` / `Transport` / `Protocol`。**它不在共享面 §二冻结的类型清单里**
（那是逐条列出的接口类型），它是支撑类型；本子项目对它的处置见 §5.1。

`continuum-provider` 全 crate **只定义 trait、零实现、零 I/O**，其 crate 文档写明「不含实现，也不引用任何
实现类型（§346）」（`crates/continuum-provider/src/lib.rs:1`）。它今天的依赖边只有 `continuum-core`
一条，`ALLOWED` 里逐对断言。

## 2.2 缺的东西，一处一处说

**一、注册：没有任何东西能把一个适配器登记进来。**
全仓对 `continuum_provider` 的引用**只有 `continuum-provider` 自己**（`grep -rn continuum_provider crates/`
除本 crate 外零命中）；`ModelProvider` / `ToolProvider` 这两个名字在别的 crate 里**只出现在注释里**
（`crates/continuum-graph/src/execution.rs:17` 预告 P3 会收紧 `ExecutionProfile` 的六个 `Option<String>` 字段；
`crates/continuum-capability` 的注释说明 `ToolId` 复用）。今天唯一实现这两个 trait 的东西是测试夹具
`crates/continuum-provider/tests/fake_provider.rs:29,89`（`FakeModel` / `FakeTool`），**只在测试里**。

**二、发现：调用方没有任何按 id 找到适配器的入口。**
`InvokeRequest.model: ModelId`（D 侧）与 `ToolInvocation.tool: ToolId`（F 侧）都是「调用方按 id 找服务」
的形状，而今天没有任何东西回答「哪个适配器服务这个 id」。

**三、非流式不可取消（`cancel` 对非流式调用不可达）。** 这是一处**既有类型事实**，不是漏实现：`ModelStream` 带
`call: CallId`（`crates/continuum-core/src/model.rs:94`），故 `stream()` 的返回给得出可取消的句柄；
而 `InvokeResponse`(56)、`ToolInvocation`(25)、`ToolResult`(31) **都没有 `CallId` 字段**，
`invoke()` 与 `ToolProvider::invoke()` 的调用方**拿不到任何 `CallId`**，也就无从调用
`cancel(&CallId)`。见 §5.3 与 §12 第 1 条。

**四、`describe_tool` 没有「无此工具」这一种 `Err`。** `ProviderError` 有 `UnknownModel` 而**没有
`UnknownTool`**（`crates/continuum-core/src/error.rs:4-15`）。既有夹具因此把「这个 provider 没有这个工具」
报成 `Unavailable`（`crates/continuum-provider/tests/fake_provider.rs:106`），而 `Unavailable` 的另一层
意思是「provider 挂了」——两者被同一变体承载。本子项目的处置见 §5.2。

**五、`ExecutionProfile` 里模型/工具相关字段仍是 `Option<String>`。** `crates/continuum-graph/src/execution.rs:18-20`
的注释预告「前四者 P3 接入 `ModelProvider` 与 `ToolProvider` 时收紧为强类型 id」。本子项目**不做这次收紧**：
它要动的是一张已落库的表与一个已冻结的 `struct`，而**没有消费方要求它**（无 D、无 F），
按「不预先发明」不动。记在 §12 第 9 条，收件人是 D 或 F。

---

# 3. 注册与发现

## 3.1 决定：进程内注册表，按 id 显式登记，装配期填装

**注册表**建在 `continuum-provider` 内，一个 `ProviderRegistry` 结构体，持两张表：`ModelId → Arc<dyn ModelProvider>`
与 `ToolId → Arc<dyn ToolProvider>`，另各持一份登记顺序的适配器列表（供枚举）。

**注册**在**装配期**由驱动（`continuum-runtime` 的装配点，今天 `main.rs`）调用，形如
「把这一组 id 登记给这个适配器」——**id 由登记方显式给出**，不由注册表去问适配器。

**发现**由调用方按 id 查，**两条 trait 的暴露面不同，理由是强制点 (1) 只覆盖工具**（§7.2）：

- **模型侧不设强制点**（§5.3 的三个强制点都不管模型），故 `model_for(&ModelId)` 直接交出
  `Arc<dyn ModelProvider>`，另开 `model_providers()` 供 D 构建候选时遍历全部适配器。
- **工具侧设强制点 (1)**，故注册表**不交出** `Arc<dyn ToolProvider>`：不提供 `tool_for`、也不提供
  `tool_providers()` 枚举。工具侧只开两类入口——**只读的** `list_tools()` / `describe_tool(&ToolId)`
  （它们不是副作用，可自由暴露），以及**唯一的调用入口** `invoke_tool(&AuthorizedTool, input)`
  （§7.2）。适配器在注册表内部持有，出不了这个 crate。

未命中：模型侧返回 `RegistryError::NotFound`，工具侧返回 `ToolCallError::Unregistered`（§7.1）。

**重复登记同一 id** 返回 `RegistryError::Duplicate`，与 `crates/continuum-operator/src/registry.rs:20-29`
的 `OperatorError::Duplicate` 同形——本仓已有这个先例，本子项目沿用其形状，不另发明一套。

## 3.2 为什么是它，而不是另外三种

**为什么是进程内，不是一张表。** 适配器是**代码**（`Arc<dyn ModelProvider>`），不是数据。数据库表存不下一枚
trait object；一张「provider 表」最终还是要在进程内再建一次 `表行 → 适配器` 的映射，等于**同一件事两个产生点**。
本仓的数据库访问一律经 `continuum-persist` 的 `Tx`、`continuum-core` 不含 I/O（P3A 设计 §3.4 记了这条边界），
故一张自带连接的注册表在本仓无从写出——这一点与 P3A 判 `authorize` 取自由函数、不建 `ToolRegistry`
**结论相同而理由不同**：那里的理由是「没有需要挂在 `self` 上的状态」（`crates/continuum-capability/src/registry.rs:1-16`），
这里的注册表**有**状态（已登记的适配器），故建结构体是对的。**两者不可互推。**

**为什么按 id 显式登记，而不是注册表去问适配器。** 备选是「登记时调一次 `list_models()` / `list_tools()` 建索引」，
或「发现时逐个 `describe_model()` 探测」。两者都能免掉 §3.3 的重复来源，但都有代价：

- 登记时问：注册变成 `async`，装配期就要联网，且装配会因**远端 provider 暂时不可达**而失败——
  把一个「谁来服务这个 id」的**路由事实**绑到一次**网络调用**上。
- 发现时探测：路由路径上每查一次就发一次 I/O；更糟的是**「不是我的」这个信号不可靠**——
  模型侧有 `UnknownModel` 可用，工具侧没有（§2.2 第四条），且任一 provider 报 `Transport` / `Unavailable`
  时无从区分「它没有这个工具」与「它挂了」。为这条改 `ProviderError` 的取值域，代价大于收益（§5.2）。

显式登记的代价见 §3.3，据实记，不粉饰。

## 3.3 这个选择的代价

**代价一：登记 id 与适配器自己的 `list_models()` / `list_tools()` 是两个产生点。**
「这个适配器服务哪些 id」这件事被写了两次：一次在装配期的登记调用里，一次在适配器自己的
`list_models()` / `list_tools()` 里。这是本项目一贯判为缺陷的那一类形状。**处置**：
登记是**路由**的唯一权威（发现只查登记），`list_models()` / `list_tools()` 是适配器给**调用方**的
**描述**（D 取 `ModelDescriptor` 用于画像与排序）。两者若不一致（登记了 `m`、适配器不认 `m`），
是一个**配置缺陷**，其可观察形态是「`model_for` 命中、随后 `describe_model` 报 `UnknownModel`」。
**本阶段没有照片**：要拍它必须先有一个真实适配器（§9），本阶段把它记成 §12 第 6 条。

**代价二：登记集合是静态的。** 适配器运行中新增一个模型，注册表不会知道，除非重新登记。
本阶段的适配器集合与模型集合都是静态的，接受。适配器若要做动态模型，改法是「把 `list_models` 也纳入发现路径」，
那正是 §3.2 否掉的两条备选之一，届时连同 §3.3 代价一一起重新裁定。

## 3.4 两个类型，还是一张注册表：取一张

`ModelProvider` 与 `ToolProvider` 是两条 trait，本可以各建一个注册表。本子项目取**一个 `ProviderRegistry`
持两张表**，理由：装配点只有一个（驱动），一张注册表让驱动只持一个句柄；拆成两个会让驱动为「另一张它不同时
需要」的表多持一个句柄。**否掉的替代**：两个结构体——好处是模型侧的消费方（D）在类型上连
`list_tools` 都够不到。但工具侧的**调用**入口已由 `AuthorizedTool` 把关（§7.1），注册表本身不是
授权边界；拆开省下的只是形式上的整洁，不够抵掉装配的额外形状。

## 3.5 名称与位置

- 结构体 `ProviderRegistry`，文件 `crates/continuum-provider/src/registry.rs`，经 `lib.rs` 再导出。
- 两个错误类型：`RegistryError`（`NotFound` / `Duplicate`，登记与查找用），与 **`ToolCallError`**
  （`Unregistered { id }` / `Provider(ProviderError)`，受门禁的 `invoke_tool` 用）。
  **为什么需要第二个**：`invoke_tool` 的失败有两个不同来源——「这个 id 没有适配器」（路由，
  配置缺陷）与「适配器调用失败」（可能瞬时）；一个 `Result` 只能带一个错误类型，把两者压进
  `ProviderError` 就是把「配置错了」报成「provider 挂了」（§5.2 记的正是这种混淆的既有实例）。
- **三者判据不同，不可合并**：`ProviderError` 描述**一次 provider 调用**的失败；`ToolCallError::Unregistered`
  与 `RegistryError` 描述**路由 / 装配**的失败（配置缺陷，不是瞬时）。合并会让「重试一次」这条判据
  拿到一个不该重试的错（§5.1）。

---

# 4. 中立性怎么被强制

## 4.1 三条边，逐条钉

§315/§316 与 §10.3 要求核心不得持有实现细节、只能经接口调用；`docs/02-工程.md:239` 原文是
「**Provider Adapter** 与 **ToolProvider** 是中立边界，核心不能持有其实现细节，只能通过 §315 / §316
定义的接口调用」。P3A 的 `ALLOWED` 纪律（`crates/continuum-runtime/tests/dependency_direction.rs:25-139`）
是这条要求的现成载体，本子项目**不另建机制**，只把落点写清：

| 角色 | crate | 允许依赖 | 依据 |
|---|---|---|---|
| 接口（trait） | `continuum-provider` | `continuum-core` + `continuum-capability` | §4.1 订正：`capability` 是为工具侧的唯一调用入口 `invoke_tool(&AuthorizedTool, ..)` 而加的（层内边，§7.2） |
| 接口类型 | `continuum-core` | 无内部依赖 | 现状 |
| 实现（适配器） | 未来的 `continuum-adapter-*`，**本阶段不建** | `continuum-provider` + `continuum-core` | §4.2 |
| 核心 / 持久化 | `continuum-core`、`continuum-persist` | **不得**依赖 `continuum-provider` | 已有专门用例 |

**订正（2026-10-05，协调者裁定强制点 (1) 走类型强制之后）**：本节原稿写「注册表加进来之后
`continuum-provider` 的内部依赖边**不变**，`ALLOWED` 一字不改」。**那句现在为假，就地订正。**
工具侧改为只开**受门禁的**调用入口 `invoke_tool(&AuthorizedTool, ..)` 之后，本 crate 必须能命名
`continuum_capability::AuthorizedTool`，故 `ALLOWED` 的该条目由 `&["continuum-core"]` 改为
`&["continuum-capability", "continuum-core"]`（层内边，见 §7.2）。**原稿那句错误的来历**是：
它是在「注册表只做路由、交出裸适配器」的前提下写的，那条前提已被裁定改掉。

**仍然成立、且是本子项目真正要断言的关键性质**：注册表加进来之后，
**`continuum-provider` 没有多出任何指向适配器实现的边**。注册表用的是
`std::collections::HashMap` 与 `std::sync::Arc`，引用的类型是 `Arc<dyn ModelProvider>` /
`Arc<dyn ToolProvider>` / `ModelId` / `ToolId` / `AuthorizedTool`——**无一是实现类型**。
若注册表引用了任何具体适配器类型，编译期就会要求一条新边，而那条边必须先写进 `ALLOWED`
才能通过 `every_crate_depends_only_on_its_allowed_set`
（`crates/continuum-runtime/tests/dependency_direction.rs:203`）。故中立性仍是**结构性**的，
只是多出的一条边是「接口 → 能力类型」，不是「接口 → 实现」。

## 4.2 违规长什么样

三种，各有**可观察形态**（这也是本子项目对「中立性怎么强制」的回答——强制点不是一句话，是三条会红的用例）：

1. **核心 crate 反向依赖 provider。** `continuum-core/Cargo.toml` 或 `continuum-persist/Cargo.toml` 加了
   `continuum-provider` → `core_and_persist_do_not_depend_on_provider`
   （`crates/continuum-runtime/tests/dependency_direction.rs:244`）红，且 `ALLOWED` 的逐对断言同时红。
2. **中立 crate 引用了适配器实现。** `continuum-provider` 里出现某个具体适配器类型名 → 它没有那条依赖边，
   **编译不过**；若有人把边补进 `Cargo.toml`，则 `ALLOWED` 的逐对断言红（表里没有那条边）。
3. **消费方持有实现。** D 的 Router（未来的 `continuum-router`）或 F（驱动）把适配器类型写进自己的类型签名里
   → 它的 crate 必须依赖 `continuum-adapter-*`，而它的 `ALLOWED` 条目只应含 `continuum-provider`。
   这条**本阶段无照片**（D 与适配器都不存在），它的判据写在 `ALLOWED` 表的注释里，留给登记 D 的那个 task。

## 4.3 一条遗漏：`ALLOWED` 对未来的适配器 crate 该怎么登记

现状是 `ALLOWED` 里每一对都逐对 `assert_eq!`，且 `workspace_crates()` 派生出的成员必须**逐个**出现在
`ALLOWED` 的主体里（`crates/continuum-runtime/tests/dependency_direction.rs:209-221`），否则一条用例红
「workspace 成员 X 未列入 ALLOWED」。故未来加一个 `continuum-adapter-*` 时，**加 crate 与加 `ALLOWED` 条目
是同一件事**，漏了会红。这是「依赖边只登记实际用到的」这条横切约束（共享面 §四.4）在 C 上的落点。
本子项目**不预建**任何适配器 crate。

---

# 5. 错误、超时与取消的契约

## 5.1 `ProviderError` 不映射到 `FailureClass`，映射是调用方的事

`FailureClass`（`crates/continuum-graph/src/failure.rs:6-16`）有七个取值，判定函数 `decide_retry`
（同文件 `:106`）要一个 `&Operator` 与一个 `&RetryPolicy`。

**决定：provider 边界不产出 `FailureClass`，也不把它映射成一个函数。** 理由两条：

1. `FailureClass` 属于执行层（`continuum-graph`），它是**重试策略**的词汇，而重试策略由调用方（执行器）
   持有——`decide_retry` 的另外两个入参（`Operator`、`RetryPolicy`）在 provider 边界上都不存在。
   在边界上映射等于把调用方的策略词汇塞进中立层。
2. 映射在**每条路径上是否为真**，本子项目答不出：`Transport` 在一个适配器上可能是瞬时的（连接抖动），
   在另一个上可能是永久的（端点配错）。把它写成一个函数，等于替所有适配器宣布一条**它未必成立**的分类。

**故本子项目的产物是一张「建议映射」的文档表，不是代码**（无消费方，不预置 API）：

| `ProviderError` | 建议 `FailureClass` | 说明 |
|---|---|---|
| `Transport` | `Transient` | 适配器若明知是永久错，应改报 `Protocol` |
| `Unavailable` | `Transient` | 同上 |
| `Protocol` | `Permanent` | 契约不符，重试无益 |
| `UnknownModel` | `Permanent` | 配置缺陷 |
| `Cancelled` | **不进入分类** | 是调用方自己发的取消，不是失败（§5.3） |

另两个**不属于** `ProviderError`、但调用方同样要分类的错：`ToolCallError::Unregistered` 与
`RegistryError::NotFound`（模型侧的未命中）——两者都是**路由 / 配置缺陷**，建议归 `Permanent`，
**且不得当作可重试**（它们不是 provider 的瞬时状态，重试不会变）。§5.2 记的既有夹具正把这类错
报成 `Unavailable`（一个瞬时类），本子项目在公开面上把两者分开（§3.5 的 `ToolCallError`）。

**这张表不被任何用例钉住**（无消费方），故它按本项目的规矩**标明它是建议**：D 或 F 落映射时以**自己**的
适配器集合为准，本表只是默认起点。§12 第 7 条记一处它现在答不出的东西：`FailureClass` 有 `Resource`
（P1 交接 `docs/superpowers/p1-followups.md` 第三节要求 Router 为 `Resource` 显式给
`max_attempts >= 2`），而 `ProviderError` **没有任何表示「被限流 / 配额耗尽」的变体**——这类情形今天
只能落进 `Unavailable` 或 `Protocol`，**丢掉了 `Resource` 这个类别**。

## 5.2 不加 `UnknownTool`，但把缺口记下

§2.2 第四条记了 `UnknownModel` 有、`UnknownTool` 无。**决定：本阶段不加这个变体。** 理由是本项目的
「不预先发明」：按 §3.1 的显式登记，`describe_tool` 只会被「注册表说这个适配器拥有这个 id」之后调用，
故「无此工具」这一情形在正常路径上**不可达**；为一个不可达的取值加一个变体，等于造一个无产生方的值。
既有夹具把未知工具报成 `Unavailable`（`crates/continuum-provider/tests/fake_provider.rs:106`）是**夹具的选择**，
本子项目不改它，但在 §12 第 8 条记明：F 接上之后，「工具表里有、适配器不认」这一情形会**变成可达**，
届时再决定是加变体还是定一条 `Protocol` 的用法。**这条不留给读者推断**，故写在此处与 §12。

## 5.3 超时与取消

**超时不进签名。** §315/§316 的方法集是冻结的（共享面 §二），且规范未给 provider 调用的超时来源。
本子项目**不新增 `timeout` 参数**（那会发明一个无规范来源的 API）。调用方对 `async` 调用加**异步截止**：
本仓已有承载处——`ExecutionProfile.timeout_ms: Option<u64>`（`crates/continuum-graph/src/execution.rs:28`）。
**否掉的替代**：把超时放进 `InvokeRequest`——它会与 `ExecutionProfile.timeout_ms` 形成两个超时来源，
且 §315 的方法集不含它。

**取消只有一条路径：`cancel(&CallId)`。** `ModelStream` 的文档已写死一条不留给读者推断的契约：
「取消经 `CallId` 走 `cancel()`，**不经流的 drop**」（`crates/continuum-core/src/model.rs:91-92`）。
故：**丢弃一个流不是取消**，调用方要停必须显式调 `cancel(&stream.call)`。

**`cancel` 的语义定为幂等、尽力而为**：对已完成或未知的 `CallId` 调用返回 `Ok(())`（既有夹具即此形，
`crates/continuum-provider/tests/fake_provider.rs:73`）。理由：取消与完成**天然竞态**，把「取消了恰好已完成的调用」
判成错误会让正常路径必须处理一个不该是错的错。**否掉的替代**：为未知 `CallId` 加一个 `Err` 变体——
它把竞态变成错误，且要改 `ProviderError` 的取值域。

**非流式不可取消**（即非流式调用不可取消）**——定案，且不改任何冻结类型。** 事实：`InvokeResponse`（`crates/continuum-core/src/model.rs:56-61`）
与 `ToolResult`（`crates/continuum-core/src/tool.rs:31-35`）都不带 `CallId`，只有
`ModelStream`（`crates/continuum-core/src/model.rs:93-96`）带。**结构上的原因是**：`invoke()` 的返回值
**只有在调用完成之后才存在**——调用方拿到它的那一刻，已经没有东西可取消了。故「给 `InvokeResponse` /
`ToolResult` 加一个 `call: CallId` 字段」**修不好这件事**：那是一个完成态的物证，不是一张未完成调用的句柄。

要让非流式调用可取消，得让调用方**在调用之前**就持有 `CallId`——即**改请求侧**，不是响应侧：
`InvokeRequest` 由调用方自带一个 id，或把 `invoke` 拆成「发起 → 拿句柄 → await」。那是一次大得多的接口改动，
本阶段没有消费方要求它（无 D、无 F 需要取消非流式调用），故**不做**。

**结论**：`cancel` 的有效射程**只有 `stream()`**（经 `ModelStream.call`）。任何按「所有调用都能取消」
写的实现都是错的。将来若要非流式可取消，**动的是请求侧**。§12 第 1 条记此。

---

# 6. 与 D 的关系

D 的 Router 是 `ModelProvider` 的消费方。**调用面，按序**：

1. **枚举**：`ProviderRegistry::model_providers()` 拿到全部适配器。
2. **取描述**：对每个适配器 `list_models()`，或对已知 id 用 `describe_model(&ModelId)`；
   得到 `ModelDescriptor`（含 `provider: String`、`context_window`、`capabilities`）。
3. **路由**（D 自己的逻辑：画像、Registry 生命周期、预算视图、§248 禁止 `overall_score`）——**不在本层**。
4. **解析**：`ProviderRegistry::model_for(&ModelId)` → `Arc<dyn ModelProvider>`；
   未登记 → `RegistryError::NotFound`。
5. **调用**：`invoke(InvokeRequest)` 或 `stream(InvokeRequest)`。
6. **消费流**：读 `ModelStream.chunks`；要中止则 `cancel(&stream.call)`（§5.3）。
7. **健康**：`health()`。

**D 可以假定：**

- `health(&self) -> ProviderHealth` **不返回 `Result`**（`crates/continuum-provider/src/model.rs:21`）——
  它是 §315 七项方法里唯一不会失败的一项，故 D 可以无条件调用它探活，不必为它准备失败路径。
  **这是签名决定的，不是约定**：一个「探活本身可能失败」的接口没有意义。
- 第 4 步命中之后，第 5 步的调用**不需要再出示任何凭据**——`ModelProvider` 不在强制点上
  （三个强制点分别管工具、副作用、凭据，共享面 §四）。
- 第 2 步的 `ModelDescriptor.provider` 是**描述**，第 4 步的登记是**路由**；两者可能不一致（§3.3 代价一）。

**D 不得假定：**

- 不得持有任何适配器实现类型（§4.2）。
- **不得用 `usage()` 做成本决策。** ENG-005 已定 Router 的成本输入是**预算视图**（剩余额度），
  它来自语义层（共享面 §三），不是 provider 的 `usage()`。`usage()` 的语义（累计？本会话？）
  今天**未定义且无消费方**，见 §12 第 3 条。
- 不得因为 `list_models()` 里没有某模型就断定它不存在于登记表（两者是两个产生点，§3.3）。

---

# 7. 与 F 的关系

F（Runtime / 驱动）是 `ToolProvider` 的消费方，路径是 `authorize` → `ToolProvider::invoke`
（共享面 §一）。

## 7.1 工具侧**唯一**的调用入口是受门禁的 `invoke_tool`

**决定（协调者裁定，2026-10-05）**：强制点 (1) 走**类型强制**。注册表**只开**
`invoke_tool(&AuthorizedTool, input) -> Result<ToolResult, ToolCallError>` 这一条调用入口，
**不交出 `Arc<dyn ToolProvider>`**（无 `tool_for`、无 `tool_providers()` 枚举）。理由是本项目一贯取
「非法状态不可表达」而非「调用方记得」：拿不到 `AuthorizedTool` 就**拿不到**调用工具的那条路径。
P3A 的两处守卫失败（`AuthorizedTool` 与 `mint`）都指向同一个取向。

**调用面（F 可以假定）：**

- **解析与调用是同一步**：`invoke_tool(&AuthorizedTool, input)`。适配器按
  `AuthorizedTool::tool_id()`（`crates/continuum-capability/src/registry.rs` 的访问器）查得；
  **未登记 → `Err(ToolCallError::Unregistered { id })`**。工具 id **只有这一个来源**——`invoke_tool`
  不另收 `ToolInvocation.tool`，也就没有「出示的 id 与被授权的 id 不是一个」这一种可能。
- **三种结果，不是两种**：工具**跑起来了但自身失败** → `Ok(ToolResult { is_error: true, .. })`；
  provider **没跑成**（传输、协议、不可用）→ `Err(ToolCallError::Provider(e))`；
  **没有适配器** → `Err(ToolCallError::Unregistered { .. })`。`crates/continuum-core/src/tool.rs:31-35`
  的 `ToolResult.is_error` 是第一个通道，故断言与审计要按它分辨，不能把 `is_error = true` 当成 `Err`，
  也不能把 `Unregistered` 当成本次调用失败去重试（它是配置缺陷，§5.1）。
- **只读方法自由暴露**：`list_tools()` / `describe_tool(&ToolId)`，供把工具 schema 交给模型时用。
  它们不是副作用，故不受门禁。

**F 的义务不变**：它必须**先**调 `continuum_capability::authorize` 取得 `AuthorizedTool`，再把它交给
`invoke_tool`（共享面 §四.1）。

**残留（据实记，不声称不存在的保证）**：`AuthorizedTool` 的构造通道只有 `authorize` 一条，
`invoke_tool` 的类型要求因此是真的；**但装配者（composition root，即驱动自己）在装配期构造适配器，
手上本来就有裸 `Arc<dyn ToolProvider>`，它可以直接调 `invoke`。** 注册表不交出来，收不回装配者手里那一份。
这条与 P3A 对 `mint` 的记录同形（`docs/superpowers/p3a-followups.md` 第四节第 6 条：
「唯一签发点」的类型层部分只到「crate 外除 `mint` 外没有第二条产出 `Capability` 的公开路径」，
不是「crate 外造不出来」）。**本子项目承诺的是：在本 crate 的公开面上，除 `invoke_tool` 外没有第二条
到达 `invoke` 的路径**——对照物是 `crates/continuum-provider/tests/compile_fail/` 的编译失败样例（§11），
不是「全仓无人能调用它」。

**代价**：`continuum-provider` 多一条 `→ continuum-capability` 的边（层内边，§4.1）。这条边是
「接口 → 能力类型」，不是「接口 → 实现」，不违反中立性（§4.1 订正段）。

## 7.2 C **不**做的事

- **C 不判定一次工具调用是否获准。** 那是**强制点 (1)**，由 F 调 `authorize` 取得 `AuthorizedTool`
  来保证（§7.1）。C 只保证「没有 `AuthorizedTool` 就走不到本 crate 公开面上的 `invoke`」。
- **C 不改 §316 的 trait。** `ToolProvider::invoke` 仍然收**裸** `ToolInvocation`
  （`crates/continuum-provider/src/tool.rs:12`）——冻结的接口一字不动。门禁落在**注册表**这一层：
  **入参的类型**从 `ToolInvocation` 收紧为「`AuthorizedTool` + 输入」，而**适配器自己的 trait 方法**
  保持原样。故「不交出适配器」是注册表的性质，不是 trait 的性质。
- **C 不决定工具是否授权、作用域够不够、凭据怎么取**——分别是 F、执行点、B（共享面 §四.1、§四.3）。
- **C 不提供任何绕过 `authorize` 的公开入口**，也不改 `authorize`（共享面 §四.1 的措辞见
  `docs/superpowers/p3a-followups.md` 第五节第 2 条与第四节第 6 条）。

## 7.3 与 §252/§316 两个「工具」的关系

见 §8。

---

# 8. §252 的 `Tool` 与 §316 的 `ToolDescriptor` 的处置

P3A 遗留第 8 条把这件事交给 C（`docs/superpowers/p3a-followups.md` 第三节第 8 行），理由是这个形状
「同一个工具在两个层各有一份描述」。两者是：

- `continuum_core::tool::ToolDescriptor`（`crates/continuum-core/src/tool.rs:18-23`）：
  `id` + `description` + `input_schema`。§316 的**接口类型**，是**适配器对外声明的调用形状**。
- `continuum_capability::tool::Tool`（P3A)：`id` + `version` + `input_schema` + `output_schema` +
  `required_capabilities` + `effect_class` + `deterministic`。§252 的**登记项**，是**Registry 的治理记录**。

**决定：两者并存，不合并，靠共用的 `ToolId` 绑定。** 理由：`ToolDescriptor` 的消费者是中立边界另一侧的
适配器（可能是远端、第三方），让它持有 `required_capabilities` / `effect_class` / `deterministic` 等于把
**治理数据**交给**被治理方**——那是强制点 (1) 的语义反转。§316 冻结的 `ToolDescriptor` 刻意不含这三个字段，
本子项目**不动它**。

**由此产生的一处重叠**：`input_schema` 两边都有。**决定权威**：**适配器的 `ToolDescriptor.input_schema`
是调用（invoke）的权威**（它才是真正接收并校验输入的一方）；`Tool.input_schema` 是**规划**用的快照
（Planner 构造调用时用）。两者的一致性**本阶段无从强制**（无 Planner、无适配器），
记在 §12 第 5 条，收件人是 F/D。

`effect_class` 的取值集若被后续判定为「与 `EffectType` 是两个轴」，P3A 遗留第 3 条把重新处置交给 C/D；
**本子项目的立场是维持绑到 `EffectType`**（P3A 设计 §3.2 的理由「开放类型会让策略表漏判」在 C 上同样成立），
不改。

---

# 9. 什么拍不到照片

本子项目要诚实说出它**没有**的东西，理由与本仓 P1/P3A 的先例相同
（`docs/superpowers/p1-followups.md` 第二节、`docs/superpowers/p3a-followups.md` 第一节）：
「机制建好、运行路径不经过」是这个仓库反复出现的缺陷形态。

**没有生产调用方，逐个说：**

| 机制 | 生产调用方 | 为什么没有 |
|---|---|---|
| `ProviderRegistry`（注册 / 发现） | **无** | 消费者是 D 的 Router 与 F 的工具路径，两者**都尚不存在** |
| `ModelProvider` 的任何方法 | **无** | 同上（全仓零引用，§2.2 第一条） |
| `ToolProvider` 的任何方法 | **无** | 同上 |
| 建议映射表（§5.1） | **无** | 是文档，不是代码 |

**能拍到的照片（本阶段就能有）：**

- 注册 / 发现 / 重复 / 未命中：用**既有**的 `FakeModel` / `FakeTool`
  （`crates/continuum-provider/tests/fake_provider.rs:29,89`）即可，不新建夹具。
- 中立性：`dependency_direction.rs` 的 `ALLOWED` 逐对断言（已有）＋反依赖用例（已有）。
- `cancel` 不可达这一条是**结构事实**，照片是类型签名本身（`InvokeResponse` / `ToolResult` /
  `ToolInvocation` 无 `CallId` 字段），不需要运行。

**拍不到的（§12 逐条记）：** 真实适配器的一致性（§3.3）、超时与取消对真实 provider 的行为、
`is_error` 与 `Err` 在真实适配器上的分流、§4.2 第 3 条的消费方持有实现。

**驱动侧的悬空边（据实记，且不由本子项目登记）**：`crates/continuum-runtime/Cargo.toml` 今天声明了
`continuum-provider`，而 `continuum-runtime` 内对它的**引用为零**
（`grep -rn continuum_provider crates/continuum-runtime` 零命中；`ALLOWED` 的注释亦记明
「core / events / provider 在 runtime 内至今无任何引用」）。**定案：本子项目不动它**——
这条边的使用点是 F（驱动侧工具调用路径），而本仓的规矩是**边由用它的那个 task 登记**
（P2b 为此删过两条零使用的边，`docs/superpowers/p2-followups.md` 第四节）。F 落地时自己登记/确认这条边。
§12 第 2 条记此。

---

# 10. crate 划分与依赖边

**不新增 crate，扩 `continuum-provider`。** 理由：

1. 注册表的每一个类型（两条 trait、`ModelId`、`ToolId`、`Arc`、`HashMap`）都已在
   `continuum-provider` 或 `continuum-core` 里；新建 crate 只会给每个消费者多加一条边，换不来隔离。
2. 适配器（实现）将来落在**各自的新 crate** 里，依赖 `continuum-provider`——**中立性靠这条边守住**
   （§4.1），不需要把注册表也搬出去。
3. 层规则（《工程》§4.3）把 `Provider Adapter` 放在资源层，与 `Router` / `Tool Registry` 同层；
   注册表是**接口级**的路由表，不含任何实现，留在中立接口 crate 里不违反该图。

**新增的错误类型** `RegistryError` 与 `ToolCallError` 亦在 `continuum-provider`。
**依赖边只有一处改动**：多一条 `→ continuum-capability`（理由与订正见 §4.1 与 §7.1）。

**本子项目不改的**：`continuum-core`（接口类型与 `ProviderError` 都不动，除 §5.2 明说不加变体之外本来也不加）、
`continuum-capability`（不动 `Tool` / `ToolId` / `authorize`）、`continuum-runtime`（本轮不接线，§9）。

**一处要补的规范缺口**（与共享面 §八 为 B 记的那条同形）：`docs/02-工程.md` §4.3 的**层内依赖图里没有
§316 ToolProvider**。图里有 `Provider Adapter → 被 Router 调用，接口中立`（`docs/02-工程.md:250`），
但 §316 的 `ToolProvider` **既不在图的节点里、也没有它与 Tool Registry / F 的关系**，尽管 §4.1 的组件表
把「ToolProvider（中立）§316」列为本层组件（`docs/02-工程.md:224`）。本子项目**不改那份文档**（不属本轮范围），
记在 §12 第 10 条。

---

# 11. 测试策略

| 验什么 | 怎么验（本阶段可做） |
|---|---|
| 模型侧登记后按 id 发现 | 用 `FakeModel`：登记 → `model_for` 命中同一适配器 |
| 工具侧：拿不到裸适配器 | **trybuild 编译失败样例**：`tool_for` / 任何返回 `Arc<dyn ToolProvider>` 的公开入口都写不出来（照片即编译不过） |
| 工具侧：`invoke_tool` 只收 `AuthorizedTool` | trybuild 编译失败样例：裸 `ToolInvocation` 传不进 `invoke_tool` |
| 工具侧：门禁内的正常路径 | 用 `FakeTool`：先经 `authorize` 取 `AuthorizedTool`，再 `invoke_tool` 得到 `ToolResult` |
| 模型侧未登记 id | `RegistryError::NotFound`，且断言**是哪一个变体** |
| 工具侧未登记 id | `invoke_tool` 返回 `ToolCallError::Unregistered`（**不是** `Provider(..)`），且断言**是哪一个变体** |
| 重复登记同一 id | `RegistryError::Duplicate`，且断言**是哪一个变体** |
| 模型侧枚举 | `model_providers()` 返回全部已登记模型适配器 |
| 中立性（核心不依赖 provider） | 既有用例 `core_and_persist_do_not_depend_on_provider` |
| 中立性（`ALLOWED` 逐对精确） | 既有用例 `every_crate_depends_only_on_its_allowed_set`（条目改为 `&["continuum-capability", "continuum-core"]`） |
| `cancel` 对非流式不可达 | **结构事实**，照片是类型签名（§5.3、§9），不写运行用例 |

**编译失败样例是本子项目对「类型强制」那一半的证据**，与 P3A 的 `tests/compile_fail/` 同形：
判据是**编译不过**，由 `tests/type_level.rs` 的 glob 驱动，每份带 `.stderr` 钉住预期报错。

**本项目既有的三条纪律一并适用**：变异须在全量 `--no-fail-fast` 下得出否定结论（且确认红的**位置**）；
凡注释写绝对措辞须有对应用例（枚举式断言**逐项**有照片）；失败路径须断言是哪一种 `Err`——
§11 表里两条错误用例已按此写。

---

# 12. 遗留与未决项

1. **非流式不可取消——已定案**（§2.2 三、§5.3）：`InvokeResponse` 与 `ToolResult` 的返回值
   **只在调用完成后才存在**，调用方拿到它时已无可取消，故给它们加 `call: CallId` 修不好这件事。
   `cancel` 的有效射程只有 `stream()`（经 `ModelStream.call`）；**冻结类型一字不改**。
   将来若要非流式可取消，**动的是请求侧**（`InvokeRequest` 自带 id，或把 `invoke` 拆成
   发起 → 句柄 → await），是一次大得多的接口改动。**收件人：将来提出该需求的人（优先级：D/F 之外）**。
2. **`continuum-runtime → continuum-provider` 是悬空边，由 F 登记**（§9）：`Cargo.toml` 声明了、
   引用为零，`ALLOWED` 目前容忍它（该表两种语义之别见 `docs/superpowers/p2-followups.md` 第四节）。
   **定案：本子项目不登记、也不删它**——使用点是 F，边由用它的那个 task 登记（P2b 的规矩）。
   **收件人：F**（它落地时确认这条边、并把它变成真使用点，或按叶子 crate 的口径处置）。
3. **`usage()` 的语义未定义且无消费方**（§6）：累计还是本会话？`InvokeResponse.usage` 是**单次**的，
   `usage()` 是**无参**的，两者关系未定。ENG-005 把 Router 的成本输入定为预算视图，故 D **不应**用它。
   **收件人：D**（它若有对账需求，那时定语义）；在那之前，它是一个无产生方语义的接口。
4. **强制点 (1) 走类型强制——已定案，并附一条残留**（§7.1、§7.2）：注册表只开受门禁的
   `invoke_tool(&AuthorizedTool, ..)`，不交出裸适配器；代价是 `continuum-provider → continuum-capability`
   这一条层内边（§4.1 订正）。**残留据实记**：装配者（驱动自己）在装配期构造适配器，手上本来就有
   裸 `Arc<dyn ToolProvider>`，可以直接调 `invoke`——注册表收不回那一份。本子项目承诺的**只到**
   「本 crate 的公开面上除 `invoke_tool` 外没有第二条到达 `invoke` 的路径」（编译失败样例钉住），
   与 P3A 对 `mint` 的记录同形。**收件人：F**（它持有装配点，须自知这一份残留）；
   **收件人：控制器**（若将来要连装配者一起约束，需另设机制，不是本 crate 能给的）。
5. **`input_schema` 有两个产生点**（§8）：适配器的 `ToolDescriptor.input_schema`（调用的权威）与
   `Tool.input_schema`（规划的权威）的一致性**本阶段无从强制**。**收件人：F / D**（F 落调用时若发现
   两者不符，处置在彼处）。
6. **登记的 id 与适配器 `list_*()` 是两个产生点**（§3.3 代价一），一致性**本阶段无照片**（无适配器）。
   要有照片必须先有真实适配器。**收件人：C 的实现计划**（第一个适配器落地时，同轮补一条一致性用例）。
7. **`ProviderError` 没有表示「限流 / 配额耗尽」的变体**（§5.1）：这类情形今天落进 `Unavailable` 或
   `Protocol`，**丢掉 `FailureClass::Resource` 这个类别**，而 P1 交接要求 Router 为 `Resource` 显式给
   `max_attempts >= 2`（`docs/superpowers/p1-followups.md` 第三节）。**本子项目不新增变体**——
   无消费方时不预先发明 API。**收件人：D**，它落 `ProviderError → FailureClass` 映射时一并处置；
   若那时确认需要一个变体，由 D 提出（不属 C）。
8. **`describe_tool` 无「无此工具」这一种 `Err`**（§2.2 四、§5.2）：本阶段不加变体（不可达）。
   F 接上后「工具表里有、适配器不认」会变成可达。**收件人：F / C 的实现计划**。
9. **`ExecutionProfile` 的 `model` / `provider` / `tool` 仍是 `Option<String>`**（§2.2 五）：
   P1 预告 P3 收紧为强类型 id，本子项目**不做**（无消费方，且要动已落库的表）。
   **收件人：D 或 F**（谁先用谁收紧）。
10. **`docs/02-工程.md` §4.3 的层内依赖图缺 §316 ToolProvider 这个节点**（§10），
    与共享面 §八 为 B 记的 Connector 缺口同形。本子项目不改那份文档。**收件人：工程文档维护者 / 控制器**。
11. **`effect_class` 是否与 `EffectType` 是两个轴**（§8，源自 P3A 遗留第 3 条）：本子项目维持 P3A 的绑定，
    不改。**收件人：D**（若 D 的排序需要另一个轴，届时重新裁定）。
