//! Transactional outbox for reliable asynchronous message production.
//!
//! # Architecture
//!
//! Four-stage pipeline: **incoming → sequencer → outgoing → processor**.
//!
//! 1. **Enqueue** — messages are written atomically within business transactions
//!    to the `incoming` table via [`Outbox::enqueue()`].
//! 2. **Sequencer** — a background task claims incoming rows, assigns
//!    per-partition sequence numbers, and writes to the `outgoing` table.
//! 3. **Processor** — one long-lived task per partition reads from `outgoing`,
//!    dispatches to the registered handler, and acks via cursor advance
//!    (append-only — no deletes on the hot path).
//! 4. **Reaper** — when a partition is idle, the processor bulk-deletes
//!    processed outgoing and body rows.
//!
//! # Processing modes
//!
//! - **Transactional** — handler runs inside the DB transaction holding the
//!   partition lock. Provides exactly-once semantics within the database.
//! - **Decoupled** — handler runs outside any transaction, with lease-based
//!   locking. Provides at-least-once delivery; handlers must be idempotent.
//!
//! # Usage
//!
//! ```ignore
//! let handle = Outbox::builder(db)
//!     .poll_interval(Duration::from_millis(100))
//!     .queue("orders", Partitions::of(4))
//!         .decoupled(my_handler)
//!     .start().await?;
//! // ... enqueue via handle.outbox() ...
//! handle.stop().await;
//! ```
//!
//! # Backend notes
//!
//! - **`PostgreSQL`** — Full support. Uses `FOR UPDATE SKIP LOCKED` for partition
//!   locking and `INSERT ... RETURNING` for body ID retrieval.
//! - **`MySQL` 8.0+** — Requires `MySQL` 8.0 or later for `FOR UPDATE SKIP LOCKED`
//!   support (added in 8.0.1). Earlier versions will fail at runtime when
//!   attempting to acquire partition locks. Uses `LAST_INSERT_ID()` for body IDs.
//! - **`SQLite`** — Single-process only. `SQLite` has no row-level locking; the
//!   outbox relies on `SQLite`'s single-writer model. Suitable for development,
//!   testing, and single-instance deployments. Not recommended for production
//!   multi-process scenarios.
//!
//! # Dead letters
//!
//! Messages that a handler permanently rejects ([`HandlerResult::Reject`]) are
//! moved to a dead-letter table with the original payload, partition, sequence,
//! and error reason preserved. The outbox does **not** auto-replay dead letters;
//! that policy is owned by the application.
//!
//! Dead letter operations are available as methods on [`Outbox`]:
//! [`dead_letter_list`](Outbox::dead_letter_list),
//! [`dead_letter_count`](Outbox::dead_letter_count),
//! [`dead_letter_replay`](Outbox::dead_letter_replay), and
//! [`dead_letter_purge`](Outbox::dead_letter_purge).
//!
//! ## Consumption patterns
//!
//! 1. **Scheduled background worker** — poll on a timer, replay if count > 0:
//!
//!    ```ignore
//!    loop {
//!        let count = outbox.dead_letter_count(&db, &DeadLetterFilter::default()).await?;
//!        if count > 0 {
//!            tracing::warn!(count, "dead letters detected — replaying");
//!            outbox.dead_letter_replay(&db, &DeadLetterFilter::default()).await?;
//!        }
//!        tokio::time::sleep(Duration::from_secs(300)).await;
//!    }
//!    ```
//!
//! 2. **Admin REST endpoint** — expose list/replay/purge via HTTP for manual
//!    investigation and recovery.
//!
//! 3. **On-demand CLI** — a maintenance command that replays or purges with
//!    filters (by queue, partition, time range).
//!
//! Replayed messages go through the full pipeline (incoming → sequencer →
//! outgoing → processor). Handlers must be idempotent — a replayed message
//! may be delivered more than once if the handler previously produced
//! side-effects before rejecting.

mod builder;
mod core;
mod dead_letter;
mod dialect;
mod handler;
mod manager;
mod migrations;
mod processor;
mod sequencer;
mod strategy;
mod types;

#[cfg(test)]
#[cfg(feature = "sqlite")]
#[cfg_attr(coverage_nightly, coverage(off))]
mod integration_tests;

pub use builder::QueueBuilder;
pub use core::Outbox;
pub use dead_letter::{DeadLetterFilter, DeadLetterMessage};
pub use handler::{
    Handler, HandlerResult, MessageHandler, OutboxMessage, PerMessageAdapter, TransactionalHandler,
    TransactionalMessageHandler,
};
pub use manager::{OutboxBuilder, OutboxHandle};
pub use migrations::outbox_migrations;
pub use types::{EnqueueMessage, OutboxError, OutboxMessageId, Partitions};

// Internal re-exports for tests and internal modules
