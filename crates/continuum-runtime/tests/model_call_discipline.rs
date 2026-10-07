//! 模块面守卫三条：用词纪律（设计 §1.3）、不命名适配器实现（设计 §10.3）、
//! 零能力／效应／凭据引用（设计 §2.2 第一条）。
//!
//! **语料只有一份**：`crates/continuum-runtime/src/model_call.rs` 的源码文本。
//! 本文件不构造任何东西、不用夹具，故不走 `tests/common`。
//!
//! **为什么判据取在文本上。** 这三条判据没有运行期形态可拍：混称是**文本**上的事，
//! 不是行为上的事。一个模块「借用了工具调用路径的名字」，它的每一次调用都可以是正常的，
//! 行为面因此一条也挡不住。故判据只能取在**模块面的源码文本**上。
//!
//! # 一、用词纪律（设计 §1.3）
//!
//! 裁决原文是「G 不得借用 F 的名字」。落成判据：`model_call.rs` 的源码文本里不出现
//! [`TOOL_PATH_NAMES`] 那五个标识符。**红的条件里包含注释**——在 `model_call.rs` 里写一句
//! 「本模块不碰 `ToolProvider`」即红。**这是刻意的**：判据是**拼法**，不是「用了它」，
//! 设计 §1.3 要的正是模块里不出现这个拼法。**为什么值得有这条**：混称在本项目出现过
//! （出处：设计 §1.3 引的裁决 §一.1 与 `p3bcdf-followups.md` §六.4——前者记「四份设计混称过」，
//! 后者记协调者的派单措辞是漂移源；**本文件照设计引，未逐字核那两份原文**），
//! 而「同一件事两个词汇表」在本项目一贯判为 Critical；守卫逮的是最便宜的那一类复发。
//!
//! # 二、不命名适配器实现（设计 §10.3）
//!
//! 同法，不出现 [`ADAPTER_NAMES`]。**本守卫是下界，上界那条边由
//! `crates/continuum-runtime/tests/dependency_direction.rs` 兜住**：若 G 真想持有一个
//! 实现类型，它必须依赖那个 crate，而该文件的 `continuum-runtime` 条目
//! （实测 `:172-189`，crate 名在 `:173`；含 `continuum-capability` / `continuum-effect` /
//! `continuum-secrets`，**不含**任何 `continuum-adapter-*`）与逐对 `assert_eq!`
//! （用例 `every_crate_depends_only_on_its_allowed_set` 在 `:255`，那条断言在 `:285-289`）
//! 会因为多出那条边而红。
//!
//! **行号按实测写，不照抄设计**：设计 §10.3 引的是 `:246` 与 `:276-281`，
//! 而 2026-10-08 实测是 `:255` 与 `:285-289`。两枚都不一致，本文件取实测值
//! （引用行号须实测）；设计引的那两枚为何不同，在这里判不了（可能是更早版本的读数），
//! **故只记差异，不改设计里的行**。
//!
//! # 三、零能力／效应／凭据引用（设计 §2.2 第一条：「它没有可越的权」）
//!
//! 同法，不出现 [`NO_POWER_NAMES`]。三条判据缺一不可：
//!
//! 1. **它挡的是什么**：G 一旦引用了能力／效应／凭据的任何东西，它就有了「可越的权」这个问题
//!    的**物质基础**。这条守卫把「它没有可越的权」从一句话变成**一条会红的用例**；
//!    它同时是设计 §2.1 的三个强制点逐个核在 G 侧的落点。
//! 2. **`ALLOWED` 挡不住它，据实写明**：`continuum-runtime` 的条目**本来就含**这三个 crate
//!    （F 在用，实测 `dependency_direction.rs:172-189`），故多引一个**不会**让任何依赖断言变红。
//!    **这条守卫是本判据唯一的结构性落点。**
//! 3. **它是下界**：见第四节。
//!
//! **取词判据（免得后来者改宽或改窄）**：取 **crate 路径与类型名**，**不取裸词**
//! `capability` / `effect` / `secret`——那样一句中文注释里带一个英文词就会误伤，
//! 守卫会退化成「禁止谈论」而不是「禁止使用」，而**一个会误伤的守卫会被绕过或被删掉**。
//!
//! **反向的边界（都不是本守卫的红条件，据实写明）**：本守卫**不禁止**注释里用中文说「能力」
//! （那不是一个标识符，故它不在 [`NO_POWER_NAMES`] 里）；也**不禁止** `ModelCallError` 里
//! 出现 `FailureClass`——那是 `continuum-graph`，不落在 §2.1 的三个强制点的任何一条上
//! （实测：`model_call.rs` 今天就 `use continuum_graph::failure::FailureClass;`，本守卫不红）。
//!
//! # 四、三条都是下界，不是封闭判定
//!
//! 匹配的是**字面拼法**。**逃逸面据实写，且与设计 §1.3 抄来的那一句不同**：设计 §1.3 把 C 那条
//! 守卫的逃逸面（`use … as` 别名 / 全限定路径 / `include!`）**整句搬了过来**，而
//! **那三条里只有第三条在本 needle 集上成立**。判据：C 那条的 needle 是 `impl ModelProvider for`
//! 这种**多 token 短语**，别名与全限定会改掉那个短语；本文件的 needle 是**单标识符**
//! （`invoke_tool`、`ToolProvider` …），而 Rust 里一个名字要被用到，**别名与全限定路径都得把原拼法
//! 写出来**（`use continuum_provider::tool::AuthorizedTool as AT;` 里 `AuthorizedTool` 原样在），
//! 故它们**照样命中，不是逃逸面**。真正的逃逸面是：
//!
//! 1. **在本文件之外写出、这里经拐弯引用**：他处 `pub use` 之后写 `crate::ToolCall`；
//! 2. **`include!`**：把别处的代码整段拉进来，那段的文本不在本文件里；
//! 3. **宏展开**：`some_macro!()` 展开成那个拼法，而文本里没有它；
//! 4. **清单没列到的同义名字**（**这张清单不穷尽**，别按闭合读）：例如一个名叫
//!    `continuum-model-<vendor>` 的适配器 crate、或 `continuum_capability` 里没列进
//!    [`NO_POWER_NAMES`] 的另一个类型。
//!
//! 要把上界也钉住得**解析**源码（`syn`），本阶段不做。故**归一化之后它仍是一个下界**。
//!
//! # 五、归一化（裁决 C7）在本文件上**不承重**，据实写明
//!
//! C7 的口径是「空白变体是同一个字面拼法」。**它承重的是 C 那条守卫**——那边的 needle 是
//! `impl ModelProvider for` 这种**多 token 短语**，`impl  ModelProvider for` 对它才是真逃逸面。
//! **本文件的三张清单里没有一枚 needle 含内部空白**（都是单标识符或 crate 路径），而匹配用的是
//! **词边界**：把连续空白折成一个空格**既不产生也不消除任何词边界**，故对这三条判定而言
//! `normalize` 与恒等函数**结论相同**。**这是实测的，不是推的**：见本 task 报告
//! `.superpowers/sdd-p3g-impl/task-4-report.md` 的「等价变异体」一节（把 `normalize` 换成恒等，
//! 三条判定用例**逐条仍绿**，只有本文件的 `the_guard_folds_whitespace_before_matching` 红）。
//!
//! **那为什么留着它**：① 设计 §1.3 的照片一栏点名要它（照 C 的形状）；
//! ② 本文件的判定都写在归一化文本上，若后来把 needle 改成短语（例如要钉
//! `impl ModelProvider for` 那种形态），归一化必须已经在、且已经被拍过照。
//! **真正承重的是词边界那一层**，它的照片在 `the_matcher_matches_whole_identifiers_only`。
//!
//! **已经证伪、不要照抄的说法**（出处：本 task 的派单 Step 1 与 Step 3，照设计 §1.3 的 C7 口径）：
//! 「空白归一化让 `AuthorizedTool  Invocation` 与 `AuthorizedTool Invocation` 是同一个拼法，
//! 故把红的条件写成两个空格的变体时，**这一次红同时是空白归一化生效的照片**」。
//! **对单标识符 needle，这两句都不成立**：① 三张清单里没有一枚 needle 含内部空白，
//! 而归一化折的是**连续的**空白——`AuthorizedTool  Invocation` 的两个空格折成一个空格，
//! 但「一个空格」与「没有空格」**不是**空白变体，故「折成同一个拼法」在判定上没有落点；
//! ② 于是那个两个空格的变异体红了，红的是**拼法出现了**，与归一化无关。
//! `the_guard_folds_whitespace_before_matching` 拍的是 `normalize` 这个函数**自身的性质**，
//! 它不因此让三条判定多挡住任何一种拼法。
//!
//! # 六、本守卫的射程边界：设计 §1.3 说的是「（及其测试）」，本文件只读源码一份
//!
//! 设计 §1.3 的原话是「`crates/continuum-runtime/src/model_call.rs`（及其测试）的源码文本里」。
//! **本文件只读 `model_call.rs` 这一份**，理由两条：① 本文件的字面量**就是**那几枚拼法
//! （三张清单与探针语料），把本文件算进语料就是**拿清单查清单**——它必红，
//! 而必红的守卫与恒绿的守卫一样读不出信息（这正是「正控制不得自证」要挡的形状）；
//! ② 派单把语料划在 `model_call.rs` 一份上（Task 4 的 Interfaces：只读该文件）。
//! **未闭合的那一面记在这里**：G 自己的 `tests/model_call.rs`（以及后续 task 新增的测试文件）
//! 里若出现这些拼法，本守卫**不报**。实测 2026-10-08：`tests/model_call.rs` 对三张清单
//! **逐枚零命中**，故今天不欠账；这条记的是射程，不是欠账。
//!
//! # 七、用例次序
//!
//! 机制的照片排在结论之前：`the_guard_sees_the_module`、
//! `the_guard_reddens_on_the_real_module_with_a_probe_line_appended`、
//! `the_guard_folds_whitespace_before_matching`、`the_matcher_matches_whole_identifiers_only`、
//! `every_banned_spelling_is_found_in_an_independently_written_probe` 五条，
//! **一条都不依赖**下面三条判定的绿——反过来，下面三条判定的「零命中」若没有它们，
//! 与「守卫什么都没读到」在输出上无法区分。

