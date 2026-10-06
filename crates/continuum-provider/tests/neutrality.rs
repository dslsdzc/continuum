//! 模块面中立性守卫（设计 §4.1 末段、§4.2 形态 4）。
//!
//! **挡的是哪条路径。** 把一份具体适配器**写进 `continuum-provider` 自己内部**
//! （例如新增 `src/deepseek.rs`，里面 `struct DeepSeek;` 加
//! `impl ModelProvider for DeepSeek`）。这条路是**全绿**的：它不引用任何外部 crate 的
//! 实现类型，故 §4.2 形态 2 的机制（未定义的类型名 → 编译不过）不触发；`ALLOWED` 一字不改；
//! `core_and_persist_do_not_depend_on_provider` 照绿；`src/lib.rs` 开头那句
//! 「本 crate 只定义 trait 与登记表，不含实现」**只是文档**，今天没有别的东西钉它。
//! 结果：中立 crate 里躺着一份实现，而一片全绿。故判据取在**模块面**（`src/` 下的源码文本），
//! 不取在行为——行为面挡不住它，因为那条路径的每一处行为都正常。
//!
//! **归一化关掉了什么**（裁决 C7，2026-10-05；设计 §4.1 末段）。匹配前把**连续空白**
//! （空格 / `\t` / `\n`）折成一个空格再比对。于是 `impl  ModelProvider for`（多一个空格）、
//! `impl\tModelProvider for`、`impl\nModelProvider for` 与 `impl ModelProvider for`
//! **是同一个字面拼法**，四种写法一视同仁，**不再是逃逸面**——留一种不归一就是没关严。
//!
//! **空白归一化之后仍逃逸的三种**（它们**不是**同一个拼法，而是换了写法）：
//!
//! 1. **全限定 trait 路径**：`impl crate::model::ModelProvider for DeepSeek`；
//! 2. **`use ... as` 别名后实现**：`use crate::model::ModelProvider as Mp;` 再 `impl Mp for ...`；
//! 3. **把实现 `include!` 进来**：`include!("../somewhere/deepseek.rs")`。
//!
//! 三种都照样全绿。故本守卫是**一个下界，不是封闭判定**；这三种写法**没有照片**，落在评审。
//! 要把上界也钉住，得让守卫**解析 trait 路径**（例如用 `syn` 解析 crate 内所有 `impl` 的 trait
//! 路径再比对），而不是匹配文本——那会新增一个 dev 依赖，**本阶段不做**（设计 §12 第 21 条）。
//!
//! **另一个方向的偏差，也写明**：匹配的既然是**文本拼法**，它就不区分那处文本是代码、注释
//! 还是字符串——归一化只折空白，**不剥注释、不剥字符串**。故注释里照抄一句这个拼法会**误报**。
//! 这是「下界」的另一面：松的一面（三种换写法）与紧的一面（注释/字符串照抄）都来自「只看文本」
//! 这一个选择。剥注释要解析 Rust，与上界同一个代价，**本阶段不做**。
//! （这一条有照片：下面 `the_guard_folds_whitespace_before_matching` 末尾断言注释**原样留下**，
//! 若哪天归一化改成剥注释，那条会红。）
//!
//! **本文件只读源码文本**：不构造任何东西、不用夹具，故不走 `tests/common`。
//!
//! **用例次序**：`the_guard_folds_whitespace_before_matching` 写在最前——它是归一化自身的照片，
//! 而下面的判定完全建立在归一化之上；归一化若错了，判定只会**静默地**变窄或变宽，没有别的东西
//! 会告诉你。

use std::fs;
use std::path::{Path, PathBuf};

/// 被测源码树的根。
///
/// **按 `CARGO_MANIFEST_DIR` 定，不按进程 CWD**：按相对路径读 `crates/continuum-provider/src`
/// 的守卫在 workspace 根下跑是绿的，换个目录跑就**读不到东西**——而「什么都没读到」正是
/// 下面那个正控制要逮的形状。`CARGO_MANIFEST_DIR` 由 cargo 编译期注入，恒等于本 crate 目录。
fn source_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// 报错信息里用的短路径（`src/probe.rs`），仍是**唯一可辨**的：相对本 crate 根。
fn shown(path: &Path) -> String {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    path.strip_prefix(manifest)
        .unwrap_or(path)
        .display()
        .to_string()
}

