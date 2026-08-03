### 1. Passed Date Behavior
* **Symptom:** Adding `"0 30 15 2 8 *"` on August 3, 2026 sets next run to `2027-08-02T15:30:00`.
* **Reason:** Cron expressions specify recurring date-time patterns without a Year field. If the specified day/time for the current year has already passed, the engine automatically rolls over to the next year.
* **Warning System:** `cron-cli` automatically detects when target time has passed for the current year and outputs a `⚠️ Warning` alert.

### 2. Timezones
* By default, `Cron::new()` uses the system local timezone (`iana-time-zone`). You can override this using `OptionSetter::Location(chrono_tz::Asia::Kolkata)`.

### 3. Removing Non-Existent Job IDs
* **Behavior:** In the original Go repository, removing a non-existent or previously removed Job ID silently did nothing.
* **`cron-rs` Improvement:** In `cron-cli`, attempting to remove an ID that does not exist or has already been removed outputs an explicit notice: `❌ Job ID <id> does not exist (or was already removed)` to prevent confusion.
