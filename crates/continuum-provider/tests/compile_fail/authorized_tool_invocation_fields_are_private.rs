//! 结构体字面量构造被拒——**字段名写对也构造不出**（设计 §7.5、裁决 C2）。
//!
//! 样例 4 钉的是构造入口 `new` 够不着（`E0624`）；本样例钉的是**绕开构造入口的另一条路
//! 也不存在**：`authorization` / `input` 两个字段私有，crate 外写不出结构体字面量。
//! 两条合起来才说明「crate 外只有把证明交给注册表这一条路」；只写一条会有缺口。
//!
//! 与 P3A 的 `authorized_tool_cannot_be_built.rs` 同形——那里同一条性质落在
//! `AuthorizedTool` 上。预期报错是 `E0451`（字段私有），不是拼错字段名（那会另报
//! `E0560` / `E0063`，钉的是另一件事）。
//!
//! **写法**：`main` 里造不出一枚真的 `AuthorizedTool`（那正是 P3A 守着的性质），
//! 故与样例 4 同形，把构造写在收参数的函数体里。
//!
//! **为什么不用 `let authorization: &AuthorizedTool = todo!();` 占位**：那是我落笔时的初稿
//! （**不是 brief 的示意**——brief 对本样例只写了一句话，其唯一的 `todo!()` 在样例 1 的示意行）。
//! 实测这个形状会引出两条警告——`unreachable_code`（`todo!()` 之后还有一条 `let _ = …` 语句）
//! 与 `unused_variables`——`.stderr` 会把它们与 `E0451` 一并钉进去，而这两条与本节要钉的性质
//! 无关。
//!
//! **订正（评审轮，2026-10-06）**：评审提出把上面这条理由换成「`todo!()` 占位表达不了
//! 『字段名写对但字段私有』这件事」，依据是「尾位置的 `todo!()` 不产生 `unreachable_code`」
//! 与「硬错误在场时后置 lint 不运行」。**两条依据在本形状上都不成立**：这里的 `todo!()` 不是
//! 尾表达式，`unreachable_code` 照发；且 `E0451` 与那两条警告**同现于同一份 `.stderr`**
//! （实跑记录 `.tmp/t6-repro-todo.log`）。「后置 lint 不运行」至多对 `dead_code` 这类**晚** lint
//! 成立（样例 4 的 `fn build` 无人调用、其 `.stderr` 里确无 `dead_code`，可作对照），对
//! `unreachable_code` / `unused_variables` 这类**早** lint 不成立。**理由据实保留原说。**

use continuum_capability::AuthorizedTool;

fn build(auth: &AuthorizedTool) -> continuum_provider::AuthorizedToolInvocation<'_> {
    continuum_provider::AuthorizedToolInvocation {
        authorization: auth,
        input: serde_json::json!({}),
    }
}

fn main() {}
