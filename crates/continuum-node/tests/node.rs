//! `ComputeNode` 与它的三个取值类型（设计 §3、需求 §287）。
//!
//! 本文件的四条用例分两类，**红形态不同，不能混读**：
//!
//! - 前两条是**运行期**照片：`new` 收什么、三条访问器交回什么，以及 `ComputeNodeId`
//!   的序。它们的红来自断言失败。
//! - 后两条是**编译期**照片（`class_label` / `trust_label` 那两个无通配臂的 `match`）：
//!   钉的是「两个枚举各只有两枚」这条**枚举定义的封闭性**。它们的红形态是**编译失败**
//!   （给枚举加第三枚 → 非穷尽 `match`），**不是断言失败**——故函数体里**一条运行期断言
//!   都没有**。写 `assert_ne!(NodeClass::Personal, NodeClass::Temporary)` 之类的断言在 Rust 里
//!   **恒真**（枚举两枚各自是一个值），那是**假照片**：它的红只会来自改动本文件自己写的那个
//!   `match`，与 crate 无关。同 G 的 `assert_send` 那条的处置。
//!
//! 两条编译期照片**各写一遍、不合并**：`NodeClass` 与 `NodeTrust` 能各自漂移，
//! 一个 `match` 覆盖不了另一个。

use continuum_node::{ComputeNode, ComputeNodeId, NodeClass, NodeTrust};

/// §287 的 `class`：`PERSONAL | TEMPORARY`，逐字照录（设计 §3.2）。
///
/// **两臂、无通配臂**——这就是本文件要钉的那条编译期性质本身：给 `NodeClass` 加第三枚时，
/// 这个 `match` 会以「非穷尽 match」编不过（常见码 `E0004`，**以实跑为准**）。
fn class_label(c: NodeClass) -> &'static str {
    match c {
        NodeClass::Personal => "PERSONAL",
        NodeClass::Temporary => "TEMPORARY",
    }
}

/// 节点在信任域中的位置。**取值域是本设计定的，不是规范的形状**（设计 §3.3：
/// §287 只给字段名、§94 只给 "Node trust class" 这个词）——故两个标签取本设计的名字，
/// 刻意**不**冒用规范里的字面，与上面 `class_label` 照录 §287 的写法相对照。
///
/// 同样**两臂、无通配臂**，理由同上。若规范后来给出更细的分级，这里会先编不过——
/// 而按设计 §3.3 限度 1，那时**放行方向也要重新逐值归类**（§5.4 通道 (a)），
/// 故这一处「编不过」正是一个**该停下来重读规范**的信号，不是一条要顺手补上的机械修复。
fn trust_label(t: NodeTrust) -> &'static str {
    match t {
        NodeTrust::TrustedPersonal => "TrustedPersonal",
        NodeTrust::OutsidePersonalTrustDomain => "OutsidePersonalTrustDomain",
    }
}

/// 夹具：六项**两两可分辨**（设计 §3.1 的六个字段）。
///
/// **三组 `Vec<String>` 取两两不同的值是承重的**，不是随手写的：若三者取相等的值
/// （例如都取 `["x"]`），把 `new` 里 `resources` 与 `availability` 两个赋值**互换**
/// 就**不产生任何可观察差别**——那是一个**等价变异体**，下面第一条用例会**假绿**。
/// 「变异须真落到实现体」那条纪律在本 task 的落法就是这三组值。
fn fixture() -> ComputeNode {
    ComputeNode::new(
        ComputeNodeId::new("n1"),
        NodeClass::Personal,
        NodeTrust::TrustedPersonal,
        vec!["cap".to_string()],
        vec!["res".to_string()],
        vec!["avail".to_string()],
    )
}

/// 六项**逐条**断言，不抽代表：`id` / `class` / `trust` / `capabilities` / `resources` /
/// `availability` 各一条，六条都在这里，没有一条靠另一条代表。
///
/// **红的条件（档位：取反）**：把 `new` 里 `resources` 与 `availability` 两个赋值**互换**
/// （两者同为 `Vec<String>`，**互换后照样编译**）→ `resources` 与 `availability` 那两条断言红。
#[test]
fn the_six_accessors_return_what_new_was_given() {
    let node = fixture();

    assert_eq!(node.id(), &ComputeNodeId::new("n1"));
    assert_eq!(node.class(), NodeClass::Personal);
    assert_eq!(node.trust(), NodeTrust::TrustedPersonal);
    assert_eq!(node.capabilities(), ["cap".to_string()].as_slice());
    assert_eq!(node.resources(), ["res".to_string()].as_slice());
    assert_eq!(node.availability(), ["avail".to_string()].as_slice());
}

/// `ComputeNodeId` 是注册表的键（设计 §3.6），也是放置结果的身份；它**可比较且大小有意义**。
///
/// **消费方是 Task 6 的兜底档**（设计 §5.9：`a.id().cmp(b.id())` 是排序的最后一道），
/// 故这条不是提前铺开的 API。
///
/// **红的条件（档位：取反）**：把 `Ord` 改成按**长度**比、或改成反转
/// （如 `other.0.cmp(&self.0)` 手写 `impl Ord`）→ 第一条断言红。
/// 第二条（`new("a") == new("a")`）钉的是 `Eq` 那一半，与 `Ord` 是两个 trait。
#[test]
fn the_id_is_the_registry_key_and_is_ordered() {
    assert!(ComputeNodeId::new("a") < ComputeNodeId::new("b"));
    assert_eq!(ComputeNodeId::new("a"), ComputeNodeId::new("a"));
}

/// `NodeClass` 只有 §287 给的那两枚（设计 §3.2、需求 §287 的 `PERSONAL | TEMPORARY`）。
///
/// **判据是「本文件编得过」**，不是本函数跑出了什么：`class_label` 是一个覆盖两臂、
/// **无通配臂**的 `match`，给 `NodeClass` 加第三枚时它以「非穷尽 match」编不过。
/// 故下面两次调用只是**让两个臂的函数体各自被类型检查**，没有断言，也**不该有**断言
/// （理由见文件头：「只有两枚」是编译期性质，运行期断言恒真）。
#[test]
fn the_two_node_classes_are_the_two_the_spec_names() {
    let _ = class_label(NodeClass::Personal);
    let _ = class_label(NodeClass::Temporary);
}

/// `NodeTrust` 只有**本设计**取的那两枚（设计 §3.3）。
///
/// 用例名刻意与上面那条**不对称**：`NodeClass` 的取值域**是规范给的**（§287 明写两枚），
/// 故那条的名字说 "the_spec_names"；`NodeTrust` 的取值域**是本设计定的**（§287 只给字段名），
/// 故这条的名字说 "this_design_names"。名字也是一种断言，两者的声称要与其来历相等。
///
/// 判据与红的形态同上面那条：**本文件编得过**，红形态是**编译失败**。
#[test]
fn the_two_node_trusts_are_the_two_this_design_names() {
    let _ = trust_label(NodeTrust::TrustedPersonal);
    let _ = trust_label(NodeTrust::OutsidePersonalTrustDomain);
}
