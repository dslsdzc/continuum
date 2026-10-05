//! 模型画像（《工程》§247 §248）的取值类型。
//!
//! [`Ratio`] 与 [`SkillScore`] 是标量取值；[`SkillDimension`] / [`SkillObservation`] /
//! [`SkillVector`] 与 [`current_observation`] 是 §24 的**时间序列观测**与 §248 的**九维向量**
//! （Task 2）。`ModelProfile` 本体在后续 task 落在同一文件里。

use crate::error::ProfileError;

/// §84 / §247 / §24 的 `[0,1]` 实数（confidence、compatibility）。
///
/// # 构造期拒 NaN / ±∞ 与越界
///
/// **为什么 `Ratio` 要拒 `NaN`**：`NaN` 与任何值的比较都是 `false`，
/// `sort_by` 在含 `NaN` 的列表上不是全序——排序结果随实现细节漂移，
/// 而 §84 的输出要被比对与记录。这不是洁癖，是不确定的排序无法有照片。
///
/// 两类坏入参各报各的错，**不并成一枚**：`NaN` / `±∞` 报 [`ProfileError::NotFinite`]，
/// 「有限但落在 `0.0..=1.0` 之外」报 [`ProfileError::OutOfRange`]。判据是**跨类型一致**——
/// 同一个 `NaN` 不该在 [`Ratio`] 上是一枚错、在 [`SkillScore`] 上是另一枚；
/// 而 `NaN` 也压根不是「落在区间之外」的点（它不是区间里的点），故
/// `OutOfRange { value }` 里的 `value` 总是一个有意义的数。
///
/// 字段私有，故**构造期是唯一的入口**，不变式不可能被绕过。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ratio(f64);

impl Ratio {
    /// 构造：`NaN` / `±∞` → [`ProfileError::NotFinite`]；有限但落在 `0.0..=1.0` 之外 →
    /// [`ProfileError::OutOfRange`]，`value` 原样带回入参。
    pub fn try_new(v: f64) -> Result<Self, ProfileError> {
        // 两侧都是承重的，缺一即漏一类入参：去掉这一枚守卫会让 `NaN` 从下面两个大小
        // 比较中间穿过去（`NaN < 0.0` 与 `NaN > 1.0` 都是 false，于是被当成合法值）；
        // 去掉下面任一侧的比较会漏掉一侧越界。
        if !v.is_finite() {
            return Err(ProfileError::NotFinite);
        }
        if v < 0.0 || v > 1.0 {
            return Err(ProfileError::OutOfRange { value: v });
        }
        Ok(Self(v))
    }

    /// 取出这个实数。
    pub fn get(&self) -> f64 {
        self.0
    }

    /// 十进制串。
    ///
    /// 契约是 `parse(as_str(x)) == Some(x)`——**只声称往返，不声称最短长度**
    /// （设计 §2.4：Rust 的 `Display` 对 `f64` 保证可精确往返，故 `1e300` 会打出
    /// 三百多个字符，这是允许的）。
    pub fn as_str(&self) -> String {
        format!("{}", self.0)
    }

    /// [`Self::as_str`] 的严格逆：非数值、非有限、越界一律 `None`，**不取默认值**。
    pub fn parse(s: &str) -> Option<Self> {
        Self::try_new(s.parse::<f64>().ok()?).ok()
    }
}

/// §23 的能力评分。示例 `coding = 9.2` 的出处是 §83
/// （`docs/spec/02-positioning.md:771-773`）。
///
/// # 取值域与单位规范未定义
///
/// §23 的示例是 9.2，§24 未给范围与方向。故本类型只保证一件事——**可比较**（全序）：
/// 本设计只使用它的序，从不使用它的量，故任何跨维度的求和、加权、归一化一处都不做
/// （设计 §2.4）。这一条正是 §248 禁令在算法上的对应物：量纲未知时，
/// 唯一合法的用法是比较。
///
/// # 全序从哪来
///
/// 构造期拒 `NaN` / `±∞`（[`ProfileError::NotFinite`]），故任何两个已构造的取值
/// 都有确定的大小关系；`-0.0` 与 `0.0` 判为相等（与 `PartialEq` 一致）。
#[derive(Debug, Clone, Copy)]
pub struct SkillScore(f64);

impl SkillScore {
    /// 构造：非有限（`NaN` / `±∞`）即 [`ProfileError::NotFinite`]。
    ///
    /// 不判大小——取值域在规范里未定义，本类型不发明一个。
    pub fn try_new(v: f64) -> Result<Self, ProfileError> {
        if !v.is_finite() {
            return Err(ProfileError::NotFinite);
        }
        Ok(Self(v))
    }

