//! §4 的进程内 Compute Node 注册表（设计 §4.1）的运行期照片，与两条**源码文本守卫**。
//!
//! # 本文件里的说明为什么全在这里
//!
//! `the_registry_module_names_no_artifact_type` 读的是 `src/registry.rs` 的**全文**——
//! **包括它的注释与文档注释**。故「本文件不出现某个名字」这类说明**一个字都不能写进
//! `src/registry.rs`**：写进去，守卫会**命中它自己的那句说明**而变红
//! （本仓的一个已知坑；`2026-10-06` 第一轮评审查出计划该行初稿没说清落点）。
//! **本文件的注释不受任何守卫约束**，故全部说明落在本文件头。
//!
//! # 守卫 `the_registry_module_names_no_artifact_type` 的证明力边界
//!
//! **它是下界，不是封闭判定。** 它匹配的是三个名字的**字面拼法**
//! （`continuum_artifact` / `Artifact` / `PrivacyClass`）；**逃逸它至少有这三条路**
//! （每条都给出可检查的写法，不是推断）：
//!
//! 1. **本 crate 的其它模块**——守卫只读 `src/registry.rs` **这一份文件**。
//!    若在 `lib.rs` 里写 `pub use continuum_artifact::Artifact as A;`，
//!    本文件再写 `use crate::A;`，则**本文件一字不沾**而依赖已经建立。
//!    **这是最要紧的一条：它管的是一个模块，不是整个 crate。**
//! 2. **`include!("…")`**——被包含的文本不经过本文件。
//! 3. **由构建脚本生成再 `include!`**——(2) 的变体，来源不同（那些文本不在本仓库里）。
//!
//! 反过来说，**「在本文件内给 crate 起别名」不是逃逸**：
//! `use continuum_artifact as ca;` 这一行本身就把 `continuum_artifact` 这个字面写出来了。
//! 故**「本文件干净」不等于「本模块零依赖」**。
//!
//! **封闭的那一层是 `ALLOWED` 的逐对断言**（Task 1；
//! `crates/continuum-runtime/tests/dependency_direction.rs` 的双向 `assert_eq!`）——
//! 本 crate 整体只允许 `continuum-artifact` **一条**边，多一条就红。
//! **不得因这条守卫而省掉 `ALLOWED` 的任何一步**（协调者裁定义务 5）。
//!
//! # 为什么这三个名字一个都不能出现在注册表模块里
//!
//! §4 的注册表按《工程》§4.3 与 §9.2 是**入度为零**的组件：它登记的是「有哪些计算节点」，
//! **不是「哪个节点能收哪一档制品」**——后者是放置（§5）的事，注册表连读都不读。
//! 故「零依赖」不是靠文档里的一句话，而是靠**这条断言 ＋ `ALLOWED` 的逐对断言**两处。
//! 一旦这里出现了与制品有关的名字，最省事的写法就会顺势把隐私裁决塞进注册表，
//! 于是「注册」与「放置」在同一处收口（设计 §4.1 判据 3 明写两者**不在同一处收口**）。
//!
//! # 两条守卫的分工
//!
//! - `the_registry_module_names_no_artifact_type`：**结论**（那三个名字不出现），
//!   命中时在断言信息里**列出命中的行号**（否则红的时候读不出是哪一处）。
//! - `the_guard_sees_the_module`：**机制的正控制**。没有它，一个「什么都没读到」的守卫
//!   （路径写错、文件被读成空串）会**永远绿**——这正是「守卫须两侧都钉」里缺的那一侧。

use continuum_node::{
    ComputeNode, ComputeNodeId, NodeClass, NodeRegistry, NodeRegistryError, NodeTrust,
};
use std::path::Path;

