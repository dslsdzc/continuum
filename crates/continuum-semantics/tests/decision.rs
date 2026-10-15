//! Task 15 的用例集：§331 的 Decision 对象与 §332 的恢复链
//! （设计 §10.1／§10.2、规范 §331／§332／§106／§107）。
//!
//! # 本文件钉的每一件，以及它的红条件
//!
//! | 用例 | 钉的判据 | 红条件（档位） |
//! |---|---|---|
//! | `options_and_consequences_are_equal_length_for_every_decision` | §106 的「每个选项带后果」在**读回侧**的落点：对每个读回的 `Decision`（枚举 0／1／3 三档，不抽代表）断言 `options.len() == consequences.len()`——**等长由类型保证，不由检查保证**（强制在 `DecisionOption::new` 的构造期） | 等长这件事本身没有运行期红法（**明写**：本用例的判别力在 trybuild 那条——加一条「只带选项」的构造路径会同时让那份反例红）；读回丢选项／丢后果／档位对不上即红 |
//! | `a_decision_is_created_with_blocking_true_at_the_two_trigger_points` | §10.1 的两个触发点各一条：(a) Constraint Validator 判违反 REQUIRED（经 Task 16 的 `validate` 的真实裁决）之后建的单；(b) Budget Validator 判预计超支且找不到合法低成本方案的场景——两条断言读回的 `blocking == true` | `blocking` 落库写错（**取反档**）；`create_decision` 把 blocking 丢掉（同断言） |
//! | `the_public_face_has_exactly_one_type_named_decision` | 本 crate 的公开面清单里 `Decision` **恰好一个**：`src/` 里以 `Decision` 为名的公开定义恰一处（`decision.rs`），且对 `continuum_policy` 的引用为零（与 `continuum_policy::Decision` 不混）。**限度写在用例文档里**（§16 第 31 条：`PolicyDecision` 别名是命名约定、不是类型守卫，故没有更强的照片） | 并列第二个 `Decision` 定义／引入对 policy 的引用即红 |
//! | `a_decision_required_event_is_written_on_creation` | §5.4：`Decision` 创建写出**恰一条** `EventType::DecisionRequired`（断言是那一枚、信封 `intent_id` 列与 payload 里的 `decision_id`）；第二次创建再来一条（不是「每次创建都写别的」） | 事件类型写成别的（**取反档**）；不写或写两条（计数断言红） |
//! | `a_decision_result_changes_only_the_fields_it_names` | §107／§10.2 的照片 (a)：一个指名了 **r-1 的 class 与 expression** 与 **plan 的 objectives** 的 `DecisionResult` 应用后：Contract 新版本与 Plan 新版本里**被指名的字段变了、其余字段逐项不变**（逐项断言，不抽代表）；旧版本行（`contract` v1、`plan` v1.0 除 `review_state` 外）未被改动；Intent 回到 `RUNNING`、`decision_result` 恰一行 | 把 `apply_result` 实现成「重新跑一遍 Intent Compiler」（**移除档**——未指名字段被重置即红）；原地改旧版本（旧行断言红）；不产新版本（新版本读回断言红） |
//! | `there_is_no_path_that_recompiles_a_decision_result_as_text` | §107 的照片 (b)，否定式：`apply_result` 的输入只有 `&DecisionResult`（**fn 指针标注**钉签名——不是 `&str`、不是 JSON 值）；`src/` 里含 `DecisionResult` 的出现点**逐条登记**，没有「从文本构造／重新编译」的路径（词法扫描的限度写在用例里） | 换输入为文本／JSON 即**编译不过**；新增出现点（如一个从 `&str` 构造 `DecisionResult` 的函数）即清单对不上、红 |
//! | `the_recovery_chain_is_atomic` | §10.2 的原子性，**失败路径 + 四张表**：四个失败点（决定步的审计写不进／`contract` 新版本撞主键／`plan` 新版本撞主键／Intent 回迁非法）各自断言 `Err`，且回滚后 `decision_result` / `contract` / `plan` / `intent` **四张表都没有半写的行**（不是只断言返回哪一种 `Err`） | 任一步拆出事务（**移除档**——对应段落的「已写的前几步」撤不掉即红）；静默吞错（`Err` 断言红）；先斩后奏（半写行断言红） |
//! | `a_user_approvals_audit_row_is_written_for_a_decision_result` | §5.4：`DecisionResult` 落库写**恰一条** `AuditKind::UserApprovals`（断言是那一枚、`occurred_at` 是 `decided_at`、payload 里是决策单与被选项）；**并写明**本层是第二个生产方、同一 kind 下不保证载荷同形（§16 第 33 条）；负侧：同一 Decision 的第二个结果撞 `decision_result` 主键（fail-closed） | kind 写错（**取反档**）；payload 不带决策单与被选项（同断言）；二次写静默成功（负侧红） |
//! | `a_result_pointing_at_a_missing_decision_or_option_is_rejected` | 「指向」的 fail-closed 两侧：`decision_id` 不存在 ⇒ [`DecisionError::UnknownDecision`]（点名 id）；`option_index` 越界 ⇒ [`DecisionError::UnknownOption`]（点名 id 与下标）——两处都不静默、不写任何行 | 不存在就静默通过／越界被钳到某一项（**放宽档**）；错误不点名（同断言） |
//! | `a_contract_update_pointing_at_missing_things_is_rejected` | `ContractUpdate` 的 fail-closed 两侧：基准版本不存在 ⇒ [`DecisionError::UnknownContract`]；指名改的 requirement id 不在基准 Contract 里 ⇒ [`DecisionError::UnknownRequirement`]（同 Task 16 悬空键的处置：判不了走 `Err`，不静默添加） | 基准不存在就静默新建／悬空 id 被凭空加进新版本（**放宽档**） |
//! | `a_plan_update_pointing_at_a_missing_plan_is_rejected` | `PlanUpdate` 基准版本不存在 ⇒ [`DecisionError::UnknownPlan`]（点名 id 与两段版本） | 静默新建（**放宽档**） |
//! | `an_out_of_table_blocking_value_is_rejected_on_read` | `decision.blocking` 的表外取值（非 0/1）读回**具体 `Err`**（[`DecisionError::InvalidBlocking`]，带原始数值、不取默认值）——**枚举 / 布尔列纪律**的读回侧 | 表外取值被静默映射成某一枚（**放宽档**）；读回 `Ok` 或不点名原始取值即红 |
//! | `a_second_creation_of_the_same_id_hits_the_primary_key` | `Decision` 的记录键 fail-closed：同 `id` 的第二次 `create_decision` 撞主键得 `Err`（不是 `INSERT OR REPLACE` 静默覆写） | 静默覆写（**移除档**）；原行被改动（逐列断言红） |
//!
//! # `blocking` 的处置（**申报**）
//!
//! §331 只给 `blocking` 一个布尔字段，**取值域未给**——`false` 的语义「不阻塞」在规范里
//! 没有用武之地（设计 §16 第 16 条）。本层**保留该字段并只在 §10.1 的两个触发点写
//! `true`**（`create_decision` 照收调用方给出的事实——本层**不从预算或任何判定算它**），
//! **不发明 `false` 的触发条件**。故本文件**没有 `false` 的照片**：没有一条「该场景下
//! 写 `false`」的规范场景可拍；两个触发点那一条拍的是「该场景语义下 `true` 的决策单
//! 被创建」这一可拍形状。
//!
//! # `DecisionResult` 的三个 update 类型的形状（**申报**）
//!
//! §10.2 只给了 `DecisionResult` 的语义（「结构化的、指向某个 `Decision` 与某个
//! `DecisionOption` 的值」，照片要求「只改被它指名的 Requirement / Plan 字段」），
//! **没给字段单**。本实现取：`decision_id` ＋ `option_index`（指向）＋
//! `contract_update` / `plan_update`（两个**逐字段 patch**——只给要改的字段，
//! `None` 即「不动」，故「其余字段逐项不变」由「从旧版本拷贝」的机制保证）＋
//! `decided_at`（决定时刻，`i64` 毫秒——**由调用方给**：本层不取时钟，同
//! `Intent.created_at` 的纪律）。完整理由与版本推进规则写在 `src/decision.rs`。
//!
//! # 夹具的来历
//!
//! `new_db()` 与 `tests/contract.rs` 的 `new_db()` 同形（P0 内建迁移 ＋ 本 crate
//! 迁移 110，建库后必须 `migrate()`）；失败点 1 的造法是「完整库先造好起点、再
//! `DROP TABLE audit_log`」（`audit_log` 表缺失 ⇒ 写审计必然失败；起点不能在一个
//! 「连内建迁移都没有」的库里造——`save_intent` / `create_decision` 要 `events` 表）。
//! `sample_*` 每个字段都取一个非默认值，好让「只改被指名的字段」那一类断言读回的是
//! 可区分的内容。

