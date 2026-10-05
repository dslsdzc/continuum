//! §249 的模型生命周期：十个状态、一张迁移表、一道可路由闸门。
//!
//! 设计 `docs/superpowers/specs/2026-10-05-p3d-model-registry-router-design.md` §4.1 / §4.2。
//!
//! # 两件事，两个类型
//!
//! [`LifecycleState`] 是**十个状态**——画像流水线、漂移标记、隔离与停用都要用它，
//! 故它在模型注册表里是**表达得出来的**。[`RoutableState`] 是其中**可被自动路由**的六个；
//! 另四个（`unprofiled` / `stale` / `quarantined` / `disabled`）**在路由路径上无处安放**。
//! **被挡住的是路由路径，不是表达能力**——本模块不声称那四态「不可表达」。
//!
//! 做法与 P3A 的 `full_access` 同：**不是「禁止作为默认」，是没有这个成员**
//! （`crates/continuum-capability/src/registry.rs`）。

use crate::error::{LifecycleError, RoutingError};
use crate::profile::ModelProfile;

/// §249 的十个状态（**照录**：`docs/spec/05-normative.md:856-884` 的六态加四个异常态）。
///
/// # 编码
///
/// 落库编码小写、多词以 `_` 连接（全局约束），由 [`LifecycleState::as_str`] /
/// [`LifecycleState::parse`] **与本类型同址**给出，不在 `persist.rs` 里另建表。
/// 十态恰好都是单词，故当前编码里一个 `_` 都不出现。
///
/// **`Degraded` 与 `ProviderHealth::Degraded` 是两个轴上的两个东西**，只是名字撞了：
/// 后者（`crates/continuum-core/src/model.rs:83-87`）是**供应商侧的可用性**，
/// 前者出自 §249 的**模型生命周期异常态清单**。故前者保持可路由（见 [`RoutableState`]），
/// 本模块**不因这次撞名而改它**（设计 §4.2 末段）。
///
/// 派生集与 `crates/continuum-graph/src/node.rs` 的 `NodeState` 同（少 serde——本 crate 的
/// 枚举编码走 [`LifecycleState::as_str`]，不经 serde，见全局约束）。四个都有当场消费方：
/// `Debug` 给错误与断言消息的 `{state:?}`；`PartialEq` / `Eq` 由 [`LifecycleError`] /
/// [`RoutingError`] 的派生要求；`Clone` 由 `Copy` 要求；`Copy` 供各处的逐项遍历。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    Discovered,
    Unprofiled,
    Researched,
    Probed,
    Verified,
    Active,
    Stale,
    Degraded,
    Quarantined,
    Disabled,
}

impl LifecycleState {
    /// 落库编码（全局约束）：小写、多词以 `_` 连接。**本函数是该编码唯一的产生点**。
    ///
    /// match 穷尽且无通配臂：加状态时本函数编译不过，编码不会漏分支。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Discovered => "discovered",
            Self::Unprofiled => "unprofiled",
            Self::Researched => "researched",
            Self::Probed => "probed",
            Self::Verified => "verified",
            Self::Active => "active",
            Self::Stale => "stale",
            Self::Degraded => "degraded",
            Self::Quarantined => "quarantined",
            Self::Disabled => "disabled",
        }
    }

    /// [`LifecycleState::as_str`] 的**严格逆**：十态各有 `parse(s.as_str()) == Some(s)`，
    /// 表外字符串一律 `None`。
    ///
    /// `None` 而非某一枚默认状态：把表外串猜成某个状态会成为第二份表示
    /// （与 `SkillDimension::parse` / `CapabilityKind::parse` 同一条理由），
    /// 调用方（`crate::persist`）把它转成具体 `Err`。
    /// 解码侧没有穷尽 match 的保护——来源是 `&str`，编译器点不出漏掉的臂，
    /// 故由 `tests/lifecycle.rs` 的 `the_ten_lifecycle_states_are_recorded_verbatim` 遍历十态兜住。
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "discovered" => Self::Discovered,
            "unprofiled" => Self::Unprofiled,
            "researched" => Self::Researched,
            "probed" => Self::Probed,
            "verified" => Self::Verified,
            "active" => Self::Active,
            "stale" => Self::Stale,
            "degraded" => Self::Degraded,
            "quarantined" => Self::Quarantined,
            "disabled" => Self::Disabled,
            _ => return None,
        })
    }
}

