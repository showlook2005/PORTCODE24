# Stage 0 — Inventory Report

## Go Source Inventory (`robfig/cron`)

| Source File | LOC | Structs / Interfaces / Types | Key Functions & Methods |
|---|---|---|---|
| `spec.go` | 189 | `SpecSchedule`, `bounds` | `SpecSchedule.Next()`, `dayMatches()` |
| `parser.go` | 435 | `ParseOption`, `Parser` | `NewParser()`, `Parser.Parse()`, `ParseStandard()`, `normalizeFields()`, `getField()`, `getRange()`, `parseDescriptor()` |
| `constantdelay.go` | 28 | `ConstantDelaySchedule` | `Every()`, `ConstantDelaySchedule.Next()` |
| `option.go` | 46 | `Option` | `WithLocation()`, `WithSeconds()`, `WithParser()`, `WithChain()`, `WithLogger()` |
| `chain.go` | 93 | `JobWrapper`, `Chain` | `NewChain()`, `Chain.Then()`, `Recover()`, `DelayIfStillRunning()`, `SkipIfStillRunning()` |
| `cron.go` | 356 | `Cron`, `ScheduleParser`, `Job`, `Schedule`, `EntryID`, `Entry`, `byTime`, `FuncJob` | `New()`, `Cron.AddFunc()`, `Cron.AddJob()`, `Cron.Schedule()`, `Cron.Entries()`, `Cron.Remove()`, `Cron.Start()`, `Cron.Run()`, `Cron.Stop()` |
| `logger.go` | 87 | `Logger`, `printfLogger` | `PrintfLogger()`, `VerbosePrintfLogger()`, `printfLogger.Info()`, `printfLogger.Error()` |
| **Total Source** | **1,234** | | |

---

## Go Test Inventory (`robfig/cron`)

| Test File | Test Functions | Subtests / Table Cases | Total Test Cases Target |
|---|---|---|---|
| `spec_test.go` | 5 (`TestActivation`, `TestNext`, `TestErrors`, `TestNextWithTz`, `TestSlash0NoHang`) | Table-driven tests | 62 |
| `parser_test.go` | 11 (`TestRange`, `TestField`, `TestAll`, `TestBits`, `TestParseScheduleErrors`, `TestParseSchedule`, `TestOptionalSecondSchedule`, `TestNormalizeFields`, `TestNormalizeFields_Errors`, `TestStandardSpecSchedule`, `TestNoDescriptorParser`) | Table-driven tests & subtests | 67 |
| `constantdelay_test.go` | 1 (`TestConstantDelayNext`) | Table-driven cases | 13 |
| `option_test.go` | 3 (`TestWithLocation`, `TestWithParser`, `TestWithVerboseLogger`) | Individual tests | 3 |
| `chain_test.go` | 4 (`TestChain`, `TestChainRecover`, `TestChainDelayIfStillRunning`, `TestChainSkipIfStillRunning`) | Subtests | 12 |
| `cron_test.go` | 24 (`TestFuncPanicRecovery`, `TestJobPanicRecovery`, `TestNoEntries`, `TestStopCausesJobsToNotRun`, `TestAddBeforeRunning`, `TestAddWhileRunning`, `TestAddWhileRunningWithDelay`, `TestRemoveBeforeRunning`, `TestRemoveWhileRunning`, `TestSnapshotEntries`, `TestMultipleEntries`, `TestRunningJobTwice`, `TestRunningMultipleSchedules`, `TestLocalTimezone`, `TestNonLocalTimezone`, `TestStopWithoutStart`, `TestInvalidJobSpec`, `TestBlockingRun`, `TestStartNoop`, `TestJob`, `TestScheduleAfterRemoval`, `TestJobWithZeroTimeDoesNotRun`, `TestStopAndWait`, `TestMultiThreadedStartAndStop`) | Individual & subtests | 28 |
| **Total Test Suite** | **48 test functions** | | **185 test cases** |

---

## Target Gate Status
- **Stage 0 Target**: `reports/inventory.md` created with total test count > 0.
- **Result**: PASSED (185 test cases target established).
