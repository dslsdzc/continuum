//! Task 4 的照片集：`register_image_operators` 的正例与两条例外臂
//! （§11.2 的第 (b)、(c)、(c2)、(d) 条）。
//!
//! **观测面写死**：`OperatorRegistry` 的 `entries` 是**私有字段、无迭代器、无 `len()`**
//! （`crates/continuum-operator/src/registry.rs:15-17`）——故「注册表的内容与调用前
//! 逐枚相同」**只能经 `resolve` 观测**：本文件**不比 `len()`**、不读私有字段。
//!
//! **期望语料手写**（设计 §11.2 的「一条预防」）：(b) 的五枚 id 清单、(c) 与 (c2)
//! 的夹具算子都**不从 `all_operators()` 派生**——语料从被测清单里取就是恒真的假照片。
//!
//! **不按下标取**：`all_operators()` 五行的次序重排是**等价变异体**（计划纪律 1(b) 的 (ii)，
//! 用例按 id 指认），故本文件不写 `all_operators()[0]` 这类下标表达式。

use continuum_artifact::ArtifactType;
use continuum_image::{ImageError, all_operators, register_image_operators};
use continuum_operator::{
    BackendId, Determinism, Operator, OperatorError, OperatorId, OperatorRegistry, OperatorVersion,
    SideEffectClass,
};

/// 设计 §3.2 表的五枚 id，**手写**（照表的行序；与 `tests/operators.rs` 的那一份
/// 各自独立——那份钉声明的逐行，这份钉「清单里每一枚都注册得进且读得回」）。
///
/// (b) 必须按这份清单逐枚 `resolve`、**不按 `all_operators()` 自己遍历**：
/// 后者的期望从被测物里取，是恒真的假照片（设计 §11.2 的「一条预防」）；
/// 若 `all_operators()` 少写一枚且返回类型也改小，本清单的对应枚会 `NotFound` 红
/// （设计 §11.3 末：「若改类型后仍不红，说明 b 是按 `all_operators()` 自己遍历的——即假照片」）。
const EXPECTED_IDS: [&str; 5] = [
    "generate-image",
    "detect-edit-region",
    "local-generative-edit",
    "hard-composite",
    "verify-outside-mask",
];

/// (c)、(c2)、(d) 的夹具：按 id 与 version 造一枚**七字段手写**的算子。
/// 其余五列取本域自洽的固定值——本文件的断言不读它们；「合法」在这里指
/// 七字段有值、自洽（本块没有运行期的声明校验：表与端口的比对在 Task 2／Task 3 的用例里）。
fn fixture_operator(id: &str, version: u32) -> Operator {
    Operator {
        id: OperatorId::new(id),
        version: OperatorVersion::new(version),
        input_schema: vec![ArtifactType::Image],
        output_schema: vec![ArtifactType::Image],
        determinism: Determinism::NonDeterministic,
        side_effect_class: SideEffectClass::Pure,
        backend_candidates: vec![BackendId::new("fixture-backend")],
    }
}

/// §11.2 第 (b) 条：以 `&all_operators()` 整批调 `register_image_operators` 后，
/// **逐枚** `resolve(&id, &OperatorVersion::new(1))` 成功，且**读回的那一枚的 `id`
/// 与期望相同**——期望取手写的 [`EXPECTED_IDS`]，不是 `all_operators()` 自己。
///
/// **红条件（移除档）**：把函数改成空实现（什么都不注册）⇒ 逐枚 `resolve` 得
/// `NotFound`，红。
#[test]
fn registering_the_whole_set_then_resolving_each_succeeds() {
    let mut registry = OperatorRegistry::new();
    register_image_operators(&mut registry, &all_operators()).expect("五枚整批应全部注册成功");

    for id in EXPECTED_IDS {
        let resolved = registry
            .resolve(&OperatorId::new(id), &OperatorVersion::new(1))
            .expect("清单里的每一枚都应 resolve 成功——批是整批注册的");
        assert_eq!(
            resolved.id,
            OperatorId::new(id),
            "读回的那一枚的 id 应与期望（手写清单）相同"
        );
    }
}