/// 可被**自动路由**的那六个状态。
///
/// `unprofiled` / `quarantined` / `disabled` 是 §249 禁的三态；`stale` 是裁决额外挡下的
/// 第四态（`docs/superpowers/specs/2026-10-05-p3bcdf-set-decisions.md` 第一节第 4 条已裁：
/// §249 的三态是**下限不是上限**；§82 说漂移中的模型正在退出服役，放行即 fail-open）。
/// 做法与 P3A 的 `full_access` 同：**不是「禁止作为默认」，是没有这个成员**。
///
/// **措辞与保证等强**：这四态在 [`LifecycleState`] 上**是表达得出来的**（画像流水线、
/// 漂移标记、隔离与停用都要用它），被挡住的是**路由路径**——本模块**不声称「四态不可表达」**。
///
/// 派生集与 [`LifecycleState`] 同（也是 `crates/continuum-graph/src/node.rs` 的 `NodeState`
/// 那一套）。`Copy` 有当场消费方——[`RoutableModel::state`] 按值返回它；其余三个是状态枚举
/// 的常规配套，`state` 要原样带进候选的 `reason`（设计 §5.2）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutableState {
    Discovered,
    Researched,
    Probed,
    Verified,
    Active,
    Degraded,
}

impl TryFrom<LifecycleState> for RoutableState {
    type Error = RoutingError;

    /// 闸门本体：六态收窄，四态 [`RoutingError::NotRoutable`] 且**带出传入的那个十态值**。
    ///
    /// match 穷尽且无通配臂：加状态时本函数编译不过，**新状态不会被默认放行**——
    /// 这正是 fail-open 的形状所要防的：一个默认臂会把「没想过的新态」静默送进路由。
    ///
    /// **四态是四条臂各写各的**，不折成 `matches!` 一句：那样改起来省事，排查时却要靠
    /// 反读谓词才能知道挡了哪几个；而本闸门是 §249 的 MUST NOT 落点，值得摊开。
    /// 收窄那六条臂同理——它们是放行侧，是 fail-open 的一侧。
    fn try_from(state: LifecycleState) -> Result<Self, Self::Error> {
        Ok(match state {
            LifecycleState::Discovered => Self::Discovered,
            LifecycleState::Researched => Self::Researched,
            LifecycleState::Probed => Self::Probed,
            LifecycleState::Verified => Self::Verified,
            LifecycleState::Active => Self::Active,
            LifecycleState::Degraded => Self::Degraded,
            LifecycleState::Unprofiled
            | LifecycleState::Stale
            | LifecycleState::Quarantined
            | LifecycleState::Disabled => return Err(RoutingError::NotRoutable { state }),
        })
    }
}

/// 一个**可交给 Router** 的模型：画像 + 已过闸门的状态。
///
/// 字段私有，故**结构体字面量这条路在 crate 外走不通**（`tests/compile_fail/
/// routable_model_fields_are_private.rs` 钉住它，E0451）；**唯一的产生点是
/// [`RoutableModel::try_new`]**，而它内部过闸门。于是在 `rank` 的签名里**不存在**
/// 未画像、被隔离、被停用、已漂移的模型——
/// **「Router 忘了检查状态」这条路径在类型上不存在**（设计 §4.2）。
///
/// 与 `ModelProfile` 的「crate 外构造不出来」同形：两条结构性保证合起来才立住
/// §21 的「新增模型不能直接进入自动 Router」（状态未过闸门，**或**尚无画像）。
pub struct RoutableModel {
    profile: ModelProfile,
    state: RoutableState,
}

impl RoutableModel {
    /// 收 `ModelProfile` 与十态的 [`LifecycleState`]，**在当前函数体内过闸门**。
    ///
    /// **入参的状态是十态，不是收窄后的 [`RoutableState`]**——若入参已是 `RoutableState`，
    /// 闸门就跑到调用方去了，「Router 忘了检查」那道保证随之失效
    /// （设计 §2.1 的示例写的是 `RoutableState`，与 §4.2 的十态逐项用例要求不一致；
    /// 本实现取 §4.2，见计划 `## 遗留`）。
    ///
    /// 失败即 [`RoutingError::NotRoutable`]，**`state` 带的是传入的那个十态值**。
    pub fn try_new(profile: ModelProfile, state: LifecycleState) -> Result<Self, RoutingError> {
        Ok(Self {
            profile,
            state: RoutableState::try_from(state)?,
        })
    }

