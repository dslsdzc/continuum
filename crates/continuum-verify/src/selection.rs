//! §7.2 的候选面与选择规则、§7.3 的身份隔离判定（设计 §7.2、§7.3）。

use continuum_operator::BackendId;

use crate::error::VerifyError;
use crate::policy::VerificationPolicy;
use crate::profile::VerificationProfile;
use crate::verifier_level::VerifierLevel;

/// §7.2 的一个 verifier 候选。**候选以值传入**（装配方给），本块不查任何注册表。
///
/// 字段开 `pub`：这是值传递的读数口，没有「唯一产生点」之类的形状约束
/// （与 [`Evidence`](crate::Evidence) 的处置不同，同 `ContractRequirement`）。
#[derive(Debug, Clone, PartialEq)]
pub struct AvailableVerifier {
    pub level: VerifierLevel,
    /// 该候选的 backend（§245）。§263 的隔离判定与 §120 的「不同证据路径」都读它。
    pub backend: BackendId,
    /// 供 §263 / §120 判「独立性」的身份。**今天恒为 `None`**（P1 设计 §14）。
    pub model: Option<String>,
    pub provider: Option<String>,
}

/// §7.2 的 Worker 身份：受验节点最近一次尝试的 `ExecutionProfile` 的**三项**
/// （`crates/continuum-graph/src/execution.rs:21`、`:22`、`:24` 的 `model` / `provider` / `backend`；`:23` 是 `tool`）。
///
/// 三项都是 `Option`（`ExecutionProfile` 的形状）：今天 `model` / `provider` 恒为 `None`
/// （P1 设计 §14），`backend` 由装配方填。任一项未知时独立性判不出
/// （[`IndependenceVerdict::NotEstablished`]）；`required` 为真时那个候选按 fail-closed
/// 被丢弃（§7.3）。
#[derive(Debug, Clone, PartialEq)]
pub struct WorkerIdentity {
    pub model: Option<String>,
    pub provider: Option<String>,
    pub backend: Option<BackendId>,
}

/// §7.2 选择规则的结果。
///
/// # 字段集是本计划定的（设计只在 §7.3 点名了 `.independence`）
///
/// `level` 与 `backend` 是选择规则的结果本身（「级别内具体选哪个 backend」在设计
/// §14 第 4 条被点名），`independence` 是 §7.3 要求「在结果里标注」的那一枚。
/// **代价**：字段集是本计划定的；**收益**：`record_verification_round` 的
/// `selection(JSON)` 列（Task 9）有可存之物。
#[derive(Debug, Clone, PartialEq)]
pub struct VerifierSelection {
    pub level: VerifierLevel,
    pub backend: BackendId,
    pub independence: IndependenceVerdict,
}

/// §7.3 的身份隔离判定（三臂由设计 §7.5 给）。
///
/// **`NotEstablished` 与 `NotIndependent` 是两回事**：前者是「判不出」（身份任一项未知），
/// 后者是「判为不独立」——压缩它们会让「没判」被当成「判为假」，正是设计 §8.4 警告过的
/// 「让调用方去查一个不存在的失败」。本类型落 `verification_round.independence` 列
/// （Task 9），故编码在 [`IndependenceVerdict::as_str`] / [`IndependenceVerdict::parse`] 上。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndependenceVerdict {
    /// 三项（`backend`、`model`、`provider`）都判过且都不同（§7.3 的三值逻辑）。
    Independent,
    /// §7.3 判据一（同 `backend`）或判据二（`model` 与 `provider` 均已知且都相同）命中。
    NotIndependent,
    /// §7.3 判据三：有任一项不知 ⇒ 判不出。候选在 [`VerifierSelection`] 里被标注
    /// 「独立性未确立」，使这一点在审计里可见。
    NotEstablished,
}

impl IndependenceVerdict {
    /// 三臂，逐臂照录（设计 §7.3/§7.5）。**本常量带数目字面量**，与
    /// [`VerifierLevel::ALL`] 同形。
    ///
    /// 顺序照设计 §7.5 的声明序；完整性与数目由 `tests/verifier.rs` 的
    /// `the_four_enums_each_name_their_arms` 把关。
    pub const ALL: [IndependenceVerdict; 3] = [
        IndependenceVerdict::Independent,
        IndependenceVerdict::NotIndependent,
        IndependenceVerdict::NotEstablished,
    ];

    /// 三臂的字符串名：小写、多词以 `_` 连接。
    ///
    /// **本函数是该编码唯一的产生点**。match 穷尽且无通配臂：加臂时本函数编译不过，
    /// 编码不会漏分支。
    ///
    /// 串名没有规范逐字出处（§7.3/§7.5 只给臂名）：三枚串名是本块的命名，照 §4.2 的
    /// 口径。`verification_round.independence` 列（Task 9）取用它。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Independent => "independent",
            Self::NotIndependent => "not_independent",
            Self::NotEstablished => "not_established",
        }
    }

    /// [`IndependenceVerdict::as_str`] 的**严格逆**：对 [`IndependenceVerdict::ALL`]
    /// 里的每臂都有 `IndependenceVerdict::parse(t.as_str()) == Some(t)`，表外字符串
    /// 一律 `None`。
    ///
    /// `None` 而不是默认臂：从值里读出表外串说明写着本版本不认得的取值，取默认会把
    /// 一份别的东西当成本臂处理（理由同
    /// [`VerifierLevel::parse`](crate::verifier_level::VerifierLevel::parse)）。
    /// 解码侧没有穷尽 match 的保护（来源是 `&str` 而非枚举），故由 `tests/verifier.rs` 的
    /// `the_four_enums_each_name_their_arms` 遍历 [`IndependenceVerdict::ALL`] 兜住。
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "independent" => Self::Independent,
            "not_independent" => Self::NotIndependent,
            "not_established" => Self::NotEstablished,
            _ => return None,
        })
    }
}

