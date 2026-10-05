//! 强制点 (2) 的连接器侧载体：一条**已获准**的效应（设计 §4.2）。
//!
//! 真正的副作用最终由 P3 的连接器（子项目 B）执行，故强制点 (2) **两处都落**：驱动侧
//! 的校验在 [`crate::effect`] 之外（`continuum-runtime` 的 `task_cmd`），连接器侧那半个
//! 义务由本模块的 [`AuthorizedEffect`] 承载。连接器要做副作用就必须收它——若 B 将来
//! 绕过，那是**类型上表达得出来的选择**（收的是别的类型），而不是「忘了接线」这种查不出
//! 来的漏（设计 §4.2 点名的正是 P2b 终审花一整轮才查出的那一类）。
//!
//! # 与强制点 (1) 的 [`crate::AuthorizedTool`] 「同形」的限度
//!
//! 设计 §4.2 说本类型「与 `AuthorizedTool` 同形」。**同形的是保证，不是构造方式**：
//! `AuthorizedTool` 的产出函数 `authorize` 在本 crate 内，故 crate 外**构造不出**它
//! （字段私有、无公开构造函数）。[`AuthorizedEffect`] 的产出者是**驱动**（另一个
//! crate），故它的构造入口必须公开——能保护的不是「外部构造不出来」，而是**配对**：
//! 给入的能力必须正是这条效应所对应的那一种（[`CapabilityKind::for_effect`]）。
//! 一枚 `filesystem.read` 的能力因此无法被当作「`charge` 已获准」的凭证——
//! 逐项照片见 `tests/authorized_effect.rs`。
//!
//! 这一区别**不削弱**它要挡的东西：能构造出 `AuthorizedEffect` 就必然持有 `mint` 铸出、
//! 且 kind 与效应相符的能力，而 `mint` 是唯一的签发点（设计 §2.4）。剩下的那条路
//! ——调用方自己调 `mint`——是 `mint` 公开本身的性质，与本类型无关。

use continuum_effect::EffectType;

use crate::capability::{Capability, CapabilityKind};
use crate::error::CapabilityError;

/// 一条**已获准**的效应：效应类型，加上准了它的那枚能力。
///
/// 字段私有、无公开的字段写入路径；唯一的构造入口是 [`AuthorizedEffect::new`]，而它会
/// 核对「能力的 kind 与效应对应」。
///
/// **本类型今天没有消费方**：连接器（子项目 B）是它的消费方，尚未建。驱动会真的构造它
/// （每声明一条 `--effect` 构造一枚），持有至命令结束、随后随作用域析构丢弃——**这不是
/// 遗留物**：它是强制点 (2) 的产物本身，其存在即是「这条效应确实过了校验」的载体。
/// 把它接到连接器上正是 B 的义务（设计 §10 第 7 条）。
///
/// 访问器只开两个，都是 B 要用的：这是哪一条效应（[`AuthorizedEffect::effect`]），
/// 以及准它的那枚能力（[`AuthorizedEffect::capability`]）。**刻意不给 `#[allow(dead_code)]`**
/// ——本 crate 是库，这两个 `pub` 访问器是公开面的一部分，字段由它们读，故不需要。
#[derive(Debug)]
pub struct AuthorizedEffect {
    effect: EffectType,
    capability: Capability,
}

impl AuthorizedEffect {
    /// **唯一产出路径**：核对能力的 kind 与效应对应之后收下。
    ///
    /// 不对应 → [`CapabilityError::EffectCapabilityMismatch`]，不静默改成某一方、也不
    /// 挑一枚别的能力（那会让「效应与能力的对应」有两个产生点）。
    ///
    /// 本函数**不读时钟**：能力的时效由 [`Capability::is_valid_at`] 判，与
    /// [`crate::mint`] 同一条约定。
    pub fn new(effect: EffectType, capability: Capability) -> Result<Self, CapabilityError> {
        let expected = CapabilityKind::for_effect(effect);
        if capability.kind() != expected {
            return Err(CapabilityError::EffectCapabilityMismatch {
                effect,
                expected,
                actual: capability.kind(),
            });
        }
        Ok(Self { effect, capability })
    }

    /// 这是一条什么效应。
    pub fn effect(&self) -> EffectType {
        self.effect
    }

    /// 准了这条效应的那枚能力。
    pub fn capability(&self) -> &Capability {
        &self.capability
    }
}
