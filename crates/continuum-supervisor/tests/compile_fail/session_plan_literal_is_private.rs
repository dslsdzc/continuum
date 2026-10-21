use continuum_supervisor::{BackgroundDigest, SessionMode, SessionPlan};

fn main() {
    let _ = SessionPlan {
        mode: SessionMode::Manual,
        candidates: vec![],
        decision: None,
        background: BackgroundDigest::default(),
    };
}
