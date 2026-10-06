// 应编译失败：`TaskSkillRequirement` **没有** `From<Vec<SkillDimension>>`
// （故 `.into()` 与 `TaskSkillRequirement::from(...)` 都走不通）→ E0277。
//
// 这是「非空」那条守卫（设计 §5.1 末段）的**第二条结构面通道**。它与
// `a_requirement_cannot_be_built_from_a_bare_vec.rs`（字段私有，E0451）**不是同一条**：
// 一个有 `From<Vec<_>>` 的实现就能在 crate 外造出需求值，而字段照样是私有的——
// 只要那个 `From` 不检查空集合，「非空」这条保证就从便利转换那一侧漏掉，
// **而字段私有那份样例照样绿**（fail-open 的那一侧）。
//
// **两份文件是必需的，不是拆得好看**：实测（rustc 1.95）两条错写进同一个函数体时，
// rustc 只报第一条、第二条根本不出现，于是有一条通道没有照片。
//
// 若本文件因**别的**缘故失败（名字拼错、导入缺失），`.stderr` 里就会出现不是 E0277 的错，
// 比对不过——这正是 `.stderr` 存在的理由。
use continuum_model_registry::{SkillDimension, TaskSkillRequirement};

fn main() {
    let _: TaskSkillRequirement = vec![SkillDimension::Coding].into();
}
