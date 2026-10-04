//! `Capability` 的五要素、三条结构性保证与唯一签发点（设计 §2.1、§2.3、§2.4）。
//!
//! 不可表达性命题（`full_access`、字符串互转）在 `tests/type_level.rs` 的编译失败
//! 样例里；本文件管的是运行期的取值与失败路径。

use continuum_capability::{
    CapabilityError, CapabilityKind, EmailAction, EnvAction, FsAction, GitAction, GithubAction,
    Grant, Issuer, PaymentAction, RegistryAction, Verdict, mint,
};

/// 词汇表里的每一个 kind，各带上 §253 二字段形状要求它给出的两个串。
///
/// **逐项手写，不抽代表**：七个 resource 下的动作集是设计 §2.1 列的，手写分支能各自
/// 漂移，故每一项都要自己的照片。用例里的串是**手工写的字面量**（钉格式）；而
/// `kind → 串` 这条**路径**由生产代码里穷尽且无通配臂的 match 钉住（加动作时编译不过）。
///
/// **这条名单限制不住什么**：12 这个数目本身也是手写的，用例无法自证它等于
/// `CapabilityKind` 的取值个数（枚举变体数在 rustc 1.95 上只经手写名单可得）。
/// 能挡住的是「删一项、补一项」：数目断言看长度，去重断言看重复——两者合起来，
/// 漏掉一个 kind 必得靠重复另一个来凑满 12，而重复会被抓住。
const ALL_KINDS: [(CapabilityKind, &str, &str); 12] = [
    (
        CapabilityKind::Filesystem(FsAction::Read),
        "filesystem",
        "read",
    ),
    (
        CapabilityKind::Filesystem(FsAction::Write),
        "filesystem",
        "write",
    ),
    (CapabilityKind::Git(GitAction::Read), "git", "read"),
    (
        CapabilityKind::Git(GitAction::WorktreeWrite),
        "git",
        "worktree.write",
    ),
    (
        CapabilityKind::Git(GitAction::CommitLocal),
        "git",
        "commit.local",
    ),
    (CapabilityKind::Git(GitAction::Push), "git", "push"),
    (
        CapabilityKind::Git(GitAction::DeleteRemote),
        "git",
        "delete_remote",
    ),
    (
        CapabilityKind::Github(GithubAction::CreatePr),
        "github",
        "create_pr",
    ),
    (CapabilityKind::Email(EmailAction::Send), "email", "send"),
    (
        CapabilityKind::Registry(RegistryAction::Publish),
        "registry",
        "publish",
    ),
    (
        CapabilityKind::Payment(PaymentAction::Charge),
        "payment",
        "charge",
    ),
    (
        CapabilityKind::Environment(EnvAction::Deploy),
        "environment",
        "deploy",
    ),
];

/// 十二个 kind 的 `resource` / `action` 逐项钉字面串。
///
/// **多词的拼法不统一是刻意的，不是笔误**，来源是**规范自己不统一**：
/// `worktree.write` 与 `commit.local` 是 §88 给例的原样（点号，
/// `docs/spec/02-positioning.md:909-910`），`create_pr` 是 §253 给例的原样（下划线，
/// `docs/spec/05-normative.md:986`）。**动手「修」这两条之前回去读那两处原文**：
/// 一律改成下划线，`Display` 就再也产不出 §88 的字面串，而 `git.worktree_write`
/// 这个串在规范里不存在。同一段说明留在 `CapabilityKind::action` 的文档里（后来者
/// 更可能先读到那里），两处都留是为防「看着不顺眼就统一掉」。
#[test]
fn resource_and_action_strings_follow_the_spec_examples() {
    assert_eq!(
        ALL_KINDS.len(),
        12,
        "词汇表的 kind 数与设计 §2.1 列的七个 resource 下的动作集不符（2+5+1+1+1+1+1）"
    );

    // 「删一项、补一项」的守卫：12 项两两不同。少了这条，名单可以少一个 kind、
    // 把另一个 kind 写两遍来凑满长度，而逐项断言照过。
    for (i, (kind, _, _)) in ALL_KINDS.iter().enumerate() {
        for (other, _, _) in &ALL_KINDS[i + 1..] {
            assert_ne!(
                kind, other,
                "名单里有重复的 kind：{kind:?}，长度断言看不到它"
            );
        }
    }

    for (kind, resource, action) in ALL_KINDS {
        let cap = mint(
            kind,
            String::from("scope/one"),
            10,
            Grant::Policy(Verdict::Allow),
        )
        .expect("Allow 裁决应能签发，本用例只借它取串");
        assert_eq!(cap.resource(), resource, "{kind:?} 的 resource 串");
        assert_eq!(cap.action(), action, "{kind:?} 的 action 串");
    }
}

