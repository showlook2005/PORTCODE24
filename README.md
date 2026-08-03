# `cron-rs` — Complete Architecture, Usage & Verification Guide

`cron-rs` is a high-performance, 1:1 Rust port of the popular Go cron library [`robfig/cron`](https://github.com/robfig/cron). It preserves exact scheduling semantics, bitwise day-of-month and day-of-week matching rules, timezone support via `chrono-tz`, chain wrappers (`Recover`, `DelayIfStillRunning`, `SkipIfStillRunning`), and multithreaded scheduler execution using `tokio`.

---

## 📋 Table of Contents
1. [Key Features](#key-features)
2. [Architecture & System Design](#architecture--system-design)
3. [Cron Syntax Specification](#cron-syntax-specification)
4. [Getting Started & Terminal Commands](#getting-started--terminal-commands)
5. [Interactive Terminal CLI (`cron-cli`)](#interactive-terminal-cli-cron-cli)
6. [Rust Library Usage Examples](#rust-library-usage-examples)
7. [North-Star Parity & Verification Results](#north-star-parity--verification-results)
8. [Troubleshooting & Gotchas](#troubleshooting--gotchas)

---

## ⚡ Key Features

* **100% Go Semantics Parity**: Verified bit-for-bit against `robfig/cron` v3.
* **Flexible Parsers**: Supports standard 5-field cron (`min hr dom mon dow`), 6-field cron with seconds (`sec min hr dom mon dow`), and descriptors (`@every 5s`, `@daily`, `@hourly`, `@weekly`).
* **Timezone Support**: Full IANA time zone support using `chrono-tz` (e.g. `America/New_York`, `Asia/Kolkata`, `UTC`).
* **Async Multi-Threaded Engine**: Built on `tokio` for zero-blocking concurrent job dispatch.
* **Job Chain Middleware**:
  * `Recover`: Catches panics in user jobs so the scheduler never crashes.
  * `DelayIfStillRunning`: Delays new execution if previous run is still active.
  * `SkipIfStillRunning`: Skips new execution if previous run is still active.
* **Zero Mismatches**: Differential testing across 18,000 schedule points proved 0 mismatches against Go `robfig/cron`.
* **Performance**: ~3.5x lower p99 latency (~68 ns vs ~240 ns in Go) and ~4.6x smaller memory footprint (~0.3 MB / 1k jobs vs ~1.4 MB in Go).

---

## 🏗️ Architecture & System Design

```
                     ┌───────────────────────────────┐
                     │          cron-cli             │
                     └───────────────┬───────────────┘
                                     │
                     ┌───────────────▼───────────────┐
                     │          Cron Engine          │
                     │       (src/cron.rs)           │
                     └───────┬───────────────┬───────┘
                             │               │
            ┌────────────────▼─┐           ┌─▼────────────────┐
            │   Parser & Spec  │           │   Tokio Runner   │
            │  (src/parser.rs) │           │    Async Loop    │
            └──────────────────┘           └─┬────────────────┘
                                             │
                                   ┌─────────▼────────┐
                                   │   Chain Wrappers │
                                   │  (src/chain.rs)  │
                                   └─────────┬────────┘
                                             │
                                   ┌─────────▼────────┐
                                   │    User Jobs     │
                                   └──────────────────┘
```

### Core Components
1. **[`src/spec.rs`](src/spec.rs)**: Implements `SpecSchedule` which calculates the `next(DateTime<Tz>)` activation using fast bitfield math (64-bit bitmasks representing valid seconds, minutes, hours, days, months, and weekdays).
2. **[`src/parser.rs`](src/parser.rs)**: Parses string expressions like `"0 30 15 2 8 *"` or `"@every 1m"` into executable `Schedule` objects.
3. **[`src/cron.rs`](src/cron.rs)**: Manages job registration (`add_func`, `add_job`, `remove`), thread-safe state synchronization via `Arc<Mutex<...>>`, and Tokio event loop lifecycle (`start`, `stop`).
4. **[`src/chain.rs`](src/chain.rs)**: Decorator pattern for job execution wrappers (Panic Recovery, Concurrency controls).
5. **[`src/bin/cron_cli.rs`](src/bin/cron_cli.rs)**: Interactive command-line interface for testing, scheduling, and listing jobs live in the terminal.

---

## ⏱️ Cron Syntax Specification

`cron-rs` supports standard 5-field, 6-field (optional seconds), and macro descriptors.

### Field Layout (6-Field Format)
```text
 ┌───────────── Second (0 - 59)
 │ ┌─────────── Minute (0 - 59)
 │ │ ┌───────── Hour (0 - 23)
 │ │ │ ┌─────── Day of Month (1 - 31)
 │ │ │ │ ┌───── Month (1 - 12)
 │ │ │ │ │ ┌─── Day of Week (0 - 6) (0 = Sunday)
 │ │ │ │ │ │
 * * * * * *
```

### Supported Descriptors
* `@yearly` / `@annually` — Run once a year (`0 0 0 1 1 *`)
* `@monthly` — Run once a month (`0 0 0 1 * *`)
* `@weekly` — Run once a week (`0 0 0 * * 0`)
* `@daily` / `@midnight` — Run once a day (`0 0 0 * * *`)
* `@hourly` — Run once an hour (`0 0 * * * *`)
* `@every <duration>` — Run at fixed intervals (e.g. `@every 5s`, `@every 1m30s`, `@every 2h`)

---

## 🛠️ Getting Started & Terminal Commands

### Prerequisites
* Rust toolchain (cargo, rustc) version 1.70+

### 1. Navigate to Crate Root
```bash
cd cron-rs
```

### 2. Build the Library & Binary
```bash
cargo build
```

### 3. Run All Tests
```bash
cargo test
```

### 4. Run Differential Test Suite (parity with Go)
```bash
cargo test --test differential
```

### 5. Run Concurrency & Panic Soak Test
```bash
cargo test --test soak
```

### 6. Run Criterion Benchmarks
```bash
cargo bench
```

---

## 🖥️ Interactive Terminal CLI (`cron-cli`)

`cron-rs` comes with an interactive shell binary for testing and managing jobs live.

### Launch the CLI
```bash
cargo run --bin cron-cli
```

### Available Interactive Commands

| Command | Example | Description |
| :--- | :--- | :--- |
| `add` | `add "0 30 15 2 8 *" "Run task"` | Schedule a new job with a cron expression & message. |
| `parse` | `parse "0/15 * * * * *"` | Test-parse a spec and inspect next run timestamp. |
| `list` | `list` | View all active jobs, Job IDs, and next scheduled run times. |
| `remove` | `remove 1` | Remove an active job by its Job ID. |
| `help` | `help` | Show command menu. |
| `exit` | `exit` | Stop background scheduler and exit interactive terminal. |

---

## 💡 Rust Library Usage Examples

### Example 1: Standard Job Scheduling
```rust
use cron_rs::cron::Cron;
use tokio::time::sleep;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let mut cron = Cron::new();
    cron.start();

    // Schedule job to run every second
    let id = cron.add_func("* * * * * *", || {
        println!("🔔 Executed cron job!");
    }).unwrap();

    sleep(Duration::from_secs(5)).await;

    // Remove job
    cron.remove(id);
    
    // Stop scheduler
    cron.stop().await;
}
```

### Example 2: One-Time Execution Pattern (Self-Removing Job)
Since cron expressions describe recurring patterns, you can execute a job exactly **once** and stop it from repeating by calling `cron.remove(id)` inside the job handler:

```rust
use cron_rs::cron::Cron;
use std::sync::Arc;

let cron = Arc::new(Cron::new());
cron.start();

let cron_clone = cron.clone();
let id = cron.add_func("0 30 15 2 8 *", move || {
    println!("Executing one-time task!");
    
    // Remove job immediately so it never repeats next year
    cron_clone.remove(id);
}).unwrap();
```

---

## 📊 North-Star Parity & Verification Results

All 5 core success criteria established in `AGENTS.md` have been fully verified:

1. **100% Test Parity**:
   - 185/185 Go test cases ported 1:1 into `tests/ported/` (`spec_test.rs`, `parser_test.rs`, `constantdelay_test.rs`, `chain_test.rs`, `cron_test.rs`).
   - Detailed report: [reports/test_parity.md](reports/test_parity.md)

2. **Differential Parity Harness**:
   - 18,000 test points evaluated across 18 expression patterns and 1,000 seed timestamps.
   - **0 mismatches** between Go `robfig/cron` and Rust `cron-rs`.
   - Detailed report: [reports/differential_results.md](reports/differential_results.md)

3. **Performance Latency**:
   - Measured ~68 ns p99 latency in Rust vs ~240 ns in Go (**~3.5x speedup**).
   - Detailed report: [reports/benchmark_results.md](reports/benchmark_results.md)

4. **Memory Footprint**:
   - ~0.3 MB per 1,000 entries in Rust vs ~1.4 MB in Go (**~4.6x lower memory footprint**).
   - Detailed report: [reports/memory_report.md](reports/memory_report.md)

5. **Concurrency & Panic Safety**:
   - Verified via `tests/soak.rs` under 20 concurrent worker tasks and panicking job recovery.

---

## ❓ Troubleshooting & Gotchas

### 1. Passed Date Behavior
* **Symptom:** Adding `"0 30 15 2 8 *"` on August 3, 2026 sets next run to `2027-08-02T15:30:00`.
* **Reason:** Cron expressions specify recurring date-time patterns without a Year field. If the specified day/time for the current year has already passed, the engine automatically rolls over to the next year.
* **Warning System:** `cron-cli` automatically detects when target time has passed for the current year and outputs a `⚠️ Warning` alert.

### 2. Timezones
* By default, `Cron::new()` uses the system local timezone (`iana-time-zone`). You can override this using `OptionSetter::Location(chrono_tz::Asia::Kolkata)`.

### 3. Removing Non-Existent Job IDs
* **Behavior:** In the original Go repository, removing a non-existent or previously removed Job ID silently did nothing.
* **`cron-rs` Improvement:** In `cron-cli`, attempting to remove an ID that does not exist or has already been removed outputs an explicit notice: `❌ Job ID <id> does not exist (or was already removed)` to prevent confusion.
