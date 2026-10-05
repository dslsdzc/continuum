//! Task 1：两个取值类型 [`Ratio`] / [`SkillScore`] 的用例。
//!
//! 每条断言的「红的条件」见各用例的注释——这些注释不是说明，是变异时的靶子。

use continuum_model_registry::{
    current_observation, ProfileError, Ratio, SkillDimension, SkillObservation, SkillScore,
    SkillVector,
};

/// §248 的九维，**逐项列出**（不抽代表）：凡「九维」「其余八维」的断言都遍历它。
///
/// **这张表是手写的，加维度时必须手工同步**：`src` 侧加第十维会让 `as_str` / `parse` /
/// `index` 的穷尽 `match` 编译失败，**但不会让这张表编译失败**——新维度会在这里静默漏掉照片
/// （编码用例里「表里漏了 {dimension:?}」那条断言遍历的正是本表，故它也跟着一起漏）。
/// 这个点不设防，靠后来者自己记得。
const ALL_DIMENSIONS: [SkillDimension; 9] = [
    SkillDimension::Reasoning,
    SkillDimension::Coding,
    SkillDimension::Vision,
    SkillDimension::Planning,
    SkillDimension::ToolUse,
    SkillDimension::ConstraintFollowing,
    SkillDimension::Verification,
    SkillDimension::Spatial,
    SkillDimension::Media,
];

/// 用例内的一次观测。断言里要区分的五个字段都给**互不相同**的值，
/// 免得「取错了哪一条」被两个字段恰好相等掩盖过去。
fn observation(
    score: f64,
    confidence: f64,
    sample_count: u64,
    version: u32,
    time_range: (i64, i64),
) -> SkillObservation {
    SkillObservation::try_new(
        SkillScore::try_new(score).unwrap_or_else(|e| panic!("{score} 应是合法评分，实为 {e:?}")),
        Ratio::try_new(confidence)
            .unwrap_or_else(|e| panic!("{confidence} 应是合法置信度，实为 {e:?}")),
        sample_count,
        version,
        time_range,
    )
    .unwrap_or_else(|e| panic!("{time_range:?} 应是自洽的时间窗，实为 {e:?}"))
}

/// `[0,1]` 闭区间内侧的往返：极小（含最小次正规）与极大（`1.0`，本类型的上界）都在内。
///
/// **被钉住的是往返，不是最短长度**——`1e300` 那类值打出三百多个字符是允许的契约。
///
/// **`Debug` 之所以不在红的条件里**（`SkillScore` 那条同一对函数，同理）：Rust 的 `f64` 的
/// `Debug` 与 `Display` 是**同一套最短精确往返算法**，把 `as_str` 换成 `{:?}` 各取值丢精度 0 格、
/// 跑全量 `exit=0` 不变红。这**不是等价变异体**——两版输出确实不同（`1e300`：`Display` 三百多个
/// 字符 vs `Debug` 的 `1e300`），只是**本用例没有观察长度**；而设计 §2.4 只声称**往返**、
/// 不声称**最短长度**，故**不为它补用例**（补了就是给契约加一句本不存在的绝对措辞）。
/// 真正红的条件是**截断格式**（如 `{:.2}`）。
#[test]
fn ratio_round_trips_through_its_text_encoding() {
    let values = [
        0.0,
        0.5,
        0.94,
        1.0, // 上界：合法值。把它写成 `v >= 1.0` 的守卫会在这里红。
        f64::MIN_POSITIVE,
        f64::from_bits(1), // 最小次正规（约 5e-324），往返的最难一格
        1.0 - f64::EPSILON,
        f64::EPSILON,
    ];
    for v in values {
        let r = Ratio::try_new(v).unwrap_or_else(|e| panic!("{v} 应是合法取值，实为 {e:?}"));
        assert_eq!(
            Ratio::parse(&r.as_str()),
            Some(r),
            "{v} 经 `{}` 未能往返——`as_str` 用了截断格式或非精确格式",
            r.as_str()
        );
    }
}

