//! §12.2 第 (s) 条的照片：`bind_media_methods` 填出的四条 `video/` 方法的
//! `realized_by`（设计 §10；P5b 设计第六节交办的判据）。
//!
//! 本文件同时持有 `all_operators()` 与 `MethodRegistry`，故是 P5b 设计 §3.2 末
//! 所说「消费块才能核」的那个消费块之一——`realized_by` 是**文本弱引用**
//! （`Vec<String>`，P5b 设计 §4.3），**没有编译期照片**，核对只能落在本文件的
//! 运行期用例。
//!
//! | 用例 | 钉的判据 | 红条件（档位） |
//! |---|---|---|
//! | `every_video_method_gets_at_least_one_realized_by_and_each_string_resolves` | (s) 的两条腿：逐条非空 ＋ 每个串可 resolve | 腿 (i) **放宽档**：把 `render-review` 的 `realized_by` 写成空 `Vec`；腿 (ii) **取反档**：某一枚里写一个非算子的串（如 `"render-review"` 自己） |
//! | `binding_does_not_add_or_remove_any_method_entry` | §10 的「本函数不做二次登记」（四块**不得**自造 `MethodEntry`） | **取反档**：`bind_media_methods` 内部顺手 `register` 一条新条目 ⇒ 数量变化、红 |
//! | `binding_an_unseeded_id_returns_not_found` | 失败路径：未 seed 的 id ⇒ `NotFound` **且是哪一条 id**；错误类型直接透出 | **取反档**：吞掉错误返回 `Ok`／透出别的 id ⇒ 红 |
//!
//! **四条 id 的语料手写**（设计 §10 表的 `MethodId` 列照录，行序即表序）：
//! 若期望值取自 `seeded()` 或实现自身，测的是「它等于它自己」——那是恒真的假照片。

use continuum_media::{all_operators, bind_media_methods, register_media_operators};
use continuum_method::{MethodDomain, MethodError, MethodId, MethodRegistry};
use continuum_operator::{OperatorId, OperatorRegistry, OperatorVersion};

/// 设计 §10 表的四条 `MethodId`（次序即表行序），**手写照录**。
const VIDEO_METHOD_IDS: [&str; 4] = [
    "shot-analysis",
    "narrative-plan",
    "timeline-compose",
    "render-review",
];

/// 某个域的条目 id 序列（`select` 已按 id 升序，P5b 设计 §4.5）。
/// 用于「bind 前后条目集不变」的快照比较——快照取 id 文本，比较即逐名。
fn ids_of(registry: &MethodRegistry, domain: MethodDomain) -> Vec<String> {
    registry
        .select(domain)
        .iter()
        .map(|entry| entry.id.as_str().to_string())
        .collect()
}

/// 第 (s) 条（**两条腿**）：先 `seeded()` 再 `bind_media_methods`（必须返回 `Ok`）——
/// （i）四条 `video/` 方法的 `realized_by` **逐条非空**；
/// （ii）对每一条 `realized_by` 里的**每一个串**，装满 17 枚算子之后的
/// `OperatorRegistry::resolve` **必须成功**。
///
/// **红条件（放宽档，腿 (i)）**：把 `render-review` 的 `realized_by` 写成空 `Vec` ⇒ 红
/// （`has_counterpart(Video) == true`，空即「漏 bind」，P5b 设计 §4.5）。
///
/// **红条件（取反档，腿 (ii)）**：把某一枚的 `realized_by` 里写一个非算子的串
/// （如 `"render-review"` 自己——它是方法名，不是算子 id）⇒ 红。
///
/// **本条的限度**（P5b 设计 §3.2 末）：`realized_by` 是**文本弱引用**，**没有编译期照片**
/// ——写错的串照样编得过（`continuum-method` 不解析它、也看不到 `OperatorRegistry`）。
/// 本条是运行期的补件：它只能断「非空」与「每个串都能 resolve」，
/// **断不了「绑的是不是对的算子」**（那是设计 §10 表的判定，对照物是 §32/§33 的步骤名，
/// 规范只给了名）。本条正是 P5b 所说「消费块才能核」的那个消费块之一。
#[test]
fn every_video_method_gets_at_least_one_realized_by_and_each_string_resolves() {
    let mut registry = MethodRegistry::seeded();
    assert_eq!(
        bind_media_methods(&mut registry),
        Ok(()),
        "对 seeded() 调 bind_media_methods 必须返回 Ok（四条 video/ id 都已 seed）"
    );

    // 腿 (i)：四条 video/ 方法逐条非空。
    // 红条件（**放宽档**）：把 render-review 的 realized_by 写成空 Vec ⇒ 本循环红。
    for id in VIDEO_METHOD_IDS {
        let entry = registry
            .resolve(&MethodId::new(id))
            .unwrap_or_else(|e| panic!("seeded() 的 video/ 条目 {id} 必须存在：{e}"));
        assert!(
            !entry.realized_by.is_empty(),
            "video/{id} 的 realized_by 不得为空（P5b 设计 §4.5：has_counterpart(Video) == true ⇒ 空即漏 bind）"
        );
    }

    // 腿 (ii)：每个串都要能在装满 17 枚算子的 OperatorRegistry 里 resolve。
    // 红条件（**取反档**）：把某一枚的 realized_by 里写一个非算子的串（如 "render-review"
    // 自己——它是方法名，不是算子 id）⇒ 本循环红。
    let mut operators = OperatorRegistry::new();
    register_media_operators(&mut operators, &all_operators())
        .expect("17 枚算子必须全部注册成功（它们都是合法批次）");
    for id in VIDEO_METHOD_IDS {
        let entry = registry
            .resolve(&MethodId::new(id))
            .unwrap_or_else(|e| panic!("seeded() 的 video/ 条目 {id} 必须存在：{e}"));
        for name in &entry.realized_by {
            // version 一律为 1（§5.1 的不变量；all_operators() 逐枚如此）。
            operators
                .resolve(&OperatorId::new(name.as_str()), &OperatorVersion::new(1))
                .unwrap_or_else(|e| {
                    panic!("video/{id} 的 realized_by 串 {name} 必须能 resolve 到算子：{e}")
                });
        }
    }
}

