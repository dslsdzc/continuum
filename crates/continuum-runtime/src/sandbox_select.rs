//! 沙箱机制的装配点：按能力选机制（设计下篇第 4.3 节）。
//!
//! 拆成两层是刻意的：
//!
//! - [`select`] 是**纯函数**，入参是两份能力报告与「是否显式指定」，出参是选中的机制或
//!   `Err`。判定逻辑在这里，故它可被单测直接驱动——两个机制的 `Sandbox` 句柄都带私有
//!   字段且没有公开构造函数（见 `continuum_sandbox` 的 `LandlockSandbox` 与
//!   `BubblewrapSandbox`），crate 外造不出「ABI = 0」或「bwrap 不在 PATH」的句柄，
//!   直接在探测处判定就只能靠改环境变量去「模拟」——那是测环境变量，不是测降级。
//! - [`select_for_this_machine`] 是薄装配：调 `Sandbox::landlock()` / `Sandbox::bubblewrap()`
//!   取真实能力（两者的探测都在构造期做一次），交给纯函数，再返回对应的句柄。
//!
//! # 可用判据是 `restricts_filesystem_writes`
//!
//! 两条分支的该项取值恰好等价于「该机制在本机不存在」（读源码确认）：
//!
//! - Landlock：`landlock::capabilities_for_abi` 给出 `abi >= MIN_ABI`，而 `MIN_ABI = 1`；
//!   `abi < 1` 正是 [`continuum_sandbox`] 里 `spawn` 据以拒绝启动的那一路
//!   （`MechanismUnavailable`）。
//! - bubblewrap：`Sandbox::capabilities` 取「构造期是否定位到 `bwrap`」，定位不到时
//!   `spawn` 同样拒绝启动。
//!
//! 该等价在 `continuum-sandbox` 内**两侧各有自己的用例**（一边断言报告为假、一边断言
//! 拒绝启动）：`sandbox::tests::spawn_refuses_when_landlock_is_unavailable` 与
//! `landlock::tests::abi_below_one_means_the_mechanism_does_not_exist`、
//! `sandbox::tests::bubblewrap_refuses_to_start_when_the_binary_is_missing`。
//! 本模块据以判定的正是那份报告，故它说谎时本模块会跟着错——这层依赖是刻意的，
//! 多立一套「可用性」判据只会与 `spawn` 漂移，而漂移的后果是这里说可用、那里拒绝启动。
//!
//! 本模块**不**读环境变量、不假定任何机制可用：不可用时一律 `Err`（两个 `Err` 变体各有
//! 用例），不静默放出一个零隔离的子进程（设计第 4.3 节的 fail-closed 方向）。

use continuum_runtime::cli::SandboxMechanism;
use continuum_sandbox::{Sandbox, SandboxCapabilities};

/// 机制选择失败的原因。
///
/// 两个变体都表示「拒绝运行」：本层的契约是不选出机制就不启动任何进程，故调用方没有
/// 「拿到 Err 但照旧往下走」这一支。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SandboxSelectError {
    /// 显式指定的机制在本机不可用。
    ///
    /// **不回退到别的机制**：见模块文档与设计第 4.3 节。`mechanism` 是调用方点名的那一个，
    /// `reason` 说明它为什么不可用。
    #[error("显式指定的沙箱机制 {mechanism} 在本机不可用（{reason}）；不自动改选别的机制")]
    ExplicitUnavailable {
        mechanism: &'static str,
        reason: &'static str,
    },
    /// 两种机制在本机都不可用。
    #[error("两种沙箱机制在本机都不可用：landlock（{landlock}）、bubblewrap（{bubblewrap}）；拒绝在无文件系统隔离下运行")]
    NoneAvailable {
        landlock: &'static str,
        bubblewrap: &'static str,
    },
}

/// 机制不可用时的说明，取自该机制的**可用判据**本身（见模块文档）。
///
/// 纯函数只拿得到能力报告，拿不到探测细节（ABI 数字、PATH 内容），故理由按机制名给出
/// 判据，而不是编一个看起来具体的数字。
fn unavailable_reason(mechanism: SandboxMechanism) -> &'static str {
    match mechanism {
        SandboxMechanism::Landlock => "内核 Landlock ABI < 1，该机制在本机不存在",
        SandboxMechanism::Bubblewrap => "PATH 中找不到 bwrap 可执行文件，该机制在本机不存在",
    }
}

/// 该机制在本机是否可用：见模块文档的判据。
fn is_available(caps: &SandboxCapabilities) -> bool {
    caps.restricts_filesystem_writes
}