/// 递归收集 `dir` 下全部 `.rs`。
///
/// **读不到的目录直接炸，不跳过**：`src/` 下有一份读不动的文件，本身就是一处发现
/// （要么权限不对、要么树被换掉），把它静默跳过只会让守卫变窄。
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
        // `Path::is_dir` 会跟随符号链接，故软链到目录也照样递归下去。
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// 读一份源文件。
///
/// **读不动或不是 UTF-8 都直接炸，不跳过**（派单给的诚实选项里取「失败要响」）：
/// 一份读不动/编不出来的源文件恰恰是守卫最该报的东西——静默跳过等于把守卫的射程悄悄缩小，
/// 而缩小之后它仍然是绿的。
fn read_source(path: &Path) -> String {
    let bytes = fs::read(path).unwrap_or_else(|e| {
        panic!(
            "读不到源文件 {}：{e}。**不跳过**——`src/` 下有读不动的文件本身就是一处发现。",
            shown(path)
        )
    });
    String::from_utf8(bytes).unwrap_or_else(|e| {
        let at = e.utf8_error().valid_up_to();
        panic!(
            "源文件 {} 不是 UTF-8（第 {at} 个字节起非法）。**不跳过**——不跳过才谈得上「读全了」。",
            shown(path)
        )
    })
}

