//! `tool` 子命令的端到端用例（F 设计 §2、§3、§9 的端到端那一半）。
//!
//! 全部经 `env!("CARGO_BIN_EXE_continuum-runtime")` 驱动**真实二进制**（与 `task_cli.rs`
//! 同法）：判据的对象是「调用方敲下这条命令行之后，驱动实际做了什么」——退出码、stderr 的
//! 出错行、以及库里留下的行。import 本 crate 的函数看不到这些。
//!
//! # 与同族用例的分工（据实划清，免得三处被读成三份证据）
//!
//! - **解析那一半**（每个 `CliError` 变体、信息里点名哪一个 token）在 `tests/cli.rs`：
//!   它直接调 `cli::parse`、看解析结果本身；
//! - **库级那一半**（这条路径调没调注册表、写没写哪一行、报哪一个变体）在
//!   `tests/tool_call.rs`：它经 `continuum_runtime::tool_call::run_tool_call` 注入夹具适配器；
//! - 本文件只钉**这半边**：那些结论在**真二进制**上同样成立。故 `audit_log` 的**行数与
//!   构成**（P-15）、`authorization` 字段（§7.2）、幂等键冲突的整条拒绝（P-2）这些
//!   **库级版本在 Task 5 / Task 6**，本文件不重复造第二份。
//!
//! # 一处刻意**不**断言的东西
//!
//! 「注册表里没有这个 id」那一跳的失败**文案**不在这里断言：那是 C
//! （`continuum-provider`）的 `ToolCallError::Unregistered` 的 `Display`，**F 没有冻结它**
//! （这一点记在 F 设计 §9 的「遗留」）。故那条用例只断言**退出码**与**库里的行**。
//!
//! # 「stderr 含某个选项名」为什么不算一条判据
//!
//! `main` 在解析失败时先打一行 `参数错误：<CliError>`、紧接着打整份 `cli::USAGE`，而
//! `USAGE` 里**逐字列出了** `--base` / `--sandbox` / `--input` / `--intent` / `--effect`
//! 这些选项名（`crates/continuum-runtime/src/cli.rs` 的 `USAGE`）。故「stderr 含 `--input`」
//! 这句对「解析器压根没校验 `--input`」的实现**照样成立**——`USAGE` 替它把话说圆了。
//! 本文件的判据取 **stderr 的第一行**（就是那行错误，见 [`first_line_of`]），并另加一条
//! **只有真的没开库才成立**的断言：`--db` 指向的文件不存在。

use continuum_capability::{CapabilityKind, Tool, ToolProfile, Trust, save_tool};
use continuum_core::tool::ToolId;
use continuum_effect::EffectType;
use continuum_persist::{Db, Migration, Value};
use continuum_policy::{Condition, Decision, Level, Policy, Scope, save_policy};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_continuum-runtime");

// ── 夹具 ────────────────────────────────────────────────────────────────

/// 跑一次 `tool`，返回输出。`after_db` 是 `--db <路径>` 之后的全部参数（含 `--tool`）。
fn run_tool(db: &Path, after_db: &[&str]) -> Output {
    Command::new(BIN)
        .arg("tool")
        .arg("--db")
        .arg(db)
        .args(after_db)
        .output()
        .expect("continuum-runtime 无法执行")
}

fn stderr_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// stderr 的**第一行**，即 `main` 打的那行出错信息（`参数错误：<CliError>`）。
///
/// 只取第一行是刻意的：`USAGE` 排在它后面，而 `USAGE` 里逐字含有本文件要断言的几个选项名
/// （见模块文档）。拿整份 stderr 去断言，等于让 `USAGE` 替被测实现背书。
fn first_line_of(text: &str) -> &str {
    text.lines().next().unwrap_or_default()
}

/// 本文件要用的迁移集合：`tool` / `policy` / `effect` 三张表 + 内建（`audit_log`）。
///
/// 不复用 `main.rs` 那份清单（`runtime_migrations` 在 bin 里、`pub(crate)`）：驱动自己开库时
/// 会把它那份清单里其余几条补上（`open_db` 每次都 `migrate`），故这里只需要写下**用例要写
/// 进去的那几张表**。这份写法与 `tests/tool_call.rs` 的 `migrations()` 同形。
fn migrations() -> Vec<Migration> {
    let mut migrations = continuum_persist::builtin_migrations();
    migrations.extend(continuum_capability::p3_capability_migrations());
    migrations.extend(continuum_policy::p2_policy_migrations());
    migrations.extend(continuum_effect::p2_effect_migrations());
    migrations
}

