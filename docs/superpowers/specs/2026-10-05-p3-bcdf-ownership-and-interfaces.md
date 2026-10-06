# P3 子项目 B/C/D/F 的归属与接口（共享面）

> 本文不是设计，是**四份设计并行的前提**。它的作用是：让 B、C、D、F 四份设计**对着同一套归属、
> 同一套接口类型**写，不至于各造一套词汇——「同一件事两个词汇表」在本项目一贯判为 **Critical**。
>
> 本文里的判断若与你（用户）在 2026-10-05 的决定冲突，以本文为准并回报；本文之外的口径以规范为准。

---

## 一、路径归属

| 路径 | 归属 | 规范依据 |
|---|---|---|
| 模型/工具**机制**（`ModelProvider` / `ToolProvider`） | **C** | §315 §316 §80 |
| 外部服务（`Connector`） | **B** | §124 §125 |
| 模型画像、Model Registry、候选排序（Router） | **D** | §247–§251 §27 |
| **工具调用路径**（`authorize` → `ToolProvider::invoke`） | **F（Runtime / 驱动）** | 《总纲》§1：「执行过程中的模型选择、Agent 编排、**工具调用**、资源调度、验证、失败恢复和结果交付由 Runtime 负责」 |

**工具调用路径归 Runtime 是本轮由用户拍板的**（2026-10-05）。依据是《总纲》§1 与 §22
（`docs/01-总纲.md:16`、`:22`）。**不要**把工具当作「另一种外部资源」并进 §124 连接器：§10.3 把
§124 / §315 / §316 列为**三组各自待冻结**的接口，且 §316 与 §124 是规范里两个不同的节。

---

## 二、接口冻结现状：**无需重新冻结**

三组接口的 trait 与类型**已在代码里**（P0/P1 期间落地），本轮四份设计**不得**新增第二套 id 或描述类型。

**订正（2026-10-05，事后补）**：本节的标题「**无需重新冻结**」在裁决 `§一` 第 2 条（§316 的请求面扩一个承载
授权/凭据的位置）之后**只对 §124 与 §315 成立**，**对 §316 已不成立**——`ToolProvider::invoke` 的请求参数
要换成 C 设计的新类型（C 的设计 §7.5）。本节初稿写在裁决之前，故原话照留。
**来历**：这条缺口是 C 的定点复核（`final-c`）指出来的——它核对时发现 C 的设计已宣布「共享面 §二『无需重新冻结』
被取代」，却查无对应订正，因为**协调者的裁决 §二订正清单里漏了这一条**。这正是本项目反复出现的形状：
**一处宣布了，另一处没落**。

### §315 ModelProvider（七项方法，与规范清单逐一对应）

`crates/continuum-provider/src/model.rs:14-21`

```
list_models / describe_model / invoke / stream / cancel / usage / health
```

类型在 `crates/continuum-core/src/model.rs`：`ModelId`、`ModelDescriptor`、`Role`、`Message`、
`InvokeRequest`、`Usage`、`InvokeResponse`、`StreamChunk`、`CallId`、`ProviderHealth`、`ModelStream`。
§80 的 `discover` / `capabilities` 已裁决为以 §315 为准（P0 设计第 7.1 节）。

### §316 ToolProvider（四项方法）

`crates/continuum-provider/src/tool.rs:9-13`

```
list_tools / describe_tool / invoke / cancel
```

类型在 `crates/continuum-core/src/tool.rs`：`ToolId`、`ToolDescriptor`、`ToolResult`。
**`invoke` 的请求类型不在那里**——它是 `continuum-provider` 的 `AuthorizedToolInvocation`（见下方订正）。

**订正（2026-10-05，裁决 §五）**：`ToolInvocation` **删除**——`invoke` 的请求侧被 C 设计 §7.5 的新请求类型
（`AuthorizedToolInvocation`）取代，旧类型不再有生产调用方，留着就是**同一个概念两个类型**。
删在 §7.5 那次改动里同批做，**连测试夹具一起**。
`ToolId` / `ToolDescriptor` / `ToolResult` 三个**不动**。
（上面那句「连测试夹具 `crates/continuum-provider/tests/fake_provider.rs` 一起」是 2026-10-05 的原话，
**该路径到落地时已经不对了**：Task 1 / Task 2 把 `FakeModel`、`FakeTool` 搬进了
`tests/common/mod.rs`，Task 2 又新造了 `tests/registry_tools.rs`（内含两个 `impl ToolProvider`）。
**要改的是三处**：`tests/common/mod.rs` 与 `tests/registry_tools.rs` 的两个 `impl`。原话照留为来历。）

**落地（2026-10-06，P3 子项目 C 的 Task 3，commit `a0a8a07`）**：
上述删除与请求面换血已完成。`AuthorizedToolInvocation<'a>` 落在 `crates/continuum-provider/src/tool.rs`
（类型 `pub`、构造入口 `pub(crate)`、字段私有——C 设计 §7.5 的「可见性三件套」）；
`grep -rnw ToolInvocation crates/` 已**零命中**（`-w` 是词边界，否则会被 `AuthorizedToolInvocation` 子串命中）。

**注意一处既有事实**：`ToolId` **同时**是 §252 的 `Tool.id`——P3 子项目 A 判为同一个类型并**复用它**
（`crates/continuum-capability/src/lib.rs:40` 有说明）。**再不要造第二个 `ToolId`。**

