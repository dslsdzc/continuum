//! Task 1：两个取值类型 [`Ratio`] / [`SkillScore`] 的用例。
//!
//! 每条断言的「红的条件」见各用例的注释——这些注释不是说明，是变异时的靶子。

use continuum_model_registry::{ProfileError, Ratio, SkillScore};

/// `[0,1]` 闭区间内侧的往返：极小（含最小次正规）与极大（`1.0`，本类型的上界）都在内。
///
/// **被钉住的是往返，不是最短长度**——`1e300` 那类值打出三百多个字符是允许的契约。
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
/// 而 `NaN` 也不是「落在区间之外」的点。故这里**两枚变体各三格**，且互不重叠。
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
