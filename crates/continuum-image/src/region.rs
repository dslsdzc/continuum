//! 编辑区域的四种表示（§325）与辅助类型（§163、§166）。

/// §325 `:2351-2358` 的四种表示（§166 `:745-752` 是同一组的另两个名字，对应关系见 §4.1）。
/// 封闭四值。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditRegionKind {
    Point,
    Box,
    Mask,
    SemanticObject,
}

impl EditRegionKind {
    /// 全部四枚。**这里的 `const` 是成立的**——本型是无字段枚举，
    /// 与 `crates/continuum-artifact/src/artifact.rs:28` 的 `ArtifactType::ALL` 同形。
    /// **这一条与 `all_operators()` 那一处的差别就在此**：`Operator` 有字段，故它不能是 `const`（§3.1）。
    pub const ALL: [EditRegionKind; 4] = [
        EditRegionKind::Point,
        EditRegionKind::Box,
        EditRegionKind::Mask,
        EditRegionKind::SemanticObject,
    ];

    /// 本编码**唯一的产生点**（照 `ArtifactType::as_str` 的既有做法）：
    /// 小写、多词以 `_` 连接。match 穷尽且无通配臂：加一臂时本函数编译不过。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Point => "point",
            Self::Box => "box",
            Self::Mask => "mask",
            Self::SemanticObject => "semantic_object",
        }
    }

    /// [`EditRegionKind::as_str`] 的**严格逆**：对 [`EditRegionKind::ALL`] 里每枚都有
    /// `parse(t.as_str()) == Some(t)`，表外字符串一律 `None`。**它收 `&str`、必有通配臂**，
    /// 故它不是「加一臂即编译不过」那张照片。
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "point" => Self::Point,
            "box" => Self::Box,
            "mask" => Self::Mask,
            "semantic_object" => Self::SemanticObject,
            _ => return None,
        })
    }
}

/// §163 `:626-652` 的坐标归一化（`x, y ∈ [0, 1]`，实测该两句在 `:648-649`），避免与分辨率绑定。
/// 字段名 `x` / `y` 有规范出处；**类型名是本设计定的**（规范原文那个类型叫 `ImageEditAnchor`，
/// `docs/spec/03-product-drive.md:639`），见计划 `## 遗留` 第一节第 8 条。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NormalizedAnchor {
    pub x: f64,
    pub y: f64,
}

/// §166 `:749` 的 Bounding Box：它的**两个角**（设计 §4.1 定的；规范只给名字）。
/// **两角的名字不表达几何次序**：§166 没给「哪一角是左上」，故投影时逐维取 min/max，
/// 两角对调给出同一个掩码（Task 5 有一条用例钉它）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NormalizedBox {
    pub p: NormalizedAnchor,
    pub q: NormalizedAnchor,
}

/// §166 `:751` / `:769` 的 Semantic Object：以名字指一个对象。
/// **不透明串**——它不规定名字的来源、也不封闭取值域。
/// **这不填 §4.3 表第 5 行的缺口**（设计 §4.1 的「不给它定形状」讲的是不替它定名字空间）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticObjectRef(String);

impl SemanticObjectRef {
    /// 以任意串构造——**不透明串不设取值域**：不校验来源，也不拒绝表外取值。
    /// **这个名字（`from_str`）是本计划定的**：它不是 `std::str::FromStr` 的实现
    /// （那个返回 `Result`，而本型不拒任何串），与 [`SemanticObjectRef::as_str`] 互为一对往返。
    pub fn from_str(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// 原样取回名字。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// §166 `:775` 的 `EditMask`：四种表示解析后的共同产物。
/// **带的是掩码位图本身，不是指向掩码制品的 id**（Box 支的掩码由 §4.2 投影得出，
/// 那一刻仓里还没有它的制品；本块不做 IO）。
/// **单通道位图**，故本型**没有 `channels` 字段**——`Raster`（Task 6）有，因为它是图像。
/// **不变量**：`pixels.len() == width as usize * height as usize`（Task 6 的两个函数在入口核它）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditMask {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// §325 `:2351-2358` 的四种表示。
#[derive(Debug, Clone, PartialEq)]
pub enum EditRegion {
    Point(NormalizedAnchor),
    Box(NormalizedBox),
    /// §166 `:750` 的 Brush Mask：调用方已给出掩码位图，故本支带的是**已经解析好的位图**。
    Mask(EditMask),
    SemanticObject(SemanticObjectRef),
}

impl EditRegion {
    /// 本支的判别式。`RegionUnresolved { region }` 要它才能表达「是哪一型没解析出来」。
    /// **本计划自定的一处读法**（设计没给它的名字）。
    pub fn kind(&self) -> EditRegionKind {
        match self {
            Self::Point(_) => EditRegionKind::Point,
            Self::Box(_) => EditRegionKind::Box,
            Self::Mask(_) => EditRegionKind::Mask,
            Self::SemanticObject(_) => EditRegionKind::SemanticObject,
        }
    }
}
