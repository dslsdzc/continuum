# P5a 验证与证据判定 — 设计

**范围**（切分文档 `docs/superpowers/specs/2026-10-08-p5-scope-and-split.md` §五 的 P5a 行的原话，逐字照用）：

> 把节点输出转成 `Evidence`，按每项 REQUIRED 建证据集合并判定覆盖是否完整，按任务取得
> `VerificationProfile`、按 §262 的五级序选 Verifier 并限定它的输入（不含 Worker 的完成声明与完整推理），
> 最后按 §266 判定 Intent 是否可以完成——它是 §342 第 5、10 条不变量的执行点。

**规范依据**：§258 §259 §260 §261 §262 §263 §264 §265 §266 §188–§195 §215 §89 §29；《工程》§8.1 前九行、§8.2–§8.4；《总纲》§8.2。

**本文件只写 P5a。** 其余五块（P5b 执行方法库、P5c/d/e/f 四个领域算子）各有其设计，本文件不替它们定任何形状。凡本设计不得不假设别的块的某枚形状之处，一律在 §14 写成一条**具名的对账条目**，不在正文里就近决定。

**箭头约定**：本文件全篇沿用 §9.1 的读法——`A → B` 读作「A 被 B 依赖（B 依赖 A）」（判据与订正原文见 `docs/02-工程.md:561-567`）。凡引他文的箭头，原样照录并标注。

---

# 1. 范围

## 1.1 建什么

《工程》§8.1（`docs/02-工程.md:481-499`）的**前九行**归本块，逐行落到本设计的一节：

| §8.1 的组件 | 规范依据 | 本设计的落点 | 本轮可达的形态 |
|---|---|---|---|
| Evidence 数据模型 | §258 | §4 | 全量（类型 + 持久化 + 唯一产生点） |
| Requirement Coverage Graph | §260 §190 | §6 | **只判「确定的欠缺」，不判「充分」**（§3） |
| VerificationProfile | §261 §194 | §5.2 | 形状全量；**生成者不建**（A6/OPEN-009，§5.3） |
| Verifier 优先级判定 | §262 | §7.1 §7.2 | 全量（五级序 + 可用集合） |
| Independent Verifier | §263 §29 | §7.3 §7.4 | 全量（隔离判定 + 两个输入面） |
| Verifier Adversary | §264 §215 | §9.1 | **只冻类型与消费规则，无触发**（B1） |
| Mutation Verification | §265 §192 | §9.2 | **只冻插槽，不判**（量纲缺，§9.2） |
| Completion Predicate 判定 | §266 §195 | §8 | **只出「不得完成」的两臂**（§8.2） |
| Test Validity Verifier | §191 | §6.4 | 全量（五检查 + 聚合规则） |

§8.1 的第 10–14 行（执行方法库与四个领域算子）**不在本块**，也不在本文件留桩。

## 1.2 不建什么

以下各条**不建、不留桩**，逐条给出归属或理由：

| 不建的东西 | 归属 / 理由 |
|---|---|
| Execution Method Library 的登记形态 | P5b（切分 §四第 3 条：在 P5b 过审前，任何块不得自造方法登记形态） |
| 代码／研究／媒体／图像四个领域的算子 | P5c/P5d/P5e/P5f |
| `ArtifactType` 的 P5 扩展清单 | P5e（切分 §四第 2 条：只由 P5e 一处改） |
| `Checkpointable` 的错误类型 | P5e（切分 §二第 4 条） |
| `Node.execution_policy` 的判定 | P4（切分 §四第 5 条） |
| `Intent.completion_predicate` 的**求值器** | 本块接下判定（P4 设计 §16 第 9 条把该字段交给第 8 层），但**只到「只存不判」**——理由见 §8.2 与 §15 第 7 条 |
| `Effect` 与 `EffectState` | P2（`crates/continuum-effect` 已建）；本块只取「强制效应是否完成」的**值** |
| `Decision` 对象与恢复链 | P4；本块只把「判不出」输出成可被升级成 Decision 的形状（§7.5、§8.2） |
| 节点状态迁移、`Queued → Running`、`OperatorRegistry::resolve` 的调用点 | 第 3 层（切分 §四第 4 条：P5 不得自建执行路径） |
| 算子的执行 | 第 3 层（同上） |

## 1.3 「模型宣告完成」为什么不是本块的一条接口

§342 第 10 条（`docs/spec/05-normative.md:2680`）要求「完成必须由 Completion Predicate 与 Verification 决定，不由 Worker 模型自行宣布」。本设计的兑现方式不是「检查模型有没有宣告」，而是**让那条宣告没有输入位置**：`InitialVerifierInput`（§7.4）的字段里没有 Worker 的完成声明，`CompletionVerdict`（§8.2）没有「完成」这一臂。两者都是类型层的事，不是运行时判断——理由与照片见 §7.4、§8.2。

---

# 2. 归属与裁定

## 2.1 crate 与依赖边

本块新建 **一个** crate：`continuum-verify`（实测：本仓 `docs/`、`crates/`、`Cargo.toml` 对 `continuum-verify` 零命中，名字未占用）。

依赖边表（**只登记实际用到的**，切分 §四第 6 条）：

| 被依赖 | 用到什么（逐项实测） |
|---|---|
| `continuum-artifact` | `Artifact` / `ArtifactId` / `ArtifactType`：`Evidence.artifact_refs`（§4.1）与 §266 的 `artifact_exists`（§8.3）；内容与隐私不入判定 |
| `continuum-events` | `EventType::VerificationFailed`（`crates/continuum-events/src/event.rs:25`）——本块是它的**第一个产生点**（实测：该型今天**零个写入方**；提到它的只有 `event.rs` 的定义/枚举表与 `crates/continuum-events/tests/event_envelope.rs:18` 的一条往返用例） |
| `continuum-graph` | `NodeId`（生产者与受验节点的身份）、`Node` / `AdfirGraph`（§266 的 `artifact_exists` 与 `EdgeKind::Evidence` 的核对）、`ExecutionProfile`（§7.3 的 Worker 身份） |
| `continuum-operator` | `BackendId`（§7.2 的候选 backend、§4.3 的 `EvidenceProducer`） |
| `continuum-persist` | `Tx` / `Migration` / `PersistError`（§10） |

**不登记**的边，逐条给理由（零使用的边即假边，P2b 为此删过两条）：

- **不登记 `continuum-port`**：本块不构造端口、不做端口兼容判定（§239 的 `compatible` 是 P1 的）。
- **不登记 `continuum-policy` / `continuum-effect` / `continuum-capability`**：§266 的 `mandatory_effects_completed` 以**值**传入（§8.4）。§9.1 的图里 `跨领域 (8)` 只从 `执行层 (3)` 接边（`docs/02-工程.md:573-586`），故这一处不登记边，与 P4 设计 §1.2.3 的规则同形。（该图是否为穷尽列表，本设计不假定；即便它漏了一条边，本条也不受影响——判据是「本块实际用到什么」。）
- **不登记 `continuum-model-registry` / `continuum-connector` / `continuum-provider`**：§262 第 3、4 级的可用性由装配方以**值**传入（§7.2），本块不查注册表。

**共写文件**（切分 §一 的共写文件表）：

| 文件 | 本块的改动 | 说明 |
|---|---|---|
| `Cargo.toml` 的 `[workspace] members` | 加一行 `crates/continuum-verify` | 切分建议由先落地者一次加齐六行；本块**只加自己这一行**，六行齐否由协调者定 |
| `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED`（`:25`） | 加 `("continuum-verify", &[…])` | 新 crate 会先让该门变红再被补齐（`:288` 的用例），这是刻意的 |
| `crates/continuum-runtime/src/main.rs` 的 `runtime_migrations()`（`:98`） | 加一行注册 `p5a_verify_migrations()` | 「谁的表谁注册」 |
| `crates/continuum-graph/src/node.rs` | **只改 `:19` 的注记一行**，不改字段类型 | 理由与替代方案见 §2.3 |

## 2.2 冻结清单：本块持有的类型与判定

**类型**（全部定义在 `continuum-verify`，例外见 §2.3）：

```
EvidenceId  EvidenceType  EvidenceSubject  Claim  EvidenceProducer
EvidenceStrength  EvidenceScope  Evidence  Validity  TestValidityCheck
CheckVerdict  TestValidity
VerificationPolicy  VerificationProfile  CoverageVerdict  IncompleteReason  RequirementCoverage
VerifierLevel  AvailableVerifier  VerifierSelection
VerifierVerdict  VerdictAggregate  IndependenceVerdict
BlindVerdict  WorkerCompletionDeclaration  InitialVerifierInput  PostBlindVerifierInput
Conjunct  CompletionVerdict  CompletionInput  AdversaryOutcome  VerifyError
```

**这份清单是本块的定义面，不是「本块用到的类型」的清单**：判定的输入与输出形状若由本块定义，就登记在此——`VerificationPolicy`（§5.1 定义、§7.2 消费）、`IndependenceVerdict`（§7.5 定义、§7.3 与 §8.4 消费）、`CompletionInput`（§8.4 定义、§12.1 (q) 用）三枚因此在此，**它们是另外四块照本清单接线的合同的一部分**。

**一枚早先列在这里的死条目已删**：`VerdictSet`。实测全文除本清单外**零命中**——没有定义、没有一处判定面用它；本块做聚合的是 `VerdictAggregate`（§7.5、§8.3 第 5 项、§8.4）。此处留一句，是为免下一轮再照旧清单补一枚同名类型。

**判定**（本块是它们的唯一落点）：

1. 工具结果 → `Evidence` 的唯一产生点（§89，§4.4）。
2. `Node.verification_policy` 的 `Value` → `VerificationPolicy` 的严格解析（§2.3，§5.1）。
3. 每项 REQUIRED 的覆盖判定（§260 §190，§6）。
4. 测试证据的有效性判定（§191，§6.4）。
5. Verifier 的级别选择（§262）与独立性判定（§263，§7）。
6. Completion Predicate 的判定（§266 §195，§8）。

**P5c/P5d/P5e/P5f 只产出、只消费**（切分 §四第 1 条）：四块**不得**各自定义证据类型、各自的「充分」判据、或各自的完成判定。本设计为此提供的接口只有两个方向：产出走 `Evidence::from_tool_result`（§4.4），消费走 §6/§8 的判定返回值。

## 2.3 `Node.verification_policy` 的定型：**裁定取「字段保持 `Value`，定型在 P5a 侧的严格解析」**

事实（逐项实测）：

- `crates/continuum-graph/src/node.rs:20` 的字段是 `pub verification_policy: Value`，`:19` 的注记是「结构保存，判定属 P5。」；`Node::new`（`:34`）把它置为 `Value::Null`；落库经 serde_json 序列化成 `adfir_node.verification_policy TEXT NOT NULL`（`crates/continuum-graph/src/persist.rs:31`、`:102`、`:251`）。
- 切分 §二第 2 条要求「P5a 冻结它的类型，形状由 P5a 的设计定」；切分 §一 把该文件登记为「共写」。

**裁定**：`verification_policy` 的**字段类型不动**（仍是 `serde_json::Value`）；本块提供 `VerificationPolicy` 类型（§5.1）与 `VerificationPolicy::from_value(&Value) -> Result<VerificationPolicy, VerifyError>` 这一个产生点。`node.rs:19` 的注记补一句「判定在 `continuum-verify` 的 `VerificationPolicy` 上」。

**理由（两条，按分量）**：

1. **定型在字段上会造出一条反向边。** `VerificationPolicy` 若定义在 `continuum-verify`（第 8 层），则 `continuum-graph` 要 `use` 它，即 `跨领域 (8) ← 执行层 (3)`——而 §9.1 给的是 `执行层 (3) → 跨领域 (8)`（`docs/02-工程.md:573-586` 的图；`:588` 明写「依赖方向单向，无环」：跨领域依赖执行层）。两处并存即第 3 层与第 8 层之间的 2-环，与 §9.1 的「依赖方向单向，无环」直接相抵。若把 `VerificationPolicy` 连同它要用的 `EvidenceType` / `VerifierLevel` 一并下放到执行层，则 §二第 1 条「`Evidence` 的持有方是 P5a」当场不成立——**两个方向的代价都落在同一条规则上，而那条规则是规范给的**。**这一侧的代价据实说小**：定型落消费侧之后，本块只有**一个**解析点（§5.1 的 `from_value`），§7.2 的 `select_verifier` 收的是解析后的 `&VerificationPolicy`，故「N 个消费者各解析一遍」并不成立；真正的消费者是**装配方**（写这个 `Value` 的人），而今天**没有任何产生点写它**（`Node::new` 置 `Value::Null`，`crates/continuum-graph/src/node.rs:34`）。代价落在「解析必须收敛在一处、且非空不认识的策略取 `Err`」（§5.1 的两侧），不是分散在各处的解析成本。
2. **同一份文件、相邻一行已有先例。** `execution_policy`（`:18`，注记 `:17`「判定属 P4」）在 P4 的设计里正是这么处置的：P4 设计 §1.2.4 的表第三行明写「图的 `Node.execution_policy` 与 `Node.constraints` 以 `&str` / `serde_json::Value` 原样传入，本层不 `use continuum_graph`」，并给出「本层不登记任何指向执行层／资源层／边界层的 `Cargo.toml` 依赖边；凡跨层的输入，一律以『值』由装配方传入」（P4 设计 §1.2.3，`:99-104`）。两个相邻字段取两种形状，会让「同一件事两个词汇表」以**同一份文件的相邻两行**的形式出现。