/// 按两份能力报告与「是否显式指定」选出机制（设计第 4.3 节）。
///
/// - 显式指定：**指定的那个可用就用它，不可用即 `Err`**——即使另一个可用也不改选。
/// - 未指定（自动）：优先 Landlock，其不可用时退到 bubblewrap，两者都不可用即 `Err`。
///
/// 入参取**两份报告**而不是一个 `Sandbox`：`SandboxCapabilities` 的字段公开且 `Copy`，
/// 两个机制的句柄却不是（见模块文档）。
pub fn select(
    explicit: Option<SandboxMechanism>,
    landlock: SandboxCapabilities,
    bubblewrap: SandboxCapabilities,
) -> Result<SandboxMechanism, SandboxSelectError> {
    // 两条分支的能力报告各取各的：显式指定那条只查被点名的那一份，故「另一个可用」
    // 在这条路上不起任何作用。
    let caps = |mechanism: SandboxMechanism| match mechanism {
        SandboxMechanism::Landlock => landlock,
        SandboxMechanism::Bubblewrap => bubblewrap,
    };

    if let Some(mechanism) = explicit {
        return if is_available(&caps(mechanism)) {
            Ok(mechanism)
        } else {
            Err(SandboxSelectError::ExplicitUnavailable {
                mechanism: mechanism.as_str(),
                reason: unavailable_reason(mechanism),
            })
        };
    }

    if is_available(&landlock) {
        return Ok(SandboxMechanism::Landlock);
    }
    if is_available(&bubblewrap) {
        return Ok(SandboxMechanism::Bubblewrap);
    }
    Err(SandboxSelectError::NoneAvailable {
        landlock: unavailable_reason(SandboxMechanism::Landlock),
        bubblewrap: unavailable_reason(SandboxMechanism::Bubblewrap),
    })
}

