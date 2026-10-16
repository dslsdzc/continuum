use std::path::Path;
use std::process::Command;

fn cargo_tree(pkg: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    // --edges all 同时覆盖 normal、build、dev。只查 normal 不够：
    // dev-dependency 同样允许测试代码引用 provider 类型，
    // 而 §346 的中立性约束要拦住所有引用路径。
    let out = Command::new(env!("CARGO"))
        .args(["tree", "-p", pkg, "--edges", "all", "--prefix", "none"])
        .current_dir(root)
        .output()
        .expect("cargo tree 无法执行");
    assert!(
        out.status.success(),
        "cargo tree -p {pkg} 失败: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("cargo tree 输出不是 UTF-8")
}

/// 每个包：自身 + 允许依赖的内部 crate。
/// 只查「不含某字符串」不够——P1 新增四个 crate，
/// 必须逐包断言允许集合，否则反向边不会被发现。
const ALLOWED: &[(&str, &[&str])] = &[
    ("continuum-core", &[]),
    ("continuum-events", &["continuum-core"]),
    ("continuum-persist", &["continuum-events"]),
    // P3 子项目 C 的 Task 3 起加上 capability：`tool.rs` 的 `AuthorizedToolInvocation`
    // 要能命名 `continuum_capability::AuthorizedTool`（§316 请求面换血，层内边，
    // 设计 §4.1 订正段）。
    // Task 4 起加上 persist：**dev 边**——`tests/invoke_tool.rs` 的夹具要起真库、
    // 经 `Tx` 调 `continuum_capability::authorize` 取 `AuthorizedTool`（`cargo tree --edges all`
    // 把 dev 边算进依赖，故与普通边一样登记）。数组按字母序。
    (
        "continuum-provider",
        &["continuum-capability", "continuum-core", "continuum-persist"],
    ),
    // Task 15 起 artifact 直接依赖 continuum-events：`save_artifact` 与元数据
    // 同事务写入 ArtifactCreated（§16）。
    (
        "continuum-artifact",
        &["continuum-core", "continuum-events", "continuum-persist"],
    ),
    ("continuum-port", &["continuum-artifact"]),
    ("continuum-operator", &["continuum-artifact"]),
    // Task 15 起 graph 直接依赖 continuum-events（`apply_transition` 在迁移的
    // 同一事务内写 NodeStarted / NodeCompleted），dev-dependencies 同一条。
    (
        "continuum-graph",
        &[
            "continuum-artifact",
            "continuum-events",
            "continuum-port",
            "continuum-operator",
            "continuum-persist",
        ],
    ),
    // P2 边界层：Workspace 抽象与只读强制。events 与 persist 由设计第 4 节要求——
    // Gate 的审计记录与 Task Workspace 的元数据都要落库。
    // 不含 continuum-core：本 crate 至今没有用到它，而本表与 Cargo.toml 必须精确一致。
    (
        "continuum-workspace",
        &["continuum-events", "continuum-persist"],
    ),
    // P2 边界层的进程与内核层：子进程隔离。
    // 只依赖 workspace 抽象（spawn 的参数是 `&TaskWorkspace`，见设计第 4.2 节），
    // 不依赖 P2 其余 crate——沙箱不知道 Effect Journal 与 Policy 的存在。
    // 不含 continuum-core：本 crate 至今没有用到它，而本表与 Cargo.toml 必须精确一致。
    ("continuum-sandbox", &["continuum-workspace"]),
    // P2 边界层的效应记录层。persist 用于 Journal 落库、events 用于状态变更的审计
    // 记录（设计上篇第 7.3 节：状态变更与对应审计记录在同一事务内提交）。
    // 本 task 只建记录体与状态机，两个依赖的使用点分别在 Task 2 与驱动。
    // 不含 continuum-core：本 crate 不使用它的任何类型（设计下篇第 3 节），
    // 而叶子 crate 的条目记的是实际依赖。
    (
        "continuum-effect",
        &["continuum-events", "continuum-persist"],
    ),
    // P2 边界层的策略层。artifact 与 effect 有使用点：设计下篇第 5.6 节的
    // `PolicyContext` 装 `PrivacyClass` 与 `EffectType`；persist 自 Task 6 起大量使用
    // （`src/persist.rs` 的迁移与 `load_policies`）。
    // **不含 continuum-core**：本 crate 对它的引用为零（`grep -rn continuum_core` 无命中），
    // 设计第 3 节虽在依赖方向图里列过这条边，但同一节又定「叶子 crate 的条目记实际依赖，
    // 故无使用点的边即假边」——留一条零使用的边与该口径冲突，故删（`Cargo.toml` 同步删）。
    // 不含 continuum-events：策略变更不在 `§313` 的必录清单内（设计上篇第 3 节）。
    (
        "continuum-policy",
        &["continuum-artifact", "continuum-effect", "continuum-persist"],
    ),
    // P3 资源层：Capability 词汇表、Tool/ToolProfile、ToolRegistry 与签发点。
    // Task 1 用到 effect（`CapabilityKind::for_effect` 取 `EffectType`，方向写在本
    // crate 上以免 continuum-effect 反向依赖）；Task 3 起加上 core——`Tool.id` 复用
    // `continuum_core::tool::ToolId`，**不另建第二个 `ToolId`**（设计 §3.1）；
    // Task 4 起加上 persist——`tool` 表的迁移与读写经 `Tx`（设计 §3.3、§7）。
    // Task 5 起加上 events——`registry.rs` 的 `authorize` 在成功授权时写一条
    // `AuditKind::CapabilityGrants`（设计 §3.4）。至此设计 §6 的边表在本 crate 上到齐。
    (
        "continuum-capability",
        &[
            "continuum-core",
            "continuum-effect",
            "continuum-events",
            "continuum-persist",
        ],
    ),
    // P3 子项目 A 的强制点 (3)：密钥运行时（设计第 5 节）。设计 §6 的终态边表在
    // 本 crate 上写的是 `continuum-capability, continuum-core`，但本 crate 对
    // `continuum-core` 零引用（`grep -rn continuum_core crates/continuum-secrets`
    // 无命中），故只登记实际用到的那一条——叶子 crate 的条目记实际依赖，
    // 零使用的边即假边（P2b 为此删过两条）。
    ("continuum-secrets", &["continuum-capability"]),
    // P3 子项目 B 的连接器边界层（设计 §3.2）。Task 4 起加两条：
    // `continuum-provider`——`adapter.rs` 的适配器实现 §124 的 `Connector`（决定 B-4）；
    // `continuum-persist`——**dev 边**，`tests/audit.rs` 要起真库读 `audit_log`
    // （设计 §7.3 已写明这条 dev 边必须有）。`cargo tree --edges all` 把 dev 边算进依赖，
    // 故它与普通边一样登记。`continuum-effect` 由 Task 5 加（那时 `error.rs` 的
    // `EffectAuthorizationRequired` 才写出 `EffectType` 这个类型名）——不提前，叶子 crate
    // 的条目记的是实际依赖，一次声明齐会让条目在中间若干 task 里说谎。
    (
        "continuum-connector",
        &[
            "continuum-capability",
            "continuum-core",
            "continuum-effect",
            "continuum-persist",
            "continuum-provider",
            "continuum-secrets",
        ],
    ),
    // P3 资源层（子项目 D）：模型画像、Model Registry 与 Router。设计 §8.2 的三条边是
    // core（ModelId / ToolId / ProviderHealth）、capability（Cost / Latency 两个**类型**，
    // 不取能力凭据）、persist（三张表经 Tx）；边由需要它们的 task 增量加。
    // Task 3 起登记前两条：`profile.rs` 的 `ModelProfile` 同时用到 `ModelId` / `ToolId`
    // 与 `Cost` / `Latency`（设计 §2.1、§2.5）——**数组按字母序**，与 C 的 provider 条目同形。
    // Task 5 起登记第三条 persist：`persist.rs` 的 `p3d_model_migrations()` 返回
    // `Vec<Migration>`。**这条边由用它的那个 task 登记**（就是 Task 5），不推给别处。
    (
        "continuum-model-registry",
        &[
            "continuum-capability",
            "continuum-core",
            "continuum-persist",
        ],
    ),
    // P3 子项目 E：计算节点与放置。设计 §7.2 的唯一一条边是 artifact（`Artifact` /
    // `PrivacyClass`，§243 的隐私输入）。**不登记 model-registry**：《工程》§4.3 的
    // 「节点放置 ← Router 输出」**已记为阻断**（今天无落点，收下从不读的形参按裁定删去，
    // 设计 §6；收件人见 §10 第 8 条）——**不是漏登记**。
    // **不登记 core**：本设计对它零引用。**不登记 persist / events**：本设计不写库（§4.2）。
    ("continuum-node", &["continuum-artifact"]),
    // P5b 执行方法库（设计 §9；《工程》§8.3 `docs/02-工程.md:526`「Execution Method Library ← 无」）。
    // **空数组是硬断言**：任何直接的反向边（含指向 continuum-graph 者）都会让本门变红——
    // 这正是「realized_by 取文本而非 OperatorRef」那条读法的**编译期照片**（设计 §3.2、§9）。
    // 该照片只覆盖依赖边，**不覆盖 realized_by 的文本内容**（那在 Vec<String> 里，编译期无从核）。
    // 本门查的是直接边（--depth 1），故对 continuum-operator 的传递依赖不出现在这张照片里。
    ("continuum-method", &[]),
    // P4 语义层（设计 §2.1）：Input Canonicalization Frontend 与 Reference Resolver。
    // 本层不登记任何指向执行层／资源层／边界层的边；允许集只含 continuum-persist
    // （alias 表经 Tx）与层内前驱。`continuum-semantics ← continuum-canonical` 是
    // 反向的读取方，不在这里。
    //
    // P4 的 Task 7 起填入那条边：`alias` 表的迁移 100 与四条读写路径经 `Tx` 访问
    // （设计 §4.6）。**上面几行的「只含 `continuum-persist`」在 Task 7 之前是「将来时」**
    // ——那时数组是 `&[]`，本行注释写的是「允许集是空条目……今天还没有边」；
    // Task 7 兑现了它。表是**允许集合**的转录（不是使用点派生），故这一格此前一直空着
    // 不算错，只是当时还没有使用者。
    ("continuum-canonical", &["continuum-persist"]),
    // P4 语义层（设计 §2.1）：Intent / Specification / Contract / ContractDiff / Plan / Decision /
    // Constraint Validator。`← continuum-canonical` 是 §1.3 的类型边（Intent.goal 取 EntityId）；
    // `← continuum-persist` 由本 task 登记（intent 表）。
    // **允许集里没有 continuum-budget**——§11.5 的第 1 条守卫（两道门分离）。
    ("continuum-semantics", &["continuum-canonical", "continuum-persist"]),
    // P4 语义层（设计 §2.1）：预算树、Allocation / Reservation / Remaining、账本、Budget Validator。
    // `← continuum-semantics` 是 owner 要 IntentId（§12.2）；`← continuum-persist` 是账本与树经 Tx。
    // **反向没有边**：continuum-semantics 的允许集里没有本 crate（§11.5 的两侧守卫之一）。
    ("continuum-budget", &["continuum-persist", "continuum-semantics"]),
    // P5e 媒体领域算子（设计 §2.1）。Task 3 起登记当时实际有的两条边；
    // continuum-graph / continuum-method / continuum-verify 三条分别由需要它们的 task 追加。
    // **订正（Task 7）**：原注记写「（前两条是 dev 边，理由见各 task）」——照三条的列举
    // 次序读会得出 method 是 dev 边，而设计 §2.1 的边别是：dev 边只有 `continuum-graph`
    // 与 `continuum-verify`（生产代码对它们零调用），`continuum-method` 是普通边
    // （`bind_media_methods` 是生产入口，不是用例）。订正为「（其中 `continuum-graph` 与
    // `continuum-verify` 是 dev 边，理由见各 task：生产代码对它们零调用）」。
    // Task 7 起加上 continuum-graph：**dev 边**——`tests/reuse.rs`（§6.2 的 (r) 条、
    // §305 的复用照片）经 `cache_key` / `can_reuse` 核对本块声明的 `determinism`
    // 真能让那条判据成立；`cargo tree --edges all` 把 dev 边算进依赖，故与普通边一样登记
    // （先例：continuum-provider 那一行的 persist）。数组按字母序。
    (
        "continuum-media",
        &[
            "continuum-artifact",
            "continuum-graph",
            "continuum-operator",
        ],
    ),
    // P5a 验证与证据判定（设计 §2.1）。本块是第 8 层，只依赖执行层与其下：
    // continuum-artifact / continuum-graph / continuum-operator（Task 2 加）、
    // continuum-events / continuum-persist（Task 9 加）。
    // Task 2 起登记三条：`continuum-artifact`——`Evidence.artifact_refs` 取 `ArtifactId`；
    // `continuum-graph`——`EvidenceProducer::Node` 取 `NodeId`；`continuum-operator`——
    // 同上的 `BackendId`。Task 9 再加两条：`continuum-events`——写 `VerificationFailed`
    // 取 `Event` / `EventType`；`continuum-persist`——迁移 130 与四个写/读函数经 `Tx`。
    // 数组按字母序。
    // **不含 continuum-semantics**：`语义层 (2) → 执行层 (3) → 跨领域 (8)` 已在链上，
    // **不是**因为成环（见跨计划前置第二节的订正：按 §9.1 的箭头约定，本块依赖语义层沿链、不成环）。
    // **不登记它是无条件的**（设计 §6.1）：本块以值承载 Contract / Requirement 身份、
    // 不 `use` 语义层类型，故无实际用到的边可登记。
    // 〔订正 2026-10-16〕`continuum-semantics` 已随 P4 合入；结论不变——不得登记该边，
    // 理由是以值承载（此前「该 crate 尚不存在」的表述已过时）。
    (
        "continuum-verify",
        &[
            "continuum-artifact",
            "continuum-events",
            "continuum-graph",
            "continuum-operator",
            "continuum-persist",
        ],
    ),
    // P5 跨领域层（设计 §2.1）：研究领域的七枚算子。
    // 本 crate 不新建表、不写事件、不碰策略与能力面。
    // Task 3 起加上 continuum-method：**普通边**——`bind_research_methods` 是生产入口
    // （取 `&mut MethodRegistry` 逐条 `bind`，不是用例；设计 §2.1）。
    // Task 5 起加上 continuum-graph：**dev 边**——`tests/reuse.rs`（§9.2 第 (k) 条、
    // §305 的复用照片）经 `cache_key` / `can_reuse` 核对本块声明的 `determinism`
    // 真能让那条判据成立；`cargo tree --edges all` 把 dev 边算进依赖，故与普通边一样
    // 登记（先例：continuum-provider 那一行的 persist）。
    // Task 6 起加上 continuum-verify：**dev 边**——`tests/type_level.rs` 的两份
    // `trybuild` 编译失败样例（§9.4 第 1 条）钉「本 crate 里造不出 `Evidence`」；
    // 生产代码对它零调用，这两份样例是本块在它上面的**唯一使用点**。`trybuild` 把
    // [dev-dependencies] 也算进样例的依赖（实测 trybuild-1.0.121/src/run.rs:239），
    // 故样例能引用 `continuum_verify::Evidence` 而不必把这条边升成普通边——
    // 这是该 dev 边成立的依据。`cargo tree --edges all` 把 dev 边算进依赖，
    // 故与普通边一样登记（先例：continuum-provider 那一行的 persist）。数组按字母序。
    (
        "continuum-research",
        &[
            "continuum-artifact",
            "continuum-graph",
            "continuum-method",
            "continuum-operator",
            "continuum-verify",
        ],
    ),
    // Task 12 起 runtime 直接依赖这几个：artifact 与 graph 用于在启动流程里
    // 注册 P1 迁移，workspace 用于注册 P2 的 workspace 表迁移（Task 4）。
    //
    // Task 8 起加上 effect：驱动的 `--effect <类型>:<目标>` 要在**解析期**把类型
    // 收窄到 `EffectType` 的封闭枚举（下篇第 6.8 节：开放类型会让策略表漏判，
    // 未知类型不该到运行期才失败），故解析器取用该 crate 的 `EffectType::parse`。
    //
    // Task 10 起加上 policy：第 7 步的集成路径要查策略并按裁决决定是否铸造批准值
    // （下篇第 5.7 节），故取用该 crate 的 `load_policies` / `decide` / `PolicyContext`
    // 与它的 `p2_policy_migrations()`（`policy` 表由驱动自己建，见 `main.rs` 的装配处）。
    //
    // P3 的 Task 4 起加上 capability：装配处注册 `p3_capability_migrations()`
    // （`tool` 表由用它的那个 task 注册）。
    //
    // P3 子项目 D 的 Task 5 起加上 model-registry：装配处注册 `p3d_model_migrations()`
    // （三张模型表由用它的那个 task 注册，即 D 自己——「谁的表谁注册」）。
    //
    // P3 子项目 B 的 Task 7 起加上 secrets：`main.rs` 的装配处在分派之前构造并持有
    // 密钥运行时（`src/secrets.rs` 的 `assemble`，设计 §4.1「驱动装配好传进来」、
    // §11 第 15 条把这条边的所有者写死为 B）。
    //
    // P4 的 Task 19 起加上 budget：**dev 边**——`tests/budget_projection.rs` 的形状乙
    // 两条断言（裁定 §12.6.1）要在同一处穷尽解构 `continuum_budget::Remaining` 与
    // `continuum_model_registry::BudgetView`；本 task 只把它作 dev 边用，**Task 20 才把它
    // 升成普通依赖**（理由是注册迁移与钩子）。`cargo tree --edges all` 把 dev 边算进依赖，
    // 故与普通边一样登记（`docs/superpowers/p3bcdf-followups.md` §四.1）。
    //
    // P4 的 Task 20 起加上 continuum-canonical 与 continuum-semantics（budget 同 task 由
    // dev 升为普通依赖）：装配处注册三者的迁移与预算恢复钩子——`main.rs` 的
    // `runtime_migrations()` 追加 100／110／120 三行，`recover_cmd::run` 注册
    // `BudgetReservationRecovery`（挂 `MarkLostExecutions`）。上一段「本 task 只把它作
    // dev 边用」说的是 Task 19 当时的状态，自本 task 起 budget 是普通依赖。
    //
    // 本表是设计第 3 节「依赖方向」的允许集合按**已存在的 crate** 转录，不是按使用点派生，
    // 故它容忍「允许但尚无使用点」的边。这一条今天对 sandbox 与 events 成立
    // （**core 与 provider 已不成立**——下方的订正块记着这两处的使用点）：
    // sandbox 的使用点是驱动（设计第 10 节）装配 Sandbox，而驱动不在 P2 上篇。
    // **这句话原写「对 sandbox 与 core / events / provider 都成立」，是过宽的主句**；
    // 旧说法留在原处（见下一段订正块的引文），此处只收窄射程——初稿漏改，
    // 与紧邻的订正块相抵（F 终审 I-2）。
    //
    // **订正（2026-10-07，P3 子项目 F 的 Task 3）**：本段原先写「core / events / provider 在
    // runtime 内**至今无任何引用**」——那半句对 core 与 provider **已假**：`src/cli.rs` 用
    // `continuum_core::tool::ToolId`（`tool` 子命令的 `--tool`；`tests/cli.rs` 与
    // `tests/tool_call.rs` 也各引它一处），`src/error.rs` / `src/tool_call.rs` / `src/tool_cmd.rs`
    // 用 `continuum_provider` 的 `ToolCallError` / `ProviderRegistry`（`tests/tool_call.rs`
    // 也引它一处）。**`events` 仍无引用**：`grep -rn continuum_events crates/continuum-runtime/src/`
    // **exit 1、零命中**（语料限定在 `src/`），故那句话的射程收成「events 至今无引用」。
    // **原句是过宽的说法，自本 task 起不再成立**
    // （它出现在一份正要引入那个引用的计划里——「至今无任何引用」这种状态陈述自带到期日，
    // 本 task 就是兑现它的那一天）。
    // **订正（2026-10-08，F 终审 m-2）**：此处原写「`grep -rln continuum_events
    // crates/continuum-runtime/` 无命中」——**那句按字面执行有命中，命中处就是这句注释自己**
    // （注释里写着那个标识符），按字面做判据的人会以为结论被推翻。旧说法留此以免下一轮再照它
    // 得出错的结论；判据本身改为**限定到 `src/`**（该处为零命中）或读成「命中处只有这句注释」。
    // 同族见 m-1（`src/` 的「唯一一处 `println!`」）与本仓 Global Constraints 那条
    // 「凡『零命中』类判据，先问本仓的哪条规矩会往那个语料里加字」。
    // 订正的来历记此，免得下一轮扫查照着它得出错的结论（设计 §10.1 点名要求本处随实现同步订正）。
    // 表的取值不变——本表本就不按使用点派生，变的只是这段注释的真假。
    //
    // **本 task 未新增/删减任何 workspace 成员之间的边**（实测：`crates/continuum-runtime/Cargo.toml`
    // 的 `[dependencies]` 一字未动；新增的 `async-trait` 是**外部** crate 的 **dev 边**，不进本表
    // ——本表断言的是工作区成员之间的边）。故上面 `("continuum-runtime", &[…])` 那一行**不动**，
    // 本文件两个方向（已声明而越界、本表列了却没用到）的断言全绿。
    // 反向的边（真依赖却没登记）由本文件的断言抓住，不会静默。
    //
    // **订正（P4 的 Task 19）**：上一段的「那一行不动」说的是 F 的 Task 3 当时；自本 task 起，
    // 该行**含 `continuum-budget`**（本 task 新增的 dev 边，见上）。旧句留此，免得下一轮按它
    // 得出「runtime 条目此后不再变」的错结论。
    (
        "continuum-runtime",
        &[
            "continuum-artifact",
            "continuum-budget",
            "continuum-canonical",
            "continuum-capability",
            "continuum-core",
            "continuum-effect",
            "continuum-events",
            "continuum-graph",
            "continuum-model-registry",
            "continuum-persist",
            "continuum-policy",
            "continuum-provider",
            "continuum-sandbox",
            "continuum-secrets",
            "continuum-semantics",
            "continuum-workspace",
        ],
    ),
];

/// workspace 成员名单，派生自 cargo 自身解析出的 workspace 定义。
///
/// 这份名单以前是手工维护的常量。手工名单的全部问题是它会随仓库长大而变坏：
/// 新增 crate 若漏列，它既不会被当作反向边的目标检查（不在内层循环的迭代域里），
/// 也不会作为主体被检查（外层循环走的是 `ALLOWED`）——两个方向同时静默漏检，
/// 而本文件的两个用例仍然全绿。
///
/// 故改为派生：名单的来源只有一个（workspace 定义本身），不可能与仓库不同步。
/// `ALLOWED` 表不随之派生——它列的是设计第 5 节的允许集合，是规范的直接转录。
fn workspace_crates() -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    // `--workspace` 以全部成员为根，`--depth 0` 只打印各根自身，每行形如
    // "continuum-port v0.1.0 (/path/to/crate)"；`--prefix none` 去掉树形前缀。
    // 用 `cargo tree` 而非 `cargo metadata`：本文件的解析口径已经是前者的输出格式
    // （见 `cargo_tree_direct`），且无需为解析 JSON 新增依赖。
    let out = Command::new(env!("CARGO"))
        .args(["tree", "--workspace", "--depth", "0", "--prefix", "none"])
        .current_dir(root)
        .output()
        .expect("cargo tree 无法执行");
    assert!(
        out.status.success(),
        "cargo tree --workspace 失败: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).expect("cargo tree 输出不是 UTF-8");
    let mut names: Vec<String> = stdout
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .map(str::to_string)
        .collect();
    names.sort();
    assert!(
        !names.is_empty(),
        "cargo tree --workspace 未给出任何成员：输出格式已变，或 workspace 为空"
    );
    names
}

/// 只取直接依赖（`--depth 1`）。
///
/// 既有的 `cargo_tree()` 不带 `--depth`，输出的是整棵传递树，
/// 与 `ALLOWED` 表的语义不符——`ALLOWED` 列的是直接边，
/// 与设计第 5 节的依赖方向一致。直接边足以拦住反向依赖：
/// 任何反向边本身必然是一条直接边。
fn cargo_tree_direct(pkg: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = Command::new(env!("CARGO"))
        .args([
            "tree", "-p", pkg, "--depth", "1", "--edges", "all", "--prefix", "none",
        ])
        .current_dir(root)
        .output()
        .expect("cargo tree 无法执行");
    assert!(
        out.status.success(),
        "cargo tree -p {pkg} 失败: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("cargo tree 输出不是 UTF-8")
}

#[test]
fn every_crate_depends_only_on_its_allowed_set() {
    let all_crates = workspace_crates();

    // 推导出的名单与手工维护的 `ALLOWED` 必须互为覆盖。少了这一步，派生只解决一半：
    // 新 crate 会被当作反向边的目标检查，却不作为主体被检查——它的依赖方向仍然无人过问。
    let subjects: Vec<&str> = ALLOWED.iter().map(|(pkg, _)| *pkg).collect();
    for name in &all_crates {
        assert!(
            subjects.contains(&name.as_str()),
            "workspace 成员 {name} 未列入 ALLOWED，它的依赖方向不会被检查"
        );
    }
    for pkg in &subjects {
        assert!(
            all_crates.iter().any(|n| n == pkg),
            "ALLOWED 列出的 {pkg} 不是 workspace 成员：表已过期"
        );
    }

    for (pkg, allowed) in ALLOWED {
        let tree = cargo_tree_direct(pkg);
        for other in &all_crates {
            let other = other.as_str();
            if other == *pkg {
                continue;
            }
            // cargo tree 的 --prefix none 输出每行形如 "continuum-port v0.1.0 (path...)"
            let appeared = tree
                .lines()
                .any(|l| l.split_whitespace().next() == Some(other));
            assert_eq!(
                appeared,
                allowed.contains(&other),
                "{pkg} 对 {other} 的依赖与允许集合不符（出现={appeared}，允许={}）:\n{tree}",
                allowed.contains(&other)
            );
        }
    }
}

#[test]
fn core_and_persist_do_not_depend_on_provider() {
    for pkg in ["continuum-core", "continuum-persist"] {
        let tree = cargo_tree(pkg);
        assert!(
            !tree.contains("continuum-provider"),
            "{pkg} 依赖了 continuum-provider，违反 §346:\n{tree}"
        );
    }
}