**替代方案与被否的理由（留档）**：把字段改成 `VerificationPolicy` 并让它定义在 `continuum-graph`（即类型随字段下放到执行层）。这条**能编过**、也不造反向边，代价是 §262 的五级序与 §258 的证据类型这两套验证词汇的**权威**从跨领域层移到执行层，而 §236（`docs/spec/05-normative.md:558`）对本字段只有字段名、没有词汇表——把词汇表放在执行层，等于让第 3 层替第 8 层决定了它要判什么。**本设计不取这条**，但把它写在此处：若协调者要取它，改动是「本节的裁定 + §5.1 的位置 + §7.1 的 `VerifierLevel` 位置」三处，判定的归属不变。

**这一行改动的归属是唯一的**：它与 §2.1 的共写文件表第 4 行（`crates/continuum-graph/src/node.rs`，「**只改 `:19` 的注记一行**，不改字段类型」）是**同一条**，也与 §14 第 6 条 (i) 互为指针；本块需读 `Node`，本来就依赖 `continuum-graph`，故这一行由**本块**改，**不再单列一条对 P1 的请求**。要补的内容是：`Node.verification_policy` 与 `Node.constraints` 一样，今天在 `node.rs` 上没有说「谁是它的读者」，故 `:19` 的注记里一并写出读者的名字。这是一行文档改动，不涉及类型。

## 2.4 本块的层间位置

§9.1（`docs/02-工程.md:561-575`）里与本块有关的两条，按「被依赖者 → 依赖者」读：

- `执行层 (3) → 跨领域 (8)`：本节 §2.1 的边表全是它的实例（`continuum-verify` 依赖 `continuum-graph` / `continuum-artifact` / `continuum-operator`）。
- `跨领域 (8) → 长期循环 (7)`：§232 的 `ProductReadiness` 要存 `value + evidence + last_verified`（切分 §三）。本块为此提供的是 `EvidenceId` 与只追加的证据表（§10）——**不提供** ProductReadiness 本身（属长期循环）。

§9.4（`docs/02-工程.md:627-638`）把 `执行层 → 跨领域　Evidence 数据模型` 列为载荷较重的跨层接口：本设计冻结 `Evidence` 的形状时，受影响的是执行层（产出方）与长期循环（消费 `last_verified`）两侧。§10 的表结构与 §4.1 的字段表就是这条代价的具体形态。

---

# 3. OPEN-001 / ENG-004 的显式处置

切分 §八第 1 条要求本设计**显式处置**「证据集合满足什么条件才算充分」没有判据这一缺口，「不得绕开」。本节是那处处置。

## 3.1 三个可选方向各自缺的那一步（**构造性论证**）

`ENG-004`（`docs/02-工程.md:671`）与 `OPEN-001`（`docs/01-总纲.md:1782`）给出同一组三个方向。**逐条问「它落地时需要的最后一件规范没给的东西是什么」**：

| 方向 | 落地所需的最后一步 | 规范里有没有 |
|---|---|---|
| a) 由 Contract 编译时显式写出每条 REQUIRED 的判据 | 判据的**语法**——即 `Requirement.verification_requirement`（§224，`docs/spec/05-normative.md:209-254`）与 `expression` 怎么读、怎么与证据比对 | **没有**。§224 只给字段名；P4 的设计已把 `expression` 取 `Opaque` 且明写不解析（P4 设计 §7.1），`verification_requirement` 是一个只有 `as_str` / `from_str` 的 newtype（P4 设计 §7.2，`:1018-1030`） |
| b) 模型判定，但用确定性检查限制可声明的证据类型 | 那张**限制表**——哪些 Requirement 上哪些证据类型是**不可声明**的 | **没有**。§258 给的是证据类型的枚举，没有任何一处把「证据类型」与「Requirement 的形态」连起来 |
| c) 不判充分，只判「未发现反例」，把 confidence 作为一等输出 | 「未发现反例」是**可判**的（见 §3.2）；缺的是 `confidence` 的**取值域**——§122 只给了一个 `confidence: high` 的例（`docs/spec/02-positioning.md:1752-1773`），`high` 之外还有什么、`high` 与别的档怎么比，规范未给 | **半有**：判定面可落地，一等输出落不下来 |

**这一节是构造性的，不是实测的**：上表的每一行断的是「按该方向落地时必然要写出某个规范未给之物」，其依据是规范逐条的字段名清单，不是「我没找到判据」。行文上与本仓的处置习惯一致：**缺的是哪一步**要写到步骤，不用「后续」「长期阶段」兜。

**结论**：三个方向都**不能在「不发明规范未给之物」的前提下**落地。故本设计**不选任何一个方向**，也**不宣布关闭 OPEN-001**——本块没有那个权限。

## 3.2 本设计取的判定面：只判「确定的欠缺」

本设计把覆盖判定收缩到**规范能判的那部分**，并让「充分」在类型上写不出来：

- **判**：§190 的「没有有效证据」与 §261 的「必需证据类型缺项」——两者都是**确定的欠缺**，判据不依赖充分性（§6.2）。
- **不判**：某条 REQUIRED 的一堆有效证据**合起来是否算满足**。§259（`docs/spec/05-normative.md:1107`）与 §342 第 5 条禁止把测试通过当作需求满足；而「满足」的判据就是 OPEN-001 缺的那一件。
- **在类型上**：覆盖判定的枚举**没有「满足」这一臂**（§6.2）。这不是「实现了但没接线」，是「该结论不可表达」。

## 3.3 后果（逐条写清，不掩）

1. **§342 第 5、10 条在本轮是「只禁止、不放行」的形态。** 第 5 条（测试通过不等价于需求满足）由「覆盖判定无正臂」构成；第 10 条（完成由 Predicate 与 Verification 决定）由「`CompletionVerdict` 无『完成』臂」构成（§8.2）。两条的**禁止面**成立且可拍照片；**放行面**不存在。这与《工程》§8.5 的判断一致——该节明写「A1 阻断本层的收口……该问题解决前，Completion Predicate 无法被可靠判定，§342 第 5、10 条不变量没有支撑」（`docs/02-工程.md:540-556`）。
2. **`§344` 的闭环末端仍然可达。** §344（`docs/spec/05-normative.md:2750`）的链条末端是 `Result`，不是 `COMPLETED`：本设计可产出 `Result`（含逐项合取项的状态），见 §8.4。
3. **Intent 迁不到 `COMPLETED`。** §223（`docs/spec/05-normative.md:172`）的状态集里有 `COMPLETED`，而本设计不产出许可它的判定。执行器在 `VERIFYING` 上要么升 `Decision`（§7.5、§8.2），要么继续验证。**这条的落点在第 3 层**，不在本块（切分 §四第 4 条）。
4. **《工程》§8.4 第一行（`:530-538`）的正面从句本轮为空。** 原文「测试通过不进入 Completed，除非 Requirement Coverage 判定充分」——「除非」的那一支不存在，故该行实际是「测试通过不进入 Completed」。这是收紧，不是放宽，故不触 §342 第 1 条。

## 3.4 翻案条件（**本裁定有失效条件，不是「它永远对」**）

OPEN-001 一旦有判据，本设计的翻案是**三处**，且三处都在类型层、编译期可见：

1. `CoverageVerdict` 加一枚正臂（§6.2）；
2. `CompletionVerdict` 加一枚「完成」臂（§8.2）；
3. `Conjunct::AllRequiredRequirementsVerified` 的判定函数从「恒 `Undetermined`」改为读新判据。

**在那之前，任何一处出现「先按某个阈值放行」的实现，都是在本块内绕过 §342 第 5、10 条**——上表三处就是它的检查点。

## 3.5 一处规范正文的相抵：§189 直接要求「判断证据是否充分」

**据实记录**：§189（`docs/spec/04-method.md:210`「Test Evidence」）的四阶段表里，`Verification` 一行逐字是「判断证据是否充分」（`:242-243`）。这一句正落在本设计 §3.2 裁定**不判**的那件事上，而 §189 出现在本设计自己的「规范依据」行（`:9` 的 `§188–§195`）所覆盖的区间内——**它是规范正文里直接要求判充分的那一句**，比 `ENG-004` / `OPEN-001`（两处都只是登记缺口）更直接。

本设计**不执行该句**，理由即 §3.1 的构造性论证（OPEN-001 缺「充分」的判据）。**这与 §13 末记录的那处相抵同形**（§8.3 的依赖行缺 Contract，与 §263/§8.2 相抵）：都是「规范正文的一句与本设计的裁定相抵」，两处都**不改规范、也不假装没看见**，各记一条。§3.1 只列了 `ENG-004` / `OPEN-001` / §259 / §342 第 5 条，漏了这一句；本段补上。

**另一种读法（据实列出）**：§189 的 `Verification → 判断证据是否充分` 也可以读作**对四个阶段职责的描述**，而不是**对本块的一条 MUST**——该表通篇没有 MUST/SHOULD。若规范维护者取这一读法，本条相抵即消解；本设计不替它取。**收件人：规范维护者 + 协调者**（§15 第 17 条）。

---

# 4. `Evidence` 数据模型（§258）

§258（`docs/spec/05-normative.md:1073`）给的字段名：`id` / `type` / `subject` / `claim` / `producer` / `artifact_refs[]` / `strength` / `scope`。

## 4.1 逐字段：照录、推导、自定

| §258 字段 | 本设计的类型 | 来历 |
|---|---|---|
| `id` | `EvidenceId`（`String` newtype） | 照录 |
| `type` | `evidence_type: EvidenceType`（封闭枚举，§4.2） | 照录；字段名不叫 `type`（Rust 保留字），与 `Artifact.artifact_type`（`crates/continuum-artifact/src/artifact.rs:172`）同形 |
| `subject` | `subject: EvidenceSubject`（两臂，§4.3） | **本设计定的读法**，理由见 §4.3 |
| `claim` | `claim: Claim`（`String` newtype） | 照录。**不解析**：§258 没给 `claim` 的语法，本块不发明（与 P4 对 `expression` 的处置同形） |
| `producer` | `producer: EvidenceProducer`（两臂，§4.3） | **本设计定的读法**：§263 的隔离判定要能命名「这条证据是谁产的」，否则那条 MUST 没有判据 |
| `artifact_refs[]` | `artifact_refs: Vec<ArtifactId>` | 照录；元素取 P1 的 `ArtifactId`，不另造 |
| `strength` | `strength: EvidenceStrength`（§4.3） | 照录字段名；**结构性保存，不判** |
| `scope` | `scope: EvidenceScope`（§4.3） | 照录字段名；**结构性保存，不判** |

**证据记录只追加。** `Evidence` 一经写入不改：§241（`docs/spec/05-normative.md:681`）的不可变原则正文说的是 Artifact，但同一条纪律用在这里的理由是具体的——**有效性判定**（§6.4）与**证据**必须是两条记录，否则「验证者改了被验之物」。故 `Evidence` 无更新路径（§10 的表只有 INSERT）。

## 4.2 `EvidenceType`：十一臂，逐臂出处

§258 的九个例（原文前缀是「Evidence 类型**例如**」）**照录九臂**；§193（`docs/spec/04-method.md:361`）另举六种验证方式，其中两种不在 §258 的表里，**补入二臂**：

| 臂 | 出处 | 备注 |
|---|---|---|
| `Test` | §258 的 `TEST` | §259 的 `Evidence<Test>` 就是它 |
| `Fuzz` | §258 的 `FUZZ` | 与 §193 的 Fuzzing 同名同物 |
| `Property` | §258 的 `PROPERTY` | 与 §193 的 Property-based Testing 同名同物 |
| `FormalProof` | §258 的 `FORMAL_PROOF` | 与 §193 的 Formal Proof 同名同物 |
| `StaticAnalysis` | §258 的 `STATIC_ANALYSIS` | 与 §193 的 Static Verification 同名同物 |
| `Benchmark` | §258 的 `BENCHMARK` | §194 的「大型重构」一例含 benchmark |
| `VisualCheck` | §258 的 `VISUAL_CHECK` | §326/§168 的 mask 外像素判定是它的实例（那一步的宿主见 §14 第 3 条） |
| `MetadataCheck` | §258 的 `METADATA_CHECK` | §29 的「读 resolution metadata」是它的实例 |
| `HumanConfirmation` | §258 的 `HUMAN_CONFIRMATION` | |
| `Differential` | §193 的 Differential Testing | **§258 未列** |
| `Metamorphic` | §193 的 Metamorphic Testing | **§258 未列** |

**为什么是封闭枚举，而不是开放字符串**：`VerificationProfile.required_evidence_types[]`（§261）与产出的证据之间要做**集合包含**判定（§6.2）。若类型是字符串，「必需类型齐了吗」就退化成字符串比较——两边各写各的拼法即静默漏判，而且漏判的方向是「以为齐了」。封闭枚举把这条判定变成编译期穷尽的 `match`。