use std::fs;
use std::path::{Path, PathBuf};

/// 设计 §1.3 的五个标识符：工具调用路径的名字。
///
/// **一枚都不许出现在 `model_call.rs` 里**（见文件头第一节）。
const TOOL_PATH_NAMES: [&str; 5] = [
    "AuthorizedTool",
    "AuthorizedToolInvocation",
    "ToolProvider",
    "invoke_tool",
    "authorize",
];

/// 设计 §10.3：实现 crate 与实现类型名。**一个适配器 crate 今天都不存在**，
/// 故这张清单取的是「已经约定的那几个拼法」——**清单外的同义名字照样逃逸**（文件头第四节）。
const ADAPTER_NAMES: [&str; 5] = [
    "continuum-adapter",
    "continuum_adapter",
    "DeepSeek",
    "OpenAi",
    "Anthropic",
];

/// 设计 §2.2 第一条的取词：**crate 路径与类型名**，不取裸词（理由见文件头第三节）。
const NO_POWER_NAMES: [&str; 11] = [
    "continuum_capability",
    "continuum-capability",
    "continuum_effect",
    "continuum-effect",
    "continuum_secrets",
    "continuum-secrets",
    "AuthorizedEffect",
    "EffectJournal",
    "CapabilityKind",
    "SecretsRuntime",
    "SecretStore",
];