/// 夹具：**只有 `id` 与 `trust` 两项是本文件的用例要分辨的**，其余四项由 Task 2 的
/// `tests/node.rs` 钉住（那里六项逐条各一条断言），故这里取常量、不再重复分辨。
///
/// **`trust` 做成参数是承重的**：`a_failed_registration_does_not_overwrite` 必须拿
/// 「同一 `id`、不同 `trust`」的两枚节点去撞注册表，否则「覆盖」与「不覆盖」不可区分。
fn node(id: &str, trust: NodeTrust) -> ComputeNode {
    ComputeNode::new(
        ComputeNodeId::new(id),
        // `class` 不是本文件的判据（§5 的硬闸门才读它），取定值。
        NodeClass::Personal,
        trust,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
}

/// 登记顺序即遍历顺序（设计 §4.1 的 `nodes()` 契约）。
///
/// **夹具按 `"c"` / `"a"` / `"b"` 登记，故意不是升序**：若按升序登记，
/// 「保持登记序」与「按 `ComputeNodeId` 升序排序」两种实现在 `nodes()` 上给出同一个结果，
/// 下面这批断言**一整批都不可区分**——那是一个等价变异体，会让「红条件」假绿。
///
/// **红的条件（档位：取反）——两个形态各拍一次，都红在同一条断言上**（实测，
/// 见 `.superpowers/sdd-p3e-impl/task-3-report.md` 的 M1 / M6）：
///
/// - `push` → `insert(0, node)`（登记序反转）→ 红在下面第一条**按序**断言（`c` 那一枚）；
/// - **`register` 末尾加一步 `sort_by(id)`**（「保持登记序」与「按 id 升序」两种实现，
///   计划把后者的写法写成「让 `nodes()` 返回排序后的副本」——**那个写法在本签名下写不出来**
///   （`nodes()` 交回 `&[ComputeNode]`，副本活不过返回），故实测取的是可表达的等价形态：
///   **在登记时就把表维护成升序**，`nodes()` 照旧交回借用）→ 同样红在那一条上。
///
/// **用例名里的「two」与夹具的三枚相抵（来历留在原地）**：本名逐字取自 E 计划 Task 3
/// （`docs/superpowers/plans/2026-10-06-p3e-compute-placement.md`，Task 3 Step 1 第一条），
/// 而它同一条的正文写的是「登记**三**枚（id `"c"` / `"a"` / `"b"`）」。**名字说二、正文说三**，
/// 是上游计划的一处笔误（本 task 按派单要求「用例名逐字照用」，故不改名；已另报协调者）。
/// 实际登记 **三**枚——三条 `id` 才能让「登记序」与「升序」在一个含逆序对的序列上分开，
/// 两枚里若恰好是升序同样不可区分。
#[test]
fn registering_two_nodes_keeps_the_registration_order() {
    let mut registry = NodeRegistry::new();
    for id in ["c", "a", "b"] {
        registry
            .register(node(id, NodeTrust::TrustedPersonal))
            .expect("首次登记不应失败");
    }

    assert_eq!(registry.nodes().len(), 3, "三枚都该在册");

    // 逐项断言，不抽代表：三条各一条，**按位置**钉住次序。
    assert_eq!(registry.nodes()[0].id(), &ComputeNodeId::new("c"));
    assert_eq!(registry.nodes()[1].id(), &ComputeNodeId::new("a"));
    assert_eq!(registry.nodes()[2].id(), &ComputeNodeId::new("b"));
}

/// 同一 `ComputeNodeId` 二次登记返回**具名**的 `Err`（设计 §4.1 判据 1、2）。
///
/// **夹具里有两个不同的 `id`（`a` 与 `b`）是承重的**：只有一个 `id` 在场时，
/// 「返回的是 `a` 那一枚」这条断言**恒真**——任何 `Duplicate { id }` 都只能是它。
/// 两个 `id` 在场，`id == ComputeNodeId::new("a")` 才是一条真断言（纪律 14）。
///
/// **红的条件（档位：移除）**：删掉 `register` 里判重那一步
/// （`if self.nodes.iter().any(…)` 那一整块）→ 第二次登记 `a` 返回 `Ok`，
/// **红在 `expect_err` 那一行**（它拿到 `Ok(())` 而 panic）。
#[test]
fn registering_the_same_id_twice_is_a_named_error() {
    let mut registry = NodeRegistry::new();
    registry
        .register(node("a", NodeTrust::TrustedPersonal))
        .expect("首次登记 a 不应失败");
    registry
        .register(node("b", NodeTrust::TrustedPersonal))
        .expect("首次登记 b 不应失败");

    let err = registry
        .register(node("a", NodeTrust::TrustedPersonal))
        .expect_err("同一 id 二次登记应失败");

    // `match` 而非 `matches!`：要**取出** id 再断言是哪一枚。
    match err {
        NodeRegistryError::Duplicate { id } => assert_eq!(id, ComputeNodeId::new("a")),
    }
}

/// **失败**的登记不改动注册表——**不静默覆盖**（设计 §4.1 判据 2）。
///
/// **这一条是「不静默覆盖」这条判据唯一的照片。** 只断「第二次返回 `Err`」是不够的：
/// 一个「**先覆盖、再返回 `Err`**」的实现会绿，而它已经把第一枚节点换掉了。
/// 故这里四处一起断：`Err` 是 `Duplicate{a}`、**枚数**、**次序**、
/// **`a` 那一枚的 `trust()` 仍是第一次给的**。
///
/// **红的条件（档位：放宽）——两个形态各拍一次，结论不同，不合并写**（实测，见
/// `.superpowers/sdd-p3e-impl/task-3-report.md` 的 M3a / M3b）：
///
/// - **覆盖并返回 `Ok`** → 红在 `expect_err` 那一行（它拿到 `Ok(())` 而 panic）。
///   **后面四条断言一条都没跑到**——「同时红」是错的说法，Rust 在该处 panic 即中止。
/// - **先覆盖、再返回 `Err`**（**正是本条要挡的形状**）→ `expect_err` 绿、`id` 绿、
///   枚数 2 绿、**次序也绿**（就地改写不动位置），**只有 `trust()` 那一条红**。
///
/// 故**承重的只有 `trust()` 那一条**：它是上一条形态下本测试体内**唯一**失败的断言
/// （纪律 2 的强版本：B 有独立照片 ⟺ ∃M 使 B 是该测试体内唯一失败的断言）。
/// **枚数与次序那两条钉的是**另一条**子句，不是本条的「不静默覆盖」**，据实记：
/// 就地覆盖不动位置，故它们在本条要挡的那个形态下**不红**。
/// 而它们**各有自己的照片**——只是照片落在**另一形状**的失败登记上，两枚都是 `e-t3-rev`
/// 评审构造、本 task **自己复跑过**的：
/// **M7**（先入册、再返回 `Err`：`push` 挪到 `return Err` 之前）→ **只有枚数那一条单独红**；
/// **M8**（判重时就地轮转：`remove(index)` ＋ `push(旧的那一枚)` 再返回 `Err`）→
/// **只有次序那一条单独红**（枚数仍 2、`trust()` 仍 `TrustedPersonal`，都绿）。
/// 故**它们各自承重**，承的是「失败的登记不增删节点」与「失败的登记不动位置」这两条子句。
///
/// **初稿那句「举不出让它们单独红的 M」是错的**（订正于修复轮 1，来历留在原地）：
/// 事实句是真的——**本 task 原先那 11 个变异体里，两枚都单独红不了**
/// （M1 的 `insert(0, …)` 与 M6 的 `sort_by(id)` 都同时红了
/// `registering_two_nodes_keeps_the_registration_order`）；**错的是紧跟的那句推论**——
/// 它把射程丢掉、由「**我没试出来**」跳到了「**不存在**」。**这两件事不同**：
/// 前者是「试过的形态不够」，后者才是「没有照片」，而后者需要把**所有**形态走遍才能说。
/// 评审造出的 M7／M8 两枚即是反例，**故本段只写「本 task 原先那 11 个变异体里，两枚都单独红不了」**（引号内是本段实句；初稿在此引的「试过的那些形态里没有」**本段没有**，修复轮 2 订正）。
#[test]
fn a_failed_registration_does_not_overwrite() {
    let mut registry = NodeRegistry::new();
    registry
        .register(node("a", NodeTrust::TrustedPersonal))
        .expect("首次登记 a 不应失败");
    registry
        .register(node("b", NodeTrust::TrustedPersonal))
        .expect("首次登记 b 不应失败");

    let err = registry
        .register(node("a", NodeTrust::OutsidePersonalTrustDomain))
        .expect_err("同一 id 二次登记应失败，且不得静默覆盖");
    match err {
        NodeRegistryError::Duplicate { id } => assert_eq!(id, ComputeNodeId::new("a")),
    }

    assert_eq!(registry.nodes().len(), 2, "失败的登记不该增删节点");
    assert_eq!(registry.nodes()[0].id(), &ComputeNodeId::new("a"));
    assert_eq!(registry.nodes()[1].id(), &ComputeNodeId::new("b"));
    assert_eq!(
        registry.nodes()[0].trust(),
        NodeTrust::TrustedPersonal,
        "在册的那一枚必须还是**第一次**登记的那一枚（trust = TrustedPersonal），\
         不是被第二次那枚（trust = OutsidePersonalTrustDomain）顶掉的"
    );
}

/// 读 `src/registry.rs` 的全文。**路径由 `CARGO_MANIFEST_DIR` 拼出**，
/// 不用相对路径——测试进程的 cwd 由 cargo 决定，写成相对路径会在别处静默读空。
fn registry_module_source() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/registry.rs");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("读不到 {}：{e}", path.display()))
}

