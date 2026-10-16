//! §34 的生成内容单独授权（设计 §7）：五枚旗标（§7.1）、四条「算子 → 旗标」映射与
//! fail-closed 的判定点（§7.2）。调用点不在本块（§7.3）。

use continuum_operator::OperatorId;

use crate::error::MediaError;

/// §34 的五个旗标（`docs/spec/01-concepts.md:1424-1428`），逐条照录。
///
/// `Debug`：`MediaError` 自身 derive(Debug)，且其 `NotPermitted` 臂以 `{required:?}` 引用本型。
/// `PartialEq`：§12.1 的逐臂往返断言要与 `parse` 的结果比——`parse(as_str(p)) == Some(p)`，
/// 没有值比较就写不出来。`Clone` / `Eq` 照本仓枚举措辞的既有做法
/// （`crates/continuum-operator/src/definition.rs:7`、`:15` 的两枚枚举同形）。
/// **不取 `Copy`**：本型不是可复制值，`PermittedGenerativeContent::contains` 按**引用**收一枚即可。
/// **这一行与设计 §7.1 的现文相同**（订正前那版只给 `#[derive(Debug)]`，与 §12.1 的断言不齐；
/// 设计 2026-10-10 的订正已给出这一行，见 `## 遗留` 第一节）。
///
/// **`ThreeDGeneration` 这一枚旗标的地位**：§34 的旗标与算子不是一一对应（旗标是 Contract 的
/// 词汇，算子是执行单元）；**本块不给它编算子**（那会落进切分 §八「`3d/` 无对家」那条无解）。
/// **这不是死臂**——一个 Contract 声明 `allow_3d_generation` 而系统里没有 3D 生成算子，
/// 是**系统不提供该能力**。它与其余四枚一起参与编码与往返（§7.2）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenerativePermission {
    Broll,              // allow_generated_broll
    Voice,              // allow_generated_voice
    Music,              // allow_generated_music
    FrameInterpolation, // allow_frame_interpolation
    ThreeDGeneration,   // allow_3d_generation
}

/// 臂数与编码的守卫：`ALL` 与两个穷尽 `match`，形状照 `ArtifactType` 的既有做法。
impl GenerativePermission {
    /// 全部五枚。供全量遍历的用例使用（与 `ArtifactType::ALL` 同形）。
    ///
    /// 完整性与数目由 `tests/permission.rs` 的
    /// `the_five_flags_are_exactly_the_five_from_section_34` 把关。
    pub const ALL: [GenerativePermission; 5] = [
        GenerativePermission::Broll,
        GenerativePermission::Voice,
        GenerativePermission::Music,
        GenerativePermission::FrameInterpolation,
        GenerativePermission::ThreeDGeneration,
    ];

    /// §34 各旗标的字符串名：小写、多词以 `_` 连接。
    ///
    /// **串的取值是本计划定的**（设计只给了类型名与五个旗标的规范名 `allow_generated_broll`
    /// 一类，没给 `as_str` 的串，见 `## 遗留` 第三节）——将来若要改是破坏性变更。
    /// **本函数是该编码唯一的产生点**。match 穷尽且无通配臂：加一枚旗标时本函数编译不过，
    /// 编码不会漏分支。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Broll => "broll",
            Self::Voice => "voice",
            Self::Music => "music",
            Self::FrameInterpolation => "frame_interpolation",
            Self::ThreeDGeneration => "three_d_generation",
        }
    }

    /// [`GenerativePermission::as_str`] 的**严格逆**：对 [`GenerativePermission::ALL`]
    /// 里的每枚都有 `GenerativePermission::parse(p.as_str()) == Some(p)`，
    /// 表外字符串一律 `None`。
    ///
    /// `None` 而不是默认枚：`parse` 的输入来自 Contract 的旗标名，取默认会把一个不认得的
    /// 旗标当成某一枚已授权的旗标处理（fail-open 的形态）。
    ///
    /// 解码侧没有穷尽 `match` 的保护（来源是 `&str` 而非枚举），故由
    /// `tests/permission.rs` 的 `the_five_flags_are_exactly_the_five_from_section_34`
    /// 遍历对照手写清单兜住。
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "broll" => Self::Broll,
            "voice" => Self::Voice,
            "music" => Self::Music,
            "frame_interpolation" => Self::FrameInterpolation,
            "three_d_generation" => Self::ThreeDGeneration,
            _ => return None,
        })
    }
}