    /// 这份画像。`RankingPolicy::evaluate` 要读它（设计 §5.3 的签名即判据）。
    pub fn profile(&self) -> &ModelProfile {
        &self.profile
    }

    /// 已过闸门的状态。策略据此降权（`degraded` 可见但不禁，设计 §4.2 末段）。
    pub fn state(&self) -> RoutableState {
        self.state
    }
}

/// §249 未给迁移关系，本表由本设计定（与 §237 落在 P1 的情形相同，设计 §4.1）。
///
/// # 表
///
/// ```text
/// 正常阶梯（线性、单调，§21 §22）:
///   discovered → unprofiled → researched → probed → verified → active
/// 边：
///   active      → stale         §82 行为指纹漂移
///   stale       → researched    §82「重新 profiling」——回到画像流水线的入口
///   active      → degraded      性能退化
///   degraded    → active        恢复
///   *           → quarantined   §249 的异常态，可由任意状态进入
///   *           → disabled
///   quarantined → disabled
///   disabled    → unprofiled    重新启用须重走 §22 的 onboarding
/// ```
///
/// # 自环不是特例
///
/// **自环的合法性由这张表决定**（执行期裁定，计划 Task 4）：`* → quarantined` 与
/// `* → disabled` 里的 `*` **含它自己**，故 `quarantined → quarantined` 与
/// `disabled → disabled` 是 `Ok`；而 `active → active` 这类**没有列进表**，仍是 `Illegal`。
/// 故实现里**不加**「`from == to` 一律拒绝」这一臂——它与本表自己的 `*` 相抵。
///
/// # 返回值
///
/// **迁移前的旧态（`from`）**，不是 `to`（设计 §3.2 的定稿）：调用方要记「从哪来」，
/// 而「到哪去」它就是自己传的 `to`，返回它没有信息量。
///
/// **本函数是纯函数**，不带 `Tx`——落库版的同名函数是 `persist::transition_in_tx`
/// （设计 §3.2，计划 `## 遗留` 的那处取名）。
pub fn transition(from: LifecycleState, to: LifecycleState) -> Result<LifecycleState, LifecycleError> {
    use LifecycleState::*;

    // `* → quarantined` / `* → disabled`：事故与人工下线不挑时机，与 P1 的
    // `INVALIDATED` / `CANCELLED` 同一判据（`crates/continuum-graph/src/state.rs:16-19`）。
    // **这条在自环检查之前返回**（若实现里另加一条 `from == to` 的早退，本表的 `*` 随之失效）：
    // `*` 含来源状态自己，故 `quarantined → quarantined` / `disabled → disabled` 也走这一条。
    if matches!(to, Quarantined | Disabled) {
        return Ok(from);
    }

    // 其余各边逐对列出，**不含自环**——它的合法性由上面那条 `*` 给出，不在这里重复。
    //
    // **这张表与 `tests/lifecycle.rs` 里那组逐条合法对断言是两份手写副本**（`#[cfg(test)]`
    // 的 crate 内模块与集成测试是两个 crate，共享不了表，理由同下面 `ALL_STATES`）。
    // 故 §4.1 若添一条边而两处同漏，**没有任何用例会红**——编译器兜不住，只有靠加边时记得两处都加。
    let listed = matches!(
        (from, to),
        (Discovered, Unprofiled)
            | (Unprofiled, Researched)
            | (Researched, Probed)
            | (Probed, Verified)
            | (Verified, Active)
            | (Active, Stale)
            | (Stale, Researched)
            | (Active, Degraded)
            | (Degraded, Active)
            // `quarantined → disabled` 不在这里列：`:224` 的 `* → Disabled` 已先返回，
            // 此处再列一条是不可达的死臂（表上它仍是一条边，由那一行覆盖）。
            | (Disabled, Unprofiled)
    );

    if listed {
        Ok(from)
    } else {
        Err(LifecycleError::Illegal { from, to })
    }
}

