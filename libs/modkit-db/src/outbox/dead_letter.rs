use std::fmt::Write as _;

use sea_orm::{ConnectionTrait, DbBackend, FromQueryResult, Statement, TransactionTrait};

use super::dialect::Dialect;
use super::types::OutboxError;
use crate::Db;

/// A dead-lettered message with self-contained payload.
#[derive(Debug, FromQueryResult)]
pub struct DeadLetterMessage {
    pub id: i64,
    pub partition_id: i64,
    pub seq: i64,
    pub payload: Vec<u8>,
    pub payload_type: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub failed_at: chrono::DateTime<chrono::Utc>,
    pub last_error: Option<String>,
    pub attempts: i16,
    pub replayed_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Filter for dead letter queries.
pub struct DeadLetterFilter {
    pub partition_id: Option<i64>,
    pub queue: Option<String>,
    pub failed_after: Option<chrono::DateTime<chrono::Utc>>,
    pub failed_before: Option<chrono::DateTime<chrono::Utc>>,
    /// Filter to entries where `replayed_at IS NULL` (default: true).
    pub only_pending: bool,
    pub limit: Option<u32>,
}

impl Default for DeadLetterFilter {
    fn default() -> Self {
        Self {
            partition_id: None,
            queue: None,
            failed_after: None,
            failed_before: None,
            only_pending: true,
            limit: None,
        }
    }
}

/// List dead-lettered messages with optional filtering.
pub async fn dead_letter_list(
    db: &Db,
    filter: &DeadLetterFilter,
) -> Result<Vec<DeadLetterMessage>, OutboxError> {
    let conn = db.sea_internal();
    let backend = conn.get_database_backend();
    let (sql, values) = build_select_query(backend, filter);

    let rows =
        DeadLetterMessage::find_by_statement(Statement::from_sql_and_values(backend, &sql, values))
            .all(&conn)
            .await?;
    Ok(rows)
}

/// Count dead-lettered messages matching the filter.
pub async fn dead_letter_count(db: &Db, filter: &DeadLetterFilter) -> Result<u64, OutboxError> {
    #[derive(Debug, FromQueryResult)]
    struct Count {
        cnt: i64,
    }

    let conn = db.sea_internal();
    let backend = conn.get_database_backend();
    let (sql, values) = build_count_query(backend, filter);

    let row = Count::find_by_statement(Statement::from_sql_and_values(backend, &sql, values))
        .one(&conn)
        .await?;

    #[allow(clippy::cast_sign_loss)]
    Ok(row.map_or(0, |r| r.cnt as u64))
}

/// Replay dead-lettered messages: re-insert into incoming, set `replayed_at`.
pub async fn dead_letter_replay(db: &Db, filter: &DeadLetterFilter) -> Result<u64, OutboxError> {
    let conn = db.sea_internal();
    let backend = conn.get_database_backend();
    let dialect = Dialect::from(backend);
    let txn = conn.begin().await?;

    let (sql, values) = build_select_query(backend, filter);
    let rows =
        DeadLetterMessage::find_by_statement(Statement::from_sql_and_values(backend, &sql, values))
            .all(&txn)
            .await?;

    let count = rows.len();
    if count == 0 {
        txn.commit().await?;
        return Ok(0);
    }

    // Re-insert body rows and incoming rows via dialect helpers
    let payloads: Vec<(&[u8], &str)> = rows
        .iter()
        .map(|r| (r.payload.as_slice(), r.payload_type.as_str()))
        .collect();
    let body_ids = dialect
        .exec_insert_body_batch(&txn, backend, &payloads)
        .await?;

    let entries: Vec<(i64, i64)> = rows
        .iter()
        .zip(&body_ids)
        .map(|(r, &bid)| (r.partition_id, bid))
        .collect();
    dialect
        .exec_insert_incoming_batch(&txn, backend, &entries)
        .await?;

    // Batch replayed_at update
    let now: sea_orm::Value = chrono::Utc::now().into();
    let dl_ids: Vec<i64> = rows.iter().map(|r| r.id).collect();
    let update_sql = build_batch_replayed_at_update(backend, dl_ids.len());
    let mut update_values: Vec<sea_orm::Value> = Vec::with_capacity(1 + dl_ids.len());
    update_values.push(now);
    for &id in &dl_ids {
        update_values.push(id.into());
    }
    txn.execute(Statement::from_sql_and_values(
        backend,
        &update_sql,
        update_values,
    ))
    .await?;

    txn.commit().await?;

    #[allow(clippy::cast_possible_truncation)]
    Ok(count as u64)
}

/// Permanently delete dead-lettered messages.
/// Only purges messages where `replayed_at IS NOT NULL` (already replayed)
/// unless `force = true`.
pub async fn dead_letter_purge(
    db: &Db,
    filter: &DeadLetterFilter,
    force: bool,
) -> Result<u64, OutboxError> {
    let conn = db.sea_internal();
    let backend = conn.get_database_backend();

    let effective_filter = DeadLetterFilter {
        partition_id: filter.partition_id,
        queue: filter.queue.clone(),
        failed_after: filter.failed_after,
        failed_before: filter.failed_before,
        only_pending: false,
        limit: filter.limit,
    };

    let (sql, values) = if force {
        build_delete_query(backend, &effective_filter, false)
    } else {
        build_delete_query(backend, &effective_filter, true)
    };

    let result = conn
        .execute(Statement::from_sql_and_values(backend, &sql, values))
        .await?;

    Ok(result.rows_affected())
}

struct QueryBuilder {
    sql: String,
    values: Vec<sea_orm::Value>,
    param_idx: usize,
    has_where: bool,
    is_mysql: bool,
}

impl QueryBuilder {
    fn new(base: &str, backend: DbBackend) -> Self {
        Self {
            sql: base.to_owned(),
            values: Vec::new(),
            param_idx: 1,
            has_where: false,
            is_mysql: backend == DbBackend::MySql,
        }
    }