/// 被测源码：`model_call.rs` 的绝对路径。
///
/// **按 `CARGO_MANIFEST_DIR` 定，不按进程 CWD**：按相对路径读的守卫在 workspace 根下跑是绿的，
/// 换个目录跑就**读不到东西**——而「什么都没读到」正是 `the_guard_sees_the_module` 要逮的形状。
/// `CARGO_MANIFEST_DIR` 由 cargo 编译期注入，恒等于本 crate 目录。
fn module_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/model_call.rs")
}

/// 读 `model_call.rs` 全文。
///
/// **读不动或不是 UTF-8 都直接炸，不跳过**：一份读不动的源文件恰恰是守卫最该报的东西
/// ——静默跳过等于把守卫的射程悄悄缩小，而缩小之后它仍然是绿的。
///
/// **空语料也直接炸。** 语料为空时三条判定**每一条都恒真、恒绿**；把这条断言放在唯一的读取口上，
/// 是让「漏读」与「读过且零命中」在输出上可区分的最低代价（另一侧的照片是
/// `the_guard_sees_the_module` 与 `the_guard_reddens_on_the_real_module_with_a_probe_line_appended`）。
fn read_module() -> String {
    let path = module_path();
    let bytes = fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "读不到源文件 {}：{e}。**不跳过**——读不到语料时下面每一条判定都是恒真的。",
            path.display()
        )
    });
    let text = String::from_utf8(bytes).unwrap_or_else(|e| {
        let at = e.utf8_error().valid_up_to();
        panic!(
            "源文件 {} 不是 UTF-8（第 {at} 个字节起非法）。**不跳过**——不跳过才谈得上「读全了」。",
            path.display()
        )
    });
    assert!(
        !text.is_empty(),
        "源文件 {} 是空的。空语料让三条判定**每一条都恒真**，故这里先红。",
        path.display()
    );
    text
}

