//! 模型画像（《工程》§247 §248）的取值类型。
//!
//! [`Ratio`] 与 [`SkillScore`] 是标量取值；[`SkillDimension`] / [`SkillObservation`] /
//! [`SkillVector`] 与 [`current_observation`] 是 §24 的**时间序列观测**与 §248 的**九维向量**
//! （Task 2）；[`ModelProfile`] 本体是 §247 的十二字段（Task 3）。

use continuum_capability::{Cost, Latency};
use continuum_core::model::ModelId;
use continuum_core::tool::ToolId;

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

/// §247（`docs/spec/05-normative.md:802-824`）的模型画像，十二个字段。
///
/// # 字段私有：**crate 外没有构造入口**
///
/// **措辞与保证等强**：被挡住的是 **crate 外构造**；crate 内的 `try_new` 是 crate 可见的
/// （`persist::load_profile` 要用它）。若写成「唯一产生点」就过头了——**它是「crate 外无入口」，
/// 不是「全仓只有一个构造点」**（设计 §2.1）。
///
/// 这条保证与 `RoutableModel` 的闸门**同为结构性**：若画像能在内存里自由构造，一个从未落库、
/// 从未过画像流水线的模型就能被送进 `rank`，§21 的「新增模型不能直接进入自动 Router」随之失效
/// （设计 §2.1、§4.2）。
///
/// 两条通道各有一张照片，**不合并**：
/// - 结构体字面量 → `tests/compile_fail/model_profile_cannot_be_built.rs`（E0451，字段私有）；
/// - 构造函数 → `tests/compile_fail/model_profile_has_no_constructor.rs`（E0624，`try_new` 非 `pub`）。
///
/// 两张都**不是「名字无关」的证明**：它们钉的是这两个名字（字段名与 `try_new`）在 crate 外
/// 不可用，钉不住「今后不会有人加一个别的名字的公开构造函数」——名字无从枚举。
/// 这一限制据实写在这里，不假装覆盖到了。
///
/// # 用既有的类型，不新建第二个
///
/// `id` 取 §315 的 [`ModelId`]、`tools[]` 的元素取 §252/§316 的 [`ToolId`]、
/// `latency_profile` / `cost_profile` 复用 `continuum-capability` 的 [`Latency`] / [`Cost`]
/// （设计 §2.1、§2.5）——同一件事两个类型是本项目一贯判为 Critical 的那一类。
///
/// **本层不给这两个画像取值域**：它们是**单位结构体**，`Option` 的 `Some` 只表示
/// 「画像已登记」，与 P3A 的 `tool.cost` 列同一语义（`None` = 尚未登记）。取值域要等有单位
/// 可依据时才有（设计 §2.5、§11 第 15 条）。
///
/// # 不擅自补字段
///
/// §81 另要 `deployment` 与 `capability fingerprint`，P3A 还把 `trust` 一并判给过本层：
/// **§247 的十二个字段里没有它们中的任何一个**，故一处都不加（设计 §2.1、§2.5，
/// §11 第 7 / 23 条；`trust` 的退件理由见设计的 §2.5 末节）。
///
/// # 不设 `dead_code` 豁免（曾经有两处，来历留在这里）
///
/// Task 3 落地时本类型带两处 `#[allow(dead_code)]`，理由是「五个字段没有访问器、
/// `try_new` 在非测试构建下没有调用方」（计划 §遗留 的那条，明写「Task 11 及其后接上读数
/// 之后必须复核并从源码里去掉」）。**Task 7 已把两处都去掉**：
/// 写入点 `persist::save_profile` 要用满十二个字段，故五个字段各补了**真访问器**
/// （[`Self::version`] / [`Self::provider`] / [`Self::model_revision`] /
/// [`Self::latency_profile`] / [`Self::cost_profile`]），`try_new` 也由
/// `persist::load_profile` 在非测试构建下调用。**剩下的读取方一个不缺**，若将来又出现
/// `dead_code` 警告，那是要处置的发现（删字段或写明为什么留），不是可以再压豁免的事。
///
/// **本类型不派生 `PartialEq` / `Clone`** 这条判据不变：至今没有一处比对或克隆整份画像
/// （[`crate::persist`] 的往返用例逐字段比对，用的是访问器）。
pub struct ModelProfile {
    id: ModelId,
    version: String,
    provider: String,
    model_revision: String,
    modalities: Vec<String>,
    tools: Vec<ToolId>,
    skill_vector: SkillVector,
    failure_modes: Vec<String>,
    latency_profile: Option<Latency>,
    cost_profile: Option<Cost>,
    evidence_count: u64,
    confidence: Ratio,
}

