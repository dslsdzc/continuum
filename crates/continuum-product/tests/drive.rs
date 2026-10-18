//! §4.7 的相位机（J17a）：只前进、不回退。
//!
//! 这里的 `Shippable` 是**合成的**：`evaluate_shippable` 今天没有正臂，
//! 所以它不是任何生产判定的产物。

use continuum_product::{
    ContinuousProductDrive, NotShippableReason, ProductDriveState, ProductPhase,
    ShippableUndetermined, ShippableVerdict, advance_product_phase,
};

fn shippable() -> ShippableVerdict {
    ShippableVerdict::Shippable
}

fn not_shippable() -> ShippableVerdict {
    ShippableVerdict::NotShippable {
        reason: NotShippableReason::CompletionBlocked,
    }
}

fn undetermined() -> ShippableVerdict {
    ShippableVerdict::Undetermined {
        reason: ShippableUndetermined::MissingProduct,
    }
}

fn all_verdicts() -> [ShippableVerdict; 3] {
    [shippable(), not_shippable(), undetermined()]
}

#[test]
fn pre_shippable_advances_only_on_shippable() {
    assert_eq!(
        advance_product_phase(ProductPhase::PreShippable, &shippable()),
        ProductPhase::Shippable
    );
    assert_eq!(
        advance_product_phase(ProductPhase::PreShippable, &not_shippable()),
        ProductPhase::PreShippable
    );
    assert_eq!(
        advance_product_phase(ProductPhase::PreShippable, &undetermined()),
        ProductPhase::PreShippable
    );
}

#[test]
fn shippable_is_not_overwritten_by_not_shippable_or_undetermined() {
    assert_eq!(
        advance_product_phase(ProductPhase::Shippable, &shippable()),
        ProductPhase::Shippable
    );
    assert_eq!(
        advance_product_phase(ProductPhase::Shippable, &not_shippable()),
        ProductPhase::Shippable
    );
    assert_eq!(
        advance_product_phase(ProductPhase::Shippable, &undetermined()),
        ProductPhase::Shippable
    );
}

#[test]
fn post_shippable_is_never_overwritten() {
    for verdict in all_verdicts() {
        assert_eq!(
            advance_product_phase(ProductPhase::PostShippableImprovement, &verdict),
            ProductPhase::PostShippableImprovement
        );
    }
}

#[test]
fn pre_shippable_never_jumps_to_post() {
    for verdict in all_verdicts() {
        assert_ne!(
            advance_product_phase(ProductPhase::PreShippable, &verdict),
            ProductPhase::PostShippableImprovement
        );
    }
}

#[test]
fn the_drive_defaults_to_disabled() {
    assert!(!ContinuousProductDrive::default().enabled);
    assert_eq!(
        ProductDriveState::default().phase,
        ProductPhase::PreShippable
    );
    assert!(!ProductDriveState::default().continuous_product_drive.enabled);
}
