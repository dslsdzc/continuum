//! §5.4 的「逐位可复现 backend」注册期规则的守卫（设计 §12.2 的 (h)(i)）。
//!
//! (h) 的语料（九枚 backend 名单）**手写**、逐名取自设计 §5.4 的封闭表：
//! 若从实现侧的常量取语料，测的是「它等于它自己」——那是恒真的假照片
//! （设计 §12.2 末对 (f) 条的点名同形；实现侧那份常量在 `continuum-media` 内是私有的）。
//! 名单两侧（实现常量与用例语料）各自从设计照抄：实现侧抄错时 (i) 的合法批次被拒
//! 或违规算子被放行，抄错即红。
//! (i) 的语料（违规算子与两枚合法算子）同样是**手写的七字段字面量**
//! （`Operator` 七字段全 `pub`，可以直接构造，`crates/continuum-operator/src/definition.rs:79-88`）。

use continuum_artifact::ArtifactType;
use continuum_media::{MediaError, all_operators, register_media_operators};
use continuum_operator::{
    BackendId, Determinism, Operator, OperatorError, OperatorId, OperatorRegistry, OperatorVersion,
    SideEffectClass,
};

/// §5.4 的「逐位可复现的 backend」名单（封闭表，九枚），**手写照录**（设计 §5.4）。
///
/// 这份语料必须手写：它对照的是设计原文的九枚，不是实现侧的常量
///（正控制不得自证——若从被测清单里取语料，测的是「它等于它自己」）。
const BIT_REPRODUCIBLE_BACKENDS: [&str; 9] = [
    "local-fs",
    "ffmpeg",
    "ffmpeg-scene-detect",
    "ffmpeg-filtergraph",
    "ffprobe",
    "pyscenedetect",
    "librosa",
    "essentia",
    "builtin",
];

/// 第 (h) 条（正例侧）：`all_operators()` 里 `determinism == Deterministic` 的
/// **每一枚**算子，其**每一个** backend 候选都在 §5.4 的名单内。
///
/// **红条件（取反档）**：给 `render` 的候选里加 `nvenc`（硬件编码器）⇒
/// 遍历到它时红。
///
/// **偏离最显然写法处**：最显然的写法是给 `render` 放一个硬件编码器（性能更好）；
/// **这里不是那样：因为硬件编码器的输出不保证与软件编码器逐位一致**，混入即让该
/// 算子的 `Deterministic` 声明为假，而那条声明的消费者是 §305 的复用。
/// 代价是硬件加速不可用——**性能代价，不是正确性代价**（设计 §5.4）。
///
/// **这条规则的规范来源：没有。** §245 只给「一个 Operator 多 backend」，不给
/// backend 的确定性；名单本身是本设计定的（设计 §14 第 7 条，收件人：规范维护者）。
///
/// **本条按 `determinism` 的取值挑选要遍历的算子，但不断言那个取值**（设计 §12.2 的
/// (h) 行注记）：把一枚 `Deterministic` 改成 `NonDeterministic` 只让它少遍历一枚，
/// 本条仍绿——`determinism` 取值本身的承担者另有其用例（复用侧的六枚）。
///
/// **本条钉的射程**：它钉的是「算子声明的每一个候选都在**手写名单**内」；
/// 实现侧那份名单的「不少」由第 (i) 条约束（少一枚 ⇒ 合法批次被拒），
/// 「不多出 `nvenc`」也由第 (i) 条约束（多出即违规算子被放行）；
/// 名单多出**其他**名字这一格没有照片，据实记在此处。
#[test]
fn every_deterministic_operator_declares_only_bit_reproducible_backends() {
    let operators = all_operators();
    for operator in &operators {
        if operator.determinism != Determinism::Deterministic {
            continue;
        }
        for backend in &operator.backend_candidates {
            assert!(
                BIT_REPRODUCIBLE_BACKENDS.contains(&backend.as_str()),
                "{} 声明 Deterministic，但其候选 backend {} 不在 §5.4 的名单内",
                operator.id,
                backend
            );
        }
    }
}