**为什么是这十一臂、不是九臂、也不是更多**：取「§258 的九臂 ＋ §193 里 §258 未列的两臂」= **§258 与 §193 两处提到过的证据产物的并集**，这是「不发明」口径下能确定的最大集合；**`§191` 的五个检查与 `§265` 的 mutation 不进本枚举**，理由：规范把这两者写成对**已有**证据的作用（前者判测试证据是否有效、后者说「test evidence strength ↓」），不是新的证据。**「只有这十一臂」这句话本设计不写**：§258 的九臂前缀是「例如」，§193 的六种方式之外规范没有穷尽声明，故此处只能说「已实测的出处有上表十一行」。**P5d（CitationVerification，§330）与 P5f（§326 的 mask 外像素判定）是否需要各自的臂，见 §14 第 2、3 条**——若需要，由它们向本块报出，本块一次改动落地（同 `ArtifactType`／P5e 的约定）。

`EvidenceType` 与 `ArtifactType` 一样带 `ALL` 常量与 `as_str` / `parse`，`ALL` 的完整性由「数目断言 + 两个穷尽 `match`」把关（与 `crates/continuum-artifact/src/artifact.rs:28-35`、`:47`、`:68` 同形）。

## 4.3 三处需要解释的形状

### 4.3.1 `EvidenceSubject`：两臂，**不是** artifact

```
pub enum EvidenceSubject {
    /// 该证据**为**某项 Requirement 提出。§260 的 `Requirement → Evidence Set` 靠它成立。
    Requirement(RequirementId),
    /// 该证据不针对任何 Requirement。
    Unattached,
}
```

**读法的理由（三条）**：

1. **不与 `artifact_refs[]` 重复。** §258 已经把制品放在 `artifact_refs[]` 里，故 `subject` 若也指制品，两者语义重叠、且哪个权威无判据。
2. **§260 需要它，而语义上只有 `subject` 是那个位置。** 建 `Requirement → Evidence Set` 必须有「这条证据属于哪条 REQUIRED」；§258 的八个字段里，`artifact_refs` 已归制品、`type` / `strength` / `scope` / `producer` 都是对证据自身的描述、`claim` 是断言的文本，**只有 `subject` 的语义是「关于什么」**。（这一条是**读法**，不是 §258 的明文：`claim` 里写一个 Requirement id 在语法上也塞得下。本设计否掉那种塞法的理由是不体面的——它把归属藏进自由文本，而归属判定要的是可比较的值。）
3. **必须是强类型。** 若取 `String`，归属判定退化成字符串比较，且**失配的方向是静默漏配**——一条证据谁都不要，覆盖判定就看到一个空集合，把「有证据但没挂上」误判成「没证据」。两臂都在类型里，`Unattached` 是显式的。

**`RequirementId` 的来源**：§224 的 `Requirement.id`，属 P4 的 `continuum-semantics`。本设计**不**定义第二个 `RequirementId`（本仓对「同一个概念两个类型」一贯判为 Critical，P4 设计 §13.2 有 `ContractId` 的前例）。处置见 §14 第 1 条：本块以**值**承载它（P4 的规则），落地时以 P4 的 `RequirementId` 为准。

**为什么需要 `Unattached`**：§259 明写测试通过只产生 `Evidence<Test>`、不产生 `requirement_satisfied`；§89 说工具结果必须先转成 Evidence。两者合起来意味着**证据先于归属存在**：工具节点产出一条证据时，它可能还不知道（或不该由它决定）面向哪条 Requirement。给这个状态一个臂，是为了让它**看得见**而不是被硬塞进某条 Requirement。

### 4.3.2 `Claim` 与 `WorkerCompletionDeclaration` 的分界

```
/// §258 的 `claim`。**不解析**：规范未给语法。
pub struct Claim(String);

/// §29 的「Worker reasoning summary / Worker 自己的完成声明」。
/// **它不是一个 `Evidence`**，且**没有任何到 `Evidence` 的转换**：
/// §263 禁止它进入初判输入，若它能变成 Evidence，那条禁止就没有落点。
pub struct WorkerCompletionDeclaration(String);
```

`WorkerCompletionDeclaration` **可以**被构造（§29 的「之后才比较」需要它），但它的唯一消费者是 `PostBlindVerifierInput`（§7.4）。

**一条据实记录的残余**：`Claim` 是自由文本，故它**是**一个能把自然语言塞进初判输入的口子。结构性保证只能做到「`Evidence` 没有 reasoning 字段」，做不到「`claim` 里没人塞 reasoning」。**这是缺口，不是已解决的问题**，见 §15 第 2 条。

### 4.3.3 `EvidenceProducer`：两臂

```
pub enum EvidenceProducer {
    /// 由某个算子节点产出（§89：工具结果转成 Evidence）。
    Node { node: NodeId, backend: BackendId },
    /// 由人在环中给出（§258 的 `HUMAN_CONFIRMATION`）。
    Human,
}
```

`backend` 取 §245 的 `BackendId`（`crates/continuum-operator`）——§262 第 3、4 级的可用性、§120 的「不同 family / 不同证据路径」都要能命名「哪一条路径产出了这条证据」。

**这里没有 `Worker` 这一臂**，是刻意的：Worker 的完成声明**不是证据**（§263），把它做成 `EvidenceProducer` 的一个臂，等于承认它可以是一条 `Evidence`。它以独立的类型存在（§4.3.2），不在本枚举内。

### 4.3.4 `EvidenceStrength` 与 `EvidenceScope`：结构性保存

```
/// §258 的 `strength`。**规范只给字段名**：取值域与比较规则都没有。
/// 只有 `as_value` / `from_value`，**没有** `Ord`、没有阈值、没有命名档。
pub struct EvidenceStrength(serde_json::Value);

/// §258 的 `scope`。同上。
pub struct EvidenceScope(serde_json::Value);
```

**`strength` 为什么不给判据**：§265（`docs/spec/05-normative.md:1229`）说「大量 mutation 无法被测试捕获 ⇒ test evidence strength ↓」。这是一条**方向**陈述，而**等级集**（能降到哪一档、档与档之间怎么比）规范未给。方向可以写进文档，但**下降**在类型上需要 `Ord`；给一个规范没给的 `Ord`，就是发明。故：方向**照录在此**，判定**不落**。

**`scope` 为什么不给判据**：`scope` 的语义若与覆盖有关（「这条证据覆盖了 Requirement 的哪一部分」），那么「各部分合起来是否盖满」正是 OPEN-001 缺的那件东西（§3）。故它同样只存。

**两者的消费者**：本轮**没有**。这不是遗漏：**「谁读它」取决于 OPEN-001 怎么解**（`strength` 是充分性判据的输入，`scope` 是组合规则的输入）。这条与 §15 第 1、3 条同源。

## 4.4 产生点：§89 的「工具结果必须先转成 Evidence」

**唯一产生点**。「唯一」在这里是**结构事实**，不是承诺：`Evidence` 的八个字段全部私有、无 `pub` 字段、无 `Default`、无 `From`/`FromStr`、**无第二构造点**。故「绕过 `from_tool_result` 造一条证据」在类型上写不出来。**照片**：`trybuild` 的 `compile_fail` 用例写 `Evidence { .. }`（字段私有）与 `Evidence::default()`（无实现），两条都编不过。

```
impl Evidence {
    /// §89：工具结果转成 `Evidence` 的**唯一**构造点。
    pub fn from_tool_result(
        id: EvidenceId,
        evidence_type: EvidenceType,
        subject: EvidenceSubject,
        claim: Claim,
        producer: EvidenceProducer,
        artifact_refs: Vec<ArtifactId>,
        strength: EvidenceStrength,
        scope: EvidenceScope,
    ) -> Result<Evidence, VerifyError>;
}
```

**它拒绝什么**（逐条给判据）：

| 拒绝条件 | 判据 |
|---|---|
| `evidence_type` ∈ {`MetadataCheck`, `Benchmark`, `Test`, `Fuzz`, `Property`, `FormalProof`, `StaticAnalysis`, `Differential`, `Metamorphic`, `VisualCheck`} 且 `artifact_refs` 为空 | 这十型的判定都要读某个制品（§29 的 4K 例子读的正是 resolution metadata）。空引用意味着判定方没有可读之物，而**判定方「无物可读」必须走到 `Unknown`，不能走到 `Pass`**（§7.5）。把这一条放在构造点，是为了不让「无物可读」悄悄变成一次空判。**例外是 `HumanConfirmation`**（§258 的 `HUMAN_CONFIRMATION`）：人的确认不需要制品，故不受此条约束——上表逐型列出，就是为了让「哪几型需要制品」这件事有一个穷尽的落点 |
| `evidence_type = HumanConfirmation` 且 `producer != Human` | §258 的类型名与生产者不匹配；放过去会让「谁确认的」不可判 |

上表的第一条**对 `evidence_type` 做穷尽 `match`**（无通配臂）：十一臂里哪几型需要制品、哪一型不需要，是逐臂写死的，因此 `EvidenceType` 加一臂时这条规则**编译期**被强制重新决定——不会出现「新臂悄悄落进『不需要制品』那一支」。

**它不检查什么（据实列出）**：`claim` 的内容、`strength` 的值、`scope` 的值、`artifact_refs` 里制品是否存在（那要查库，是 `load_artifact` 的事，见 §10）、生产者的节点是否真属该图。最后一条在 §7.4 的输入装配处检查（那里才有「受验节点」这个上下文）。

---

# 5. `VerificationPolicy` 与 `VerificationProfile`

## 5.1 §236 的节点级 `VerificationPolicy`

§236（`docs/spec/05-normative.md:558`）只给字段名，没有词汇表。本设计取：

```
/// §236 的 `verification_policy`。**不是** §261 的 `VerificationProfile`：
/// 后者按**任务**，本型按**节点**（§236 的字段挂在 Node 上）。
pub struct VerificationPolicy {
    /// §262 的五级序里的**最低可接受级**。`None` = 不限，按 §262 的顺序取最高可用级。
    pub min_verifier_level: Option<VerifierLevel>,
    /// 本节点**额外**要求产出的证据类型（任务级 `VerificationProfile` 之外的要求）。
    pub required_evidence_types: Vec<EvidenceType>,
}
```

**为什么是这两个字段**：两个字段都直接进 §7.2 与 §6.2 的判定，无一是「为将来可能用到」——切分 §四第 6 条要求依赖边只登记实际用到的，同一条口径用在字段上就是这条。

**`from_value` 的两侧都要钉**（这是本设计里一处容易做成 fail-open 的地方）：

- `Value::Null`（`Node::new` 的初值，`crates/continuum-graph/src/node.rs:34`）⇒ `Ok(VerificationPolicy::default())`（无限定）。**这不是「无要求」**：任务级 `VerificationProfile` 仍然适用，故不是放宽。
- **非空但不认识**（缺字段、多出未定义字段、`min_verifier_level` 不是五级序里的值、`required_evidence_types` 含未知类型名）⇒ **`Err(VerifyError::UnknownVerificationPolicy)`**。取 `Err` 而不是取默认：一个写着内容的策略被当成空策略，正是「静默放宽」。这一条与 `ArtifactType::parse` 对未知串返回 `None` 同形（`crates/continuum-artifact/src/artifact.rs:58-67` 的注释把这条例由写全了）。

## 5.2 §261 的任务级 `VerificationProfile`

§261（`docs/spec/05-normative.md:1152`）的六个字段，逐条处置：

| §261 字段 | 本设计的类型 | 处置 |
|---|---|---|
| `required_evidence_types[]` | `Vec<EvidenceType>` | **进判定**（§6.2 的缺项判据） |
| `optional_evidence_types[]` | `Vec<EvidenceType>` | **只存不判**。§261 未说 optional 有什么用；本块不给它派活（派活就是发明）。收件人见 §15 第 5 条 |
| `fuzz_budget?` | `Option<serde_json::Value>` | **只存不判**：量纲未给（§261 只给字段名，§193 也没说预算记的是什么）。故 §8.3 的 `ApplicableFuzzBudgetComplete` 本轮不可判 |
| `mutation_threshold?` | `Option<serde_json::Value>` | **只存不判**：§192 的 `mutation_score` 量纲未给（`docs/spec/04-method.md:354` 只说「可以维护 mutation_score」），故阈值无从比较 |
| `property_requirements[]` | `Vec<serde_json::Value>` | **只存不判**：§261 未给语法，§193 只列了方式的种类 |
| `independent_verifier_required` | `bool` | **进判定**（§7.2） |

`required_evidence_types` 与 `optional_evidence_types` **不取 `Vec<String>`**：理由同 §4.2（集合包含判定要编译期穷尽）。

## 5.3 取得（§194 §261 §8.3）：**只定义取得点，不建生成器**

事实：§194（`docs/spec/04-method.md:397`）说「不同任务**自动生成** VerificationProfile」，且给的是四行**例子**（简单文案修改 → basic verifier；parser 修改 → unit+integration+property+fuzz+mutation；大型重构 → …；安全关键代码 → …）——**是例，不是规则**。《工程》§8.2 明写「该组件的生成者未定义（A6）」，§8.3 的依赖行写 `VerificationProfile ← Contract（生成者待定）`。`A6`（`docs/01-总纲.md:1650`）与 `OPEN-009`（`:1865`）把「生成者的可信度」列为**可推迟**的未决项，三个方向（固定模板集 / 自动生成 + Adversary / 纳入 Contract 由用户确认）**一个都没选**。

**本设计的处置**：

