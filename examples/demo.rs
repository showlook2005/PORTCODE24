use cron_rs::cron::Cron;
use cron_rs::parser::{parse_option, Parser};
use cron_rs::cron::OptionSetter;
use std::time::Duration;
use tokio::time::sleep;

fn second_parser() -> Parser {
    Parser::new(
        parse_option::SECOND
            | parse_option::MINUTE
            | parse_option::HOUR
            | parse_option::DOM
            | parse_option::MONTH
            | parse_option::DOW_OPTIONAL
            | parse_option::DESCRIPTOR,
    )
}

#[tokio::main]
async fn main() {
    println!("===========================================");
    println!("      cron-rs Live Terminal Demo           ");
    println!("===========================================");

    let cron = Cron::with_options(vec![OptionSetter::Parser(second_parser())]);

    // 1. Schedule a job every second
    let id1 = cron
        .add_func("* * * * * *", || {
            println!("  ⏱️  [Job 1] Ticked at second boundary!");
        })
        .unwrap();
    println!("✓ Added Job 1 (Every 1s) with ID: {}", id1);

    // 2. Schedule a job every 2 seconds
    let id2 = cron
        .add_func("@every 2s", || {
            println!("  🔄 [Job 2] Triggered every 2 seconds!");
        })
        .unwrap();
    println!("✓ Added Job 2 (Every 2s) with ID: {}", id2);

    // 3. Start the scheduler
    println!("\n▶️  Starting cron scheduler...");
    cron.start();

    // Let it run for 4 seconds
    sleep(Duration::from_secs(4)).await;

    // 4. Inspect active entries
    println!("\n📋 Active Entries Snapshot:");
    for entry in cron.entries() {
        println!(" - Entry ID: {}, Next Run: {:?}", entry.id, entry.next);
    }

    // 5. Remove Job 1 while running
    println!("\n🗑️  Removing Job 1 (ID: {})...", id1);
    cron.remove(id1);

    // Run remaining job for 3 seconds
    sleep(Duration::from_secs(3)).await;

    // 6. Stop the scheduler cleanly
    println!("\n⏹️  Stopping cron scheduler...");
    let rx = cron.stop();
    let _ = rx.await;

    println!("\n===========================================");
    println!("    cron-rs Demonstration Complete!        ");
    println!("===========================================");
}
