// 正控制（应编译**通过**）：Task 6 加进来的**公开面确实可用**——
// `ResolutionRisk` 可构造、`RequiredStrength::new` 可用、`resolve` 的五参数签名可用、
// `ExplicitBinding` 可构造、`binding::apply` 可用、`BindingSource::User` 可达。
//
// 没有这一份，`tests/compile_fail/` 下那三份反例的绿可以来自任何与判据无关的原因
// （`RequiredStrength` 这个名字不存在、`resolve` 的名字改了、`ResolutionRisk` 的字段
// 写不出来……）。这就是本仓「正控制不得自证」那条纪律要的形状：
// 反例断「那几条路走不通」，正例断「该走得通的那条走得通」。
//
// 它同时钉住四件事，各一句：
// 1. `RequiredStrength` / `ResolutionConfidence` 都从**不透明值**来（§16 第 6 条同形）；
// 2. `RequiredStrength` **有且只有** `new` 这一个公开构造入口（没有 `Default`、没有 `from_risk`）
//    —— 反例是 `required_strength_has_no_default.rs` 与 `required_strength_has_no_from_risk.rs`；
// 3. `resolve` 的签名**恰好五个参数**，第二个是 `Option<(&ExplicitBinding, EntityKind)>`
//    —— 反例是 `resolve_requires_the_strength_argument.rs`（少传 `required`、以及把它传成 `None`）；
// 4. §4.6 的注入点 `binding::apply` 从 crate 外可达，且它交出的工作集能被喂回 `resolve`。
//
// **本样例只到「可构造、可调、签名对得上」为止**：判据本身（缩而不加权、注入两侧、
// 约束不被豁免）由 `tests/risk.rs` 断言，不在这里再断一遍——一份样例守两句要求的话，
// 那句额外的断言会成为**别的**变异体的第二个失败面，让「谁红」的预告与实际对不上。
use continuum_canonical::{
    BindingSource, Candidate, CandidateFacts, EntityId, EntityKind, ExplicitBinding,
    ExplicitConstraint, Mention, MentionKind, Opaque, RequiredStrength, ResolutionConfidence,
    ResolutionRisk, ResolutionScope, binding, resolve,
};

fn main() {
    // §276 的四个风险事实：字段是 `pub` 的，crate 外拼得出。
    let risk = ResolutionRisk {
        effect: Opaque::from_str("effect"),
        cost: Opaque::from_str("cost"),
        reversibility: Opaque::from_str("reversibility"),
        privacy: Opaque::from_str("privacy"),
    };
    assert_eq!(risk.effect.as_str(), "effect");

    // 门槛与置信：各自只有一条公开构造入口，两枚都从**不透明值**来。
    let required = RequiredStrength::new(Opaque::from_str("calibrated"));
    let confidence = ResolutionConfidence::new(Opaque::from_str("calibrated"));
    assert!(confidence.meets(&required), "同档 ⟹ 放行");

    // §277 的绑定，四字段照录。
    let b = ExplicitBinding {
        mention: Mention {
            span: 0..4,
            text: "tool".to_owned(),
            kind: MentionKind::from_str("noun_phrase"),
        },
        entity: EntityId::from_str("tool.example"),
        scope: ResolutionScope::Project,
        source: BindingSource::User,
    };

    let retrieved: Vec<(Candidate, CandidateFacts)> = vec![(
        Candidate {
            entity_id: EntityId::from_str("tool.other"),
            evidence: Vec::new(),
            retrieval_score: Some(0.5),
        },
        CandidateFacts {
            scope: ResolutionScope::Global,
            kind: EntityKind::Tool,
        },
    )];
    let constraints = [ExplicitConstraint::Scope(ResolutionScope::Project)];

    // §4.6 的注入点：crate 外可达，且它的输出能喂回 `resolve`。
    let working = binding::apply(&retrieved, Some((&b, EntityKind::Tool)));
    assert_eq!(
        working.len(),
        retrieved.len() + 1,
        "点名的实体不在检索集里 ⟹ 注入一枚"
    );

    // 五参数签名：`candidates` / `injection` / `constraints` / `confidence` / `required`。
    let r = resolve(
        &retrieved,
        Some((&b, EntityKind::Tool)),
        &constraints,
        &confidence,
        &required,
    );
    assert!(
        matches!(r, continuum_canonical::ResolutionResult::Resolved(_)),
        "注入的那一枚满足约束、门槛已过 ⟹ `Resolved`",
    );
}