1. **不建生成器。** 生成者待定期间造一个，等于替 `OPEN-009` 选了一个方向。
2. **只冻结取得点**：本块的判定函数收一个 `&VerificationProfile`，由装配方以**值**传入（P4 设计 §1.2.3 的规则）。
3. **缺 Profile 是错，不是默认**：

   ```
   /// 取得该任务的验证档案。
   /// **没有档案时返回 `Err(ProfileMissing)`，不返回「无要求」的档案**——
   /// 后者会让每项 REQUIRED 的 required 集合为空、覆盖判定无缺项，
   /// 即「无档案 ⇒ 无要求 ⇒ 不阻断」——这正是 fail-open 的那一侧。
   pub fn profile_for(task: &VerificationProfileSource) -> Result<VerificationProfile, VerifyError>;
   ```

   `VerificationProfileSource` 的形态（来自 Contract 的哪一字段？还是另一个输入）**本轮不决定**——它取决于 `OPEN-009` 怎么解。见 §14 第 1 条。

---

# 6. Requirement Coverage（§260 §190 §261 §191）

## 6.1 迭代域：Contract 里 `class = REQUIRED` 的**全部**项

§260（`docs/spec/05-normative.md:1125`）说「每项 REQUIRED SHOULD 建立 Requirement → Evidence Set」；§190（`docs/spec/04-method.md:251`）说「每一项 REQUIRED 都必须有对应证据」。§224 的 `class` 四值里只有 `REQUIRED` 受此约束（§225 的不可越权原则判的也是 `class`，P4 设计 §7.1 已记）。

**判定函数的形状**：

```
pub fn coverage(contract: &ContractView, evidence: &[Evidence], validity: &[TestValidity],
                profile: &VerificationProfile) -> Result<Vec<RequirementCoverage>, VerifyError>;
```

- **输入是 `ContractView`，不是 `TaskContract`**：本块对 Contract 只读 `requirements[].{id, class}` 与 §7.2 的 `verification_requirement`（只读不判），故取一个**视图**，其字段以值传入。理由同 §2.1：不登记到 `continuum-semantics` 的边，也不造第二个 `TaskContract`。
- **返回值逐项一条**，与 REQUIRED 项的**数目相同**。这是 §342 第 1 条（用户 REQUIRED 不得被静默降低）在本块唯一的可执行守卫：**漏掉一项即数目不符**。照片见 §12.1 (k)。

**`class` 取封闭四值**（P4 已定；四种取值各要一条用例，见 §12.1）；本块只做 `match`，不在本块里判 `class` 的语义。

## 6.2 判定枚举：只有两臂，**没有正臂**

```
pub struct RequirementCoverage {
    pub requirement: RequirementId,
    pub verdict: CoverageVerdict,
}

pub enum CoverageVerdict {
    /// **确定的欠缺**。§260 的 `verification = INCOMPLETE` 落在这一臂。
    Incomplete { reason: IncompleteReason },
    /// **没有确定的欠缺，但「充分」无判据**（OPEN-001，§3）。
    /// 这一臂**不是**「满足」，也不是「不确定的满足」——它是「本块不放行」。
    NotEstablished,
}

pub enum IncompleteReason {
    /// §190：该 Requirement 没有有效证据（集合为空，或全部被判 `Invalid`）。
    NoValidEvidence,
    /// §261：`required_evidence_types` 里有类型在本 Requirement 的证据集合里缺席。
    MissingRequiredTypes { missing: Vec<EvidenceType> },
}
```

**为什么没有第三臂（`Satisfied`）**：§3.2。**`NotEstablished` 这个名字是刻意的**：它不叫 `Unknown`，因为「未知」听起来像信息不足；它说的是**本层没有判据、因而不放行**。§123（`docs/spec/02-positioning.md:1778`）允许正式表达 `UNKNOWN`，但这个枚举的语义比 `UNKNOWN` 更强：它连「可能满足」都不声明。

**两臂的严重度有序**（`Incomplete` 严于 `NotEstablished`），且**两臂都不得完成**——这条使 §6.3 的计数规则即使取错，代价也只落在「哪一种不放行」上，不落在「放不放行」上。**这是本设计的一处刻意余量，写出来是为了让复审不必为它重建论证。**

## 6.3 计数规则（§190 的「有效证据」）

`counts(e, r)` 的判据，逐条（第 1 条是前提，2–5 条互斥且穷尽 `TestValidity` 的四种取值形态：无记录 / `Valid` / `Invalid` / `Unknown`）：

1. `e.subject == EvidenceSubject::Requirement(r)`；否则不计数（`Unattached` 的证据不属于任何 Requirement）。
2. `e.evidence_type == Test` 且**没有**对应的 `TestValidity` 记录 ⇒ **不计数**。判据是 §191（`docs/spec/04-method.md:298`）的原文：「测试不能因为存在于 `test/` 目录里就自动被认为有效」。
3. `e.evidence_type == Test` 且 `TestValidity.verdict == Invalid` ⇒ 不计数（§190 的「有效证据」里的「有效」由 §191 给）。
4. `e.evidence_type == Test` 且 `TestValidity.verdict ∈ {Valid, Unknown}` ⇒ 计数。`Unknown` 也计数：§123 的正式未知是「有判据但判不出」，与「判为无效」是两件事；不计数会把它变成**确定的欠缺**，那是把判不出说成判为假。
5. `e.evidence_type != Test` ⇒ 计数。§191 的判据只覆盖测试证据，其「不能自动被认为有效」那句话的主语也是测试，故非测试证据**没有**「不计数」的判据；规范未给，本设计不发明。

然后：`NoValidEvidence` ⟺ 该 Requirement 计数的证据为空集。`MissingRequiredTypes` ⟺ `profile.required_evidence_types` 的某一型在该 Requirement 的**计数**证据里不出现。

**翻转条件**：这套规则今天只决定 `Incomplete` 与 `NotEstablished` 的分界；一旦 `CoverageVerdict` 加了正臂（§3.4），**第 2 与第 4 条立即变成载荷规则**（它们决定「哪些证据算数」，而那是正臂的直接输入），必须重审。这条写在类型旁边，不留在本文件里。

## 6.4 Test Validity Verifier（§191）

§191 的五个检查**逐条照录**成闭合枚举：

```
pub enum TestValidityCheck {
    RequirementMapping,    // 测试对应哪条 Requirement
    AssertionStrength,     // 断言是否足够强
    ErrorImplWouldPass,    // 错误实现是否可能仍通过
    MockBypassesRealPath,  // Mock 是否绕过真实路径
    MissingBoundary,       // 边界条件是否缺失
}

pub enum CheckVerdict { Pass, Fail, Unknown }

pub struct TestValidity {
    pub evidence: EvidenceId,
    pub checks: Vec<(TestValidityCheck, CheckVerdict)>,  // 五臂各一条，缺即错
    pub verdict: Validity,
}

pub enum Validity { Valid, Invalid, Unknown }
```

**聚合规则（本设计定，规范未给）**：`checks` 里**任一 `Fail` ⇒ `Invalid`**；五条全 `Pass` ⇒ `Valid`；其余（无 `Fail` 但有 `Unknown`）⇒ `Unknown`。取「任一 `Fail` 即整条失效」的理由：这是三档聚合里**更严的一档**——代价是可能**多判**为无效，而不是漏判。另有一种更严的读法（把 `Unknown` 也算作 `Invalid`），本设计**不取**：那会把 §123 的正式未知降格成「判为无效」，与 §123 允许正式表达未知的立意相抵。**规范未给这条聚合规则**，故它写在此处并在 §15 第 4 条登记。

**`TestValidity` 的构造点要求 `checks` 五臂齐**：缺臂返回 `Err`。理由是这些检查里任何一条都可能是唯一发现「测试无效」的那条，静默缺一条等于静默放宽。

**为什么有效性是独立的记录而不是 `Evidence` 的一个字段**：§4.1 已述——验证者不能改被验之物。

## 6.5 取得（§8.3 `Test Validity Verifier ← 测试代码 + Requirement`）：**只定义判定与消费点，不建产生者**

**事实**：《工程》§8.3（`docs/02-工程.md:523`）给 Test Validity Verifier 的输入面逐字是 `Test Validity Verifier ← 测试代码 + Requirement`；而本设计冻结的是它的**产出**（`TestValidity`，§6.4）与聚合规则，**「测试代码」在本设计里零命中**（实测：`docs/superpowers/specs/` 与 `docs/spec/` 下唯一一处 `测试代码` 是 `2026-10-05-p3f-tool-call-path-design.md:642`，与验证无关）。§191（`docs/spec/04-method.md:298`）只列五个检查名，**没有**规定执行者与触发点。

**本设计的处置（与 §5.3 对 `VerificationProfile` 的处置同形）**：

1. **不建产生者。** 「谁跑这五个检查、什么时候跑」规范未给；在未给期间造一个，等于替 §191 选了一条它没写的执行路径（切分 §四第 4 条也禁止 P5 自建执行路径）。
2. **只定义消费点**：`coverage`（§6.1）与 `counts`（§6.3）只读 `&[TestValidity]`，由装配方以**值**传入（P4 设计 §1.2.3 的规则）。`TestValidity` 的构造点只做形状校验（`checks` 五臂齐，§6.4），不产生判定。
3. **输入面记缺口**：规范给的输入是「测试代码」，而本设计里测试的载体是 `Evidence<Test>`（§4.4 的唯一产生点）——「测试代码」（源码）**没有**落到本块的载体。**这一条不能就近决定**：把 `Evidence<Test>` 当作「测试代码」，等于替 §191 决定它的输入是什么。见 §15 第 16 条。

---

# 7. Verifier 选择与隔离（§262 §263 §29 §120 §121 §123）

## 7.1 `VerifierLevel`：五臂，逐臂照录 §262

```
pub enum VerifierLevel {
    DeterministicChecker,     // §262 第 1 级
    FormalStaticChecker,      // 第 2 级
    IndependentModelVerifier, // 第 3 级
    CrossFamilyVerifier,      // 第 4 级
    HumanReview,              // 第 5 级
}
```

声明顺序即 §262 的优先序（`docs/spec/05-normative.md:1169`），`Ord` 由声明序派生。**§262「可确定性验证 SHOULD 优先于模型判断」是 SHOULD**：本设计实现成**严格取最高可用级**（把 SHOULD 当 MUST 用）。这是**收紧**，不触 §342 第 1 条；据实记录在 §15 第 6 条。

## 7.2 候选面与选择规则

**候选以值传入**（装配方给），本块不查任何注册表：

```
pub struct AvailableVerifier {
    pub level: VerifierLevel,
    /// 该候选的 backend（§245）。§263 的隔离判定与 §120 的「不同证据路径」都读它。
    pub backend: BackendId,
    /// 供 §263/§120 判「独立性」的身份：产出该节点输出的 `ExecutionProfile` 的这三项。
    pub model: Option<String>,
    pub provider: Option<String>,
}

pub fn select_verifier(
    available: &[AvailableVerifier],
    policy: &VerificationPolicy,
    profile: &VerificationProfile,
    worker: &WorkerIdentity,
) -> Result<VerifierSelection, VerifyError>;
```

规则，逐条：

1. 取 `level` 最高（`Ord` 最小）的**可用**候选。`available` 里出现的即「可用」——**本块不判可用性**：那是资源层与策略层的事（`A5`/`OPEN-004`，§15 第 8 条）。
2. `policy.min_verifier_level` 有值时，不作为的候选被丢弃；一个不剩即 `Err(NoAvailableVerifier)`。
3. `profile.independent_verifier_required == true` 时，与 Worker **不独立**的候选（§7.3）被丢弃；一个不剩即 `Err(NoIndependentVerifier)`。
4. **不可用时返回 `Err`，不静默降级。** `A5`/`OPEN-004` 明写降级路径未定义；在未定义期间替它选一条，就是替规范做决定。降级只有两个合法来路：`profile.independent_verifier_required` 显式给了假，或用户经 `Decision` 授权（§121 的「请求用户决定」、§123 的「请求用户决定」）——**两条都不在本块**。

`WorkerIdentity` 的来源：受验节点最近一次尝试的 `ExecutionProfile`（`crates/continuum-graph/src/execution.rs:20-32`，字段 `model` / `provider` / `backend` 三项；P1 设计 §14 明写这六处字段在 P3 之前恒为 `None`）。**故今天独立性与 family 判定只能读到 `None`** ⇒ 见 §14 第 4 条。

## 7.3 §263 的独立性判定

§263（`docs/spec/05-normative.md:1185`）：「Verifier MUST 尽可能与 Worker 隔离。SHOULD 不共享：完整 Worker reasoning、Worker 自己的完成声明」。

本设计把这条拆成**两件不同的事**，各自有落点：

| 事 | 落点 | 类型层还是运行层 |
|---|---|---|
| 不共享 Worker 的完成声明与完整推理 | **输入面的裁剪** | 类型层（§7.4） |
| Verifier 与 Worker 的**身份**隔离 | 候选过滤 | 运行层（本节） |

身份隔离的判据（本设计定，规范未给比较规则）：