// ===== 闸门的用例（crate 内） =====
//
// **为什么这三条在 `src/` 而不在 `tests/lifecycle.rs`**：它们各要一份画像，而
// `ModelProfile` 的构造是 crate 内的（`pub(crate) fn try_new`，设计 §2.1），
// 集成测试构造不出来（计划第 84 行已点明这一后果；Task 3 的字段级用例同样落在
// `src/profile.rs` 的 `#[cfg(test)]` 里）。**这不是把闸门用例降级**——
// 它们与迁移表那组一样跑在 `cargo test --workspace` 里。
// 十态编码与迁移表那七条不需要画像，住在 `tests/lifecycle.rs`。
#[cfg(test)]
mod tests {
    use super::*;
    use continuum_capability::{Cost, Latency};
    use continuum_core::model::ModelId;
    use continuum_core::tool::ToolId;

    use crate::profile::{Ratio, SkillDimension, SkillObservation, SkillScore, SkillVector};

    /// 十态**逐项列出**（不抽代表）：闸门那两条用例都遍历它。
    ///
    /// 与 `tests/lifecycle.rs` 的 `ALL_STATES` 是**同一份手写表的两处副本**——两处都得手工同步，
    /// 因为 `#[cfg(test)]` 的 crate 内模块与集成测试是两个 crate，共享不了 `const`。
    /// 这个点不设防，靠后来者自己记得（加第十一态时两处都要加）。
    const ALL_STATES: [LifecycleState; 10] = [
        LifecycleState::Discovered,
        LifecycleState::Unprofiled,
        LifecycleState::Researched,
        LifecycleState::Probed,
        LifecycleState::Verified,
        LifecycleState::Active,
        LifecycleState::Stale,
        LifecycleState::Degraded,
        LifecycleState::Quarantined,
        LifecycleState::Disabled,
    ];

    /// 一份**能过闸门**的画像。十二个字段取非默认值（与 `src/profile.rs` 的 `a_profile` 同形，
    /// 但本模块只需要「有一份画像」，不逐字段比对，故取值从简——**不为此再写一份十二项夹具**）。
    fn a_profile() -> ModelProfile {
        ModelProfile::try_new(
            ModelId::new("model-alpha"),
            String::from("version-beta"),
            String::from("provider-gamma"),
            String::from("revision-delta"),
            vec![String::from("text")],
            vec![ToolId::new("tool-epsilon")],
            SkillVector::from_current(vec![(
                SkillDimension::Coding,
                SkillObservation::try_new(
                    SkillScore::try_new(9.2).expect("9.2 应是合法评分"),
                    Ratio::try_new(0.8).expect("0.8 应是合法置信度"),
                    7,
                    3,
                    (1_700_000_000_000, 1_700_000_001_000),
                )
                .expect("该时间窗应自洽"),
            )]),
            vec![String::from("loses constraints in very long tasks")],
            Some(Latency),
            Some(Cost),
            42,
            Ratio::try_new(0.94).expect("0.94 应是合法置信度"),
        )
    }

