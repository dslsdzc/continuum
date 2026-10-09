//! §3.1 的 Temporal Resolver（`docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md:332`）。
//!
//! 设计那一行给的全部内容是：`resolve_temporal(&Mention, &Now) -> TemporalBinding`，
//! 以及一句纪律——**「`Now`」是入参，本层不取系统时钟**（与 P2 的入口同一条纪律）。
//! `Now` 与 `TemporalBinding` 的**形状设计都没给**（全篇各只出现一次：`design:332`），
//! 相对时态表达的**词表也没有**。故本文件有三个地方是**本层自定的**，逐个写明理由与代价。
//!
//! # 一、`Now` 取 `i64` 毫秒
//!
//! 不是发明：设计 §5.1 已经给本层的时间表示定了口径——`created_at` / `updated_at`
//! 取 **`i64`（毫秒）**，「照 P0 的 `now_millis()`」（`design:840`；
//! 那个函数在 `crates/continuum-persist/src/db.rs:152`，今天 `pub(crate)`）。
//! 本层的瞬时量取同一表示，免得同一份设计里两处时间不同轴。
//!
//! **为什么另立一个类型而不是裸 `i64`**：「`Now` 是入参」这条纪律需要**有一个入参的位置**
//! 才写得出来。裸 `i64` 下，「函数体里自己取一次时钟」与「用入参 `now`」在类型上无从分辨
//! ——而本 task 那条守卫（两个不同的 `Now` 得不同结果）要的正是「调用方给的那个值真的被用上」。
//!
//! # 二、`TemporalBinding` 只有 `At` 与 `Unbound` 两枚
//!
//! **`At` 装的是绝对时刻**（与 `Now` 同一坐标系：自 Unix epoch 起的毫秒），
//! 不是相对偏移。**这里不是 `Offset(i64)`：偏移不需要 `Now`**——那样写，
//! `&Now` 就成了一个用不上的入参，「`Now` 是入参」这条纪律随之空转
//! （`tests/canonical.rs::temporal_resolution_takes_now_as_an_argument` 钉的正是这一点：
//! 同一个 mention 配两个相隔很远的 `Now` 必须得到不同结果）。
//!
//! `Unbound` 是 **fail-closed** 的那一支：mention 里没有本层认识的时态表达时，
//! **不绑定**，而不是拿 `now` 充一个数（那会造出一个没人能分辨的假绑定）。
//!
//! # 三、字面表是本层自定的（与 Task 1 那两张表同一处境）
//!
//! §270 与 §196／§197 **都没有给时态表达的词表**，§196 列的十类输入里**没有时态一类**
//! （设计 §16 第 5b 条记的是同类缺口，`design:2309-2318`：那一条管 Task 1 的
//! `PRONOUNS` / `CONTEXT_REFERENCES` 两张表，本处是第三张）。
//! 故 `RELATIVE_DAYS`（本文件下方）是一张**封闭的、可读的最小表**，**本层自定**。
//! **收件人：规范维护者**。
//!
//! ## 代价照实记（本处无照片）
//!
//! 匹配是**整串精确相等**（`m.text` 与表项逐字相等），不做词内切分、不做修饰解析。
//! 故「明天下午」「下周三」「三天后」**都不命中**，得 `Unbound`。
//! 这些读数是**本层的设计后果**、不是实测值（没有探针），
//! 而**交付用例集里也没有对应断言**——故这段代价**今天没有守卫**（同 `surface.rs` 的 `PRONOUNS`
//! 文档里那一段「本处无照片」的处境）。
//!
//! **真实的时态 mention 来源今天零个**：Task 1 的 `detect` 只切代词与上下文引用，
//! **切不出时态表达**，故 `resolve_temporal` 在真实路径上今天拿不到输入
//! ——`tests/canonical.rs` 的夹具是**手写 `Mention`**。这与设计 §15.2 第 3 条
//! （`design:2264-2266`）写的是同一件事：能拍的只有机制。
//!
//! # 四、日历假设（本层自己标的）
//!
//! 表里的偏移按**恒定 86400000 毫秒/日**推进：不处理时区、不处理夏令时、
//! 不处理「一周从周几开始」。**规范未给日历**（§270 只给了组件名）。
//! 故 `下周` ＝ `+7` 日是一个**本层的读法**，不是从规范推出来的。
//! **收件人：规范维护者**。
//!
//! 溢出取 `saturating_add`：`Now` 是调用方给的值，本层不为它假设取值范围
//! （`i64` 毫秒已能表示 ±2.9 亿年，饱和只会出现在调用方故意给的极值上）。

