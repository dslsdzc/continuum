//! Task 3 的照片：`MethodEntry` 与 `MethodRegistry` 的登记／解析／绑定／选取（设计 §4.3／§4.5）。
//!
//! 夹具**全部手建**——本 task 尚无目录（`seeded()` 在 Task 4），本文件不依赖它。
//!
//! | 用例 | 钉的判据 | 红条件（档位） |
//! |---|---|---|
//! | `register_rejects_a_second_entry_with_the_same_id` | 判据 4 的 `Duplicate` 侧，**并钉「不覆盖」这一侧** | **移除档**（fail-open 侧）：把 `contains_key` 判定删掉、直接 `insert` ⇒ **两条断言都红**（第一次 `register` 变 `Ok`，且 `resolve` 读回 A'） |
//! | `resolve_returns_not_found_for_an_unregistered_id` | 判据 4 的 `resolve: NotFound` 侧 | **取反档**：改成返回 `Ok`（panic 不是本用例要的错型）⇒ 红 |
//! | `bind_returns_not_found_for_an_unregistered_id_and_creates_nothing` | 判据 4 的 `bind: NotFound` 侧，**并钉「不新建条目」这一侧** | **放宽档**（fail-open 侧）：让 `bind` 对未注册 id 走插入路径、替掉存在性判定 ⇒ **两条断言都红** |
//! | `binding_twice_overwrites_the_previous_value` | 设计 §5.1 的覆盖语义＋「`bind` 只碰 `realized_by`」 | **取反档**：把覆盖改成追加（`bind` 用 `extend`）⇒ 第一条断言红 |
//! | `select_returns_only_the_entries_of_the_requested_domain` | `select` 只要对的（恰为 `{a, b}`，逐名） | **放宽档**：把 domain 过滤删掉 ⇒ 红 |
//! | `select_is_ordered_by_id_ascending` | 设计 §10 判据 5 的「按 id 升序」这一半 | **取反档**：把排序键取反 ⇒ 逐位断言红（「移除排序」是弱变异体，见用例内注释） |
//! | `select_of_a_domain_without_entries_is_empty` | 没有时给空而不是造条目 | **放宽档**：把 domain 过滤删掉（与上上条同一个变异）⇒ 返回非空、红 |
//! | `method_entry_has_exactly_three_public_fields` | `MethodEntry` 字段集（纪律 5） | **编译期照片**（纪律 1(c)：加字段 ⇒ 缺字段 `E0063`；删字段 ⇒ 多出字段名 `E0560`；**本条没有运行期红**） |
//!
//! # 射程与归属
//!
//! - **`Duplicate` 在生产路径上不可达**（本文件第一、三条用例的注释各写一处，另两处
//!   在 Task 4 Step 3 的 `seeded()` 与 Task 5 Step 4 的判据 4 一行）：`seeded()` 的
//!   十八个 id 互不相同（§187 的清单无重名），而 `register` 在本 crate 的唯一生产
//!   调用者就是 `seeded()`——故 `Duplicate` 的照片只能是本文件里**手建**的两条同 id
//!   条目。这不是缺口，是判据 4 的射程。
//! - **两处 fail-open 侧**（纪律 6）：`register` 的「不覆盖」与 `bind` 未注册时的
//!   「不新建条目」——只断「返回了 `Err`」会漏掉「顺手把旧值覆盖了／把条目建出来了」，
//!   故两处都额外断言了副作用面。
//! - **`method_entry_has_exactly_three_public_fields` 是编译期照片**，没有运行期红
//!   （纪律 1(c)）；本文件其余七条都有运行期红（档位见上表）。
//! - 设计 §10 判据 5 的**「同一 registry 两次 `select` 给出同一序列」那一半不在此列**：
//!   同一进程内不排序的 `HashMap` 迭代序也稳定，那条断言抓不到任何东西（等价变异体，
//!   纪律 6）——据实记为无判据的加强句，见 `select_is_ordered_by_id_ascending` 内注释。

use continuum_method::{MethodDomain, MethodEntry, MethodError, MethodId, MethodRegistry};

/// 手建一条 [`MethodEntry`]——本 task 尚无目录（`seeded()` 在 Task 4），
/// 本文件所有夹具都由它或等价的字面量直接构造，不依赖 `seeded()`。
fn entry(id: &str, domain: MethodDomain, realized_by: &[&str]) -> MethodEntry {
    MethodEntry {
        id: MethodId::new(id),
        domain,
        realized_by: realized_by.iter().map(|s| s.to_string()).collect(),
    }
}

