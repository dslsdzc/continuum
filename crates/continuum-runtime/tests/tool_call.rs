//! 工具调用路径的库级用例（F 设计 §3、§6.4、§9）。
//!
//! 这些用例经 `continuum_runtime::tool_call::run_tool_call` 直接看**这条路径本身**：
//! 它调没调注册表、把哪一份证明与哪一段输入交给了适配器、失败时报哪一个变体。驱动二进制
//! 看不到这些——生产的装配点是空的（`tool_cmd.rs` 的文档），夹具适配器只能由测试代码构造，
//! 而夹具是**用例的一部分**，不是生产形状的样例。
//!
//! # 夹具为什么是第二份副本（**不合并**）
//!
//! 形状照 `crates/continuum-provider/tests/common/mod.rs` 的 `FakeTool` / `RecordingTool`。
//! 跨 crate 的 `tests/` 目录不可互相导入，故只能另写一份。**两份各有其用、不合并**
//! （`docs/superpowers/p3bcdf-followups.md` §七第 8 条）：C 的那一份钉的是「工具级失败走
//! `Ok(is_error: true)`、provider 级失败走 `Err`」这条**加在适配器上的约定**；本文件的这一份
//! 只服务本路径的用例，要记的是**被调用的次数**与**收到的那份请求**（工具 id 与 input），
//! 而 `ToolResult` 里没有任何回执字段可比对——那是设计 §7.1 记下的限度。
//!
//! # 「不登记任何适配器」那条用例为什么仍要往 `tool` 表里写一行
//!
//! 强制点 (1)（`authorize`）读的是 **`tool` 表**（登记项：要哪些能力、什么 `effect_class`），
//! 而路由看的是**注册表**（谁去跑）。两者是两件事（C 设计 §3.3 代价三）。不登记适配器
//! 指的是后者，前者必须有一条合法登记项，否则它先以 `UnknownTool` 拒掉、走不到那一跳。

use async_trait::async_trait;
use continuum_capability::{
    CapabilityError, CapabilityKind, FsAction, GitAction, GithubAction, Tool, ToolProfile, Trust,
    save_tool,
};
use continuum_core::ProviderError;
use continuum_core::tool::{ToolId, ToolResult};
use continuum_effect::{EffectState, EffectType, find_by_idempotency_key};
use continuum_persist::{Db, Migration, PersistError};
use continuum_policy::{Condition, Decision, Level, Policy, Scope, save_policy};
use continuum_provider::ProviderRegistry;
use continuum_provider::ToolCallError;
use continuum_provider::tool::{AuthorizedToolInvocation, ToolProvider};
use continuum_runtime::TaskError;
use continuum_runtime::cli::{EffectSpec, ToolArgs};
use continuum_runtime::tool_call::run_tool_call;
use continuum_workspace::IntentId;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

// ── 夹具 ────────────────────────────────────────────────────────────────

/// 夹具适配器交出什么。
#[derive(Clone, Copy)]
enum Outcome {
    /// 回显输入，`is_error: false`。
    Echo,
    /// **先睡若干毫秒**再回显输入，`is_error: false`。
    ///
    /// 只服务一条用例：证明**终态那次的 `updated_at` 是「推进那一刻」，不是步骤 4 取的那个
    /// `now`**（工具调用可以跑任意久，故两者之间的间隔是这条性质唯一的可观察形式）。用
    /// `std::thread::sleep` 而不是异步定时器：本用例跑在**当前线程**运行时上、且不开
    /// `enable_all`（没有 `time` feature 的反应堆），阻塞睡正是这里能用的那一种。
    EchoAfterDelay(u64),
    /// **适配器自己**没跑成：`Err(ProviderError::Transport(..))`。
    ProviderFailure,
}

/// 夹具适配器**收到的一次请求**。
///
/// `ToolResult` 里没有回执字段（设计 §7.1 的限度），故「适配器收到的授权是哪一枚、输入是
/// 哪一段」只能由适配器自己抄进一块共享内存，用例再从观测端读回来。
#[derive(Clone, Debug)]
struct Seen {
    tool_id: ToolId,
    input: Value,
}

/// 记录型夹具适配器：记两件事——**被调用的次数**与**每次收到的那份请求**。
struct RecordingTool {
    outcome: Outcome,
    calls: Arc<AtomicUsize>,
    seen: Arc<Mutex<Vec<Seen>>>,
}

impl RecordingTool {
    fn new(outcome: Outcome) -> Self {
        Self {
            outcome,
            calls: Arc::new(AtomicUsize::new(0)),
            seen: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 被调用过几次（只数 `invoke`——`list_tools` / `describe_tool` 本路径不调，
    /// 与 C 那份「三者任一都算」的计数口径**不同**：这里要断言的是「那一跳发生了几次」）。
    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    /// 收到的全部请求，按调用次序。
    fn seen(&self) -> Vec<Seen> {
        self.seen.lock().unwrap().clone()
    }
}

#[async_trait]
impl ToolProvider for RecordingTool {
    async fn list_tools(&self) -> Result<Vec<continuum_core::tool::ToolDescriptor>, ProviderError> {
        Ok(Vec::new())
    }

    async fn describe_tool(
        &self,
        id: &ToolId,
    ) -> Result<continuum_core::tool::ToolDescriptor, ProviderError> {
        Err(ProviderError::Unavailable(id.as_str().to_owned()))
    }

    /// **夹具按 C 的约定交出结果**（工具级失败走 `Ok(is_error: true)`、provider 级失败走
    /// `Err`）：这是对约定的服从，不是「适配器都这么干」。本文件今天只用得到 provider 级
    /// 那一条与正常那一条——`is_error: true` 那一支的处置在 Task 6（见 `run_tool_call`
    /// 的文档「今天没做完的那一格」）。
    async fn invoke(
        &self,
        call: AuthorizedToolInvocation<'_>,
    ) -> Result<ToolResult, ProviderError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.seen.lock().unwrap().push(Seen {
            // 工具 id 只经授权证明取（设计 §7.5）——夹具读它，正是为了把那份证明记下来。
            tool_id: call.authorization().tool_id().clone(),
            input: call.input().clone(),
        });
        match self.outcome {
            Outcome::Echo => Ok(ToolResult {
                output: call.input().clone(),
                is_error: false,
            }),
            Outcome::EchoAfterDelay(millis) => {
                std::thread::sleep(std::time::Duration::from_millis(millis));
                Ok(ToolResult {
                    output: call.input().clone(),
                    is_error: false,
                })
            }
            Outcome::ProviderFailure => {
                Err(ProviderError::Transport("适配器自己没跑成".to_owned()))
            }
        }
    }

    async fn cancel(&self, _call: &continuum_core::model::CallId) -> Result<(), ProviderError> {
        Ok(())
    }
}

/// 一次用例的固定装置：一个真库（含 `tool` 表）与一条登记项。
struct Fixture {
    /// 保活用：临时目录一析构，库文件就没了。字段从不读取，故用下划线名。
    _dir: tempfile::TempDir,
    db_path: PathBuf,
    tool: ToolId,
}

impl Fixture {
    /// 建库、建表、往 `tool` 表写一条登记项。
    ///
    /// `required` 是工具的**声明表**（强制点 (1) 比对的对象），`effect_class` 必须被它覆盖
    /// （`save_tool` 的登记期不变量）。多个 id 各写一条，用于「授权哪个工具」那条用例。
    fn with_tools(
        ids: &[&str],
        required: Vec<CapabilityKind>,
        effect_class: Option<EffectType>,
    ) -> Self {
        let rows: Vec<(&str, Vec<CapabilityKind>, Option<EffectType>)> = ids
            .iter()
            .map(|id| (*id, required.clone(), effect_class))
            .collect();
        Self::with_registrations(&rows)
    }