use crate::Mention;

/// 一枚 UTC 日的毫秒数（本层的日历假设，见文件头第四条）。
const DAY_MS: i64 = 86_400_000;

/// 「现在」——**由调用方给**，本层不取系统时钟（`design:332`）。
///
/// 表示是 `i64` 毫秒，与设计 §5.1 给本层定的时间口径同轴（`design:840`）。
///
/// `Ord` 是结构性的（同一个坐标系上的两个瞬时可以比先后），不是本层发明的语义；
/// `Copy` 是因为它就是一个数，随值传递比借用自然。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Now(i64);

impl Now {
    /// 从「自 Unix epoch 起的毫秒」构造。**无校验**——本类型没有「合法取值」这个概念
    /// （负数毫秒是 1970 年之前，那是一个合法的瞬时）。
    pub fn from_millis(millis: i64) -> Self {
        Now(millis)
    }

    /// 取回毫秒数。
    pub fn as_millis(&self) -> i64 {
        self.0
    }
}

/// 一枚 mention 的时态绑定。
///
/// 两枚的含义、以及**为什么 `At` 装绝对时刻**，见文件头第二条。
///
/// `PartialEq` / `Eq`：本 task 的用例要拿两个 `Now` 下的结果**比是否相同**
/// ——那条守卫（`assert_ne!`）是「`Now` 是入参」这件事唯一的运行期照片。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalBinding {
    /// 解出了一个**绝对**时刻：自 Unix epoch 起的毫秒（与 [`Now`] 同一坐标系）。
    At(i64),
    /// mention 里没有本层认识的时态表达 ⟹ **不绑定**（fail-closed，不发明一个时刻）。
    Unbound,
}

/// 相对时态表达 → 相对「现在」的**日偏移**。**本层自定**，见文件头第三条。
///
/// 键是**规范化后**的字面串（`Surface` 已折过大小写，故英文条目一律小写），
/// 与 `Mention::text` 逐字相等才命中。
const RELATIVE_DAYS: &[(&str, i64)] = &[
    // 中文
    ("今天", 0),
    ("明天", 1),
    ("昨天", -1),
    ("后天", 2),
    ("前天", -2),
    ("本周", 0),
    ("这周", 0),
    ("下周", 7),
    ("上周", -7),
    ("现在", 0),
    ("此刻", 0),
    // 英文
    ("today", 0),
    ("tomorrow", 1),
    ("yesterday", -1),
    ("this week", 0),
    ("next week", 7),
    ("last week", -7),
    ("now", 0),
];

/// §3.1 的 Temporal Resolver：把一枚 mention 的时态表达解成**相对于入参 `now`** 的时刻。
///
/// **本函数是入参 `now` 的唯一时间来源**：函数体里没有任何取时钟的调用
/// （本 crate 的源码今天没有任何取系统时钟的调用；那一条**源码面**复核是 Task 21 的收尾项，
/// `plans/2026-10-06-p4-semantic-layer.md:2275`）。
///
/// 命中 `RELATIVE_DAYS` 的整串时给 `At(now + 偏移)`；否则给 `Unbound`。
/// **不 panic、不返回 `Result`**：本层没有第二种失败（同 §4.1 对 `resolve` 的理由，
/// `design:433-437`：输入是值，不做 I/O、不落库）。
pub fn resolve_temporal(mention: &Mention, now: &Now) -> TemporalBinding {
    match RELATIVE_DAYS
        .iter()
        .find(|(expression, _)| *expression == mention.text.as_str())
    {
        Some((_, days)) => TemporalBinding::At(
            now.as_millis()
                .saturating_add(days.saturating_mul(DAY_MS)),
        ),
        None => TemporalBinding::Unbound,
    }
}