use std::fs;
use std::path::{Path, PathBuf};

use continuum_canonical::{EntityId, Opaque};
use continuum_persist::{
    AuditKind, Db, EventType, Migration, PersistError, Tx, Value, builtin_migrations,
};
use continuum_semantics::{
    CandidatePlan, ConstraintVerdict, ContractError, ContractId, ContractRef, ContractUpdate,
    Decision, DecisionError, DecisionId, DecisionOption, DecisionResult, Intent, IntentError,
    IntentId, IntentKind, IntentStatus, IntentStatusError, Plan, PlanError, PlanFootprint, PlanId,
    PlanType, PlanUpdate, PlanVersion, Priority, Requirement, RequirementClass, RequirementId,
    RequirementUpdate, ReviewState, TaskContract, VerificationRequirement, apply_result,
    create_decision, load_contract, load_decision, load_intent, load_plan, p4_semantics_migrations,
    save_contract, save_decision_result, save_intent, save_plan, validate,
};

/// 建一个只装了 P0 内建迁移 ＋ 本 crate 迁移的临时库（与 `tests/contract.rs` 的夹具同形）。
fn new_db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut migrations: Vec<Migration> = builtin_migrations();
    migrations.extend(p4_semantics_migrations());
    let db = Db::open_with(&path, migrations).unwrap();
    db.migrate().unwrap();
    (dir, db)
}

/// 一条基准 Intent：每个字段都取一个非默认值（同 `tests/intent_state.rs` 的 `sample`）。
fn sample_intent(id: &str, status: IntentStatus) -> Intent {
    Intent {
        id: IntentId::from_str(id),
        version: 0,
        kind: IntentKind::Task,
        goal: EntityId::from_str(&format!("goal-{id}")),
        status,
        priority: Priority::from_str("normal"),
        created_at: 100,
        updated_at: 100,
        parent_intent: None,
        contract_id: ContractRef {
            id: ContractId::from_str("c-1"),
            version: 1,
        },
        active_plan_id: None,
        graph_id: None,
        product_id: None,
        completion_predicate: Opaque::from_str("done"),
    }
}

/// 一条基准 TaskContract：两条 requirement（`r-1` 是 REQUIRED——§106 例子里「4K 要求」
/// 的对应物；`r-2` 是 PREFERRED），六个 Opaque 字段各取一个不同的值。
fn sample_contract(id: &str, version: u32) -> TaskContract {
    fn requirement(id: &str, class: RequirementClass) -> Requirement {
        Requirement {
            id: RequirementId::from_str(id),
            class,
            expression: Opaque::from_str(&format!("expr-{id}")),
            source: Opaque::from_str(&format!("source-{id}")),
            verification_requirement: VerificationRequirement::from_str(&format!("vr-{id}")),
        }
    }
    TaskContract {
        id: ContractId::from_str(id),
        version,
        intent_id: IntentId::from_str("i-1"),
        requirements: vec![
            requirement("r-1", RequirementClass::Required),
            requirement("r-2", RequirementClass::Preferred),
        ],
        budget: Opaque::from_str("budget-a"),
        authority_limit: Opaque::from_str("authority-a"),
        data_policy: Opaque::from_str("data-policy-a"),
        model_policy: Opaque::from_str("model-policy-a"),
        node_policy: Opaque::from_str("node-policy-a"),
        acceptance_predicate: Opaque::from_str("accept-a"),
    }
}

/// `(major, minor)` 的两个数——用例里到处要写 `PlanVersion`，包一层省噪。
fn version(major: u32, minor: u32) -> PlanVersion {
    PlanVersion { major, minor }
}

/// 一条基准 Plan：五个列表各取非默认值（`objectives` 取两项，好让「整列替换」与
/// 「逐项拼接」在断言下可区分），`review_state` 取 `USER_APPROVED`。
fn sample_plan(id: &str, version: PlanVersion) -> Plan {
    Plan {
        id: PlanId::from_str(id),
        version,
        intent_id: IntentId::from_str("i-1"),
        r#type: PlanType::Execution,
        objectives: vec![Opaque::from_str("obj-1"), Opaque::from_str("obj-2")],
        milestones: vec![Opaque::from_str("ms-1")],
        assumptions: vec![Opaque::from_str("asm-1")],
        risks: vec![Opaque::from_str("risk-1")],
        acceptance_conditions: vec![Opaque::from_str("ac-1")],
        review_state: ReviewState::UserApproved,
    }
}

/// 一条基准 Decision：§106 的例子（4K 无法满足）三个选项各带后果。
fn sample_decision(id: &str, intent: &str) -> Decision {
    Decision {
        id: DecisionId::from_str(id),
        intent_id: IntentId::from_str(intent),
        issue: Opaque::from_str("当前无法满足 4K"),
        options: vec![
            DecisionOption::new(
                Opaque::from_str("使用云 GPU"),
                Opaque::from_str("¥8–12，保持 4K"),
            ),
            DecisionOption::new(
                Opaque::from_str("改成 1080p"),
                Opaque::from_str("免费，违反当前 4K 要求"),
            ),
            DecisionOption::new(
                Opaque::from_str("等待 Desktop 节点上线"),
                Opaque::from_str("不确定的延迟"),
            ),
        ],
        blocking: true,
    }
}

/// 一条基准 DecisionResult：选 B（下标 1），指名改 `c-1` v1 的 `r-1` 两个字段与
/// `p-1` v1.0 的 `objectives`。
fn sample_result(decision: &str) -> DecisionResult {
    DecisionResult {
        decision_id: DecisionId::from_str(decision),
        option_index: 1,
        contract_update: Some(ContractUpdate {
            id: ContractId::from_str("c-1"),
            base_version: 1,
            requirements: vec![RequirementUpdate {
                id: RequirementId::from_str("r-1"),
                class: Some(RequirementClass::Preferred),
                expression: Some(Opaque::from_str("1080p，放弃 4K")),
                source: None,
                verification_requirement: None,
            }],
        }),
        plan_update: Some(PlanUpdate {
            id: PlanId::from_str("p-1"),
            base_version: version(1, 0),
            objectives: Some(vec![Opaque::from_str("交付 1080p 版本的输出")]),
            milestones: None,
            assumptions: None,
            risks: None,
            acceptance_conditions: None,
        }),
        decided_at: 10_000,
    }
}

/// 五个列表逐项读成字符串（`Opaque` 无 `PartialEq`，逐项 `as_str` 才是判据）。
fn strs(items: &[Opaque]) -> Vec<String> {
    items.iter().map(|o| o.as_str().to_owned()).collect()
}

/// 表的行数（表名由本文件提供，不是用户输入）。
fn row_count(tx: &Tx<'_>, table: &str) -> i64 {
    match &tx.query(&format!("SELECT COUNT(*) FROM {table}"), &[]).unwrap()[0][0] {
        Value::Int(i) => *i,
        other => panic!("COUNT(*) 应是整数，实得 {other:?}"),
    }
}

/// `contract` 表一行（raw `SELECT *`，列序 = 建表序）——「未动／半写」的逐列判据。
fn raw_contract_row(tx: &Tx<'_>, id: &str, version: u32) -> Vec<Value> {
    let rows = tx
        .query(
            "SELECT * FROM contract WHERE id = ?1 AND version = ?2",
            &[Value::text(id), Value::Int(i64::from(version))],
        )
        .unwrap();
    assert_eq!(rows.len(), 1, "contract 表里应有恰一行 ({id:?}, v{version})");
    rows.into_iter().next().unwrap()
}

/// `plan` 表一行（raw `SELECT *`）。
fn raw_plan_row(tx: &Tx<'_>, id: &str, v: PlanVersion) -> Vec<Value> {
    let rows = tx
        .query(
            "SELECT * FROM plan WHERE id = ?1 AND major = ?2 AND minor = ?3",
            &[
                Value::text(id),
                Value::Int(i64::from(v.major)),
                Value::Int(i64::from(v.minor)),
            ],
        )
        .unwrap();
    assert_eq!(rows.len(), 1, "plan 表里应有恰一行 ({id:?}, v{v:?})");
    rows.into_iter().next().unwrap()
}

/// `decision_result` 表的行数（`WHERE decision_id = ?1`）。
fn decision_result_count(tx: &Tx<'_>, decision: &str) -> i64 {
    match &tx
        .query(
            "SELECT COUNT(*) FROM decision_result WHERE decision_id = ?1",
            &[Value::text(decision)],
        )
        .unwrap()[0][0]
    {
        Value::Int(i) => *i,
        other => panic!("COUNT(*) 应是整数，实得 {other:?}"),
    }
}

