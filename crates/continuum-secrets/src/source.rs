//! 凭据源（设计 §5.3）：文件 + 环境变量两个**真实现**。
//!
//! # 与 §100 的据实偏离（设计 §10 第 1 条）
//!
//! §100 要求 TOTP 那类高敏 secret「**应优先**存放在 TPM / Secure Enclave / 硬件密钥
//! 库」。**本项目没有硬件后端**。§100 是 SHOULD 而非 MUST，故本阶段采用文件 + 环境
//! 变量是**据实记录的偏离**，不是违规；**长期阶段的替换点就是 TPM / Secure Enclave**
//! （设计 §10 第 1 条指向的那一处）。
//!
//! 本阶段的凭据源是**真实现、不是桩**：能签发、能按能力校验作用域（源不覆盖该作用域
//! 即拒签）、能过期（声称自己的到期时刻，运行时按能力截断）、能轮换（文件源 `reload`，
//! 环境变量源每次取用现读）。
//!
//! # 材料不落凭据
//!
//! 两个源都只在 [`CredentialSource::fetch`] 里交出材料；凭据上只有作用域、到期时刻与
//! 代号（见 `runtime.rs` 的 `Credential`）。材料类型 [`SecretMaterial`] 的 `Debug`
//! 手工写成隐去内容，且**没有** `Display`——任何把它打进日志的形状都不存在。

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::error::SecretsError;

/// 凭据材料。
///
/// **不派生 `Debug`，也不实现 `Display`**：材料是密钥，凡是能把它打进日志的形状都是
/// 泄露面。取用必须走显式的 [`SecretMaterial::expose`]——调用点写出这个名字，就等于
/// 在评审里显形了一次「此处把材料取出来用了」。
pub struct SecretMaterial(Vec<u8>);

impl SecretMaterial {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Self {
        Self(bytes.into())
    }

    /// 取出材料的字节。函数名就是它的用途：取用即显式。
    pub fn expose(&self) -> &[u8] {
        &self.0
    }
}

/// 只出长度，不出内容。照片：`tests/issue.rs` 的
/// `the_material_is_redacted_in_debug_output`。
impl fmt::Debug for SecretMaterial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecretMaterial(<已隐去>, {} 字节)", self.0.len())
    }
}

/// 一个凭据源。材料与它**声称的**到期时刻都由源给出（设计 §5.2：材料本身由凭据源
/// 给出）；运行时取二者中更早的那一个（`issue` 里的 `min`）。
///
/// `now` 由调用方给：源**不读时钟**，与 `Capability` 同一条约定（核不收时钟）。
pub trait CredentialSource {
    /// 本源的短名。只进错误消息，**不含材料**。
    fn name(&self) -> &str;

    /// 本源为 `scope` 声称的到期时刻（Unix 毫秒）。源不覆盖该作用域 →
    /// [`SecretsError::ScopeNotCovered`]。
    ///
    /// **不判「是否已经过去」**：源只报数；过去的数会让签出的凭据在访问点被
    /// [`crate::Credential::is_valid_at`] 判失效，比较不在源里再来一份。
    fn claimed_expiry(&self, scope: &str, now: i64) -> Result<i64, SecretsError>;

    /// 取 `scope` 的材料。源不覆盖该作用域 → [`SecretsError::ScopeNotCovered`]。
    fn fetch(&self, scope: &str, now: i64) -> Result<SecretMaterial, SecretsError>;
}

/// 文件凭据源：每行一条 `作用域<TAB>材料<TAB>到期时刻`，`#` 开头与空行忽略。
///
/// 作用域按**逐字匹配**，不做任何规范化：`repo/X` 与 `repo.X` 是两条不同的作用域
/// ——与 [`env_var_name`] 的编码同口径：那边的编码也是单射，故两个源对「作用域是
/// 什么」不会各说各话。
///
/// 材料字段不得含 TAB 或换行（文件是按行按 TAB 切分的）。这一约束是格式的一部分，
/// 不是「尽量」。
pub struct FileCredentialSource {
    name: String,
    path: PathBuf,
    entries: BTreeMap<String, Entry>,
}

struct Entry {
    material: Vec<u8>,
    expiry: i64,
}