impl ModelProfile {
    /// 十二个字段按 §247 的顺序全量传入。**crate 内可见**（`persist::load_profile` 与测试用）。
    ///
    /// 无 `Result`：十二个字段的类型各自在**自己的构造期**就把关（[`Ratio`] / [`SkillScore`] /
    /// [`SkillObservation`]），到这一步没有可再失败的判据——故不发明一个永不会出现的 `Err`
    /// （设计 §2.4 末段：构造失败在构造期就被拒）。
    ///
    /// # 不加 `dead_code` 豁免
    ///
    /// Task 3 落地时本函数在**非测试**构建里没有调用方（`persist::load_profile` 尚未写），
    /// 故当时带 `#[allow(dead_code)]` 并写明「`load_profile` 接上之后这一行应当删掉」。
    /// **Task 7 接上了**：本函数现在是 `persist::load_profile` 的唯一构造调用，属普通构建下的
    /// 真实调用方，故那一行已删。删掉而不是留着，是因为留着会让将来真正多余的函数也静默。
    pub(crate) fn try_new(
        id: ModelId,
        version: String,
        provider: String,
        model_revision: String,
        modalities: Vec<String>,
        tools: Vec<ToolId>,
        skill_vector: SkillVector,
        failure_modes: Vec<String>,
        latency_profile: Option<Latency>,
        cost_profile: Option<Cost>,
        evidence_count: u64,
        confidence: Ratio,
    ) -> Self {
        Self {
            id,
            version,
            provider,
            model_revision,
            modalities,
            tools,
            skill_vector,
            failure_modes,
            latency_profile,
            cost_profile,
            evidence_count,
            confidence,
        }
    }

    /// §247 的 `id`（§315 的 [`ModelId`]）。
    pub fn id(&self) -> &ModelId {
        &self.id
    }

    /// §247 的 `version`——**画像的版本**，不是 §24 观测的 `version`（后者在
    /// [`SkillObservation::version`]）。
    pub fn version(&self) -> &str {
        &self.version
    }

    /// §247 的 `provider`。词表规范未定义，故是自由文本。
    pub fn provider(&self) -> &str {
        &self.provider
    }

    /// §247 的 `model_revision`。
    pub fn model_revision(&self) -> &str {
        &self.model_revision
    }

    /// §247 的 `skill_vector`（§248 的九维，见 [`SkillVector`]）。
    pub fn skill_vector(&self) -> &SkillVector {
        &self.skill_vector
    }

    /// §247 的 `confidence`：`[0,1]` 的 [`Ratio`]，**不是 `f64`**。
    ///
    /// 越界与非有限的入参在 [`Ratio`] 的构造期即被拒，故画像里装不进一个越界置信度
    /// （设计 §2.1：取值照 §84 的示例 `0.94` / `0.51` 推导）。
    pub fn confidence(&self) -> Ratio {
        self.confidence
    }

    /// §247 的 `evidence_count`。
    pub fn evidence_count(&self) -> u64 {
        self.evidence_count
    }

    /// §247 的 `modalities[]`。**元素词表规范未定义**（§12 未给封闭集合），故是 `String`
    /// 而非某个本层自造的枚举（设计 §2.1、§11 第 8 条）。
    pub fn modalities(&self) -> &[String] {
        &self.modalities
    }

    /// §247 的 `tools[]`：元素是既有的 [`ToolId`]（§252/§316）。
    pub fn tools(&self) -> &[ToolId] {
        &self.tools
    }

    /// §247 的 `failure_modes[]`。§83 给的是自由文本例
    /// （`loses constraints in very long tasks` 等），**无封闭词表**（设计 §2.1、§11 第 9 条）。
    pub fn failure_modes(&self) -> &[String] {
        &self.failure_modes
    }