/// 跑一遍完整的「起点」：Intent（`SUSPENDED`）、Decision、Contract v1、Plan v1.0，
/// 全部提交——这样后续 `apply_result` 的回滚只撤它自己的半写，不撤起点。
fn commit_start_state(tx: &Tx<'_>) {
    save_intent(tx, &sample_intent("i-1", IntentStatus::Suspended)).unwrap();
    create_decision(tx, &sample_decision("d-1", "i-1"), 5_000).unwrap();
    save_contract(tx, &sample_contract("c-1", 1), 5_000).unwrap();
    save_plan(tx, &sample_plan("p-1", version(1, 0))).unwrap();
}

// ===== §331：构造期的等长（§106 的「每个选项带后果」） =====

/// §106 的「每个选项带后果」落在**构造期**：对每个读回的 `Decision` 断言
/// `options.len() == consequences.len()`——**枚举式**（0／1／3 三档，不抽代表）。
///
/// # 等长是**类型保证**，不是**检查保证**
///
/// `Decision.options` 是 `Vec<DecisionOption>`，每一项由 `DecisionOption::new(option,
/// consequence)` 构造（两个参数都是必填的 `Opaque`）——不存在「只带选项」的记录形态，
/// 故本用例**没有自己的红法**：它的判别力在 `tests/type_level.rs` 那条
/// `a_decision_option_cannot_be_built_without_a_consequence`（反例：`new` 少一个参数、
/// 结构体字面量少 `consequence` 字段，两份都编不过）＋ 它的正控制。
///
/// `load_decision` 对不存在的 id 返回 `Ok(None)`（那不是错误，同 `load_intent`）。
#[test]
fn options_and_consequences_are_equal_length_for_every_decision() {
    let (_d, db) = new_db();
    let tx = db.begin().unwrap();

    // 三档：0、1、3 项。**空 `options` 不被任何已知判据禁止**（§331 未给「至少一项」），
    // 本用例不为它发明判据——0 档也照读回断言。
    let sizes = [0usize, 1, 3];
    for (n, size) in sizes.iter().enumerate() {
        let mut options = Vec::new();
        for k in 0..*size {
            options.push(DecisionOption::new(
                Opaque::from_str(&format!("选项-{n}-{k}")),
                Opaque::from_str(&format!("后果-{n}-{k}")),
            ));
        }
        let d = Decision {
            id: DecisionId::from_str(&format!("d-{n}")),
            intent_id: IntentId::from_str(&format!("i-{n}")),
            issue: Opaque::from_str(&format!("议题-{n}")),
            options,
            blocking: true,
        };
        create_decision(&tx, &d, 1_000).unwrap();
    }

    // 「对每个读回的 Decision」——逐档断言，不抽代表。
    for (n, size) in sizes.iter().enumerate() {
        let d = load_decision(&tx, &DecisionId::from_str(&format!("d-{n}")))
            .unwrap()
            .unwrap();
        assert_eq!(d.options.len(), *size, "读回的 d-{n} 应有 {size} 项选项");
        assert_eq!(
            d.options.len(),
            d.consequences().len(),
            "读回的 d-{n}：options 与 consequences 必须等长（§106——等长由构造期保证，\
             本断言只是它读回侧的照片）"
        );
        // 逐项再钉一层：第 k 项的后果是写进去的那一个（不是被合并／挪位的）。
        for k in 0..*size {
            assert_eq!(d.options[k].option.as_str(), format!("选项-{n}-{k}"));
            assert_eq!(d.consequences()[k].as_str(), format!("后果-{n}-{k}"));
        }
    }

    assert!(
        load_decision(&tx, &DecisionId::from_str("d-404"))
            .unwrap()
            .is_none(),
        "不存在的 id 返回 Ok(None)（那不是错误，同 load_intent）"
    );

    tx.commit().unwrap();
}

// ===== §10.1：两个触发点与 blocking =====

/// §10.1 的两个触发点，**两条各一条**（不抽代表）：
///
/// - **(a) Constraint Validator 判违反 REQUIRED**（§6 的「违反 REQUIRED → 挂起 →
///   请求用户决定」）：本段经 Task 16 的 `validate` 得真实的
///   [`ConstraintVerdict::ViolatesRequired`]，再在这件事实下建单——「违反 REQUIRED 的
///   候选计划被拦截并生成 Decision」是同一 crate 的两步（计划的总表：Task 16 的拦截
///   ＋ Task 15 的触发点）。
/// - **(b) Budget Validator 判预计超支且找不到合法低成本方案**（§110）：**预算侧判定
///   今天不存在**（Task 17／18 建 `BudgetVerdict`；「先寻找合法低成本方案」与「什么
///   算找不到」的判定归 Planner，设计 §16 第 20 条）。故本段拍的是「**该场景语义下**
///   `blocking == true` 的 Decision 被创建」这一可拍形状——`blocking` 是**调用方给出
///   的事实**，本层不从任何预算读数算它（§10.1 的「两条都产出一个 Decision，
///   `blocking = true`」是触发点的行为，不是本层的计算）。
///
/// # 为什么没有 `false` 的照片
///
/// §331 未给 `blocking` 的取值域；`false` 的语义「不阻塞」在规范里没有用武之地
/// （设计 §16 第 16 条）。本层保留该字段并只在两个触发点写 `true`，**不发明 `false`
/// 的触发条件**——故没有「该场景写 `false`」的用例可拍。
///
/// **红条件**：`blocking` 落库写错方向／被丢掉（**取反档**，两段各一条断言）。
#[test]
fn a_decision_is_created_with_blocking_true_at_the_two_trigger_points() {
    let (_d, db) = new_db();
    let tx = db.begin().unwrap();

    // (a) 违反 REQUIRED 的那条链：validate 的裁决是「键命中一条 REQUIRED」。
    let candidate = sample_plan("candidate-1", version(1, 0));
    let footprint = PlanFootprint {
        authority: Opaque::from_str("authority-1"),
        privacy_class: Opaque::from_str("privacy-class-1"),
        external_effects: vec![Opaque::from_str("effect-1")],
        estimated_cost: Opaque::from_str("cost-1"),
    };
    let verdict = validate(
        &CandidatePlan {
            plan: &candidate,
            footprint: &footprint,
        },
        &sample_contract("c-1", 1),
        &["r-1".to_owned()],
        &serde_json::json!([]),
    )
    .unwrap();
    let requirement = match verdict {
        ConstraintVerdict::ViolatesRequired { requirement } => requirement,
        other => panic!("(a) 前置：`r-1` 命中 REQUIRED 应得 ViolatesRequired，实得 {other:?}"),
    };
    assert_eq!(
        requirement.as_str(),
        "r-1",
        "(a) 前置：被违反的应是那一个 requirement"
    );

    let d_a = Decision {
        id: DecisionId::from_str("d-a"),
        intent_id: IntentId::from_str("i-1"),
        issue: Opaque::from_str("候选计划违反 REQUIRED 要求 r-1（挂起，等用户决定）"),
        options: vec![
            DecisionOption::new(
                Opaque::from_str("放宽 r-1"),
                Opaque::from_str("要求被改弱"),
            ),
            DecisionOption::new(
                Opaque::from_str("保持 r-1、放弃候选计划"),
                Opaque::from_str("任务挂起"),
            ),
        ],
        blocking: true,
    };
    create_decision(&tx, &d_a, 1_000).unwrap();
    let back = load_decision(&tx, &d_a.id).unwrap().unwrap();
    assert!(
        back.blocking,
        "(a) 违反 REQUIRED 的触发点应创建 blocking == true 的 Decision"
    );

    // (b) 预计超支且找不到合法低成本方案的场景（预算侧判定的落点不在本层，见用例文档）。
    let d_b = Decision {
        id: DecisionId::from_str("d-b"),
        intent_id: IntentId::from_str("i-1"),
        issue: Opaque::from_str("预计超支，且找不到合法低成本方案"),
        options: vec![
            DecisionOption::new(
                Opaque::from_str("批准追加预算"),
                Opaque::from_str("超支"),
            ),
            DecisionOption::new(
                Opaque::from_str("降低目标"),
                Opaque::from_str("交付变小"),
            ),
        ],
        blocking: true,
    };
    create_decision(&tx, &d_b, 2_000).unwrap();
    let back = load_decision(&tx, &d_b.id).unwrap().unwrap();
    assert!(
        back.blocking,
        "(b) 预计超支场景的触发点应创建 blocking == true 的 Decision"
    );

    tx.commit().unwrap();
}

