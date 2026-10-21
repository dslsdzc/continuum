//! Task 13：`SynthesisIo` 端口与 §171 的三个转发。

mod support;

use continuum_memory::{
    MemoryError, MemoryScope, MemorySourceId, SynthesisJobId, SynthesisProposal, SynthesisReport,
    SynthesisRequest, SynthesisTrigger,
};
use continuum_supervisor::{SupervisorError, SynthesisTriggerPoint};

use support::{code_only, fn_body, CountingIo, FailingIo};

fn request() -> SynthesisRequest {
    SynthesisRequest {
        job: SynthesisJobId::new("job-1"),
        scope: MemoryScope::Project,
        anchor: None,
        trigger: SynthesisTrigger::UserRequested,
        sources: vec![MemorySourceId::new("source-1")],
        started_at: 0,
    }
}

fn report() -> SynthesisReport {
    SynthesisReport {
        created: vec![],
        merged: vec![],
        superseded: vec![],
        summary_written: true,
        staled_summaries: 0,
    }
}

#[test]
fn begin_apply_fail_forward_exactly_once() {
    let io = CountingIo::new(report());
    let trigger = SynthesisTriggerPoint;
    let request = request();
    trigger
        .begin_synthesis(&io, &request)
        .expect("begin 必须转发成功");
    let proposal = SynthesisProposal {
        claims: vec![],
        summary: None,
    };
    let job = SynthesisJobId::new("job-1");
    let applied = trigger
        .apply_synthesis(&io, &job, &proposal, 10)
        .expect("apply 必须转发成功");
    assert_eq!(applied, report());
    trigger
        .fail_synthesis(&io, &job, "取消", 11)
        .expect("fail 必须转发成功");

    assert_eq!(io.begin_calls.get(), 1);
    assert_eq!(io.apply_calls.get(), 1);
    assert_eq!(io.fail_calls.get(), 1);

    let copied = trigger;
    copied
        .begin_synthesis(&io, &request)
        .expect("复制后的触发点必须可用");
    assert_eq!(io.begin_calls.get(), 2, "Copy 不改变计数，只多一次调用");
}

#[test]
fn memory_errors_are_wrapped_not_swallowed() {
    let trigger = SynthesisTriggerPoint;
    let request = request();
    let proposal = SynthesisProposal {
        claims: vec![],
        summary: None,
    };
    let job = SynthesisJobId::new("job-1");

    let begin = trigger.begin_synthesis(&FailingIo, &request).unwrap_err();
    assert!(matches!(
        begin,
        SupervisorError::Memory(MemoryError::UnknownJob)
    ));
    let apply = trigger
        .apply_synthesis(&FailingIo, &job, &proposal, 10)
        .unwrap_err();
    assert!(matches!(
        apply,
        SupervisorError::Memory(MemoryError::UnknownJob)
    ));
    let fail = trigger
        .fail_synthesis(&FailingIo, &job, "取消", 11)
        .unwrap_err();
    assert!(matches!(
        fail,
        SupervisorError::Memory(MemoryError::UnknownJob)
    ));
}

#[test]
fn trigger_point_has_no_clock_retry_or_threshold() {
    let corpus = [
        include_str!("../src/lib.rs"),
        include_str!("../src/task.rs"),
        include_str!("../src/supervisor.rs"),
        include_str!("../src/session.rs"),
        include_str!("../src/resource.rs"),
        include_str!("../src/trigger.rs"),
        include_str!("../src/error.rs"),
    ]
    .map(code_only)
    .join("\n");
    for banned in [
        "Duration",
        "Instant",
        "SystemTime::now",
        "thread::sleep",
        "retry",
        "interval",
        "idle",
        "threshold",
    ] {
        assert!(!corpus.contains(banned), "src 不应出现 {banned}");
    }

    let trigger = code_only(include_str!("../src/trigger.rs"));
    for banned in ["impl SynthesisIo", "enum SynthesisTrigger"] {
        assert!(!trigger.contains(banned), "trigger.rs 不应出现 {banned}");
    }
    for name in ["begin_synthesis", "apply_synthesis", "fail_synthesis"] {
        let body = fn_body(&trigger, name);
        assert_eq!(
            body.matches("io.").count(),
            1,
            "{name} 只应转发一次：{body}"
        );
    }
}
