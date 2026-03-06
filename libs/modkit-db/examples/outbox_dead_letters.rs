#![allow(clippy::unwrap_used, clippy::expect_used, clippy::use_debug)]

//! Dead letter lifecycle: reject -> list -> replay -> process -> purge.
//!
//! Run:
//!   cargo run -p cf-modkit-db --example `outbox_dead_letters` --features sqlite,preview-outbox

use std::time::Duration;

use modkit_db::outbox::{
    DeadLetterFilter, HandlerResult, MessageHandler, Outbox, OutboxMessage, Partitions,
    outbox_migrations,
};
use modkit_db::{ConnectOpts, connect_db, migration_runner::run_migrations_for_testing};

struct RejectAll;

#[async_trait::async_trait]
impl MessageHandler for RejectAll {
    async fn handle(
        &self,
        _msg: &OutboxMessage,
        _cancel: tokio_util::sync::CancellationToken,
    ) -> HandlerResult {
        HandlerResult::Reject {
            reason: "bad format".into(),
        }
    }
}

struct SucceedAll;

#[async_trait::async_trait]
impl MessageHandler for SucceedAll {
    async fn handle(
        &self,
        msg: &OutboxMessage,
        _cancel: tokio_util::sync::CancellationToken,
    ) -> HandlerResult {
        println!("  replayed seq={} processed OK", msg.seq);
        HandlerResult::Success
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db = connect_db(
        "sqlite:file:outbox_dl?mode=memory&cache=shared",
        ConnectOpts {
            max_conns: Some(1),
            ..Default::default()
        },
    )
    .await?;
    run_migrations_for_testing(&db, outbox_migrations()).await?;

    // Reject 3 messages to populate dead letters
    let h1 = Outbox::builder(db.clone())
        .poll_interval(Duration::from_millis(50))
        .queue("events", Partitions::of(1))
        .decoupled(RejectAll)
        .start()
        .await?;
    let conn = db.conn()?;
    for i in 0..3 {
        h1.outbox()
            .enqueue(
                &conn,
                "events",
                0,
                format!("evt-{i}").into_bytes(),
                "text/plain;events.logged.v1",
            )
            .await?;
    }
    h1.outbox().flush();
    tokio::time::sleep(Duration::from_secs(1)).await;
    let outbox = std::sync::Arc::clone(h1.outbox());
    h1.stop().await;

    // List
    let items = outbox
        .dead_letter_list(&db, &DeadLetterFilter::default())
        .await?;
    println!("Dead letters: {}", items.len());
    for dl in &items {
        println!(
            "  seq={} error={}",
            dl.seq,
            dl.last_error.as_deref().unwrap_or("?")
        );
    }

    // Replay 1 entry back into the pipeline
    let replayed = outbox
        .dead_letter_replay(
            &db,
            &DeadLetterFilter {
                limit: Some(1),
                ..Default::default()
            },
        )
        .await?;
    println!("Replayed: {replayed}");

    // Process the replayed message with a success handler
    let h2 = Outbox::builder(db.clone())
        .poll_interval(Duration::from_millis(50))
        .queue("events", Partitions::of(1))
        .decoupled(SucceedAll)
        .start()
        .await?;
    h2.outbox().flush();
    tokio::time::sleep(Duration::from_secs(1)).await;
    h2.stop().await;

    // Purge all (force=true deletes even non-replayed entries)
    let purged = outbox
        .dead_letter_purge(&db, &DeadLetterFilter::default(), true)
        .await?;
    println!("Purged: {purged}");

    let remaining = outbox
        .dead_letter_count(&db, &DeadLetterFilter::default())
        .await?;
    println!("Remaining: {remaining}");
    assert_eq!(remaining, 0);

    println!("Done.");
    Ok(())
}
