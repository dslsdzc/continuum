// 应编译失败：**一次路由请求没有预算就装配不出来**。这是 ENG-005「预算是必填输入」的
// 照片形态——**签名层面，没有运行期用例**（设计 §9 的对应行、§10 第 2 条：语义层未建，
// `BudgetView` 的每个 `Some` 都只能由测试构造，「剩余额度算得对不对」在本阶段不可观察）。
//
// **本样例的两段各钉这条保证的一侧，缺一条就是只钉了一半**：
//
//   (a) `missing_a_budget_does_not_compile`：漏掉 `budget` 这一格 → E0063。
//   (b) `a_real_budget_view_fits_the_field`：把一枚**写明类型是 `BudgetView`** 的值交给
//       `budget` 位 → 必须**通过**。这一侧钉的是**字段的类型正是 `BudgetView`**：
//       若把字段改成 `Option<BudgetView>`（＝「预算可以省」，正是 fail-open 的那一侧），
//       这里就是 E0308，于是**整段 stderr 多出一块**、与 `.stderr` 不符而变红。
//       **没有 (b)，改 `Option` 不会有任何用例变红**——(a) 在 `Option<BudgetView>` 下
//       照样是 E0063（`Option` 字段一样要写出来）。这一侧正是不设防时最容易漏掉的那一侧。
//
// 两段都在同一个文件里是刻意的：trybuild 的判据是**整段 stderr**，两段一起比对，
// 任一侧被改动都会让 stderr 变样。(b) 不贡献任何 stderr 行——它编译得过——
// 但它是**可被观测的**：它一变错就会往 stderr 里加一块 E0308。
//
// 两个函数都取了函数项（`let _ = ...`），免得 `dead_code` 警告混进本样例的 stderr；
// 函数体照常被类型检查。
use continuum_model_registry::{BudgetView, FamilyPreference, RoutingRequest, TaskSkillRequirement};

/// (a) 漏掉 `budget`：字段一个都不能省。
fn missing_a_budget(requirements: TaskSkillRequirement) -> RoutingRequest {
    RoutingRequest {
        requirements,
        family: FamilyPreference::Auto,
        availability: Vec::new(),
    }
}

/// (b) 五个量纲都写明类型：`budget` 位收的**就是** [`BudgetView`]，不是一个装得下它的 `Option`。
fn a_real_budget_view_fits_the_field(
    requirements: TaskSkillRequirement,
    budget: BudgetView,
) -> RoutingRequest {
    RoutingRequest {
        requirements,
        family: FamilyPreference::Auto,
        availability: Vec::new(),
        budget,
    }
}

fn main() {
    let _ = missing_a_budget;
    let _ = a_real_budget_view_fits_the_field;
}
