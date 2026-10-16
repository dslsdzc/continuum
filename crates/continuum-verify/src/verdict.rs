//! §7.5 判定结果的形状：`VerifierVerdict` / `VerdictAggregate` / `BlindVerdict`
//! 与 `clamp_by_readability`（设计 §7.5）。

use crate::evidence::Claim;

/// 一轮判定的一枚结论（设计 §7.5）。
///
/// **没有 `blocking: bool` 参数**（在 [`VerifierVerdict::Fail`] 的文档里）：
/// 规范未给「非阻断失败」的判据，故本轮所有失败都是阻断的。
///
/// **本型不进落库列**（Task 9 落的是聚合 [`VerdictAggregate`] 与独立性判定）。
/// **不设 `parse`**——没有读回路径，也不造一个。
///
/// 〔订正（2026-10-16，Task 9）〕本段初版写「**不设** `as_str` / `parse`——没有第二处
/// 编码的需要」，其中 `as_str` 半句在 Task 9 落地时**已假**：`VerificationFailed`
/// 事件的 payload 要写**逐项状态**（§10.3），那需要每一条判定的判别串，编码即
/// [`VerifierVerdict::as_str`]（与类型同址）。旧说法记此，免得下一轮照它得出错的结论。〕
#[derive(Debug, Clone, PartialEq)]
pub enum VerifierVerdict {
    Pass,
    /// §266 的 `no_blocking_verification_failure` 为假。
    /// **没有 `blocking: bool` 参数**：规范未给「非阻断失败」的判据，故本轮所有失败都是阻断的。
    Fail { reason: Claim },
    /// §123：正式表达未知，不逼模型在证据不足时给结论。
    Unknown { reason: Claim },
}

impl VerifierVerdict {
    /// 三臂的判别串（小写、多词以 `_` 连接）。
    ///
    /// **本函数是该编码唯一的产生点**。match 穷尽且无通配臂：加臂时本函数编译不过，
    /// 编码不会漏分支。
    ///
    /// **不进任何 SQL 列**（落库的是聚合，见类型文档）：它是 `VerificationFailed`
    /// 事件 payload 里「逐项状态」的编码（§10.3，由 `record_verification_round` 写）。
    /// **不设 `parse`**：没有读回路径。三枚串名与
    /// [`CheckVerdict::as_str`](crate::test_validity::CheckVerdict::as_str) 逐型相同，
    /// 是**各自独立**的编码（两枚类型不同，不共享实现，改一处不牵另一处）。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail { .. } => "fail",
            Self::Unknown { .. } => "unknown",
        }
    }
}

impl VerifierVerdict {
    /// 三臂，逐臂照录（设计 §7.5）。**本常量带数目字面量**，与
    /// [`VerifierLevel::ALL`](crate::verifier_level::VerifierLevel::ALL) 同形。
    ///
    /// 两处载荷臂的载荷取 `Claim` 的空串（crate 内的编译期常量）：`Claim::from_str`
    /// 不是 `const fn`，常量数组需要一个编译期可得的载荷；名单是**臂清单**，
    /// 载荷的值不参与「有哪些臂」这件事——逐臂点名由 `tests/verifier.rs` 的
    /// `the_four_enums_each_name_their_arms` 把关。
    pub const ALL: [VerifierVerdict; 3] = [
        VerifierVerdict::Pass,
        VerifierVerdict::Fail { reason: Claim::EMPTY },
        VerifierVerdict::Unknown { reason: Claim::EMPTY },
    ];
}

