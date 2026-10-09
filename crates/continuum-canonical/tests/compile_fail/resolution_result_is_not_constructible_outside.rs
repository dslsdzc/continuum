// 应编译失败：四个载荷类型（`Resolved` / `ConfirmRequired` / `Ambiguous` / `Unknown`）
// 的字段**私有**、且没有公开构造函数（`src/resolver.rs` 的文件头写了为什么）
// ——故 crate 外拼不出任何一个载荷，也就**造不出一枚 `ResolutionResult` 值**：
// 「一枚判定结果是不是 `resolve` 产出的」因此有**类型判据**。
//
// **四支逐支各写一次**（不是「写一支代表四支」）：`ResolutionResult` 有四枚变体，
// 「造不出一枚 `ResolutionResult`」这句话要对**每一支**都取到照片——
// 一句话比材料宽，本仓的判据是「绝对措辞须有用例」。实跑得四段
// `E0451: fields … are private`（`.stderr` 是实跑取值）。
// 那四支的**字段清单**（逐支穷尽解构、不带 `..`）另由
// `src/resolver.rs` 的 `#[cfg(test)]::every_arm_carries_exactly_its_declared_fields` 钉。
//
// **正控制**：`tests/resolver.rs` 与 `src/resolver.rs` 的 `#[cfg(test)]` 里那些
// **经 `resolve` 拿到载荷**的用例（各自跑得绿）。没有它们，本样例的绿可以来自
// 「`Resolved` 这个名字整个不存在」这类与私有性无关的原因（本仓「正控制不得自证」）。
//
// **本样例不声称「crate 外拿不到 `ResolutionResult`」**：经 `resolve` 拿得到，
// 经只读访问器也读得到（`tests/resolver.rs` 就是这么读的）。被挡住的只是「自己拼一个」。
use continuum_canonical::{
    Ambiguous, ConfirmRequired, EntityId, ResolutionResult, Resolved, Unknown, UnknownReason,
};

fn main() {
    // —— `Resolved`：三条字段 ——
    let _ = ResolutionResult::Resolved(Resolved {
        entity: EntityId::from_str("e"),
        satisfied: Vec::new(),
        candidate_count: 1,
    });

    // —— `ConfirmRequired`：三条字段 ——
    let _ = ResolutionResult::ConfirmRequired(ConfirmRequired {
        first: EntityId::from_str("e"),
        surviving: Vec::new(),
        candidate_count: 1,
    });

    // —— `Ambiguous`：两条字段 ——
    let _ = ResolutionResult::Ambiguous(Ambiguous {
        surviving: Vec::new(),
        candidate_count: 2,
    });

    // —— `Unknown`：两条字段 ——
    let _ = ResolutionResult::Unknown(Unknown {
        reason: UnknownReason::NoCandidate,
        candidate_count: 0,
    });
}