/// 递归读 `src/` 下全部 `.rs`，路径排序（消息里的次序因此是确定的，不看文件系统心情）。
fn read_all_sources() -> Vec<(PathBuf, String)> {
    let root = source_root();
    assert!(
        root.is_dir(),
        "源码树 {} 不是一个目录。**不跳过**——读不到源码树的守卫会永远绿。",
        root.display()
    );
    let mut files = Vec::new();
    collect_rs(&root, &mut files);
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
/// 行号必须与折叠**同批**记录：折完之后换行符已经没了，回头再数行数无从数起，
/// 而断言信息里要报出文件名与行号（派单）——红的时候读不出是哪一处，这条守卫就只剩一个「红」。
struct Normalized {
    text: String,
    /// 与 `text` 的字节一一对应；`line_of[i]` 是 `text` 第 `i` 个字节在源文件里的 1 起行号。
    line_of: Vec<usize>,
}

/// 把连续空白（空格 / `\t` / `\n`）折成一个空格。
///
/// 用一个朴素状态机而不是正则：本仓不为此新增依赖（派单：本 task 只用 `std`）。
/// 折出来的那个空格记的是**这段空白起始处**的行号——匹配若从空白之前开始，报到的是读者想看的
/// 那一行。
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

/// **归一化自身的照片。** 没有这一条，归一化写错了也没人知道——而它正是裁决 C7 要的那一件事。
///
/// 四个输入：前三个是三种空白变体（两个空格 / `\t` / `\n`），第四个是已折好的本形。
/// 第四项是设计 §4.1 末段「**这四种是同一个字面拼法**」里的那第一种，摆进来，
/// 归一化在本形上**幂等**这件事才有照片，而不只是一句说法。
///
/// 只断言 `norm.text` 相等**不够**：判定真正用的是「归一化之后找得到 needle」，
/// 故每个输入同时断言 `matches_in` 恰好命中一次。
#[test]
fn the_guard_folds_whitespace_before_matching() {
    let needle = "impl ModelProvider for";
    let variants = [
        ("本形（已折好）", "impl ModelProvider for"),
        ("两个空格", "impl  ModelProvider for"),
        ("制表符", "impl\tModelProvider for"),
        ("换行", "impl\nModelProvider for"),
    ];

    for (name, variant) in variants {
        let norm = normalize(variant);
        assert_eq!(
            norm.text, needle,
            "{name}：归一化没有把 {variant:?} 折成 {needle:?}，得到 {:?}",
            norm.text
        );
        let hits = matches_in(&norm, needle);
        assert_eq!(
            hits.len(),
            1,
            "{name}：折叠后的文本是 {:?}，按 {needle:?} 找却命中 {} 次",
            norm.text,
            hits.len()
        );
    }

    // **归一化不吞注释**（文件头「另一个方向的偏差」那一句的照片）。归一化只折空白，
    // 故注释里照抄这个拼法**照样命中**，本守卫会误报——这是「只看文本」的代价，
    // 与三种换写法的逃逸是同一个选择的两面。若哪天归一化改成剥注释，这条会红。
    let commented = normalize("// 不得写 impl  ModelProvider for\n");
    assert_eq!(
        commented.text, "// 不得写 impl ModelProvider for ",
        "归一化把注释吞了：本守卫的射程变了，文件头那句话要一并改"
    );
    assert_eq!(
        matches_in(&commented, needle).len(),
        1,
        "注释里的拼法没有命中——归一化已经不是「只折空白」了"
    );
}

/// **守卫自身的正控制。** 没有这一条，一个「什么都没读到」的守卫会永远绿——
/// 这正是「守卫须两侧都钉」里缺的那一侧。
///
/// 两条：**读到至少 4 个 `.rs`**（今天 `src/` 下有 `lib.rs` / `model.rs` / `tool.rs` /
/// `connector.rs` / `registry.rs` 五份），**且拼起来的文本里有 `trait ModelProvider`**。
/// 第一条挡「一个文件都没读到」，第二条挡「读到了 4 个文件但不是这棵树」。
#[test]
fn the_guard_sees_the_source_tree() {
    let sources = read_all_sources();

    assert!(
        sources.len() >= 4,
        "只读到 {} 个 `.rs` 文件（{:?}）：源码树没被读到。**一个读不到东西的守卫会永远绿**，\
         故这条先红。",
        sources.len(),
        sources.iter().map(|(p, _)| shown(p)).collect::<Vec<_>>()
    );

    let joined = sources
        .iter()
        .map(|(_, source)| source.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("trait ModelProvider"),
        "读到了 {} 个文件，但拼起来找不到 `trait ModelProvider`——读到的不是这棵树。\
         文件：{:?}",
        sources.len(),
        sources.iter().map(|(p, _)| shown(p)).collect::<Vec<_>>()
    );
}

/// 中立 crate 的 `src/` 里**不出现**三个字面拼法（设计 §11、§4.1 末段）。
///
/// 判据是**模块面**：折叠空白之后的文本里找 `impl ModelProvider for` /
/// `impl ToolProvider for` / `impl Connector for`。红的时候列出**文件名与行号**。
///
/// **本断言是一个下界，不是封闭判定**——三种换写法（全限定路径 / `use ... as` 别名 /
/// `include!`）仍逃逸，理由与出处见文件头。
#[test]
fn the_neutral_crate_contains_no_implementation() {
    let needles = [
        "impl ModelProvider for",
        "impl ToolProvider for",
        "impl Connector for",
    ];

    let sources = read_all_sources();

    // 这条排在本体断言**之前**，且本体断言在它之下仍是一条**真守卫**：
    // 变异体 M（临时新增 `src/probe.rs`，内含两个空格的 `impl  ModelProvider for Probe`
    // 与 `mod probe;`）下，读过的东西非空、这条照过，而下面那条红。
    assert!(
        !sources.is_empty(),
        "没读到任何 `.rs`：{} 下的语料是空的，下面那条断言因此恒真、恒绿。",
        source_root().display()
    );

    let mut hits: Vec<String> = Vec::new();
    for (path, source) in &sources {
        let norm = normalize(source);
        for needle in needles {
            for at in matches_in(&norm, needle) {
                hits.push(format!("{}:{}: `{needle}`", shown(path), norm.line_of[at]));
            }
        }
    }

    assert!(
        hits.is_empty(),
        "中立 crate `continuum-provider` 的 `src/` 下出现了 trait 实现，共 {} 处：\n  {}\n\
         本 crate 只定义 trait 与登记表，不含实现（§346、设计 §4.1）。\n\
         判定已折叠空白，故 `impl  …` / `impl\\t…` / `impl\\n…` 与本形是同一个拼法。\n\
         若这一步红了而你不认为有实现：**先查归一化，不要去改被匹配的拼法**。",
        hits.len(),
        hits.join("\n  ")
    );
}