/// 一轮里全部 verifier 判定的聚合。**五臂的判据互斥且有序**，声明序即优先序。
///
/// 本类型落 `verification_round.aggregate` 列（Task 9），故编码在
/// [`VerdictAggregate::as_str`] / [`VerdictAggregate::parse`] 上（小写、多词以 `_` 连接）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerdictAggregate {
    /// §121：同时存在 `Pass` 与 `Fail`。**不得多数投票**。优先级最高。
    AdjudicationRequired,
    /// 至少一条 `Fail` **且无 `Pass`**。
    Failed,
    /// 无 `Fail`、无 `Pass`（含空集，见 [`VerdictAggregate::from_verdicts`]）。
    Unknown,
    /// 无 `Fail`、有 `Pass` 且有 `Unknown`。
    PassedWithUnknown,
    /// 全部 `Pass`。
    Passed,
}

impl VerdictAggregate {
    /// 五臂，逐臂照录（设计 §7.5）。**本常量带数目字面量**，与
    /// [`VerifierLevel::ALL`](crate::verifier_level::VerifierLevel::ALL) 同形。
    ///
    /// 顺序照设计 §7.5 的声明序（**声明序即优先序**）；完整性与数目由
    /// `tests/verifier.rs` 的 `the_four_enums_each_name_their_arms` 把关。
    pub const ALL: [VerdictAggregate; 5] = [
        VerdictAggregate::AdjudicationRequired,
        VerdictAggregate::Failed,
        VerdictAggregate::Unknown,
        VerdictAggregate::PassedWithUnknown,
        VerdictAggregate::Passed,
    ];

    /// 五臂的字符串名：小写、多词以 `_` 连接。
    ///
    /// **本函数是该编码唯一的产生点**。match 穷尽且无通配臂：加臂时本函数编译不过，
    /// 编码不会漏分支。
    ///
    /// 串名没有规范逐字出处（§7.5 只给臂名）：五枚串名是本块的命名，照 §4.2 的口径。
    /// `verification_round.aggregate` 列（Task 9）取用它。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AdjudicationRequired => "adjudication_required",
            Self::Failed => "failed",
            Self::Unknown => "unknown",
            Self::PassedWithUnknown => "passed_with_unknown",
            Self::Passed => "passed",
        }
    }

    /// [`VerdictAggregate::as_str`] 的**严格逆**：对 [`VerdictAggregate::ALL`] 里的每臂
    /// 都有 `VerdictAggregate::parse(t.as_str()) == Some(t)`，表外字符串一律 `None`。
    ///
    /// `None` 而不是默认臂：从值里读出表外串说明写着本版本不认得的取值，取默认会把
    /// 一份别的东西当成本臂处理（理由同
    /// [`VerifierLevel::parse`](crate::verifier_level::VerifierLevel::parse)）。
    /// 解码侧没有穷尽 match 的保护（来源是 `&str` 而非枚举），故由 `tests/verifier.rs` 的
    /// `the_four_enums_each_name_their_arms` 遍历 [`VerdictAggregate::ALL`] 兜住。
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "adjudication_required" => Self::AdjudicationRequired,
            "failed" => Self::Failed,
            "unknown" => Self::Unknown,
            "passed_with_unknown" => Self::PassedWithUnknown,
            "passed" => Self::Passed,
            _ => return None,
        })
    }

    /// §7.5 的五臂，**声明序即优先序**（`AdjudicationRequired` 优先于 `Failed`——
    /// §121 对这一形态的处理是「进入 Adjudication，不得多数投票」，取 `Failed` 会把一个
    /// 需要裁决的形态压成一次普通失败）。
    ///
    /// **空集落在 `Unknown` 臂**（本计划的读法，设计未写）：空与「全是 `Unknown`」都落在
    /// 「无 `Fail`、无 `Pass`」条下，而**不许**落在 `Passed`（fail-closed 的一侧）。
    pub fn from_verdicts(v: &[VerifierVerdict]) -> VerdictAggregate {
        let has_pass = v
            .iter()
            .any(|verdict| matches!(verdict, VerifierVerdict::Pass));
        let has_fail = v
            .iter()
            .any(|verdict| matches!(verdict, VerifierVerdict::Fail { .. }));
        if has_pass && has_fail {
            VerdictAggregate::AdjudicationRequired
        } else if has_fail {
            VerdictAggregate::Failed
        } else if !has_pass {
            // 无 Fail、无 Pass（含空集）：其余只剩 Unknown 载荷这一种。
            VerdictAggregate::Unknown
        } else if v
            .iter()
            .any(|verdict| matches!(verdict, VerifierVerdict::Unknown { .. }))
        {
            VerdictAggregate::PassedWithUnknown
        } else {
            VerdictAggregate::Passed
        }
    }
}

