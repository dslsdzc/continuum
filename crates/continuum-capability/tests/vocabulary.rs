//! 词汇表与 `EffectType` 的对应（设计第 2.2 节）。

use continuum_capability::{
    CapabilityKind, EmailAction, EnvAction, FsAction, GitAction, GithubAction, PaymentAction,
    RegistryAction,
};
use continuum_effect::EffectType;

/// 像（`for_effect` 的值域）**之外**的六枚 kind，手工清单。
///
/// 字面量是**手工写的**，不是 `for_effect` 的返回：本清单钉的是「非像的那六枚是哪些」
/// 这个事实本身。设计 §3.3 第 2 条的初稿只列了五枚、漏了 `Filesystem(Write)`，
/// 现已逐枚列全（裁决文件 B1 裁为设计改）。
///
/// 抽成一个函数供 `every_kind_outside_the_image_has_no_effect` 与
/// `the_two_lists_are_disjoint` 共用，免得两份手写清单各自漂移。
fn kinds_outside_the_image() -> [CapabilityKind; 6] {
    [
        CapabilityKind::Filesystem(FsAction::Read),
        CapabilityKind::Filesystem(FsAction::Write),
        CapabilityKind::Git(GitAction::Read),
        CapabilityKind::Git(GitAction::WorktreeWrite),
        CapabilityKind::Git(GitAction::CommitLocal),
        CapabilityKind::Github(GithubAction::CreatePr),
    ]
}

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

/// [`CapabilityKind::effect`] 是 [`CapabilityKind::for_effect`] 的逆：对 `EffectType::ALL`
/// 的**每一个**变体，`for_effect(e).effect() == Some(e)`。**六条各断言一次**（循环每轮一次）。
///
/// 用例里的 `EffectType` 值来自 `ALL` 的遍历（**被测路径**），不是手写字面量：它钉的是
/// 「逆确实沿 `for_effect` 那条路径回来」。`ALL` 漏列变体由
/// `every_effect_type_has_exactly_one_capability` 的长度断言兜住，本用例只遍历它。
///
/// 这一侧只钉住**像内那六枚**；「像外那六枚该给 `None`」由
/// `every_kind_outside_the_image_has_no_effect` 钉，两份手工清单不混进彼此由
/// `the_two_lists_are_disjoint` 钉。
#[test]
fn the_inverse_of_for_effect_returns_the_same_effect_for_every_effect_type() {
    for effect in EffectType::ALL {
        assert_eq!(
            CapabilityKind::for_effect(effect).effect(),
            Some(effect),
            "{effect:?} 经 for_effect 后再取逆，没有回到同一枚效应"
        );
    }
}

/// 像之外的六枚 kind 没有对应效应，`effect()` 一律 `None`。**六枚各断言一次**
/// （经 [`kinds_outside_the_image`] 的六枚手写字面量遍历）。
///
/// 这六枚的字面量是**手工写的**，钉「非像的那六枚是哪些」这个事实。`effect()` 的
/// `match` 穷尽且**无通配臂**，加第 13 枚 kind 时它编译不过，实现者必须补一个臂；
/// **但那个新臂该给 `Some` 还是 `None`，编译器判不了**，只能由人在上面那张清单里补一项。
///
/// 「kind 一共十二枚」这一半是**编译期保证、不是运行期用例**——本 crate 没有
/// `CapabilityKind::ALL`，且 `as_str` 是实例方法，运行期遍历不出全集（据实标明它
/// 为什么没有照片）。取代它的是 `effect()` 的穷尽 `match`。
#[test]
fn every_kind_outside_the_image_has_no_effect() {
    for kind in kinds_outside_the_image() {
        assert_eq!(kind.effect(), None, "{kind:?} 不该有对应效应");
    }
}

/// 两份清单互不相同：像那六枚由 `EffectType::ALL` 遍历经 `for_effect` 得到
/// （**被测函数返回的值**，钉路径），非像那六枚是 [`kinds_outside_the_image`] 里
/// **手工写的字面量**（钉「有哪六枚」这个事实）。
///
/// 断言两组**互不相同**——把两组合并去重后应是**十二枚互异的 kind**。它钉的是
/// 「那份手工清单里没有混进像里的那一枚」：只按 `contains` 逐枚比对不足以发现手工
/// 清单内部出现重复（那样两份清单仍「不重叠」，却不再覆盖十二枚），故判据取并集的
/// 去重基数而非单向 `contains`。
///
/// 本条**不读 `effect()`**，故「把某非像臂的 `None` 改成 `Some`」那条变异打不到它；
/// 它唯一的守卫是「手工清单被换进一枚像里的 kind」——见计划的变异清单。
#[test]
fn the_two_lists_are_disjoint() {
    let mut union: Vec<CapabilityKind> = Vec::new();

    for effect in EffectType::ALL {
        union.push(CapabilityKind::for_effect(effect));
    }
    union.extend(kinds_outside_the_image());

    let mut distinct: Vec<CapabilityKind> = Vec::new();
    for kind in union {
        if !distinct.contains(&kind) {
            distinct.push(kind);
        }
    }

    assert_eq!(
        distinct.len(),
        12,
        "像与非像两份清单并起来不是十二枚互异的 kind：清单混进了像里的一枚，或自身有重复"
    );
}
