// 正控制（应编译**通过**，且运行时通过）：一个**只实现 `retrieve`** 的 `CandidateSource`
// 实现编得过 ⇒ 该 trait 的**必需**方法集恰好是那一个。
//
// 这一份同时是两枚变异体的哨兵：
//   (i) 「给 trait 加第二个**必需**方法（无默认实现）」——本样例立刻编不过；
//   (ii) 「给 trait 加一个返回单实体的方法并给它默认实现」——本样例仍编得过，
//        那时红的是 `tests/compile_fail/candidate_source_has_no_single_entity_method.rs`
//        （两份样例各守一侧，这就是「守卫须两侧都钉」）。
//
// 它顺带钉住「`Candidate` 的字段是 `pub`、且就是 §272 那三个」：
// 这份样例在 **crate 外**用结构体字面量拼了一枚候选，字段少一个或多一个都编不过。
use continuum_canonical::{Candidate, CandidateSource, EntityId, Mention, detect, normalize};

/// 只实现 `retrieve`，**没有别的实现**——这就是本样例要证的那句话。
struct OnlyRetrieve;

impl CandidateSource for OnlyRetrieve {
    fn retrieve(&self, _m: &Mention) -> Vec<Candidate> {
        vec![Candidate {
            entity_id: EntityId::from_str("only"),
            evidence: vec!["固定来源".to_owned()],
            retrieval_score: None,
        }]
    }
}

fn main() {
    let surface = normalize("请把它改掉");
    let mentions = detect(&surface);
    assert_eq!(mentions.len(), 1, "夹具前提：恰好一枚 mention");

    let got = OnlyRetrieve.retrieve(&mentions[0]);
    assert_eq!(got.len(), 1, "返回值是集合，本样例只交一枚");
    assert_eq!(got[0].entity_id.as_str(), "only");
    assert!(got[0].retrieval_score.is_none(), "缺席就是 None");
}