/// 折叠连续空白之后的文本，**以及每个字节在源文件里的行号**。
///
/// 行号必须与折叠**同批**记录：折完之后换行符已经没了，回头再数行数无从数起，
/// 而断言信息里要报出行号。红的时候读不出是哪一处，这条守卫就只剩一个「红」。
struct Normalized {
    text: String,
    /// 与 `text` 的字节一一对应；`line_of[i]` 是 `text` 第 `i` 个字节在源文件里的 1 起行号。
    line_of: Vec<usize>,
}

/// 把连续空白折成一个空格。
///
/// 「空白」取 `char::is_whitespace` 的**全集**（设计举例时点的是空格 / `\t` / `\n` 三种，
/// 实现折的比那三种**多**：`\r`、换页、NBSP、U+2028 之类一并折）。
/// **「方向只会放宽」那一句不照抄 C**：C 那条守卫的 needle 是**多 token 短语**，
/// 折得更宽只会命中更多；本文件三张清单的 needle 都不含内部空白，
/// 折与不折**命中集完全相同**（文件头第五节的实测）——别把 C 的这句话当成本函数的效力。
///
/// 用一个朴素状态机而不是正则：本仓不为此新增依赖。
/// 折出来的那个空格记的是**这段空白起始处**的行号——匹配从空白之前开始时，报到的是读者想看的那一行。
///
/// **本函数的判定力见文件头第五节**：对单标识符 needle 它与恒等函数结论相同。
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

/// 算不算「标识符里能出现的字符」。
///
/// **取 ASCII**（`is_ascii_alphanumeric` 或 `_`），**不取 `char::is_alphanumeric`**——
/// 被否掉的那个写法在本仓会**静默变窄**：中日韩表意文字在 Unicode 里是 Alphabetic，故
/// `不借用ToolProvider` 这一行里那个标识符**左边**会被当成词的一部分、边界不成立、命中丢失；
/// 而中文与标识符之间不空格正是本仓注释的常见写法。取 ASCII 之后
/// `不借用ToolProvider` **命中**、`xToolProvider` **不命中**——两者都有照片
/// （`the_matcher_matches_whole_identifiers_only`）。
fn is_ident_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

/// `needle` 在归一化文本里的**词边界**命中位置（字节下标）。
///
/// **两侧都要查**：只查一侧会让 `AuthorizedTool` 认领 `AuthorizedToolInvocation`
/// （那枚有自己的 needle，两侧都查才能让两枚各认各的），也会让 `authorize` 认领
/// `authorized_tool`、`invoke_tool` 认领 `invoke_tools`。
fn matches_in(norm: &Normalized, needle: &str) -> Vec<usize> {
    let mut hits = Vec::new();
    let mut from = 0usize;

    while from < norm.text.len() {
        let Some(offset) = norm.text[from..].find(needle) else {
            break;
        };
        let at = from + offset;
        let end = at + needle.len();
        let before_ok = norm.text[..at]
            .chars()
            .next_back()
            .is_none_or(|c| !is_ident_char(c));
        let after_ok = norm.text[end..]
            .chars()
            .next()
            .is_none_or(|c| !is_ident_char(c));
        if before_ok && after_ok {
            hits.push(at);
        }
        from = end;
    }

    hits
}

