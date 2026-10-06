//! `tool` 表的**唯一生产写点**守卫（设计 §5.2、§3.3）。
//!
//! **挡的是哪条路径。** 在 `save_tool` 之外再添一个往 `tool` 表写的**生产**路径——别的 crate
//! 里的装载器、驱动里的快捷写、`UPDATE tool` 改一行、`DELETE FROM tool` 删一行。
//! 这条路是**全绿**的：新写点自己编得过，`save_tool` 的用例一条不红，`tests/persist.rs` 的
//! 登记期不变量用例也照旧全过——因为**不变量只长在 `save_tool` 里**，别处的写入压根不经过它。
//! 结果：「不变量关在唯一入口上」这条**全部理由**被静默废掉，而一片全绿。
//! 故判据取在**模块面**（`crates/*/src/` 下的源码文本），不取在行为——行为面挡不住它，
//! 那条路径的每一处行为都正常。
//!
//! **为什么射程到 `src/` 为止，不到 `tests/`。** 本守卫判的是「**生产**写点只有一个」。
//! `tests/` 下的写点是有意存在的，且**必须**存在：`insert_raw` 要绕过 `save_tool` 写进
//! 「能力表不覆盖的历史行」，`tests/persist.rs` 的
//! `a_legacy_row_that_violates_the_invariant_is_still_readable` 与
//! `an_unknown_enum_column_value_is_rejected` 正是靠它造语料，`authorize.rs` 的
//! `insert_bad_capabilities` 同理。把 `tests/` 也算进射程，本守卫就与那几条用例的**目的**打架。
//! 今天 `tests/` 下有四处 `"INSERT INTO tool"`（`authorize.rs:73`、`persist.rs:130`、`:599`、
//! `:607`），全部在射程之外，这是**刻意的**，不是漏了。
//!
//! **语料是怎么来的**：由 `crates/` 目录**枚举**，不是手写一份 crate 名单——新加一个 crate
//! 时它自动进射程（同 `dependency_direction.rs` 用 `workspace_crates()` 枚举 crate，而非
//! 在用例里写死一串名字）。**手写名单的守卫会在「新 crate 里添一个写点」时照样绿**，
//! 而那正是本守卫要逮的形状。
//!
//! # 哪条规矩会往语料里加字（零命中类判据的前置问题）
//!
//! 本判据只在源码文本里找字面拼法，故**分不清代码、注释与字符串**。会往这份语料里加字的
//! 本仓规矩有两条，都不是缺陷：
//!
//! 1. **文档规矩本身**：本仓要求文档详实、准确，且「订正时把错误说法的来历留在原地」，
//!    于是注释里**频繁原文引用**标识符与代码片段。一句照着 `save_tool` 写的注释
//!    （「这里是唯一的 `INSERT INTO tool`」）会**误报**。
//! 2. **单元测试写在 `src/` 里**：`#[cfg(test)] mod tests` 是 `src/` 下的合法内容
//!    （本 crate 的 `src/persist.rs` 末尾就有一个），其中的 `#[test]` 与测试辅助函数
//!    也**可以**正当地直接写 `tool` 表（它们是测试代码，不是生产写点）。今天那里没有写语句，
//!    但添一条是合规的，届时会误报。
//!
//! 故本判据是**一个下界，不是封闭判定**：误报（上面两条）与逃逸（下一段）都来自「只看文本」
//! 这一个选择。剥注释要解析 Rust，与下面的上界同一个代价，**本阶段不做**（设计 §12）。
//!
//! # 逃逸（换写法即躲过，清单**非穷尽**，别按封闭枚举读）
//!
//! 表名由变量拼出（`format!("INSERT INTO {t} …")`）、`include!` 进来的 SQL、DDL 变体、
//! 用 rusqlite 的 `prepare` 拼串……都躲得过字面匹配。**随手还能再举，故这里不数「N 种」**
//! ——能穷举的才数（同 `neutrality.rs` 文件头那条判据）。
//!
//! **本文件只读源码文本**：不构造任何东西、不用夹具，故不走 `tests/common`。

use std::fs;
use std::path::{Path, PathBuf};

/// 仓库根（`crates/` 的父目录）。
///
/// **按 `CARGO_MANIFEST_DIR` 定，不按进程 CWD**：按相对路径扫 `crates/` 的守卫在仓库根下跑
/// 是绿的，换个目录跑就**读不到东西**——而「什么都没读到」正是下面那个正控制要逮的形状。
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// 报错信息里用的短路径，相对**仓库根**（`crates/continuum-capability/src/persist.rs`）。
///
/// **不相对本 crate 根**：本守卫跨 crate，相对本 crate 根会印出
/// `../continuum-provider/src/…` 这种读起来不像路径的串。
fn shown(repo: &Path, path: &Path) -> String {
    path.strip_prefix(repo).unwrap_or(path).display().to_string()
}

