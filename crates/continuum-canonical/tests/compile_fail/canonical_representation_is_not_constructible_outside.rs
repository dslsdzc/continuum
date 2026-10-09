// 应编译失败：`CanonicalRepresentation` 的字段**私有**，且唯一构造入口
// `CanonicalRepresentation::new` 是 `pub(crate)`（`src/canonical.rs`）
// ——故 crate 外拼不出结构体字面量。
//
// 这份样例是 **Task 2 转来的 P-2-1**（`docs/superpowers/p4-followups.md`）：
// 那条转账写的是「`CanonicalRepresentation` 的 crate 外不可构造性今天无照片」，
// 它的那半张照片要 `trybuild`，而 `trybuild` 这个 dev-dep 是 **Task 3** 才引入的。
//
// **它钉的是「值这一层造不出 `Canonical` 里带着未解 mention 的值」那半件事**
// （§3.2 的「两支互斥」）：`CanonicalRepresentation` 只能由 `crate::finish` 造出，
// 而 `finish` 那一支里已经确认过「每一枚都是 `Resolved`」。
// 运行期那一半（恰有一支成立）由 `tests/canonical.rs::the_two_arms_are_complements` 钉。
//
// **本样例不声称「crate 外拿不到 `CanonicalRepresentation`」**：经 `finish` 拿得到
// （`Canonicalized::Canonical(..)` 那一支就带着它），`mentions()` 也读得到。
// 被挡住的只是「绕过 `finish` 自己拼一个」。
use continuum_canonical::CanonicalRepresentation;

fn main() {
    let _ = CanonicalRepresentation {
        mentions: Vec::new(),
    };
}
