use chrono::{Datelike, Utc};
use cron_rs::cron::{Cron, OptionSetter};
use cron_rs::parser::{parse_option, Parser};
use std::io::{self, Write};
use std::sync::Arc;

fn second_parser() -> Parser {
    Parser::new(
        parse_option::SECOND_OPTIONAL
            | parse_option::MINUTE
            | parse_option::HOUR
            | parse_option::DOM
            | parse_option::MONTH
            | parse_option::DOW
            | parse_option::DESCRIPTOR,
    )
}

#[tokio::main]
async fn main() {
    println!("\n=======================================================");
    println!("        Welcome to cron-rs Interactive Terminal        ");
    println!("=======================================================");
    println!("Commands:");
    println!("  add <cron_expr> <message>   - Schedule a job with a cron expression");
    println!("  parse <cron_expr>           - Test parse a spec & calculate next time");
    println!("  list                        - List all active scheduled entries");
    println!("  remove <id>                 - Remove a job by Entry ID");
    println!("  help                        - Show this command menu");
    println!("  exit                        - Stop scheduler and exit");
    println!("-------------------------------------------------------\n");

    let cron = Arc::new(Cron::with_options(vec![OptionSetter::Parser(second_parser())]));
    cron.start();

    println!("▶️  Cron scheduler started in background.\n");

    let stdin = io::stdin();
    let mut input = String::new();

    loop {
        print!("cron-rs> ");
        let _ = io::stdout().flush();
        input.clear();

        if stdin.read_line(&mut input).is_err() || input.trim().is_empty() {
            continue;
        }

        let line = input.trim();
        if line == "exit" || line == "quit" {
            println!("\n⏹️  Stopping cron-rs scheduler...");
            let rx = cron.stop();
            let _ = rx.await;
            println!("Goodbye!");
            break;
        }

        if line == "help" {
            println!("\nCommands:");
            println!("  add <cron_expr> <message>   - e.g. add \"* * * * * *\" \"Hello every second\"");
            println!("                                e.g. add \"@every 3s\" \"Runs every 3 seconds\"");
            println!("  parse <cron_expr>           - e.g. parse \"0/15 8-18 * * Mon-Fri\"");
            println!("  list                        - Show active scheduled jobs");
            println!("  remove <id>                 - Remove job by ID");
            println!("  exit                        - Stop and exit\n");
            continue;
        }

        if line == "list" || line == "entries" {
            let entries = cron.entries();
            if entries.is_empty() {
                println!("No active entries found.");
            } else {
                println!("\n📋 Active Jobs ({} total):", entries.len());
                for e in entries {
                    let next_str = e.next.map_or("None".to_string(), |t| t.to_rfc3339());
                    println!("  [ID: {}] Next Run: {}", e.id, next_str);
                }
                println!();
            }
            continue;
        }

        if line.starts_with("remove ") {
            let id_str = line["remove ".len()..].trim();
            match id_str.parse::<usize>() {
                Ok(id) => {
                    if cron.entry(id).is_some() {
                        cron.remove(id);
                        println!("🗑️  Removed job ID: {}", id);
                    } else {
                        println!("❌ Job ID {} does not exist (or was already removed).", id);
                    }
                }
                Err(_) => println!("❌ Invalid ID: {}", id_str),
            }
            continue;
        }

        if line.starts_with("parse ") {
            let spec = line["parse ".len()..].trim().trim_matches('"');
            let parser = second_parser();
            match parser.parse(spec) {
                Ok(sched) => {
                    let now = Utc::now();
                    let next = sched.next(now.with_timezone(&chrono_tz::Tz::UTC));
                    println!("✓ Expression '{}' is VALID!", spec);
                    println!("  Current Time: {}", now.to_rfc3339());
                    if let Some(next_dt) = next {
                        println!("  Next Activation: {}", next_dt.to_rfc3339());
                        if next_dt.year() > now.year() {
                            println!("  ⚠️  Warning: Target time for {} has already passed! Next run scheduled for next year ({})", now.year(), next_dt.year());
                        }
                    } else {
                        println!("  Next Activation: None");
                    }
                }
                Err(err) => println!("❌ Parse Error for '{}': {}", spec, err),
            }
            continue;
        }

        if line.starts_with("add ") {
            let rest = line["add ".len()..].trim();
            let (spec, msg) = if rest.starts_with('"') {
                if let Some(end_q) = rest[1..].find('"') {
                    let spec = &rest[1..end_q + 1];
                    let msg = rest[end_q + 2..].trim().trim_matches('"');
                    (spec, if msg.is_empty() { "Job Executed!" } else { msg })
                } else {
                    (rest, "Job Executed!")
                }
            } else {
                let parts: Vec<&str> = rest.splitn(2, ' ').collect();
                if parts.len() == 2 {
                    (parts[0], parts[1].trim_matches('"'))
                } else {
                    (parts[0], "Job Executed!")
                }
            };

            let msg_owned = msg.to_string();
            match cron.add_func(spec, move || {
                println!("\n  🔔 [cron-rs alert] {}", msg_owned);
                print!("cron-rs> ");
                let _ = io::stdout().flush();
            }) {
                Ok(id) => {
                    println!("✓ Added Job ID {} for spec '{}'", id, spec);
                    let parser = second_parser();
                    if let Ok(sched) = parser.parse(spec) {
                        let now = Utc::now();
                        let local_now = chrono::Local::now();
                        if let Some(next_dt) = sched.next(now.with_timezone(&chrono_tz::Tz::UTC)) {
                            if next_dt.year() > local_now.year() {
                                println!("  ⚠️  Warning: Target time for {} has already passed! Next run scheduled for: {}", local_now.year(), next_dt.to_rfc3339());
                            }
                        }
                    }
                }
                Err(err) => println!("❌ Failed to parse spec '{}': {}", spec, err),
            }
            continue;
        }

        println!("Unknown command: '{}'. Type 'help' for command list.", line);
    }
}
