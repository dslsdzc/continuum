//! 迁移 190：记忆的八表六索引（设计 §9.2）。
//!
//! 一次写齐：八条 `CREATE TABLE` 与六条 `CREATE INDEX` 全在一个 `&'static str` 里。
//! 建表顺序让外键的参照表先建。**Task 12 之后本 SQL 不得再改**（`Db::migrate()`
//! 按 version 跳过已应用者）。
//!
//! 列清单逐字照设计 §9.2；可空列只有 `removed_at`／`anchor_id`／`finished_at`／`report`。

use continuum_persist::Migration;

/// 迁移 190（函数名见设计 §9.1）。版本恰 `[190]`，名称 `p6c_memory`。
pub fn p6c_memory_migrations() -> Vec<Migration> {
    vec![Migration::new(190, "p6c_memory", "CREATE TABLE memory_source (
            source_id   TEXT PRIMARY KEY NOT NULL,
            kind        TEXT NOT NULL,
            external_ref TEXT NOT NULL,
            observed_at INTEGER NOT NULL,
            removed_at  INTEGER
        );
        CREATE INDEX idx_memory_source_kind ON memory_source(kind);
        CREATE INDEX idx_memory_source_ref ON memory_source(external_ref);
        CREATE TABLE memory_claim (
            claim_id       TEXT PRIMARY KEY NOT NULL,
            scope          TEXT NOT NULL,
            statement      TEXT NOT NULL,
            validity       TEXT NOT NULL,
            confidence     TEXT NOT NULL,
            last_confirmed INTEGER NOT NULL
        );
        CREATE INDEX idx_memory_claim_scope_validity_time
            ON memory_claim(scope, validity, last_confirmed);
        CREATE TABLE memory_claim_source (
            claim_id  TEXT NOT NULL,
            source_id TEXT NOT NULL,
            PRIMARY KEY (claim_id, source_id),
            FOREIGN KEY (claim_id) REFERENCES memory_claim(claim_id),
            FOREIGN KEY (source_id) REFERENCES memory_source(source_id)
        );
        CREATE INDEX idx_memory_claim_source_source ON memory_claim_source(source_id);
        CREATE TABLE memory_claim_anchor (
            claim_id  TEXT PRIMARY KEY NOT NULL,
            anchor_id TEXT NOT NULL,
            FOREIGN KEY (claim_id) REFERENCES memory_claim(claim_id)
        );
        CREATE INDEX idx_memory_claim_anchor ON memory_claim_anchor(anchor_id);
        CREATE TABLE memory_correction (
            source_id   TEXT PRIMARY KEY NOT NULL,
            claim_id    TEXT NOT NULL UNIQUE,
            statement   TEXT NOT NULL,
            occurred_at INTEGER NOT NULL,
            FOREIGN KEY (source_id) REFERENCES memory_source(source_id),
            FOREIGN KEY (claim_id) REFERENCES memory_claim(claim_id)
        );
        CREATE TABLE memory_synthesis_job (
            job_id       TEXT PRIMARY KEY NOT NULL,
            scope        TEXT NOT NULL,
            anchor_id    TEXT,
            trigger_kind TEXT NOT NULL,
            trigger_ref  TEXT NOT NULL,
            status       TEXT NOT NULL,
            started_at   INTEGER NOT NULL,
            finished_at  INTEGER,
            report       TEXT
        );
        CREATE TABLE memory_synthesis_input (
            job_id    TEXT NOT NULL,
            source_id TEXT NOT NULL,
            PRIMARY KEY (job_id, source_id),
            FOREIGN KEY (job_id) REFERENCES memory_synthesis_job(job_id),
            FOREIGN KEY (source_id) REFERENCES memory_source(source_id)
        );
        CREATE TABLE memory_summary (
            summary_id   TEXT PRIMARY KEY NOT NULL,
            scope        TEXT NOT NULL,
            anchor_id    TEXT,
            body         TEXT NOT NULL,
            job_id       TEXT NOT NULL,
            generated_at INTEGER NOT NULL,
            stale        INTEGER NOT NULL,
            FOREIGN KEY (job_id) REFERENCES memory_synthesis_job(job_id)
        );
        CREATE INDEX idx_memory_summary_scope_time
            ON memory_summary(scope, anchor_id, generated_at);",
    )]
}
