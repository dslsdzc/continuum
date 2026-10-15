//! `MethodDomain`：§187 的四个目录名，**封闭**枚举（设计 §4.1）。

/// §187 的四个目录名（`docs/spec/04-method.md:116-142`）。
///
/// # 封闭的来历：设计决定，不是规范断言
///
/// §187 在列目录前写了「例如：」（`docs/spec/04-method.md:114`），《总纲》§8.1
/// 又以**四行**界定（`docs/01-总纲.md:1332-1335`；`:1331`／`:1336` 是围栏行）
/// ——**没有规范说这四个是穷尽的**。取封闭是为了给四个算子块一个共同的词汇表
/// （设计 §4.1；开放目录会让「未知域」到运行期才失败，与 P2 的 `EffectType`
/// 取封闭同一理由）。此 caveat 引出的缺口（image 域无目录）见设计 §8 R3。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MethodDomain {
    Software,
    Video,
    Research,
    ThreeD,
}

impl MethodDomain {
    /// §187 原文的目录次序（`docs/spec/04-method.md:117`、`:126`、`:132`、`:138`）。
    pub const ALL: [MethodDomain; 4] = [
        MethodDomain::Software,
        MethodDomain::Video,
        MethodDomain::Research,
        MethodDomain::ThreeD,
    ];

    /// 目录名（编码侧）。**穷尽 `match`、无通配臂**：加一目录即编译失败
    /// （设计 §4.1——它是「加一个目录即编译失败」这张照片的承担者之一）。
    pub fn as_str(&self) -> &'static str {
        match self {
            MethodDomain::Software => "software",
            MethodDomain::Video => "video",
            MethodDomain::Research => "research",
            MethodDomain::ThreeD => "3d",
        }
    }

    /// 解析目录名（解码侧）。
    ///
    /// **必须有通配臂 `_ => return None`**（设计 §4.1）：入参是 `&str`，
    /// 加目录**不会**让它编译失败——**它不是「加一臂即编译失败」那张照片的
    /// 承担者**，照片在运行期（`tests/domain.rs` 的往返与逐串用例）。
    /// 同形先例：`crates/continuum-artifact/src/artifact.rs:68-78`。
    pub fn parse(s: &str) -> Option<MethodDomain> {
        Some(match s {
            "software" => MethodDomain::Software,
            "video" => MethodDomain::Video,
            "research" => MethodDomain::Research,
            "3d" => MethodDomain::ThreeD,
            _ => return None,
        })
    }

    /// 本轮**是否有算子块作它的对家**（设计 §5.1、§7、§8 R2）。
    ///
    /// `Software` / `Video` / `Research` → `true`；`ThreeD` → `false`。
    /// **这是本设计的决定**（来源是当前分块，不是规范），不是规范断言。
    /// 注意：它是**域级**谓词；`Software` 里两条**条目级**的例外（`TDD`／
    /// `debugging`）另由 `UNREALIZED_BY_DESIGN` 承载（设计 §4.5），不并入本谓词。
    ///
    /// **穷尽 `match`**：加一目录即编译失败（设计 §4.1——这张照片的另一位承担者）。
    pub fn has_counterpart(&self) -> bool {
        match self {
            MethodDomain::Software | MethodDomain::Video | MethodDomain::Research => true,
            MethodDomain::ThreeD => false,
        }
    }
}
