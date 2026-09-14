# Timing engine

VxClick's main engineering goal is precise long-running timing without turning the application into a permanent 100% CPU spin loop.

## Clock

The Windows core uses `QueryPerformanceCounter` through `QpcClock`.

The scheduler works in counter ticks and converts to microseconds only when needed for waiting decisions and telemetry.

## Absolute deadlines

For target CPS `r` and QPC frequency `f`:

```text
interval_ticks = f / r
```

The engine keeps a floating-point absolute deadline:

```text
deadline = start
wait(deadline)
click()
deadline += interval_ticks
```

This is intentionally different from:

```text
click()
sleep(interval)
click()
sleep(interval)
```

Chained relative sleeps accumulate oversleep and work time into drift. Absolute deadlines preserve the intended timeline over longer runs.

## Hybrid waiting

The default precision clicker currently uses:

- coarse-wait threshold: **2,000 µs**
- final spin window: **350 µs**

The wait loop behaves approximately as follows:

1. When far from the deadline, sleep for a bounded short period.
2. When closer, yield to the scheduler.
3. Inside the final window, use `spin_loop()` until the QPC deadline.

The coarse sleep is capped at 1 ms per iteration. This lets the worker re-evaluate the deadline and stop state frequently rather than sleeping through a long cancellation request.

## Stop responsiveness

The live scheduler uses `wait_until_controlled`, which checks the atomic stop flag:

- before waiting work
- on each outer wait iteration
- periodically inside the final spin loop

This keeps stop behavior independent from frontend render/update cadence.

## Long scheduling stalls

Trying to replay every missed interval after a long OS stall can produce a destructive click burst.

The live scheduler therefore re-anchors when current time is more than four intervals beyond the next deadline:

```text
if now > deadline + interval * 4:
    deadline = now + interval
```

The goal is stable forward behavior, not artificial catch-up throughput.

## Worker priority

When `PrecisionClicker` is created on Windows, the current implementation requests:

- `HIGH_PRIORITY_CLASS` for the process
- `THREAD_PRIORITY_HIGHEST` for the current worker thread

Changes here require benchmark evidence. Higher priority is not free: it can affect responsiveness of other processes and should not be escalated blindly.

## Safety cap

The current validator rejects values above **20,000 CPS**. This is an explicit safety/validation cap, not a claim that every machine can physically deliver 20,000 distinct input events per second with useful accuracy.

The cap should only be raised after real Windows benchmark data supports the change.

## Benchmark mode

`PrecisionClicker::run` stores click timestamps and deadline error samples and returns `BenchmarkStats`.

The CLI reports:

- target CPS
- actual CPS
- elapsed time / total clicks
- mean, p95 and p99 interval
- mean, p95, p99 and worst jitter
- missed deadlines

A deadline is counted as missed when lateness exceeds one requested click interval.

## Benchmark matrix

Scheduler changes should at minimum be checked at representative rates such as:

```text
1 CPS
10 CPS
20 CPS
50 CPS
100 CPS
500 CPS
1000 CPS
2000 CPS
```

Also test longer runs because a scheduler can look correct over one second while drifting over minutes.

## What to measure before optimizing

Do not optimize only for peak CPS. Record:

- requested vs actual CPS
- mean interval error
- p95/p99 interval
- mean and worst jitter
- missed deadlines
- CPU usage
- stop latency
- behavior under system load

An optimization is a regression if it gains small timing improvements by continuously consuming a full CPU core without a justified use case.

## Future timing work

Useful future improvements include:

- benchmark-driven oversleep compensation
- a high-resolution waitable timer for the coarse phase where it improves CPU/timing tradeoffs
- persisted benchmark runs for regression comparison
- in-app jitter/deviation graphs sourced from native telemetry
- per-machine adaptive tuning only if it remains deterministic and bounded

Any adaptive mechanism should preserve absolute scheduling and must not create uncontrolled catch-up bursts.
