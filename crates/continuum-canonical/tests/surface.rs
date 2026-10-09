//! §270 的 Surface Normalizer 与 §196／§197 的 Mention Detector 用例。
//!
//! 每条用例的头注释写明**它钉的是什么**与**红条件是什么**。
//! 这些注释不是装饰：本仓的规矩是「注释里写了红条件的，那条红条件就要真的取到」
//! ——故下面每一条都在本 task 的变异轮里真的被取过，取到的结果记在报告里。
//! （修复轮 1 补的 `normalize_does_not_trim_surrounding_whitespace` 在**修复轮**
//! 取过，结果记在报告的「修复轮 1」一节。）

use continuum_canonical::{MentionKind, detect, normalize};

/// 四步折叠**各一条**。
///
/// 钉的是：Unicode 归一、宽度归一、空白折叠、大小写折叠四件事**都在**。
///
/// 红条件（收紧档）：把 [`normalize`] 缩成 `to_lowercase` 一档。四类里
/// 大小写的那个断言不变，其余三个会红。
///
/// **次序有讲究**：断言按 连字 → 全角 → 空白 → 大小写 排，
/// 是为了让「**只去宽度归一**」的那枚隔离变异体（把 `raw.nfkc().collect()` 换成
/// 只保住连字的 `raw.replace('\u{FB01}', "fi")`）**第一条**失败落在全角那一行。
/// **实测**：该变异体首败在**全角那一行** `:40:5`（`left: "ａ"`）——连字臂先过，全角臂才红。
/// 而把整个 [`normalize`] 缩成 `raw.to_lowercase()` 的那一枚首败在**连字那一行**
/// `:37:5`（`left: "ﬁ"`），因为连字臂排在它前面。
/// **实际红集以变异轮实测为准**——预告与实测不一致时以实测为准并记录下来，
/// 不去改断言迁就预告。
///
/// 〔订正（修复轮 1）〕本段初稿写：「是为了让「缩成 `to_lowercase`」这一枚变异体的
/// **第一条**失败落在全角那一行（简报 Step 2 预告的红是「宽度那条」）。」
/// **那句被实测证伪**：缩成 `to_lowercase` 的那一枚首败在连字臂，不在全角臂；
/// 满足该句的是上文的**隔离**变异体。**这句错预告是协调者简报里的原话被原样抄进
/// 本文件的**——简报写错不豁免自己这份抄写（本仓「归因上游≠豁免自己那份」那条纪律），
/// 故此处订正，原句留此。
#[test]
fn normalize_folds_unicode_width_whitespace_and_case() {
    // Unicode 归一：U+FB01 LATIN SMALL LIGATURE FI。它只有**兼容分解**（NFKC/NFKD）会拆，
    // 规范分解（NFC/NFD）不拆——故这一条同时钉住了「取的是兼容分解」。
    assert_eq!(normalize("ﬁ").as_str(), "fi");

    // 宽度归一：全角 Ａ（U+FF21）→ 半角 a。NFKC 一步同时给出宽度与大小写两件事。
    assert_eq!(normalize("Ａ").as_str(), "a");

    // 空白折叠：连续空白（空格、制表、换行）与全角空格（U+3000）**塌成一个半角空格**。
    // 断言里 `a` 与 `b` 之间那一枚是 ASCII 空格（0x20），不是 U+3000。
    assert_eq!(normalize("a \t\n\u{3000} b").as_str(), "a b");

    // 大小写折叠。
    assert_eq!(normalize("MiXeD").as_str(), "mixed");
}

/// [`normalize`] **不裁剪首尾**：它是「折叠」，不是 `trim`。
///
/// 钉的是 `surface.rs` 里那两句绝对措辞——函数头「**四步之外不做任何事**」
/// 与空白折叠那一步的注释「**不裁剪首尾**（这里不是 `trim`）」。
/// §270 只说「折叠」，裁剪是**另一件事**，规范没给，故本层不做
/// （要做的调用方在调用点显式做）。**这一条是修复轮 1 补的**。
///
/// **为什么夹具必须带首尾空白**：在没有首尾空白的输入上，「折叠」与「折叠＋裁剪」
/// 给出同一个结果（四步折叠那条的空白臂夹具 `"a \t\n\u{3000} b"` 首尾都没有空白，
/// 两版都是 `"a b"`），故若不专门造一枚带首尾空白的输入，本用例与「加了 `trim`」
/// 就是等价变异体（本仓「变异须真落到实现体」那条纪律）。
///
/// 红条件（移除档）：在 [`normalize`] 的返回处加 `.trim()` 即红。
/// **实测**：把 `Surface(out)` 改成 `Surface(out.trim().to_owned())`，
/// 本用例首败在下面第一行断言（`left: "a"`、`right: " a "`）；
/// **加这条用例之前，该变异体在 5 条用例上全绿、exit 0**——即变异不可观察。
#[test]
fn normalize_does_not_trim_surrounding_whitespace() {
    // 前导与后继各留一枚：全角空格 U+3000 先被折叠成 ASCII 空格，然后**仍不裁剪**。
    assert_eq!(normalize("  a  ").as_str(), " a ");
    assert_eq!(normalize("\u{3000}他\u{3000}").as_str(), " 他 ");
}

