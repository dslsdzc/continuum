//! 模型画像（《工程》§247 §248）的取值类型。
//!
//! 本模块自 Task 1 起只放两个**取值类型**——[`Ratio`] 与 [`SkillScore`]；九维向量、
//! 观测与 `ModelProfile` 本体在后续 task 落在同一文件里。

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
