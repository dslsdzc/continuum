//! `tool` 表的迁移、编码适配与行级读写（设计 §3.3、§7）。
//!
//! 表定义、编码委托与行级读写同址（同 P2 的 `continuum-effect/src/persist.rs`）。
//! **编码本体不在本文件**：
//! - `required_capabilities` 的**元素**取自 [`CapabilityKind::as_str`] / `parse`；
//! - `effect_class` 取自 `continuum-effect` 既有的 [`EffectType::as_str`] / `parse`，
//!   **不自建第二份**；
//! - `cost` / `latency` / `trust` 取自 [`Cost`] / [`Latency`] / [`Trust`] 自己的
//!   那一对函数。
//!
//! 本文件只做两件适配：`required_capabilities` 的**容器**（JSON 字符串数组，与
//! `continuum-graph` 的 `adfir_node.capabilities` 列同形），以及**列 ↔ `Option`**
//! （`NULL` ↔ `None`）。表外取值一律转成 [`PersistError`]，**不取默认值**。
//!
//! **本文件的第三个职责是唯一一条语义判定**：写侧的登记期不变量
//! （`assert_effect_class_is_covered`，设计 §5.2）。它与上面两条适配不同，是本文件
//! 自己的判断而非委托。落在此处的理由见该函数的文档：`save_tool` 是 `tool` 表的唯一
//! 生产写点，不变量必须关在唯一入口上；那句「唯一生产写点」由
//! `tests/single_writer.rs` 的 `only_save_tool_writes_the_tool_table` 扫 `crates/*/src/`
//! 的源码文本钉住（本文件自己的用例钉不住它——别处的写入不经过本文件）。
//! **读侧不重判**：`load_tool` / `load_tools` 只解码，不因不满足不变量而报错（旧库里的
//! 历史行仍读得出来，这与本仓「表外取值一律 `Err`」那条**不冲突**——这里管的是取值合法性，
//! 不变量管的是登记项自洽性）。历史行的照片是 `tests/persist.rs` 的
//! `a_legacy_row_that_violates_the_invariant_is_still_readable`（「读得出来」这一句原先
//! 没有照片，Task 1 评审 Minor 3 指出后补）。

use continuum_core::tool::ToolId;
use continuum_effect::EffectType;
use continuum_persist::{Migration, PersistError, Tx, Value, value::kind_name};
use serde_json::Value as JsonValue;

use crate::capability::CapabilityKind;
use crate::tool::{Cost, Latency, Tool, ToolProfile, Trust};

/// 列清单只写一份：两条读路径（按 id、全量）共用同一列序，`row_to_profile` 按它
/// 取列，列序改了只有一处要改。
const COLUMNS: &str = "id, version, input_schema, output_schema, required_capabilities, \
                       effect_class, deterministic, cost, latency, trust";

/// 本 crate 在 `tool` 表上注册的迁移。
///
/// 编号 50：1、2 是 `continuum-persist` 的内建（events、audit_log），10 是 P1 的
/// artifact，20 是 P1 的 graph，30 是 P2 的 workspace，40 是 P2 的 effect，41 是 P2
/// 的 policy——七条都在**本驱动装配的那个集合**里，50 在该集合里未被占用（唯一性由
/// `crates/continuum-runtime/tests/migrations.rs` 的 `migration_versions_are_unique`
/// 逐条断言）。
///
/// **「未被占用」是按库说的，不是全仓**：`crates/continuum-persist/tests/recovery.rs`
/// 的 `db_with_probe()` 另有一张探针表也用编号 50，但它只与 `builtin_migrations()`
/// 一起开库，与驱动的集合**永不同库**，故不冲突。本段先前写的是「50 未被占用」，
/// 那是全仓层面的假命题——「我 grep 到的那些」不等于全集，这条订正留在此处。
pub fn p3_capability_migrations() -> Vec<Migration> {
    vec![Migration::new(
        50,
        "p3_capability",
        "CREATE TABLE tool (
            id                     TEXT PRIMARY KEY,
            version                TEXT NOT NULL,
            input_schema           TEXT NOT NULL,
            output_schema          TEXT NOT NULL,
            required_capabilities  TEXT NOT NULL,
            effect_class           TEXT,
            deterministic          INTEGER NOT NULL,
            cost                   TEXT,
            latency                TEXT,
            trust                  TEXT NOT NULL
        );",
    )]
}