    /// 同上，但**每条登记项自带 id 与声明表**（`with_tools` 是「一张声明表给多个 id」的那一格）。
    ///
    /// 用在「六条登记项各声明一枚不同的 kind」那种用例上：那些行的声明表各不相同，
    /// 用 `with_tools` 表达不出来。
    fn with_registrations(rows: &[(&str, Vec<CapabilityKind>, Option<EffectType>)]) -> Self {
        assert!(!rows.is_empty(), "夹具至少要有一条登记项");
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("continuum.db");
        let db = open_db(&db_path);
        let tx = db.begin().unwrap();
        for (id, required, effect_class) in rows {
            let tool = Tool::new(
                ToolId::new(*id),
                "1.0".to_owned(),
                json!({"type": "object"}),
                json!({"type": "object"}),
                required.clone(),
                *effect_class,
                true,
            );
            save_tool(&tx, &ToolProfile::new(tool, None, None, Trust)).unwrap();
        }
        tx.commit().unwrap();
        Self {
            _dir: dir,
            db_path,
            tool: ToolId::new(rows[0].0),
        }
    }

    /// 往 `policy` 表放一条**恒真**的 `Allow`（第 5 级）。
    ///
    /// 用它而不是带事实的条件：本文件要钉的是这条路径的接线，条件怎么求值是
    /// `continuum-policy` 的范围。
    fn allow_everything(&self) {
        let db = open_db(&self.db_path);
        let tx = db.begin().unwrap();
        let policy = Policy {
            level: Level::RuntimeDefault,
            condition: Condition::parse(&json!({"all": []})).expect("空合取是合法条件"),
            decision: Decision::Allow,
            scope: Scope::User,
        };
        save_policy(&tx, "allow-all", &policy).unwrap();
        tx.commit().unwrap();
    }

    /// 往 `policy` 表再放一条**只对某一种效应类型成立**的 `RequireApproval`（第 2 级）。
    ///
    /// 它**与第 2 级内建的 `Allow` 同层且更严**，故 `--approve` 已给出时它照样取胜
    /// （同层取更严）——这正是让「两条效应各自得到不同裁决」可造出来的那条规则。
    fn require_approval_for(&self, effect_type: EffectType) {
        let db = open_db(&self.db_path);
        let tx = db.begin().unwrap();
        let policy = Policy {
            level: Level::ExplicitCurrent,
            condition: Condition::parse(&json!({
                "all": [{"fact": "effect_type", "eq": effect_type.as_str()}]
            }))
            .expect("effect_type 是封闭事实，取值取自枚举"),
            decision: Decision::RequireApproval,
            scope: Scope::User,
        };
        save_policy(&tx, "charge-needs-approval", &policy).unwrap();
        tx.commit().unwrap();
    }

    /// 本条路径的输入参数。默认是零效应的最小合法形态，用例按需改写。
    fn args(&self) -> ToolArgs {
        ToolArgs {
            db: self.db_path.clone(),
            tool: self.tool.clone(),
            input: json!({}),
            intent: None,
            approve: false,
            effects: Vec::new(),
        }
    }

    /// 跑一次 `run_tool_call`（开库一次，与 bin 侧的装配同形）。
    fn run(&self, registry: &ProviderRegistry, args: &ToolArgs) -> Result<(), TaskError> {
        let db = open_db(&self.db_path);
        run_tool_call(&db, registry, args)
    }

    /// 库里某条效应的状态；不存在则是 `None`。
    ///
    /// 幂等键用**手写的字面量**（`<意图字节长>:<意图>:<类型>:<目标>`）而不是调
    /// `effect_key` 去拼：格式改了两者一起漂移、断言照过（`task_cli.rs` 的同一条取向）。
    fn effect_state(&self, key: &str) -> Option<EffectState> {
        let db = open_db(&self.db_path);
        let tx = db.begin().unwrap();
        let state = find_by_idempotency_key(&tx, key).unwrap().map(|e| e.state);
        tx.commit().unwrap();
        state
    }

    /// 库里某条效应的 `authorization` 字段（第 6.7 节那个只记录、不校验的不透明串）。
    fn effect_authorization(&self, key: &str) -> Option<String> {
        let db = open_db(&self.db_path);
        let tx = db.begin().unwrap();
        let field = find_by_idempotency_key(&tx, key)
            .unwrap()
            .map(|e| e.authorization);
        tx.commit().unwrap();
        field
    }

