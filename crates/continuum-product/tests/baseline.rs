//! §150 十字段与生成端口的构造面。

use async_trait::async_trait;
use continuum_canonical::Opaque;
use continuum_product::{
    BaselineRequest, EngineeringBaseline, EngineeringBaselineGenerator, PortError,
};

fn field_names(src: &str, header: &str) -> Vec<String> {
    let start = src.find(header).expect("找不到结构体定义") + header.len();
    let rest = &src[start..];
    let end = rest.find("\n}").expect("结构体没有闭合");
    rest[..end]
        .lines()
        .filter_map(|line| {
            let t = line.trim();
            if t.starts_with("pub ") {
                Some(
                    t["pub ".len()..]
                        .split(':')
                        .next()
                        .expect("字段行没有冒号")
                        .trim()
                        .to_owned(),
                )
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn the_ten_baseline_fields_are_verbatim_in_order() {
    let src = include_str!("../src/baseline.rs");
    assert_eq!(
        field_names(src, "pub struct EngineeringBaseline {"),
        [
            "product_goal",
            "target_users",
            "required_capabilities",
            "hard_constraints",
            "architecture",
            "technology_choices",
            "minimum_complete_system",
            "known_unknowns",
            "engineering_risks",
            "acceptance_criteria",
        ]
    );
}

#[test]
fn a_port_error_carries_its_message() {
    let e = PortError::new("model unavailable");
    assert_eq!(e.message(), "model unavailable");
    assert_eq!(e.clone(), e);
}

struct FixedGenerator {
    result: Result<EngineeringBaseline, PortError>,
}

#[async_trait]
impl EngineeringBaselineGenerator for FixedGenerator {
    async fn generate(&self, _request: &BaselineRequest) -> Result<EngineeringBaseline, PortError> {
        self.result.clone()
    }
}

fn baseline() -> EngineeringBaseline {
    EngineeringBaseline {
        product_goal: Opaque::from_str("ship a product"),
        target_users: vec![Opaque::from_str("developers")],
        required_capabilities: vec![Opaque::from_str("loop")],
        hard_constraints: vec![],
        architecture: Opaque::from_str("layered"),
        technology_choices: vec![Opaque::from_str("rust")],
        minimum_complete_system: vec![Opaque::from_str("baseline")],
        known_unknowns: vec![],
        engineering_risks: vec![Opaque::from_str("scope")],
        acceptance_criteria: vec![Opaque::from_str("tests pass")],
    }
}

#[tokio::test]
async fn the_generator_trait_is_usable_from_an_async_test() {
    let generator = FixedGenerator {
        result: Ok(baseline()),
    };
    let request = BaselineRequest {
        idea: Opaque::from_str("an idea"),
        constraints: vec![Opaque::from_str("no network")],
    };
    let got = generator.generate(&request).await.unwrap();
    assert_eq!(got.product_goal.as_str(), "ship a product");
    assert_eq!(got.acceptance_criteria[0].as_str(), "tests pass");
}