/// 被禁的三个名字，**字面拼法**。**这是结论那条用例读的唯一一份清单。**
///
/// 正控制**不从这里取名字**，而是在 [`PROBE`] 里把这三种拼法**独立地再写一遍**——
/// 「同一份清单喂给正控制」看似省一处重复，实际会让正控制**恒真**：
/// 语料里的名字跟着清单一起坏，扫不中与扫得中永远同步（本 task 实测过，见 [`PROBE`] 的说明）。
const BANNED_NAMES: [&str; 3] = ["continuum_artifact", "Artifact", "PrivacyClass"];

/// 扫一份源码文本，交回每一处命中的 `"<行号>: <名字>: <该行原文>"`。
///
/// 行号从 **1** 起（与编辑器一致）。**这一条是唯一一处数行号的地方**，
/// 两条用例共用它——正控制与结论必须走同一条扫法，否则正控制证明的不是结论那条路。
fn hits(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (index, line) in source.lines().enumerate() {
        for name in BANNED_NAMES {
            if line.contains(name) {
                found.push(format!("{}: {name}: {}", index + 1, line.trim()));
            }
        }
    }
    found
}

/// **源码文本守卫**：`src/registry.rs` 的全文里不出现那三个名字的字面拼法。
///
/// 为什么不能出现、以及这条守卫的证明力边界（**下界，不是封闭判定**；
/// 封闭的那一层是 `ALLOWED` 的逐对断言），逐条写在**本文件头**——不写在
/// `src/registry.rs`，理由也在那里。
///
/// **红的条件（档位：取反）**：在 `src/registry.rs` 里加一行
/// `use continuum_artifact::Artifact;` → 本用例红，断言信息里报出该行的行号。
/// **本 task 实施期真做过一次**（临时加、跑、确认红且报出行号、再删，
/// 前后 `sha256` 与日志见 `.superpowers/sdd-p3e-impl/task-3-report.md`）。
#[test]
fn the_registry_module_names_no_artifact_type() {
    let source = registry_module_source();
    let found = hits(&source);

    assert!(
        found.is_empty(),
        "src/registry.rs 出现了被禁的 artifact 名字（下界守卫：字面拼法）——\
         注册表是入度为零的组件，放行裁决归 §5 的放置，不归这里。命中处：\n{}",
        found.join("\n")
    );
}

