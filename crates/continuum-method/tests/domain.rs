//! Task 2 的照片：`MethodDomain` 的四个目录名与 `has_counterpart`（设计 §4.1／§5.1）。
//!
//! | 用例 | 钉的判据 | 红条件（档位） |
//! |---|---|---|
//! | `every_domain_round_trips_through_its_encoding` | 判据 2 的域级侧：`ALL.len() == 4`（字面量）、**逐值**往返（不抽代表）、`as_str` 四串逐字 | 解码侧（**取反档**）：删掉 `parse` 的 `"3d"` 臂或指向别的目录 ⇒ 该值往返红；编码侧（**取反档**）：`as_str` 两臂返回同一串 ⇒ 其中一个值往返红 |
//! | `all_is_in_the_order_of_the_specs_directories` | `ALL` 次序**逐位**＝§187 原文的目录次序 | 对调 `ALL` 任意两项（**取反档**）⇒ 红。**本条的用场是补上一条的集合比较缺口**（纪律 1(b)：集合比较与元素次序无关，是等价变异体，故上一条不写它） |
//! | `parse_rejects_every_string_outside_the_catalog` | 判据 2 的域级侧：四值之外**逐串** `None`（含 `"image"`） | 放宽档：通配臂 `_ => Some(Software)` ⇒ 逐串红；把 `"image"` 加成新臂 ⇒ 该串那一条红（实现比所钉的**少判一条**：原本该判 `None` 的串被收下了） |
//! | `has_counterpart_is_true_for_each_paired_domain_and_false_for_three_d` | 判据 6 的域级侧：**逐臂**四条——三个有对家的域各真、`ThreeD` 假 | 取反档：`ThreeD => true` ⇒ 第四条红；三枚真值里任一枚改 `false` ⇒ 对应条红 |
//!
//! # 照片的归属（纪律 1(c)）
//!
//! 本文件四条全部是**运行期**照片。`parse` **不是**「加一目录即编译失败」的
//! 承担者——`match` 在 `&str` 上、必须有通配臂 `_ => return None`（设计 §4.1；
//! 仓库同形先例：`crates/continuum-artifact/src/artifact.rs:68-78` 的 `parse`）。
//! 「加一臂即编译失败」由实现的另两处**穷尽 `match`**（`as_str` / `has_counterpart`，
//! 均 `match self`）承担——那是编译期照片，不在本文件的运行期红之列。
//!
//! # `"3d"` 为何单列一条逐字断言
//!
//! 它是四条目录名里**唯一可能被顺手写成 `"3D"`** 的一条：其余三条
//! （`"software"` / `"video"` / `"research"`）全小写字母、没有大小写可混。
//! 往返断言也罩得住它，但单独一条逐字断言把「小写是 §187 原文」钉在自己的
//! 红条件上。
//!
//! # `has_counterpart` 是**本设计的决定**，不是规范断言
//!
//! 设计 §4.1 末：来源是**当前分块**（`software/`↔P5c、`research/`↔P5d、
//! `video/`↔P5e，`3d/` 本轮无对家，设计 §5.1）——规范没有一处指定「哪个域
//! 有没有对家」。R2 被重新裁决时（例如 `3d/` 分给了某一块），本用例按裁决改
//! （设计 §7 末）。

use continuum_method::MethodDomain;

