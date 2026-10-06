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
//! 故与样例 4 同形，把构造写在收参数的函数体里。**不用 `todo!()` 占位**：那样会先
//! 引出一条 `unreachable_code` 警告、一条 `unused_variables` 警告，`.stderr` 就得
//! 把这两条也钉进去，而它们与本节要钉的性质无关。

use continuum_capability::AuthorizedTool;

fn build(auth: &AuthorizedTool) -> continuum_provider::AuthorizedToolInvocation<'_> {
    continuum_provider::AuthorizedToolInvocation {
        authorization: auth,
        input: serde_json::json!({}),
    }
}

fn main() {}