/// 正控制语料：**把三个名字的字面拼法在这里独立地再写一遍**，不从 [`BANNED_NAMES`] 里取。
///
/// **这一份「独立再写一遍」就是第 2 侧的全部证明力**：两处各写一份，任一处被改坏，两条断言就对不上。
///
/// **不得从清单里取名字来拼语料**——那是**清单自证清单**，一个恒真的假照片：
/// 语料里的名字跟着清单一起坏，于是扫不中与扫得中永远同步。本 task 实测过这一版，
/// **把清单里任一枚改掉（含 `Artifact` → `Artifct` 这类真拼错），三枚各自都照样绿**
/// （M5a / M5b / M5c，见 `.superpowers/sdd-p3e-impl/task-3-report.md` 的「作废的尝试」）。
///
/// 第 2 行含**两枚**（`continuum_artifact` 与 `Artifact`），第 3 行含 `PrivacyClass`——
/// 故意让命中**跨两行**，这样正控制连**行号**一起钉住，不只是「命中了没有」。
const PROBE: &str = "pub struct NodeRegistry;\n\
                     use continuum_artifact::Artifact;\n\
                     let level: PrivacyClass;\n";

/// **守卫自身的正控制**：先钉机制，再看结论。
///
/// 没有这一条，一个「什么都没读到」的守卫（路径写错、文件读成空串）会**永远绿**——
/// 上面那条用例的绿就成了一句没有来历的话。故这里钉**两侧**：
///
/// 1. **读得到**：`src/registry.rs` 非空，且找得到 `pub struct NodeRegistry` 的拼法；
/// 2. **扫得中**：把与清单**互相独立**的 [`PROBE`] 喂进同一套扫法——
///    命中数要对（3 处）、行号要对（第 2 / 2 / 3 行）、且清单里每一枚都要能在语料里找到。
///    **三条各管一头，不重复**：命中数钉「清单**漏**枚」；逐枚 `contains` 钉
///    「清单里每一枚都还扫得中语料里那一枚」——**清单多出**一枚时只有它能红
///    （多出来那枚扫不中语料，而命中数仍是 3）；行号钉「报的位置是真的」。
///
/// **红的情形（本用例的红不是「被测对象坏了」，而是「尺子坏了」）**，逐档实测：
///
/// - 某枚被改成**扫不中它本来拼法**的拼法（`Artifact` → `Artifct`；`continuum_artifact`
///   → `continuum_artifct`）→ 第 2 侧**命中数**那条断言红，信息里报出实得的 2 处；
/// - 清单**少一枚**（三枚 → 两枚）→ 同上红；
/// - 清单**多一枚**（三枚 → 四枚）→ 红在**逐枚 `contains`** 那一条上，命中数仍是 3；
/// - `registry_module_source` 的路径改错、或 `src/registry.rs` 变空 → 第 1 侧红。
///
/// **一条它抓不到的，据实记**：把某一枚**截短成真拼法的前缀**（`PrivacyClass` →
/// `PrivacyClas`）→ 命中数仍是 3、逐枚 `contains` 仍真、**本用例绿**。
/// **这不是漏**：前缀是**超集**，真拼法的每一处仍被扫中，故守卫不会漏报
/// （只会多报，方向是 fail-closed）——**这一句是逻辑蕴含**（超集的定义），
/// 故没有也不该有照片。**它是本正控制的能力边界**：它钉的是「清单不会 fail-open」，
/// 不是「清单与某个理想值逐字相等」。
///
/// **它不因 `src/registry.rs` 里加了内容而红**（除非加的内容让文件读不出来）。
#[test]
fn the_guard_sees_the_module() {
    let source = registry_module_source();

    assert!(!source.is_empty(), "src/registry.rs 读成了空串——尺子没量到东西");
    assert!(
        source.contains("pub struct NodeRegistry"),
        "src/registry.rs 里找不到 `pub struct NodeRegistry` 的拼法——\
         要么路径读的不是那份文件，要么注册表已经不在这个模块里了"
    );

    // 第 2 侧。
    let probe_hits = hits(PROBE);
    assert_eq!(
        probe_hits.len(),
        3,
        "正控制语料里逐字写了三枚被禁名字（第 2 行两枚、第 3 行一枚），应恰有 3 处命中，\
         实得 {probe_hits:?}——清单漏了一枚，或某一枚的拼法与语料里的对不上"
    );
    let probe_lines: Vec<&str> = probe_hits
        .iter()
        .map(|hit| hit.split(':').next().unwrap_or("?"))
        .collect();
    assert_eq!(
        probe_lines,
        ["2", "2", "3"],
        "命中应报在第 2 / 2 / 3 行，实得 {probe_hits:?}——行号算错了"
    );
    for name in BANNED_NAMES {
        assert!(
            PROBE.contains(name),
            "{name:?} 在清单里，却扫不中正控制语料里那一枚——\
             语料里写的是 `continuum_artifact` / `Artifact` / `PrivacyClass` 三条字面拼法，\
             这一枚在清单里的拼法与它不一致，结论那条用例对它等于没设防"
        );
    }
}
