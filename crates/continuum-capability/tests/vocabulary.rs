//! 词汇表与 `EffectType` 的对应（设计第 2.2 节）。

use continuum_capability::{
    CapabilityKind, EmailAction, EnvAction, GitAction, PaymentAction, RegistryAction,
};
use continuum_effect::EffectType;

/// 六个 `EffectType` 各断言一次（逐项有照片，不抽代表）。
///
/// 遍历 [`EffectType::ALL`]，期望值写成穷尽且无通配臂的 `match`：`EffectType`
/// 加变体时本用例编译不过（`E0004`），逼作者回到这里补第六/第七张照片。
/// 与此同时，`ALL` 漏列变体不会被 `match` 点出（那只在遍历到它时才查），
/// 故另断言它的长度。长度这条看得见「多一个 / 少一个」，**看不见「删一个、补一个」**
/// ——那一种由 `no_capability_kind_maps_to_two_effect_types` 兜住（那里有实测记录）。
#[test]
fn every_effect_type_has_exactly_one_capability() {
    assert_eq!(
        EffectType::ALL.len(),
        6,
        "EffectType 的变体数与对照表（设计第 2.2 节）的六条不符：ALL 漏列或枚举已变"
    );

    for effect in EffectType::ALL {
        let expected = match effect {
            EffectType::SendEmail => CapabilityKind::Email(EmailAction::Send),
            EffectType::PushBranch => CapabilityKind::Git(GitAction::Push),
            EffectType::Publish => CapabilityKind::Registry(RegistryAction::Publish),
            EffectType::DeleteRemote => CapabilityKind::Git(GitAction::DeleteRemote),
            EffectType::Charge => CapabilityKind::Payment(PaymentAction::Charge),
            EffectType::Deploy => CapabilityKind::Environment(EnvAction::Deploy),
        };
        assert_eq!(
            CapabilityKind::for_effect(effect),
            expected,
            "{effect:?} 的 CapabilityKind 与设计第 2.2 节的对照表不符"
        );
    }
}

/// 单射：六个 `EffectType` 得到六个**互不相同**的 `CapabilityKind`。
///
/// 判据是「已出现过的 `CapabilityKind` 不再出现」。少了这条，两个 `EffectType`
/// 落到同一枚能力上也能过——两个词汇表就会各走各的，而那正是 P2 出过 Critical
/// 的那一类（设计第 2.2 节）。
///
/// 本条**不是**上一条的重复：上一条按 `ALL` 逐项精确取值，但它**看不见「`ALL` 里
/// 删一个变体、同时重复另一个」**——长度仍为六、被访问的每个元素都与期望相符，
/// 故它照样通过；而那种漂移会让两个 `EffectType` 落到同一个 `CapabilityKind` 上，
/// **只有本条抓得住**。
///
/// 这条判别力是实测的，不是推出来的：把 `continuum_effect::EffectType::ALL` 改成
/// `[SendEmail, SendEmail, PushBranch, Publish, DeleteRemote, Charge]`（`Deploy` 被删）
/// 后跑全量，**只有本条失败**（`Email(Send) 对应了两个 EffectType：SendEmail 与先前的
/// 某一个`），上一条与 `continuum-effect` 自己的 `every_effect_type_round_trips_…`
/// 都通过。
#[test]
fn no_capability_kind_maps_to_two_effect_types() {
    let mut seen: Vec<CapabilityKind> = Vec::new();

    for effect in EffectType::ALL {
        let kind = CapabilityKind::for_effect(effect);
        assert!(
            !seen.contains(&kind),
            "{kind:?} 对应了两个 EffectType：{effect:?} 与先前的某一个"
        );
        seen.push(kind);
    }
}