/// 递归收集 `dir` 下全部 `.rs`。
///
/// **读不到的目录直接炸，不跳过**：`src/` 下有一份读不动的文件本身就是一处发现
/// （权限不对、或树被换掉），静默跳过只会让守卫变窄——而变窄之后它仍然是绿的。
fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| {
        panic!(
            "列不出目录 {}：{e}。**不跳过**——`src/` 下有读不动的目录本身就是一处发现。",
            dir.display()
        )
    });
    for entry in entries {
        let entry = entry.unwrap_or_else(|e| {
            panic!("列目录 {} 时出错：{e}。**不跳过**。", dir.display())
        });
        let path = entry.path();
        // `Path::is_dir` 跟随符号链接，故软链到目录也照样递归下去。
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// 读一份源文件。**读不动或不是 UTF-8 都直接炸，不跳过**：一份读不动的源文件恰恰是守卫
/// 最该报的东西——静默跳过等于把射程悄悄缩小。
fn read_source(path: &Path) -> String {
    let bytes = fs::read(path).unwrap_or_else(|e| {
        panic!(
            "读不到源文件 {}：{e}。**不跳过**——`src/` 下有读不动的文件本身就是一处发现。",
            shown(&repo_root(), path)
        )
    });
    String::from_utf8(bytes).unwrap_or_else(|e| {
        let at = e.utf8_error().valid_up_to();
        panic!(
            "源文件 {} 不是 UTF-8（第 {at} 个字节起非法）。**不跳过**——不跳过才谈得上「读全了」。",
            shown(&repo_root(), path)
        )
    })
}

/// 枚举 `crates/` 下的每个 crate，递归读其 `src/` 下全部 `.rs`；路径排序（消息里的次序
/// 因此确定，不看文件系统心情）。
fn read_all_sources() -> Vec<(PathBuf, String)> {
    let repo = repo_root();
    let crates = repo.join("crates");
    let entries = fs::read_dir(&crates).unwrap_or_else(|e| {
        panic!(
            "列不出 {}：{e}。**不跳过**——读不到源码树的守卫会永远绿。",
            crates.display()
        )
    });

    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.unwrap_or_else(|e| {
            panic!("列目录 {} 时出错：{e}。**不跳过**。", crates.display())
        });
        let src = entry.path().join("src");
        if src.is_dir() {
            collect_rs(&src, &mut files);
        }
    }
    files.sort();

    files
        .into_iter()
        .map(|path| {
            let source = read_source(&path);
            (path, source)
        })
        .collect()
}

/// 折叠连续空白之后的文本，**以及每个字节在源文件里的行号**。
///
/// 行号必须与折叠**同批**记录：折完之后换行符已经没了，回头再数行数无从数起，而断言信息里
/// 要报出文件与行号——红的时候读不出是哪一处，这条守卫就只剩一个「红」。
struct Normalized {
    text: String,
    /// 与 `text` 的字节一一对应；`line_of[i]` 是 `text` 第 `i` 个字节在源文件里的 1 起行号。
    line_of: Vec<usize>,
}

/// 把连续空白折成一个空格。
///
/// **为什么归一化**：`tx.execute("INSERT INTO tool` 与
/// `tx.execute(\n            "INSERT INTO tool` 是同一处写，只差换行与缩进。不归一化，
/// 换个缩进就能躲过匹配——留一种不归一就是没关严（同 `neutrality.rs` 的裁决 C7）。
///
/// 「空白」取 `char::is_whitespace` 的**全集**，比举例常见的三种多。方向是**只放宽守卫**：
/// 折得更多只会命中更多，不会漏。用一个朴素状态机而不是正则：本仓不为此新增依赖。
/// 折出来的那个空格记的是**这段空白起始处**的行号——匹配若从空白之前开始，报到的是读者想看的那一行。
fn normalize(source: &str) -> Normalized {
    let mut text = String::with_capacity(source.len());
    let mut line_of = Vec::with_capacity(source.len());
    let mut line = 1usize;
    let mut in_whitespace = false;

    for ch in source.chars() {
        if ch.is_whitespace() {
            if !in_whitespace {
                in_whitespace = true;
                text.push(' ');
                line_of.push(line);
            }
            if ch == '\n' {
                line += 1;
            }
        } else {
            in_whitespace = false;
            text.push(ch);
            for _ in 0..ch.len_utf8() {
                line_of.push(line);
            }
        }
    }

    debug_assert_eq!(text.len(), line_of.len());
    Normalized { text, line_of }
}