/// 插入一条登记项。调用方（Task 5 起是 Registry 的装载方）负责给出完整的画像。
///
/// 写入前先过登记期不变量 `assert_effect_class_is_covered`（设计 §5.2）：声明了
/// `effect_class == Some(t)` 而 `required_capabilities` 不含 `for_effect(t)` 的登记项
/// 在此即被拒，**一行都不写**。
///
/// 通过之后是裸 `INSERT`，不 `OR REPLACE`：同 id 的第二次写入由主键拒绝，判据在库层而非
/// 调用方自查（设计 §3.3；与 `effect` 表的唯一索引同一条判据）。
/// `tests/persist.rs` 的 `saving_the_same_id_twice_is_rejected` 断言被拒后原行不变。
///
/// 两条拒绝**顺序固定**：不变量在前，主键冲突在后。故同 id 且不满足不变量的第二次写入
/// 报的是不变量那条，不是主键那条——不变量在前是因为它不需要读库，且「登记项本身自相
/// 矛盾」比「这个 id 已被占」更根本。
pub fn save_tool(tx: &Tx<'_>, profile: &ToolProfile) -> Result<(), PersistError> {
    let tool = profile.tool();
    assert_effect_class_is_covered(tool)?;
    tx.execute(
        "INSERT INTO tool
           (id, version, input_schema, output_schema, required_capabilities,
            effect_class, deterministic, cost, latency, trust)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        &[
            Value::text(tool.id().as_str()),
            Value::text(tool.version()),
            Value::text(to_json(tool.input_schema(), "input_schema")?),
            Value::text(to_json(tool.output_schema(), "output_schema")?),
            Value::text(encode_capabilities(tool.required_capabilities())?),
            match tool.effect_class() {
                Some(effect) => Value::text(effect.as_str()),
                None => Value::Null,
            },
            Value::Int(if tool.deterministic() { 1 } else { 0 }),
            presence(profile.cost().as_ref().map(Cost::as_str)),
            presence(profile.latency().as_ref().map(Latency::as_str)),
            Value::text(profile.trust().as_str()),
        ],
    )?;
    Ok(())
}

/// 登记期不变量（设计 §5.2）：声明了 `effect_class == Some(t)` 的工具，
/// 其 `required_capabilities` 必须含 [`CapabilityKind::for_effect`]`(t)`。
///
/// # 为什么落在本函数
///
/// `save_tool` 是 `tool` 表**唯一的生产写点**（`load_tool` / `load_tools` 只读），故不变量
/// 「关在唯一入口上」，不依赖任何调用方自觉。反过来，若把它写在调用方（Registry 的装载方
/// 或更上层的驱动），就多出一个可以忘记的入口。
///
/// **「唯一」这两个字不是修辞**：它由 `tests/single_writer.rs` 的
/// `only_save_tool_writes_the_tool_table` 扫 `crates/*/src/` 的源码文本钉住。
/// 少了那条守卫，别处新添一个写点会让本段理由静默失效而全仓照绿
/// （Task 1 评审 Minor 4 指出后补）。
///
/// # 为什么这条不变量够
///
/// 它使 `authorize` 的两向合取**要求**出示集里有一枚 `for_effect(t)`，而出示集只可能来自
/// `--effect` 铸出的能力（[`CapabilityKind::for_effect`] 是单射），于是那条效应必进
/// Journal——「工具做了外部效应却没有效应记录」的洞由此堵上。
///
/// # 射程
///
/// 前提不成立（`effect_class == None`）时**无结论**：本函数不管 `None` 的工具，其
/// `required_capabilities` 取何值都放行。不变量形如 `Some(t) ⇒ …`，不是双条件。
///
/// # 判定的关键点
///
/// - `for_effect` 写在 [`CapabilityKind`] 上（不是 `EffectType` 上）——后者会让
///   `continuum-effect` 反向依赖本 crate，而依赖方向在本项目是逐对断言的硬约束；
/// - 比对按 `CapabilityKind` 的**相等**（`contains`），不比字符串。串是落库编码，
///   两者今天同形，但编码改了不该动判定；
/// - `required_capabilities` 是**列表不是集合**：重复项不影响判定。
fn assert_effect_class_is_covered(tool: &Tool) -> Result<(), PersistError> {
    let Some(effect) = tool.effect_class() else {
        return Ok(());
    };
    let expected = CapabilityKind::for_effect(effect);
    if tool.required_capabilities().contains(&expected) {
        return Ok(());
    }
    Err(PersistError::Database(format!(
        "工具 {} 声明了 effect_class = {}，但 required_capabilities 不含其对应的能力 {}",
        tool.id().as_str(),
        effect.as_str(),
        expected.as_str(),
    )))
}

