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
//! 今天 `tests/` 下有**五处写语句**含该字面（实测 `authorize.rs:73`、`persist.rs:130`、`:660`、
//! `:668`、`crates/continuum-runtime/tests/tool_call.rs:409`），全部在射程之外，这是**刻意的**，
//! 不是漏了。**订正（2026-10-08，F 终审 m-3）**：原写「四处」——第五处是 F 的 `dfefb54` 新增的
//! `crates/continuum-runtime/tests/tool_call.rs`（同一个提交也新增了 `src/tool_cmd.rs`，那是
//! 下面那个 `.rs` 计数断言的消息里 105→106 的来源）。
//! **上面那五个行号是「定位」不是「钉死」**：以
//! `grep -rn 'INSERT INTO tool' crates/*/tests/` 的**当下输出为准**——它们在**别的文件**里，
//! 本文件的编辑不动它们，但 `persist.rs` / `authorize.rs` / `tool_call.rs` 的编辑会动
//! （那属通常的时机陈旧）。
//! **命令的写法也是 m-3 的订正处**：本条原写 `grep -rn '"INSERT INTO tool" crates/*/tests/`，
//! **单引号未闭合**（照抄进 shell 是语法错误），且内层那对 `"…"` 还会把语料换掉——写语句是
//! **多行字符串，只有开引号**，故带闭合引号的形态只命中「**把该字面整体写在一行里**」的地方
//! （本文件头这段注释、以及本文件的用例里那几个字面量），**看不见上面那五处写点**。
//! 故此处给出的是**不带内层引号**的形态。
//! （**这里不给命中行数**：本行自己的编辑也在往那个语料里加字，数与本文同寿——见下面
//! 「数的东西包含自己」那段。）
//!
//! **「写语句」与「不含本文件」这两个限定词都必要**（修复轮 3，评审判得对）：
//! **本文件之外**，`crates/*/tests/` 里另有**五处写语句**含该字面（`authorize.rs:73`、
//! `persist.rs:130`、`:660`、`:668`、`crates/continuum-runtime/tests/tool_call.rs:409`）——**本文件
//! 自己的注释与 `WRITE_STATEMENTS` 里的 needle 也是这个字面**，把它们一并算进来，「五处」按字面
//! 就不成立；而本段紧邻的下一段正在讲「本判据分不清代码、注释与字符串」，**在这种地方放松限定
//! 尤其误导**。（**订正 2026-10-08，F 终审 m-3**：原写「四处」——第五处见本段上文。）
//!
//! **这里刻意不给「本目录一共多少处」这个数**（修复轮 4，复核抓出，见下）。
//!
//! **⚠️ 数的东西包含自己 ⇒ 数不稳定（修复轮 4，复核抓出）**：
//! 本段初稿（修复轮 3）写的是「该字面在 `crates/*/tests/` 下**共出现 15 次**，其中 **11 次**
//! 就在本文件」。那个数**天生钉不住**——它数的是「**本文件自己也包含的那个字面**」，
//! 故**对本文件的任何一次编辑都会改变它**。实测逐版（复核按 commit 量的，我复核一致）：
//!
//! | commit | 本文件内 | 本目录合计 |
//! |---|---|---|
//! | `dceb767` | 9 | **13** |
//! | `8cc4fd2` | 11 | **15** |
//! | `28ea6bf` | 13 | **17** |
//!
//! **三版都在改「同一个数的表述」，而每次改都在挪动那个数本身**——上一版写 15，下一版
//! 就成了 17。故这里**不钉数**，把「有多少」交给读者当场跑：
//! `grep -rn 'INSERT INTO tool' crates/*/tests/`（**不带内层引号**；带上闭合的 `"…"` 会换掉
//! 语料，只命中把该字面整体写在一行里的地方，见上段）。
//! **订正（2026-10-08，F 终审 m-3）**：原给的命令是 `grep -rn '"INSERT INTO tool" crates/*/tests/`
//! ——**单引号未闭合**，不可执行。（**本条同样不给行数**：本段自己的编辑就在往那个语料里加字，
//! 正是下面那句「数不稳定」的活标本。）
//! **上表第三格按提交字节复量对不上**：按 `28ea6bf` 的字节数（不带内层引号的形态），本文件内
//! **12**、本目录合计 **16**（前两格 9/13 与 11/15 逐格吻合），表里写的是 13/17，**各多一**。
//! 成因未定，很可能是当时按**工作树**而非该提交的字节量的——**本段的教训正是这类数天生钉不住**，
//! 故只记读数、不改表。
//!
//! **这与下面那条自戒里「引本文件、且位于被引用行之前的行号会自失效」是同一族的两个面**：
//! 一个随编辑**平移**（行号），一个随编辑**增减**（自计数）。两条合起来是一条——
//! **不要在本文件里钉任何「本文件的编辑会改变它」的东西**，改成可复跑的定位。
//!
//! **订正（修复轮 2，评审 Minor）**：本段初稿写的是「`persist.rs:130`、`:599`、`:607`」——
//! 后两个是**陈旧的读数**：量它们的时候 `persist.rs` 还没有本轮新增的
//! `with_effect` 夹具等三段，行号整体下移了。旧值留在此处，是因为**成因值得记**：
//! 方法与数都对，只是**报的时候已经不是那个数了**（时机陈旧）。
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
//! 故本判据是**一个只看文本的必要不充分近似**：误报（上面两条）与逃逸（下一段）都来自
//! 「只看文本」这一个选择。剥注释要解析 Rust，同一个代价，**本阶段不做**（设计 §12）。
//!
//! **订正（修复轮 2，评审 Minor）**：本句初稿写的是「**一个下界，不是封闭判定**」。
//! 同段同时列了**误报**（过报）与**逃逸**（漏报）两种**相反方向**的偏差——
//! **一个既过报又漏报的谓词既不是下界也不是上界**，「下界」这个词在此处是错的。
//! （注意本文件另一处「这两个数是**下界**」指的是文件数/ crate 数的**数值下限**，
//! 那是另一个意思，对的，没改。）
//!
//! # 逃逸（换写法即躲过，清单**非穷尽**，别按封闭枚举读）
//!
//! 表名由变量拼出（`format!("INSERT INTO {t} …")`）、`include!` 进来的 SQL、DDL 变体、
//! 用 rusqlite 的 `prepare` 拼串……都躲得过字面匹配。**随手还能再举，故这里不数「N 种」**
//! ——能穷举的才数（同 `neutrality.rs` 文件头那条判据）。
//!
//! **已经照过面的一枚（修复轮 2 实测）**：**限定表名** `INSERT INTO main.tool`。
//! needle 是 `INSERT INTO tool`，而 `main.tool` **不是它的前缀**，故照样躲过——
//! 这正是我造「写点搬到别处」那个**有效**变异体时用的写法（H2，见 Task 1 报告 §修复轮 2）。
//! **不为它（以及同类的 `INSERT INTO "tool"`、`INSERT INTO temp.tool`、
//! `concat!("INSERT ", "INTO tool")`……）补 needle，是一处取舍，不是一条必然**：
//! **闭合一枚不等于闭合这一类**：补一枚只挡得住这一种拼法，换一种写法仍然躲得过，
//! 且每补一枚都多一分误报的面
//! （本判据分不清代码、注释与字符串）。故这里选的是**留下一枚已知逃逸并写明**，
//! **而不是声称这一类已闭合**。
//!
//! **一枚被删掉的错例（修复轮 3，评审判得对）**：本段初稿把 `WITH … INSERT INTO tool`
//! 列作逃逸——**它是错的**。`WITH x AS (…) INSERT INTO tool …` 里 `INSERT INTO tool`
//! **连续出现**，needle 照样命中，**它会被逮住**。已换成
//! `concat!("INSERT ", "INTO tool")`（实测：该串里 needle 不连续出现，确为逃逸）。
//!
//! **本文件只读源码文本**：不构造任何东西、不用夹具，故不走 `tests/common`。
//!
//! **本文件里出现的每个数与行号，都是以当时最终字节为准实测的，不是推算的。**
//! 这是给后来者的检查点：**数值会陈旧**——本文件初稿里的 103（`.rs` 数）与
//! `persist.rs:599/:607`（`tests/` 侧写点）都是这么过期的（修复轮 2 重测为 105 与
//! `:660/:668`）。改动本文件或它引用的文件之后，**回头把这些数重测一遍**。
//!
//! **⚠️ 但「重测一遍」只治得好一半（修复轮 4）**。本文件里踩过**两个面**，都属
//! 「**本文件自己的编辑会改变它**」这一族，而**它们不是重测能治的**：
//!
//! 1. **随编辑平移**：引**本文件**、且位于**被引用行之前**的行号会自失效
//!    （修复轮 3：我给一段注释写它自己引用的行号 `:268`，写完那一刻它就已经不是那个数了）；
//! 2. **随编辑增减**：**数一个本文件自己也包含的东西**（修复轮 4：数本目录里 `"INSERT INTO tool"`
//!    的处数——三版 13→15→17，每次改的正是那个数的表述本身）。
//!
//! **治法也不一样**：面 1 靠**写完再测一次**（并把旧值留原地）；面 2 **重测治不了**
//! ——换成**可复跑的定位**（把 `grep` 命令写出来，让读者当场跑）或**钉到不可变的坐标上**
//! （例如按 commit 记历史值）。**判据**：**在本文件里钉任何东西之前，先问「本文件的下一次
//! 编辑会不会改变它」**；会，就别钉那个形式。

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
        // **这一条保留 ＋ 写明，不删**（修复轮 3，协调者裁定；与 `tests/persist.rs` 里
        // `:466` 那条同法）。两句都要读，缺一不可：
        //
        // 1. **它不是被上一句严格蕴含的**——那条蕴含**依赖 `matches_in` 正确**
        //    （`norm.text == needle` ⇒ 恰命中一次，这个推理只在 `matches_in` 老实计数时成立）。
        //    删掉它等于**放弃「`matches_in` 回归」这一格的观察点**，即使那一格今天被
        //    下面的主体断言（`only_save_tool_writes_the_tool_table`）也覆盖着。
        // 2. **它确实是重复覆盖**：`matches_in` 一旦回归（例如凭空多报一次），
        //    主体断言同样会红——故它守的东西**不是本文件独有的判别力**。
        //
        // **「它是守卫」与「它不可替代」是两件事**：前者真，后者假（重复覆盖）。
        // 另注：变异体 K（不折制表符）下**这两条都会红**（上面那条 `norm.text == needle`
        // 先红；若走到下面那条命中数，它也是 0）⇒ **K 是本循环里这**两条**断言的合取照片，
        // 不是第一条自己的**（这正是「独立照片」判据要求「B 之后也全过」的由来）。
        //
        // **这里刻意不写这两个断言的行号**（修复轮 4）。来历：修复轮 3 我在这里写过
        // `:263 ∧ :284`（更早还估过 `:268`），**而修复轮 4 往本文件头部加了几十行文档，
        // 两条断言当场平移到 `:297 ∧ :318`**——引用与被引用者在**同一个文件**里，
        // 且引用者在上，**本文件任何一次头部编辑都会让它自失效**。
        // 故改用**结构性定位**（「上面那条 `norm.text == needle`」「下面那条命中数」）：
        // 判据是**钉任何东西之前先问「本文件的下一次编辑会不会改变它」**，
        // 会，就别钉那个形式（同一族两面的完整说明见文件头「数的东西包含自己」一段）。
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
/// 三条：扫到的 crate 数、`src/` 下 `.rs` 文件数各有一个**下界**（实测分别是 17 与 105），
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
        "只读到 {} 个 `.rs` 文件：源码树没被读到。实测 106 个（F 终审 m-3 重测；上一版写 105，
         那之后 F 的 `dfefb54` 新增了 `crates/continuum-runtime/src/tool_cmd.rs`；初稿写 103，
         那之后 `9254108` 新增了 `crates/continuum-runtime/src/{{error,tool_call}}.rs` 两个文件
         ——同一个「时机陈旧」）。**不跳过**。",
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