/// `needle` 在归一化文本里的全部出现位置（字节下标）。
fn matches_in(norm: &Normalized, needle: &str) -> Vec<usize> {
    let mut hits = Vec::new();
    let mut from = 0;
    while let Some(offset) = norm.text[from..].find(needle) {
        let at = from + offset;
        hits.push(at);
        from = at + needle.len();
    }
    hits
}

/// **归一化自身的照片。** 没有这一条，归一化写错了也没人知道——而下面两条判定完全建立在
/// 归一化之上，归一化错了只会**静默地**变窄或变宽，没有别的东西会告诉你。
///
/// 四个输入各取一种空白变体，第四项是已折好的本形，摆进来是为了让「归一化在本形上幂等」
/// 这件事有照片，而不只是一句说法。
///
/// **空白必须落在 needle 内部**（订正：初稿把 `\t` 与换行放在 needle **之前**的
/// `tx.execute(` 与 `"` 之间，那几版**对归一化不敏感**——折与不折，`INSERT INTO tool`
/// 都照样连着一整段，于是三条变体全都不承重，等于没测。判据：一条对某性质的照片，
/// 必须在**那个性质被破坏时**红；把被测的空白挪到 needle 之外，照片就失效了）。
#[test]
fn the_guard_folds_whitespace_before_matching() {
    let needle = "INSERT INTO tool";
    let variants = [
        ("本形（已折好）", "INSERT INTO tool"),
        ("两个空格", "INSERT  INTO tool"),
        ("制表符", "INSERT\tINTO tool"),
        ("换行加缩进", "INSERT\n        INTO tool"),
    ];

    for (name, variant) in variants {
        let norm = normalize(variant);
        assert_eq!(
            norm.text, needle,
            "{name}：归一化没把 {variant:?} 折成 {needle:?}，得到 {:?}",
            norm.text
        );
        assert_eq!(
            matches_in(&norm, needle).len(),
            1,
            "{name}：折叠后的文本是 {:?}，按 {needle:?} 找却命中 {} 次",
            norm.text,
            matches_in(&norm, needle).len()
        );
    }

    // 行号与折叠**同批**记录这件事的照片：换行加缩进那一版，命中的是**第二行**。
    // 若哪天行号改成事后另数，这条会红（那时报出的行号会指到 `tx.execute(` 那一行）。
    let norm = normalize("tx.execute(\n            \"INSERT INTO tool");
    let at = matches_in(&norm, needle)[0];
    assert_eq!(
        norm.line_of[at], 2,
        "命中的应是源文件的第 2 行，实际报第 {} 行",
        norm.line_of[at]
    );
}

/// **守卫自身的正控制。** 没有这一条，一个「什么都没读到」的守卫会永远绿——这正是
/// 「守卫须两侧都钉」里缺的那一侧。
///
/// 三条：扫到的 crate 数、`src/` 下 `.rs` 文件数各有一个**下界**（今天分别是 17 与 103），
/// 且拼起来的文本里有 `CREATE TABLE tool`。前两条挡「一个文件都没读到 / 只读到本 crate」，
/// 第三条挡「读到了一堆文件但不是这棵树」。
///
/// **这两个数是下界，不是「这个数」**：树的规模不是固定的（加 crate、加文件都正常），
/// 故按「能穷举的才数」的判据，这里只断下界。取值刻意远低于今天的实测值——它的职责是逮
/// 「几乎什么都没读到」，不是逮「少了一个文件」。
#[test]
fn the_guard_sees_the_source_tree() {
    let sources = read_all_sources();

    let mut crates: Vec<&Path> = sources
        .iter()
        .filter_map(|(p, _)| p.parent().and_then(|src| src.parent()))
        .collect();
    crates.sort();
    crates.dedup();

    assert!(
        crates.len() >= 10,
        "只扫到 {} 个 crate 的 `src/`（{:?}）：源码树没被读到。**一个读不到东西的守卫会永远绿**，\
         故这条先红。今天实测 17 个。",
        crates.len(),
        crates
    );
    assert!(
        sources.len() >= 50,
        "只读到 {} 个 `.rs` 文件：源码树没被读到。今天实测 103 个。**不跳过**。",
        sources.len()
    );

    let joined = sources
        .iter()
        .map(|(_, source)| source.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("CREATE TABLE tool"),
        "读到了 {} 个文件，但拼起来找不到 `CREATE TABLE tool`——读到的不是这棵树。",
        sources.len()
    );
    // 第二条锚：本守卫的**期望位置**所指的那个函数确实在语料里（`persist.rs` 的 `save_tool`）。
    // 少了它，把 `save_tool` 改名之后本文件仍在断言一个不存在的函数名，而它照样绿。
    assert!(
        joined.contains("pub fn save_tool("),
        "语料里找不到 `pub fn save_tool(`——本守卫断言的期望位置（`{}`）已经不存在了，\
         该处守卫要跟着改。",
        EXPECTED_WRITER
    );
}

