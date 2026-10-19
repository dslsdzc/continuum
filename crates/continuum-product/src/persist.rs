//! 迁移 200：产品循环四表（`product`／`milestone`／`product_progress`／
//! `product_drive`）。
//!
//! C2=(b) 已批准：`ContinuousProductDrive` 落在独立的 `product_drive` 表，不是
//! `Product` 的第八个字段。**本 SQL 一经应用不得再改**（`Db::migrate()` 按版本
//! 跳过已应用者）；P7A 只占 200，不占 201–209。
//!
//! 列名与顺序逐字照设计 §4.9。

use continuum_persist::Migration;

/// 迁移 200，名称 `p7a_product`。版本恰 `[200]`。
pub fn p7a_product_migrations() -> Vec<Migration> {
    vec![Migration::new(
        200,
        "p7a_product",
        "CREATE TABLE product (
            id                   TEXT PRIMARY KEY NOT NULL,
            intent_id            TEXT NOT NULL UNIQUE,
            baseline             TEXT NOT NULL,
            readiness            TEXT NOT NULL,
            current_milestone    TEXT,
            shippable_predicate  TEXT NOT NULL,
            created_at           INTEGER NOT NULL,
            updated_at           INTEGER NOT NULL
        );
        CREATE TABLE milestone (
            id                   TEXT PRIMARY KEY NOT NULL,
            product_id           TEXT NOT NULL,
            seq                  INTEGER NOT NULL,
            objective            TEXT NOT NULL,
            dependencies         TEXT NOT NULL,
            entry_conditions     TEXT NOT NULL,
            exit_conditions      TEXT NOT NULL,
            state                TEXT NOT NULL,
            created_at           INTEGER NOT NULL,
            updated_at           INTEGER NOT NULL,
            UNIQUE (product_id, seq)
        );
        CREATE TABLE product_progress (
            product_id             TEXT NOT NULL,
            milestone_id           TEXT NOT NULL,
            previous_product_state TEXT NOT NULL,
            current_product_state  TEXT NOT NULL,
            progress_delta         TEXT NOT NULL,
            recorded_at            INTEGER NOT NULL,
            PRIMARY KEY (product_id, milestone_id)
        );
        CREATE TABLE product_drive (
            product_id                TEXT PRIMARY KEY NOT NULL,
            continuous_product_drive  INTEGER NOT NULL,
            phase                     TEXT NOT NULL,
            updated_at                INTEGER NOT NULL
        );
        CREATE INDEX idx_milestone_product_seq ON milestone(product_id, seq);",
    )]
}