- `backend` 相同 ⇒ **不独立**（同一条执行路径不可能独立于自己）。
- `model` 与 `provider` 均已知且两项都相同 ⇒ **不独立**。
- 任一项为 `None` ⇒ **判不出**。此时：`independent_verifier_required == true` ⇒ 该候选被丢弃（fail-closed）；否则候选保留并**在结果里标注「独立性未确立」**（`VerifierSelection.independence: IndependenceVerdict`），使这一点在审计里可见。

**§120 的「不同 family」**（`docs/spec/02-positioning.md:1703`）需要 family 的判据，而今天 `model` / `provider` 是裸 `String`（`execution.rs:22`，P1 设计 §14 明写 P3 接入时收紧为强类型 id）。故**本设计不实现第 4 级的 family 判定**，只把候选的 `level` 当作已由装配方分类好的事实，并把 family 判据的缺席记为对账条目（§14 第 4 条）。

## 7.4 两个输入面：`InitialVerifierInput` 与 `PostBlindVerifierInput`

§263 的初次验证输入面是「Task Contract / Artifact / Evidence」三者；§29（`docs/spec/01-concepts.md:1252`）规定「先进行 blind verification，**之后才**比较 Worker reasoning summary」。

```
/// §263 的初判输入面。**是一个独立的类型，不是 `Evidence` 的切片**：
/// 只有这样，「不共享 Worker 的完成声明与完整推理」才是结构上的——
/// 它的字段里没有任何一个装得下这两样。
pub struct InitialVerifierInput {
    pub contract: ContractView,
    pub artifacts: Vec<Artifact>,
    pub evidence: Vec<Evidence>,
}

impl InitialVerifierInput {
    /// 装配处**做两件事**：
    /// 1. 拒绝任何 `producer` 指向受验节点自己的证据（§263）；
    /// 2. 输入面**只有** §263 点名的三类，不加第四类。
    ///
    /// **不接受 `WorkerCompletionDeclaration`，也没有第二个构造点**——
    /// 这是 §263 与 §29 在本设计里的结构性兑现。
    ///
    /// **`TestValidity` 也不在输入面里**：它是 Test Validity Verifier 的**产出**，
    /// 把它再喂进初判输入，等于让上一轮的判定喂下一轮——那正是 §120 的
    /// 「共享证据路径」的一种。它只进 `coverage()`（§6.1），不进 verifier 的输入。
    pub fn assemble(
        contract: ContractView,
        artifacts: Vec<Artifact>,
        evidence: Vec<Evidence>,
        verified_node: &NodeId,
        occurred_at: i64,
    ) -> Result<InitialVerifierInput, VerifyError>;
}

/// §29 的「blind verification 之后才比较 Worker reasoning summary」。
/// **构造点的第一个参数是 `BlindVerdict`**：没有盲判结论，这一类输入写不出来。
pub struct PostBlindVerifierInput {
    pub blind: BlindVerdict,
    pub worker_declaration: WorkerCompletionDeclaration,
}
```

两条结构性保证，逐条给**照片**：

1. **初判输入装不下 Worker 的完成声明**——`InitialVerifierInput` 的字段里没有这种类型，且它只有一个构造点。照片：一条 `trybuild` 的 `compile_fail` 用例，写 `InitialVerifierInput { worker_declaration: .. }`，编不过（本仓已有 `trybuild` 的既有做法，见 §12.3）。**这是构造性论证**（类型层），不是「我没找到办法塞进去」。
2. **`PostBlindVerifierInput` 不能先于盲判存在**——构造点的参数是 `BlindVerdict`。照片：同上，写 `PostBlindVerifierInput::new(declaration)`，编不过。
3. **受验节点自己产的证据进不了初判输入**——`assemble` 返回 `Err(EvidenceFromVerifiedNode { .. })`。照片：一条正例（别的节点产的证据 ⇒ `Ok`）与一条反例（受验节点产的 ⇒ `Err`），**两侧都拍**。

**第 3 条超出 §263 的文本，代价据实记在此处**：§263 的 SHOULD 逐字只点名两样——「完整 Worker reasoning」与「Worker 自己的完成声明」（`docs/spec/05-normative.md:1185`）——它**没有**说受验节点自己产的 `Evidence` 要排除。本设计第 3 条比它更严（fail-closed）。**代价**：一条叶子节点的证据若**只能**由它自己产出，则 `assemble` 返回 `Err`，该轮**装不出初判输入** ⇒ 判定不得产出（`Err` 直接冒泡为 `VerifyError`），节点在该轮完不成。本设计接受这个代价，理由是另一侧更坏：允许自产证据进初判输入，等于让受验者自证，§263 的隔离在同一节点内失效。**第 3 条的正例（n 用例）有一个前提**：图上有别的节点产同类证据——本条不退化为「一律 `Err`」靠的是这个前提，而**无别的节点时**它的形态是「初判输入不可装配」，与本设计其余 fail-closed 侧同向（不静默放行），不是未定义。

**残余，据实记录**：`Claim` 是自由文本（§4.3.2），协议上仍可以把一段推理写进 `claim`。第 1 条保证的是「结构里没有那个位置」，不是「内容里没有那句话」。**这是缺口**，见 §15 第 2 条。

## 7.5 判定结果的形状（§123 §121）

```
pub enum VerifierVerdict {
    Pass,
    /// §266 的 `no_blocking_verification_failure` 为假。
    /// **没有 `blocking: bool` 参数**：规范未给「非阻断失败」的判据，
    /// 故本轮所有失败都是阻断的（更严的一侧，不触 §342 第 1 条）。
    Fail { reason: Claim },
    /// §123：正式表达未知，不逼模型在证据不足时给结论。
    Unknown { reason: Claim },
}

/// 一轮里全部 verifier 判定的聚合。**五臂的判据互斥且有序**，
/// 声明序即优先序——`AdjudicationRequired` 优先于 `Failed`（理由见本节下段）。
pub enum VerdictAggregate {
    /// §121：同时存在 `Pass` 与 `Fail`。**不得多数投票**（§121 原文）。
    /// **优先级最高**：只要同时出现 `Pass` 与 `Fail`，即取本臂，不取 `Failed`。
    AdjudicationRequired,
    /// 至少一条 `Fail` **且无 `Pass`**。
    Failed,
    /// 无 `Fail`、无 `Pass`（全是 `Unknown`）。
    Unknown,
    /// 无 `Fail`、有 `Pass` 且有 `Unknown`。
    PassedWithUnknown,
    /// 全部 `Pass`。
    Passed,
}

pub enum IndependenceVerdict { Independent, NotIndependent, NotEstablished }

/// §29 的 `blind verification`（`docs/spec/01-concepts.md:1264-1266`：
/// 「先进行 blind verification。之后才比较 Worker reasoning summary。」）的结论。
/// **臂与 `VerifierVerdict` 逐臂同形**，单独命名的理由只有一条：
/// `PostBlindVerifierInput`（§7.4）要的是**盲判这一轮**的结论，而不是任何一轮 verifier 的结论——
/// 两者在类型上分开，「先盲判、后比较」的构造顺序才可检查（§7.4 第 2 条）。
///
/// **臂从哪来**：规范**未给** `BlindVerdict` 的臂（§29 只规定顺序，未规定结论的形态）。
/// 本设计取「与 `VerifierVerdict` 同形的三臂」，唯一构造点是
/// `BlindVerdict::from(VerifierVerdict)`——盲判即 Verifier 在看不到 Worker 完成声明
/// 与推理时的判定，本设计不为它新增判定语义。
pub enum BlindVerdict {
    Pass,
    Fail { reason: Claim },
    Unknown { reason: Claim },
}
```

**`VerdictAggregate` 的臂为何要有优先序**：早先 `Failed` 写成「至少一条 `Fail`」，而 `Pass ∧ Fail` 同时满足「至少一条 `Fail`」与「同时存在 `Pass` 与 `Fail`」两条——**两臂判据互相覆盖、无优先级**，实现者按臂的次序写 `match` 会得到两种结果之一。现把 `Failed` 收窄为「至少一条 `Fail` **且无 `Pass`**」，两臂即互斥。取 `AdjudicationRequired` 优先（而不是反过来把 `Pass ∧ Fail` 判成 `Failed`）的理由：§121 对这一形态的处理是「进入 Adjudication，不得多数投票」，取 `AdjudicationRequired` 才让调用方看到它需要裁决；取 `Failed` 会把一个需要裁决的形态压成一次普通失败。§12.1 (p) 的照片钉的就是这一侧。**两臂对 §8.3 第 5 项同效**（都不满足 `NoBlockingVerificationFailure`），故这个次序不动完成判定的结果，只动返回给调用方的变体。

**§121 的处置**：本块只做**检测**（`Pass ∧ Fail ⇒ AdjudicationRequired`），**不选**四种补救里的任何一种（运行额外确定性检查 / 增加第三 verifier / 降低 confidence / 请求用户决定）——§121 用的是「可能」。选一种就是发明；故把 `AdjudicationRequired` 作为返回值交给调用方，由其升 `Decision`（那属 P4 的 Decision 面，§14 第 1 条）。

**§29 的 4K 例子在本设计里的落点**：level 1 的 verifier 读 `artifact_refs` 里制品的 metadata，发现 `1920×1080` ≠ 用户要求的 4K ⇒ `Fail`。输入面里**没有** Worker 的「任务完成」声明（§7.4），故 Worker 的宣告不能覆盖它——§29 的最后一句由此成立。**「verifier 从哪个字段读 metadata」不在本块**：`Artifact.metadata` 是 `serde_json::Value`（`crates/continuum-artifact/src/artifact.rs:177`），其 schema 规范未给（§15 第 9 条）。

**「无物可读」时判 `Unknown` 而不是 `Pass`**：level 1 的确定性检查需要一个可读的制品；没有制品时不许给 `Pass`。这是 §4.4 的构造点规则在判定侧的对应，两侧都钉。

## 7.6 `C3`（§263 vs §340）的处置

`C3`（`docs/01-总纲.md:1729`）：「§263 要求与 Worker 隔离，§340 要求独立审查优先用更强模型。只有一家强模型时两者不可兼得」。

**本设计的读法（据实）**：§340（`docs/spec/05-normative.md:2645`）出现在「计划审查三层结构 / AI Reviewer / Self Review 与 Independent Review 分离 / User Review View」这一组里（§338–§341），它管的是**计划审查**；§262/§263 管的是**验证**。两者不是同一件事的两条规则，是两条不同领域的规则——`C3` 把它们写成冲突，本设计**不承认这条冲突约束本块**：本块只实现 §262 的顺序 + §263 的隔离。

**这条读法要写清它是读法**：若协调者认为 §340 也管验证侧（即「独立 verifier 也要优先取更强模型」），则 §7.2 的规则要加一条**强度序**，而「更强」的判据（§247 的 Model Capability Profile？§248 的 SkillVector？）不在本块。**收件人：协调者 + P3d**，记在 §14 第 4 条。

---

# 8. Completion Predicate（§266 §195）

## 8.1 十个合取项：**取 §266 与 §195 的并集**

§266（`docs/spec/05-normative.md:1245`）给五项，§195（`docs/spec/04-method.md:437`）给八项——**两者不是同一张表的两个抄本**：§195 有五项在 §266 里没有对应物。若只取 §266，那五项被静默丢掉；§195 的标题正是「Completion Predicate **强化**」，丢掉它是**放宽**，触 §342 第 1 条的方向。故取**并集**。

**并集里的两条去重（本设计定，理由附）**：

| 两条名字 | 合成一项的理由 |
|---|---|
| §266 `all_required_requirements_verified` ≈ §195 `requirement_coverage_sufficient` | **同义性本设计未论证**：前者可读作「每项 REQUIRED 都拿到了 verifier 的判定」，后者是 OPEN-001 缺的那件「充分」，两者**不是显然同一件事**。合并之所以安全，只在**方向**上成立：合并后的臂 `AllRequiredRequirementsVerified` 恒 `Undetermined`（§3.4 第 3 条），故即便两者本非同义，合并也不会把任何一项放行——**这是 fail-closed 的方向，不是同义性论证** |
| §266 `contract_satisfied` ≈ §195 `task_contract_satisfied` | 判据同一：§225 的 Constraint Validator 的结论（这一条是**同义**，两者都指向同一处结论） |

**若协调者要它们分开**，改动是枚举加两臂 + 两个判定函数各自接一条输入；判定的归属不变。**第一对与第二对的证据强度不同**：第二对有「同一处结论」这一条可指，第一对只有 fail-closed 的方向——分开第一对时，`requirement_coverage_sufficient` 仍缺判据（是 OPEN-001 那件），加臂也只是多一枚恒 `Undetermined` 的臂。

十个臂（`Conjunct`）：

```
pub enum Conjunct {
    ArtifactExists,                        // §266 第 1 项
    AllRequiredRequirementsVerified,       // §266 第 2 项（≈§195 的 requirement_coverage_sufficient）
    ContractSatisfied,                     // §266 第 3 项（≈§195 的 task_contract_satisfied）
    MandatoryEffectsCompleted,             // §266 第 4 项
    NoBlockingVerificationFailure,         // §266 第 5 项
    MandatoryTestsPass,                    // §195
    ApplicablePropertiesHold,              // §195
    ApplicableFuzzBudgetComplete,          // §195
    ApplicableMutationQualitySufficient,   // §195
    IndependentVerificationPass,           // §195
}
```

## 8.2 `CompletionVerdict`：**没有「完成」这一臂**