/// §10 的「**本函数不做二次登记**」的照片：`bind_media_methods` 调用前后，
/// 条目集**不增不减**（四块**不得**自造 `MethodEntry`；`video/` 的四条 id
/// 由 P5b 的 `seeded()` 给出，本块只把这四条「每一条已 seed 的 id」逐条 `bind` 一次）。
///
/// 快照取**全部四个域**各自的 id 序列：用例名是「any method entry」，判据也不分域——
/// 只数 `video/` 的话，顺手 `register` 一条别的域的条目会漏网。
///
/// **红条件（取反档）**：让 `bind_media_methods` 内部顺手 `register` 一条新条目
/// ⇒ 数量变化、逐名比较红。
#[test]
fn binding_does_not_add_or_remove_any_method_entry() {
    let mut registry = MethodRegistry::seeded();

    // brief 的数量口径直译：video/ 恰四条（§187 照录；夹具前提）。
    assert_eq!(
        ids_of(&registry, MethodDomain::Video).len(),
        4,
        "seeded() 的 video/ 恰四条——数量口径的前提"
    );

    // 全域名册快照（bind 前）。select 已按 id 升序，故「序列相同」比「数量相同」更紧。
    let before: Vec<(MethodDomain, Vec<String>)> = MethodDomain::ALL
        .iter()
        .map(|domain| (*domain, ids_of(&registry, *domain)))
        .collect();

    assert_eq!(
        bind_media_methods(&mut registry),
        Ok(()),
        "对 seeded() 调 bind_media_methods 必须返回 Ok"
    );

    let after: Vec<(MethodDomain, Vec<String>)> = MethodDomain::ALL
        .iter()
        .map(|domain| (*domain, ids_of(&registry, *domain)))
        .collect();

    assert_eq!(
        after, before,
        "bind 前后条目集必须逐域逐名相同——§10：本函数不做二次登记（不 register、不构造 MethodEntry）"
    );
}

/// 失败路径：对**没有该条目**的注册表（`MethodRegistry::new()`）调
/// `bind_media_methods` ⇒ `Err(MethodError::NotFound { .. })`，**且是哪一条 id**。
///
/// 空表上四条 id 全不存在：实现按 §10 表的行序逐条 `bind`、`?` 短路，
/// 故返回的是**第一条**失败即 `shot-analysis`——本断言顺带钉住「按行序、fail-fast」，
/// 不是「返回了 `Err`」的弱断言。
///
/// **错误类型不并进 `MediaError`**：本函数的失败是「方法库拒绝了这次 bind」，
/// 判定、收件人与处置都不同，**故直接透出**——这与 §5.1 把 `OperatorError::Duplicate`
/// 包进 `MediaError::Registry` 是同一条判据的两面：**判定是同一件事则合并，是两件事则分开。**
///
/// **红条件（取反档）**：吞掉错误返回 `Ok`、或透出别的 id ⇒ 红。
#[test]
fn binding_an_unseeded_id_returns_not_found() {
    let mut registry = MethodRegistry::new();
    assert_eq!(
        bind_media_methods(&mut registry),
        Err(MethodError::NotFound {
            id: MethodId::new("shot-analysis")
        }),
        "空表上首条 shot-analysis 不存在——NotFound 必须直接透出且 id 正是它"
    );
}