/// 唯一允许的生产写点。跨 crate 的**绝对**口径（相对仓库根），故写成常量而不是拼出来的。
const EXPECTED_WRITER: &str = "crates/continuum-capability/src/persist.rs";

/// 往 `tool` 表写的语句形状（设计 §3.3：本表只有 `INSERT`，没有 `UPDATE` / `DELETE` 路径——
/// 故后两枚是**为将来**留的，今天零命中）。
///
/// `INSERT OR REPLACE INTO tool` 与 `REPLACE INTO tool` 两枚会命中**同一行**，故下面的
/// 判定按 `(文件, 行)` **去重**，不按 needle 计数。
const WRITE_STATEMENTS: &[&str] = &[
    "INSERT INTO tool",
    "INSERT OR REPLACE INTO tool",
    "REPLACE INTO tool",
    "UPDATE tool",
    "DELETE FROM tool",
];

/// `tool` 表的**生产**写点只有一处：`crates/continuum-capability/src/persist.rs` 的 `save_tool`。
///
/// 判据是**对行**的：把命中收成 `(文件, 行)` 去重后的清单，两条断言各钉一侧——
///
/// - **文件集**恰好是 `{EXPECTED_WRITER}`。缺了它，「唯一的写点搬到了别的 crate」会漏
///   （那时总数仍是 1）；
/// - **总数**恰好 1。缺了它，「同一个文件里多写一处」会漏。
///
/// 两条都有名字可叫的变异体（见 Task 1 报告 §修复轮 1）。
#[test]
fn only_save_tool_writes_the_tool_table() {
    let sources = read_all_sources();
    let repo = repo_root();

    // 排在主体断言**之前**，且主体断言在它之下仍是真的：语料为空时这条先红，
    // 否则下面那条会因「一处都没找到」而**恒真、恒绿**。
    assert!(
        !sources.is_empty(),
        "没读到任何 `.rs`：{} 下的语料是空的，下面那条断言因此恒真、恒绿。",
        repo.join("crates").display()
    );

    let mut deduped: Vec<(String, usize)> = Vec::new();
    for (path, source) in &sources {
        let norm = normalize(source);
        for needle in WRITE_STATEMENTS {
            for at in matches_in(&norm, needle) {
                deduped.push((shown(&repo, path), norm.line_of[at]));
            }
        }
    }

    // 同一行被两枚 needle 命中时只算一处（`INSERT OR REPLACE INTO tool` 的两种读法）：
    // 判定按 `(文件, 行)` 去重，不按 needle 计数。
    deduped.sort();
    deduped.dedup();

    let rendered = if deduped.is_empty() {
        "（一处都没有）".to_owned()
    } else {
        deduped
            .iter()
            .map(|(file, line)| format!("{file}:{line}"))
            .collect::<Vec<_>>()
            .join("\n  ")
    };

    let mut files: Vec<String> = deduped.iter().map(|(file, _)| file.clone()).collect();
    files.sort();
    files.dedup();

    assert_eq!(
        files,
        vec![EXPECTED_WRITER.to_owned()],
        "`tool` 表的生产写点出现在这些文件里：\n  {rendered}\n\
         应当只有 `{EXPECTED_WRITER}` 一处（`save_tool` 内）。\n\
         多出来的写点会**静默废掉**登记期不变量——它只长在 `save_tool` 里，别处的写入不经过它。\n\
         若这一步红了而你不认为有新写点：**先查归一化与注释**（本判据不分代码/注释/字符串），\
         见文件头「哪条规矩会往语料里加字」。",
    );
    assert_eq!(
        deduped.len(),
        1,
        "`tool` 表有 {} 处生产写点，应恰一处：\n  {rendered}\n\
         同一个文件里再添一处同样静默废掉不变量，故这一条与上面那条是**两侧**，缺一不可。",
        deduped.len()
    );
}
