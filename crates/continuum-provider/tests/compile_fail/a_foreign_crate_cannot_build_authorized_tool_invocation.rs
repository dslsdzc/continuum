//! 外部 crate **手里有一枚真的 `AuthorizedTool` 也构造不出这个请求**（设计 §7.5，
//! 裁决 C2）——构造入口 `AuthorizedToolInvocation::new` 是 `pub(crate)`，
//! 唯一构造点是 `ProviderRegistry::invoke_tool`。
//!
//! **本样例的判据是报错落在「`new` 是私有的」这一条上**（实测报错码是
//! `E0624`：associated function `new` is private），不是 `E0061` / `E0308`：
//! 少传一个参数、或参数类型不对，同样编译不过，但那是「构造入口签名不合」这件事，
//! 与「有授权也够不着构造入口」不是同一条性质。本样例钉的是后者。
//!
//! **订正（Task 6 实测，2026-10-06）**：task-6-brief 第 44 行写「预期报错是 `E0603`」，
//! 该码不对——`E0603` 是**按路径访问私有条目**的码，而这里是**私有关联函数**，
//! rustc 报 `E0624`。brief 那段话的**实质**（钉的是「私有」，不是元数/类型不合）成立，
//! 故判据未变、样例未变，只有码订正；`.stderr` 按实跑生成，钉的是 `E0624`。
//!
//! 与「没有 `AuthorizedTool` 就构造不出它」相比，`pub(crate)` 让这条更强：这里
//! 参数位置上**已经有一枚真的 `AuthorizedTool`**，仍然构造不出。

use continuum_capability::AuthorizedTool;
use continuum_provider::AuthorizedToolInvocation;

fn build(auth: &AuthorizedTool) -> AuthorizedToolInvocation<'_> {
    AuthorizedToolInvocation::new(auth, serde_json::json!({}))
}

fn main() {}
