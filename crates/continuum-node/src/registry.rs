//! §4 的进程内 Compute Node 注册表（设计 §4.1）。
//!
//! **本模块的源码文本受一条守卫约束**（`crates/continuum-node/tests/registry.rs` 的
//! `the_registry_module_names_no_artifact_type`，以及它旁边的正控制
//! `the_guard_sees_the_module`）。**该守卫的判据、它为什么存在、以及它的证明力边界
//! （下界而非封闭判定），逐条写在那个测试文件的文件头**——**不写在这里**：
//! 守卫匹配的是本文件的**全文**，包括注释，把说明写进来会**命中它自己**。
//! 本模块只留这一句指向。

use crate::node::{ComputeNode, ComputeNodeId};

/// 登记失败的原因。
///
/// **只派生 `Debug` 与 `thiserror::Error`，不派生 `PartialEq`**（设计 §4.1）：
/// 用例一律用 `match` 取出 `id` 再断言是哪一枚；派生 `PartialEq` 会把
/// 「两枚错误相等」变成一条**本层没有判据**的命题——本层从不比较两枚错误。
///
/// **`Debug` 是必需的、不是装饰**：它是失败路径上 `expect` / `expect_err` / `unwrap`
/// 的签名所要求的（`T: Debug` / `E: Debug`），也是断言红时能读出内容的前提。
///
/// 形状沿用本仓已有的两个注册表（`ProviderRegistry` 与
/// `crates/continuum-operator/src/registry.rs` 的 `Duplicate { id, version }`）——
/// **沿用的是这个形状（具名变体、不静默），不是它的字段数**：本设计的键只有
/// [`ComputeNodeId`] 一个，故变体只有一个字段。
#[derive(Debug, thiserror::Error)]
pub enum NodeRegistryError {
    /// 同一个 [`ComputeNodeId`] 已经登记过。注册表**不静默覆盖**，
    /// 故新节点既不进册、也不动在册的那一枚（设计 §4.1 判据 2）。
    #[error("计算节点 {id:?} 已登记，注册表不静默覆盖")]
    Duplicate { id: ComputeNodeId },
}

/// 已注册的计算节点。**登记顺序即遍历顺序**（设计 §4.1）。
///
/// # 不落库：这是设计决定的，代价照实记
///
/// 本类型只在进程内（设计 §4.2 的三条判据：规范没有一句话使已登记的节点成为持久事实；
/// 持久事实的来源是 Authority，不在本子项目范围；一张表最终还是要在进程内再建一次映射）。
/// 故**本 crate 不取任何迁移号**。
///
/// **代价**：**进程重启后节点集合为空**，`place` 在重启后只会返回
/// `NoPlaceableNode`，直到有人重新登记。今天的唯一登记方是测试，故这一条
/// **没有可观察形态**。
///
/// # 不建的东西
///
/// 不建 `get` / `deregister` / `len` / `is_empty`——**零消费方**
/// （`deregister` 另有一层：按 §292 撤销归 Authority）。也不建节点状态刷新或心跳：
/// 那会让 `availability` 从「只搬运」变成「会被改写」，而 §291 的 `current load`
/// 因此连「会变」都不成立。
///
/// 派生 `Debug` 与同族的 `OperatorRegistry` 同形；**不派生 `Clone` / `PartialEq`**——
/// 今天零消费方。
///
/// # 「零消费方」与「唯一登记方是测试」这两句话的照片
///
/// 本段与上面两处用到**零消费方**、**唯一登记方是测试**这类**对缺席的断言**。
/// **它们的判据是全仓 grep，不是用例**（本 task 实测，2026-10-07：
/// `grep -rn NodeRegistry --include='*.rs' crates/` 在 `crates/continuum-node/` 之外**零命中**；
/// 本 crate 内的三处是 `lib.rs` 的导出、本文件、与 `tests/registry.rs`）。
/// **不补用例**，理由与 `src/node.rs` 里 `capabilities` 那条相同：对**缺席**设守卫要自己
/// 先自证守卫两侧都动得起来，成本与收益不成比例。
/// **将来出现第一个非测试调用方时，这句话要重取。**
#[derive(Debug)]
pub struct NodeRegistry {
    /// 登记顺序即遍历顺序；键是 [`ComputeNode::id`]，但它**不是**一个 `HashMap`：
    /// 遍历顺序是契约的一部分，而哈希表的顺序不是。
    nodes: Vec<ComputeNode>,
}

impl NodeRegistry {
    /// 空注册表。
    ///
    /// **不派生 `Default`**：零消费方——今天没有一处需要一个默认构造的注册表
    /// （唯一构造的人是登记方自己）。
    ///
    /// **代价照实记**：clippy 会为它报 `clippy::new_without_default`
    /// （实测 `cargo clippy -p continuum-node --all-targets` 得 1 条 warning）。
    /// **本仓的门禁是 `cargo test` ＋ `cargo build`，不含 clippy**，故这条不进门；
    /// **将来若把 clippy 纳入门禁，它会立刻变成一条红**——那时要么补 `Default`，
    /// 要么把「零消费方」这条判据重新核一遍。
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// 登记一枚节点。同一 [`ComputeNodeId`] 二次登记返回
    /// [`NodeRegistryError::Duplicate`]，**不静默覆盖**：失败的登记既不增删节点，
    /// 也不改动在册的那一枚。
    pub fn register(&mut self, node: ComputeNode) -> Result<(), NodeRegistryError> {
        if self.nodes.iter().any(|existing| existing.id() == node.id()) {
            return Err(NodeRegistryError::Duplicate {
                id: node.id().clone(),
            });
        }
        self.nodes.push(node);
        Ok(())
    }

    /// 在册节点，**按登记顺序**。
    ///
    /// 消费方是 `place` 的**调用方**（它把这一份填进请求面），不是本 crate 内部——
    /// 本 crate 不持有「从注册表直接放置」的旁路：注册是事实来源，放置是纯函数，
    /// 两者不在同一处收口（设计 §4.1 判据 3）。
    pub fn nodes(&self) -> &[ComputeNode] {
        &self.nodes
    }
}
