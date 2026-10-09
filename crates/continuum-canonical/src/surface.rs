//! §270 的 Surface Normalizer 与 Mention Detector。
//!
//! §270（`docs/spec/05-normative.md:1344-1365`）只给了七个组件的**名字**与次序，
//! 未给任何组件的输入输出类型；§196（`docs/spec/04-method.md:471-513`）给了一段流程，
//! §197（同文件 `:514-551`）给了一份手段清单。本模块的两件事在设计里的处置写在
//! 设计 §3.1 的组件表前两行。
//!
//! 三处行号是**本 task 实测**的（`grep -n '^## 27[01]\.'` 与 `grep -n '^## 19[678]\.'`），
//! 不是从设计文档转抄的——设计 §3.1 那两处引的是 `:1344-1363` 与 `:471-550`，
//! 都比实测**短**两行（各自漏掉了 §270 末尾的 `---` 与 §197 的最后一段）。

use std::ops::Range;

use unicode_normalization::UnicodeNormalization;

/// §270 的 Surface Normalizer 的产物。规范化只做字面处理。
///
/// 内部串私有：**构造入口只有 [`normalize`]**。于是「进了本层的串都过了那四步」
/// 是**类型**保证的，不是调用点上的约定——`Surface` 一旦在手，规范化就已经发生过。
/// 这正是设计 §3.2 要的形状：`Canonical` 与「有 mention 未解」互为补集，
/// 而不是两个可以同时为真也可以同时为假的标志位。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Surface(String);

impl Surface {
    /// 规范化后的字面串。**Mention 的 `span` 就活在这个串的字节坐标上**。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// §270 的 Surface Normalizer：**四步之外不做任何事**。
///
/// 四步依次是 Unicode 归一 → 宽度归一 → 空白折叠 → 大小写折叠。
///
/// # 不做同义词替换、不做错别字纠正
///
/// 那是**候选生成**（§197）的活：§197 明确说那些组件负责「高召回候选生成，
/// 而不是直接决定最终语义」。§197 的原例「克拉的code」可能表示「Claude Code」
/// ——**这个映射不在这里**。`tests/surface.rs` 的
/// `normalize_does_not_substitute_synonyms_or_correct_typos` 是本条的用例。
///
/// # 也不是 `to_lowercase` 一档
///
/// 反过来也要说清：本函数**不只**折大小写。`Ａ`（U+FF21）与 `a` 折到同一个结果，
/// 是**宽度归一**给的，不是大小写折叠给的。这一条也在同一个用例里。
///
/// # 为什么取 NFKC
///
/// 「Unicode 归一」与「宽度归一」两步都是**兼容分解**（compatibility decomposition）
/// 的产物，故一次 NFKC 同时给出：`ﬁ`（U+FB01）→ `fi`、全角 `Ａ`（U+FF21）→ `A`、
/// 全角空格 U+3000 → U+0020。**这里不是 NFC**：`ﬁ` 与全角形在 NFC 下**不拆**，
/// 那样写会静默丢掉「宽度归一」这半件事。
pub fn normalize(raw: &str) -> Surface {
    // 第一步（Unicode 归一）＋ 第二步（宽度归一）：见上面「为什么取 NFKC」。
    let compatibility_folded: String = raw.nfkc().collect();

    // 第三步（空白折叠）：每一段**连续**空白塌成一个 ASCII 空格。
    // 第四步（大小写折叠）：逐 `char` 映射，且映射结果**可以是多枚** `char`
    // （如 U+0130 → "i̇"），故用 `extend` 而不是 `push`。
    //
    // **不裁剪首尾**（这里不是 `trim`）：§270 只说「折叠」，裁剪是**另一件事**，
    // 规范没有给，故本层不做——它若要做，那是一条独立的判定，应由需要它的调用方
    // 在调用点显式做，而不是藏在「归一」这个名字底下。
    let mut out = String::with_capacity(compatibility_folded.len());
    let mut in_whitespace = false;
    for ch in compatibility_folded.chars() {
        if ch.is_whitespace() {
            if !in_whitespace {
                out.push(' ');
                in_whitespace = true;
            }
        } else {
            in_whitespace = false;
            out.extend(ch.to_lowercase());
        }
    }

    Surface(out)
}

/// §270 的 Mention Detector 的产物。
///
/// `span` 是**字节偏移**（在 [`Surface`] 的 UTF-8 串上），`text` 是同一个串上
/// `span` 取回的那一段。两者在同一个坐标系里——`tests/surface.rs` 的
/// `detect_spans_are_byte_offsets_into_the_surface` 逐 mention 校验这一点。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mention {
    /// `Surface` 的字节串上的半开区间。
    pub span: Range<usize>,
    /// `surface.as_str()[span]` 的副本。冗余但方便调用点，且是校验坐标系的那一端。
    pub text: String,
    /// 见 [`MentionKind`]。
    pub kind: MentionKind,
}

/// §196／§197 **未给** `kind` 的取值域（只给了「代词／上下文引用」两个例），
/// 故取不透明串、**不做封闭枚举**（设计 §16 第 3 条）。
///
/// # 为什么不封闭
///
/// 封闭一条本层没有判据的取值域，会让「系统不认识的一个 mention 种类」在**编译期**
/// 就被拒——那是**发明判据**，而不是**保存事实**。本层在 §196／§197 里拿不到
/// 「有哪些种类」这份事实，故只能把「种类」原样存下来。
///
/// 与 [`crate::Opaque`] 同形，但**另立一个类型**：将来若规范真的给了取值域，
/// 收窄 `MentionKind` 不应该连带收窄 `completion_predicate` 那一类占位。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MentionKind(String);

