//! 七个上下文来源标签的唯一类型 `ContextTrust`。
//!
//! 变体名是 Rust 拼法；[`ContextTrust::as_str`] 给出 §281 的逐字拼法。

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

/// §281 的七个上下文来源标签。**本类型是全仓唯一的一份**（切分 §2.3）。
///
/// 变体名是 Rust 拼法；[`ContextTrust::as_str`] 给出 §281 的逐字拼法。
///
/// **不派生 `Ord` / `PartialOrd`**：§281 没有给过七枚之间的次序，派生会把声明次序
/// 偷偷变成一条「信任阶梯」，而任何 `a < b` 都会读成「a 比 b 更不可信」——规范没有
/// 这句话。**不派生 `Default`**：默认标签会让一段内容静默拿到一个信任级别。
/// **不写 `#[non_exhaustive]`**：七枚是闭集，加第八枚必须让每个穷尽 `match` 重新分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContextTrust {
    UserInstruction,
    TrustedRuntime,
    ProjectData,
    WebUntrusted,
    ToolOutput,
    Memory,
    ModelOutput,
}

impl ContextTrust {
    /// §281 的次序，逐字照录。**次序不表示信任高低**，只用于 `ALL` 的全量遍历与
    /// 测试的确定性输出（设计 §3.8）。
    pub const ALL: [ContextTrust; 7] = [
        ContextTrust::UserInstruction,
        ContextTrust::TrustedRuntime,
        ContextTrust::ProjectData,
        ContextTrust::WebUntrusted,
        ContextTrust::ToolOutput,
        ContextTrust::Memory,
        ContextTrust::ModelOutput,
    ];

    /// §281 的七个串，逐字。**本函数是该编码唯一的产生点**；穷尽 `match`、无通配臂
    /// ——加第八枚时编译不过。
    pub fn as_str(self) -> &'static str {
        match self {
            ContextTrust::UserInstruction => "USER_INSTRUCTION",
            ContextTrust::TrustedRuntime => "TRUSTED_RUNTIME",
            ContextTrust::ProjectData => "PROJECT_DATA",
            ContextTrust::WebUntrusted => "WEB_UNTRUSTED",
            ContextTrust::ToolOutput => "TOOL_OUTPUT",
            ContextTrust::Memory => "MEMORY",
            ContextTrust::ModelOutput => "MODEL_OUTPUT",
        }
    }

    /// [`ContextTrust::as_str`] 的**严格逆**：表外字符串一律 `None`。
    /// **不接受小写、别名、前后空白**：接受它们等于给 §281 的词汇表造第二套拼法。
    ///
    /// 这里的七个串是解码侧的反向表，不是第二个产生点：它与 `as_str` 同处本文件、
    /// 认同一组规范串；改一边而另一边不跟改时，round-trip 用例会红。
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "USER_INSTRUCTION" => Some(ContextTrust::UserInstruction),
            "TRUSTED_RUNTIME" => Some(ContextTrust::TrustedRuntime),
            "PROJECT_DATA" => Some(ContextTrust::ProjectData),
            "WEB_UNTRUSTED" => Some(ContextTrust::WebUntrusted),
            "TOOL_OUTPUT" => Some(ContextTrust::ToolOutput),
            "MEMORY" => Some(ContextTrust::Memory),
            "MODEL_OUTPUT" => Some(ContextTrust::ModelOutput),
            _ => None,
        }
    }

    /// 这枚标签**可不可以**出现在 `instructions[]`。真值只有两枚：
    /// `USER_INSTRUCTION` / `TRUSTED_RUNTIME`。
    ///
    /// **它回答的是标签层的「允许」，不是「这一段在包里是不是指令」**：后者看它落在
    /// 哪个数组。`TRUSTED_RUNTIME` 两面都可（runtime 既产指令也产事实），故它
    /// 不在 [`Self::is_data_only`] 里。**`USER_INSTRUCTION` 有一层 provenance 级的额外
    /// 限制**：`UserInstruction { historical: true }` 即使标签是 `USER_INSTRUCTION`，
    /// 也不得进 `instructions[]`；这枚谓词只表达标签层，不替该限制背书。
    pub fn may_sit_in_instructions(self) -> bool {
        match self {
            ContextTrust::UserInstruction | ContextTrust::TrustedRuntime => true,
            ContextTrust::ProjectData
            | ContextTrust::WebUntrusted
            | ContextTrust::ToolOutput
            | ContextTrust::Memory
            | ContextTrust::ModelOutput => false,
        }
    }

    /// 这枚标签是不是 **data-only**：`PROJECT_DATA` / `WEB_UNTRUSTED` / `TOOL_OUTPUT` /
    /// `MEMORY` / `MODEL_OUTPUT` 五枚。真值支不得进 `instructions[]`。
    ///
    /// **不要**写成 `!may_sit_in_instructions()`：加第八枚时，那条否定式不会强迫
    /// 实现者重新回答「它属于哪一类」。
    pub fn is_data_only(self) -> bool {
        match self {
            ContextTrust::UserInstruction | ContextTrust::TrustedRuntime => false,
            ContextTrust::ProjectData
            | ContextTrust::WebUntrusted
            | ContextTrust::ToolOutput
            | ContextTrust::Memory
            | ContextTrust::ModelOutput => true,
        }
    }

    /// 这枚标签是不是 `USER_INSTRUCTION`。它是**唯一**在数据数组里需要额外规则的标签：
    /// `artifacts[]` / `memories[]` 一律拒绝；`references[]` 只放行
    /// `historical: true` 的历史用户轮。当前指令（`historical: false`）不得进入任何
    /// 数据数组——否则同一段用户文本就有了第二个信任位。
    pub fn is_user_instruction(self) -> bool {
        match self {
            ContextTrust::UserInstruction => true,
            ContextTrust::TrustedRuntime
            | ContextTrust::ProjectData
            | ContextTrust::WebUntrusted
            | ContextTrust::ToolOutput
            | ContextTrust::Memory
            | ContextTrust::ModelOutput => false,
        }
    }
}

impl fmt::Display for ContextTrust {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for ContextTrust {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ContextTrust {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::parse(&s)
            .ok_or_else(|| de::Error::custom(format!("不是 §281 的上下文信任标签：{s}")))
    }
}
