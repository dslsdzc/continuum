// 应编译失败：（在 media 里）想给证据的 subject **指名一条 Requirement**——指名所需的
// 名字（`continuum_semantics::RequirementId`）在本 crate 里**写不出来**，故
// `EvidenceSubject::Requirement(..)` 的这次构造过不了。预期是「无法解析」那一类
// （常见码 E0433）；**判据是实跑产出的那份 `.stderr` 本身，不是这个码**。
//
// # 它钉的是什么
//
// 归属的**权威身份**在语义层（P4）：P5a 设计 §4.3.1 自记 `EvidenceSubject::Requirement`
// 的载荷「落地时以 P4 的 `RequirementId` 为准」（按内容指：`docs/superpowers/specs/`
// `2026-10-08-p5a-verification-and-evidence-design.md` §4.3.1——**按内容指、不写行号**）。
// 本块**不登记**到 `continuum-semantics` 的边（设计 §2.1、§6.1：跨层一律以「值」由
// 装配方传入），故这个类型名在本 crate 里不可及——「面向哪条 Requirement」的装配按设计
// §9.2 的归属表由**装配方**完成，本块的算子产出 `Unattached` 的证据。
//
// # 与计划假设的照实对账（2026-10-16 实测）
//
// 本份在计划里的**原形态**，其成立条件自记为「**P5a 不 re-export `RequirementId`**」。
// **该假设实测已不成立**：P5a 自 Task 2 起把 `RequirementId`（P5a 自定的一枚，与 P4 的
// 同名、不同型——`crates/continuum-verify/src/contract_view.rs`，按内容指）放在 crate 根的
// `pub use` 清单里（按内容指：`crates/continuum-verify/src/lib.rs` 的根导出那一段），
// 且其 `from_str` 公开 ⇒ 用 **P5a 那枚镜像**捏一个 id 去构造该臂，**编得过**（实测）；
// P5a 计划该 task 的 `Produces` 行本来就把 `RequirementId` 列为公开产出——这种写法
// **自始就不可能被挡住**（按内容指：`docs/superpowers/plans/2026-10-10-p5a-verification-and-evidence.md`）。
// 故按本块设计 §12.4 第 2 条的预案换上现在的形态：钉「**指名的权威名字在本块不可及**」，
// 判据仍是 §9.2 的「本块不构造 `Requirement` 归属」。
//
// # 它证不到什么（照实声明，免得被读成覆盖更广）
//
// - **它证不到「在本块里构造不出 `EvidenceSubject::Requirement` 的任何值」**：
//   P5a 的镜像 `RequirementId::from_str` 可用来**捏造**一枚 id、类型上就能构造出该臂——
//   那正是上面「原假设失效」的实测结果。**「本块不构造归属」在 P5a 的今日实现下是
//   行为纪律（§9.2 的归属表），不是结构事实**；本份只守住「权威来源不可及」的一半。
// - **它证不到** P4 那枚与 P5a 那枚的对应（由装配方维持；P5a 设计 §15 第 19 条自记
//   该对应今天没有判据）。
use continuum_verify::EvidenceSubject;

fn attach_requirement() -> EvidenceSubject {
    // 想指名一条 Requirement：权威 id 在语义层——本 crate 里这个名字写不出来。
    EvidenceSubject::Requirement(continuum_semantics::RequirementId::from_str("REQ-1"))
}

fn main() {
    // 取一次函数项，免得 `dead_code` 警告混进本样例的 `.stderr`；
    // 函数体照常被类型检查，故上面那次构造仍是本份要的解析。
    let _ = attach_requirement;
}