    /// §247 的 `latency_profile`：`None` = 尚未登记，`Some` = 已登记
    /// （与 P3A 的 `tool.latency` 列同一存在性编码，设计 §2.5）。
    ///
    /// **`Some` 不携带取值**：[`Latency`] 是单位结构体、取值域为空（设计 §3.1 只定容器形状），
    /// 故 `Some(Latency)` 的信息量就是「这个模型的延迟画像已登记」。
    pub fn latency_profile(&self) -> Option<Latency> {
        self.latency_profile
    }

    /// §247 的 `cost_profile`。语义与 [`Self::latency_profile`] 同（`None` = 尚未登记）。
    pub fn cost_profile(&self) -> Option<Cost> {
        self.cost_profile
    }
}

// ===== `ModelProfile` 的字段级用例（crate 内） =====
//
// 计划 Step 1 的第三条用例名是 `a_profile_has_no_total_score`。**它没有运行期形态**，
// 故这里没有同名的 `#[test]`——「读不到总分」是不可表达性命题，运行期用例只能证明
// 「我没这么读」，证明不了「读不到」。它的照片在
// `tests/compile_fail/overall_score_cannot_be_read.rs`（判据是编译失败，E0599）。
// **留名于此，以免被当成漏项**（计划 Task 3 Step 1、设计 §2.3 第 3 条 (a)）。

#[cfg(test)]
mod tests {
    use super::*;
    use continuum_capability::{Cost, Latency};
    use continuum_core::model::ModelId;
    use continuum_core::tool::ToolId;

    /// 十二个字段**全取非默认值、且同类型的两两互不相同**。
    ///
    /// 这样取值的理由就是这条用例的红的条件：全零/空串/`None` 会让「某字段漏存」与
    /// 「两个同类型字段写串」这两类缺陷**静默**——比如 `version` 与 `provider` 都被写成
    /// 空串时，对调它们也看不出来。
    fn a_profile() -> ModelProfile {
        ModelProfile::try_new(
            ModelId::new("model-alpha"),
            String::from("version-beta"),
            String::from("provider-gamma"),
            String::from("revision-delta"),
            vec![String::from("text"), String::from("image")],
            vec![ToolId::new("tool-epsilon"), ToolId::new("tool-zeta")],
            SkillVector::from_current(vec![(
                SkillDimension::Coding,
                SkillObservation::try_new(
                    SkillScore::try_new(9.2).expect("9.2 应是合法评分"),
                    Ratio::try_new(0.8).expect("0.8 应是合法置信度"),
                    7,
                    3,
                    (1_700_000_000_000, 1_700_000_001_000),
                )
                .expect("该时间窗应自洽"),
            )]),
            vec![String::from("loses constraints in very long tasks")],
            Some(Latency),
            Some(Cost),
            42,
            Ratio::try_new(0.94).expect("0.94 应是合法置信度"),
        )
    }