/// 一次用例的固定装置：一个建好的库，`tool` 表里有一条登记项、`policy` 表里有一条恒真的
/// `Allow`（强制点 (2) 要能铸出能力，命令才走得到强制点 (1)）。
struct Fixture {
    /// 保活用：临时目录一析构，库文件就没了。字段从不读取，故用下划线名。
    _dir: tempfile::TempDir,
    db: PathBuf,
}

impl Fixture {
    /// 登记工具 `t1`：声明表是 `required`，`effect_class` 由调用方给。
    ///
    /// **用 `save_tool` / `save_policy` 写，不手写 SQL 字面量**：驱动读的是那两个 crate 的
    /// 编码（`required_capabilities`、`effect_class`、`level`、`decision` 等列），手抄一份到
    /// 测试里就是给同一件事立第二个来源——编码改了两者一起漂移、而驱动读不回。
    /// （读回来那几处反过来要手写 SQL / 手写字面量，理由见各访问器。）
    ///
    /// `required` 必须**恰等于**调用将出示的那批能力：`authorize` 的两遍比对是双向的
    /// （多声明一枚 → `MissingCapability`，少声明一枚 → `UndeclaredCapability`，
    /// 照片在 `crates/continuum-capability/tests/authorize.rs`）。
    fn with_registration(required: Vec<CapabilityKind>, effect_class: Option<EffectType>) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("continuum.db");
        let db = Db::open_with(&db_path, migrations()).expect("打开数据库失败");
        db.migrate().expect("应用迁移失败");
        let tx = db.begin().unwrap();
        let tool = Tool::new(
            ToolId::new("t1"),
            "1.0".to_owned(),
            json!({"type": "object"}),
            json!({"type": "object"}),
            required,
            effect_class,
            true,
        );
        save_tool(&tx, &ToolProfile::new(tool, None, None, Trust)).unwrap();
        let allow = Policy {
            // 第 5 级（`RuntimeDefault`）：本仓在「第 5 级」这个名字下的既有写法见
            // `tests/task_cli.rs` 的同名夹具。恒真是刻意的——本文件钉的是接线，
            // 条件怎么求值属 `continuum-policy` 的用例。
            level: Level::RuntimeDefault,
            condition: Condition::parse(&json!({"all": []})).expect("空合取是合法条件"),
            decision: Decision::Allow,
            scope: Scope::User,
        };
        save_policy(&tx, "allow-all", &allow).unwrap();
        tx.commit().unwrap();
        Self {
            _dir: dir,
            db: db_path,
        }
    }
}

// ── 读库 ────────────────────────────────────────────────────────────────

/// 打开一个**已经由驱动建好**的库来读。
///
/// 不经 `Db::open_with(.., migrations())` 再 `migrate`：驱动已经把它那份完整清单应用过了，
/// 这里再迁移一次没有意义。`tests/task_cli.rs` 的读回处同法。
fn read_open(db: &Path) -> Db {
    Db::open(db).expect("打开数据库失败")
}

/// `audit_log` 里 `kind` 列恰为 `"capability grants"` 的行数。
///
/// 判据是**手写的字面量**，不是把 `AuditKind::CapabilityGrants` 转一圈——那样只证明枚举
/// 等于自己、钉不住落库编码（`tests/tool_call.rs` 的同名判据用同一条）。
fn capability_grants_rows(db: &Path) -> i64 {
    let handle = read_open(db);
    let tx = handle.begin().unwrap();
    let rows = tx
        .query(
            "SELECT COUNT(*) FROM audit_log WHERE kind = 'capability grants'",
            &[],
        )
        .expect("读 audit_log 失败");
    let count = match &rows[0][0] {
        Value::Int(i) => *i,
        other => panic!("COUNT(*) 列应为整数，实际 {other:?}"),
    };
    tx.commit().unwrap();
    count
}

/// `audit_log` 里 `kind` 恰为 `capability grants` 的各行 `payload`，按 `seq` 升序，已解析成
/// JSON（P-18 要读 `capabilities[].scope`）。
///
/// 该列存的是 JSON 对象的文本（`Tx::append_audit` 的编码），故这里解析回来而不是断言原文
/// ——原文的键序由 `serde_json` 的 map 决定，与 payload 的内容无关。
fn capability_grants_payloads(db: &Path) -> Vec<serde_json::Value> {
    let handle = read_open(db);
    let tx = handle.begin().unwrap();
    let rows = tx
        .query(
            "SELECT payload FROM audit_log WHERE kind = 'capability grants' ORDER BY seq",
            &[],
        )
        .expect("读 audit_log 失败");
    let out = rows
        .iter()
        .map(|row| match &row[0] {
            Value::Text(text) => serde_json::from_str(text).expect("payload 列是 JSON 对象的文本"),
            other => panic!("payload 列应为文本，实际 {other:?}"),
        })
        .collect();
    tx.commit().unwrap();
    out
}

