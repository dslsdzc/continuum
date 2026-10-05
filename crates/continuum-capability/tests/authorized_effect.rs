//! `AuthorizedEffect`（强制点 (2) 的连接器侧载体，设计 §4.2）。
//!
//! 本类型的全部保证是一条**配对**：`AuthorizedEffect` 里的效应与能力必须互相对应
//! （[`CapabilityKind::for_effect`]）。缺了它，一枚 `filesystem.read` 的能力就能被
//! 当作「`charge` 已获准」的凭证交给连接器——那正是 §4.2 要拦的形状。
//!
//! 逐项有照片：六个 `EffectType` 各一条正例，各一条反例（配一枚**别的** kind）。

use continuum_capability::{AuthorizedEffect, Capability, CapabilityKind, mint};
use continuum_effect::EffectType;

/// 铸一枚作用域非空的能力（`mint` 的唯一失败路径是空作用域，见 `CapabilityError`）。
fn minted(kind: CapabilityKind) -> Capability {
    mint(kind, "scope:x".to_owned(), 1_000).expect("作用域非空，必然铸得出")
}

/// 与给定 `EffectType` **不对应**的一枚能力：固定取 `Filesystem(Read)`，除非那正是
/// 它对应的那一枚（本表里没有这种情形，但写成断言而不是假设——`for_effect` 若将来
/// 改了对应，本用例会在这里红，而不是静默地拿一枚正确的 kind 去当反例）。
fn mismatched(effect: EffectType) -> Capability {
    let wrong = CapabilityKind::Filesystem(continuum_capability::FsAction::Read);
    assert_ne!(
        CapabilityKind::for_effect(effect),
        wrong,
        "反例不能正好是对应的那一枚，否则本用例验不到拒绝"
    );
    minted(wrong)
}

/// 逐一：六个效应各配它自己的 kind → 收下，且携带的正是那两者。
#[test]
fn every_effect_type_is_carried_with_its_own_capability_kind() {
    for effect in EffectType::ALL {
        let kind = CapabilityKind::for_effect(effect);
        let authorized = AuthorizedEffect::new(effect, minted(kind))
            .unwrap_or_else(|e| panic!("{effect:?} 配它自己的 kind 应被收下，得到 {e}"));
        assert_eq!(
            authorized.effect(),
            effect,
            "携带的效应不是构造时给的那一个"
        );
        assert_eq!(
            authorized.capability().kind(),
            kind,
            "携带的能力不是构造时给的那一枚"
        );
    }
}

/// 逐一：六个效应各配一枚**别的** kind → 一律拒，且断言是**哪一种** `Err`。
///
/// 反例与正例成对：只有正例时，一个恒收下的实现全绿；只有反例时，一个恒拒绝的实现
/// 也全绿。
#[test]
fn every_effect_type_rejects_a_capability_of_another_kind() {
    for effect in EffectType::ALL {
        let wrong = mismatched(effect);
        let wrong_kind = wrong.kind();
        let err = AuthorizedEffect::new(effect, wrong).expect_err(
            "配一枚不对应的能力必须被拒——否则连接器会拿错的能力去执行这条效应",
        );
        match err {
            continuum_capability::CapabilityError::EffectCapabilityMismatch {
                effect: reported_effect,
                expected,
                actual,
            } => {
                assert_eq!(reported_effect, effect, "报的不是这条效应");
                assert_eq!(
                    expected,
                    CapabilityKind::for_effect(effect),
                    "报的应得 kind 不是 for_effect 给出的那一枚"
                );
                assert_eq!(actual, wrong_kind, "报的实际 kind 不是被拒的那一枚");
            }
            other => panic!("期望 EffectCapabilityMismatch，得到 {other:?}"),
        }
    }
}

/// 一枚**不与任何 `EffectType` 对应**的能力（`git.read`）同样被拒。
///
/// `git.read` 是 [`CapabilityKind`] 里的成员，但**不在 `for_effect` 的像里**（像只有
/// 六个：`email.send` / `git.push` / `registry.publish` / `git.delete_remote` /
/// `payment.charge` / `environment.deploy`）。轮转表用的那枚像外 kind 是
/// `filesystem.read`，本条换成一枚**不同的**像外 kind——两条臂的**输入不同**，
/// 各自的落点也具名可查。
///
/// **本条不声称它挡住了轮转表挡不住的形状**：轮转表的六条反例本身也都用像外的
/// `filesystem.read`，故「像外 kind 被拒」这条性质那六条已经覆盖；本条是它的第二个
/// 输入、不是第二种形状。
#[test]
fn a_capability_outside_the_effect_vocabulary_is_rejected() {
    let err = AuthorizedEffect::new(
        EffectType::Charge,
        minted(CapabilityKind::Git(continuum_capability::GitAction::Read)),
    )
    .expect_err("git.read 不得当作 charge 已获准");
    assert!(
        matches!(
            err,
            continuum_capability::CapabilityError::EffectCapabilityMismatch { .. }
        ),
        "期望 EffectCapabilityMismatch，得到 {err:?}"
    );
}