// ===== 命名冲突（§16 第 31 条） =====

/// 本 crate 的公开面清单里 `Decision` **恰好一个**（不是两个，与
/// `continuum_policy::Decision` 不混）。
///
/// # 拍的是什么
///
/// 1. **定义计数**：`src/` 里以 `Decision` 为名的公开定义（`pub struct` / `pub enum`
///    / `pub type` / `pub trait`，名字**恰为** `Decision`）恰一处，在 `decision.rs`；
/// 2. **与 `continuum_policy` 不混**：本 crate 的 `src/` 对 `continuum_policy` /
///    `PolicyDecision` 零引用，`Cargo.toml` 里也没有到 `continuum-policy` 的依赖边
///    ——设计 §10.1 的处置（「同时引用两者的模块把策略的那个别名成 `PolicyDecision`」）
///    在本 crate 里根本没有需要别名的模块。
///
/// # 这条守卫的限度（照实写，§16 第 31 条）
///
/// `PolicyDecision` 别名是**命名约定、不是类型守卫**——没有任何编译期机制阻止后来者
/// 并列 `use continuum_policy::Decision;` 与 `use continuum_semantics::Decision;`
/// （那要等本 crate 引入 policy 的依赖边，而 §11.5 的第 1 条守卫正是「不行」）。
/// **故这条没有更强的照片**——更强的做法（重命名本层的类型）会偏离 §331 的名字。
///
/// **红条件**：并列第二个 `Decision` 定义／引入对 policy 的引用即红。
#[test]
fn the_public_face_has_exactly_one_type_named_decision() {
    let sources = crate_sources();
    assert!(
        sources.len() >= 5,
        "只扫到 {} 个 `src/` 下的 `.rs`：源码树没被读到",
        sources.len()
    );

    // (1) 定义计数：名字**恰为** `Decision` 的公开定义（`DecisionOption` / `DecisionId`
    // / `DecisionResult` 等以 `Decision` 开头的更长名字不算——下一 token 不是分隔符）。
    let mut definitions: Vec<String> = Vec::new();
    for (path, body) in &sources {
        for line in strip_comments(body).lines() {
            let tokens: Vec<&str> = line.split_whitespace().collect();
            let is_definition = tokens.len() >= 3
                && tokens[0] == "pub"
                && matches!(tokens[1], "struct" | "enum" | "type" | "trait")
                && tokens[2] == "Decision";
            if is_definition {
                definitions.push(format!("{}: {}", path.display(), tokens.join(" ")));
            }
        }
    }
    assert_eq!(
        definitions,
        vec!["src/decision.rs: pub struct Decision {".to_owned()],
        "`src/` 里以 `Decision` 为名的公开定义应恰一处（`decision.rs`）——\
         与 `continuum_policy::Decision` 不混靠的是「本 crate 根本不引用 policy」"
    );

    // (2) 对 policy 零引用（源码面与依赖面各一层）。
    for (path, body) in &sources {
        let code = strip_comments(body);
        for (needle, what) in [
            ("continuum_policy", "策略 crate 的标识符"),
            ("PolicyDecision", "策略那个 Decision 的别名"),
        ] {
            assert!(
                !code.contains(needle),
                "{} 里出现了 `{needle}`（{what}）——本 crate 不引用它，\
                 本层的 `Decision` 是 §331 的对象",
                path.display()
            );
        }
    }
    let manifest = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .expect("Cargo.toml 应可读");
    assert!(
        !manifest.contains("continuum-policy"),
        "`continuum-semantics` 的 Cargo.toml 不得登记指向 `continuum-policy` 的依赖边"
    );
}

// ===== §5.4：DecisionRequired 事件 =====

/// `Decision` 创建写出**恰一条** `EventType::DecisionRequired`（§5.4 的写入点之一）。
///
/// 断言四件：事件类型**是那一枚**（经 raw 列取文本与 `as_str` 比对）、信封的
/// `intent_id` 列是决策单挂的 Intent、payload 里的 `decision_id`、以及第二次创建
/// （另一张单）再来一条——不是「每次创建都写别的」。
///
/// **红条件**：事件类型写成别的（**取反档**）；不写或写两条（计数断言红）。
#[test]
fn a_decision_required_event_is_written_on_creation() {
    let (_d, db) = new_db();
    let tx = db.begin().unwrap();

    create_decision(&tx, &sample_decision("d-1", "i-1"), 7_000).unwrap();
    create_decision(&tx, &sample_decision("d-2", "i-1"), 8_000).unwrap();

    let rows = tx
        .query(
            "SELECT event_type, occurred_at, intent_id, payload FROM events ORDER BY rowid",
            &[],
        )
        .unwrap();
    assert_eq!(rows.len(), 2, "两张决策单应各写恰一条事件");
    for (i, (decision, occurred_at)) in [("d-1", 7_000i64), ("d-2", 8_000)].iter().enumerate() {
        let event_type = match &rows[i][0] {
            Value::Text(s) => s.clone(),
            other => panic!("event_type 列应是文本，实得 {other:?}"),
        };
        assert_eq!(
            event_type,
            EventType::DecisionRequired.as_str(),
            "第 {i} 条事件的类型应是 `DecisionRequired` 那一枚"
        );
        assert_eq!(
            rows[i][1],
            Value::Int(*occurred_at),
            "第 {i} 条事件的 occurred_at 应是调用方给的那一个"
        );
        assert_eq!(
            rows[i][2],
            Value::text("i-1"),
            "第 {i} 条事件的信封 intent_id 应是决策单挂的 Intent"
        );
        let payload_str = match &rows[i][3] {
            Value::Text(s) => s.clone(),
            other => panic!("payload 列应是文本，实得 {other:?}"),
        };
        let payload: serde_json::Value = serde_json::from_str(&payload_str).unwrap();
        assert_eq!(
            payload.get("decision_id").and_then(|v| v.as_str()),
            Some(*decision),
            "第 {i} 条事件的 payload 应指向那张决策单"
        );
    }

    tx.commit().unwrap();
}

// ===== §107／§10.2：只改被指名的字段 =====