/// §11.2 第 (c) 条：对同一注册表、以**同一批**注册两次 ⇒ 第二次得
/// `Err(ImageError::Registry(OperatorError::Duplicate { .. }))`——**不是静默覆盖**。
///
/// **判定臂要判准**（不是「是 `Err` 就算过」）：`match` 穷尽解构出 `Duplicate`，
/// 并**逐字段**判准 `id` 与 `version` 两个字段的值——两枚期望都手写。
/// **锚点唯一**：`register.rs` 里把 `OperatorError` 转进 `ImageError::Registry`
/// 的那个 `?`（本函数只有这一处；「静默覆盖」的变异就瞄准它）。
///
/// **红条件（移除档）**：把重复注册改成静默覆盖（吞掉 register 的 `Err`）⇒
/// 第二次调用不再是 `Err`，红。
///
/// **夹具说明**：id 取 `"image-dup"`、version 取 2——两枚值都不与表内五枚撞、
/// version 不用到处出现的 1，故「实现把 version 丢掉」这类漂移在字段断言里也有照片；
/// 第二次的批**与第一次是同一个批**（同一枚算子再注册一遍）。
#[test]
fn registering_twice_yields_the_registry_arm_not_a_silent_overwrite() {
    let batch = [fixture_operator("image-dup", 2)];
    let mut registry = OperatorRegistry::new();

    register_image_operators(&mut registry, &batch).expect("第一次注册应成功");
    let err = register_image_operators(&mut registry, &batch)
        .expect_err("同一批第二次注册必须被拒——不得静默覆盖");

    match err {
        ImageError::Registry(OperatorError::Duplicate { id, version }) => {
            assert_eq!(
                id,
                OperatorId::new("image-dup"),
                "Duplicate 的 id 应是重复那一枚的 id"
            );
            assert_eq!(
                version,
                OperatorVersion::new(2),
                "Duplicate 的 version 应是重复那一枚的 version"
            );
        }
        other => {
            panic!("应为 ImageError::Registry(OperatorError::Duplicate {{ .. }})，实际 {other:?}")
        }
    }
}