### §124 / §125 Connector

**出处**（补记，2026-10-05）：**§124 在 `docs/spec/02-positioning.md:1803`**（Service Connectors）、
**§125 在同文件 `:1828`**（Connector Permission）。**不在 `docs/spec/05-normative.md`**（该文件里
`^## 124.` / `^## 125.` 零命中）——本文初稿只写了节号、未写出处，此处补上，免得后来者按「§ 都在
05-normative」的习惯找错文件。§125 的给例：`GitHub.read_repo / GitHub.create_issue /
GitHub.push_branch / GitHub.merge`、`Email.read / Email.draft / Email.send`；§124 有一句定性：
**「Connector 不等于 Agent，它只是 Capability Provider」**（`docs/01-总纲.md:690`；`docs/spec/02-positioning.md:1818-1824` 为原文所在处）。

`crates/continuum-provider/src/connector.rs:12-16`（`descriptor()` / `invoke(op, input)`）；
类型在 `crates/continuum-core/src/connector.rs`：`ConnectorId`、`ConnectorOp`、`ConnectorDescriptor`。

**§125 的「按操作细分，不得退化为服务级授权」已经落在类型上**：`ConnectorOp::new` 返回 `Result`
（非法操作名在构造期即拒），且 `ConnectorDescriptor::new(id, operations)` 要求显式声明操作集。

---

## 三、跨层边：D 的预算视图

D 的成本输入定为**预算视图**（剩余额度，不是「一个数」），依据是 ENG-005 的裁决
（`docs/superpowers/specs/2026-10-05-eng-005-budget-accounting.md`）。

**它来自语义层（第 2 层）**——§9.1 的层间依赖图里 **`语义层 (2) → 资源层 (4)`** 是既有方向
（`docs/02-工程.md` §9.1，「依赖方向单向，无环」）。

**语义层尚未建**，故 D 的设计必须把这个视图写成**一个待实现的接口**（对着已冻结的签名写），
**不得**自己造一套预算记账——记账（含预扣/结算、父子分配）按裁决归语义层。

---

## 四、四条横切约束（四份设计都必须遵守）

1. **强制点 (1)**：由 **F** 调用 `continuum_capability::authorize`；**拿不到 `AuthorizedTool` 就调不了工具**。
   B / C / F **都不得**提供绕过它的入口。它的措辞细节见
   `docs/superpowers/p3a-followups.md` 第五节第 2 条（同形限度）与第四节第 6 条。
2. **强制点 (2)**：`AuthorizedEffect` 由**驱动**铸出，**B 收下才能做副作用**；驱动不得把它或其内容
   塞进命令的环境（§51）。
3. **强制点 (3)**：凭据由 `continuum-secrets` 按能力**逐枚签发**；**B 是它的第一个真消费方**
   （今天全仓零引用，连依赖边都没有——见 `docs/superpowers/p3a-followups.md` 第一节）。
4. **依赖边只登记实际用到的**（`ALLOWED` 与实际依赖**精确一致**，断言是逐对 `assert_eq!`）；
   P2b 曾为此删过两条零使用的边。

---

## 五、四份设计的范围

| 子项目 | 一句话范围 | 主要规范依据 |
|---|---|---|
| **B** | §124/§125 的落地——连接器的操作集、**权限细分到能力的映射**、**凭据怎么取**、以及如何收下 `AuthorizedEffect` | §124 §125 §51 §100 §103 |
| **C** | §315/§316 中立边界的落地——trait 已在，缺的是**适配器的注册/发现**与**中立性强制**（核心不得持有实现细节） | §315 §316 §80 |
| **D** | §247–§251 的落地——画像 / Registry / 候选排序（带 confidence、alternatives、reason；**不用 `overall_score`**；UNPROFILED / QUARANTINED / DISABLED **不自动路由**；成本输入是预算视图） | §247–§251 §27 §84–§86 §248 |
| **F** | 驱动侧工具调用路径——`authorize` → `ToolProvider::invoke`，含**审计行**与失败路径；它把强制点 (1) 接上生产路径 | §4.2 §252 §253 §313 |

---

## 六、复审安排

每份设计由**另一个代理**交叉复审，且复审的任务是**试着推翻**它的断言、并找**漏项**——不是核对其措辞。
本子项目（P3A）的实测：这一做法推翻了作者自扫保留的一条断言，以及作者报告里的两处假证据。

---

## 七、本轮验收

四份设计文档过审 + 各自的实现计划。**本轮不写实现代码**（brainstorming 的硬门：设计未批不落实现）。

---

## 八、本轮的已知缺口（先记在此，免得四份设计各自踩一遍）

- **P3A 的两处「建了但无生产调用方」**：强制点 (1) 的 `authorize`、以及整个 `continuum-secrets`。
  **F 是前者的兑现处，B 是后者的兑现处**——本轮若不把它们接上，这两条会继续挂着。
- **`docs/02-工程.md` §4.3 的层内依赖图里没有 Connector（B）**——图里既没有它、也没有它与 Router /
  Tool Registry 的关系。B 的设计须把这条补进图里（或说明它为何不在该图内）。
- **§334 的阈值**已按 ENG-005 的裁决取 §7 的数值（B3 已闭）；探索预算的**记账**仍归语义层。
