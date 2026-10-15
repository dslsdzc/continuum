// 正控制（应编译**通过**，且运行时通过）：`EntityId::from_str` 是**公开的**构造入口。
//
// 没有这一份，`tests/compile_fail/entity_id_has_no_public_literal_construction.rs` 的绿
// 可以来自任何与私有性无关的原因（例如 `EntityId` 这个名字整个不存在、或 `from_str` 改名了）。
// 这就是本仓「正控制不得自证」那条纪律要的形状：把**漏／多／位置**三件一起断——
// 反例断「元组构造写不出来」，正例断「该写得出来的那一条写得出来」。
//
// 它同时钉住「本类型**没有失败路径**」这一条：`from_str` 不返回 `Result`
// （一个恒返回 `Ok` 的 `Result` 会把「可能失败」写成谎话，见 `src/candidate.rs` 的 `from_str`）。
// 这份样例里没有 `unwrap`，故若有人把签名改成 `Result`，本样例立刻编不过。
use continuum_canonical::EntityId;

fn main() {
    let id = EntityId::from_str("tool.github");
    assert_eq!(
        id.as_str(),
        "tool.github",
        "该构造入口交回来的就是给它的那个串"
    );

    // **本样例只到「可构造、且不返回 `Result`」为止**：取值域的处置（空串被接受、
    // 不改写不裁剪）由 `tests/candidate.rs` 的同名用例断言，不在这里再断一遍——
    // 一份样例守两句要求的话，那句额外的断言会成为**别的**变异体的第二个失败面，
    // 让「谁红」的预告与实际对不上（M1 那枚变异体实测就会顺带打到这里）。
}