/// `effect` 表里各行的 `idempotency_key`，按字典序。
///
/// 读**列**而不是经 `find_by_idempotency_key`：本文件要看的正是落库的那个键形
/// （`<意图字节长>:<意图>:<类型>:<目标>`），经类型化的读回会把它的来源盖住。
fn effect_keys(db: &Path) -> Vec<String> {
    let handle = read_open(db);
    let tx = handle.begin().unwrap();
    let rows = tx
        .query(
            "SELECT idempotency_key FROM effect ORDER BY idempotency_key",
            &[],
        )
        .expect("读 effect 表失败");
    let out = rows
        .iter()
        .map(|row| match &row[0] {
            Value::Text(text) => text.clone(),
            other => panic!("idempotency_key 列应为文本，实际 {other:?}"),
        })
        .collect();
    tx.commit().unwrap();
    out
}

// ── 用例 ────────────────────────────────────────────────────────────────

/// P-12 的端到端那一半：不可解析的 `--input` 在**解析期**即被拒，库根本没被创建。
///
/// 判据取 stderr 的**第一行**（见模块文档）：整份 stderr 里的 `USAGE` 也含 `--input` 那一串，
/// 拿它当判据对「解析器不校验 `--input`」的实现照样成立。
///
/// **「库未被创建」这一条才是承重的**：解析排在装配之前（`main` 里 `cli::parse` 在
/// `secrets::assemble` 与分派之前），故一次被拒的调用**一个文件都不该建出**。写成
/// 「退出码非零」是不够的：不校验 `--input` 的实现随后会走到 `tool_cmd::run`，以「工具未登记」
/// 之类的**另一条**错误非零退出，下面那行错误断言仍能被 `USAGE` 顶过去。
#[test]
fn an_unparsable_input_fails_at_parsing_time() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("t.db");

    let out = run_tool(&db, &["--tool", "t1", "--input", "{"]);
    assert!(
        !out.status.success(),
        "不可解析的 --input 必须让整条命令非零退出\n--- stderr ---\n{}",
        stderr_of(&out)
    );
    let stderr = stderr_of(&out);
    let first = first_line_of(&stderr);
    assert!(
        first.contains("--input") && first.contains("JSON"),
        "出错行应点名 --input 且说明它不是合法 JSON，实际：{first}"
    );
    assert!(
        !db.exists(),
        "被拒的调用不该碰数据库：解析在装配与分派之前，一个文件都不该建出（{}）",
        db.display()
    );
}

/// P-13：选项组**两向都拒**（F 设计 §2.2），两向各一条端到端用例。
///
/// 方向一：给了 `--effect` 不给 `--intent`（效应没有幂等键的来源）。
/// 方向二：给了 `--intent` 不给任何 `--effect`（`--intent` 不落任何地方、不可观察）。
///
/// # 两向的 stderr **逐字相同**，故两向的区分不在文案上
///
/// 两个方向报的是同一个变体、且 `option` 字段都取 `"--intent"`（方向一是**缺**它的那一个，
/// 方向二是**给错**了的那一个），`Display` 模板里本就同时含 `--intent` 与 `--effect`。
/// 故本条的两向靠**两份不同的输入**各自被拒来成立，不靠文案的不同——把这一点写出来，
/// 免得后来者以为漏了一条文案断言。两向各自另有一条**只有真的没开库才成立**的断言。
///
/// 两向用**两个不同的库路径**：同一个路径时，方向一若（错误地）建出了库，方向二的
/// 「库不存在」断言就会被方向一的副作用掩盖，两向的读数分不开。
#[test]
fn the_option_group_is_rejected_in_both_directions() {
    let dir = tempfile::tempdir().unwrap();

    // 方向一：有 --effect、无 --intent。
    let db = dir.path().join("effect-without-intent.db");
    let out = run_tool(&db, &["--tool", "t1", "--effect", "charge:x"]);
    assert!(
        !out.status.success(),
        "有 --effect 无 --intent 必须被拒\n--- stderr ---\n{}",
        stderr_of(&out)
    );
    let stderr = stderr_of(&out);
    let first = first_line_of(&stderr);
    assert!(
        first.contains("--intent"),
        "出错行应点名 --intent，实际：{first}"
    );
    assert!(
        first.contains("同进同出"),
        "必须报的是选项组那条规则（`OptionRequiresEffect`），不是「--intent 是未知选项」\
         之类的别的解析失败，实际：{first}"
    );
    assert!(!db.exists(), "被拒的调用不该碰数据库（{}）", db.display());

    // 方向二：有 --intent、无 --effect。
    let db = dir.path().join("intent-without-effect.db");
    let out = run_tool(&db, &["--tool", "t1", "--intent", "i1"]);
    assert!(
        !out.status.success(),
        "有 --intent 无 --effect 必须被拒\n--- stderr ---\n{}",
        stderr_of(&out)
    );
    let stderr = stderr_of(&out);
    let first = first_line_of(&stderr);
    assert!(
        first.contains("--intent"),
        "出错行应点名 --intent（它是被给错的那一个），实际：{first}"
    );
    assert!(
        first.contains("同进同出"),
        "必须报的是选项组那条规则（`OptionRequiresEffect`），实际：{first}"
    );
    assert!(!db.exists(), "被拒的调用不该碰数据库（{}）", db.display());
}

