use continuum_graph::{AdfirGraph, NodeId, SchedulerConfig};

/// §303 的 Graph Scheduler 消费入口。**零逻辑转调**：READY／并发判定全部在
/// `continuum_graph::select_runnable`。本函数不读 `NodeState`、不遍历边、不判前驱、
/// 不截断、不排序、不做状态迁移；`SchedulerConfig.max_parallel` 原样传入，本块不解释它。
pub fn runnable_nodes(graph: &AdfirGraph, config: &SchedulerConfig) -> Vec<NodeId> {
    continuum_graph::select_runnable(graph, config)
}