    fn add_condition(&mut self, clause: &str, value: sea_orm::Value) {
        if self.has_where {
            self.sql.push_str(" AND ");
        } else {
            self.sql.push_str(" WHERE ");
            self.has_where = true;
        }
        if self.is_mysql {
            self.sql
                .push_str(&clause.replace(&format!("${}", self.param_idx), "?"));
        } else {
            self.sql.push_str(clause);
        }
        self.values.push(value);
        self.param_idx += 1;
    }

    fn add_raw_condition(&mut self, clause: &str) {
        if self.has_where {
            self.sql.push_str(" AND ");
        } else {
            self.sql.push_str(" WHERE ");
            self.has_where = true;
        }
        self.sql.push_str(clause);
    }

    fn finish(mut self, limit: Option<u32>) -> (String, Vec<sea_orm::Value>) {
        self.sql.push_str(" ORDER BY failed_at DESC");
        if let Some(n) = limit {
            #[allow(clippy::let_underscore_must_use)]
            let _ = write!(self.sql, " LIMIT {n}");
        }
        (self.sql, self.values)
    }
}

fn apply_filters(qb: &mut QueryBuilder, filter: &DeadLetterFilter) {
    if let Some(pid) = filter.partition_id {
        let idx = qb.param_idx;
        qb.add_condition(&format!("d.partition_id = ${idx}"), pid.into());
    }
    if let Some(ref queue) = filter.queue {
        let idx = qb.param_idx;
        qb.add_condition(
            &format!(
                "d.partition_id IN (SELECT id FROM modkit_outbox_partitions WHERE queue = ${idx})"
            ),
            queue.clone().into(),
        );
    }
    if let Some(after) = filter.failed_after {
        let idx = qb.param_idx;
        qb.add_condition(&format!("d.failed_at >= ${idx}"), after.into());
    }
    if let Some(before) = filter.failed_before {
        let idx = qb.param_idx;
        qb.add_condition(&format!("d.failed_at < ${idx}"), before.into());
    }
    if filter.only_pending {
        qb.add_raw_condition("d.replayed_at IS NULL");
    }
}

fn build_select_query(
    backend: DbBackend,
    filter: &DeadLetterFilter,
) -> (String, Vec<sea_orm::Value>) {
    let mut qb = QueryBuilder::new(
        "SELECT d.id, d.partition_id, d.seq, d.payload, d.payload_type, d.created_at, \
         d.failed_at, d.last_error, d.attempts, d.replayed_at \
         FROM modkit_outbox_dead_letters d",
        backend,
    );
    apply_filters(&mut qb, filter);
    qb.finish(filter.limit)
}

fn build_count_query(
    backend: DbBackend,
    filter: &DeadLetterFilter,
) -> (String, Vec<sea_orm::Value>) {
    let mut qb = QueryBuilder::new(
        "SELECT COUNT(*) AS cnt FROM modkit_outbox_dead_letters d",
        backend,
    );
    apply_filters(&mut qb, filter);
    // Count doesn't need ORDER BY or LIMIT but we strip them
    let (mut sql, values) = qb.finish(None);
    // Remove the ORDER BY clause for count queries
    if let Some(pos) = sql.find(" ORDER BY") {
        sql.truncate(pos);
    }
    (sql, values)
}

/// Build a direct DELETE query with the same filter conditions as SELECT.
/// Uses `DELETE FROM ... WHERE id IN (SELECT id FROM ... )` subquery approach
/// for all backends — this avoids alias issues (`SQLite`/`MySQL` don't support
/// aliases on DELETE targets) and handles LIMIT correctly (`SQLite` doesn't
/// support `DELETE ... LIMIT`).
fn build_delete_query(
    backend: DbBackend,
    filter: &DeadLetterFilter,
    only_replayed: bool,
) -> (String, Vec<sea_orm::Value>) {
    let mut inner_qb = QueryBuilder::new("SELECT d.id FROM modkit_outbox_dead_letters d", backend);
    apply_filters(&mut inner_qb, filter);
    if only_replayed {
        inner_qb.add_raw_condition("d.replayed_at IS NOT NULL");
    }
    let (inner_sql, values) = inner_qb.finish(filter.limit);
    let sql = format!("DELETE FROM modkit_outbox_dead_letters WHERE id IN ({inner_sql})");
    (sql, values)
}

/// Build `UPDATE modkit_outbox_dead_letters SET replayed_at = ? WHERE id IN (?, ?, ...)`.
fn build_batch_replayed_at_update(backend: DbBackend, count: usize) -> String {
    let is_mysql = backend == DbBackend::MySql;
    let mut sql = String::from("UPDATE modkit_outbox_dead_letters SET replayed_at = ");
    if is_mysql {
        sql.push('?');
    } else {
        sql.push_str("$1");
    }
    sql.push_str(" WHERE id IN (");
    for i in 0..count {
        if i > 0 {
            sql.push_str(", ");
        }
        if is_mysql {
            sql.push('?');
        } else {
            #[allow(clippy::let_underscore_must_use)]
            let _ = write!(sql, "${}", i + 2); // $2, $3, ...
        }
    }
    sql.push(')');
    sql
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn build_query_empty_filter_pg() {
        let filter = DeadLetterFilter::default();
        let (sql, values) = build_select_query(DbBackend::Postgres, &filter);
        assert!(sql.contains("replayed_at IS NULL"));
        assert!(values.is_empty());
    }

    #[test]
    fn build_query_partition_filter_pg() {
        let filter = DeadLetterFilter {
            partition_id: Some(42),
            ..Default::default()
        };
        let (sql, values) = build_select_query(DbBackend::Postgres, &filter);
        assert!(sql.contains("partition_id = $1"));
        assert_eq!(values.len(), 1);
    }

    #[test]
    fn build_query_all_fields_pg() {
        let filter = DeadLetterFilter {
            partition_id: Some(1),
            queue: Some("orders".into()),
            failed_after: Some(chrono::Utc::now()),
            failed_before: Some(chrono::Utc::now()),
            only_pending: true,
            limit: Some(10),
        };
        let (sql, values) = build_select_query(DbBackend::Postgres, &filter);
        assert!(sql.contains("$1"));
        assert!(sql.contains("$2"));
        assert!(sql.contains("$3"));
        assert!(sql.contains("$4"));
        assert!(sql.contains("LIMIT 10"));
        assert_eq!(values.len(), 4);
    }

    #[test]
    fn build_query_mysql_uses_question_marks() {
        let filter = DeadLetterFilter {
            partition_id: Some(1),
            queue: Some("q".into()),
            ..Default::default()
        };
        let (sql, values) = build_select_query(DbBackend::MySql, &filter);
        assert!(sql.contains('?'));
        assert!(!sql.contains('$'));
        assert_eq!(values.len(), 2);
    }

    #[test]
    fn count_query_has_no_order_by() {
        let filter = DeadLetterFilter::default();
        let (sql, _) = build_count_query(DbBackend::Postgres, &filter);
        assert!(sql.contains("COUNT(*)"));
        assert!(!sql.contains("ORDER BY"));
    }
}