impl FileCredentialSource {
    /// 读入并解析。解析失败时**不留下半个源**（没有可用的 `Self`）。
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, SecretsError> {
        let path = path.into();
        let name = path.display().to_string();
        let contents = read(&name, &path)?;
        let entries = parse(&name, &contents)?;
        Ok(Self {
            name,
            path,
            entries,
        })
    }

    /// 重读文件（轮换的一种：运维重写文件后调用）。解析失败时**保留旧的条目**——
    /// 失败不该让一个本来可用的源变成空的。
    pub fn reload(&mut self) -> Result<(), SecretsError> {
        let contents = read(&self.name, &self.path)?;
        self.entries = parse(&self.name, &contents)?;
        Ok(())
    }
}

impl CredentialSource for FileCredentialSource {
    fn name(&self) -> &str {
        &self.name
    }

    fn claimed_expiry(&self, scope: &str, _now: i64) -> Result<i64, SecretsError> {
        self.entries
            .get(scope)
            .map(|entry| entry.expiry)
            .ok_or_else(|| not_covered(&self.name, scope))
    }

    fn fetch(&self, scope: &str, _now: i64) -> Result<SecretMaterial, SecretsError> {
        self.entries
            .get(scope)
            .map(|entry| SecretMaterial::new(entry.material.clone()))
            .ok_or_else(|| not_covered(&self.name, scope))
    }
}

/// 手工写的 `Debug`：**只出源名、路径与条目数**，不出材料。
/// 照片：`tests/source.rs` 的 `the_source_debug_output_does_not_carry_the_material`。
impl fmt::Debug for FileCredentialSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileCredentialSource")
            .field("name", &self.name)
            .field("path", &self.path)
            .field("scopes", &self.entries.len())
            .finish()
    }
}

fn read(name: &str, path: &Path) -> Result<String, SecretsError> {
    std::fs::read_to_string(path).map_err(|err| SecretsError::SourceIo {
        origin: name.to_owned(),
        message: err.to_string(),
    })
}

/// 解析整份文件。任何一行不合法即整份拒收——半个源比没有源更危险。
fn parse(name: &str, contents: &str) -> Result<BTreeMap<String, Entry>, SecretsError> {
    let format = |line: usize, reason: &'static str| SecretsError::SourceFormat {
        origin: name.to_owned(),
        line,
        reason,
    };

    let mut entries = BTreeMap::new();
    for (index, raw) in contents.lines().enumerate() {
        let line = index + 1;
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        if raw.trim_start().starts_with('#') || raw.trim().is_empty() {
            continue;
        }

        let mut fields = raw.split('\t');
        let (Some(scope), Some(material), Some(expiry), None) =
            (fields.next(), fields.next(), fields.next(), fields.next())
        else {
            return Err(format(line, "应为 作用域<TAB>材料<TAB>到期时刻 三个字段"));
        };
        if scope.is_empty() {
            return Err(format(line, "作用域为空"));
        }
        if material.is_empty() {
            return Err(format(line, "材料为空"));
        }
        let Ok(expiry) = expiry.parse::<i64>() else {
            return Err(format(line, "到期时刻不是 Unix 毫秒整数"));
        };
        if entries.contains_key(scope) {
            return Err(format(line, "作用域重复"));
        }
        entries.insert(
            scope.to_owned(),
            Entry {
                material: material.as_bytes().to_vec(),
                expiry,
            },
        );
    }
    Ok(entries)
}

/// 环境变量凭据源：变量名由 [`env_var_name`] 从作用域推出，取值即材料。
///
/// 声称的到期时刻是 `now + ttl_ms`——环境变量本身不带到期信息，故 TTL 由本源配置给。
///
/// **每次取用现读环境**，故没有 `reload`：运维改掉变量的值，下一次 [`CredentialSource::fetch`]
/// 就取到新的。
pub struct EnvCredentialSource {
    name: String,
    prefix: String,
    ttl_ms: i64,
}