/// `Display` 是 §253 的形状 `resource.action:scope`，逐条照录规范给例。
///
/// 给例里的串由**手工字面量**给（钉格式），能力本身经 `mint` 铸出（不绕签发点）。
#[test]
fn display_renders_the_253_shape() {
    let cases: [(CapabilityKind, &str, &str); 5] = [
        (
            CapabilityKind::Filesystem(FsAction::Read),
            "/project",
            "filesystem.read:/project",
        ),
        (
            CapabilityKind::Git(GitAction::Push),
            "origin/main",
            "git.push:origin/main",
        ),
        (
            CapabilityKind::Github(GithubAction::CreatePr),
            "repo/X",
            "github.create_pr:repo/X",
        ),
        (
            CapabilityKind::Git(GitAction::WorktreeWrite),
            "/task-worktree",
            "git.worktree.write:/task-worktree",
        ),
        (
            CapabilityKind::Git(GitAction::CommitLocal),
            "/task-worktree",
            "git.commit.local:/task-worktree",
        ),
    ];

    for (kind, scope, expected) in cases {
        let cap = mint(kind, scope.to_string(), 10, Grant::Policy(Verdict::Allow)).unwrap();
        assert_eq!(cap.to_string(), expected, "{kind:?} 的 Display 形状");
    }
}

/// `expiry` 的**有效侧**：`now` 严格小于 expiry 时有效。
///
/// 与下面那条各占一侧——只写「已过期」那一侧的话，一个恒返回 `Err` 的实现会全绿。
#[test]
fn valid_before_the_expiry_instant() {
    let cap = mint(
        CapabilityKind::Git(GitAction::Push),
        String::from("origin/main"),
        1_000,
        Grant::Policy(Verdict::Allow),
    )
    .unwrap();

    assert_eq!(cap.is_valid_at(0), Ok(()), "远早于 expiry");
    assert_eq!(cap.is_valid_at(999), Ok(()), "expiry 前一毫秒");
}

/// `expiry` 的**失效侧**，含**恰好到期**那一格。
///
/// `expiry` 是失效时刻而非「最后有效的时刻」，故 `now == expiry` 即已失效。
/// 断言的是**具体哪一种** `Err`（连字段一起），不是「返回了 Err」。
#[test]
fn expired_at_and_after_the_expiry_instant() {
    let cap = mint(
        CapabilityKind::Git(GitAction::Push),
        String::from("origin/main"),
        1_000,
        Grant::Policy(Verdict::Allow),
    )
    .unwrap();

    assert_eq!(
        cap.is_valid_at(1_000),
        Err(CapabilityError::Expired {
            expiry: 1_000,
            now: 1_000
        }),
        "恰好到期即已失效"
    );
    assert_eq!(
        cap.is_valid_at(1_001),
        Err(CapabilityError::Expired {
            expiry: 1_000,
            now: 1_001
        }),
        "到期后一毫秒"
    );
    // 「不过期」没有哨兵值：极远的 `now` 照样按同一条判据失效，不存在被特判成
    // 「永久有效」的取值。
    assert_eq!(
        cap.is_valid_at(i64::MAX),
        Err(CapabilityError::Expired {
            expiry: 1_000,
            now: i64::MAX
        }),
        "极远的 now 不构成「不过期」"
    );
}

/// `scope` 为空即拒，非空即收：**两侧各一格**。
///
/// 判据与 P2b 的「效应目标为空在解析期即拒」（`continuum-runtime/src/cli.rs:151`）逐字
/// 相同。只写「空即拒」那一侧的话，一个恒返回 `Err` 的实现会全绿，故有效侧同条断言。
///
/// 断言的是**具体哪一种** `Err`，且有效侧连「`scope` 原样带过来」一起断——否则一个
/// 「反正不收空串」而把 scope 丢掉的实现也能过。
#[test]
fn mint_rejects_an_empty_scope() {
    let kind = CapabilityKind::Git(GitAction::Push);

    assert_eq!(
        mint(kind, String::new(), 7, Grant::Policy(Verdict::Allow)),
        Err(CapabilityError::EmptyScope),
        "空 scope 应被拒"
    );

    // 有效侧（承重：没有它，恒 Err 的实现照过）。
    let cap = mint(
        kind,
        String::from("origin/main"),
        7,
        Grant::Policy(Verdict::Allow),
    )
    .expect("非空 scope 应被收下");
    assert_eq!(cap.scope(), "origin/main", "scope 应原样带过来");
}

