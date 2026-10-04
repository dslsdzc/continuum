//! §252 的 `Tool`（工具定义）与 §87 的 `ToolProfile`（Registry 的登记项）。
//!
//! 规范给了两处字段有重叠的形状（§252 的 `Tool`、§87 的 `ToolProfile`）。本模块按职责
//! 分开，**不让重叠字段出现两份**：`Tool` 是**定义**，`ToolProfile` 是**登记项**，且
//! 登记项**含定义**而非与它并列（设计 §3.1）。§87 的 `capabilities` 与 §252 的
//! `required_capabilities` 是同一件事，取 [`Tool`] 一处；§87 的 `effects` 与 §252 的
//! `effect_class` 同理。
//!
//! `Tool.id` 复用既有的 [`ToolId`]（`continuum_core::tool`，§316 的 ToolProvider 接口
//! 类型），本模块**不另建第二个** `ToolId`：同一件事两个类型正是本项目一贯判为缺陷的
//! 那一类。`continuum-core` 的 `ToolId` 没有 `Display`，本处**不给它加**——`as_str()`
//! 够用，等真有消费方再说。

use continuum_core::tool::ToolId;
use continuum_effect::EffectType;
use serde_json::Value;

use crate::capability::CapabilityKind;

/// §252 的工具定义。
///
/// 字段私有，唯一的构造点是 [`Tool::new`]，读经各访问器。
///
/// # `effect_class` 绑到 [`EffectType`]
///
/// `effect_class` 的取值集规范没有定义（§244 的 `side_effect_class` 同样只给了字段名）。
/// 本模块**不发明第二套分类**，把它绑到既有封闭枚举 [`EffectType`] 上（设计 §3.2）——
/// 依据是 `EffectType` 已是本项目「外部副作用」的封闭集合，策略亦按它裁决；另造一个
/// `effect_class` 枚举等于让同一件事有两个词汇表。
///
/// [`Tool::effect_class`] 为 `None` 表示**无外部副作用**（纯计算或只读工具）。
/// 「`None` 不携带任何 `EffectType`」的照片是 `tests/tool.rs` 的
/// `effect_class_none_means_no_external_effect`（`EffectType::ALL` 六项逐项断言）、
/// 另一侧（有副作用的工具给出其类型）同用例承重。
///
/// # `deterministic`
///
/// §252 只给了字段名，未定义取值语义。本类型照录为 `bool`，**不替它发明含义**；
/// 它与 [`Cost`] / [`Latency`] 同属「据实写明未定」的处置。
#[derive(Debug, Clone, PartialEq)]
pub struct Tool {
    id: ToolId,
    version: String,
    input_schema: Value,
    output_schema: Value,
    required_capabilities: Vec<CapabilityKind>,
    effect_class: Option<EffectType>,
    deterministic: bool,
}

impl Tool {
    /// 逐字段构造，无缺省值：§252 的七个字段都给全（`effect_class` 给 `Option`，
    /// 故「无外部副作用」也是一次显式的填写，不是漏填）。
    pub fn new(
        id: ToolId,
        version: String,
        input_schema: Value,
        output_schema: Value,
        required_capabilities: Vec<CapabilityKind>,
        effect_class: Option<EffectType>,
        deterministic: bool,
    ) -> Self {
        Self {
            id,
            version,
            input_schema,
            output_schema,
            required_capabilities,
            effect_class,
            deterministic,
        }
    }