    /// 库里某条效应的 `(planned_at, updated_at)`。
    fn effect_times(&self, key: &str) -> Option<(i64, i64)> {
        let db = open_db(&self.db_path);
        let tx = db.begin().unwrap();
        let times = find_by_idempotency_key(&tx, key)
            .unwrap()
            .map(|e| (e.planned_at, e.updated_at));
        tx.commit().unwrap();
        times
    }

    /// `effect` 表的**总行数**。
    ///
    /// 「一条都不许落下」这类断言只能读数行数：本文件其余访问器都按幂等键取某一条，
    /// 那样读不出「别的行也没被写」。
    fn effect_rows(&self) -> i64 {
        self.count("SELECT COUNT(*) FROM effect")
    }

    /// `audit_log` 表的**总行数**。
    fn audit_rows(&self) -> i64 {
        self.count("SELECT COUNT(*) FROM audit_log")
    }

    /// `audit_log` 里 `kind` 列恰为 `"capability grants"` 的行数。
    ///
    /// 判据是**手写的字面量**，不是把 `AuditKind::CapabilityGrants` 转一圈——那样只证明
    /// 枚举等于自己、钉不住落库编码（`crates/continuum-capability/tests/authorize.rs`
    /// 的同名断言用同一条判据）。
    fn capability_grants_rows(&self) -> i64 {
        self.count("SELECT COUNT(*) FROM audit_log WHERE kind = 'capability grants'")
    }

