// 应编译失败：`ComputeNode` 的六个字段私有，故**全字段结构体字面量**这条路在 crate 外走不通
// （预期 E0451）。**文件名即这条通道——本样例只钉结构体字面量这一条**。
//
// **它不声称「crate 外构造不出 `ComputeNode`」**：`ComputeNode::new` 是 `pub fn`
// （`src/node.rs`，设计 §3.5 自己就写的 `pub`），crate 外经它就能拿到一枚节点；
// 被挡住的只是「绕过 `new` 自己拼一个」。设计 §9 那一格的**标签**一度写成「不可外部构造」，
// 与同格**正文**及 §3.5 相抵，该标签已在 `3a7ddad` 收到正文的口径。
//
// 为什么这条通道值得钉（设计 §3.5）：`ComputeNode` 要进放置的硬闸门，而闸门只读 `class` 与
// `trust`。六个 `pub` 字段会让「闸门读到的 `class`」与「构造时给的 `class`」之间没有任何一处
// 可挂不变量；私有字段加访问器把这两处收成一条路径。**代价照实记**：今天没有任何不变量可挂，
// 故这一处私有性的收益是**结构性的**（将来加字段不许外部直接写字面量），不是今天可观察的。
use continuum_node::{ComputeNode, ComputeNodeId, NodeClass, NodeTrust};

fn build() -> ComputeNode {
    ComputeNode {
        id: ComputeNodeId::new("n1"),
        class: NodeClass::Personal,
        trust: NodeTrust::TrustedPersonal,
        capabilities: Vec::new(),
        resources: Vec::new(),
        availability: Vec::new(),
    }
}

fn main() {
    // 取一次函数项，免得 `build` 触发 `dead_code` 警告混进本样例的 `.stderr`；
    // 函数体照常被类型检查，故上面那次字面量仍是本样例要的那次解析。
    let _ = build;
}