/// 两件不同的事各有照片：**非有限**（`NaN` / `±∞`）报 `NotFinite`，
/// **有限但越界**（`1.5` / `-0.1`）报 `OutOfRange { value }`。
///
/// 两类**不共用变体**：同一个 `NaN` 在 `Ratio` 与 `SkillScore` 上必须报同一枚错，
/// 而 `NaN` 也不是「落在区间之外」的点。故这里**非有限三格、越界两格**，且互不重叠。
#[test]
fn ratio_rejects_non_finite_and_out_of_range() {
    // 非有限三格：`NaN` 用 `match` 而非 `assert_eq!`——`NaN != NaN`，
    // 等值断言在 `value` 这一格恒假；但**变体**仍被逐字钉住。
    match Ratio::try_new(f64::NAN) {
        Err(ProfileError::NotFinite) => {}
        other => panic!("NaN 应被拒为 NotFinite，实为 {other:?}"),
    }
    assert_eq!(Ratio::try_new(f64::INFINITY), Err(ProfileError::NotFinite));
    assert_eq!(
        Ratio::try_new(f64::NEG_INFINITY),
        Err(ProfileError::NotFinite)
    );

    // 有限越界两格：`value` 是**给的那个数**。
    assert_eq!(
        Ratio::try_new(1.5),
        Err(ProfileError::OutOfRange { value: 1.5 })
    );
    assert_eq!(
        Ratio::try_new(-0.1),
        Err(ProfileError::OutOfRange { value: -0.1 })
    );
}

/// 同一对函数，另一侧的守卫：本类型的域**无界**，故负数与极大值都必须往返。
/// 红的条件同 `ratio_round_trips_through_its_text_encoding`（截断格式；`Debug` 不变红，见那条的注释）。
#[test]
fn skill_score_round_trips_through_its_text_encoding() {
    let values = [
        9.2, // §83 的示例（`docs/spec/02-positioning.md:773`）
        0.0,
        -3.5, // 方向未定义：负值同样合法
        f64::MAX,
        f64::MIN,
        f64::MIN_POSITIVE,
        f64::from_bits(1),
        -f64::from_bits(1),
    ];
    for v in values {
        let s = SkillScore::try_new(v).unwrap_or_else(|e| panic!("{v} 应是合法取值，实为 {e:?}"));
        assert_eq!(
            SkillScore::parse(&s.as_str()),
            Some(s),
            "{v} 经 `{}` 未能往返——`as_str` 用了截断格式或非精确格式",
            s.as_str()
        );
    }
}

/// `NaN` / `±∞` 三格各一条照片，且断言的是**哪一枚**（`NotFinite`，不是别的）。
#[test]
fn skill_score_rejects_nan_and_infinities() {
    assert_eq!(SkillScore::try_new(f64::NAN), Err(ProfileError::NotFinite));
    assert_eq!(
        SkillScore::try_new(f64::INFINITY),
        Err(ProfileError::NotFinite)
    );
    assert_eq!(
        SkillScore::try_new(f64::NEG_INFINITY),
        Err(ProfileError::NotFinite)
    );
}

/// `parse` 是 `as_str` 的**严格逆**：非数值、非有限、越界一律 `None`，**不取默认值**。
///
/// 这是 fail-open 的那一侧：把 `None` 写成 `Some(Self(0.0))`（取默认值）是一处
/// 只在这个用例里才可见的漂移。
#[test]
fn parse_is_strict_inverse_rejecting_non_values() {
    // 两类型共有的拒收格：非数值、空串、溢出成 ±∞、`NaN` 字面量。
    for s in ["", "abc", "1e400", "-1e400", "NaN", "nan", "inf", "-inf"] {
        assert_eq!(Ratio::parse(s), None, "Ratio::parse({s:?}) 应拒收");
        assert_eq!(SkillScore::parse(s), None, "SkillScore::parse({s:?}) 应拒收");
    }
    // `Ratio` 独有：数值但越界。
    for s in ["1.5", "-0.1", "2"] {
        assert_eq!(Ratio::parse(s), None, "Ratio::parse({s:?}) 应拒收");
    }
    // 对照臂：同一批字面量在 `SkillScore` 这一侧是**合法**的（域无界），
    // 免得上面的循环被读成「`parse` 什么都不收」。
    assert_eq!(SkillScore::parse("1.5"), SkillScore::try_new(1.5).ok());
    assert_eq!(SkillScore::parse("-0.1"), SkillScore::try_new(-0.1).ok());
}