#[test]
fn register_rejects_a_second_entry_with_the_same_id() {
    // **照实写明本用例的射程**：`seeded()`（Task 4）的十八个 id 互不相同
    // （§187 的清单无重名），而 `register` 在本 crate 的唯一生产调用者就是
    // `seeded()`，故 `Duplicate` 在生产路径上**不可达**——它的照片只能是本用例里
    // **手建**的这两条同 id 条目。这不是缺口，是判据 4 的射程
    // （同一句话另两处在 Task 4 Step 3 的 `seeded()` 与 Task 5 Step 4 的判据 4 一行）。
    let id = MethodId::new("dup");
    let a = entry("dup", MethodDomain::Software, &["op-a"]);
    // A'：同 id、不同 domain 与 realized_by——若第一次登记被覆盖，读回的会是这条。
    let a_prime = entry("dup", MethodDomain::Video, &["op-a-prime"]);

    let mut registry = MethodRegistry::new();
    assert_eq!(registry.register(a.clone()), Ok(()), "第一次登记必须成功");

    // 红条件（**移除档**，fail-open 侧）：把 `contains_key` 判定删掉、直接 `insert`
    // ⇒ 本断言红（返回 `Ok` 而不是 `Duplicate`）。
    assert_eq!(
        registry.register(a_prime),
        Err(MethodError::Duplicate { id: id.clone() }),
        "同 id 的第二次登记必须返回 Duplicate"
    );

    // **「另一侧」是不覆盖**：只断「返回了 `Duplicate`」会漏掉「同时也把旧值覆盖了」
    // ——那正是 fail-open 的方向。故读回条目，必须还是 A。
    assert_eq!(
        registry.resolve(&id),
        Ok(&a),
        "第二次登记被拒之后，第一次登记的那条（A）不得被覆盖"
    );
}

#[test]
fn resolve_returns_not_found_for_an_unregistered_id() {
    // 红条件（**取反档**）：改成返回 `Ok`（panic 不是本用例要的错型）⇒ 红。
    let registry = MethodRegistry::new();
    assert_eq!(
        registry.resolve(&MethodId::new("nope")),
        Err(MethodError::NotFound {
            id: MethodId::new("nope")
        }),
        "未注册 id 的解析必须返回 NotFound"
    );
}

#[test]
fn bind_returns_not_found_for_an_unregistered_id_and_creates_nothing() {
    // 红条件（**放宽档**，fail-open 侧）：让 `bind` 对未注册 id 走插入路径、
    // 替掉存在性判定 ⇒ **两条断言都红**（`Err(NotFound)` 变成 `Ok`，且条目被新建）。
    // **写变异体时注意**：`entry(id).or_default()` 需要 `MethodEntry: Default`，
    // 而 `MethodEntry` **没有**派生 `Default`（Step 3 只给 `Debug, Clone, PartialEq, Eq`）
    // ——**字面照抄那一句是编译失败**，按纪律 1(c) 不算红。变异体要显式构造：
    // `entry(id).or_insert_with(|| MethodEntry { id: id.clone(), domain: MethodDomain::Software, realized_by: vec![] })`。
    // **这是本 task 的第二处 fail-open 侧**：只断「返回了 `Err`」会漏掉
    // 「顺手把条目建出来了」。
    let id = MethodId::new("nope");
    let mut registry = MethodRegistry::new();

    assert_eq!(
        registry.bind(&id, &["op"]),
        Err(MethodError::NotFound { id: id.clone() }),
        "对未注册 id 的绑定必须返回 NotFound"
    );

    // 另一侧：没有半写——`bind` 不得为未注册的 id 新建条目。
    assert_eq!(
        registry.resolve(&id),
        Err(MethodError::NotFound { id: id.clone() }),
        "bind 被拒之后，该 id 仍必须是未注册的（不得留下半写的条目）"
    );
}

#[test]
fn binding_twice_overwrites_the_previous_value() {
    // 本条钉的是设计 §5.1 的覆盖语义：一对一配对下覆盖不丢别人的贡献
    // （设计 §4.5「重复 bind 覆盖旧值」）；改成追加会让「同一块重复调它」
    // 变成只增不减。**「`bind` 只碰 `realized_by`」由第二条断言钉住。**
    // 红条件（**取反档**）：把覆盖改成追加 ⇒ 第一条断言红。
    let id = MethodId::new("target");
    let mut registry = MethodRegistry::new();
    assert_eq!(
        registry.register(entry("target", MethodDomain::Software, &["orig"])),
        Ok(()),
        "夹具登记必须成功"
    );

    assert_eq!(registry.bind(&id, &["a"]), Ok(()), "第一次 bind 必须成功");
    assert_eq!(
        registry.bind(&id, &["b", "c"]),
        Ok(()),
        "第二次 bind 必须成功"
    );

    let entry = registry.resolve(&id).expect("夹具条目必须可解析");
    assert_eq!(
        entry.realized_by,
        vec!["b".to_string(), "c".to_string()],
        "重复 bind 必须覆盖旧值（等于 [\"b\", \"c\"]，而不是 [\"a\", \"b\", \"c\"]）"
    );
    assert_eq!(entry.id, id, "bind 只碰 realized_by——id 未变");
    assert_eq!(
        entry.domain,
        MethodDomain::Software,
        "bind 只碰 realized_by——domain 未变"
    );
}