```
pub enum CompletionVerdict {
    /// 至少一项为假。列出为假的项。
    Blocked { failing: Vec<Conjunct> },
    /// 无一项为假，但至少一项**无判据或判不出**。列出这些项。
    Undetermined { undecidable: Vec<Conjunct> },
}
```

**「完成」为什么不是第三臂**：`AllRequiredRequirementsVerified` 的输入是 `CoverageVerdict`，而那个枚举**没有正臂**（§6.2），故合取**不可能为真**。

**这是构造性论证，不是实测**：断的是「本类型里不存在一个可达的『完成』值」，判据是 §6.2 的类型定义 + 本函数的合取结构，与「我试着跑了一遍没跑出来」不是一回事。**它的实测补件**是一条 `trybuild` 的 `compile_fail` 用例：写 `CompletionVerdict::Complete { .. }`，编不过——把「该臂不存在」变成一条可执行断言（§12.3）。

**两条臂与 Intent 状态的对接不在本块**（切分 §四第 4 条）。本块给出的两个返回值都是「不得完成」，如何路由（`Blocked` → 节点 `FAILED` 或重试；`Undetermined` → `Decision`）是执行器与 P4 的 Decision 面的事，记为 §14 第 1、6 条。

**§266 的最后一句**（「模型 MUST NOT 自己宣告『我认为已经完成』作为最终依据」）在本设计里是**类型层**的事实：那条宣告既进不了初判输入（§7.4），也换不出 `CompletionVerdict` 的正臂（本节）。

## 8.3 逐项：本轮可否判定 / 缺的是哪一步

| # | 合取项 | 本轮 | 缺的是哪一步 | 收件人 |
|---|---|---|---|---|
| 1 | `ArtifactExists` | **可判** | —— 判据：受验图的终端节点各自至少有一个产出制品（读 `artifact` 表，§10） | —— |
| 2 | `AllRequiredRequirementsVerified` | 不可 | 覆盖充分性的判据（OPEN-001） | 规范维护者 + 协调者（§3） |
| 3 | `ContractSatisfied` | 不可 | §225 的 Constraint Validator 的结论以**值**传入；本块不判它。值没给出来时不得臆断 | P4（§14 第 1 条） |
| 4 | `MandatoryEffectsCompleted` | 不可 | 该 Intent 的强制效应集合与它们的终态，以**值**传入（§268 的 Effect Journal 属边界层；§9.1 里它不在本层的上游） | P2 + 装配方（§14 第 5 条） |
| 5 | `NoBlockingVerificationFailure` | **可判** | —— 判据：`VerdictAggregate` 里不出现 `Failed`（§7.5），且无 `AdjudicationRequired` | —— |
| 6 | `MandatoryTestsPass` | 不可 | **「测试是否通过」的承载处未定义**：`Evidence` 无 outcome 字段（§258），`Artifact.metadata` 的 schema 未给（§15 第 9 条） | P5c + 规范维护者（§14 第 2 条） |
| 7 | `ApplicablePropertiesHold` | 不可 | 同上，且「applicable」的判据（§195 的限定词）未给 | P5c + 规范维护者 |
| 8 | `ApplicableFuzzBudgetComplete` | 不可 | `fuzz_budget` 的量纲（§5.2） | 规范维护者（§15 第 3 条） |
| 9 | `ApplicableMutationQualitySufficient` | 不可 | `mutation_score` 的量纲与 `mutation_threshold` 的比较规则（§9.2） | 规范维护者（§15 第 3 条） |
| 10 | `IndependentVerificationPass` | **可判**（独立性未确立的一值不可判） | 判据（`IndependenceVerdict` 三臂各有落点）：`Independent` ∧ 该 verifier 的 `VerdictAggregate ∈ {Passed, PassedWithUnknown}` ⇒ **真**；`NotIndependent` ⇒ **假**；`NotEstablished` ⇒ **不可判**（`Undetermined`），**不得写成假**——见本节下段 | —— |

**第 10 项的 `NotEstablished` 一臂：不可判，不是判为假。** 早先这一项的判据写成 `independence != NotEstablished ∧ …`，当独立性为 `NotEstablished`（§7.3 明写这是 `independent_verifier_required == false` 时**保留候选并标注**的那个状态）时给出 `false` ⇒ `Blocked { failing: [IndependentVerificationPass] }`——**这正是 §8.4 刚刚警告过的「把『没判』写成『判为假』，会让调用方去查一个不存在的失败」**，且与本设计自己的纪律相抵。今按同一条纪律处置：`NotEstablished` ⇒ `Undetermined`（第 10 项列入 `undecidable`），与 §8.4 那四项 `Option` 的 `None` 同法。**两侧都 fail-closed**（既不完成、也不谎报失败），照片见 §12.1 (s)。

**另有第十一项，不是合取项但同等效力**（§8.4 的第 6 条输入）：`Intent.completion_predicate` 的声明值。

**「可判」的三项（1、5、10）不是「可放行」**：它们只是可能为假。第 2 项恒不可判 ⇒ 合取永不为真。

## 8.4 输入面与 `Result`

```
pub struct CompletionInput {
    /// 第 1 项：由受验图与制品表算出。
    pub artifact_exists: bool,
    /// 第 2 项：§6 的逐项覆盖判定，数目与 REQUIRED 项相同。
    pub coverage: Vec<RequirementCoverage>,
    /// 第 3、4 项：**以值传入**（P4 的约束结论 / P2 的效应终态）。`None` = 未给出 ⇒ 不可判。
    pub contract_satisfied: Option<bool>,
    pub mandatory_effects_completed: Option<bool>,
    /// 第 5、10 项：§7 的判定产出。
    /// `independence == NotEstablished` ⇒ 第 10 项**不可判**（列入 `undecidable`），**不是为假**——
    /// 与下面四项 `Option` 的 `None` 同一条纪律（见本节末「`Option<bool>` 而不是 `bool`」）。
    pub verifier_verdicts: VerdictAggregate,
    pub independence: IndependenceVerdict,
    /// 第 6–9 项：以值传入（装配方从制品与预算面读出）。`None` = 未给出 ⇒ 不可判。
    pub mandatory_tests_pass: Option<bool>,
    pub applicable_properties_hold: Option<bool>,
    pub applicable_fuzz_budget_complete: Option<bool>,
    pub applicable_mutation_quality_sufficient: Option<bool>,
    /// §222 的 `Intent.completion_predicate`——**用户声明的追加谓词**。
    /// 规范只给字段名、未给语法（P4 设计 §16 第 9 条把判定交给本层）。
    /// **本块只判「它有没有内容」**：有内容 ⇒ 追加一项不可判（fail-closed，§342 第 1 条）；
    /// 空/缺 ⇒ 不追加。**不解析它**——解析就是发明语法。
    pub declared_completion_predicate: Option<serde_json::Value>,
}

pub fn judge_completion(input: &CompletionInput) -> Result<CompletionVerdict, VerifyError>;
```

`CompletionVerdict` 与 `Result`：`Blocked`/`Undetermined` 都**不是** `Result` 本身；§344 的 `Result` 由调用方据本返回值组装（含逐项状态）。本块**不定义** `Result` 的形状（那是执行层与长期循环的合流处，切分 §三）。

**`Option<bool>` 而不是 `bool`**：`None` 与 `false` 是两件事——前者是「没给出这一项」⇒ `Undetermined`，后者是「明确为假」⇒ `Blocked`。取 `bool` 会把两件事压成一件，而压缩的方向是**把「没判」写成「判为假」**——那是把不放行说成了失败，会让调用方去查一个不存在的失败。**两侧都要钉**（§12.1 (c)/(d)）。

## 8.5 《工程》§8.4 五行的落点

| §8.4 的行（`docs/02-工程.md:530-538`） | 本块的落点 | 本轮形态 |
|---|---|---|
| 测试通过不进入 Completed，除非 Requirement Coverage 判定充分 | §6.2（无正臂）+ §8.2（无完成臂） | 「除非」那一支不存在（§3.3 第 4 条） |
| 每项 REQUIRED 存在对应证据集合，无证据时为 INCOMPLETE | §6.1（迭代域逐项）+ §6.2（`NoValidEvidence`） | 全量 |
| 模型不能自行宣告完成（§266） | §7.4（输入面）+ §8.2（无正臂） | 类型层 |
| mask 外像素差被确定性检出 | §7.1 第 1 级的插槽 + §7.5 的「无物可读 ⇒ `Unknown`」 | **比较器本身属 P5f**（§326/§168），见 §14 第 3 条 |
| Verifier 的初判输入不含 Worker 的完成声明与完整推理 | §7.4 第 1、2 条 | 结构层（有一条残余，§15 第 2 条） |

---

# 9. Verifier Adversary（§264 §215）与 Mutation（§265 §192）

## 9.1 Verifier Adversary：**只冻类型与消费规则，本轮无触发**

§264（`docs/spec/05-normative.md:1206`）说「高价值任务 MAY 创建 VerifierAdversary」，目标是「寻找错误实现但当前 verifier 仍 PASS」；§215（`docs/spec/04-method.md:1144`）把这条写成流程。`B1`（`docs/01-总纲.md:1712`）：「规范未规定其强制触发条件；『高价值任务』尚未定义，相关成本归属也未明确」，评级为**可推迟**。

本设计的处置：

1. **不定义「高价值任务」**——定义它要一个取值域，规范未给。
2. **冻类型与消费规则**：

   ```
   pub enum AdversaryOutcome {
       NoCounterexample,
       /// §264：找到反例 ⇒ `verification_strategy = insufficient`。
       CounterexampleFound { passed_verifier: BackendId, counterexample: Claim },
   }
   ```

3. **消费规则**：`CounterexampleFound` 时，该轮 `VerdictAggregate` **不得**取 `Passed`/`PassedWithUnknown`（§215 的「必须增强验证体系」落在这一条上）。

4. **据实记录一件不体面的事**：在「不放行」的判定面上（§3.2），§215/§264 的加固**没有可观察的完成后果**——加固前后都是「不得完成」。故本轮它唯一的可见形态是**一条 finding 记录 + `verification_strategy` 的标注**（写进 §10 的事件与表），不是判定反转。**这不是可以把 §264 丢掉的理由**：它记的是「这套验证策略已被证伪」，而那个事实在 OPEN-001 解决之后立刻变成载荷（§3.4 的翻案条件）。

## 9.2 Mutation Verification：**只冻插槽，不判**

§265（`docs/spec/05-normative.md:1229`）：「适用任务 SHOULD 执行 Mutation Testing……如果大量 mutation 无法被测试捕获：test evidence strength ↓」。§192（`docs/spec/04-method.md:322`）给出四种变异（`x > 10` → `x >= 10` / `x < 10` / `true` / `false`）与 `mutation_score`。

**本设计的落点**：mutation 是一种**确定性检查**（跑测试套件对变异体），故它是 §262 第 1 级的候选之一，其产物是一条 `Evidence`——**类型为 `EvidenceType::Test`，而不是新臂**，理由是 §265 的原文把它的作用写成「test evidence strength ↓」（对**已有**测试证据的调制），不是新证据（§4.2 已记这条读法）。

**不判的部分**：`strength` 无 `Ord`（§4.3.4），`mutation_score` 无进 `Evidence` 的字段（§258 的字段表里没有它），`mutation_threshold` 无量纲（§5.2）。**故「大量 mutation 未被捕获」这一步本轮落不下来**，`ApplicableMutationQualitySufficient` 因此恒不可判（§8.3 第 9 项）。

**缺的是哪一步（写全）**：`mutation_score` 的**量纲与取值域**（是比例还是计数？分母是什么？满分是多少？）——没有它，`threshold` 无从比较，「大量」无从判定。收件人：规范维护者（§15 第 3 条）。

---

# 10. 持久化

## 10.1 迁移号段

**实测占用**（2026-10-08，逐 crate 读 `Migration::new` 的第一个参数）：`1`、`2`（`crates/continuum-persist/src/db.rs:29`、`:46`）、`10`（artifact）、`20`（graph）、`30`（workspace）、`40`（effect）、`41`（policy）、`50`（capability）、`80`（model-registry）。`101`、`102` 只出现在 `crates/continuum-persist/tests/migrations.rs:68-69` 的测试夹具里，**不进生产链**（`Db::open_with` 的迁移集由调用方给）。P4 的设计取 `100/110/120`（P4 设计 §2.4）。

**本块取 `130`**（实测：全仓 `Migration::new(` 无 `130`）。**这一档不是切分文档分配的**——切分文档没有号段表；P6 的切分 §四第 2 条明写该轮的档「**待与 P5 那轮对账**」。故：本块只声明自己用的号，**P5 其余五块与 P6 的档由协调者统一划**（§14 第 9 条）。

## 10.2 表

```
evidence              evidence_id PK, graph_id, verified_node, producer_node,
                      subject_kind, subject_requirement, evidence_type,
                      claim, strength(TEXT/JSON), scope(TEXT/JSON), occurred_at
evidence_artifact     (evidence_id, artifact_id) PK, → artifact 的 id
test_validity         evidence_id PK, verdict
test_validity_check   (evidence_id, check_name) PK, verdict
verification_round    round_id PK, graph_id, verified_node, selection(JSON),
                      aggregate, independence, strategy_verdict, occurred_at
```