#[test]
fn every_domain_round_trips_through_its_encoding() {
    // 字面量 4：四臂封闭（设计 §4.1）。加第五臂的编译期防线在实现的
    // as_str / has_counterpart（穷尽 match），本条是它在运行期的数字照片。
    assert_eq!(
        MethodDomain::ALL.len(),
        4,
        "ALL 恰有四枚（字面量；设计 §10 判据 2）"
    );

    // **逐值**（不抽代表）：对 ALL 的每一枚走「编码 → 解码」一个来回。
    // 不写集合比较（如 HashSet 等值）——那与元素次序无关，对调 ALL 任意两项
    // 它也绿，是纪律 1(b) 的等价变异体；次序由下一条逐位比对补。
    for d in MethodDomain::ALL {
        // 红条件（解码侧，**取反档**）：把 parse 的 "3d" 臂删掉（落到
        // `_ => return None`）或指向别的目录 ⇒ ThreeD 那一轮的往返红（运行期）。
        assert_eq!(
            MethodDomain::parse(d.as_str()),
            Some(d),
            "{d:?} 的编码必须经 parse 回到自身"
        );
    }

    // 编码侧逐字：四串照 §187 原文（`docs/spec/04-method.md:117`、`:126`、
    // `:132`、`:138`）。
    // 红条件（编码侧，**取反档**）：把 as_str 的两臂返回同一个串 ⇒ 其中一个
    // 值的往返红（例：Video 臂返回 "software" 时 parse("software") 得 Software
    // ≠ Video），同臂的逐字断言也红。
    assert_eq!(
        MethodDomain::Software.as_str(),
        "software",
        "software/ 的目录名照 §187 原文"
    );
    assert_eq!(
        MethodDomain::Video.as_str(),
        "video",
        "video/ 的目录名照 §187 原文"
    );
    assert_eq!(
        MethodDomain::Research.as_str(),
        "research",
        "research/ 的目录名照 §187 原文"
    );
    // "3d" 单列一条逐字断言：四条里唯一可能被顺手写成 "3D" 的一条（见文件头）。
    assert_eq!(
        MethodDomain::ThreeD.as_str(),
        "3d",
        "3d/ 的目录名照 §187 原文，小写"
    );
}

#[test]
fn all_is_in_the_order_of_the_specs_directories() {
    // 次序**逐位**＝§187 原文的目录次序（`docs/spec/04-method.md:117`、`:126`、
    // `:132`、`:138`）：software → video → research → 3d。
    // 红条件（**取反档**）：对调 ALL 的任意两项 ⇒ 红。
    // **本条是上一条「集合比较缺口」的补丁**：上一条刻意不写集合比较
    // （集合比较与元素次序无关、对调两项它也绿），次序这件事只有逐位比对能钉。
    assert_eq!(
        MethodDomain::ALL,
        [
            MethodDomain::Software,
            MethodDomain::Video,
            MethodDomain::Research,
            MethodDomain::ThreeD,
        ],
        "ALL 必须逐位等于 §187 的目录次序 [software, video, research, 3d]"
    );
}

#[test]
fn parse_rejects_every_string_outside_the_catalog() {
    // 判据 2 的域级侧（设计 §10）：四个 as_str 之外的输入**逐串** None。
    // 红条件（**放宽档**）：给 parse 加一条通配臂 `_ => Some(MethodDomain::Software)`
    // ⇒ 下面的逐串断言全红。
    // 红条件（**放宽档**）：把 "image" 加成一个新臂（**本块未裁决，不得发明**）
    // ⇒ "image" 那一串红（实现比所钉的**少判一条**：原本该判 None 的串被收下了）。
    let outside = [
        "",          // 空串
        "Software",  // 大小写——目录名全小写
        "3D",        // "3d" 的大小写变体（四条里唯一有字母可大写的目录名）
        "3d ",       // 尾空格
        "software/", // 带斜杠的目录写法（§187 清单里目录名都带尾斜杠）
        // `"image"` 的来历：**设计 §8 R3 点名的缺口**——§187 与《总纲》§8.1
        // 都没有 image 目录，而 P5f 被要求有条目；本块未裁决，**不得发明**
        // image 臂。它不是为凑数加的样例串。
        "image",
        "images",
    ];
    for s in outside {
        assert_eq!(
            MethodDomain::parse(s),
            None,
            "目录表外的串必须解析失败：{s:?}"
        );
    }
}

#[test]
fn has_counterpart_is_true_for_each_paired_domain_and_false_for_three_d() {
    // 判据 6 的**域级侧**：配对是 software/↔P5c、research/↔P5d、video/↔P5e，
    // 3d/ 本轮无对家（设计 §5.1）。
    // **它是本设计的决定而不是规范断言**：设计 §4.1 末写「来源是当前分块，
    // 不是规范」——没有规范指定哪个域有对家。
    // 红条件（**取反档**）：ThreeD => true ⇒ 第四条红；三枚真值里的任一枚
    // 改成 false ⇒ 对应那条红。
    assert!(
        MethodDomain::Software.has_counterpart(),
        "software/ 有对家（P5c）"
    );
    assert!(
        MethodDomain::Video.has_counterpart(),
        "video/ 有对家（P5e）"
    );
    assert!(
        MethodDomain::Research.has_counterpart(),
        "research/ 有对家（P5d）"
    );
    assert!(
        !MethodDomain::ThreeD.has_counterpart(),
        "3d/ 本轮无对家（设计 §7）"
    );
}
