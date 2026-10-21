#![allow(dead_code)] // 同一 helper 会被多个 test target 编译，某些 target 未用到的项不算问题。

use std::fs::read_dir;
use std::path::Path;

/// `src/*.rs` 的**原文**，按文件名排序；不写文件白名单。
pub fn src_files() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files: Vec<(String, String)> = read_dir(dir)
        .expect("读取 src/ 失败")
        .map(|entry| entry.expect("读取 src/ 条目失败"))
        .filter(|entry| entry.file_type().expect("读取文件类型失败").is_file())
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.ends_with(".rs") {
                return None;
            }
            let text = std::fs::read_to_string(entry.path()).expect("读取源文件失败");
            Some((name, text))
        })
        .collect();
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

/// 对每个源文件应用 `code_only`，文件名与排序同 `src_files`。
pub fn code_files() -> Vec<(String, String)> {
    src_files()
        .into_iter()
        .map(|(name, text)| (name, code_only(&text)))
        .collect()
}

/// 去掉 `//`／`///`／`/* */` 注释与字符串字面量，保留换行。
/// 至少覆盖普通字符串与 `r#"…"#`／`r##"…"##` 原始字符串。
pub fn code_only(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut out = String::with_capacity(source.len());
    let mut i = 0;
    while i < bytes.len() {
        // 行注释：`//`、`///`、`//!` 都走到行尾。
        if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            while i < bytes.len() && bytes[i] != b'\n' {
                mask_push(&mut out, bytes[i]);
                i += 1;
            }
            continue;
        }
        // 块注释，支持嵌套。
        if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            let mut depth = 1usize;
            mask_push(&mut out, b' ');
            mask_push(&mut out, b' ');
            i += 2;
            while i < bytes.len() && depth > 0 {
                if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
                    depth += 1;
                    mask_push(&mut out, b' ');
                    mask_push(&mut out, b' ');
                    i += 2;
                } else if bytes[i] == b'*' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                    depth -= 1;
                    mask_push(&mut out, b' ');
                    mask_push(&mut out, b' ');
                    i += 2;
                } else {
                    mask_push(&mut out, bytes[i]);
                    i += 1;
                }
            }
            continue;
        }
        // 原始字符串：`r"…"`、`r#"…"#`、`r##"…"##`，以及 `br#"…"#`。
        let raw_start = if bytes[i] == b'r' {
            Some(i)
        } else if bytes[i] == b'b' && i + 1 < bytes.len() && bytes[i + 1] == b'r' {
            Some(i + 1)
        } else {
            None
        };
        if let Some(r) = raw_start {
            let mut j = r + 1;
            let mut hashes = 0usize;
            while j < bytes.len() && bytes[j] == b'#' {
                hashes += 1;
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'"' {
                while i <= j {
                    mask_push(&mut out, bytes[i]);
                    i += 1;
                }
                loop {
                    if i >= bytes.len() {
                        break;
                    }
                    if bytes[i] == b'"' {
                        let mut k = 0usize;
                        while k < hashes
                            && i + 1 + k < bytes.len()
                            && bytes[i + 1 + k] == b'#'
                        {
                            k += 1;
                        }
                        if k == hashes {
                            for _ in 0..=hashes {
                                mask_push(&mut out, bytes[i]);
                                i += 1;
                            }
                            break;
                        }
                    }
                    mask_push(&mut out, bytes[i]);
                    i += 1;
                }
                continue;
            }
        }
        // 普通字符串，支持反斜杠转义。
        if bytes[i] == b'"' {
            mask_push(&mut out, b'"');
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    mask_push(&mut out, bytes[i]);
                    i += 1;
                    if i < bytes.len() {
                        mask_push(&mut out, bytes[i]);
                        i += 1;
                    }
                } else if bytes[i] == b'"' {
                    mask_push(&mut out, b'"');
                    i += 1;
                    break;
                } else {
                    mask_push(&mut out, bytes[i]);
                    i += 1;
                }
            }
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn mask_push(out: &mut String, byte: u8) {
    if byte == b'\n' {
        out.push('\n');
    } else {
        out.push(' ');
    }
}

/// 返回 `needle` 之后第一个 `{` 与配对 `}` **之间**的体。
pub fn struct_block<'a>(text: &'a str, needle: &str) -> &'a str {
    block_after(text, needle)
}

/// 返回 `needle` 之后第一个 `{` 与配对 `}` **之间**的体。
pub fn impl_block<'a>(text: &'a str, needle: &str) -> &'a str {
    block_after(text, needle)
}

/// 同一文件里**全部**匹配的 impl 体。
pub fn impl_blocks<'a>(text: &'a str, needle: &str) -> Vec<&'a str> {
    let mut blocks = Vec::new();
    let mut search = 0;
    while let Some(relative) = text[search..].find(needle) {
        let start = search + relative;
        let Some(open_relative) = text[start..].find('{') else {
            break;
        };
        let open = start + open_relative;
        let close = matching_brace(text, open);
        blocks.push(&text[open + 1..close]);
        search = close + 1;
    }
    blocks
}

fn block_after<'a>(text: &'a str, needle: &str) -> &'a str {
    let start = text
        .find(needle)
        .unwrap_or_else(|| panic!("找不到 {needle}"));
    let open = start
        + text[start..]
            .find('{')
            .unwrap_or_else(|| panic!("{needle} 之后没有花括号"));
    let close = matching_brace(text, open);
    &text[open + 1..close]
}

fn matching_brace(text: &str, open: usize) -> usize {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return i;
                }
            }
            _ => {}
        }
        i += 1;
    }
    panic!("花括号不配对");
}

/// `struct` 体内的字段名，按源码出现顺序。
pub fn field_names(block: &str) -> Vec<&str> {
    let mut names = Vec::new();
    for part in block.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let Some(colon) = part.find(':') else {
            continue;
        };
        let prefix = part[..colon].trim();
        let Some(name) = prefix.split_whitespace().last() else {
            continue;
        };
        names.push(name);
    }
    names
}

/// `impl` 体内 `pub fn` 的名字，按源码出现顺序；`pub(crate) fn` 不算公开访问器。
pub fn public_fn_names<'a>(block: &'a str) -> Vec<&'a str> {
    const MARKER: &str = "pub fn ";
    let mut names = Vec::new();
    let mut search = 0;
    while let Some(relative) = block[search..].find(MARKER) {
        let start = search + relative + MARKER.len();
        let rest = &block[start..];
        let end = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(rest.len());
        names.push(&rest[..end]);
        search = start + end;
    }
    names
}

/// 按标识符边界匹配；标识符字符是 ASCII 字母／数字／`_`。
pub fn contains_token(text: &str, token: &str) -> bool {
    assert!(!token.is_empty(), "token 不能为空");
    let bytes = text.as_bytes();
    let mut search = 0;
    while let Some(relative) = text[search..].find(token) {
        let start = search + relative;
        let end = start + token.len();
        let before_ok = start == 0 || !is_identifier_byte(bytes[start - 1]);
        let after_ok = end == bytes.len() || !is_identifier_byte(bytes[end]);
        if before_ok && after_ok {
            return true;
        }
        search = start + 1;
    }
    false
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}