/// `SkillScore` 的**全序**：类型文档声称的那一件事，逐项有照片。
///
/// 顺序按数值；相等的两个取值（含 `-0.0` 与 `0.0`）判 `Equal`——`Ord` 与 `Eq` 必须一致。
#[test]
fn skill_score_is_totally_ordered() {
    let ordered = [-3.5, -1.0, 0.0, 0.5, 9.2, f64::MAX];
    let scores: Vec<SkillScore> = ordered
        .iter()
        .map(|v| SkillScore::try_new(*v).expect("构造期应接受有限值"))
        .collect();
    for (i, a) in scores.iter().enumerate() {
        for (j, b) in scores.iter().enumerate() {
            let expected = i.cmp(&j);
            assert_eq!(
                a.cmp(b),
                expected,
                "{} 与 {} 的比较次序不符",
                ordered[i],
                ordered[j]
            );
            assert_eq!(a.partial_cmp(b), Some(expected));
            assert_eq!(a == b, expected.is_eq());
        }
    }
    // `-0.0` 与 `0.0`：`==` 判相等，故次序也必须是 `Equal`（否则 `Ord` 与 `Eq` 不一致）。
    let neg_zero = SkillScore::try_new(-0.0).expect("构造期应接受 -0.0");
    let zero = SkillScore::try_new(0.0).expect("构造期应接受 0.0");
    assert_eq!(neg_zero.cmp(&zero), std::cmp::Ordering::Equal);
    assert!(neg_zero == zero);
    // 排序可用：`sort` 需要一个全序（这正是 `Ratio` / `SkillScore` 拒 NaN 的理由）。
    let mut shuffled = scores.clone();
    shuffled.reverse();
    shuffled.sort();
    assert_eq!(shuffled, scores);
}

// ============================================================================
// Task 2：九维、观测与「缺席不是 0」
// ============================================================================

/// 「缺席不是 0」：未画像的向量**九维逐项**为 `None`——不抽代表、不给默认值。
///
/// 这条钉的是设计 §2.2 的第一条：`None`（没有观测）与 `SkillScore(0.0)`（评分是 0）
/// 是两件事。红的条件：把 `Option` 换成默认值（如全维填 `SkillScore(0.0)` 的观测）即红
/// ——那样一个未画像的模型看起来「所有维度都很差」，在 §23 的语义下与「很强」一样是假的。
#[test]
fn every_dimension_is_absent_before_any_observation() {
    let vector = SkillVector::from_current(Vec::new());
    for dimension in ALL_DIMENSIONS {
        assert!(
            vector.get(dimension).is_none(),
            "{dimension:?} 在未画像的向量上应是 None（缺席不是 0），实为 {:?}",
            vector.get(dimension)
        );
    }
}