/// §107 的照片 (a)：一个**只指名** `r-1` 的 `class` 与 `expression`、以及 `p-1` 的
/// `objectives` 的 `DecisionResult` 应用后，**其余字段逐项不变**（逐项断言，不抽代表）。
///
/// # 逐项清单（每一行都是一条断言）
///
/// - **Contract 侧**：新版本 v2 存在（`r-1` 的两个字段变了）；`r-1` 的
///   `source` / `verification_requirement` / `id` 与 `r-2` 的五项**逐项不变**；
///   v2 的 `intent_id` 与六个 Opaque 字段**逐项不变**；**v1 行 raw 逐列不变**
///   （§226 的 MUST NOT 原地修改——多版本并存）。
/// - **Plan 侧**：新版本 v2.0（`major` +1——§9.2 的修订语义：新版本 `DRAFT`）存在；
///   `objectives` 是**指名的那个新值**（整列替换，不是拼接）；`milestones` /
///   `assumptions` / `risks` / `acceptance_conditions` / `type` / `intent_id` 逐项不变；
///   旧版本 v1.0 仍在、`review_state = SUPERSEDED`（**raw 差异集恰为 `{review_state}` 一列**）、
///   其余内容逐项不变（旧行只动 `review_state` 一列——那是 §9.2「修订」语义的另一半，
///   同 `revise` 的值层先例）。
/// - **其余两张**：`decision_result` 恰一行（决策单与被选项）；Intent 回到 `RUNNING`。
///
/// # 红条件
///
/// 把 `apply_result` 实现成「重新跑一遍 Intent Compiler」（**移除档**——未指名的字段
/// 被重新生成／重置即红）；原地改旧版本（旧行断言红）；不产生新版本（新版本读回断言红）。
#[test]
fn a_decision_result_changes_only_the_fields_it_names() {
    let (_d, db) = new_db();

    // 起点：全部提交（回滚边界清楚——起点不该被后续任何一步撤掉）。
    let tx = db.begin().unwrap();
    commit_start_state(&tx);
    tx.commit().unwrap();

    // apply 之前把两张旧行的 raw 副本存下来（「旧版本逐列不变」的判据）。
    let tx = db.begin().unwrap();
    let contract_v1_before = raw_contract_row(&tx, "c-1", 1);
    let plan_v1_before = raw_plan_row(&tx, "p-1", version(1, 0));
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    apply_result(&tx, &sample_result("d-1")).unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();

    // ---- Contract 侧 ----
    let v1 = load_contract(&tx, &ContractId::from_str("c-1"), 1)
        .unwrap()
        .unwrap();
    assert_eq!(
        raw_contract_row(&tx, "c-1", 1),
        contract_v1_before,
        "v1 行应 raw 逐列不变（§226：不原地修改，新版本是另起一行）"
    );
    let v2 = load_contract(&tx, &ContractId::from_str("c-1"), 2)
        .unwrap()
        .unwrap();
    assert_eq!(v2.version, 2, "应产生 v2（base_version + 1）");
    let r1 = v2
        .requirements
        .iter()
        .find(|r| r.id.as_str() == "r-1")
        .expect("v2 里应有 r-1");
    assert_eq!(
        r1.class,
        RequirementClass::Preferred,
        "r-1.class 被指名改过（REQUIRED → PREFERRED）"
    );
    assert_eq!(
        r1.expression.as_str(),
        "1080p，放弃 4K",
        "r-1.expression 被指名改过"
    );
    // r-1 的其余字段逐项不变。
    assert_eq!(r1.id.as_str(), "r-1", "r-1.id 不变（定位键）");
    assert_eq!(r1.source.as_str(), "source-r-1", "r-1.source 未被指名，应不变");
    assert_eq!(
        r1.verification_requirement.as_str(),
        "vr-r-1",
        "r-1.verification_requirement 未被指名，应不变"
    );
    // r-2 五项逐项不变。
    let r2 = v2
        .requirements
        .iter()
        .find(|r| r.id.as_str() == "r-2")
        .expect("v2 里应有 r-2");
    assert_eq!(r2.id.as_str(), "r-2");
    assert_eq!(r2.class, RequirementClass::Preferred);
    assert_eq!(r2.expression.as_str(), "expr-r-2");
    assert_eq!(r2.source.as_str(), "source-r-2");
    assert_eq!(r2.verification_requirement.as_str(), "vr-r-2");
    // v2 的归属与六个 Opaque 字段逐项不变。
    assert_eq!(v2.intent_id.as_str(), v1.intent_id.as_str());
    assert_eq!(v2.budget.as_str(), v1.budget.as_str());
    assert_eq!(v2.authority_limit.as_str(), v1.authority_limit.as_str());
    assert_eq!(v2.data_policy.as_str(), v1.data_policy.as_str());
    assert_eq!(v2.model_policy.as_str(), v1.model_policy.as_str());
    assert_eq!(v2.node_policy.as_str(), v1.node_policy.as_str());
    assert_eq!(
        v2.acceptance_predicate.as_str(),
        v1.acceptance_predicate.as_str()
    );

    // ---- Plan 侧 ----
    let old = load_plan(&tx, &PlanId::from_str("p-1"), version(1, 0))
        .unwrap()
        .unwrap();
    assert_eq!(
        old.review_state,
        ReviewState::Superseded,
        "旧版本应转 SUPERSEDED（§9.2 修订语义的另一半，同 revise 的值层先例）"
    );
    assert_eq!(old.objectives.len(), 2, "旧版本内容不因新版本而改");
    assert_eq!(strs(&old.objectives), vec!["obj-1", "obj-2"]);
    assert_eq!(strs(&old.milestones), vec!["ms-1"]);
    assert_eq!(strs(&old.assumptions), vec!["asm-1"]);
    assert_eq!(strs(&old.risks), vec!["risk-1"]);
    assert_eq!(strs(&old.acceptance_conditions), vec!["ac-1"]);
    let plan_v1_after = raw_plan_row(&tx, "p-1", version(1, 0));
    assert_eq!(
        plan_v1_after.len(),
        plan_v1_before.len(),
        "旧 plan 行的列数不变（先钉列数，下面的逐列比对才不含没比到的列）"
    );
    // 差异集**恰为** `{review_state}`（建表序第 10 列，见 `persist.rs` 的 `CREATE TABLE plan`）
    // ——与上面 Contract 侧的整行 raw 比对同级强度；只比列数会让其余列被改都漂过去。
    let differences: Vec<usize> = plan_v1_before
        .iter()
        .zip(&plan_v1_after)
        .enumerate()
        .filter(|(_, (before, after))| before != after)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(
        differences,
        vec![10],
        "旧 plan 行的差异集应恰为 {{review_state}}（第 10 列）——其余列 raw 逐列不变"
    );

    let new = load_plan(&tx, &PlanId::from_str("p-1"), version(2, 0))
        .unwrap()
        .unwrap();
    assert_eq!(
        new.version,
        version(2, 0),
        "新版本应是 major +1、minor 归零（§9.2 的修订按 MATERIAL 档推进）"
    );
    assert_eq!(
        new.review_state,
        ReviewState::Draft,
        "修订后的新版本回到 DRAFT（重开审查门）"
    );
    assert_eq!(
        strs(&new.objectives),
        vec!["交付 1080p 版本的输出"],
        "objectives 被指名改过（整列替换，不是与旧值拼接）"
    );
    // 其余四个列表与 type / intent_id 逐项不变。
    assert_eq!(strs(&new.milestones), vec!["ms-1"]);
    assert_eq!(strs(&new.assumptions), vec!["asm-1"]);
    assert_eq!(strs(&new.risks), vec!["risk-1"]);
    assert_eq!(strs(&new.acceptance_conditions), vec!["ac-1"]);
    assert_eq!(new.r#type, PlanType::Execution, "type 未被指名，应不变");
    assert_eq!(new.intent_id.as_str(), "i-1", "intent_id 未被指名，应不变");

    // ---- 其余两张 ----
    assert_eq!(
        decision_result_count(&tx, "d-1"),
        1,
        "DecisionResult 应恰落一行（决策单与被选项）"
    );
    let status = load_intent(&tx, &IntentId::from_str("i-1"))
        .unwrap()
        .unwrap()
        .status;
    assert_eq!(
        status,
        IntentStatus::Running,
        "Intent 应从 SUSPENDED 回迁到 RUNNING（§332 的 Resume）"
    );

    tx.commit().unwrap();
}

// ===== §107 的否定式照片 (b) =====

/// §107 的照片 (b)，**否定式**：不存在「把 `DecisionResult` 序列化成文本再重新编译」
/// 的路径。
///
/// # 拍的两件（与它们的限度）
///
/// 1. **输入面**：`apply_result` 的输入只有 `&DecisionResult`（下面那行 `fn` 指针标注
///    钉死签名）——不是 `&str`、不是 `serde_json::Value`、不是「文本再解释」的别的形态。
/// 2. **出现点逐条登记**：扫本 crate 的 `src/`（剥注释的代码行），含 `DecisionResult`
///    的每一行与写死的清单逐条比对——新增一个「从文本构造 `DecisionResult`」的函数
///    （如 `fn apply_result_from_text(t: &str)` 内部拼 `DecisionResult`）会让出现点
///    多出行来 ⇒ 红。
///
/// **限度照实写**：`impl DecisionResult` 块里的方法定义行不含类型名，扫不到——函数名
/// 是**开放集**，`trybuild` 也枚举不完（同 `VerificationRequirement` 那条的处置）。
/// 对冲的是本类型的**公开面只有字段**：没有任何方法（构造、序列化、编译都在别处写不出来
/// ——`DecisionResult` 的字段是 `pub` 的记录体，本层没有第二个构造入口）。
///
/// **红条件**：换输入为文本／JSON 即**编译不过**；新增出现点即清单对不上、红。
#[test]
fn there_is_no_path_that_recompiles_a_decision_result_as_text() {
    // (1) 签名：输入就是 `&DecisionResult`。
    let _signature: fn(&Tx<'_>, &DecisionResult) -> Result<(), DecisionError> = apply_result;

    // (2) 出现点清单。
    let sources = crate_sources();
    let mut occurrences: Vec<String> = Vec::new();
    for (path, body) in &sources {
        let code = strip_comments(body);
        for line in code.lines() {
            if line.contains("DecisionResult") {
                let normalized = line.split_whitespace().collect::<Vec<_>>().join(" ");
                let normalized = normalized.trim();
                if !normalized.is_empty() {
                    occurrences.push(format!("{}: {normalized}", path.display()));
                }
            }
        }
    }
    occurrences.sort();
    occurrences.dedup();

    // 允许清单（按字符串序；每条都写明用途）——新增一行意味着多了一个**需要人工
    // 复核**的出现点：确认它不是「从文本构造／重新编译 DecisionResult」。
    // 注释行经 `strip_comments` 剥掉（清单只登记代码行）。
    let expected = [
        "src/decision.rs: pub struct DecisionResult {",
        "src/lib.rs: ContractUpdate, Decision, DecisionOption, DecisionResult, PlanUpdate, RequirementUpdate,",
        "src/persist.rs: pub fn apply_result(tx: &Tx<'_>, r: &DecisionResult) -> Result<(), DecisionError> {",
        "src/persist.rs: pub fn save_decision_result(tx: &Tx<'_>, r: &DecisionResult) -> Result<(), DecisionError> {",
        "src/persist.rs: use crate::decision::{ContractUpdate, Decision, DecisionOption, DecisionResult, PlanUpdate};",
    ];
    let expected: Vec<String> = expected.iter().map(|s| (*s).to_owned()).collect();
    assert_eq!(
        occurrences, expected,
        "`src/` 里含 `DecisionResult` 的代码出现点应逐条等于登记清单：新增出现点\
         （如一个从 `&str` 构造 DecisionResult 的函数）要先人工复核用途，再把它加进清单；\
         移除/改写了既有出现点则要更新对应行"
    );
}

// ===== §10.2：恢复链的原子性（失败路径 + 四张表） =====

/// §10.2 的原子性：`DecisionResult` 落库 ＋ Contract/Plan 新版本 ＋ Intent 状态回迁
/// **在同一事务**。四个失败点各起一个库，各自断言：具体 `Err`，且**回滚后
/// `decision_result` / `contract` / `plan` / `intent` 四张表都没有半写的行**
/// （不是只断言返回哪一种 `Err`）。
///
/// # 四个失败点（各自的「半写」面不同，合起来才算钉住「任一步」）
///
/// 1. **决定步的审计写不进**（`audit_log` 表被移走的库）：`decision_result` 行已写、
///    审计失败 ⇒ `Err(Persist 点名 audit_log)`；回滚后四张表都没有半写。
///    **造法**：完整库先造好起点（`save_intent` / `create_decision` 要 `events`
///    表、`save_contract` 要写审计，故起点不能在一个「连内建迁移都没有」的库里
///    造），再 `DROP TABLE audit_log`——「迁移集里去掉那张表」的等价物，仍是真实的
///    数据库错误。
/// 2. **`contract` 新版本撞主键**（v2 已存在）：决定步已写、contract 步失败 ⇒
///    `Err(Contract(Persist))`；回滚后 `decision_result` 无行、预先存在的 v2 行 raw
///    逐列未变、`plan`／`intent` 未动。
/// 3. **`plan` 新版本撞主键**（v2.0 已存在）：决定步与 contract 步都已写、plan 步失败
///    ⇒ `Err(Plan(Persist))`；回滚后前三步全撤（`decision_result` 无行、无 contract v2）、
///    预先存在的 plan 行与旧版本行都未变。
/// 4. **Intent 回迁非法**（起点 Intent 是 `PROPOSED`，`PROPOSED → RUNNING` 不在
///    §5.3 的迁移表里）：前三步都已写、第四步被状态机拒绝 ⇒ `Err(Intent(Transition))`；
///    回滚后四张表全复原。
///
/// **前提**：写函数都不自己 `commit`——「调用方在 `Err` 后回滚」就是各段丢掉 `Tx`。
#[test]
fn the_recovery_chain_is_atomic() {
    // ---- 失败点 1：决定步的审计写不进（audit_log 表被移走）----
    {
        let (_d, db) = new_db();
        let tx = db.begin().unwrap();
        commit_start_state(&tx);
        tx.execute("DROP TABLE audit_log", &[]).unwrap();
        tx.commit().unwrap();

        let tx = db.begin().unwrap();
        let err = apply_result(&tx, &sample_result("d-1")).unwrap_err();
        assert!(
            matches!(&err, DecisionError::Persist(PersistError::Database(m)) if m.contains("audit_log")),
            "失败点 1 期望「audit_log 表缺失」的 Persist，实得 {err:?}"
        );
        drop(tx);

        let tx = db.begin().unwrap();
        assert_eq!(
            decision_result_count(&tx, "d-1"),
            0,
            "失败点 1 回滚后 decision_result 应无半写行（行在审计之前写入）"
        );
        assert_eq!(
            tx.query("SELECT COUNT(*) FROM contract WHERE version = 2", &[]).unwrap()[0][0],
            Value::Int(0),
            "失败点 1 回滚后 contract 不应有 v2"
        );
        assert_eq!(
            tx.query("SELECT COUNT(*) FROM plan WHERE id = 'p-1' AND major = 2", &[])
                .unwrap()[0][0],
            Value::Int(0),
            "失败点 1 回滚后 plan 不应有新版本"
        );
        assert_eq!(
            raw_plan_row(&tx, "p-1", version(1, 0))[10],
            Value::text("user_approved"),
            "失败点 1 回滚后旧 plan 行的 review_state 应原样"
        );
        assert_eq!(
            load_intent(&tx, &IntentId::from_str("i-1")).unwrap().unwrap().status,
            IntentStatus::Suspended,
            "失败点 1 回滚后 Intent 应仍是 SUSPENDED"
        );
        tx.commit().unwrap();
    }

    // ---- 失败点 2：contract 新版本撞主键（v2 预先存在）----
    {
        let (_d, db) = new_db();
        let tx = db.begin().unwrap();
        commit_start_state(&tx);
        save_contract(&tx, &sample_contract("c-1", 2), 6_000).unwrap();
        tx.commit().unwrap();
        let tx = db.begin().unwrap();
        let c2_before = raw_contract_row(&tx, "c-1", 2);
        let audit_before = tx.audit_records().unwrap().len();
        tx.commit().unwrap();

        let tx = db.begin().unwrap();
        let err = apply_result(&tx, &sample_result("d-1")).unwrap_err();
        assert!(
            matches!(
                &err,
                DecisionError::Contract(ContractError::Persist(PersistError::Database(m)))
                    if m.contains("contract")
            ),
            "失败点 2 期望 contract 撞主键的 Err，实得 {err:?}"
        );
        drop(tx);

        let tx = db.begin().unwrap();
        assert_eq!(
            decision_result_count(&tx, "d-1"),
            0,
            "失败点 2 回滚后 decision_result 无行（决定步的红利随回滚消失）"
        );
        assert_eq!(
            raw_contract_row(&tx, "c-1", 2),
            c2_before,
            "失败点 2：预先存在的 v2 行 raw 逐列未变（撞键不是覆盖）"
        );
        assert_eq!(
            tx.query("SELECT COUNT(*) FROM plan WHERE id = 'p-1' AND major = 2", &[])
                .unwrap()[0][0],
            Value::Int(0),
            "失败点 2 回滚后 plan 无新版本"
        );
        assert_eq!(
            load_intent(&tx, &IntentId::from_str("i-1")).unwrap().unwrap().status,
            IntentStatus::Suspended,
            "失败点 2 回滚后 Intent 仍 SUSPENDED"
        );
        assert_eq!(
            tx.audit_records().unwrap().len(),
            audit_before,
            "失败点 2 回滚后审计不增（决定步写的那条随回滚撤销）"
        );
        tx.commit().unwrap();
    }

    // ---- 失败点 3：plan 新版本撞主键（v2.0 预先存在）----
    {
        let (_d, db) = new_db();
        let tx = db.begin().unwrap();
        commit_start_state(&tx);
        save_plan(&tx, &sample_plan("p-1", version(2, 0))).unwrap();
        tx.commit().unwrap();
        let tx = db.begin().unwrap();
        let p2_before = raw_plan_row(&tx, "p-1", version(2, 0));
        let p1_before = raw_plan_row(&tx, "p-1", version(1, 0));
        let audit_before = tx.audit_records().unwrap().len();
        tx.commit().unwrap();

        let tx = db.begin().unwrap();
        let err = apply_result(&tx, &sample_result("d-1")).unwrap_err();
        assert!(
            matches!(
                &err,
                DecisionError::Plan(PlanError::Persist(PersistError::Database(m)))
                    if m.contains("plan")
            ),
            "失败点 3 期望 plan 撞主键的 Err，实得 {err:?}"
        );
        drop(tx);

        let tx = db.begin().unwrap();
        assert_eq!(
            decision_result_count(&tx, "d-1"),
            0,
            "失败点 3 回滚后 decision_result 无行"
        );
        assert_eq!(
            tx.query("SELECT COUNT(*) FROM contract WHERE version = 2", &[]).unwrap()[0][0],
            Value::Int(0),
            "失败点 3 回滚后 contract 新版本也撤掉（它写在 plan 步之前）"
        );
        assert_eq!(
            raw_plan_row(&tx, "p-1", version(2, 0)),
            p2_before,
            "失败点 3：预先存在的 v2.0 行 raw 逐列未变"
        );
        assert_eq!(
            raw_plan_row(&tx, "p-1", version(1, 0)),
            p1_before,
            "失败点 3：旧版本行未变（若实现先改旧行再插新行，回滚也须复原）"
        );
        assert_eq!(
            load_intent(&tx, &IntentId::from_str("i-1")).unwrap().unwrap().status,
            IntentStatus::Suspended,
            "失败点 3 回滚后 Intent 仍 SUSPENDED"
        );
        assert_eq!(
            tx.audit_records().unwrap().len(),
            audit_before,
            "失败点 3 回滚后审计不增"
        );
        tx.commit().unwrap();
    }

    // ---- 失败点 4：Intent 回迁非法（起点是 PROPOSED）----
    {
        let (_d, db) = new_db();
        let tx = db.begin().unwrap();
        save_intent(&tx, &sample_intent("i-1", IntentStatus::Proposed)).unwrap();
        create_decision(&tx, &sample_decision("d-1", "i-1"), 5_000).unwrap();
        save_contract(&tx, &sample_contract("c-1", 1), 5_000).unwrap();
        save_plan(&tx, &sample_plan("p-1", version(1, 0))).unwrap();
        tx.commit().unwrap();
        let tx = db.begin().unwrap();
        let audit_before = tx.audit_records().unwrap().len();
        tx.commit().unwrap();

        let tx = db.begin().unwrap();
        let err = apply_result(&tx, &sample_result("d-1")).unwrap_err();
        assert!(
            matches!(
                &err,
                DecisionError::Intent(IntentError::Transition(IntentStatusError::Illegal {
                    from: IntentStatus::Proposed,
                    to: IntentStatus::Running,
                }))
            ),
            "失败点 4 期望 PROPOSED → RUNNING 的非法迁移，实得 {err:?}"
        );
        drop(tx);

        let tx = db.begin().unwrap();
        assert_eq!(
            decision_result_count(&tx, "d-1"),
            0,
            "失败点 4 回滚后 decision_result 无行"
        );
        assert_eq!(
            tx.query("SELECT COUNT(*) FROM contract WHERE version = 2", &[]).unwrap()[0][0],
            Value::Int(0),
            "失败点 4 回滚后 contract 新版本撤掉"
        );
        assert_eq!(
            tx.query("SELECT COUNT(*) FROM plan WHERE id = 'p-1' AND major = 2", &[])
                .unwrap()[0][0],
            Value::Int(0),
            "失败点 4 回滚后 plan 新版本撤掉"
        );
        assert_eq!(
            raw_plan_row(&tx, "p-1", version(1, 0))[10],
            Value::text("user_approved"),
            "失败点 4 回滚后旧 plan 行的 review_state 复原"
        );
        assert_eq!(
            load_intent(&tx, &IntentId::from_str("i-1")).unwrap().unwrap().status,
            IntentStatus::Proposed,
            "失败点 4 回滚后 Intent 仍是 PROPOSED"
        );
        assert_eq!(
            tx.audit_records().unwrap().len(),
            audit_before,
            "失败点 4 回滚后审计不增"
        );
        tx.commit().unwrap();
    }
}

// ===== §5.4：UserApprovals 审计（第二生产方） =====

/// `DecisionResult` 落库写出**恰一条** `AuditKind::UserApprovals`（§5.4 的写入点之一）。
///
/// # 断言三件
///
/// - kind **是 `UserApprovals`**（经 `Tx::audit_records()` 读回那一枚，不是「写了审计」）；
/// - `occurred_at` 是 `decided_at`（本层不取时钟——决定时刻由调用方在记录里给）；
/// - payload 是**决策单与被选项**（`decision_id` ＋ `option_index`）。
///
/// # 并写明（§5.4、§16 第 33 条）——**本层是它的第二个生产方**
///
/// 第一个是 `continuum-workspace` 的 Integration Gate（`crates/continuum-workspace/src/gate.rs`
/// 的 `"apply_patch"` 等操作名）。`append_audit` 的 `payload` 是**自由 JSON**
/// （`crates/continuum-persist/src/tx.rs`）：同一个 kind 下**不保证载荷同形**——
/// 读 `audit_log` 的一方**不得假定「一个 kind 一种载荷」**。本层**不改 Gate、也不新增
/// kind**。这也使得 `AuditKind::UserApprovals` 是 §5.4 里**唯一非首个生产方**的一项。
///
/// 负侧：同一 Decision 的第二个结果撞 `decision_result` 的主键（fail-closed——
/// 一个决策恰一个回答；要重答得先有一条能表达「多个结果」的设计）。
///
/// **红条件**：kind 写错（**取反档**）；payload 不带决策单与被选项；二次写静默成功。
#[test]
fn a_user_approvals_audit_row_is_written_for_a_decision_result() {
    let (_d, db) = new_db();
    let tx = db.begin().unwrap();

    let r = sample_result("d-1");
    save_decision_result(&tx, &r).unwrap();

    let records = tx.audit_records().unwrap();
    assert_eq!(records.len(), 1, "一次落库应恰写一条审计");
    assert_eq!(
        records[0].kind,
        AuditKind::UserApprovals,
        "kind 应是 `UserApprovals` 那一枚"
    );
    assert_eq!(
        records[0].occurred_at, r.decided_at,
        "审计的 occurred_at 应是决定时刻（记录里的 decided_at）"
    );
    assert_eq!(
        records[0].payload.get("decision_id").and_then(|v| v.as_str()),
        Some("d-1"),
        "payload 应带决策单"
    );
    assert_eq!(
        records[0].payload.get("option_index").and_then(|v| v.as_u64()),
        Some(1),
        "payload 应带被选项"
    );
    assert_eq!(
        decision_result_count(&tx, "d-1"),
        1,
        "DecisionResult 应恰落一行"
    );

    // 负侧：同一 Decision 的第二个结果撞主键（不是静默第二条）。
    let mut second = sample_result("d-1");
    second.option_index = 2;
    let err = save_decision_result(&tx, &second).unwrap_err();
    assert!(
        matches!(&err, DecisionError::Persist(PersistError::Database(m)) if m.contains("decision_result")),
        "第二个结果期望撞 decision_result 主键的 Err，实得 {err:?}"
    );
    assert_eq!(
        decision_result_count(&tx, "d-1"),
        1,
        "撞键后不应有第二条（也不是覆写）"
    );

    tx.commit().unwrap();
}

// ===== 错误路径：「指向」的 fail-closed =====

/// `DecisionResult` 的「指向」两处都 fail-closed（**不静默**）：
///
/// - `decision_id` 不存在 ⇒ [`DecisionError::UnknownDecision`]（点名 id）——同
///   `apply_intent_transition` 的 `UnknownIntent` 处置：把一次笔误的 id 变成一次
///   「恢复」，会让「没见过的决策」与「打错字的决策」在库里长得一样；
/// - `option_index` 越界 ⇒ [`DecisionError::UnknownOption`]（点名 id 与下标）——
///   「指向某个 DecisionOption」指向不存在的项，判不了就走 `Err`。
///
/// 两侧都不写任何行。**红条件**：静默通过／越界被钳到某一项（**放宽档**）。
#[test]
fn a_result_pointing_at_a_missing_decision_or_option_is_rejected() {
    let (_d, db) = new_db();
    let tx = db.begin().unwrap();
    commit_start_state(&tx);

    // 不存在 decision：连 Decision 都没造。
    let err = apply_result(&tx, &sample_result("d-404")).unwrap_err();
    assert_eq!(
        err,
        DecisionError::UnknownDecision {
            id: "d-404".to_owned()
        },
        "不存在的 decision_id 应原样带出"
    );

    // 越界 option_index（d-1 只有 3 项，下标 3 越界）。
    let r = DecisionResult {
        decision_id: DecisionId::from_str("d-1"),
        option_index: 3,
        contract_update: None,
        plan_update: None,
        decided_at: 10_000,
    };
    let err = apply_result(&tx, &r).unwrap_err();
    assert_eq!(
        err,
        DecisionError::UnknownOption {
            decision_id: "d-1".to_owned(),
            index: 3,
        },
        "越界的 option_index 应原样带出（含下标）"
    );

    // 两侧都不写任何行。
    assert_eq!(decision_result_count(&tx, "d-1"), 0);
    assert_eq!(row_count(&tx, "decision_result"), 0, "被拒的恢复不该留行");
    assert_eq!(
        load_intent(&tx, &IntentId::from_str("i-1")).unwrap().unwrap().status,
        IntentStatus::Suspended,
        "被拒的恢复不该改 Intent 状态"
    );
    tx.commit().unwrap();
}

/// `ContractUpdate` 的 fail-closed 两侧：
///
/// - 基准版本 `(id, base_version)` 不存在 ⇒ [`DecisionError::UnknownContract`]——
///   「新版本」要以一个真实基准起算（§226 的版本化是显式动作），没有基准就不静默新建；
/// - 指名改的 requirement id 不在基准 Contract 里 ⇒ [`DecisionError::UnknownRequirement`]
///   （同 Task 16 悬空键的处置：「判不了」走 `Err`，不静默添加一个 requirement）。
///
/// **红条件**：基准不存在就静默新建／悬空 id 被凭空加进新版本（**放宽档**）。
#[test]
fn a_contract_update_pointing_at_missing_things_is_rejected() {
    let (_d, db) = new_db();
    let tx = db.begin().unwrap();
    commit_start_state(&tx);

    // 基准版本不存在（c-9 没造过）。
    let mut r = sample_result("d-1");
    r.contract_update = Some(ContractUpdate {
        id: ContractId::from_str("c-9"),
        base_version: 1,
        requirements: vec![],
    });
    let err = apply_result(&tx, &r).unwrap_err();
    assert_eq!(
        err,
        DecisionError::UnknownContract {
            id: "c-9".to_owned(),
            version: 1,
        },
        "不存在的基准版本应原样带出 (id, version)"
    );

    // 悬空 requirement id（r-404 不在 c-1 v1 里）。
    let mut r = sample_result("d-1");
    r.contract_update = Some(ContractUpdate {
        id: ContractId::from_str("c-1"),
        base_version: 1,
        requirements: vec![RequirementUpdate {
            id: RequirementId::from_str("r-404"),
            class: Some(RequirementClass::Preferred),
            expression: None,
            source: None,
            verification_requirement: None,
        }],
    });
    let err = apply_result(&tx, &r).unwrap_err();
    assert_eq!(
        err,
        DecisionError::UnknownRequirement {
            contract_id: "c-1".to_owned(),
            version: 1,
            id: "r-404".to_owned(),
        },
        "悬空的 requirement id 应原样带出（不静默添加）"
    );

    // 被拒的两次都不留行。
    assert_eq!(row_count(&tx, "decision_result"), 0);
    assert!(
        load_contract(&tx, &ContractId::from_str("c-1"), 2)
            .unwrap()
            .is_none(),
        "被拒的更新不该凭空造出 v2"
    );
    tx.commit().unwrap();
}

/// `PlanUpdate` 的基准版本不存在 ⇒ [`DecisionError::UnknownPlan`]（点名 id 与两段版本）。
///
/// **红条件**：静默新建（**放宽档**）。
#[test]
fn a_plan_update_pointing_at_a_missing_plan_is_rejected() {
    let (_d, db) = new_db();
    let tx = db.begin().unwrap();
    commit_start_state(&tx);

    let mut r = sample_result("d-1");
    r.plan_update = Some(PlanUpdate {
        id: PlanId::from_str("p-9"),
        base_version: version(1, 0),
        objectives: Some(vec![Opaque::from_str("新的目标")]),
        milestones: None,
        assumptions: None,
        risks: None,
        acceptance_conditions: None,
    });
    let err = apply_result(&tx, &r).unwrap_err();
    assert_eq!(
        err,
        DecisionError::UnknownPlan {
            id: "p-9".to_owned(),
            major: 1,
            minor: 0,
        },
        "不存在的基准版本应原样带出 (id, major, minor)"
    );

    assert_eq!(row_count(&tx, "decision_result"), 0);
    assert!(
        load_plan(&tx, &PlanId::from_str("p-9"), version(1, 0))
            .unwrap()
            .is_none(),
        "被拒的更新不该凭空造出 Plan"
    );
    tx.commit().unwrap();
}

// ===== 表外取值一律具体 Err，不取默认值 =====

/// `decision.blocking` 的表外取值（非 0/1）读回**具体 `Err`**（[`DecisionError::InvalidBlocking`]，
/// 带原始数值、**不取默认值**——枚举 / 布尔列纪律的读回侧）。
///
/// **注意**：`plan.type` / `plan.review_state` 的两个表外取值照片在 `tests/plan.rs`
/// 的 `enum_codes_use_lowercase_underscore_and_reject_out_of_table_values`（那两列的
/// 编码对已随 Task 15 的行级 I/O 落地收窄为 `pub(crate)`，照片经库读回）——本用例
/// 不重复它。
///
/// **红条件**：表外取值被静默映射成某一枚（**放宽档**）；读回 `Ok` 或不点名原始取值即红。
#[test]
fn an_out_of_table_blocking_value_is_rejected_on_read() {
    let (_d, db) = new_db();
    let tx = db.begin().unwrap();
    commit_start_state(&tx);

    // decision.blocking：raw 改成 2。
    tx.execute("UPDATE decision SET blocking = 2 WHERE id = 'd-1'", &[])
        .unwrap();
    let err = load_decision(&tx, &DecisionId::from_str("d-1")).unwrap_err();
    assert_eq!(
        err,
        DecisionError::InvalidBlocking { value: 2 },
        "blocking 的表外取值应原样带出（不是静默当成 true）"
    );

    tx.commit().unwrap();
}

// ===== 记录键 fail-closed =====

/// 同 `id` 的第二次 `create_decision` **撞主键得 `Err`**（不是 `INSERT OR REPLACE`
/// 静默覆写——那会把一张单悄悄换成另一张）。
///
/// **红条件**：静默覆写（**移除档**）；原行被改动（逐列断言红）。
#[test]
fn a_second_creation_of_the_same_id_hits_the_primary_key() {
    let (_d, db) = new_db();
    let tx = db.begin().unwrap();

    create_decision(&tx, &sample_decision("d-1", "i-1"), 5_000).unwrap();
    let before = tx
        .query("SELECT * FROM decision WHERE id = ?1", &[Value::text("d-1")])
        .unwrap();
    assert_eq!(before.len(), 1);

    // 内容不同的同 id 记录（issue 与选项都换过）——「原行未变」才有内容判别力。
    let mut second = sample_decision("d-1", "i-1");
    second.issue = Opaque::from_str("另一张单");
    second.options = vec![DecisionOption::new(
        Opaque::from_str("另一个选项"),
        Opaque::from_str("另一个后果"),
    )];
    let err = create_decision(&tx, &second, 6_000).unwrap_err();
    assert!(
        matches!(&err, DecisionError::Persist(PersistError::Database(m)) if m.contains("decision")),
        "第二次创建期望撞主键的 Err，实得 {err:?}"
    );
    let after = tx
        .query("SELECT * FROM decision WHERE id = ?1", &[Value::text("d-1")])
        .unwrap();
    assert_eq!(after, before, "撞键后原行应 raw 逐列不变");

    tx.commit().unwrap();
}

// ===== 源码扫描的共用件（照 tests/contract.rs 的同名件） =====

/// 本 crate `src/` 下的全部 `.rs`（路径排序、**相对 `src/`**——出现点清单里写的是
/// 相对路径，照 `tests/contract.rs` 的同名件）。
fn crate_sources() -> Vec<(PathBuf, String)> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rs(&src, &mut files);
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let rel = Path::new("src")
                .join(path.strip_prefix(&src).expect("收集自 `src/` 下的路径应以它开头"));
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("读不到源文件 {}：{e}", path.display()));
            (rel, source)
        })
        .collect()
}