/// 按 id 读一条登记项。不存在返回 `None`。
pub fn load_tool(tx: &Tx<'_>, id: &ToolId) -> Result<Option<ToolProfile>, PersistError> {
    let rows = tx.query(
        &format!("SELECT {COLUMNS} FROM tool WHERE id = ?1"),
        &[Value::text(id.as_str())],
    )?;
    let Some(row) = rows.into_iter().next() else {
        return Ok(None);
    };
    Ok(Some(row_to_profile(&row)?))
}

/// 全部登记项，按 id 升序（顺序固定，调用方才不必自己排）。
pub fn load_tools(tx: &Tx<'_>) -> Result<Vec<ToolProfile>, PersistError> {
    tx.query(&format!("SELECT {COLUMNS} FROM tool ORDER BY id"), &[])?
        .iter()
        .map(|row| row_to_profile(row))
        .collect()
}

/// 存在性编码：`None` → `NULL`（尚未登记画像），`Some(字面量)` → 该字面量。
fn presence(literal: Option<&'static str>) -> Value {
    match literal {
        Some(s) => Value::text(s),
        None => Value::Null,
    }
}

/// `required_capabilities` 的容器编码：JSON 字符串数组。
///
/// 元素一律取自 [`CapabilityKind::as_str`]，**不派生 serde、不用 `Debug`**。容器选
/// JSON 数组是因为本仓已有同形的列（`continuum-graph` 的 `adfir_node.capabilities`），
/// 且数组里的元素是自由文本、定长分隔符（如逗号）在有取值域的那天会撞上取值本身。
fn encode_capabilities(kinds: &[CapabilityKind]) -> Result<String, PersistError> {
    let names: Vec<&str> = kinds.iter().map(CapabilityKind::as_str).collect();
    serde_json::to_string(&names)
        .map_err(|e| PersistError::Database(format!("required_capabilities 不可序列化为 JSON: {e}")))
}

/// [`encode_capabilities`] 的逆。表外元素报 [`PersistError::Database`]，
/// 不跳过也不取默认 kind。
fn decode_capabilities(s: &str) -> Result<Vec<CapabilityKind>, PersistError> {
    let names: Vec<String> = serde_json::from_str(s).map_err(|e| {
        PersistError::Database(format!("required_capabilities 不是字符串数组（{s}）: {e}"))
    })?;
    names
        .iter()
        .map(|name| {
            CapabilityKind::parse(name)
                .ok_or_else(|| PersistError::Database(format!("未知 CapabilityKind: {name}")))
        })
        .collect()
}

fn to_json(value: &JsonValue, column: &str) -> Result<String, PersistError> {
    serde_json::to_string(value)
        .map_err(|e| PersistError::Database(format!("{column} 不可序列化为 JSON: {e}")))
}

fn parse_json(s: &str, column: &str) -> Result<JsonValue, PersistError> {
    serde_json::from_str(s)
        .map_err(|e| PersistError::Database(format!("{column} 不是合法 JSON: {e}")))
}