/// §11.2 第 (c2) 条（**先查重后写的写入面**）：被拒的批**一枚都不留**——
/// 注册表的内容与调用前逐枚相同。
///
/// **夹具的顺序是这条断言承重的一部分**：批 = `[新算子 A, 新算子 B, 算子 A]`，
/// **重复项排在批的末尾**；注册表里**已有**算子 A（A@v1，先经注册入口写入）：
/// - 「新算子 A」＝ A 的**新版本**（`"image-a"` 的 v2）——它的键不在表里；
/// - 「新算子 B」＝ 全新的一枚（`"image-b"` v1）；
/// - 「算子 A」（批尾）＝ 表内 A@v1 的克隆——同 (id, version) ⇒ 与表内重复。
///
/// **若把重复项排在首位，本用例在「边查重边写」那一档下也绿**——那一档逐枚注册、
/// 注册到违规枚才返回，违规枚在首位时它之前没有任何写入；「先查重后写」与
/// 「边查重边写」的差别只在一个**中间位置**上显形（这里：批尾的重复枚之前，
/// 「新算子 B」是否已被留下）。
///
/// **红条件（取反档）**：把函数改成边查重边写（先注册前几枚、遇到违规才返回）⇒
/// 「新算子 B」会被写入，它的 `resolve` 断言红；而 (c) **仍绿**（返回的 `Err` 臂没变）。
///
/// **观测方式写死**：`entries` 私有、无迭代器、无 `len()`（`registry.rs:15-17`）——
/// 「注册表内容与调用前逐枚相同」**只能经 `resolve` 观测**：**不比 `len()`**、
/// 不读私有字段。批里三枚逐枚拍到：① 表内 A@v1 读回**逐字段**同调用前
/// （`Operator` 的 `PartialEq` 同为逐字段派生，等值比较即逐字段比较）；
/// ② 「新算子 A」的新键 A@v2 必须 `NotFound`（「一个都没有留下」里它那一枚的落点——
/// 只拍 ①③ 的话，批首那枚是否被静默写入没有照片）；③ 「新算子 B」必须 `NotFound`。
#[test]
fn a_rejected_batch_leaves_the_registry_exactly_as_it_was() {
    let a_v1 = fixture_operator("image-a", 1);
    let a_v2 = fixture_operator("image-a", 2);
    let b_v1 = fixture_operator("image-b", 1);

    let mut registry = OperatorRegistry::new();
    register_image_operators(&mut registry, &[a_v1.clone()])
        .expect("先注册 A@v1——本用例的「表内已有」经注册入口写入");

    let before = registry
        .resolve(&OperatorId::new("image-a"), &OperatorVersion::new(1))
        .expect("调用前 A@v1 应在表里")
        .clone();

    let batch = [a_v2, b_v1, a_v1];
    let err = register_image_operators(&mut registry, &batch)
        .expect_err("批尾与表内既存同 (id, version)——整批必须被拒");

    match err {
        ImageError::Registry(OperatorError::Duplicate { id, version }) => {
            assert_eq!(
                id,
                OperatorId::new("image-a"),
                "被拒的应是批尾那一枚（算子 A）的 id"
            );
            assert_eq!(
                version,
                OperatorVersion::new(1),
                "被拒的应是批尾那一枚的 version（v1）"
            );
        }
        other => {
            panic!("应为 ImageError::Registry(OperatorError::Duplicate {{ .. }})，实际 {other:?}")
        }
    }

    // ① 表内那枚（A@v1）读回逐字段同调用前——被拒的批不接受任何改写。
    let after = registry
        .resolve(&OperatorId::new("image-a"), &OperatorVersion::new(1))
        .expect("A@v1 应在表里（调用前就在）")
        .clone();
    assert_eq!(after, before, "A@v1 读回应与调用前逐字段相同");

    // ② 「新算子 A」的键（A@v2）没有留下。
    assert!(
        registry
            .resolve(&OperatorId::new("image-a"), &OperatorVersion::new(2))
            .is_err(),
        "「新算子 A」的键（A@v2）不得留下——被拒的批不留半注册"
    );

    // ③ 「新算子 B」没有留下。
    assert!(
        registry
            .resolve(&OperatorId::new("image-b"), &OperatorVersion::new(1))
            .is_err(),
        "「新算子 B」不得留下——它是批里的中间位置，「边查重边写」会把它写进表里"
    );
}

/// §11.2 第 (d) 条（**(c) 的另一侧**）：一枚 id **不在 §3.2 表内**的合法算子，
/// 与 `all_operators()` 的整批**混合**注册 ⇒ 逐枚成功。
///
/// **这一条是 (c) 的另一侧**：只钉 (c) 的话，一个「一律拒」的实现全绿——
/// (c) 拍「重复必须拒」，本条拍「没重复时必须放行」。
///
/// **红条件（移除档）**：给 id 加一层白名单校验（只放行 `all_operators()` 里的 id）⇒
/// 本条红（整批被拒，或表外的这一枚缺席）。
///
/// **`OperatorId` 的取值域不由本块封闭**：`OperatorId` 是有字段的 newtype、
/// **不是枚举**（`crates/continuum-operator/src/definition.rs:24-31`）——
/// 本块**不发明**一个 id 白名单（那会是「什么叫本域算子」的第二套词汇表）。
#[test]
fn an_operator_with_an_id_outside_the_table_is_accepted() {
    let extra = fixture_operator("image-extra", 1);
    let mut batch: Vec<Operator> = all_operators().to_vec();
    batch.push(extra);

    let mut registry = OperatorRegistry::new();
    register_image_operators(&mut registry, &batch)
        .expect("表外 id 的合法算子必须被接受——本块不发明 id 白名单");

    for operator in &batch {
        let resolved = registry
            .resolve(&operator.id, &operator.version)
            .expect("批里每一枚（含表外的 image-extra）都应 resolve 成功");
        assert_eq!(
            resolved.id, operator.id,
            "读回的那一枚的 id 应与批里那一枚相同"
        );
    }
}