/// Contract 里**已声明为真**的旗标集合。以**值**从 Contract 传入（P4 的规则：凡跨层输入以值传入）。
///
/// **没有 `Default`**：本型的「空集」由一个具名构造函数给出，理由是 §34 的 fail-closed——
/// 一个默认构造是全放行的类型，会让「没写旗标的 Contract」自动获得全部生成权限。
/// **这一组的三个方法与设计 §7.1 的现文相同**（订正前那版只有 `{ /* ... */ }`；设计 2026-10-10
/// 的订正已补齐，且 `contains` 按**引用**收——见 `## 遗留` 第一节）。
pub struct PermittedGenerativeContent {
    /// 已声明为真的旗标。字段私有：构造只经下面两个具名构造函数。
    permissions: Vec<GenerativePermission>,
}

impl PermittedGenerativeContent {
    /// 空集：什么都不放行。这是 §7.2 fail-closed 的**缺省**。
    pub fn none() -> Self {
        Self {
            permissions: Vec::new(),
        }
    }

    /// 逐枚给定已授权的旗标（由已声明为真的旗标构造）。
    pub fn from_permissions(permissions: &[GenerativePermission]) -> Self {
        Self {
            permissions: permissions.to_vec(),
        }
    }

    /// 本集合是否含该旗标。
    pub fn contains(&self, permission: &GenerativePermission) -> bool {
        self.permissions.iter().any(|p| p == permission)
    }
}

/// 本领域的生成类算子各自需要哪一枚旗标。`None` 表示该算子不是生成类（不触 §34 的约束）。
///
/// 四条映射逐条对应 §5.2 的四枚生成算子（§7.2）：`generate-broll` → `Broll`；
/// `generate-voice` → `Voice`；`generate-music` → `Music`；`frame-interpolation` →
/// `FrameInterpolation`。
///
/// **`ThreeDGeneration` 这一臂今天没有算子映射它**：§34 的旗标与算子不是一一对应；
/// **本块不给它编一个算子**（编一个就落进切分 §八「`3d/` 在四个具名算子里没有对家」
/// 那条无解里）。一个 Contract 声明 `allow_3d_generation` 而系统里没有 3D 生成算子，
/// 是**系统不提供该能力**。
///
/// 表外的算子一律 `None`：只有上面四条映射的算子触 §34，其余算子（含未来的新算子）
/// 不因本函数被默认套上任何旗标。
pub fn required_permission(operator: &OperatorId) -> Option<GenerativePermission> {
    match operator.as_str() {
        "generate-broll" => Some(GenerativePermission::Broll),
        "generate-voice" => Some(GenerativePermission::Voice),
        "generate-music" => Some(GenerativePermission::Music),
        "frame-interpolation" => Some(GenerativePermission::FrameInterpolation),
        _ => None,
    }
}

/// §34「系统不得擅自使用生成内容替换原始素材」的判定点。
/// 未声明 ⇒ 未授权（fail-closed），返回 `MediaError::NotPermitted { operator, required }`
/// （`required` 取 `required_permission` 给出的那一枚）。
///
/// 非生成算子（`required_permission` 为 `None`）不触 §34，一律放行。
///
/// **调用点不在本块**：谁在 `Queued → Running` 前调它，属第 3 层执行器（切分 §四第 4 条：
/// P5 不得自建执行路径）。**据实记录**：它在 P5e 落地后**没有生产调用方**，
/// 与 §4.5 的 `checkpoint()` 同形（§7.3）。
pub fn authorize_generative(
    operator: &OperatorId,
    granted: &PermittedGenerativeContent,
) -> Result<(), MediaError> {
    match required_permission(operator) {
        Some(required) => {
            if granted.contains(&required) {
                Ok(())
            } else {
                Err(MediaError::NotPermitted {
                    operator: operator.clone(),
                    required,
                })
            }
        }
        None => Ok(()),
    }
}