/// 第 (i) 条（fail-open 的反面）：一枚 `Deterministic` 但候选含名单外 backend 的
/// 算子，连同若干**合法**算子一次调 `register_media_operators` 时——
/// （i）返回 `Err(MediaError::UnverifiableDeterminism { operator, backend })`，
/// **且 `operator` 与 `backend` 正是那一枚**（不是「返回了那一臂」）；
/// （ii）**注册表的内容与调用前逐枚相同**（先核后写，不留半注册）。
///
/// **红条件（取反档）**：
/// （i）把名单判定写成恒真 ⇒ 返回 `Ok`，红；
/// （ii）把实现改成**边核边写**（先注册前几枚，遇到违规才返回）⇒ 注册表里
/// 多出违规算子**之前**的那几枚，红。
///
/// **这两半是「同一处规则的写入面」**：只钉返回值那一半会让「半注册」漂过去
/// （设计 §12.2 的 (i) 行）。
#[test]
fn an_out_of_list_backend_is_rejected_before_anything_is_written() {
    /// 违规批次里的一枚合法算子（`Deterministic` 且候选全在 §5.4 名单内）。
    /// 构造器收 id：批次里要放两枚 id 不同的合法算子。
    fn legit_batch_operator(id: &str) -> Operator {
        Operator {
            id: OperatorId::new(id),
            version: OperatorVersion::new(1),
            input_schema: vec![ArtifactType::Video],
            output_schema: vec![ArtifactType::Video],
            determinism: Determinism::Deterministic,
            side_effect_class: SideEffectClass::Pure,
            backend_candidates: vec![BackendId::new("ffmpeg")],
        }
    }

    // 违规算子：形如「给 shot-detect 的候选混进一个硬件编码器」。
    // 候选里保留一枚合法的 `pyscenedetect`：规则是**逐个**候选都要在名单内，
    // 不是「有一枚在内即可」；返回的 `backend` 应是遍历到的那一枚名单外的。
    let rogue = Operator {
        id: OperatorId::new("shot-detect-hw"),
        version: OperatorVersion::new(1),
        input_schema: vec![ArtifactType::Video],
        output_schema: vec![ArtifactType::ShotSet],
        determinism: Determinism::Deterministic,
        side_effect_class: SideEffectClass::Pure,
        backend_candidates: vec![BackendId::new("pyscenedetect"), BackendId::new("nvenc")],
    };

    // `legit-batch-first` 排在违规算子之前：边核边写的变体会把它写进注册表 (ii) 的红位。
    // `legit-batch-last` 排在之后：它钉「违规点之后的部分同样没被写」。
    let first_legit = legit_batch_operator("legit-batch-first");
    let last_legit = legit_batch_operator("legit-batch-last");

    // 先注册一个合法批次（也是 (ii) 的「调用前状态」）。
    let mut registry = OperatorRegistry::new();
    register_media_operators(&mut registry, &all_operators()).expect("合法批次应全部注册成功");

    // 对照集 = 合法批次的所有 id（调用前已注册，`Ok`）＋ 违规批次的三个 id
    //（调用前未注册，`NotFound`）。两侧都有条目，「逐枚相同」才不是真空的。
    let mut watched: Vec<OperatorId> = all_operators().iter().map(|op| op.id.clone()).collect();
    watched.push(first_legit.id.clone());
    watched.push(rogue.id.clone());
    watched.push(last_legit.id.clone());

    let snapshot =
        |registry: &OperatorRegistry| -> Vec<(OperatorId, Result<Operator, OperatorError>)> {
            watched
                .iter()
                .map(|id| {
                    (
                        id.clone(),
                        registry.resolve(id, &OperatorVersion::new(1)).cloned(),
                    )
                })
                .collect()
        };
    let before = snapshot(&registry);

    let batch = [first_legit.clone(), rogue.clone(), last_legit.clone()];
    let err = register_media_operators(&mut registry, &batch)
        .expect_err("候选含名单外 backend 的批次必须被拒，不得放行");

    // 第一半：错误臂的两枚字段**正是那一枚**。
    match err {
        MediaError::UnverifiableDeterminism { operator, backend } => {
            assert_eq!(operator, rogue.id, "operator 字段应是违规那一枚的 id");
            assert_eq!(
                backend,
                BackendId::new("nvenc"),
                "backend 字段应是名单外的那一枚"
            );
        }
        other => panic!("应为 UnverifiableDeterminism，实际 {other:?}"),
    }

    // 第二半：注册表的内容与调用前逐枚相同（先核后写，不留半注册）。
    let after = snapshot(&registry);
    assert_eq!(
        before, after,
        "违规批次被拒后，注册表内容必须与调用前逐枚相同（先核后写，不留半注册）"
    );
}