/// P-14：`task` 的四个选项在 `tool` 上都是**未知选项**，四条各一条（F 设计 §2.1）。
///
/// 四条**逐项过、不抽代表**：它们落到 `parse_tool` 的同一个兜底臂上，但四条选项名是四个
/// 各自可漂移的取值，而本用例断言的正是在出错行里点名了**哪一个**（把兜底臂的 `name`
/// 写成一个常量，本用例就有三条变红）。这与 `tests/cli.rs` 的 `tool_rejects_the_task_options`
/// 是同一条取向，只是那一条看解析结果、本一条看真二进制。
///
/// 四条各用一个**独立的库路径**，理由同 [`the_option_group_is_rejected_in_both_directions`]。
#[test]
fn the_task_only_options_are_unknown_here() {
    let dir = tempfile::tempdir().unwrap();
    let cases: [(&[&str], &str); 4] = [
        (&["--base", "/b"], "--base"),
        (&["--exec", "true"], "--exec"),
        (&["--apply"], "--apply"),
        (&["--sandbox", "landlock"], "--sandbox"),
    ];

    for (extra, name) in cases {
        let db = dir
            .path()
            .join(format!("{}.db", name.trim_start_matches("--")));
        let mut args = vec!["--tool", "t1"];
        args.extend_from_slice(extra);
        let out = run_tool(&db, &args);

        assert!(
            !out.status.success(),
            "{name} 不得被 tool 接受\n--- stderr ---\n{}",
            stderr_of(&out)
        );
        let stderr = stderr_of(&out);
        let first = first_line_of(&stderr);
        assert!(
            first.contains("未知选项"),
            "{name} 应报「未知选项」而不是别的解析失败，实际：{first}"
        );
        assert!(first.contains(name), "出错行应点名 {name}，实际：{first}");
        assert!(
            !db.exists(),
            "被拒的调用不该碰数据库：解析期即拒（{}）",
            db.display()
        );
    }
}