/// §29 的 blind verification 的结论。
///
/// **臂与 [`VerifierVerdict`] 逐臂同形**，单独命名的理由只有一条：
/// [`PostBlindVerifierInput`](crate::input::PostBlindVerifierInput)（§7.4）要的是
/// **盲判这一轮**的结论，而不是任何一轮 verifier 的结论——两者在类型上分开，
/// 「先盲判、后比较」的构造顺序才可检查（§7.4 第 2 条）。
///
/// **臂从哪来**：规范**未给** `BlindVerdict` 的臂（§29 只规定顺序，未规定结论的形态）。
/// 本设计取「与 `VerifierVerdict` 同形的三臂」，转换入口是 [`BlindVerdict::from`]——
/// 盲判即 Verifier 在看不到 Worker 完成声明与推理时的判定，本设计不为它新增判定语义。
/// **据实**：三个臂与 `VerifierVerdict` 同形，枚举字面量本身也可直接构造（Rust 没有
/// 「唯一构造点」的强制手段）；上句钉的是「盲判结论由 verifier 判定转换而来」这条入口，
/// 而不是一条绕不过的守卫。
#[derive(Debug, Clone, PartialEq)]
pub enum BlindVerdict {
    Pass,
    Fail { reason: Claim },
    Unknown { reason: Claim },
}

impl BlindVerdict {
    /// 三臂，与 [`VerifierVerdict::ALL`] 逐臂同形（载荷同样取空串的编译期常量）。
    /// **本常量带数目字面量**，完整性与逐臂点名由 `tests/verifier.rs` 的
    /// `the_four_enums_each_name_their_arms` 把关。
    pub const ALL: [BlindVerdict; 3] = [
        BlindVerdict::Pass,
        BlindVerdict::Fail { reason: Claim::EMPTY },
        BlindVerdict::Unknown { reason: Claim::EMPTY },
    ];
}

/// 逐臂同形（设计 §7.5）：载荷原样搬运，不新增判定语义。
/// **无通配臂**：`VerifierVerdict` 加臂时本函数编译不过。
impl From<VerifierVerdict> for BlindVerdict {
    fn from(verdict: VerifierVerdict) -> Self {
        match verdict {
            VerifierVerdict::Pass => BlindVerdict::Pass,
            VerifierVerdict::Fail { reason } => BlindVerdict::Fail { reason },
            VerifierVerdict::Unknown { reason } => BlindVerdict::Unknown { reason },
        }
    }
}

/// §7.5：**无物可读 ⇒ `Unknown`，不得 `Pass`**。
///
/// 只动「`Pass` 且无物可读」这一支：`Fail` 与 `Unknown` 原样保留——本条规定的是
/// 「没有制品时不许给 `Pass`」，不是「没有制品时一律 `Unknown`」。
///
/// 生成的 `Unknown` 的 `reason` 取一条固定说明文本：本函数没有可搬运的上下文文本
/// （`raw` 是 `Pass`，没有理由可带），而「为什么到这里」由调用方与
/// [`InitialVerifierInput::has_readable_artifact`](crate::input::InitialVerifierInput::has_readable_artifact)
/// 的输入面共同承载。
pub fn clamp_by_readability(readable: bool, raw: VerifierVerdict) -> VerifierVerdict {
    match raw {
        VerifierVerdict::Pass if !readable => VerifierVerdict::Unknown {
            reason: Claim::from_str("无可读制品，不得判 Pass（§7.5）"),
        },
        other => other,
    }
}