/// 在本机选机制并给出对应的句柄。
///
/// 两个句柄都构造一次：构造只做探测（内核 ABI、`bwrap` 定位），不起进程、不改磁盘，
/// 故「先问双方能力再选」与「选完再构造」在可观察行为上等同。一并返回选中的机制，
/// 是为了让「装配处与纯函数在本机给出同一个答案」这件事可被断言——只返回句柄的话，
/// 那份一致性没有胶水可用。
pub fn select_for_this_machine(
    explicit: Option<SandboxMechanism>,
) -> Result<(SandboxMechanism, Sandbox), SandboxSelectError> {
    let landlock = Sandbox::landlock();
    let bubblewrap = Sandbox::bubblewrap();
    let mechanism = select(explicit, landlock.capabilities(), bubblewrap.capabilities())?;
    let sandbox = match mechanism {
        SandboxMechanism::Landlock => landlock,
        SandboxMechanism::Bubblewrap => bubblewrap,
    };
    Ok((mechanism, sandbox))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 造一份能力报告。只给 `restricts_filesystem_writes` 一个变量——本模块的判据只看它，
    /// 另两项按 `Sandbox::capabilities` 的实际取值固定（两者本子项目都不隔离）。
    fn caps(available: bool, mechanism: &'static str) -> SandboxCapabilities {
        SandboxCapabilities {
            restricts_filesystem_writes: available,
            isolates_network: false,
            isolates_pid: false,
            mechanism,
        }
    }

    fn landlock(available: bool) -> SandboxCapabilities {
        caps(available, "landlock")
    }

    fn bubblewrap(available: bool) -> SandboxCapabilities {
        caps(available, "bubblewrap")
    }

    /// 自动选择第一条分支：Landlock 可用时选它（设计第 4.3 节的「优先 Landlock」）。
    ///
    /// 两个方向都要：只断言「两个都可用时选 Landlock」的话，一个恒选 Landlock 的实现
    /// 也能过——而它在 Landlock 不可用时正是要退到 bubblewrap 的那一个。
    #[test]
    fn landlock_is_chosen_first_when_it_is_available() {
        assert_eq!(
            select(None, landlock(true), bubblewrap(true)),
            Ok(SandboxMechanism::Landlock),
            "两者都可用时应优先 Landlock"
        );
        assert_eq!(
            select(None, landlock(true), bubblewrap(false)),
            Ok(SandboxMechanism::Landlock),
            "只有 Landlock 可用时应选它"
        );
    }

    /// 自动选择第二条分支：Landlock 不可用时退到 bubblewrap。
    ///
    /// 这就是「ABI < 1」那一路的降级——本机内核支持 Landlock，故只能在这个纯函数上测。
    #[test]
    fn bubblewrap_is_chosen_when_landlock_is_unavailable() {
        assert_eq!(
            select(None, landlock(false), bubblewrap(true)),
            Ok(SandboxMechanism::Bubblewrap),
            "Landlock 不可用时应退到 bubblewrap"
        );
    }

    /// 两者都不可用：拒绝运行，并点名两个机制各自为什么不可用。
    ///
    /// 断言到**具体变体**：只断言「返回了 Err」不足以把它与「显式指定那个机制不可用」
    /// 分开（那是另一条分支，处置相同但理由不同）。
    #[test]
    fn both_unavailable_refuses_to_run() {
        let err = select(None, landlock(false), bubblewrap(false)).unwrap_err();
        // 期望值用字面量写死（不用被测模块的 `unavailable_reason`）：用它拼期望值的话，
        // 理由文案可以随便改而断言照过，钉住的就只剩变体本身。
        assert_eq!(
            err,
            SandboxSelectError::NoneAvailable {
                landlock: "内核 Landlock ABI < 1，该机制在本机不存在",
                bubblewrap: "PATH 中找不到 bwrap 可执行文件，该机制在本机不存在",
            }
        );
        // 两个机制名都要在信息里：只说「都不可用」会让人无从知道该装哪一个。
        let msg = err.to_string();
        for name in ["landlock", "bubblewrap"] {
            assert!(msg.contains(name), "错误信息应点名 {name}，实际：{msg}");
        }
    }

    /// 显式指定且该机制可用：用它——**包括它不是自动选择会挑的那一个**。
    ///
    /// 「显式指定的那个可用 → 用它」若只测 Landlock，一个「显式指定一律当没给」的实现
    /// 也能过（自动选择恰好也挑 Landlock），故这里用 bubblewrap 作显式值。
    #[test]
    fn an_explicitly_named_available_mechanism_is_used() {
        assert_eq!(
            select(Some(SandboxMechanism::Bubblewrap), landlock(true), bubblewrap(true)),
            Ok(SandboxMechanism::Bubblewrap),
            "显式指定 bubblewrap 且它可用时，必须用 bubblewrap 而不是自动选择的 Landlock"
        );
        assert_eq!(
            select(Some(SandboxMechanism::Landlock), landlock(true), bubblewrap(true)),
            Ok(SandboxMechanism::Landlock),
        );
    }

    /// 显式指定但该机制不可用：`Err`，**即使另一个可用**。
    ///
    /// 两个方向各一条：指定的不可用而另一个可用 → 仍 Err（不许静默改选）；反过来
    /// 指定的可用而另一个不可用 → 用指定的那个（不许因为自动选择的次序而拒绝）。
    #[test]
    fn an_explicitly_named_unavailable_mechanism_is_refused_even_when_the_other_is_available() {
        let err = select(Some(SandboxMechanism::Bubblewrap), landlock(true), bubblewrap(false))
            .unwrap_err();
        assert_eq!(
            err,
            SandboxSelectError::ExplicitUnavailable {
                mechanism: "bubblewrap",
                reason: "PATH 中找不到 bwrap 可执行文件，该机制在本机不存在",
            },
            "显式指定的机制不可用时应拒绝，而不是改选另一个"
        );
        assert!(
            err.to_string().contains("bubblewrap"),
            "错误信息应点名被点名的那个机制，实际：{err}"
        );

        // 另一个方向：指定的可用、另一个不可用。
        assert_eq!(
            select(Some(SandboxMechanism::Bubblewrap), landlock(false), bubblewrap(true)),
            Ok(SandboxMechanism::Bubblewrap),
            "显式指定的那个可用时不得因为「优先 Landlock」而拒绝"
        );
    }

    /// 装配函数与纯函数在本机给出同一个答案。
    ///
    /// 少了这条，纯函数可以被测得很周全而装配处接错线（例如把两份能力报告传反），
    /// 三条纯函数用例照旧全绿。本机实际的答案由 `Sandbox::capabilities()` 决定，
    /// 故断言写成「装配 = 纯函数（用同样的两份报告）」，不在用例里假定本机有哪种机制。
    #[test]
    fn the_assembly_point_agrees_with_the_pure_function_on_this_machine() {
        for explicit in [
            None,
            Some(SandboxMechanism::Landlock),
            Some(SandboxMechanism::Bubblewrap),
        ] {
            let expected = select(
                explicit,
                Sandbox::landlock().capabilities(),
                Sandbox::bubblewrap().capabilities(),
            );
            let actual = select_for_this_machine(explicit).map(|(mechanism, _)| mechanism);
            assert_eq!(
                actual, expected,
                "显式指定 {explicit:?} 时装配点的答案与纯函数不符——能力报告可能接错了侧"
            );
        }
    }
}