    /// 取出这个实数。**本设计不使用它的量**（见类型文档），取出是为了落库编码
    /// 与外部比对。
    pub fn get(&self) -> f64 {
        self.0
    }

    /// 十进制串。契约与 [`Ratio::as_str`] 同：**只声称往返，不声称最短长度**。
    pub fn as_str(&self) -> String {
        format!("{}", self.0)
    }

    /// [`Self::as_str`] 的严格逆：非数值、非有限一律 `None`，**不取默认值**。
    pub fn parse(s: &str) -> Option<Self> {
        Self::try_new(s.parse::<f64>().ok()?).ok()
    }
}

impl PartialEq for SkillScore {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for SkillScore {}

impl PartialOrd for SkillScore {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SkillScore {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // 构造期已拒 NaN，两个有限值必有大小关系。
        // `total_cmp` 会把 `-0.0` 排在 `0.0` 之前，而那与 `PartialEq`（`==` 判它们相等）
        // 冲突，违反 `Ord` 与 `Eq` 的一致性；故先按 `==` 收口相等这一格。
        if self.0 == other.0 {
            std::cmp::Ordering::Equal
        } else {
            self.0.total_cmp(&other.0)
        }
    }
}

/// §248（`docs/spec/05-normative.md:828-842`）的九个维度。
///
/// 半封闭枚举：非法维度名不可表达。落库编码一律小写、多词以 `_` 连接（设计 §2.4），
/// 由 [`SkillDimension::as_str`] / [`SkillDimension::parse`] 这一对与类型同址的函数读写。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillDimension {
    Reasoning,
    Coding,
    Vision,
    Planning,
    ToolUse,
    ConstraintFollowing,
    Verification,
    Spatial,
    Media,
}

impl SkillDimension {
    /// 本维在 [`SkillVector`] 数组里的下标（**内存布局**）。
    ///
    /// 与 [`SkillDimension::as_str`] 是两张独立的表：前者管数组下标，后者管落库编码。
    /// 两处的顺序恰好一致，但没有任何东西依赖这一点。
    ///
    /// match 穷尽且无通配臂：加维度时本函数编译不过。
    fn index(self) -> usize {
        match self {
            Self::Reasoning => 0,
            Self::Coding => 1,
            Self::Vision => 2,
            Self::Planning => 3,
            Self::ToolUse => 4,
            Self::ConstraintFollowing => 5,
            Self::Verification => 6,
            Self::Spatial => 7,
            Self::Media => 8,
        }
    }

    /// 落库编码（设计 §2.4）：小写，多词以 `_` 连接。**本函数是该编码唯一的产生点**。
    ///
    /// match 穷尽且无通配臂：加维度时本函数编译不过，编码不会漏分支。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Reasoning => "reasoning",
            Self::Coding => "coding",
            Self::Vision => "vision",
            Self::Planning => "planning",
            Self::ToolUse => "tool_use",
            Self::ConstraintFollowing => "constraint_following",
            Self::Verification => "verification",
            Self::Spatial => "spatial",
            Self::Media => "media",
        }
    }

    /// [`SkillDimension::as_str`] 的**严格逆**：九维各有 `parse(d.as_str()) == Some(d)`，
    /// 表外字符串一律 `None`。
    ///
    /// `None` 而非某一枚默认维度：把表外串猜成某个维度会成为第二份表示
    /// （与 `CapabilityKind::parse` 同一条理由），调用方（`crate::persist`）把它转成具体 `Err`。
    /// 解码侧没有穷尽 match 的保护——来源是 `&str`，编译器点不出漏掉的臂，
    /// 故由 `tests/profile.rs` 的 `skill_dimension_encoding_is_lowercase_with_underscores` 遍历九维兜住。
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "reasoning" => Self::Reasoning,
            "coding" => Self::Coding,
            "vision" => Self::Vision,
            "planning" => Self::Planning,
            "tool_use" => Self::ToolUse,
            "constraint_following" => Self::ConstraintFollowing,
            "verification" => Self::Verification,
            "spatial" => Self::Spatial,
            "media" => Self::Media,
            _ => return None,
        })
    }
}

/// §24（`docs/spec/01-concepts.md:1106-1131`）的一次观测：
/// `score` / `confidence` / `sample_count` / `version` / `time_range`。
///
/// **评分是时间序列，不是常数**——故一个维度上的「分」不是标量，而是这一次观测的五个字段
/// （设计 §2.2）。五个字段私有，构造期是唯一入口。
///
/// # 构造期拒不自洽的时间窗
///
/// `time_range` 是 Unix 毫秒的**闭区间**，`end < start` 不自洽，报
/// [`ProfileError::BadTimeRange`]（设计 §2.4）。
#[derive(Debug, Clone, PartialEq)]
pub struct SkillObservation {
    score: SkillScore,
    confidence: Ratio,
    sample_count: u64,
    version: u32,
    time_range: (i64, i64),
}