/// 写入一维后，**其余八维逐项**仍为 `None`；写入的那一维也逐项确认落地。
///
/// 遍历九个维度各写一次（不是抽一个代表）：这道守卫抓**下标越界**（`index()` 里某臂写成
/// `9` 那类，`get` 处直接 panic）与**两臂撞到同一格**（那一格串味）——只这两类能靠本用例抓。
///
/// **两臂互换抓不到**，别把置换当成这里钉的东西：`dimensions` 私有，`from_current` 写入与
/// `get` 读出走的是同一个 `index()`，且没有用例格式化整个向量，故**任意置换在可观察行为上
/// 完全等价**（跑出来是绿的）。
#[test]
fn one_observation_leaves_the_other_eight_absent() {
    for written in ALL_DIMENSIONS {
        let vector = SkillVector::from_current(vec![(written, observation(1.0, 0.5, 3, 1, (0, 10)))]);
        for dimension in ALL_DIMENSIONS {
            if dimension == written {
                assert!(
                    vector.get(dimension).is_some(),
                    "{dimension:?} 刚写入，不该是 None"
                );
            } else {
                assert!(
                    vector.get(dimension).is_none(),
                    "只写了 {written:?}，{dimension:?} 不该有观测（下标错位？），实为 {:?}",
                    vector.get(dimension)
                );
            }
        }
    }
}

/// 同一维度给两次：**覆盖**，不是合并——后给的那条胜。
///
/// 照片对应 [`SkillVector::from_current`] 文档里「同一维度给两次即覆盖前一次，不合并」那句话。
#[test]
fn a_second_observation_for_the_same_dimension_overwrites_the_first() {
    let vector = SkillVector::from_current(vec![
        (SkillDimension::Coding, observation(1.0, 0.1, 1, 1, (0, 10))),
        (SkillDimension::Coding, observation(9.0, 0.9, 2, 2, (20, 30))),
    ]);
    let got = vector
        .get(SkillDimension::Coding)
        .expect("写过 Coding，应有一条观测");
    assert_eq!(got.score().get(), 9.0, "后给的那条应覆盖前一条，不合并");
    assert_eq!(got.version(), 2);
    assert_eq!(got.time_range(), (20, 30));
    // 覆盖只发生在同一维：其余八维仍空。
    for dimension in ALL_DIMENSIONS {
        if dimension != SkillDimension::Coding {
            assert!(vector.get(dimension).is_none(), "{dimension:?} 不该有观测");
        }
    }
}

/// 「当前值」的判据是 `version` **最大**的那次观测，**不是** `time_range` 最晚的那次。
///
/// 三条观测的 `version` 大小与 `time_range` 早晚**刻意相反**（设计 §2.2 记的选择，
/// §11 第 17 条）：`version` 是 §24 列出的字段里唯一由产生方显式递增的量。
///
/// 两种排列各断言一次：取「第一条」或「最后一条」的实现也会红，不只是取时间最晚的那种。
/// 五个字段各断言一次（访问器逐个有照片）。
#[test]
fn the_current_observation_is_the_highest_version_not_the_latest_time_range() {
    let latest_time_lowest_version = observation(1.0, 0.1, 11, 2, (3000, 4000));
    let highest_version = observation(2.0, 0.2, 22, 9, (1000, 2000));
    let earliest_time = observation(3.0, 0.3, 33, 5, (0, 500));

    let forward = vec![
        latest_time_lowest_version.clone(),
        highest_version.clone(),
        earliest_time.clone(),
    ];
    let backward = vec![earliest_time, highest_version, latest_time_lowest_version];

    for (label, series) in [("正序", forward), ("倒序", backward)] {
        let got = current_observation(&series)
            .unwrap_or_else(|| panic!("{label}：非空序列必有当前观测"));
        assert_eq!(got.version(), 9, "{label}：当前观测应取 version 最大的那条");
        assert_eq!(got.score().get(), 2.0, "{label}：取错了观测");
        assert_eq!(got.confidence().get(), 0.2, "{label}：取错了观测");
        assert_eq!(got.sample_count(), 22, "{label}：取错了观测");
        assert_eq!(
            got.time_range(),
            (1000, 2000),
            "{label}：时间窗更早的那条才是当前值——按 time_range 取最大即红"
        );
    }
}

/// 空序列没有当前观测：`None`，不是 panic、不是默认值。
#[test]
fn an_empty_series_has_no_current_observation() {
    assert!(current_observation(&[]).is_none());
}

