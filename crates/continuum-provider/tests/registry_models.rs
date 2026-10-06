//! 模型侧的登记与发现（设计 §3.1，§11）。

mod common;

use common::FakeModel;
use continuum_core::model::ModelId;
use continuum_core::ProviderError;
use continuum_provider::model::ModelProvider;
use continuum_provider::{ProviderRegistry, RegistryError};
use std::sync::Arc;

fn model(id: &str) -> ModelId {
    ModelId::new(id)
}

fn fake() -> Arc<dyn ModelProvider> {
    Arc::new(FakeModel)
}

#[test]
fn a_registered_model_is_found_by_id() {
    let mut registry = ProviderRegistry::new();
    let adapter = fake();
    registry
        .register_model(vec![model("fake-1")], Arc::clone(&adapter))
        .expect("首次登记应成功");

    let found = registry.model_for(&model("fake-1")).expect("已登记的 id 应命中");
    assert!(
        Arc::ptr_eq(&found, &adapter),
        "命中的必须是登记时那一个 Arc；行为相同的新适配器不算命中"
    );
}

#[test]
fn an_unregistered_model_id_is_reported_as_not_found() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_model(vec![model("fake-1")], fake())
        .expect("首次登记应成功");

    // 用 `.err()` 而非 `expect_err()`：`Arc<dyn ModelProvider>` 不是 `Debug`，
    // 而 `expect_err` 要求成功侧可打印。
    let err = registry
        .model_for(&model("nope"))
        .err()
        .expect("未登记的 id 不该命中");
    assert!(
        matches!(err, RegistryError::NotFound { .. }),
        "必须是 NotFound 这一种 Err，实际 {err:?}"
    );
}

#[test]
fn registering_the_same_id_twice_is_rejected_as_duplicate() {
    let mut registry = ProviderRegistry::new();
    let first = fake();
    registry
        .register_model(vec![model("fake-1")], Arc::clone(&first))
        .expect("首次登记应成功");

    let second = fake();
    let err = registry
        .register_model(vec![model("fake-1")], Arc::clone(&second))
        .expect_err("同一个 id 登记第二次必须被拒");
    assert!(
        matches!(err, RegistryError::Duplicate { .. }),
        "必须是 Duplicate 这一种 Err，实际 {err:?}"
    );

    let still = registry.model_for(&model("fake-1")).expect("被拒的登记不该动原条目");
    assert!(
        Arc::ptr_eq(&still, &first),
        "原条目必须还是第一次登记的那个 Arc；被第二次的覆盖即半登记"
    );
}

#[test]
fn registering_a_group_of_ids_is_all_or_nothing() {
    let mut registry = ProviderRegistry::new();
    let occupant = fake();
    registry
        .register_model(vec![model("b")], Arc::clone(&occupant))
        .expect("首次登记应成功");

    let newcomer = fake();
    let err = registry
        .register_model(vec![model("a"), model("b")], Arc::clone(&newcomer))
        .expect_err("组内有已占用的 id，整组必须被拒");
    assert!(
        matches!(err, RegistryError::Duplicate { .. }),
        "必须是 Duplicate 这一种 Err，实际 {err:?}"
    );

    let free = registry
        .model_for(&model("a"))
        .err()
        .expect("组内未被占用的 id 也不许进去——半登记的注册表无人看得见");
    assert!(
        matches!(free, RegistryError::NotFound { .. }),
        "a 应是 NotFound 这一种 Err，实际 {free:?}"
    );

    let kept = registry.model_for(&model("b")).expect("b 仍是先登记的那个");
    assert!(Arc::ptr_eq(&kept, &occupant), "b 不该被这次被拒的登记改写");
}

#[tokio::test]
async fn a_registered_id_may_be_absent_from_the_adapters_own_list_models() {
    let mut registry = ProviderRegistry::new();
    let adapter = fake();
    // `FakeModel::list_models()` 只含 "fake-1"；登记按 id 显式给出，故两者可以不一致
    // （设计 §3.3 代价一）。
    registry
        .register_model(vec![model("m")], Arc::clone(&adapter))
        .expect("登记应成功");

    let routed = registry
        .model_for(&model("m"))
        .expect("登记是路由的权威：适配器不认它也不影响命中");
    assert!(Arc::ptr_eq(&routed, &adapter), "路由到的应是登记的那个适配器");

    let err = routed
        .describe_model(&model("m"))
        .await
        .err()
        .expect("适配器的 list_models 不含 m，故它给不出描述");
    assert!(
        matches!(err, ProviderError::UnknownModel(_)),
        "描述的权威在适配器一侧，它应报自己的 UnknownModel，实际 {err:?}"
    );
}