    /// 一条 `SELECT COUNT(*)`。
    fn count(&self, sql: &str) -> i64 {
        let db = open_db(&self.db_path);
        let tx = db.begin().unwrap();
        let rows = tx.query(sql, &[]).unwrap();
        tx.commit().unwrap();
        int_of(&rows[0][0])
    }
}

/// 本文件要用的迁移集合：`tool` / `policy` / `effect` 三张表 + 内建（`audit_log`）。
///
/// 不复用 `main.rs` 的那份清单（它在 bin 里、`pub(crate)`）：那两份的对照是
/// `tests/migrations.rs` 的活。
fn migrations() -> Vec<Migration> {
    let mut migrations = continuum_persist::builtin_migrations();
    migrations.extend(continuum_capability::p3_capability_migrations());
    migrations.extend(continuum_policy::p2_policy_migrations());
    migrations.extend(continuum_effect::p2_effect_migrations());
    migrations
}

fn open_db(path: &Path) -> Db {
    let db = Db::open_with(path, migrations()).expect("打开数据库失败");
    db.migrate().expect("应用迁移失败");
    db
}

/// 绕过 [`save_tool`] 直接写一行：模拟**被写坏的表外取值**（照
/// `crates/continuum-capability/tests/authorize.rs` 的同名函数）。
///
/// 只写坏 `required_capabilities` 一列——`load_tool` 解码它时必然失败，于是强制点 (1)
/// 读登记项那一次往返报错。用裸 SQL 是因为 `save_tool` 会先过登记期不变量与编码，
/// 那两条正是本用例要**绕过**的东西（要造的是「表里已经躺着一条坏的」）。
fn insert_bad_capabilities(db_path: &Path, id: &str, raw: &str) {
    // 列取值用 `continuum_persist::Value`（**写路径的那个编码类型**），不 import 它：
    // 本文件的 `Value` 是 `serde_json::Value`（请求侧的输入），两者同名不同物。
    let db = open_db(db_path);
    let tx = db.begin().unwrap();
    tx.execute(
        "INSERT INTO tool
           (id, version, input_schema, output_schema, required_capabilities,
            effect_class, deterministic, cost, latency, trust)
         VALUES (?1, '1', '{}', '{}', ?2, NULL, 0, NULL, NULL, '')",
        &[
            continuum_persist::Value::text(id),
            continuum_persist::Value::text(raw),
        ],
    )
    .unwrap();
    tx.commit().unwrap();
}

/// 一列整数（本文件的 `SELECT COUNT(*)` 与 `crates/continuum-capability/tests/authorize.rs`
/// 的 `int_of` 同形：列的类型不符即 panic，不猜）。
fn int_of(v: &continuum_persist::Value) -> i64 {
    match v {
        continuum_persist::Value::Int(i) => *i,
        other => panic!("列应为整数，实际 {other:?}"),
    }
}

/// 一个只登记了 `ids` 的注册表。
fn registry_serving(ids: &[&str], adapter: Arc<RecordingTool>) -> ProviderRegistry {
    let mut registry = ProviderRegistry::new();
    registry
        .register_tool(ids.iter().map(|id| ToolId::new(*id)).collect(), adapter)
        .expect("登记不应重复");
    registry
}

/// 一条 `--effect <类型>:<目标>`。
fn spec(effect_type: EffectType, target: &str) -> EffectSpec {
    EffectSpec {
        effect_type,
        target: target.to_owned(),
    }
}

/// 一条**放行的、声明了一条效应**的调用的参数：`--effect charge:c1 --intent i1`。
fn allowed_charge_call(fixture: &Fixture) -> ToolArgs {
    let mut args = fixture.args();
    args.effects = vec![spec(EffectType::Charge, "c1")];
    args.intent = Some(IntentId::new("i1"));
    args
}

// ── 用例 ────────────────────────────────────────────────────────────────

/// P-6：**F 确实经注册表调用**，不是自己另开一条路。
///
/// 登记一个夹具适配器，跑一条放行的调用，断言它**收到了一次调用**。
/// **变异体**：把步骤 6 换成「什么都不做、直接返回 `Ok`」⇒ 本条红。
#[test]
fn the_registry_is_the_only_way_the_tool_is_reached() {
    let fixture = Fixture::with_tools(
        &["t1"],
        vec![CapabilityKind::for_effect(EffectType::Charge)],
        Some(EffectType::Charge),
    );
    fixture.allow_everything();
    let adapter = Arc::new(RecordingTool::new(Outcome::Echo));
    let registry = registry_serving(&["t1"], Arc::clone(&adapter));

    fixture
        .run(&registry, &allowed_charge_call(&fixture))
        .expect("放行的调用应当成功");

    assert_eq!(
        adapter.calls(),
        1,
        "那一跳必须真的走到适配器：注册表是唯一的调用入口"
    );
}

/// P-7：工具 id **由授权证明定**。
///
/// 同一个适配器服务 `t1` 与 `t2` 两个 id（两条登记项都合法），用 `--tool t1` 调用，断言
/// 夹具收到的那份请求里**那枚授权证明的** `tool_id()` 是 `t1`。
///
/// **为什么两个 id 都要在表里**：变异体「把 `authorize` 的实参写成常量 `t2`」要能走到
/// 适配器才红在下面那条断言上——若 `t2` 没登记，它会先在强制点 (1) 以 `UnknownTool` 失败，
/// 红的是 `.expect(..)` 那一行，而**那不是本条要钉的东西**。两个 id 都在，变异体就真的把
/// 一个 `t2` 的证明交给了适配器，红在同一测试体的断言上。
#[test]
fn the_tool_id_comes_from_the_authorized_proof() {
    let fixture = Fixture::with_tools(
        &["t1", "t2"],
        vec![CapabilityKind::for_effect(EffectType::Charge)],
        Some(EffectType::Charge),
    );
    fixture.allow_everything();
    let adapter = Arc::new(RecordingTool::new(Outcome::Echo));
    let registry = registry_serving(&["t1", "t2"], Arc::clone(&adapter));

    fixture
        .run(&registry, &allowed_charge_call(&fixture))
        .expect("放行的调用应当成功");

    let seen = adapter.seen();
    assert_eq!(seen.len(), 1, "夹具应恰好收到一次调用");
    assert_eq!(
        seen[0].tool_id,
        ToolId::new("t1"),
        "请求里那枚证明的工具 id 必须是 --tool 给的那个"
    );
}

/// P-7 的第二半：适配器收到的 `input` 与 `--input` **逐字相同**。
///
/// 输入取一段**非空**的 JSON：省略 `--input` 时的默认值就是 `json!({})`，用它当输入的话，
/// 变异体「`input` 换成 `json!({})` 常量」与正确实现在本用例上不可区分（等价变异体）。
/// **变异体**：把 `input` 写成 `json!({})` 常量 ⇒ 本条红。
#[test]
fn the_input_reaches_the_adapter_verbatim() {
    let fixture = Fixture::with_tools(
        &["t1"],
        vec![CapabilityKind::for_effect(EffectType::Charge)],
        Some(EffectType::Charge),
    );
    fixture.allow_everything();
    let adapter = Arc::new(RecordingTool::new(Outcome::Echo));
    let registry = registry_serving(&["t1"], Arc::clone(&adapter));

    let input = json!({"a": 1, "b": [2, 3], "c": {"d": null}});
    let mut args = allowed_charge_call(&fixture);
    args.input = input.clone();

    fixture.run(&registry, &args).expect("放行的调用应当成功");

    let seen = adapter.seen();
    assert_eq!(seen.len(), 1, "夹具应恰好收到一次调用");
    assert_eq!(
        seen[0].input, input,
        "适配器收到的输入必须与 --input 逐字相同（本类型不对它作任何加工）"
    );
}

/// P-17（**P-6 的对照臂**）：零 `--effect` 的调用**照样**走到工具。
///
/// 登记项声明空能力表、调用不带任何效应，故强制点 (2) 铸不出任何东西、出示集为空、
/// `authorize` 的两次比对都空手通过。**变异体**：一个「有效应才调用」的实现本条红，
/// 而 P-6 仍绿——两条都要在。
///
/// **一处要说准**：本条**没有「策略放行」这一回事**——零效应时
/// [`mint_declared_effects`] 一次裁决都不做（策略表照读，但没有任何问题问它），故本用例
/// 故意**不放任何策略**：放了会让人以为放行是必要条件。
#[test]
fn an_effect_free_call_still_reaches_the_tool() {
    let fixture = Fixture::with_tools(&["t1"], Vec::new(), None);
    let adapter = Arc::new(RecordingTool::new(Outcome::Echo));
    let registry = registry_serving(&["t1"], Arc::clone(&adapter));

    let args = fixture.args();
    assert!(args.effects.is_empty() && args.intent.is_none());
    fixture.run(&registry, &args).expect("零效应的调用应当成功");

    assert_eq!(
        adapter.calls(),
        1,
        "零效应不是「不调用」的理由：这一跳与效应条数无关"
    );
}

/// P-9：**不登记任何适配器** → 路由未命中，断言到内层变体，且 id 与 `--tool` 相同。
///
/// **变异体**：把 `Unregistered` 吞成 `Ok`、或换成 `Provider(..)` ⇒ 本条红。
///
/// 登记项仍要写进 `tool` 表（见文件头「为什么仍要往 `tool` 表里写一行」）——否则强制点 (1)
/// 先以 `UnknownTool` 拒掉，测到的就不是路由这一格了。
#[test]
fn an_unregistered_id_reports_the_routing_failure() {
    let fixture = Fixture::with_tools(&["t1"], Vec::new(), None);
    let registry = ProviderRegistry::new();

    match fixture.run(&registry, &fixture.args()) {
        Err(TaskError::ToolCall(ToolCallError::Unregistered { id })) => {
            assert_eq!(id, ToolId::new("t1"), "报的应是 --tool 给的那个 id");
        }
        other => panic!("应报 ToolCall(Unregistered{{t1}})，实际 {other:?}"),
    }
}

/// **每条效应行填的是「这条效应自己那次裁决」**（设计 §7.2，与命令路径的**刻意不同**）。
///
/// 命令路径填的是**集成那次裁决**（`effect_type` 缺省、整个命令只裁一次）；工具调用
/// **没有集成裁决这一回事**，故它填逐条效应那次——两者共用同一个编码函数
/// （`authorization_field`），差别只在传进去的 `Decision` 是哪一个。
///
/// # 这条用例是计划用例清单之外补的，理由是「不补就没有照片」
///
/// 上面那句话若只由文档承载，一个把 `Decision` 写成常量的实现在**下面这一族情形的用例上**
/// 都不会红：**单条效应 ＋ 放行策略**（裁决本来就是 `Allow`，常量 `Allow` 与「填自己的那次」
/// 不可区分）。故这里造一次**两条效应各得不同裁决**的调用——`deploy` 拿 `Allow`、
/// `charge` 拿 `RequireApproval`（同层的更严规则压过内建的 `Allow`，`--approve` 已给出故照样
/// 铸造）。**变异体**：把填进去的 `Decision` 换成常量（`Allow` 或 `RequireApproval` 任一个）
/// ⇒ 本条两条断言里必有一条红。
///
/// **限定词「单条效应 ＋ 放行策略」是必须的（初稿漏掉，来历留此）**：单条效应**并不自动**
/// 让常量变异体不可观察——**单条效应 ＋ 一条按 `effect_type` 限定的 `RequireApproval` 规则 ＋
/// `--approve`** 同样能杀掉「常量 `Allow`」（那条效应的裁决是 `RequireApproval`，而常量给出
/// 的是 `allow`）。本文件已有现成装置 `Fixture::require_approval_for`。故本用例取两条效应**不是**
/// 「唯一能杀它的形状」，而是**一次就把「逐条各带自己的裁决」这件事钉在两个方向上的形状**
/// （一条 `Allow`、一条 `RequireApproval`，任何单一常量必在其中一侧露馅）。
#[test]
fn each_effect_row_records_its_own_verdict() {
    let fixture = Fixture::with_tools(
        &["t1"],
        vec![
            CapabilityKind::for_effect(EffectType::Charge),
            CapabilityKind::for_effect(EffectType::Deploy),
        ],
        Some(EffectType::Charge),
    );
    fixture.allow_everything();
    fixture.require_approval_for(EffectType::Charge);
    let adapter = Arc::new(RecordingTool::new(Outcome::Echo));
    let registry = registry_serving(&["t1"], Arc::clone(&adapter));

    let mut args = fixture.args();
    args.effects = vec![spec(EffectType::Deploy, "d1"), spec(EffectType::Charge, "c1")];
    args.intent = Some(IntentId::new("i1"));
    args.approve = true;

    fixture.run(&registry, &args).expect("两条都铸得出");

    assert_eq!(
        fixture.effect_authorization("2:i1:deploy:d1").as_deref(),
        Some("approve=true;policy=allow"),
        "deploy 那次裁决是内建/落库的 Allow"
    );
    assert_eq!(
        fixture.effect_authorization("2:i1:charge:c1").as_deref(),
        Some("approve=true;policy=require_approval"),
        "charge 那次裁决是它自己那条同层更严的 RequireApproval——\n\
         把 Decision 写成常量的实现红在这一行"
    );
}

/// **终态那次的 `updated_at` 是「推进那一刻」，不是步骤 4 取的那个 `now`。**
///
/// 这是 F Task 3 修复轮 1 把 `finish_declared_effects` 两条路径合并之后，**工具路径唯一被改动的
/// 可观察数据**：合并前终态的 `updated_at` 与 `planned_at` 同值（都取步骤 4 那个 `now`），
/// 合并后它**晚于** `planned_at`。**判它是修正**：`updated_at` 的含义是「这条记录在那一刻被推到
/// 新状态」，而一次工具调用可以跑任意久——旧写法把终态时间戳钉回调用**开始之前**，
/// 与命令路径（第 6 步在命令跑完之后取时钟）的语义也不一致。
///
/// **怎么拍**：让夹具适配器**先睡 50 ms 再返回**，于是「步骤 4 取 `now`」与「终态推进取
/// `now_millis()`」之间有一个确定大于 1 ms 的间隔；断言 `updated_at > planned_at`。
/// 1 ms 的分辨率下，只要中间真的重新取过时钟，这条断言必过；复用旧 `now` 时两者**恰好相等**
/// ——**变异体**：把终态推回成「复用步骤 4 那个 `now`」（即合并前的语义）⇒ **只本条红**
/// （其余用例都不看时间戳）。
#[test]
fn the_terminal_timestamp_is_taken_when_the_call_finishes() {
    let fixture = Fixture::with_tools(
        &["t1"],
        vec![CapabilityKind::for_effect(EffectType::Charge)],
        Some(EffectType::Charge),
    );
    fixture.allow_everything();
    let adapter = Arc::new(RecordingTool::new(Outcome::EchoAfterDelay(50)));
    let registry = registry_serving(&["t1"], Arc::clone(&adapter));

    fixture
        .run(&registry, &allowed_charge_call(&fixture))
        .expect("放行的调用应当成功");

    let (planned_at, updated_at) = fixture
        .effect_times("2:i1:charge:c1")
        .expect("本次调用应当写过这条效应行");
    assert!(
        updated_at > planned_at,
        "终态的 updated_at 必须晚于 planned_at：那一跳花了 50 ms，\
         复用计划时那个 now 会把它钉回调用开始之前（left={updated_at}，right={planned_at}）"
    );
}

/// P-10：适配器的 provider 级失败**原样带出**。
///
/// **变异体**：把它包装成另一种变体（如 `Capability(..)`）⇒ 本条红。
///
/// 本条另有一条断言（**计划用例清单之外，本条补的一处照片**）：调用失败时各效应记
/// `FAILED`（设计 §3 第 6 步）。那条处置今天没有任何别的用例覆盖，而它是一条**写了就要有
/// 照片**的行为——失败收尾若写成「什么都不写」，效应行会停在 `EXECUTING`，与命令路径
/// 「机制没拿到也记 `FAILED`」的处置分岔。
#[test]
fn a_provider_failure_is_carried_through() {
    let fixture = Fixture::with_tools(
        &["t1"],
        vec![CapabilityKind::for_effect(EffectType::Charge)],
        Some(EffectType::Charge),
    );
    fixture.allow_everything();
    let adapter = Arc::new(RecordingTool::new(Outcome::ProviderFailure));
    let registry = registry_serving(&["t1"], Arc::clone(&adapter));

    match fixture.run(&registry, &allowed_charge_call(&fixture)) {
        Err(TaskError::ToolCall(ToolCallError::Provider(ProviderError::Transport(reason)))) => {
            assert_eq!(reason, "适配器自己没跑成", "内层的消息应原样带出");
        }
        other => panic!("应报 ToolCall(Provider(Transport(..)))，实际 {other:?}"),
    }

    assert_eq!(
        fixture.effect_state("2:i1:charge:c1"),
        Some(EffectState::Failed),
        "那一跳失败时各效应记 FAILED（与命令路径「机制没拿到」的处置一致）"
    );
}

/// P-1：**铸不出能力 ⇒ 那一跳不发生，而且库上什么都没落下**（设计 §3.4 的「拒时零调用」）。
///
/// 空策略表 + 无 `--approve` ⇒ 逐条效应那次裁决落进「无规则匹配默认 `Deny`」（设计 §5.3），
/// `mints` 该格不铸造 ⇒ [`TaskError::EffectNotAuthorized`]（**复用既有变体**，本 task 不新立
/// 词汇）。这是**第 3 步**的拒绝，排在强制点 (1)（第 4 步）**之前**。
///
/// # 为什么声明两条效应、且目标是 `c2` 在前
///
/// 「报**声明次序第一条**铸不出的」这句话要有照片就不能只声明一条：只声明一条时，
/// 「报第一条」「报最后一条」「报目标字典序最小者」三种实现给出同一个答案。目标取 `c2` / `c1`
/// 这个次序是**刻意的**——声明次序第一条（`c2`）既不是最后一条、也不是目标字典序最小者，
/// 故这一条断言同时拦住后两种实现。对调次序那一半在 **lib 单元用例**
/// `tool_call::tests::the_shared_minting_function_reports_the_first_unmintable_in_declaration_order`
/// （那里两个方向各跑一次）。
///
/// # 断言次序是刻意的（它决定谁被谁挡住）
///
/// 共享的可观察面（零调用 / 零效应行 / 零审计）排在变体断言**之前**：变体一不成立即 panic，
/// 排在它后面的断言同轮观测不到。前两条各有变异体在本测试体内独立拍到——零调用 ←
/// 把强制点 (2) 的判定换成恒铸造（`mints` 恒 `true`）；零效应行 ← 把 `authorize` 那一跳挪到
/// `tx.commit()` 之后。**本条对「挪后 / 换成放行」那两枚 `authorize` 变异体都是绿的**：
/// 它在第 3 步就返回了，走不到强制点 (1)——这正是它与 P-3 / P-4 / P-5 / P-8 的区别。
#[test]
fn an_unmintable_effect_refuses_the_call_and_the_tool_is_never_invoked() {
    let fixture = Fixture::with_registrations(&[(
        "t1",
        vec![CapabilityKind::for_effect(EffectType::Charge)],
        Some(EffectType::Charge),
    )]);
    // **故意不放任何策略**：这就是「空策略表 ⇒ Deny」的那一半。
    let adapter = Arc::new(RecordingTool::new(Outcome::Echo));
    let registry = registry_serving(&["t1"], Arc::clone(&adapter));

    let mut args = fixture.args();
    args.effects = vec![
        spec(EffectType::Charge, "c2"),
        spec(EffectType::Charge, "c1"),
    ];
    args.intent = Some(IntentId::new("i1"));
    assert!(!args.approve, "本条走的是「无 --approve」那一格");

    let outcome = fixture.run(&registry, &args);

    assert_eq!(
        adapter.calls(),
        0,
        "铸不出能力时那一跳不发生：夹具一次都不该被调到"
    );
    assert_eq!(
        fixture.effect_rows(),
        0,
        "第 3 步拒在前：一条效应行都不许落下（`tx` 未提交，随作用域回滚）"
    );
    assert_eq!(
        fixture.audit_rows(),
        0,
        "零效应行 ⇒ 零审计行（`record_planned` 与两次 `advance` 各写一条）"
    );
    assert_eq!(
        fixture.capability_grants_rows(),
        0,
        "没走到强制点 (1) 的成功路径，`capability grants` 一条都不许有"
    );

    match outcome {
        Err(TaskError::EffectNotAuthorized {
            effect,
            target,
            decision,
        }) => {
            assert_eq!(effect, "charge", "报的是那条效应的类型");
            assert_eq!(
                target, "c2",
                "报的是**声明次序第一条**（c2），不是最后一条（c1）、也不是目标字典序最小者"
            );
            assert_eq!(
                decision, "禁止",
                "空策略表 + 无 --approve ⇒ 无匹配默认 Deny"
            );
        }
        other => panic!("应报 EffectNotAuthorized{{charge,c2}}，实际 {other:?}"),
    }
}

/// P-3：**库里没有这个 id ⇒ `UnknownTool`**，且那一跳不发生、库上什么都没落下。
///
/// 登记项是**另一条**（`other`）：「表里有行、但没有被点名的这个 id」与「表是空的」是两件事，
/// 本条钉的是前者（`load_tool` 对不存在的 id 返回 `Ok(None)`，`authorize` 不把它读成
/// 「这个工具不需要任何能力」）。
///
/// **夹具适配器服务被点名的那个 id**：零调用这条断言才有判别力——若注册表也不认它，
/// 零调用会以一个**无关的理由**成立（那会走到步骤 6 的 `Unregistered`），把强制点 (1)
/// 这一格掩盖掉。
///
/// 调用带一条 `--effect`：不带的活，「零效应行」会因为循环为空而恒成立，那条断言就没有
/// 判别力了（也就没照片）。
#[test]
fn an_unknown_tool_is_rejected() {
    let fixture = Fixture::with_registrations(&[("other", Vec::new(), None)]);
    fixture.allow_everything();
    let adapter = Arc::new(RecordingTool::new(Outcome::Echo));
    let registry = registry_serving(&["nope"], Arc::clone(&adapter));

    let mut args = allowed_charge_call(&fixture);
    args.tool = ToolId::new("nope");

    let outcome = fixture.run(&registry, &args);

    assert_eq!(adapter.calls(), 0, "被拒的调用不许走到那一跳");
    assert_eq!(
        fixture.effect_rows(),
        0,
        "强制点 (1) 拒在第 5 步之前：效应行一条都不许落下"
    );
    assert_eq!(fixture.audit_rows(), 0, "零效应行 ⇒ 零审计行");
    assert_eq!(
        fixture.capability_grants_rows(),
        0,
        "`UnknownTool` 不得留下授权审计"
    );

    match outcome {
        Err(TaskError::Capability(CapabilityError::UnknownTool { id })) => {
            assert_eq!(id, ToolId::new("nope"), "报的应是 --tool 给的那个 id");
        }
        other => panic!("应报 Capability(UnknownTool{{nope}})，实际 {other:?}"),
    }
}

/// P-4：**出示了工具没声明的能力 ⇒ `UndeclaredCapability`**（超范围同样不许）。
///
/// 登记项声明 `Payment(Charge)`，调用声明的是 `--effect deploy:d1`——铸出的
/// `Environment(Deploy)` 那枚不在声明表里。第 4 步按「**先查出示集、再查声明集**」的次序拒
/// （次序的由来与它的照片见 `continuum_capability::authorize` 的文档与
/// `crates/continuum-capability/tests/authorize.rs`
/// 的 `the_presented_set_is_checked_before_the_declared_set`）；本条钉的是这条次序在
/// **本路径**上真的走到了，以及拒得早、拒得干净。
#[test]
fn a_presented_capability_the_tool_did_not_declare_is_rejected() {
    let fixture = Fixture::with_registrations(&[(
        "t1",
        vec![CapabilityKind::for_effect(EffectType::Charge)],
        Some(EffectType::Charge),
    )]);
    fixture.allow_everything();
    let adapter = Arc::new(RecordingTool::new(Outcome::Echo));
    let registry = registry_serving(&["t1"], Arc::clone(&adapter));

    let mut args = fixture.args();
    args.effects = vec![spec(EffectType::Deploy, "d1")];
    args.intent = Some(IntentId::new("i1"));

    let outcome = fixture.run(&registry, &args);

    assert_eq!(adapter.calls(), 0, "被拒的调用不许走到那一跳");
    assert_eq!(
        fixture.effect_rows(),
        0,
        "拒在第 5 步之前：效应行一条都不许落下"
    );
    assert_eq!(fixture.audit_rows(), 0, "零效应行 ⇒ 零审计行");
    assert_eq!(
        fixture.capability_grants_rows(),
        0,
        "`UndeclaredCapability` 不得留下授权审计"
    );

    match outcome {
        Err(TaskError::Capability(CapabilityError::UndeclaredCapability { kind })) => {
            assert_eq!(
                kind,
                CapabilityKind::for_effect(EffectType::Deploy),
                "报的应是出示集里那一枚（`--effect deploy:d1` 铸出的），不是声明表里那枚"
            );
        }
        other => {
            panic!("应报 Capability(UndeclaredCapability{{environment_deploy}})，实际 {other:?}")
        }
    }
}

/// P-5：**缺了声明的能力 ⇒ `MissingCapability`**（两向里的另一向）。
///
/// 登记项声明 `Payment(Charge) + Environment(Deploy)`，调用只声明 `--effect charge:c1`
/// ⇒ 声明表里那枚 `Environment(Deploy)` 出示集里没有。报的是**声明表里第一枚缺的**。
///
/// `effect_class` 取 `Some(Charge)`：`save_tool` 的登记期不变量要求
/// `required_capabilities` 含 `for_effect(Charge)`＝`Payment(Charge)`，本条满足它
/// （多声明一枚 `Environment(Deploy)` 不违反该不变量：它是 `Some(t) ⇒ …` 的蕴含，不是双条件）。
#[test]
fn a_required_capability_that_is_missing_is_rejected() {
    let fixture = Fixture::with_registrations(&[(
        "t1",
        vec![
            CapabilityKind::for_effect(EffectType::Charge),
            CapabilityKind::for_effect(EffectType::Deploy),
        ],
        Some(EffectType::Charge),
    )]);
    fixture.allow_everything();
    let adapter = Arc::new(RecordingTool::new(Outcome::Echo));
    let registry = registry_serving(&["t1"], Arc::clone(&adapter));

    let outcome = fixture.run(&registry, &allowed_charge_call(&fixture));

    assert_eq!(adapter.calls(), 0, "被拒的调用不许走到那一跳");
    assert_eq!(
        fixture.effect_rows(),
        0,
        "拒在第 5 步之前：效应行一条都不许落下"
    );
    assert_eq!(fixture.audit_rows(), 0, "零效应行 ⇒ 零审计行");
    assert_eq!(
        fixture.capability_grants_rows(),
        0,
        "`MissingCapability` 不得留下授权审计"
    );

    match outcome {
        Err(TaskError::Capability(CapabilityError::MissingCapability { kind })) => {
            assert_eq!(
                kind,
                CapabilityKind::for_effect(EffectType::Deploy),
                "报的应是缺的那一枚（deploy），不是已出示的那一枚（charge）"
            );
        }
        other => panic!("应报 Capability(MissingCapability{{environment_deploy}})，实际 {other:?}"),
    }
}

/// P-8：**登记项本身读不出来 ⇒ 原样带出 `Persist`**，不吞成 `UnknownTool`、也不吞成
/// 「不需要任何能力」。
///
/// 照 `crates/continuum-capability/tests/authorize.rs` 的 `insert_bad_capabilities` 绕过
/// `save_tool` 直接写一行、把 `required_capabilities` 列写坏。吞掉它的后果正是本用例要拦的：
/// 一次**没查过声明表**的授权仍然返回 `Ok`。
///
/// 调用带一条 `--effect`（同 P-3 的理由：循环为空时「零效应行」恒成立、没有判别力）。
#[test]
fn a_corrupt_registration_is_reported_as_a_persist_failure() {
    let fixture = Fixture::with_registrations(&[("t1", Vec::new(), None)]);
    insert_bad_capabilities(&fixture.db_path, "broken", r#"["git_push","not_a_kind"]"#);
    fixture.allow_everything();
    let adapter = Arc::new(RecordingTool::new(Outcome::Echo));
    let registry = registry_serving(&["broken"], Arc::clone(&adapter));

    let mut args = allowed_charge_call(&fixture);
    args.tool = ToolId::new("broken");

    let outcome = fixture.run(&registry, &args);

    assert_eq!(adapter.calls(), 0, "被拒的调用不许走到那一跳");
    assert_eq!(
        fixture.effect_rows(),
        0,
        "拒在第 5 步之前：效应行一条都不许落下"
    );
    assert_eq!(fixture.audit_rows(), 0, "零效应行 ⇒ 零审计行");
    assert_eq!(
        fixture.capability_grants_rows(),
        0,
        "读登记项失败不得留下授权审计"
    );

    match outcome {
        Err(TaskError::Capability(CapabilityError::Persist(PersistError::Database(m)))) => {
            assert!(m.contains("not_a_kind"), "错误信息应指出表外取值，实际 {m}");
        }
        other => panic!("应报 Capability(Persist(Database(_)))，实际 {other:?}"),
    }
}

/// **设计 §5.3 措辞订正之后的形状**：不在 `PolicyContext.effect_type` 事实集合里的那六个
/// kind，**在出示集为空时一律得到 `MissingCapability`**。
///
/// # 「出示集为空时」这个限定词是修复轮 1 加的（原句过大，来历留此）
///
/// 本条原先写的是「**本路径一律得到 `MissingCapability`**」。**那句过大**：`authorize` 的次序是
/// **先查出示集、再查声明集**（`crates/continuum-capability/src/registry.rs` 的第一遍与第二遍；
/// 照片是 `crates/continuum-capability/tests/authorize.rs` 的
/// `the_presented_set_is_checked_before_the_declared_set`），故一个**还出示了工具未声明的能力**
/// 的调用方拿到的是 [`CapabilityError::UndeclaredCapability`]，不是 `MissingCapability`
/// ——P-4 就是这条机制的照片。**设计那侧已按同一判据订正**（`8fc601e`，§5.3 现在把这两条
/// 分列成两行），本条只是当时没跟上。
///
/// 六条登记项各声明其中一枚（`required_capabilities` 取单枚），**不声明任何 `--effect`**
/// （出示集为空），故每条都应报 [`CapabilityError::MissingCapability`]，且 kind 与该条自己
/// 声明的那一枚**逐项相同**。
///
/// # 为什么六项**逐项**跑、不抽代表
///
/// 六个 `CapabilityKind` 是**不同的值**，而这条路径上有一对**各十二臂的手写表**：
/// `CapabilityKind::as_str`（`crates/continuum-capability/src/capability.rs`，
/// 经 `save_tool` 的 `encode_capabilities` 写库）与 [`CapabilityKind::parse`]（同文件，
/// 经 `load_tool` 的 `decode_capabilities` 读回）。六条登记项**各把这对表的两侧走了一遍**，
/// 故抽一个代表会让另外五臂的漂移无人过问。
///
/// **原句在这里点错了分支，来历留此**：原先写的是「那两处手写分支（`CapabilityKind::for_effect`
/// 的正向与 `effect` 的反向）能各自漂移」。**两个都指错了**——`authorize` **不调** `for_effect`
/// （它只用 `declared.contains(&capability.kind())` 与相等比较），而 `CapabilityKind::effect()`
/// 那张反向表**在本路径上没有调用点**：它在 `crates/*/src` 里的唯一调用者是连接器的
/// `crates/continuum-connector/src/entry.rs`（`bound.effect()`，属 B 那条路径）。
/// **枚举这件事本身做得对（六枚确实各有照片），错的只是依据。**
///
/// # 六条的 `effect_class` **必须**取 `None`
///
/// 这六个 kind **不是任何 `EffectType` 的 `for_effect` 像**（那张对照表是单射、不是满射：
/// `capability.rs` 的 `effect()` 对它们逐个返回 `None`）⇒ 无论 `Some(t)` 取哪个 `t`，
/// `required_capabilities` 都不含 `for_effect(t)` ⇒ `save_tool` 的登记期不变量在**登记期**
/// 就把这六行拒掉，本条会红在夹具的 `.unwrap()` 上而不是本路径的 `MissingCapability` 上。
///
/// # 「调不动」这条也有照片
///
/// 夹具适配器服务全部六个 id，断言**总调用次数为 0**——§5.3 的后果不只是「报哪种错」，
/// 还有「这些工具今天一条都跑不起来」。这条断言排在前：它一旦不成立（出示集被换成
/// 「按登记项的声明逐枚铸」那种实现），panic 会挡住后面六条逐项断言。
#[test]
fn every_kind_without_a_policy_fact_is_missing_capability() {
    let rows: [(&str, CapabilityKind); 6] = [
        ("t-fs-read", CapabilityKind::Filesystem(FsAction::Read)),
        ("t-fs-write", CapabilityKind::Filesystem(FsAction::Write)),
        ("t-git-read", CapabilityKind::Git(GitAction::Read)),
        (
            "t-git-worktree-write",
            CapabilityKind::Git(GitAction::WorktreeWrite),
        ),
        (
            "t-git-commit-local",
            CapabilityKind::Git(GitAction::CommitLocal),
        ),
        (
            "t-github-create-pr",
            CapabilityKind::Github(GithubAction::CreatePr),
        ),
    ];
    let fixture = Fixture::with_registrations(
        &rows
            .iter()
            .map(|(id, kind)| (*id, vec![*kind], None))
            .collect::<Vec<_>>(),
    );
    let ids: Vec<&str> = rows.iter().map(|(id, _)| *id).collect();
    let adapter = Arc::new(RecordingTool::new(Outcome::Echo));
    let registry = registry_serving(&ids, Arc::clone(&adapter));

    let mut outcomes = Vec::with_capacity(rows.len());
    for (id, _kind) in &rows {
        let mut args = fixture.args();
        args.tool = ToolId::new(*id);
        assert!(
            args.effects.is_empty() && args.intent.is_none(),
            "本条不声明任何 --effect：出示集为空正是「声明不出这种能力」的形态"
        );
        outcomes.push(fixture.run(&registry, &args));
    }

    assert_eq!(
        adapter.calls(),
        0,
        "六条登记项一条都调不动：它们的 kind 铸不出来，出示集恒为空"
    );

    for ((id, kind), outcome) in rows.iter().zip(outcomes) {
        match outcome {
            Err(TaskError::Capability(CapabilityError::MissingCapability { kind: missing })) => {
                assert_eq!(
                    missing,
                    *kind,
                    "工具 {id} 报的必须是它自己声明的那一枚 {}",
                    kind.as_str()
                );
            }
            other => panic!(
                "工具 {id} 应报 Capability(MissingCapability{{{}}})，实际 {other:?}",
                kind.as_str()
            ),
        }
    }
}