/// 递归收集 `dir` 下全部 `.rs`。**读不到的目录直接炸，不跳过**——静默跳过只会让守卫变窄。
fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("列不出目录 {}：{e}", dir.display()))
    {
        let path = entry
            .unwrap_or_else(|e| panic!("列目录 {} 时出错：{e}", dir.display()))
            .path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// 剥掉注释后的源代码（字符串字面量里的 `//` 不算注释——`tests/contract.rs` 的同名件
/// 的理由逐字相同：扫描的是**代码行**，注释不参与）。
fn strip_comments(src: &str) -> String {
    // 四种状态：0 = 代码，1 = 行注释，2 = 块注释，3 = 字符串字面量。
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let mut state = 0u8;
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        match state {
            0 if c == '/' && next == Some('/') => {
                state = 1;
                i += 2;
            }
            0 if c == '/' && next == Some('*') => {
                state = 2;
                i += 2;
            }
            0 => {
                if c == '"' {
                    state = 3;
                }
                out.push(c);
                i += 1;
            }
            1 => {
                if c == '\n' {
                    state = 0;
                    out.push(c);
                }
                i += 1;
            }
            2 => {
                if c == '*' && next == Some('/') {
                    state = 0;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            3 => {
                if c == '\\' {
                    out.push(c);
                    if let Some(escaped) = next {
                        out.push(escaped);
                    }
                    i += 2;
                } else {
                    if c == '"' {
                        state = 0;
                    }
                    out.push(c);
                    i += 1;
                }
            }
            _ => unreachable!(),
        }
    }
    out
}