/// 一行 → 登记项。列序与 [`COLUMNS`] 一一对应。
///
/// 每条枚举列的**表外取值**都在此转成具体 `Err`，不取默认值——把串猜成另一枚能力或
/// 另一个效应类型，正是设计要拦的（同 [`EffectType::parse`] 的文档）。
fn row_to_profile(row: &[Value]) -> Result<ToolProfile, PersistError> {
    let id = ToolId::new(text_at(row, 0)?);
    let input_schema = parse_json(&text_at(row, 2)?, "input_schema")?;
    let output_schema = parse_json(&text_at(row, 3)?, "output_schema")?;
    let required_capabilities = decode_capabilities(&text_at(row, 4)?)?;

    let effect_class = match optional_text_at(row, 5)? {
        None => None,
        Some(raw) => Some(EffectType::parse(&raw).ok_or_else(|| {
            PersistError::Database(format!("未知 EffectType: {raw}"))
        })?),
    };

    // 布尔列只认 0 / 1；其它整数值是表外取值，不当作真值
    let deterministic = match int_at(row, 6)? {
        0 => false,
        1 => true,
        other => return Err(PersistError::Database(format!("未知 deterministic: {other}"))),
    };

    let cost = match optional_text_at(row, 7)? {
        None => None,
        Some(raw) => Some(
            Cost::parse(&raw)
                .ok_or_else(|| PersistError::Database(format!("未知 Cost: {raw}")))?,
        ),
    };
    let latency = match optional_text_at(row, 8)? {
        None => None,
        Some(raw) => Some(
            Latency::parse(&raw)
                .ok_or_else(|| PersistError::Database(format!("未知 Latency: {raw}")))?,
        ),
    };
    let raw_trust = text_at(row, 9)?;
    let trust = Trust::parse(&raw_trust)
        .ok_or_else(|| PersistError::Database(format!("未知 Trust: {raw_trust}")))?;

    Ok(ToolProfile::new(
        Tool::new(
            id,
            text_at(row, 1)?,
            input_schema,
            output_schema,
            required_capabilities,
            effect_class,
            deterministic,
        ),
        cost,
        latency,
        trust,
    ))
}

/// 按下标取列，用 `convert` 把该列的 [`Value`] 收窄到 `T`。
///
/// **`PersistError::ColumnType` 只在本函数里构造一处**：三个具体取列函数
/// （[`text_at`] / [`optional_text_at`] / [`int_at`]）只提供各自的 `convert`，
/// 不各写一遍 index / `kind_name` / `"missing"` 的映射。形态与
/// `continuum-persist` 的 `Tx::audit_records` 同（那里的 `text_at`）。
///
/// 收窄不了的两种情形都报同一变体，靠 `actual` 区分：列在但类型不符记该值的
/// [`kind_name`]；列缺失（`convert` 根本没被调到）记 `"missing"`。
fn column_at<T>(
    row: &[Value],
    index: usize,
    convert: impl Fn(&Value) -> Option<T>,
) -> Result<T, PersistError> {
    row.get(index)
        .and_then(convert)
        .ok_or_else(|| PersistError::ColumnType {
            index,
            actual: row.get(index).map_or("missing", kind_name),
        })
}

fn as_text(v: &Value) -> Option<String> {
    match v {
        Value::Text(s) => Some(s.clone()),
        _ => None,
    }
}

fn as_int(v: &Value) -> Option<i64> {
    match v {
        Value::Int(i) => Some(*i),
        _ => None,
    }
}

/// `NULL` 收窄成 `Some(None)`（「该列是空的」也是一个可观察的结果，不是收窄失败），
/// 文本收窄成 `Some(Some(text))`，其余 `None`（收窄失败）。
fn as_optional_text(v: &Value) -> Option<Option<String>> {
    match v {
        Value::Null => Some(None),
        Value::Text(s) => Some(Some(s.clone())),
        _ => None,
    }
}

fn text_at(row: &[Value], index: usize) -> Result<String, PersistError> {
    column_at(row, index, as_text)
}

/// `NULL` → `None`，文本 → `Some`，其余报错。
fn optional_text_at(row: &[Value], index: usize) -> Result<Option<String>, PersistError> {
    column_at(row, index, as_optional_text)
}

fn int_at(row: &[Value], index: usize) -> Result<i64, PersistError> {
    column_at(row, index, as_int)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `actual: "missing"` 那一臂在集成测试里**没有产生方**：`SELECT` 的列清单固定，
    /// 库侧不会给出短行（`NULL` 是 [`Value::Null`]，不是缺列）。故直接喂一行空列，
    /// 钉住 `column_at` 对「列缺失」的处置——否则这条错误臂无照片。
    #[test]
    fn a_short_row_reports_the_missing_column() {
        assert_eq!(
            row_to_profile(&[]).unwrap_err(),
            PersistError::ColumnType {
                index: 0,
                actual: "missing",
            }
        );
    }
}