#[test]
fn select_returns_only_the_entries_of_the_requested_domain() {
    // 红条件（**放宽档**）：把 domain 过滤删掉 ⇒ 红（返回集里混进 video 的 c）。
    let mut registry = MethodRegistry::new();
    for fixture in [
        entry("a", MethodDomain::Software, &[]),
        entry("b", MethodDomain::Software, &[]),
        entry("c", MethodDomain::Video, &[]),
    ] {
        registry.register(fixture).expect("夹具登记必须成功");
    }

    let selected: Vec<&str> = registry
        .select(MethodDomain::Software)
        .iter()
        .map(|e| e.id.as_str())
        .collect();
    assert_eq!(
        selected,
        vec!["a", "b"],
        "select(Software) 必须恰为 {{a, b}}——逐名（不是「非空」），不多不少"
    );
}

#[test]
fn select_is_ordered_by_id_ascending() {
    // 以**逆字母序**插入（gamma、beta、alpha）——插入序与升序相反，
    // 若 `select` 返回插入序或 `HashMap` 迭代序，逐位断言即红。
    //
    // 红条件（**取反档**）：把排序键取反 ⇒ 逐位断言红。
    // **并写明「移除排序」是弱变异体**：`HashMap` 的迭代序不是插入序，删掉排序后
    // 本用例**极可能**红但**不保证**（小容量下偶然升序即等价变异体）
    // ——**故取反档才是本条的判据**（纪律 1(b)／7）。
    // **「同一 registry 两次 `select` 给出同一序列」不单列断言**：同一进程内不排序的
    // `HashMap` 迭代序也稳定，那条断言抓不到任何东西（等价变异体，纪律 6）。
    // **设计 §10 判据 5 的「按 id 升序」这一半由本用例兑现，「两次同序」那一半
    // 据实记为无判据的加强句**（实现计划 `## 遗留` 第二节已记）。
    let mut registry = MethodRegistry::new();
    for id in ["gamma", "beta", "alpha"] {
        registry
            .register(entry(id, MethodDomain::Software, &[]))
            .expect("夹具登记必须成功");
    }

    let selected: Vec<&str> = registry
        .select(MethodDomain::Software)
        .iter()
        .map(|e| e.id.as_str())
        .collect();
    assert_eq!(
        selected,
        vec!["alpha", "beta", "gamma"],
        "select 必须按 id 升序逐位给出 [alpha, beta, gamma]"
    );
}

#[test]
fn select_of_a_domain_without_entries_is_empty() {
    // 对只含 software 条目的 registry 调 `select(ThreeD)`：空 `Vec`。
    // 「不 panic」由下面的 `is_empty` 断言钉（返回空而不是 panic）；
    // 「不造条目」由签名钉——`select(&self)` 是不可变借用，运行期无从违反（编译期）。
    //
    // 红条件（**放宽档**）：把 domain 过滤删掉（与
    // `select_returns_only_the_entries_of_the_requested_domain` 同一个变异）
    // ⇒ 本用例返回非空、红。**这一条与那一条同源，但两侧射程不同**：
    // 那一条钉「只要对的」，本一条钉「没有时给空而不是造条目」。
    let mut registry = MethodRegistry::new();
    registry
        .register(entry("a", MethodDomain::Software, &[]))
        .expect("夹具登记必须成功");

    let selected = registry.select(MethodDomain::ThreeD);
    assert!(
        selected.is_empty(),
        "只含 software 条目的 registry：select(ThreeD) 必须为空（不 panic、不造条目）"
    );
}

#[test]
fn method_entry_has_exactly_three_public_fields() {
    // **`MethodEntry` 字段集的编译期照片**（纪律 5：纯数据声明只钉类型，
    // 机制是**不带 `..` 的结构体字面量**）。
    // **照实标注为编译期照片，本条没有运行期红**（纪律 1(c)）：
    // 加第四个字段 ⇒ 下面的字面量**编译失败**（缺字段 `E0063`）；
    // 删一个字段 ⇒ 同样编译失败（多出的字段名 `E0560`）。
    // **不得写成 `MethodEntry { id: …, ..Default::default() }` 一类带 `..` 的写法**
    // ——`..` 会让加字段的变异照过（纪律 1）。
    let entry = MethodEntry {
        id: MethodId::new("photo"),
        domain: MethodDomain::Video,
        realized_by: vec!["op-x".to_string()],
    };
    assert_eq!(entry.id, MethodId::new("photo"), "id 字段读回构造时那一枚");
    assert_eq!(
        entry.domain,
        MethodDomain::Video,
        "domain 字段读回构造时那一枚"
    );
    assert_eq!(
        entry.realized_by,
        vec!["op-x".to_string()],
        "realized_by 字段读回构造时那一枚"
    );
}