/// `needles` 每一枚在**源码全文**里的词边界命中，形如 `(行号, 拼法)`，按 `(行号, 拼法)` 排序。
///
/// 先归一化再匹配：判定写在归一化文本上（裁决 C7 的形状，它在本文件上承重与否见文件头第五节）。
fn hits(source: &str, needles: &[&'static str]) -> Vec<(usize, &'static str)> {
    let norm = normalize(source);
    let mut out = Vec::new();
    for needle in needles {
        for at in matches_in(&norm, needle) {
            out.push((norm.line_of[at], *needle));
        }
    }
    out.sort();
    out
}

/// **独立写出来的探针语料**：这些拼法是照设计 §1.3 与 §2.2 的判据**另写一遍**，
/// **不是**从 [`TOOL_PATH_NAMES`] 里取——从清单里取就会恒真（清单漏一枚，语料也漏一枚，
/// 于是漏枚这件事永远拍不到）。
///
/// 十一行，逐行的用途：
///
/// | 行 | 内容 | 期望 |
/// |---|---|---|
/// | 1 | 说明行 | — |
/// | 2 | `AuthorizedTool` 单独一枚 | 命中 `AuthorizedTool` |
/// | 3 | `AuthorizedToolInvocation` | 命中它自己；**不**被 `AuthorizedTool` 认领（右边是 `I`） |
/// | 4 | `ToolProvider` | 命中 |
/// | 5 | `invoke_tool` | 命中 |
/// | 6 | `authorize` | 命中 |
/// | 7 | `pre_authorize` | 零命中（左边是 `_`） |
/// | 8 | `authorized_tool` | 零命中（`authorize` 右边是 `d`） |
/// | 9 | `invoke_tools` | 零命中（右边是 `s`） |
/// | 10 | `不借用ToolProvider` | 命中（中日韩文字**不是** ASCII 词字符，边界成立） |
/// | 11 | `xToolProvider` | 零命中（左边是 ASCII 字母） |
const PROBE_LINES: [&str; 11] = [
    "// 探针语料：拼法独立写出，不从判定清单里取",
    "AuthorizedTool",
    "AuthorizedToolInvocation",
    "ToolProvider",
    "invoke_tool",
    "authorize",
    "pre_authorize",
    "authorized_tool",
    "invoke_tools",
    "不借用ToolProvider",
    "xToolProvider",
];

/// 探针语料的全文（每行以 `\n` 收尾，故第 n 行的行号就是 n）。
fn probe() -> String {
    let mut text = PROBE_LINES.join("\n");
    text.push('\n');
    text
}

/// **守卫自身的正控制：先钉机制，再看结论。**
///
/// 没有这一条，一个「什么都没读到」的守卫会永远绿——这正是「守卫须两侧都钉」里缺的那一侧。
/// 三件事一起断：**语料非空**（`read_module` 里）、**读到的行数**、**两个拼法都在**。
///
/// 第二与第三个拼法是**两枚独立的**（一个是本 task 被测的同步段、一个是第②段的前半），
/// 只断一枚的话，「读到的是一份恰好含那一枚的别的东西」仍然绿。
#[test]
fn the_guard_sees_the_module() {
    let text = read_module();
    let lines = text.lines().count();

    assert!(
        lines >= 50,
        "{} 只读到 {lines} 行。**一个读不到东西的守卫会永远绿**，故这条先红。",
        module_path().display()
    );
    assert!(
        text.contains("pub fn plan_candidates"),
        "读到了 {lines} 行，但里面找不到 `pub fn plan_candidates`——读到的不是这棵树。"
    );
    assert!(
        text.contains("pub async fn snapshot"),
        "读到了 {lines} 行，但里面找不到 `pub async fn snapshot`——读到的不是这棵树。"
    );
}

/// **整条流水线在真语料上的正控制。**
///
/// `the_guard_sees_the_module` 断的是「读到了」，用的是 `str::contains`，**不过匹配器**；
/// 探针那条断的是匹配器在**人造语料**上工作。中间的缝是「匹配器在**真语料**上工作」——
/// 归一化处理**整份**（含中文注释与多字节字符）之后的文本、行号从真语料里报出来。
/// 这条把一枚探针行**追加在真语料末尾**（只是内存里的字符串，不碰源码文件），
/// 断言：**恰好命中一次**，**拼法对**，**行号正好是追加的那一行**。
///
/// **它同时挡住「语料退化成一行」**：那时期望行号与命中行号会巧合相等而恒绿，
/// 故先断行数。
#[test]
fn the_guard_reddens_on_the_real_module_with_a_probe_line_appended() {
    let real = read_module();
    let real_lines = real.matches('\n').count();
    assert!(
        real_lines >= 50,
        "真语料只有 {real_lines} 行：下面的期望行号会与命中行号巧合相等，这条就恒绿了。"
    );

    let mut with_probe = real.clone();
    // 只在语料**没有**以换行收尾时才补一个——否则先补一个再拼一行会多出一整行空行，
    // 而下面的期望行号是按「完整行的条数」算的（本文件初版就在这里多算了一行，
    // 实测把命中报成 234 而期望 233，是这条用例自己先红的）。
    if !with_probe.ends_with('\n') {
        with_probe.push('\n');
    }
    with_probe.push_str("// 探针：本模块不碰 ToolProvider\n");

    let found = hits(&with_probe, &TOOL_PATH_NAMES[..]);
    assert_eq!(
        found,
        vec![(real_lines + 1, "ToolProvider")],
        "在真语料末尾追加一行 `// 探针：本模块不碰 ToolProvider` 之后，期望恰好一次命中、\
         行号为 {}。实际命中 {found:?}。**若这里是零命中，匹配器或读取口坏了**；\
         若行号不对，行号与折叠没有同批记录。",
        real_lines + 1
    );
}

/// **归一化自身的照片。** 没有这一条，归一化写错了也没人知道。
///
/// 四个输入：前三个各取一种空白变体（两个空格 / `\t` / `\n`——**是本用例各取一枚，不是
/// 「空白只有三种」**；`normalize` 折的是 `char::is_whitespace` 全集），第四个是已折好的本形，
/// 摆进来才能让「归一化在本形上**幂等**」这件事有照片，而不只是一句说法。
///
/// 只断言 `norm.text` 相等**不够**：判定真正用的是「归一化之后找得到 needle」，
/// 故每个输入同时断言 `matches_in` 恰好命中一次。
///
/// **它拍的是 `normalize` 自身的性质，不是三条判定的效力**——本文件的三张清单里没有一枚 needle
/// 含内部空白，故归一化与恒等函数在三条判定上结论相同（文件头第五节的实测）。
#[test]
fn the_guard_folds_whitespace_before_matching() {
    let needle = "AuthorizedTool Invocation";
    let variants = [
        ("本形（已折好）", "AuthorizedTool Invocation"),
        ("两个空格", "AuthorizedTool  Invocation"),
        ("制表符", "AuthorizedTool\tInvocation"),
        ("换行", "AuthorizedTool\nInvocation"),
    ];

    for (name, variant) in variants {
        let norm = normalize(variant);
        assert_eq!(
            norm.text, needle,
            "{name}：归一化没有把 {variant:?} 折成 {needle:?}，得到 {:?}",
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

    // **归一化不吞注释**。归一化只折空白，故注释里照抄一个拼法**照样命中**，本守卫会因此误报
    // ——这是「只看文本」的代价：松的一面（换写法逃逸，见文件头第四节，**非穷尽**）
    // 与紧的一面（注释/字符串照抄）来自同一个选择。剥注释要解析 Rust，本阶段不做。
    // 若哪天归一化改成剥注释，这条会红。
    let commented = normalize("// 不得写 AuthorizedTool  Invocation\n");
    assert_eq!(
        commented.text, "// 不得写 AuthorizedTool Invocation ",
        "归一化把注释吞了：本守卫的射程变了，文件头第四节的「紧的一面」那句话要一并改"
    );
    assert_eq!(
        matches_in(&commented, needle).len(),
        1,
        "注释里的拼法没有命中——归一化已经不是「只折空白」了"
    );
}

/// **词边界那一层的照片**（本文件真正承重的一层，见文件头第五节）。
///
/// 逐条给输入与期望，不抽代表：**紧的一侧**（长标识符里嵌着禁用拼法，不该命中）与
/// **松的一侧**（中日韩文字紧挨着，该命中）**都要有**——只拍一侧的话，一个两侧都不查、
/// 或两侧都查错的匹配器照样绿。
#[test]
fn the_matcher_matches_whole_identifiers_only() {
    let count = |text: &str, needle: &str| matches_in(&normalize(text), needle).len();

    assert_eq!(
        count("ToolProvider", "ToolProvider"),
        1,
        "独立成词的拼法应当命中一次"
    );
    assert_eq!(
        count("`ToolProvider`", "ToolProvider"),
        1,
        "反引号不是标识符字符，包起来应当照样命中"
    );
    assert_eq!(
        count("不借用ToolProvider", "ToolProvider"),
        1,
        "中日韩文字**不是** ASCII 词字符，紧挨着应当命中。**这一条挡住\
         「把 is_ident_char 改成 char::is_alphanumeric」**：那样中文会被当成词的一部分，\
         这一处会静默漏掉。"
    );
    assert_eq!(
        count("ToolProvider的路径", "ToolProvider"),
        1,
        "右边紧挨中日韩文字，同上"
    );
    assert_eq!(
        count("xToolProvider", "ToolProvider"),
        0,
        "左边是 ASCII 字母：长标识符里嵌着，不命中"
    );
    assert_eq!(
        count("ToolProviderX", "ToolProvider"),
        0,
        "右边是 ASCII 字母：同上"
    );
    assert_eq!(
        count("pre_authorize", "authorize"),
        0,
        "左边是下划线：同上"
    );
    assert_eq!(
        count("authorized_tool", "authorize"),
        0,
        "右边是 ASCII 字母：`authorize` 不该认领 `authorized`"
    );
    assert_eq!(
        count("invoke_tools", "invoke_tool"),
        0,
        "右边是 ASCII 字母：同上"
    );
    assert_eq!(
        count("AuthorizedToolInvocation", "AuthorizedTool"),
        0,
        "`AuthorizedTool` 不该认领 `AuthorizedToolInvocation`——那一枚有自己的 needle，\
         两侧都查才能让两枚各认各的"
    );
}

/// **判定清单自身的照片**：清单里一枚 typo（`ToolProvder`）或一枚漏写不会让任何一条判定变红，
/// 只会让它**静默变窄**——此后那个拼法随便写，守卫全绿。故用一个**独立写出来的探针语料**
/// 把清单钉住。
///
/// 三件一起断，缺哪一件都留着一种漏法：
///
/// 1. **命中总数**（钉清单**漏**枚）：少一枚，探针里对应的那一行就没人认领；
/// 2. **逐枚**（钉清单**多**枚与**错**枚）：每枚各断言一次它自己的命中数，**不抽代表**；
/// 3. **位置报对**：行号与拼法的整表比对——前两件都过而把行号报错，红的时候读不出是哪一处。
///
/// 再加一条 [`TOOL_PATH_NAMES`] 的**枚数**：探针语料只覆盖那五枚，**第六枚若不在探针里，
/// 上面三件都拍不到它**（总数不变），故这一条必须单独断。
#[test]
fn every_banned_spelling_is_found_in_an_independently_written_probe() {
    let probe = probe();
    let found = hits(&probe, &TOOL_PATH_NAMES[..]);

    // 一、命中总数。
    assert_eq!(
        found.len(),
        6,
        "独立写出来的探针语料里应有 6 处命中（五枚拼法，其中 `ToolProvider` 出现两次），\
         实际 {} 处：{found:?}。**少一处就是清单里漏了一枚（或某枚拼错了）**。",
        found.len()
    );

    // 二、逐枚。每枚各写一次，不抽代表。
    assert_eq!(
        hits(&probe, &["AuthorizedTool"]).len(),
        1,
        "`AuthorizedTool` 在探针里应命中一次（第 2 行那一枚）"
    );
    assert_eq!(
        hits(&probe, &["AuthorizedToolInvocation"]).len(),
        1,
        "`AuthorizedToolInvocation` 在探针里应命中一次（第 3 行）"
    );
    assert_eq!(
        hits(&probe, &["ToolProvider"]).len(),
        2,
        "`ToolProvider` 在探针里应命中两次（第 4 行与第 10 行那一枚紧挨中日韩文字的）"
    );
    assert_eq!(
        hits(&probe, &["invoke_tool"]).len(),
        1,
        "`invoke_tool` 在探针里应命中一次（第 5 行）"
    );
    assert_eq!(
        hits(&probe, &["authorize"]).len(),
        1,
        "`authorize` 在探针里应命中一次（第 6 行）"
    );

    // 第三件之前先把枚数钉住：第六枚不在探针里，上面三件都拍不到它。
    assert_eq!(
        TOOL_PATH_NAMES.len(),
        5,
        "设计 §1.3 的清单是**五枚**。多出来的一枚若不在探针语料里，上面的总数与逐枚断言\
         都不会变红——故枚数单独断一次。今天的清单：{TOOL_PATH_NAMES:?}"
    );

    // 三、位置报对。
    assert_eq!(
        found,
        vec![
            (2, "AuthorizedTool"),
            (3, "AuthorizedToolInvocation"),
            (4, "ToolProvider"),
            (5, "invoke_tool"),
            (6, "authorize"),
            (10, "ToolProvider"),
        ],
        "行号或拼法对不上：判定红的时候报不出是哪一处。实际 {found:?}"
    );
}

/// 三张清单共用的判定：命中即红，**报出每一处的行号与拼法**。
///
/// 判定跑在 `source` 上而不是自己去读：这样 `the_guard_reddens_on_the_real_module_with_a_probe_line_appended`
/// 与本函数走的是**同一条**匹配路径。
fn assert_no_hits(source: &str, needles: &[&'static str], what: &str, why: &str) {
    let found = hits(source, needles);
    assert!(
        found.is_empty(),
        "{what}：`model_call.rs` 里出现了 {} 处被禁的拼法：\n  {}\n{why}\n\
         判定已折叠空白、且按词边界匹配，故 `impl  …` 这类多一个空格的写法与多一个字母\
         （`AuthorizedTool` vs `AuthorizedToolInvocation`）各有各的判定。\n\
         **若这一步红了而你不认为有借用：先查是不是写在注释里**——本判据是拼法，不是用法，\
         注释里提到这个拼法**也红**（文件头第一节，这是刻意的）。",
        found.len(),
        found
            .iter()
            .map(|(line, name)| format!("model_call.rs:{line}: `{name}`"))
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}

/// **设计 §1.3 的判定**：`model_call.rs` 里不出现工具调用路径的五个标识符。
///
/// 红的条件（档位：**取反**）：在 `model_call.rs` 里写一句「本模块不碰 `ToolProvider`」的注释即红。
/// 本守卫是**下界**，逃逸面与理由见文件头第四节。
#[test]
fn the_module_does_not_borrow_the_tool_paths_names() {
    let source = read_module();
    assert_no_hits(
        &source,
        &TOOL_PATH_NAMES[..],
        "用词纪律（设计 §1.3）",
        "G 的模型调用路径与 F 的工具调用路径是两个词汇表（裁决：G 不得借用 F 的名字）。",
    );
}

/// **设计 §10.3 的判定**：`model_call.rs` 里不出现实现 crate 与实现类型名。
///
/// **上界那条边由 `dependency_direction.rs` 兜住**（实测行号见文件头第二节）：本守卫是下界。
#[test]
fn the_module_names_no_adapter_implementation() {
    let source = read_module();
    assert_no_hits(
        &source,
        &ADAPTER_NAMES[..],
        "不命名适配器实现（设计 §10.3）",
        "G 只命名 `Arc<dyn ModelProvider>`，签名里不出现任何适配器类型。",
    );
}

/// **设计 §2.2 第一条的判定**：「它没有可越的权」——不引用能力／效应／凭据的任何东西。
///
/// 三条判据与取词判据见文件头第三节；**`ALLOWED` 挡不住它**（runtime 的条目本来就含这三个 crate），
/// 故这是本判据唯一的结构性落点。
#[test]
fn the_module_touches_neither_capability_nor_effects_nor_credentials() {
    let source = read_module();
    assert_no_hits(
        &source,
        &NO_POWER_NAMES[..],
        "零能力／效应／凭据引用（设计 §2.2 第一条）",
        "G 一旦引用了这三者的任何东西，它就有了「可越的权」这个问题的物质基础；\
         本判据把它从一句话变成一条会红的用例。",
    );
}