impl MentionKind {
    /// 取里子串。**不改写**。
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 从任意串构造。**无校验**——同 [`crate::Opaque::from_str`]，
    /// 本类型没有「合法取值」这个概念。
    ///
    /// 名字用 `from_str` 而没有实现 `std::str::FromStr`：后者要求一个 `Err` 关联类型，
    /// 而本类型**不失败**。为一个恒成功的转换实现 `FromStr` 会把「可能失败」写成谎话
    /// （`Opaque::from_str` 的同一条理由）。
    pub fn from_str(s: &str) -> Self {
        MentionKind(s.to_owned())
    }
}

/// 「代词」（§196 的十类输入之一）的字面形态表。
///
/// # 这张表是本层自定的
///
/// §196 列了十类输入（错别字、ASR 错误、音译、简称、中英混写、昵称、工具别名、
/// Skill 名称、代词、上下文引用），**没有给任何一类的词表**；§197 给的是
/// **手段**清单（edit distance、拼音、embedding……）而不是词表。
/// 本层不实现那七种手段（它们的来源今天零个，设计 §16 第 5 条），
/// 故退到**字面命中**：这张表是一个封闭的、可读的最小集合。
///
/// # 代价：宁可多切
///
/// 命中是**字面命中**，不做词法分析，故中文词条会在「词」内部切出来
/// （例如「其他」里会切出「他」）。**这是有意的**：§197 明确要求本阶段是
/// **高召回候选生成**，误切由下游的四态处置（设计 §4.1）兜住。
///
/// # 收件人
///
/// 「§196 各类输入的字面形态表」规范未给 —— 收件人：**规范维护者**
/// （与设计 §16 第 3 条同一位，那条只说了 `kind` 的取值域，词表同样是缺的）。
const PRONOUNS: &[&str] = &[
    // 中文
    "它",
    "它们",
    "他",
    "他们",
    "她",
    "她们",
    "这个",
    "那个",
    "这些",
    "那些",
    // 英文
    "it",
    "its",
    "they",
    "them",
    "this",
    "that",
    "these",
    "those",
];

/// 「上下文引用」（§196 的十类输入之一）的字面形态表。表本身的自定与代价同
/// [`PRONOUNS`]。
const CONTEXT_REFERENCES: &[&str] = &[
    // 中文
    "上面提到的",
    "上面说的",
    "之前提到的",
    "之前说的",
    "刚才提到的",
    "刚才说的",
    "上文",
    // 英文
    "the former",
    "the latter",
    "aforementioned",
    "the above",
    "the previous one",
];

/// §270 的 Mention Detector：在 [`Surface`] 上切出 mention。
///
/// 输出按 `span.start` 升序；**互不重叠**（左端相同取最长，其余被丢弃）。
/// 零命中返回空 `Vec`——**不 panic、不造占位 mention**（fail-closed 侧，
/// `tests/surface.rs` 的 `an_input_without_any_mention_yields_an_empty_vec`）。
///
/// 命中的 `kind` 取 §196 给的两个例的原话：`"pronoun"` 与 `"context_reference"`。
/// **这两个串是本层定的**（规范只给了中文名「代词」「上下文引用」），
/// 它们是取值域里的两枚取值，不是枚举——见 [`MentionKind`]。
pub fn detect(surface: &Surface) -> Vec<Mention> {
    let s = surface.as_str();

    // 先收下**所有**字面命中（含重叠），再统一裁决。分两段而不是边扫边切，
    // 是为了让「最长优先」这条规则对**跨表**的命中同样生效
    // （如「它们」与「它」同左端，前者更长）。
    let mut hits: Vec<(usize, usize, &'static str)> = Vec::new();
    for (kind, table) in [
        ("pronoun", PRONOUNS),
        ("context_reference", CONTEXT_REFERENCES),
    ] {
        for pattern in table {
            for (start, matched) in s.match_indices(pattern) {
                let end = start + matched.len();
                if boundary_ok(s, start, end) {
                    hits.push((start, end, kind));
                }
            }
        }
    }

    // 左端升序，同左端取最长。`sort_by` 稳定，故同左端同长度时按表序——两组表之间
    // 不会有同左端同长度的两个词条（那会是同一个串），故这里不需要第三把钥匙。
    hits.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));

    let mut mentions = Vec::new();
    let mut cursor = 0usize;
    for (start, end, kind) in hits {
        if start < cursor {
            continue; // 与已选中的一枚重叠，丢弃
        }
        cursor = end;
        mentions.push(Mention {
            span: start..end,
            text: s[start..end].to_owned(),
            kind: MentionKind::from_str(kind),
        });
    }
    mentions
}

/// ASCII 词条的**词边界**判定。
///
/// 首字符是 ASCII 字母数字时，它的前一字符不得是 ASCII 字母数字或 `_`；末字符同理。
/// 没有这一步，`it` 会在 `suite`、`with` 内部被切出来。
///
/// **中文词条不做这个判定**（首字符不是 ASCII 字母数字，两个条件都直接为真）：
/// 中文没有空格分词，硬套一套边界规则会引入本层没有判据的东西——故走
/// [`PRONOUNS`] 文档里写明的「宁可多切」。
fn boundary_ok(s: &str, start: usize, end: usize) -> bool {
    let first = s[start..].chars().next().expect("命中不可能为空串");
    let last = s[..end].chars().next_back().expect("命中不可能为空串");
    let left_ok = !first.is_ascii_alphanumeric()
        || !s[..start].chars().next_back().is_some_and(is_word_char);
    let right_ok = !last.is_ascii_alphanumeric()
        || !s[end..].chars().next().is_some_and(is_word_char);
    left_ok && right_ok
}

/// 构成「词」的字符。只认 ASCII：非 ASCII 字符从不参与边界判定
/// （见 [`boundary_ok`]）。
fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}
