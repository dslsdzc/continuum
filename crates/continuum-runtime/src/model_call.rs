//! 模型调用路径（设计 `docs/superpowers/specs/2026-10-06-p3g-model-calling-path-design.md`）
//! 里**不依赖四段流程**的那部分：`ProviderError` 的分类表，以及它到
//! [`ModelCallError`] 的转换。
//!
//! 本 task 落在这里的就是上面那两样：取快照、调排序、`call`、`call_stream`
//! 四段流程由后续 task 落在这里。
//!
//! [`ModelCallError`] 的定义在 [`crate::error`]——那是设计 §10.2 指定的落点
//! （该文件由 F 创建，G 只增补），与把分类表放在这里并不冲突：**表是 G 的增量，
//! 而那个文件是两个子项目共写的**。

use continuum_core::ProviderError;
use continuum_graph::failure::FailureClass;

use crate::error::ModelCallError;

/// 设计 §6.1 的分类表：**G 这一侧的默认口径，不是对适配器的断言**。
///
/// 适配器要对某次失败改口径，应当在 `ProviderError` 的**取值上**表达
/// （明知永久就报 `Protocol`），而不是指望这里为它开一个口子。
///
/// # 这张表不写成边界上的函数
///
/// C §5.1 拒掉过那件事，理由在本路径上**同样成立**：`Transport` 在一个适配器上可能是
/// 瞬时的、在另一个上可能是永久的，**故这张表是否为真，本子项目答不出**（设计 §6.2）。
/// 它是 G 产出类别时的默认读法；`ProviderError` 的取值域不因它而收窄，本表也不是
/// 适配器必须服从的契约。
///
/// # 一条已知的射程边界（据实记，不是漏项）
///
/// 既有夹具把**永久性配置缺陷**（「这个 provider 没有这个工具」）报成 `Unavailable`：
/// `crates/continuum-provider/tests/common/mod.rs:177` 的 `FakeTool::describe_tool`
/// ——`ok_or_else(|| ProviderError::Unavailable(id.as_str().to_owned()))`。
/// **若模型侧的真实适配器也这样用 `Unavailable`，本表会把一次配置缺陷分成 `Transient`。**
/// **修它在适配器，不在这里。**
///
/// **坐标订正（2026-10-07，原话照留）**：设计 §6.2 与本计划的这一条引的是
/// `crates/continuum-provider/tests/fake_provider.rs:106`，**那个坐标在本树上已失效**——
/// 实读 `wc -l crates/continuum-provider/tests/fake_provider.rs` 得 **104 行**（即 `:106`
/// 越界），且该文件里的**夹具**今天只剩 `FakeConnector`（`fake_provider.rs:13`，另有
/// `mod common;` 与两条用例）；工具侧夹具由 C 的 Task 2 整块搬进了 `tests/common/mod.rs`
/// （搬家的记录就在被搬去的那份文档注释里）。上文那个行号是本条**实读后**的坐标。
///
/// # `FailureClass::Resource` 这一格无输入
///
/// `ProviderError` 没有「限流 / 配额耗尽」的变体（C §12 第 7 条），故下面**不写
/// `Resource` 的臂**——一个没有输入的臂是假接口。**G 不擅自扩 `continuum-core`
/// 的取值域**：那是 §315 的接口类型。
///
/// 这句话的照片是 `tests/model_call.rs` 的 `each_provider_error_variant_maps_to_its_class`：
/// 五个 `ProviderError` 变体的像被**逐项钉死**（四枚失败变体各得一个写死的类别），
/// 故往这张表的任何一格塞 `Resource` 都会让那五条断言里的某一条红。
///
/// # 返回 `Option` 的那一格
///
/// **`None` 只对 `ProviderError::Cancelled` 返回**：**它是调用方自己发的取消，不是失败**
/// （设计 §6.1）。`None` 在本函数里**不是**「不知道」——「不知道」有
/// [`FailureClass::Unknown`] 这一格，而本表不给任何变体用它：**照片与上面那张
/// `Resource` 的照片是同一条**（`each_provider_error_variant_maps_to_its_class` 把
/// 五个变体的像逐项写死，其中没有 `Unknown`）。
/// 「`None` 只对 `Cancelled` 返回」的两侧照片在 `a_cancelled_call_is_not_a_failure`。
pub fn classify(e: &ProviderError) -> Option<FailureClass> {
    match e {
        ProviderError::Transport(_) => Some(FailureClass::Transient),
        ProviderError::Unavailable(_) => Some(FailureClass::Transient),
        ProviderError::Protocol(_) => Some(FailureClass::Permanent),
        ProviderError::UnknownModel(_) => Some(FailureClass::Permanent),
        ProviderError::Cancelled(_) => None,
    }
}

/// `ProviderError` → [`ModelCallError`] 的转换点：`call` / `call_stream` / `abort`
/// 共用它，**各自不再自己映射一遍**（设计 §6.1 的落点）。
///
/// **「共用」是设计要求，今天没有照片**：那三处由 Task 11 落地，本 task 里它们还不存在，
/// 故现在钉不了「别处不再自己映射」——**本 crate 今天的生产代码里本函数没有调用方**。
/// 等那三处成形，这张照片应当是一条源码文本判据（`ModelCallError::Provider` 的构造点
/// 在本 crate 的 `src/` 里只有一处），本 task 不代写。
///
/// **它只问 [`classify`] 一次**：设计 §6.1 的落点段写明「**`Cancelled` 不落在『失败』
/// 那一枚里**」正是**分类表那一行**（`Cancelled(_)` → 不进入分类）落到本类型上的结果，
/// 故这两件事是同一个决定的两种投影，分两处写会让它们漂移——一处说「不是失败」，
/// 另一处却给它一个 `class`。**代价据实记**：表一改，本函数的产出形状跟着改
/// （把 `Cancelled` 归成某一类，取消就会被记成一次该类的失败），这正是两条用例
/// 要在表的**两处**钉死它的原因。
pub fn into_call_error(e: ProviderError) -> ModelCallError {
    match classify(&e) {
        Some(class) => ModelCallError::Provider { class, source: e },
        None => ModelCallError::Cancelled { source: e },
    }
}