/// `mint` **不重判** `granted`：三种裁决值 × 显式确认给没给，**六种取值一律铸出**。
///
/// 判定（那六格）的唯一落点在驱动的 `mints`（`continuum-runtime/src/task_cmd.rs:649-654`），
/// 本 crate 只记录结果——理由见 `mint` 的文档：判两处即同一判断有两个产生点。
///
/// **本条是反过来的照片**：它把「本处不复核」这件事钉在六种取值上。日后若有人在此
/// 重加六格重判（例如让 `Deny` 拒绝），本条立刻变红——那时请回去读 `mint` 的文档，
/// 而不是删掉本条。六种取值逐个列出、不抽代表。
#[test]
fn mint_does_not_re_judge_the_grant() {
    let kind = CapabilityKind::Git(GitAction::Push);
    let scope = || String::from("origin/main");

    for granted in [
        Grant::Policy(Verdict::Allow),
        Grant::Policy(Verdict::RequireApproval),
        Grant::Policy(Verdict::Deny),
        Grant::PolicyWithExplicitApproval(Verdict::Allow),
        Grant::PolicyWithExplicitApproval(Verdict::RequireApproval),
        Grant::PolicyWithExplicitApproval(Verdict::Deny),
    ] {
        let cap = mint(kind, scope(), 7, granted)
            .unwrap_or_else(|e| panic!("本处不重判，{granted:?} 也该铸得出，却得到 {e:?}"));
        assert_eq!(cap.kind(), kind, "kind 应原样带过来");
        assert_eq!(cap.scope(), "origin/main", "scope 应原样带过来");
        assert_eq!(cap.expiry(), 7, "expiry 应原样带过来");
    }
}

/// 签发点不读时钟：`expiry` 早已过去也照样铸得出，且 `0` 不是「不过期」的哨兵。
///
/// 与「本类型不读时钟」是同一件事的两面：判有效期只发生在 [`is_valid_at`]，且那里的
/// 判据对任何 `expiry` 取值一视同仁——故「无不过期表示」在这里落到实处：铸一枚
/// `expiry = 0` 的能力得到的是一枚**在 Unix 元年即已失效**的能力，而不是一枚永久的。
#[test]
fn mint_does_not_read_the_clock() {
    let cap = mint(
        CapabilityKind::Git(GitAction::Push),
        String::from("origin/main"),
        0,
        Grant::Policy(Verdict::Allow),
    )
    .expect("本函数不读时钟，过期的 expiry 也该铸得出");

    assert_eq!(cap.expiry(), 0, "expiry 原样带过来");
    assert_eq!(
        cap.is_valid_at(1),
        Err(CapabilityError::Expired { expiry: 0, now: 1 }),
        "铸出即失效，`0` 不是「不过期」"
    );
}

/// 本阶段的**唯一**签发来源逐项沿用：六种 `Grant` 取值铸出的能力都记
/// [`Issuer::PolicyWithExplicitApproval`]。
///
/// 它同时是「[`Issuer::AuthorityHost`] 在本 crate 中不产出」的照片——本 crate 只有
/// `mint` 一个产生方，而它只记另一个变体。**这条断言限制不到下游**：`AuthorityHost`
/// 是公开枚举的变体，下游可以构造它；能构造却无人构造，正是「移交去向已标出、
/// 本阶段尚无该宿主」的据实形态。
///
/// 名字记的是**签发路径**（策略裁决 + 显式确认），不是「本次附带了一次显式确认」：
/// 未给确认的那三个取值也记它，否则本阶段无第二个来源可记。六种取值逐个断言，
/// 不抽代表。
#[test]
fn every_minted_capability_records_the_only_issuer_of_this_stage() {
    let kind = CapabilityKind::Environment(EnvAction::Deploy);
    let cases = [
        Grant::Policy(Verdict::Allow),
        Grant::Policy(Verdict::RequireApproval),
        Grant::Policy(Verdict::Deny),
        Grant::PolicyWithExplicitApproval(Verdict::Allow),
        Grant::PolicyWithExplicitApproval(Verdict::RequireApproval),
        Grant::PolicyWithExplicitApproval(Verdict::Deny),
    ];

    for granted in cases {
        let cap = mint(kind, String::from("prod"), 7, granted)
            .unwrap_or_else(|e| panic!("{granted:?} 应能签发，却得到 {e:?}"));
        assert_eq!(
            cap.issuer(),
            Issuer::PolicyWithExplicitApproval,
            "{granted:?} 那格记录的签发来源"
        );
    }
}