impl EnvCredentialSource {
    /// `prefix` 逐字加在变量名开头（建议以 `_` 结尾）；`ttl_ms` 是本源声称的有效期长度。
    pub fn new(prefix: impl Into<String>, ttl_ms: i64) -> Self {
        let prefix = prefix.into();
        Self {
            name: format!("env:{prefix}"),
            prefix,
            ttl_ms,
        }
    }
}

impl CredentialSource for EnvCredentialSource {
    fn name(&self) -> &str {
        &self.name
    }

    fn claimed_expiry(&self, scope: &str, now: i64) -> Result<i64, SecretsError> {
        // 取一次值：变量不存在与取值为空都在这里就判掉，故 `issue` **在签发前**拒，
        // 而不是先签出一枚空材料的凭据、等访问点才发现。
        self.value_for(scope)?;
        // 饱和加法：`now + ttl` 溢出会变成负数（一个过去的时刻），
        // 那会让本该长命的凭据一签出即失效——宁可钉在 `i64::MAX`。
        Ok(now.saturating_add(self.ttl_ms))
    }

    fn fetch(&self, scope: &str, _now: i64) -> Result<SecretMaterial, SecretsError> {
        let value = self.value_for(scope)?;
        // 取 `OsStr` 的编码字节：Unix 上即原始字节，Windows 上是 WTF-8 编码
        // （ASCII 密钥两种平台一致）。不要求 UTF-8——密钥不必是文本。
        Ok(SecretMaterial::new(value.as_encoded_bytes().to_vec()))
    }
}

impl EnvCredentialSource {
    /// 取该作用域对应的变量值。不存在 → `ScopeNotCovered`；存在但为空 →
    /// `EmptyMaterial`（空值不是一枚可用的凭据）。
    fn value_for(&self, scope: &str) -> Result<std::ffi::OsString, SecretsError> {
        match std::env::var_os(env_var_name(&self.prefix, scope)) {
            None => Err(not_covered(&self.name, scope)),
            Some(value) if value.is_empty() => Err(SecretsError::EmptyMaterial {
                origin: self.name.clone(),
                scope: scope.to_owned(),
            }),
            Some(value) => Ok(value),
        }
    }
}

/// 作用域 → 环境变量名：`prefix` + 作用域的**单射**编码。
///
/// 编码逐**字节**：ASCII 字母数字转大写原样留下；其余每个字节（含字面 `_`）写成
/// `_` + 两位大写十六进制。故 `repo/X` → `REPO_2FX`、`repo_a` → `REPO_5FA`。
///
/// **为什么必须单射**：环境变量名的字母表比作用域窄，若不转义，`repo/a-b`、
/// `repo/a_b`、`repo/a.b`、`repo/a/b` 会折成同一个变量名——两个不同的作用域共用一份
/// 材料，而**文件源**按作用域逐字匹配、视它们为四个不同作用域：两个源对「作用域是
/// 什么」的口径就会不一致，且环境变量源会**静默**把一份材料发给另一个作用域。
/// 转义后不会有这种塌缩；`_` 本身也被转义，故 `_2F` 不会被读成字面下划线接 `2F`。
///
/// 编码是单射（可逆）的：`_` 后必接两位十六进制，别处不出现 `_`。
/// 照片：`tests/source_env.rs` 的 `the_variable_name_is_the_prefix_plus_the_escaped_scope`
/// ——其中逐项断言 `repo/X` 与 `repo.X`、`repo_a` 与 `repo-a` 编码后**不相等**。
pub fn env_var_name(prefix: &str, scope: &str) -> String {
    let mut name = String::with_capacity(prefix.len() + scope.len());
    name.push_str(prefix);
    for byte in scope.bytes() {
        if byte.is_ascii_alphanumeric() {
            name.push(char::from(byte.to_ascii_uppercase()));
        } else {
            // 两位大写十六进制，定宽 → 解码无歧义。
            name.push('_');
            name.push(char::from_digit(u32::from(byte >> 4), 16).unwrap().to_ascii_uppercase());
            name.push(char::from_digit(u32::from(byte & 0x0f), 16).unwrap().to_ascii_uppercase());
        }
    }
    name
}

fn not_covered(source: &str, scope: &str) -> SecretsError {
    SecretsError::ScopeNotCovered {
        origin: source.to_owned(),
        scope: scope.to_owned(),
    }
}
