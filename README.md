# `cron-rs` — Complete Architecture, Usage & Verification Guide

`cron-rs` is a high-performance, 1:1 Rust port of the popular Go cron library [`robfig/cron`](https://github.com/robfig/cron). It preserves exact scheduling semantics, bitwise day-of-month and day-of-week matching rules, timezone support via `chrono-tz`, chain wrappers (`Recover`, `DelayIfStillRunning`, `SkipIfStillRunning`), and multithreaded scheduler execution using `tokio`.

---

## 🚀 Quick Start Guide for New Users / Judges

If you have just cloned this repository on your computer, follow these step-by-step instructions to build, test, and run `cron-rs` in your terminal.

### Prerequisites
Make sure you have Rust and Cargo installed on your system.
* Check installation: `cargo --version`
* If not installed, install from: [https://rustup.rs](https://rustup.rs)

---

### Step 1: Clone the Repository
Open your terminal (Terminal / Command Prompt / PowerShell) and run:

```bash
git clone https://github.com/showlook2005/PORTCODE24.git
cd PORTCODE24
```

---

### Step 2: Build the Crate & Binaries
Compile the library and the interactive CLI executable:

```bash
cargo build
```

---

### Step 3: Run the 1:1 Ported Test Suite
Run all 185 ported Go test cases to verify 100% test parity:

```bash
cargo test
```

#### Run Differential Test Suite (18,000 schedule points vs Go):
```bash
cargo test --test differential
```

#### Run Concurrency & Panic Recovery Soak Test:
```bash
cargo test --test soak
```

---

### Step 4: Launch the Interactive Terminal CLI (`cron-cli`)
Launch the interactive command-line application to test cron scheduling live:

```bash
cargo run --bin cron-cli
```

#### Interactive Commands inside `cron-cli`:
* **Parse a cron expression:**
  ```text
  parse "0/15 * * * * *"
  ```
* **Add a job to run every 5 seconds:**
  ```text
  add "0/5 * * * * *" "Execute database backup"
  ```
* **List all active jobs & next run timestamps:**
  ```text
  list
  ```
* **Remove a job by ID:**
  ```text
  remove 1
  ```
* **Exit the CLI:**
  ```text
  exit
  ```

---

### Step 5: Run Criterion Performance Benchmarks
To measure p99 execution latency on your system:

```bash
cargo bench
```

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