    pub fn id(&self) -> &ToolId {
        &self.id
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn input_schema(&self) -> &Value {
        &self.input_schema
    }

    pub fn output_schema(&self) -> &Value {
        &self.output_schema
    }

    pub fn required_capabilities(&self) -> &[CapabilityKind] {
        &self.required_capabilities
    }

    /// `None` 即无外部副作用，见本类型的文档。
    pub fn effect_class(&self) -> Option<EffectType> {
        self.effect_class
    }

    pub fn deterministic(&self) -> bool {
        self.deterministic
    }
}

/// §87 的登记项：一个工具在 Registry 里的那一条。
///
/// **含定义，不与它并列**（设计 §3.1）：定义面只有 [`Tool`] 一份，本类型不重复
/// `id` / `version` / schema / `required_capabilities` / `effect_class`。
///
/// # `cost` / `latency`：只定容器形状，取值域留空
///
/// §87 的这两个画像字段规范同样只给了名字。本类型只定**容器形状**——
/// [`Cost`] / [`Latency`] 是**单位结构体**，不是待填的壳：取值域服务子项目 D 的候选
/// 排序，而排序依据要到那时才存在（§250、§84）。**不预先发明度量。**
///
/// 两个字段取 `Option`，故「**尚未登记画像**」（`None`）与「**已登记、画像为空**」
/// （`Some(Cost)`——画像本身还没有取值域）在类型上分得开。可达 `Some` 的路径是
/// [`ToolProfile::new`] 的 `cost` / `latency` 两个入参；**两种状态都有照片**：
/// `tests/tool.rs` 的 `cost_and_latency_record_whether_a_metric_was_registered`。
///
/// # `trust` 非 `Option`
///
/// 一个工具登记进 Registry 时**必然有信任判定**，没有「尚未判定」这一状态，故本字段
/// 不收 `Option<Trust>`（§87 的第三个画像字段，取值域同样留空，形状同
/// [`Cost`] / [`Latency`] 见 [`Trust`]）。
///
/// **「没有收 `Option<Trust>` 的入口」这条没有用例，也不该有**：它是**否定的结构性
/// 命题**，运行期用例只能证明「我没那么传」，证明不了「传不进去」；钉住它需要一份
/// `trybuild` 的编译失败样例（形如 `tests/compile_fail/` 下既有那几份），而本 task
/// **刻意不为它造**——故这里没有 `.stderr` 可钉，理由留在本段。后来者若在别处读到
/// 这条保证，回来即见此段。
///
/// # 与 [`crate::Capability`] 的关系
///
/// [`Tool::required_capabilities`] 是**工具声明**它需要哪些能力，与调用方持有并出示的
/// 那几枚 [`crate::Capability`] 不是一回事：两个方向的比对（声明了没给、给了没声明）
/// 是 Task 5 的 `authorize` 的活，本 task 只做类型与取值。
#[derive(Debug, Clone, PartialEq)]
pub struct ToolProfile {
    tool: Tool,
    cost: Option<Cost>,
    latency: Option<Latency>,
    trust: Trust,
}

impl ToolProfile {
    /// 逐字段构造。`trust` 是 [`Trust`] 而非 `Option<Trust>`——本构造点是「必然有信任
    /// 判定」这条保证在 API 上的落点，见本类型的文档。
    pub fn new(tool: Tool, cost: Option<Cost>, latency: Option<Latency>, trust: Trust) -> Self {
        Self {
            tool,
            cost,
            latency,
            trust,
        }
    }

    pub fn tool(&self) -> &Tool {
        &self.tool
    }

    /// `None` 即尚未登记画像，见本类型的文档。
    pub fn cost(&self) -> Option<Cost> {
        self.cost
    }

    /// `None` 即尚未登记画像，见本类型的文档。
    pub fn latency(&self) -> Option<Latency> {
        self.latency
    }

    pub fn trust(&self) -> Trust {
        self.trust
    }
}

/// §87 的 `cost`，**单位结构体**：只定容器形状，取值域留空（设计 §3.1、§10 第 9 条）。
///
/// §87 只给了字段名，规范未给取值与单位；本模块**不预先发明度量**——它服务子项目 D
/// 的候选排序，而排序依据要到那时才存在（§250、§84）。在 [`ToolProfile`] 里以
/// `Option<Cost>` 出现，故「尚未登记画像」与「画像为空」分得开。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cost;

/// §87 的 `latency`，**单位结构体**：处置与理由同 [`Cost`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Latency;

/// §87 的 `trust`，**单位结构体**：取值域留空（设计 §3.1）。
///
/// 与 [`Cost`] / [`Latency`] 的差别只在**容器**：在 [`ToolProfile`] 里它是
/// `Trust` 而非 `Option<Trust>`——登记进 Registry 必然有信任判定（理由见
/// [`ToolProfile`] 的文档）。规范未给信任等级，本模块不发明。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Trust;