约束与不变量，逐条：

1. **`evidence` 只 INSERT，无 UPDATE**（§4.1）。二次写同一 `evidence_id` 撞主键得 `Err`（照片：裸 INSERT 断言主键拒且原行不变，与 P4 对 `contract` 表的做法同形）。
2. **枚举列一律小写 + 下划线编码**，经本 crate 的显式 `*_str` / `parse_*` 辅助函数，不依赖 serde（P1 设计 §15 的同一条纪律：`NodeState` 的 serde 是 `SCREAMING_SNAKE_CASE`，直接反序列化会失败）。
3. **一行 `evidence` 对应图上一条 `EdgeKind::Evidence` 边**：`producer_node → verified_node`（§238 的语义面由 P1 设计 §8.2 定为「验证关系。指向被验证节点」）。本块**不另造**验证关系边（切分 §二）。**这一条今天无法做成外键**：`adfir_edge` 无主键（P1 设计 §15 明写），且 `Edge` 的四个端点里有两个是**端口**（`crates/continuum-graph/src/edge.rs:47-54`）——而「验证关系」并不天然有端口。故本条以「写行时同事务核对图上存在该边，缺则 `Err`」的形态落地，**并把「EVIDENCE 边是否需要可寻址的身份」提为对账条目**（§14 第 6 条）。
4. **`verification_round` 记一次判定**，含 §9.1 的 `strategy_verdict`：这使 §215/§264 的 finding 在本轮就有落点（§9.1 第 4 条）。

## 10.3 事件与事务

- **`EventType::VerificationFailed`**（`crates/continuum-events/src/event.rs:25`）的第一个产生点：`VerdictAggregate ∈ {Failed, AdjudicationRequired}` 时写一条，payload 含逐项状态。**状态写入与事件写在同一事务**（P1 的 `apply_transition` 形状，`crates/continuum-graph/src/transition_tx.rs`）。
- **不写 `IntentCompleted`**：本块不产生「完成」（§8.2），故那个事件类型在本轮仍无产生点。**据实记录**：`crates/continuum-persist/src/tx.rs:369` 有一处按 `IntentCompleted` 判 `saw_skip` 的分支，本设计不改它、也不知道它是否有活路径（那是 P0/P2b 的面）。
- **不写审计**：§313 的必录清单里没有验证类事件（`crates/continuum-events/src/audit.rs:18-34` 的 `AuditKind` 八项），故本块不新增 `AuditKind`。

---

# 11. 与已建之物的接缝（逐条）

| 已有之物 | 本块的用法 | 是否需要改它 |
|---|---|---|
| P0 `Tx` / `Migration` / `PersistError`（`crates/continuum-persist`） | §10 的落库路径 | 否 |
| P0 `EventType`（9 类） | 写 `VerificationFailed` | 否（类型已有，只是第一次有写入方） |
| P1 `Node` / `NodeId` / `AdfirGraph` / `OperatorRef` | 身份与图结构 | **一处文档改动**：`node.rs:19` 的注记补读者（§2.3） |
| P1 `EdgeKind::Evidence` | 验证关系的**唯一**表达处（§10.2 第 3 条） | 否；**但有一条请求**（§14 第 6 条） |
| P1 `ExecutionProfile`（`execution.rs:20`） | §7.3 的 Worker 身份 | 否（本块只读三项，且它们今天恒为 `None`） |
| P1 状态机（`state.rs:30-31` 的 `Verifying → Completed`） | **本块不给许可**：§8.2 无完成臂 | 否；**执行器的路由提为对账条目**（§14 第 6 条） |
| P2 `EffectState` / Effect Journal（`crates/continuum-effect`） | 只消费「强制效应完成否」的**值** | 否（不登记边，§2.1） |
| P3d `ModelId` / Router（§250） | **不接**：§7.2 的候选以值传入 | 否；family 判据提为对账条目（§14 第 4 条） |
| P4 `TaskContract` / `Requirement` / `VerificationRequirement`（设计 §7.1 §7.2） | 以 `ContractView` 的**值**读 `id` / `class` / `verification_requirement` | 否；**本块不解析 `verification_requirement`**（P4 §7.2 的「只存不判」在本块延续为「只读不判」） |
| P4 `Intent.completion_predicate` | §8.4 的第 6 条输入（只判有无内容） | 否；改由本块接下的部分见 §15 第 7 条 |

**一条据实记录**：切分文档 §二末「与 P4 的接口面」段把「P5a 要接的 P4 接口面」写成 `TaskContract.requirements[].verification_requirement`、`constraints`、`verification_policy` 与预算面。**实测：§224 的 `TaskContract` 里没有 `constraints`，也没有 `verification_policy`**（`docs/spec/05-normative.md:209-254`；`constraints` 是 §236 的 Node 字段，`verification_policy` 同理）。故本块对 P4 的实际接口面是：`requirements[].{id, class, verification_requirement}` 与预算面（本块不用预算）；`constraints` / `verification_policy` 属执行层。**这不改变任何一方的归属，只订正一处措辞**，并记在此处以免下一轮照它去找两个不存在的字段。

---

# 12. 测试策略与照片

## 12.1 两侧守卫与逐项照片（判定侧 19 条 + 结构侧 4 条）

**枚举逐臂**（每臂一条往返 + 数目断言；`ALL` 的完整性由「数目断言 + 穷尽 `match`」把关，与 `crates/continuum-artifact/tests/artifact_type.rs:23` 同形）：`EvidenceType`（11 臂）、`VerifierLevel`（5 臂）、`Conjunct`（10 臂）、`TestValidityCheck`（5 臂）、`CheckVerdict`（3 臂）、`CoverageVerdict` + `IncompleteReason`（2+2 臂）、`Validity`（3 臂）、`VerifierVerdict` / `VerdictAggregate`（3/5 臂）、`BlindVerdict`（3 臂）、`CompletionVerdict`（2 臂）、`IndependenceVerdict`（3 臂）、`AdversaryOutcome`（2 臂）。

**这份清单要求「每一枚枚举的每一臂」都有数目断言**：`BlindVerdict` / `CompletionVerdict` / `AdversaryOutcome` 三枚早先漏在清单外（它们的臂数没有数目断言），现补入——漏一枚枚举即漏「臂数漂移」这一整类回归。

**判定侧**（逐条都是「正例 + 反例」成对，缺一条即不算钉住）：

| # | 用例 | 钉的是哪一侧 |
|---|---|---|
| a | 一条 REQUIRED、零证据 ⇒ `Incomplete { NoValidEvidence }` | `Incomplete` 的正例 |
| b | 一条 REQUIRED、证据齐备且有效 ⇒ **`NotEstablished`（且不是别的）** | **§342 第 5 条的照片**：测试通过换不来「满足」。**这是本设计最重要的一条** |
| c | `profile.required_evidence_types` 含 `Fuzz`，证据集合只到 `Test` ⇒ `Incomplete { MissingRequiredTypes }` | 缺项判据 |
| d | 同上但 `Fuzz` 已在 ⇒ `NotEstablished` | 缺项判据的**另一侧**（不是「只要缺项判据在就恒不满足」） |
| e | `Test` 证据无 `TestValidity` 记录 ⇒ 不计数 ⇒ `NoValidEvidence` | §191 的「测试不自动有效」 |
| f | `Test` 证据有 `TestValidity{verdict: Valid}` ⇒ 计数 | 上一条的**另一侧** |
| g | `TestValidity` 五条检查里一条 `Fail` ⇒ `Invalid` | §6.4 的聚合规则 |
| h | `TestValidity` 少写一条检查 ⇒ 构造点 `Err` | 五臂齐 |
| i | 无 Profile ⇒ `Err(ProfileMissing)`（**不是**「无要求」） | **fail-open 的那一侧** |
| j | 非空但不认识的 `verification_policy` ⇒ `Err(UnknownVerificationPolicy)`；`Value::Null` ⇒ `Default` | §5.1 两臂各一条 |
| k | 3 条 REQUIRED 的 Contract ⇒ `coverage` 返回**3** 条 | §342 第 1 条（不静默降低）。**契约的 REQUIRED 条数在用例里手写，不从被测清单里取** |
| l | 无可用候选 ⇒ `Err(NoAvailableVerifier)`；`independent_verifier_required = true` 且无独立候选 ⇒ `Err(NoIndependentVerifier)` | **不静默降级** |
| m | 受验节点自己产的证据进 `assemble` ⇒ `Err(EvidenceFromVerifiedNode)` | §263 |
| n | 别的节点产的同类证据 ⇒ `Ok` | 上一条的**另一侧**（否则 m 可以由「一律 Err」满足） |
| o | level-1 检查无制品可读 ⇒ `Unknown`（**不是** `Pass`） | §7.5 的 fail-closed 侧 |
| p | `Pass ∧ Fail` ⇒ `AdjudicationRequired`（**不是**多数票、不是 `Passed`） | §121 |
| q | `CompletionInput` 四项 `Option` 为 `None` ⇒ `Undecidable{undecidable: [...4 项...]}`；为 `Some(false)` ⇒ `Blocked{failing: [...4 项...]}` | §8.4 的 `None` vs `false`，**两侧都钉** |
| r | 有内容的 `declared_completion_predicate` ⇒ 追加一项不可判；空/缺 ⇒ 不追加 | §8.4 第 6 条输入，两侧 |
| s | `independence = NotEstablished` ⇒ 第 10 项列入 `undecidable`（**不是** `Blocked`）；`NotIndependent` ⇒ `Blocked{failing: [IndependentVerificationPass]}` | §8.3 第 10 项的三臂，**两侧都钉**（「没判」与「判为假」不可互写） |

## 12.2 变异预告（谁红）

四档，每档预告**哪些用例红、哪些不该红**（预告写反会让人去查不存在的问题）：

| 变异 | 该红 | 不该红 |
|---|---|---|
| **取反**：`CoverageVerdict::Incomplete` 与 `NotEstablished` 在计数规则里的位置互换（第 3 条计数变不计数） | a、c、e | b、d、f（它们对两臂的具体身份不敏感的那一半）；**j、i 恒红**（与覆盖无关） |
| **取反**：`Validity` 的 `Valid` / `Invalid` | f、g | a、b、c、d |
| **放宽**：`from_value` 的未知分支返回 `Default` 而不是 `Err` | j 的第二个子例 | 其余全部（**这一档若不红，说明 j 只拍了空策略那一侧**） |
| **收紧**：`Unknown` 的检查结果算作不计数（第 5 条计数规则取反） | b 只在「证据里有 `Unknown` 有效性」的语料上红 ⇒ **该用例须备一份这样的语料**，否则此档不红 = 缺用例 | a、c |
| **移除**：`MissingRequiredTypes` 这一臂从判定里删掉 | c | d、a、b |
| **移除**：`assemble` 里那条「受验节点自己产的证据」检查 | m | n（n 是它的反向对照，**应当仍绿**） |
| **取反**：`judge_completion` 把 `independence == NotEstablished` 当作 `NotIndependent`（「没判」写成「判为假」） | s 的第一个子例（`NotEstablished ⇒ undecidable`，被写成 `Blocked`） | s 的第二个子例（`NotIndependent ⇒ Blocked` 不受影响）、q（`Option` 四项的 `None`/`false` 是另一条纪律）、其余全部 |
| **取反**：`VerdictAggregate` 的 `AdjudicationRequired` 与 `Failed` 次序互换（`Pass ∧ Fail` 落到 `Failed`） | p | 其余全部（**这一档若不红，说明 p 只拍了单项 Fail**） |

**一条预防**：`k` 的期望值必须手写；若写成 `contract.requirements.len()`，则它测的是 Rust 的 `len()`，恒真——**语料从被测清单里取就是恒真的假照片**（本仓已有此判据）。

## 12.3 结构层的照片（`trybuild`）

本仓已有 `trybuild` 的既有做法（`crates/continuum-connector/Cargo.toml:53` 的 `trybuild` 与 `crates/continuum-connector/tests/type_level.rs` 的 `compile_fail`；`continuum-workspace`、`continuum-secrets`、`continuum-model-registry`、`continuum-capability`、`continuum-node` 的 `Cargo.toml` 同样登记了它）。本块用三条：

1. `InitialVerifierInput` 塞 `WorkerCompletionDeclaration` ⇒ 编不过（§7.4 第 1 条）。
2. `PostBlindVerifierInput::new(declaration)`（无 `BlindVerdict`）⇒ 编不过（§7.4 第 2 条）。
3. `CompletionVerdict::Complete { .. }` ⇒ 编不过（§8.2 的「该臂不存在」）。
4. `Evidence { .. }` 与 `Evidence::default()` ⇒ 编不过（§4.4 的「唯一产生点」是结构事实）。

## 12.4 拍不出照片的地方

1. **端到端（一个节点从 `RUNNING` 经 `VERIFYING` 走到某处）**：执行器（第 3 层）未建，且切分 §四第 4 条禁止 P5 自建执行路径。故本块的全部照片都是**单元与集成层**：直接构造输入、断言返回值。**P1 的 `transition` / `apply_transition` 至今也是同样情形**（P1 设计 §18 自陈「无执行点」）。
2. **§264/§215 的效果**：在「不放行」的判定面上没有可观察的完成后果（§9.1 第 4 条），故只能拍到「finding 被记录 + `strategy_verdict` 被标注」，拍不到「完成判定被反转」。
3. **`strength ↓`（§265）**：无 `Ord` 可依赖，故「下降」不可断言（§9.2）。
4. **R 条目里的「用户经 `Decision` 授权降级」**：`Decision` 对象属 P4 且未建，故这条路径今天不可构造（§14 第 1 条）。
5. **`mandatory_effects_completed` 的真值**：Effect Journal 属边界层且与本块无依赖边，本块只收值；「值从哪来」的照片在 P2 一侧。