    /// 十态**逐项**喂进 [`RoutableModel::try_new`]：六态 `Ok`、四态
    /// `Err(NotRoutable { state })` 且**断言是哪一枚**（不是「返回了 `Err`」）。
    ///
    /// 这是**枚举式绝对断言**，逐项钉：放行集与拒绝集各十项内一条，不抽代表。
    ///
    /// 红的条件两条，各在一侧：
    /// - **放宽**（拒绝集里某一态被放行）→ 那一条红；
    /// - **收紧**（放行集里某一态被挡下）→ 那一条红。
    ///
    /// 两侧都在本用例里——只钉一侧会让另一侧整片漂移（见下面那条具名的 `stale` 用例）。
    #[test]
    fn only_the_six_routable_states_pass_the_gate() {
        use LifecycleState::*;

        // 放行集（六态）：收窄成功，且收窄到的正是那一枚。
        assert!(matches!(
            RoutableModel::try_new(a_profile(), Discovered),
            Ok(m) if matches!(m.state(), RoutableState::Discovered)
        ));
        assert!(matches!(
            RoutableModel::try_new(a_profile(), Researched),
            Ok(m) if matches!(m.state(), RoutableState::Researched)
        ));
        assert!(matches!(
            RoutableModel::try_new(a_profile(), Probed),
            Ok(m) if matches!(m.state(), RoutableState::Probed)
        ));
        assert!(matches!(
            RoutableModel::try_new(a_profile(), Verified),
            Ok(m) if matches!(m.state(), RoutableState::Verified)
        ));
        assert!(matches!(
            RoutableModel::try_new(a_profile(), Active),
            Ok(m) if matches!(m.state(), RoutableState::Active)
        ));
        assert!(matches!(
            RoutableModel::try_new(a_profile(), Degraded),
            Ok(m) if matches!(m.state(), RoutableState::Degraded)
        ));

        // 拒绝集（四态）：各断言**是哪一枚**——`NotRoutable` 且 `state` 正是喂进去的那个。
        // 用 `assert_eq!` 比对整个错误值：只写 `is_err()` 的话，「拒了但是拒错了状态」不会红。
        for state in [Unprofiled, Stale, Quarantined, Disabled] {
            assert_eq!(
                RoutableModel::try_new(a_profile(), state).err(),
                Some(RoutingError::NotRoutable { state }),
                "{state:?} 应在闸门处被拒，且错误里带的是它自己"
            );
        }

        // 这条断言**只比个数**：`ALL_STATES` 恰好十项，多一项少一项即红。
        // 它**不验**「上面那两段逐项清单的并集恰好是十态」——那十条是本用例手写的、
        // 与 `ALL_STATES` 是两份表，这里求不了并集。「十态一个不漏」的兜底在
        // `RoutableState::try_from`：它穷尽且无通配臂，加第十一态时编译不过。
        assert_eq!(ALL_STATES.len(), 6 + 4, "十态 = 六个放行 ＋ 四个拒绝");
    }

    /// **两侧对钉的具名用例**：`stale` → `Err(NotRoutable { state: Stale })`，
    /// **同一份画像** ＋ `Active` → `Ok`。
    ///
    /// 依据是**已裁的** §4.2（`docs/superpowers/specs/2026-10-05-p3bcdf-set-decisions.md`
    /// 第一节第 4 条：§249 的三态是**下限不是上限**，`stale` 也禁自动路由）。
    ///
    /// **为什么这条必须两侧都写**：删掉 `stale` 那一臂（把它并进放行集）时，
    /// 「正常态确实会被路由」的那侧照样绿——`Active` 根本不受影响。**缺的那侧恰恰是
    /// fail-open 的那一侧**：单侧用例钉不住「有人把 `stale` 放进来」这件事。
    /// 反过来，把放行侧的 `Active` 臂删掉，也只由本用例的第二半变红。
    ///
    /// 两侧用**同一份画像**的取值（各调一次 `a_profile()`——它无副作用，两次产物相等）：
    /// 这样两个方向的差别只剩状态一个变量，不会把「画像不同」读成「闸门不同」。
    #[test]
    fn a_stale_model_is_not_routable_while_an_active_one_is() {
        assert_eq!(
            RoutableModel::try_new(a_profile(), LifecycleState::Stale).err(),
            Some(RoutingError::NotRoutable {
                state: LifecycleState::Stale
            }),
            "已漂移的模型正在退出服役（§82），放行即 fail-open"
        );
        assert!(
            RoutableModel::try_new(a_profile(), LifecycleState::Active).is_ok(),
            "正常态确实会被路由——钉住这侧，上面那侧才不是孤证"
        );
    }

    /// `Degraded` → `Ok`（§4.2 末段：**可见但不禁**）。
    ///
    /// 红的条件：把 `Degraded` 一并挡下即红。**撞名不构成理由**——它撞的是
    /// `ProviderHealth::Degraded`（供应商侧的可用性，`crates/continuum-core/src/model.rs:83-87`），
    /// 与 §249 的模型生命周期异常态是**两个轴上的两个东西**。
    ///
    /// 状态**原样带进候选**（`RoutableModel::state()` 读出 `Degraded`），让策略可以据此降权——
    /// 挡下它就看不到这个信息了。候选 `reason` 里的可见性是 Task 11 的事，本 task 只到 `Ok` 与 `state()`。
    #[test]
    fn a_degraded_model_still_passes_the_gate() {
        let model = RoutableModel::try_new(a_profile(), LifecycleState::Degraded)
            .expect("`Degraded` 是 §249 列的异常态，不是 `ProviderHealth::Degraded`，不应被挡");
        assert!(matches!(model.state(), RoutableState::Degraded));
    }
}