impl SkillObservation {
    /// 构造。`time_range` 是 Unix 毫秒的**闭区间**；`end < start` → [`ProfileError::BadTimeRange`]
    /// （`start` / `end` 原样带回入参）。
    ///
    /// **`end == start` 是合法的**：闭区间上的退化区间是一个点，`start == end` 自洽。
    /// 故守卫是 `end < start` 而不是 `end <= start`——两者只在这一格上不同
    /// （`tests/profile.rs` 的 `an_observation_whose_time_range_is_reversed_is_rejected` 两侧都钉）。
    pub fn try_new(
        score: SkillScore,
        confidence: Ratio,
        sample_count: u64,
        version: u32,
        time_range: (i64, i64),
    ) -> Result<Self, ProfileError> {
        let (start, end) = time_range;
        if end < start {
            return Err(ProfileError::BadTimeRange { start, end });
        }
        Ok(Self {
            score,
            confidence,
            sample_count,
            version,
            time_range,
        })
    }

    /// §24 的 `score`。
    pub fn score(&self) -> SkillScore {
        self.score
    }

    /// §24 的 `confidence`。
    pub fn confidence(&self) -> Ratio {
        self.confidence
    }

    /// §24 的 `sample_count`。
    pub fn sample_count(&self) -> u64 {
        self.sample_count
    }

    /// §24 的 `version`——**产生方显式递增**的那个量，也是「当前观测」的判据
    /// （见 [`current_observation`]）。
    pub fn version(&self) -> u32 {
        self.version
    }

    /// §24 的 `time_range`：Unix 毫秒的**闭区间**。
    pub fn time_range(&self) -> (i64, i64) {
        self.time_range
    }
}

/// §248 的九维向量：九维各自**当前**的一次观测。
///
/// # `None` = 该维度尚无观测——**缺席不是 0**
///
/// 「这个维度没有观测」与「这个维度评分是 0」是两件事。§24 说评分有 `sample_count`，
/// 没有样本就没有评分；把缺席读成 0 会让一个未画像的模型看起来「所有维度都很差」，
/// 而这在 §23 的语义下与「很强」一样是假的（设计 §2.2）。
///
/// # 分层（§23 的子技能树）不建
///
/// §248 说「每个维度 MAY 继续分层」，而**规范没有给子维度的词表或聚合规则**，
/// 建它就要发明一套（设计 §2.2，§11 第 8 条）。
#[derive(Debug, Clone, PartialEq)]
pub struct SkillVector {
    dimensions: [Option<SkillObservation>; 9],
}

impl SkillVector {
    /// 由「维度 → 该维当前的那次观测」构造。
    ///
    /// **同一维度给两次即覆盖前一次，不合并**——合并需要一条规范未给的聚合规则
    /// （与「分层不建」同一条理由），而 `current` 这个参数名的语义就是「当前的那一次观测」。
    /// 照片是 `tests/profile.rs` 的 `a_second_observation_for_the_same_dimension_overwrites_the_first`。
    pub fn from_current(current: Vec<(SkillDimension, SkillObservation)>) -> Self {
        let mut dimensions: [Option<SkillObservation>; 9] = std::array::from_fn(|_| None);
        for (dimension, observation) in current {
            dimensions[dimension.index()] = Some(observation);
        }
        Self { dimensions }
    }

    /// 取该维度**当前**的那次观测；该维度尚无观测即 `None`（**不是**某个默认值）。
    pub fn get(&self, dimension: SkillDimension) -> Option<&SkillObservation> {
        self.dimensions[dimension.index()].as_ref()
    }
}

/// 一次维度上的「当前观测」：`version` **最大**的那次，**不是** `time_range` 最晚的那次。
///
/// 二者不一致时（先记后补）以 `version` 为准——它是 §24 列出的字段里**唯一由产生方显式递增**
/// 的量（设计 §2.2，该选择记在 §11 第 17 条）。空序列 → `None`。
///
/// `version` 并列时的取舍**规范未定义**（§24 未给判据，设计也没有裁），故这里不声称某一条：
/// 被钉的契约不存在，照片写不出来。
pub fn current_observation(series: &[SkillObservation]) -> Option<&SkillObservation> {
    series.iter().max_by_key(|observation| observation.version())
}
