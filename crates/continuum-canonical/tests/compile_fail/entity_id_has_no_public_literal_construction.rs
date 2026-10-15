// 应编译失败：`EntityId` 的里子串**私有**，故**元组结构体的构造子**在 crate 外写不出来
// ——`EntityId("x".to_owned())` 这条路走不通。
//
// 它是简报里 `entity_id_round_trips_and_has_no_public_literal_construction` 的 **(b) 半边**
// （(a) 那半边是往返，在 `tests/candidate.rs` 的同名用例里）。
// 预期是「元组结构体构造子是私有的」那一类；**判据是旁边那份 `.stderr` 本身**，
// 不是某个错误码——本项目为「凭记忆写错误码」付过代价，故 `.stderr` 一律实跑取值。
//
// 另一半（正控制）在 `tests/compile_pass/entity_id_from_str_is_constructible.rs`：
// `EntityId::from_str` 是**公开的构造入口**，crate 外编得过。
// 两半合起来才是「有公开入口、但没有公开字面量构造」——只留这一份的话，
// 它的绿可以来自「整个 crate 的 API 名字都对不上」这类与私有性无关的原因。
//
// **本样例不声称「crate 外造不出 `EntityId`」**：`from_str` 造得出。
// 被挡住的只是「绕过 `from_str` 自己拼一个」。
use continuum_canonical::EntityId;

fn main() {
    let _ = EntityId("x".to_owned());
}