    /// §247 的十二个字段**逐项各断言一次**（不抽代表）。
    ///
    /// 红的条件两条，都在 `try_new` 的字段赋值上：
    /// - **漏存**：某个字段没接上入参（本夹具的值都不是该类型的默认构造，故不等即红）；
    /// - **写串**：两个同类型字段互换（如 `version` ↔ `provider`，本夹具的值互不相同，故不等即红）。
    ///
    /// 十二项里 `skill_vector` 的断言**弱一档**：`ModelProfile` 里没有第二个 `SkillVector`
    /// 字段可供写串，故它只钉「存下来的确是传进去的那一个」。这一点据实写在此处，
    /// 不冒充成与其它十一项等强。
    #[test]
    fn a_profile_carries_the_twelve_fields_of_247() {
        let profile = a_profile();

        assert_eq!(profile.id, ModelId::new("model-alpha"), "§247 第 1 项 id");
        assert_eq!(profile.version, "version-beta", "§247 第 2 项 version");
        assert_eq!(profile.provider, "provider-gamma", "§247 第 3 项 provider");
        assert_eq!(
            profile.model_revision, "revision-delta",
            "§247 第 4 项 model_revision"
        );
        assert_eq!(
            profile.modalities,
            vec![String::from("text"), String::from("image")],
            "§247 第 5 项 modalities[]"
        );
        assert_eq!(
            profile.tools,
            vec![ToolId::new("tool-epsilon"), ToolId::new("tool-zeta")],
            "§247 第 6 项 tools[]"
        );
        assert_eq!(
            profile.skill_vector,
            SkillVector::from_current(vec![(
                SkillDimension::Coding,
                SkillObservation::try_new(
                    SkillScore::try_new(9.2).expect("9.2 应是合法评分"),
                    Ratio::try_new(0.8).expect("0.8 应是合法置信度"),
                    7,
                    3,
                    (1_700_000_000_000, 1_700_000_001_000),
                )
                .expect("该时间窗应自洽"),
            )]),
            "§247 第 7 项 skill_vector"
        );
        assert_eq!(
            profile.failure_modes,
            vec![String::from("loses constraints in very long tasks")],
            "§247 第 8 项 failure_modes[]"
        );
        assert_eq!(
            profile.latency_profile,
            Some(Latency),
            "§247 第 9 项 latency_profile：`Some` 只表示「画像已登记」"
        );
        assert_eq!(
            profile.cost_profile,
            Some(Cost),
            "§247 第 10 项 cost_profile：`Some` 只表示「画像已登记」"
        );
        assert_eq!(profile.evidence_count, 42, "§247 第 11 项 evidence_count");
        assert_eq!(
            profile.confidence,
            Ratio::try_new(0.94).expect("0.94 应是合法置信度"),
            "§247 第 12 项 confidence"
        );

        // 七个访问器**逐项各断言一次**：它们也要给出所存的那个字段——「写串」在访问器
        // 这一层同样可能，且更隐蔽（`modalities()` 返回 `&self.failure_modes` 是能编译的，
        // 两者类型都是 `Vec<String>`）。
        assert_eq!(profile.id().as_str(), "model-alpha", "访问器 id()");
        assert_eq!(
            profile.skill_vector().get(SkillDimension::Coding),
            Some(&SkillObservation::try_new(
                SkillScore::try_new(9.2).expect("9.2 应是合法评分"),
                Ratio::try_new(0.8).expect("0.8 应是合法置信度"),
                7,
                3,
                (1_700_000_000_000, 1_700_000_001_000),
            )
            .expect("该时间窗应自洽")),
            "访问器 skill_vector()"
        );
        assert_eq!(
            profile.confidence(),
            Ratio::try_new(0.94).expect("0.94 应是合法置信度"),
            "访问器 confidence()"
        );
        assert_eq!(profile.evidence_count(), 42, "访问器 evidence_count()");
        assert_eq!(
            profile.modalities(),
            vec![String::from("text"), String::from("image")],
            "访问器 modalities()"
        );
        assert_eq!(
            profile.tools(),
            vec![ToolId::new("tool-epsilon"), ToolId::new("tool-zeta")],
            "访问器 tools()"
        );
        assert_eq!(
            profile.failure_modes(),
            vec![String::from("loses constraints in very long tasks")],
            "访问器 failure_modes()"
        );
    }

    /// `confidence` 是 [`Ratio`]，**不是 `f64`**。
    ///
    /// 两件事各钉一次：
    /// - **类型**：下面那行的类型标注是 `Ratio` —— 字段若被换成 `f64`（或 `Option<Ratio>`），
    ///   这里编译不过；
    /// - **取值**：越界与非有限的入参在 [`Ratio`] 的**构造期**即被拒，而 `try_new` 收的
    ///   就是这个类型，故画像里**装不进**一个越界置信度——没有第二条通道。
    #[test]
    fn the_confidence_is_a_ratio_and_nothing_else() {
        let profile = a_profile();

        let confidence: Ratio = profile.confidence();
        assert_eq!(
            confidence,
            Ratio::try_new(0.94).expect("0.94 应是合法置信度")
        );
        // 存下来的就是传进去的那一个，不是按某个默认值重算的。
        assert_eq!(profile.confidence, confidence);
        assert_eq!(profile.confidence.get(), 0.94);

        assert_eq!(
            Ratio::try_new(1.5),
            Err(ProfileError::OutOfRange { value: 1.5 }),
            "越界置信度在 `Ratio` 的构造期被拒"
        );
        assert_eq!(
            Ratio::try_new(f64::NAN),
            Err(ProfileError::NotFinite),
            "非有限的置信度也构造不出来（且不归 `OutOfRange`）"
        );
    }
}
