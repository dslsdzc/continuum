use continuum_persist::{
    run_recovery, Db, PersistError, RecoveryHook, RecoveryPhase, RecoveryRegistry, Tx, Value,
};
use std::sync::{Arc, Mutex};

struct Recorder {
    phase: RecoveryPhase,
    label: &'static str,
    log: Arc<Mutex<Vec<&'static str>>>,
    writes: bool,
}

impl RecoveryHook for Recorder {
    fn phase(&self) -> RecoveryPhase {
        self.phase
    }

    fn run(&self, tx: &Tx<'_>) -> Result<(), PersistError> {
        self.log.lock().unwrap().push(self.label);
        if self.writes {
            tx.execute(
                "INSERT INTO probe (label) VALUES (?1)",
                &[Value::text(self.label)],
            )?;
        }
        Ok(())
    }
}

fn registry() -> (RecoveryRegistry, Arc<Mutex<Vec<&'static str>>>) {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut reg = RecoveryRegistry::new();
    // 注册顺序刻意打乱，断言仍按阶段顺序执行
    reg.register(Box::new(Recorder {
        phase: RecoveryPhase::ResumeEligibleTasks,
        label: "resume",
        log: log.clone(),
        writes: false,
    }));
    reg.register(Box::new(Recorder {
        phase: RecoveryPhase::ReconcileRunningNodes,
        label: "running-1",
        log: log.clone(),
        writes: true,
    }));
    reg.register(Box::new(Recorder {
        phase: RecoveryPhase::ReconcileIncompleteEffects,
        label: "effects",
        log: log.clone(),
        writes: true,
    }));
    reg.register(Box::new(Recorder {
        phase: RecoveryPhase::ReconcileRunningNodes,
        label: "running-2",
        log: log.clone(),
        writes: true,
    }));
    (reg, log)
}

fn db_with_probe() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut ms = continuum_persist::builtin_migrations();
    ms.push(continuum_persist::Migration::new(
        50,
        "probe",
        "CREATE TABLE probe (label TEXT NOT NULL);",
    ));
    let db = Db::open_with(&path, ms).unwrap();
    db.migrate().unwrap();
    (dir, db)
}

#[test]
fn five_phases_run_in_spec_319_order() {
    let (_d, db) = db_with_probe();
    let (reg, log) = registry();
    let report = run_recovery(&db, &reg).unwrap();

    assert_eq!(
        log.lock().unwrap().clone(),
        vec!["effects", "running-1", "running-2", "resume"],
        "钩子必须按 §319 的阶段顺序执行，同阶段内保持注册顺序"
    );
    assert_eq!(report.phases.len(), 5, "报告必须覆盖五个阶段");
    let names: Vec<&str> = report.phases.iter().map(|p| p.phase.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "load durable state",
            "reconcile incomplete effects",
            "reconcile running nodes",
            "mark lost executions",
            "resume eligible tasks",
        ]
    );
}

#[test]
fn phase_without_hooks_reports_zero() {
    let (_d, db) = db_with_probe();
    let (reg, _log) = registry();
    let report = run_recovery(&db, &reg).unwrap();
    let load = report
        .phases
        .iter()
        .find(|p| p.phase == RecoveryPhase::LoadDurableState)
        .unwrap();
    assert_eq!(load.hooks_run, 0);
    let lost = report
        .phases
        .iter()
        .find(|p| p.phase == RecoveryPhase::MarkLostExecutions)
        .unwrap();
    assert_eq!(lost.hooks_run, 0);
    let running = report
        .phases
        .iter()
        .find(|p| p.phase == RecoveryPhase::ReconcileRunningNodes)
        .unwrap();
    assert_eq!(running.hooks_run, 2);
}

#[test]
fn hook_writes_are_committed() {
    let (_d, db) = db_with_probe();
    let (reg, _log) = registry();
    run_recovery(&db, &reg).unwrap();

    let tx = db.begin().unwrap();
    let rows = tx.query("SELECT label FROM probe ORDER BY label", &[]).unwrap();
    assert_eq!(rows.len(), 3, "三个写入型钩子各写一行");
}

#[test]
fn failing_hook_stops_recovery_and_rolls_back_its_phase() {
    struct Boom;
    impl RecoveryHook for Boom {
        fn phase(&self) -> RecoveryPhase {
            RecoveryPhase::ReconcileIncompleteEffects
        }
        fn run(&self, tx: &Tx<'_>) -> Result<(), PersistError> {
            tx.execute(
                "INSERT INTO probe (label) VALUES (?1)",
                &[Value::text("boom")],
            )?;
            Err(PersistError::Database("钩子失败".into()))
        }
    }

    let (_d, db) = db_with_probe();
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut reg = RecoveryRegistry::new();
    reg.register(Box::new(Recorder {
        phase: RecoveryPhase::ReconcileRunningNodes,
        label: "after",
        log: log.clone(),
        writes: true,
    }));
    reg.register(Box::new(Boom));

    run_recovery(&db, &reg).expect_err("钩子失败必须向上传播");
    assert!(
        log.lock().unwrap().is_empty(),
        "后续阶段不得执行"
    );
    let tx = db.begin().unwrap();
    let rows = tx.query("SELECT label FROM probe", &[]).unwrap();
    assert!(rows.is_empty(), "失败阶段自身的写入必须回滚");
}

const INSERT_EVENT: &str =
    "INSERT INTO events
       (event_id, event_type, schema_version, occurred_at, intent_id, node_id, ignorable, payload)
     VALUES (?1, ?2, ?3, ?4, NULL, NULL, ?5, ?6)";

#[test]
fn skipped_records_are_reported() {
    let (_d, db) = db_with_probe();
    let tx = db.begin().unwrap();
    // schema_version 写成文本，解码失败但不致命
    tx.execute(
        INSERT_EVENT,
        &[
            Value::text("e1"),
            Value::text("node.started"),
            Value::text("one"),
            Value::Int(1),
            Value::Int(0),
            Value::text("{}"),
        ],
    )
    .unwrap();
    tx.commit().unwrap();

    let report = run_recovery(&db, &RecoveryRegistry::new()).unwrap();
    assert_eq!(report.skipped_records, 1);
    assert_eq!(report.phases.len(), 5, "跳过记录不得改变阶段覆盖");
}

#[test]
fn fatal_event_log_stops_recovery_before_any_phase() {
    let (_d, db) = db_with_probe();
    let tx = db.begin().unwrap();
    tx.execute(
        INSERT_EVENT,
        &[
            Value::text("e1"),
            Value::text("future.thing"),
            Value::Int(1),
            Value::Int(1),
            Value::Int(0),
            Value::text("{}"),
        ],
    )
    .unwrap();
    tx.commit().unwrap();

    let log = Arc::new(Mutex::new(Vec::new()));
    let mut reg = RecoveryRegistry::new();
    reg.register(Box::new(Recorder {
        phase: RecoveryPhase::ResumeEligibleTasks,
        label: "resume",
        log: log.clone(),
        writes: false,
    }));

    run_recovery(&db, &reg).expect_err("致命的事件日志必须中止启动");
    assert!(
        log.lock().unwrap().is_empty(),
        "致命判定后不得执行任何阶段"
    );
}