/// §7.3 的身份隔离判据（**本设计定，规范未给比较规则**）。三值逻辑，判据逐条：
///
/// - 能证明「**同一**」——同 `backend`，或 `model` 与 `provider` 均已知且两项都相同
///   ⇒ [`IndependenceVerdict::NotIndependent`]（同一条执行路径不可能独立于自己）；
/// - 能证明「**不同**」——`backend` 已知且不同，且 `model` 与 `provider` 均已知且**都不同**
///   ⇒ [`IndependenceVerdict::Independent`]；
/// - **其余**（有任一项未知，或 `model` / `provider` **半同**——一项同、一项不同）
///   ⇒ [`IndependenceVerdict::NotEstablished`]。
///
/// **半同不判独立**（评审 I-1 收口）：`model` 与 `provider` 只对上一半时，既不足以证
/// 「同一」（判据二要求两项都同），也不足以证「全都不同」——它是「部分同一」，故判不出。
/// **`worker.backend` 为 `None` 也算「有任一项未知」**：判据一（同 backend）无从比同，
/// 「全都不同」也缺这一项，故判不出——与 §7.3 的「任一项为 `None` ⇒ 判不出」同向。
/// **不说 `Independent` 的这几侧是本块取的 fail-closed 方向**（把「没判过」说成
/// 「判为不同」会把 `required` 时该丢的候选留下）。
pub fn independence_of(
    candidate: &AvailableVerifier,
    worker: &WorkerIdentity,
) -> IndependenceVerdict {
    // 判据一：同 backend ⇒ 不独立（worker.backend 未知时这一条无从命中，继续往下判）。
    if worker.backend.as_ref() == Some(&candidate.backend) {
        return IndependenceVerdict::NotIndependent;
    }
    // 判据二 / 判据三：model 与 provider 四项全已知才判；任一项为 None ⇒ 判不出。
    match (
        candidate.model.as_ref(),
        candidate.provider.as_ref(),
        worker.model.as_ref(),
        worker.provider.as_ref(),
    ) {
        (
            Some(candidate_model),
            Some(candidate_provider),
            Some(worker_model),
            Some(worker_provider),
        ) => {
            if candidate_model == worker_model && candidate_provider == worker_provider {
                return IndependenceVerdict::NotIndependent;
            }
            // 「都不同」才独立：半同（一项同、一项不同）判不出（fail-closed，评审 I-1）；
            // 「全都不同」还差 backend 那一项（判据一未命中且已知）。
            if candidate_model != worker_model
                && candidate_provider != worker_provider
                && worker.backend.is_some()
            {
                return IndependenceVerdict::Independent;
            }
        }
        _ => {}
    }
    IndependenceVerdict::NotEstablished
}

/// 规则逐条（§7.2）：取 `level` 最高的**可用**候选（`available` 里出现的即「可用」——
/// **本块不判可用性**，那是资源层与策略层的事）；下低于 `policy.min_verifier_level` 的；
/// `profile.independent_verifier_required` 为真时下不独立的；**一个不剩即 `Err`，不静默降级**。
///
/// 两处本块定的补充（设计未给，照实申报）：
/// - **两类 `Err` 的次序**：可用性（下限）过滤先于独立性过滤——下限过滤空了报
///   [`VerifyError::NoAvailableVerifier`]，否则独立性过滤空了报
///   [`VerifyError::NoIndependentVerifier`]；
/// - **同级别并列**时取 `available` 里**先出现**者（「级别内具体选哪个 backend」
///   设计 §14 第 4 条记为未定）。
///
/// 独立性过滤**包含** [`IndependenceVerdict::NotEstablished`]：`required` 为真时的要求
/// 是「独立」，「未确立」不满足它（fail-closed，§7.3 判据三）。
/// `required` 为假时不过滤独立性——身份未知/不独立的候选保留，并在结果里标注。
pub fn select_verifier(
    available: &[AvailableVerifier],
    policy: &VerificationPolicy,
    profile: &VerificationProfile,
    worker: &WorkerIdentity,
) -> Result<VerifierSelection, VerifyError> {
    // §7.2 第 2 条：下限过滤（`None` = 不限）。「下低于下限的」= 优先序上更低者被弃。
    let above_floor: Vec<&AvailableVerifier> = available
        .iter()
        .filter(|candidate| match policy.min_verifier_level {
            Some(min) => candidate.level <= min,
            None => true,
        })
        .collect();
    if above_floor.is_empty() {
        return Err(VerifyError::NoAvailableVerifier);
    }

    // §7.2 第 3 条：`required` 为真时下不独立的候选（含「未确立」，fail-closed）。
    let eligible: Vec<&AvailableVerifier> = if profile.independent_verifier_required {
        above_floor
            .iter()
            .copied()
            .filter(|candidate| {
                independence_of(candidate, worker) == IndependenceVerdict::Independent
            })
            .collect()
    } else {
        above_floor
    };
    if eligible.is_empty() {
        return Err(VerifyError::NoIndependentVerifier);
    }

    // §7.2 第 1 条：取 `level` 最高（`Ord` 最小）者；并列取先出现者
    // （`Iterator::min_by_key` 在并列时返回第一个最小元素）。
    let chosen = eligible
        .iter()
        .min_by_key(|candidate| candidate.level)
        .expect("上面已证 eligible 非空");
    Ok(VerifierSelection {
        level: chosen.level,
        backend: chosen.backend.clone(),
        independence: independence_of(chosen, worker),
    })
}