/// 否定式照片：**没有同义词表、没有纠错表参与**。
///
/// §197 的原文例子是「克拉的code」可能实际表示「Claude Code」。那个映射
/// **不发生在 [`normalize`] 里**——它是候选生成（§197）的活，发生在 Mention Detection
/// **之后**、以候选集的形式（设计 §3.1 的 Candidate Retriever）。
///
/// 照片是一条**逐字相等**的断言：输出等于那句话只经字面折叠的结果。换言之
/// **没有任何映射表参与**。
///
/// **这是 [`normalize`] 文档里「不做同义词、不做纠错」那句话的用例**。
///
/// 〔订正（修复轮 1）〕本段初稿写：「**这是本 task 唯一的绝对措辞守卫**」——
/// **那句在文件里不成立**：`surface.rs` 的同一条文档里还有别的绝对措辞
/// （「四步之外不做任何事」「不裁剪首尾（这里不是 `trim`）」），当时它们都没有照片。
/// 现在「不裁剪」那一半由同文件的
/// `normalize_does_not_trim_surrounding_whitespace` 补上；
/// 「不做同义词、不做纠错」由本条钉住。原句留此。
///
/// 红条件（移除档）：给 [`normalize`] 加一张 `"克拉的code" -> "Claude Code"` 的映射，
/// 本用例即红（输出的 `C` 会被 lowercase 成 `c`，但无论如何不再是「克拉的code」）。
/// **红条件在移除档上的含义**：删掉本用例之后，类型里没有任何东西阻止
/// 「纠错被写进 `normalize`」——`Surface` 只是个字符串壳子。
#[test]
fn normalize_does_not_substitute_synonyms_or_correct_typos() {
    assert_eq!(normalize("克拉的Code").as_str(), "克拉的code");
}

/// `span` 是**字节偏移**，且逐 mention 地在同一个 `Surface` 上取回原串。
///
/// 钉的是：`Mention.span` 与 `Surface` 的 UTF-8 字节串同一个坐标系。
///
/// **夹具必须含多字节字符**：否则「字节偏移」与「字符下标」是同一个数，
/// 这条用例与索引序就是等价变异体（本仓「变异须真落到实现体」那条纪律）。
/// 夹具 `请把它和 this 都改掉` 里在 `它` 之前有 `请`、`把` 两个三字节字符，
/// 故 `它` 的字节偏移是 6、字符下标是 2——两数不同。
///
/// 红条件（取反档）：`span` 用字符下标而不是字节偏移，即红。
/// （届时首条断言 `texts` 仍成立，`&surface.as_str()[span]` 那一行会 panic
/// ——那不是字符边界；无论 panic 还是断言失败，都是红。）
#[test]
fn detect_spans_are_byte_offsets_into_the_surface() {
    let surface = normalize("请把它和 this 都改掉");
    let mentions = detect(&surface);

    // 逐项钉住切出来的**是哪几枚**：漏一切、多切一切、次序颠倒都会红。
    let texts: Vec<&str> = mentions.iter().map(|m| m.text.as_str()).collect();
    assert_eq!(texts, ["它", "this"]);

    // 逐 mention 校验坐标系——**不抽代表**。
    for m in &mentions {
        assert_eq!(
            &surface.as_str()[m.span.clone()],
            m.text.as_str(),
            "span 必须是 Surface 字节串上的偏移",
        );
    }

    // 把「字节 vs 字符」这个差直接摆出来：`它` 占 6..9，而它的字符下标是 2。
    assert_eq!(mentions[0].span, 6..9);
}

/// fail-closed 侧：没有命中时**不 panic、不造一个假 mention**。
///
/// 夹具 `always run the test suite before merge` 里埋了一枚暗桩：`suite` 含子串 `it`。
/// 本用例因此同时测**词边界判定**——若 `detect` 退化成不做边界的子串搜索，本用例即红。
///
/// 红条件：`detect` 在零命中时返回一枚占位 `Mention`（如 `0..0` / `""`）即红。
#[test]
fn an_input_without_any_mention_yields_an_empty_vec() {
    let surface = normalize("always run the test suite before merge");
    assert!(detect(&surface).is_empty());
}

/// `MentionKind` 是**不透明串**：取值域不由本层封闭。
///
/// 构造一个表外取值，断言它经 `as_str` / `from_str` **往返**。
///
/// **这条的限度要写明**：把 `MentionKind` 改成封闭枚举会让本用例**编译不过**
/// ——而**编译不过不算变红**（本仓纪律：编译失败是「编译器拒绝了另一份代码」，
/// 不是「这份代码的行为变了」）。故本条的守卫是**运行期往返**，
/// 它钉的是「取值域不由本层封闭」这件事（设计 §16 第 3 条：
/// `§196`／`§197` 只给了「代词／上下文引用」两个例，没给取值域），
/// **不是**「枚举加了臂会红」。
#[test]
fn mention_kind_is_an_opaque_string() {
    let kind = MentionKind::from_str("pronoun_x");
    assert_eq!(kind.as_str(), "pronoun_x");
}
