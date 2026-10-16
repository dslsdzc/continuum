//! `MethodEntry`：一条方法（设计 §4.3）。

use crate::{MethodDomain, MethodId};

/// 一条方法。**它是索引，不是知识的承载者**（设计 §4.3 末）——「这一类任务怎样
/// 做得可靠」的内容落在各领域的算子定义上，本条目只登记「有什么方法、归哪个
/// 领域、由哪些算子兑现」。
///
/// # 字段集恰为三个（全 `pub`）
///
/// - `id`：§187 给的方法名（[`MethodId`]）；
/// - `domain`：它属于哪份目录（[`MethodDomain`]）；
/// - `realized_by`：实现该方法的算子 id，**以文本登记**，本 crate 不解析
///   （设计 §3.2——「依赖集为空集」的兑现方式；写错的文本照样编得过，核对责任
///   在消费块）。
///
/// # 不带的字段与理由
///
/// - **不带** `input_schema` / `output_schema` / `determinism` / `side_effect_class` /
///   `backend_candidates`：那些是 `Operator` 的字段
///   （`crates/continuum-operator/src/definition.rs:79`），一条 Method 不是 Operator
///   （设计 §4.3）。
/// - **不带 `purpose`**（「怎样做得可靠」的文本承载）：设计 §4.3 三条理由——
///   §187 只给名、不给说明（无规范来源）；没有写入路径的字段就是死字段
///   （唯一填充入口 `bind` 只碰 `realized_by`）；留一个填得进去的口等于让四个
///   算子块各自发明语义。**本设计近期删过这个字段，此处的三字段是删后的形状。**
///
/// # 派生集（`Debug, Clone, PartialEq, Eq`）
///
/// 照 `Operator`（`crates/continuum-operator/src/definition.rs:78`）的派生集去掉
/// serde（本 crate 不序列化任何东西）。设计 §4.3 的签名列没有列派生——**本计划
/// 自定**，申报见实现计划的 `## 遗留` 第三节。
///
/// # 字段集的照片是编译期照片
///
/// `tests/registry.rs` 的 `method_entry_has_exactly_three_public_fields` 用
/// **不带 `..` 的结构体字面量**钉住字段集（纪律 5）：加第四个字段即编译失败
/// （缺字段 `E0063`）、删一个字段同样编译失败（多出的字段名 `E0560`）——
/// **这是编译期照片，不是运行期红**（纪律 1(c)）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodEntry {
    /// 方法的标识（设计 §4.2）。
    pub id: MethodId,
    /// 它属于哪份目录（设计 §4.1）。
    pub domain: MethodDomain,
    /// 实现该方法的算子 id，**以文本登记**，本 crate 不解析（设计 §3.2）。
    pub realized_by: Vec<String>,
}