---

# 13. 完成判据（逐条落点）

《工程》§8.2（`docs/02-工程.md:500-515`）的三条自陈 + §8.4 的五行为本层判据，已在 §8.5 逐行落点。此处补 §8.2 的三条：

| §8.2 的陈述 | 本设计的落点 |
|---|---|
| 「Evidence 是验证的唯一输入形态（§258）。工具结果必须先转为 Evidence 再进入判定（§89）」 | §4.4 的唯一构造点；§7.4 的输入面只收 `Evidence` 与 `Artifact`，不收裸工具结果 |
| 「Requirement Coverage Graph 把每项 REQUIRED 映射到证据集合（§260）。该组件是 §342 第 5 条不变量的执行点，也是 §342 第 10 条的输入之一」 | §6.1 的迭代域 + §6.2 的无正臂 |
| 「VerificationProfile 决定需要哪些证据类型（§261）。该组件的生成者未定义（A6）」 | §5.2 的形状 + §5.3 的「生成者不建」 |
| 「Completion Predicate 判定器独立于 Worker。输入为 Contract、Artifact、Evidence（§263），不接受 Worker 的完成声明」 | §8.4 的输入面 + §7.4 的类型层禁止 |
| 「Verifier 优先级按 §262 的五级顺序。表中第 3、4 级在单用户环境下可能不可得，降级路径未定义（A5）」 | §7.1 五臂 + §7.2 第 4 条（不可用即 `Err`，不发明降级） |

**一处措辞相抵，据实记录**：§8.3 的链写 `Verifier 判定 ← VerificationProfile + Evidence + Artifact`（**没有 Contract**），而 §263 与 §8.2 都把 Task Contract 列进 Input（`docs/02-工程.md:516-528` vs `:500-515`）。本设计取**并集**（`VerificationProfile + Contract + Artifact + Evidence`，§7.4 的 `InitialVerifierInput`），理由：少一件输入会让判定方拿不到被验证的契约；而 §263 的 SHOULD 明写它该在。**收件人：规范维护者 + 《工程》维护者**（§15 第 10 条）。

---

# 14. 与其余各块的对账条目（逐条具名）

以下每条都是**本设计假设了别的块的某枚形状**之处。逐条写清「假设了什么」与「为什么必须假设」。

1. **待与 P4（语义层）对账**：本设计假设 `RequirementId` 由 P4 的 `continuum-semantics` 提供（§4.3.1）、`ContractView` 的字段名与 §224 一致（§6.1）、`ContractSatisfied` 与「`Intent.completion_predicate` 有无内容」以**值**进入 `CompletionInput`（§8.4）。依据是切分 §二末「与 P4 的接口面**照设计写，不照实现写**」，本设计据此读 P4 设计 §7.1 / §7.2 / §16（`:999`、`:1018`、`:2321`）。**P4 的设计若再改，由本块的一方复核并订正，不由 P4 替它改**（切分原话）。
   另：`Blocked` / `Undetermined` 如何升 `Decision`（§7.5、§8.2）落在 P4 的 Decision 面上，本块只给返回值形状。
2. **待与 P5c（代码领域算子）对账**：本设计假设「测试是否通过」由**制品**承载（§8.3 第 6 项），故需要一个 `TestResult` 制品的 payload 约定。**若 P5c 把结果放在别处（或需要一枚新的 `EvidenceType` 臂），须向本块报出**（§4.2：证据类型是 P5a 的，四块不得自加）。`§193` 的 Differential / Metamorphic 两臂已在本设计里预置，P5c 若无对应实现须说明。
3. **待与 P5f（图像领域算子）与 P5e（媒体）对账**：§326/§168 的「mask 外像素差被确定性检出」是 §262 第 1 级的一个实例：**插槽与「无物可读 ⇒ `Unknown`」的规则在本块（§7.5），比较器在 P5f**。另：`EvidenceType::VisualCheck` / `Benchmark` 是否够 P5e/P5f 用，若不够须向本块报出。
4. **待与 P3d（模型注册表与 Router）对账**：本设计假设「独立 verifier」「跨 family verifier」的判据可由**候选的身份**判出，而今天 `ExecutionProfile.model` / `.provider` 是裸 `String`（`crates/continuum-graph/src/execution.rs:21-22`），**family 的判据不存在**，故 §7.6 明写本块不实现第 4 级的 family 判定。另：Verifier 选定级别之后，「级别内具体选哪个 backend」是否走 §250 的 Router，**本设计假设不走**（装配方直接给候选），这一条要与 P3d 对账。
5. **待与 P2（边界层）对账**：`mandatory_effects_completed` 以**值**传入（§8.4）。本设计假设这个值由装配方从 Effect Journal（§268）读出；**本块不登记到 `continuum-effect` 的边**（§2.1）。值的类型与产生点须与 P2 对账。
6. **待与 P1（执行层）对账**：三条。
   (i) `Node.verification_policy` 的字段类型**不动**，只改 `:19` 的注记（§2.3）——若 P1 认为字段应当定型，改动方案与其代价已写在 §2.3。
   (ii) `EdgeKind::Evidence` 的边**是否需要可寻址的身份**：§10.2 第 3 条的不变量今天只能以「写行时核对」实现，因为 `adfir_edge` 无主键、且 `Edge` 要求两个端口（`crates/continuum-graph/src/edge.rs:47-54`）而验证关系不天然有端口。
   (iii) **执行器的路由**：`state.rs:30-31` 允许 `Verifying → Completed`，而本块不产出许可它的判定（§8.2）。故 `Blocked` / `Undetermined` 各自路由到哪里（节点 `FAILED`？重试？升 `Decision`？），须由 P1／执行器／协调者指认。
7. **待与 P5b（执行方法库）对账**：§90（`docs/spec/02-positioning.md:949`）允许插件扩展 **Verifier backend**——而「Verifier backend 的登记形态」与 P5b 的「方法登记形态」是不是同一件事，本设计**不决定**（切分 §四第 3 条禁止在 P5b 过审前自造方法登记形态）。本设计只假设：候选以 `Vec<AvailableVerifier>` **值**传入（§7.2）。
8. **待与 P5d（研究领域算子）对账**：§330 的 `CitationVerification` 与 `ContradictionCheck` 是本块证据侧的两个供给方（切分 §五 P5d 行）。本设计假设它们的产物能落进现有的十一臂；**若 `CitationVerification` 需要自己的臂，须向本块报出**（§4.2）。
9. **待与 P6 与协调者对账（迁移号段 + 工具结果的两个消费者）**：P6 的切分 §四第 2 条与第 4 条都明写「待与 P5 那轮对账」。(i) 号段：本块取 `130`（§10.1），P5 其余五块与 P6 的档须统一划。(ii) §89 的工具结果有两个消费者（§279 的 Compiler 与 §89 的 Evidence 转换）：本设计假设两者**不共享 wrapper**，本块只为证据侧提供 `Evidence::from_tool_result`（§4.4）。
10. **待与装配方（`continuum-runtime`）对账**：`runtime_migrations()`（`crates/continuum-runtime/src/main.rs:98`）要加本块的一行（§2.1）；`p5a_verify_migrations()` 的名字与返回类型照既有八处的形状。

---

# 15. 遗留与未决项

**每条具名收件人；凡「缺的是哪一步」都写到步骤，不用「后续」「长期阶段」兜。**

1. **`strength` 无取值域、无比较规则**（§4.3.4）：§258 只给字段名，§265 的「strength ↓」是一条方向而**等级集未给**。本设计据实取**结构性保存**（无 `Ord`）。**缺的是「文献级强度档位与它们之间的序」这一步。收件人：规范维护者。**
2. **`Claim` 是自由文本，能把推理塞进初判输入**（§4.3.2、§7.4 残余）：类型层只能保证「没有 reasoning 字段」。**缺的是「claim 的语法或长度/形态约束」这一步。收件人：规范维护者。**
3. **OPEN-001（`ENG-004`）本块显式处置为「不判充分」**（§3）。三个可选方向各自缺的那一步已列出（§3.1）。**本块不宣布关闭它。收件人：规范维护者 + 协调者。**
4. **§191 的五检查怎么聚合成「有效/无效」未定义**（§6.4）：本设计取「任一 `Fail` ⇒ `Invalid`」（fail-closed）。**缺的是聚合规则这一步。收件人：规范维护者。**
5. **`VerificationProfile.optional_evidence_types` 的用途未给**（§5.2）：本轮只存不判。**缺的是「optional 相对于 required 的作用」这一步。收件人：规范维护者。**
6. **§262 的「可确定性验证 SHOULD 优先」被本设计当 MUST 用**（§7.1）：这是收紧，据实记录。**若协调者要留出偏离空间，须指出允许偏离的判据。收件人：协调者。**
7. **`Intent.completion_predicate` 只有字段名、无语法**（§8.4、§11）：P4 设计 §16 第 9 条把判定交给本层，本设计接下但**只到「判有无内容」**——有内容即追加一项不可判（fail-closed，§342 第 1 条）。**缺的是该谓词的语法与求值器这一步。收件人：规范维护者 + P4。**
8. **`A5`/`OPEN-004`「Verifier 独立性不可得时的降级路径」未定义**（§7.2 第 4 条）：本设计**不发明降级**，不可用即 `Err`。**缺的是降级规则这一步。收件人：规范维护者。**
9. **`Artifact.metadata` 的 schema 未给**（`crates/continuum-artifact/src/artifact.rs:177`）：§29 的「读 resolution metadata」与 §8.3 第 6 项的「测试是否通过」都依赖它。**缺的是元数据的字段约定这一步。收件人：P5c（`TestResult` 载荷）+ 规范维护者。**
10. **§8.3 的 `Verifier 判定 ←` 那一行缺 Contract**（§13 末）：与 §263/§8.2 相抵，本设计取并集。**缺的是择一改这一步。收件人：规范维护者 + 《工程》维护者。**
11. **《工程》§8.5 的 `A6`/`OPEN-009`「VerificationProfile 的生成者」未定**（§5.3）：本设计只冻结取得点、缺档案即 `Err`。**缺的是生成者与它输入面这一步。收件人：规范维护者 + 协调者。**
12. **`B1`「高价值任务」未定义**（§9.1）：故 §264 的 Adversary 本轮无触发。**缺的是触发条件与成本归属这一步。收件人：规范维护者。**
13. **`C3`「§263 vs §340」**（§7.6）：本设计按「§340 属计划审查段、不约束验证侧」读，故不实现「优先更强模型」。**这是一条读法**，若协调者不认，须给出「更强」的判据与它的持有方。**收件人：协调者 + P3d。**
14. **「测试是否通过」的承载处未定义**（§8.3 第 6 项）：`Evidence` 无 outcome 字段（§258）、`Artifact.metadata` 无 schema（第 9 条）。**缺的是「谁写、写在哪一列」这一步。收件人：P5c + 协调者。**
15. **本块的判定今天没有生产调用方**（§12.4 第 1 条）：执行器未建。**这不是本块的缺口，是第 3 层的**；据实记录以免被读成「忘了接线」（P1 设计 §18 已有同形的一条）。
16. **`Test Validity Verifier` 的产生者与输入面未定义**（§6.5）：§8.3 的依赖行逐字给 `Test Validity Verifier ← 测试代码 + Requirement`（`docs/02-工程.md:523`），而「测试代码」在本设计里零命中；§191（`docs/spec/04-method.md:298`）只给五个检查名，未给执行者与触发点。本设计只冻结产出（`TestValidity`，§6.4）与消费点（`counts`，§6.3）。**缺的是「谁跑这五个检查、输入取自哪里（源码／制品／`Evidence<Test>`）」这一步。收件人：规范维护者 + P5a 的下一轮。**
17. **§189 的 `Verification → 判断证据是否充分` 与本设计「不判充分」相抵**（§3.5）：这是规范正文里**直接要求判充分**的那一句（`docs/spec/04-method.md:210`，表 `:242-243`），且落在本设计的「规范依据」区间内。本设计不执行，理由见 §3.1；另一种读法（对阶段的描述、非对本块的 MUST）已列出。**缺的是「§189 是对本块的 MUST 还是对阶段的描述」这一步的裁定。收件人：规范维护者 + 协调者。**
18. **P5c–P5f 的六行共写今天无人认领**（§2.1 的共写文件表）：`Cargo.toml` 的 `[workspace] members` 与 `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED`（`:25`）各需六行（P5a–P5f 各一），而 **P5c/P5d/P5e/P5f 的设计今天在 `docs/superpowers/specs/` 下不存在**（实测目录：该目录只有 P5a、P5b 与两份切分），故「六行由谁加齐」在本轮无人接。**缺的是「一次加齐还是逐块各加自己一行、由谁在哪一步做」这一步。收件人：协调者（在 P5b–P5f 各自的实现计划派单时定）。**