/// 九个维度的落库编码**逐项**断言字面量，并在同一用例里反向 `parse` 回各自的变体。
///
/// 红的条件：多词项写成 `toolUse` 或 `tool-use` 即红——字面量是**手册写的**，钉的是格式；
/// 反向那半用的是被测函数，钉的是路径。表里还要覆盖全九维（漏一臂即红）。
#[test]
fn skill_dimension_encoding_is_lowercase_with_underscores() {
    let table: [(SkillDimension, &str); 9] = [
        (SkillDimension::Reasoning, "reasoning"),
        (SkillDimension::Coding, "coding"),
        (SkillDimension::Vision, "vision"),
        (SkillDimension::Planning, "planning"),
        (SkillDimension::ToolUse, "tool_use"),
        (SkillDimension::ConstraintFollowing, "constraint_following"),
        (SkillDimension::Verification, "verification"),
        (SkillDimension::Spatial, "spatial"),
        (SkillDimension::Media, "media"),
    ];

    for dimension in ALL_DIMENSIONS {
        assert!(
            table.iter().any(|(d, _)| *d == dimension),
            "{dimension:?} 在编码表里漏了——九维要逐项有照片"
        );
    }
    for (dimension, literal) in table {
        assert_eq!(
            dimension.as_str(),
            literal,
            "{dimension:?} 的落库编码不符（小写、多词 `_` 连接）"
        );
        assert_eq!(
            SkillDimension::parse(literal),
            Some(dimension),
            "{literal:?} 应反解回 {dimension:?}"
        );
    }
}

/// 表外维度名一律 `None`，**不取默认维度**——把表外串猜成某一维会成为第二份表示。
///
/// `"visual"` 是最像的一个（§22 的画像清单里有 `vision`），其余几格钉的是格式的几种
/// 近似写法：大小写、连字符、全大写、空串、前导空格。
#[test]
fn an_unknown_dimension_name_is_rejected() {
    for s in ["visual", "ToolUse", "tool-use", "TOOL_USE", "", " reasoning"] {
        assert_eq!(
            SkillDimension::parse(s),
            None,
            "{s:?} 不是九维的落库编码，应拒收为 None"
        );
    }
}

/// 时间窗反序被拒：`(200, 100)` → `BadTimeRange { start: 200, end: 100 }`，**两个端点值都断言**
/// （设计 §2.4：「外加时间窗反序一条」）。
///
/// **两侧对钉**：`(100, 200)` 与**退化区间** `(100, 100)` 各返回 `Ok`——`time_range` 是
/// **闭区间**，`start == end` 是自洽的。只写拒的那一侧不算钉住：
/// 去掉 `end < start` 判定即第一条红；把判定写成 `end <= start` 则只有退化区间那一条红。
#[test]
fn an_observation_whose_time_range_is_reversed_is_rejected() {
    let reversed = SkillObservation::try_new(
        SkillScore::try_new(9.2).expect("9.2 应是合法评分"),
        Ratio::try_new(0.5).expect("0.5 应是合法置信度"),
        7,
        3,
        (200, 100),
    );
    assert_eq!(
        reversed,
        Err(ProfileError::BadTimeRange {
            start: 200,
            end: 100
        }),
        "反序时间窗应被拒，且两个端点值原样带回"
    );

    // 放行的一侧：正序。
    assert!(
        SkillObservation::try_new(
            SkillScore::try_new(9.2).expect("9.2 应是合法评分"),
            Ratio::try_new(0.5).expect("0.5 应是合法置信度"),
            7,
            3,
            (100, 200),
        )
        .is_ok(),
        "正序时间窗应被接受"
    );
    // 放行的一侧：退化区间（闭区间上的一个点）。
    assert!(
        SkillObservation::try_new(
            SkillScore::try_new(9.2).expect("9.2 应是合法评分"),
            Ratio::try_new(0.5).expect("0.5 应是合法置信度"),
            7,
            3,
            (100, 100),
        )
        .is_ok(),
        "退化区间 start == end 应被接受（闭区间是自洽的）"
    );
}