/// **强制点 (1) 在正常路径上被执行**（设计 §12 那条判据的落点），且这一跳之后库里留下了
/// 步骤 4 与步骤 5 的行——尽管调用最终在最后一跳失败。
///
/// # 这条为什么是「有生产调用方」的照片
///
/// 生产的装配点是空的（`tool_cmd.rs`：注册表里一个适配器都没有），故一次**登记过**的调用
/// 会走到步骤 4 的 `authorize`、在此**授权成功并写下审计**，最后在步骤 6 以「未登记」
/// 失败。设计 §12 的判据要求那个调用点「在正常路径上」（不是只有测试才走到）——本条就是
/// 它的端到端形态：真二进制、真库、没有夹具适配器。
///
/// # 三件事，各自的分界
///
/// - **`capability grants` 恰 1 条** ⇒ 强制点 (1) 真的跑了、且它的审计**随步骤 5 一起落了库**
///   （步骤 4 与步骤 5 用同一个事务，设计 §3.2）；「恰一条」而不是「至少一条」是因为
///   `authorize` 在一次调用里只被调一次。
/// - **`effect` 那些行** ⇒ 步骤 5 在那一跳**之前**提交过（设计 §3 的步骤序），故失败也要
///   留下各条效应行。行数与键**逐项**断言（不是只数条数）：条数对而两条互相串了目标，
///   只数条数的断言照样通过。
/// - **退出码非零** ⇒ 那一跳确实失败了。**不断言它的文案**：那是 C 的
///   `ToolCallError::Unregistered` 的 `Display`，F 没有冻结它（见模块文档）。
///
/// 各效应行的**终态**（`FAILED`）不在这里断言：那是库级用例
/// `tests/tool_call.rs::a_provider_failure_is_carried_through` 与 P-9 的判据，本条只钉
/// 「真二进制也走这条路」。
#[test]
fn a_registered_tool_is_still_unrouted_and_the_call_fails_at_the_last_hop() {
    let fixture = Fixture::with_registration(
        vec![
            CapabilityKind::for_effect(EffectType::Charge),
            CapabilityKind::for_effect(EffectType::Deploy),
        ],
        Some(EffectType::Charge),
    );

    let out = run_tool(
        &fixture.db,
        &[
            "--tool",
            "t1",
            "--effect",
            "charge:c1",
            "--effect",
            "deploy:d1",
            "--intent",
            "i1",
        ],
    );
    assert!(
        !out.status.success(),
        "装配点为空 ⇒ 那一跳必须失败\n--- stderr ---\n{}",
        stderr_of(&out)
    );

    // 步骤 4：强制点 (1) 授权成功并落了审计（与步骤 5 同一个事务，已提交）。
    assert_eq!(
        capability_grants_rows(&fixture.db),
        1,
        "登记过的工具应恰好过一次强制点 (1)，且那条授权审计已随步骤 5 提交"
    );
    // 步骤 5：声明的两条效应行都已落库。期望值**手写**（钉 `effect_key` 的键形：
    // <意图字节长>:<意图>:<类型>:<目标>），不由命令行或 `effect_key` 生成——生成出来的
    // 期望值会跟着被测实现一起漂移，钉不住格式。
    assert_eq!(
        effect_keys(&fixture.db),
        vec!["2:i1:charge:c1".to_owned(), "2:i1:deploy:d1".to_owned()],
        "两条声明的效应各留一行（行数等于声明的效应条数），且键形逐字如此"
    );
}

/// P-18 的端到端形态：各 `--effect` 的目标**原样**经能力的 scope 进了授权审计的 payload。
///
/// 库级版本在 Task 5（`tests/tool_call.rs::each_declared_scope_is_carried_into_the_audit_payload_verbatim`），
/// 本条只钉**真二进制也走这条路**：驱动把 `--effect` 的目标铸成能力的作用域，再经 `authorize`
/// 写进 `audit_log`。`authorize` 不判断作用域够不够（那是执行点与凭据签发的活，设计 §4.3），
/// 它把已获准的能力**原样**放进 payload。
///
/// # 两个目标必须彼此不同、且都不是效应类型的字面串
///
/// 判据是 multiset 相等，故两个目标相同的话，「两条都填成同一个值」的实现照样过。取
/// `c1` / `d1`（都不是 `charge` / `deploy`）另拦住「把 scope 换成 `effect_type` 的字面串」
/// 那种换法。**变异体**：把铸币时的作用域换成常量或换成 `spec.effect_type.as_str()`
/// ⇒ 本条红。
///
/// 本条的调用同样在最后一跳失败（注册表为空）：授权已发生在它之前，故 payload 读得回来。
#[test]
fn the_declared_scopes_reach_the_audit_payload() {
    let fixture = Fixture::with_registration(
        vec![
            CapabilityKind::for_effect(EffectType::Charge),
            CapabilityKind::for_effect(EffectType::Deploy),
        ],
        Some(EffectType::Charge),
    );

    let out = run_tool(
        &fixture.db,
        &[
            "--tool",
            "t1",
            "--effect",
            "charge:c1",
            "--effect",
            "deploy:d1",
            "--intent",
            "i1",
        ],
    );
    assert!(
        !out.status.success(),
        "装配点为空 ⇒ 那一跳必须失败（本条的判据在那之前的强制点 (1) 上）\n--- stderr ---\n{}",
        stderr_of(&out)
    );

    let payloads = capability_grants_payloads(&fixture.db);
    assert_eq!(payloads.len(), 1, "成功授权恰一条 `capability grants` 行");
    let mut scopes: Vec<&str> = payloads[0]["capabilities"]
        .as_array()
        .expect("payload 的 capabilities 是数组")
        .iter()
        .map(|capability| {
            capability["scope"]
                .as_str()
                .expect("每枚已获准能力的 scope 是字符串")
        })
        .collect();
    assert_eq!(
        scopes.len(),
        2,
        "两条 `--effect` 铸出两枚能力、各带自己的 scope"
    );
    scopes.sort();
    assert_eq!(
        scopes,
        ["c1", "d1"],
        "multiset 逐项等于各 `--effect` 的目标：把 scope 换成常量、或换成 effect_type \
         的字面串，本条即红"
    );
}
