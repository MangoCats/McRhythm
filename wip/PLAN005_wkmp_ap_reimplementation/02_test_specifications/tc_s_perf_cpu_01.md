# TC-S-PERF-CPU-01: CPU Usage on Pi Zero 2W

**Test ID:** TC-S-PERF-CPU-01
**Requirement:** PERF-CPU-010 - CPU usage <40% on Pi Zero 2W during playback
**Test Type:** System/Performance Test
**Priority:** High
**Est. Effort:** 6 hours (4h test implementation + 2h hardware validation)

---

## Objective

Verify that wkmp-ap CPU usage remains below 40% (aggregate across 4 cores) on Raspberry Pi Zero 2W hardware during normal playback with 2 active decoder chains and active crossfading.

---

## Scope

- **Components Under Test:** Entire wkmp-ap service
- **Hardware:** Raspberry Pi Zero 2W (ARM Cortex-A53, 4 cores @ 1GHz, 512MB RAM)
- **Environment:** Production-like deployment on actual Pi Zero 2W hardware
- **Load Profile:** 2 active passages, crossfading, realistic audio files

---

## Test Specification

### Given (Initial Conditions)

1. **Hardware Setup:**
   - Raspberry Pi Zero 2W with clean Raspberry Pi OS install
   - Audio output: USB audio adapter (Pi Zero audio jack is PWM, not ideal for testing)
   - Network: Connected for monitoring (SSH access)

2. **Baseline Measurement:**
   - Boot system, wait 60 seconds for stabilization
   - Measure idle CPU for 60 seconds: `top -b -d 1 -n 60`
   - Record idle baseline (expected: 2-5% aggregate across 4 cores)

3. **Database Setup:**
   - Queue with 100 passages
   - Mix of formats: 50% FLAC 320kbps, 30% MP3 320kbps, 20% Opus 128kbps
   - All files at 44.1kHz sample rate
   - Crossfade settings: 5-second crossfades, Equal-Power curves

4. **System State:**
   - wkmp-ap service running
   - No other applications running (minimal OS services only)
   - Audio device open and playing

---

### When (Action/Input)

1. **Start Playback:**
   - POST /playback/play via HTTP API
   - Playback continues for 10 minutes (continuous operation)

2. **Monitoring:**
   - CPU usage sampled every 1 second for 600 seconds
   - Capture per-process CPU via: `/proc/[pid]/stat`
   - Aggregate CPU across all wkmp-ap threads

3. **Load Conditions:**
   - 2 decoder chains active (normal operation)
   - Crossfades occurring every ~3 minutes (passage transitions)
   - SSE connection active (1 client connected monitoring events)

---

### Then (Expected Result)

1. **CPU Usage Target:**
   - **Average Aggregate CPU: ≤30%** (below 40% target with margin)
   - **Peak Aggregate CPU: ≤40%** (brief spikes allowed during crossfade start)
   - **Sustained Aggregate CPU: ≤35%** (95th percentile)

2. **Measurement Methodology (Per SPEC022 Resolution):**
   - Aggregate CPU = sum of CPU time across all 4 cores
   - Calculation: `(process_user_time + process_sys_time) / elapsed_wall_time × 100`
   - Baseline subtracted: CPU above idle baseline
   - Example: If idle = 3%, playback = 33%, reported = 30% (33% - 3%)

3. **Pass Criteria Breakdown:**
   - 90% of 1-second samples ≤30% aggregate CPU
   - No single sample exceeds 50% (hard limit - indicates performance problem)
   - Average over 10-minute period ≤30%

---

## Verification Steps

### Automated CPU Monitoring Script

```rust
// test_cpu_usage.rs
use std::fs;
use std::thread;
use std::time::{Duration, Instant};

struct CpuSample {
    timestamp: Instant,
    user_time: u64,    // ticks
    sys_time: u64,     // ticks
    elapsed_ticks: u64 // wall clock ticks
}

fn read_proc_stat(pid: u32) -> Option<CpuSample> {
    let stat_path = format!("/proc/{}/stat", pid);
    let contents = fs::read_to_string(stat_path).ok()?;
    let fields: Vec<&str> = contents.split_whitespace().collect();

    // Field 13: utime (user mode), Field 14: stime (kernel mode)
    let user_time = fields[13].parse::<u64>().ok()?;
    let sys_time = fields[14].parse::<u64>().ok()?;

    Some(CpuSample {
        timestamp: Instant::now(),
        user_time,
        sys_time,
        elapsed_ticks: 0 // computed from wall clock
    })
}

fn calculate_cpu_percent(prev: &CpuSample, curr: &CpuSample, num_cores: u32) -> f64 {
    let user_delta = curr.user_time - prev.user_time;
    let sys_delta = curr.sys_time - prev.sys_time;
    let total_delta = user_delta + sys_delta;

    let elapsed_secs = (curr.timestamp - prev.timestamp).as_secs_f64();
    let ticks_per_sec = 100.0; // sysconf(_SC_CLK_TCK), typically 100 on Linux
    let elapsed_ticks = elapsed_secs * ticks_per_sec;

    // CPU percentage = (process_ticks / (elapsed_ticks * num_cores)) × 100
    (total_delta as f64 / (elapsed_ticks * num_cores as f64)) * 100.0
}

#[test]
fn test_cpu_usage_pi_zero_2w() {
    // Start wkmp-ap service
    let wkmp_ap_pid = start_wkmp_ap_service();

    // Measure idle baseline
    thread::sleep(Duration::from_secs(60));
    let idle_cpu = measure_cpu_for_duration(wkmp_ap_pid, 60);
    println!("Idle baseline: {:.2}%", idle_cpu);

    // Start playback
    http_post("http://localhost:5721/playback/play");

    // Monitor CPU for 10 minutes
    let mut samples = Vec::new();
    let start = Instant::now();
    let duration = Duration::from_secs(600); // 10 minutes

    let mut prev_sample = read_proc_stat(wkmp_ap_pid).unwrap();

    while start.elapsed() < duration {
        thread::sleep(Duration::from_secs(1));

        let curr_sample = read_proc_stat(wkmp_ap_pid).unwrap();
        let cpu_pct = calculate_cpu_percent(&prev_sample, &curr_sample, 4);
        samples.push(cpu_pct);

        prev_sample = curr_sample;
    }

    // Stop playback
    http_post("http://localhost:5721/playback/stop");

    // Analyze results
    let avg_cpu = samples.iter().sum::<f64>() / samples.len() as f64;
    let max_cpu = samples.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let mut sorted = samples.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p95_cpu = sorted[(sorted.len() * 95 / 100)];
    let samples_below_30 = samples.iter().filter(|&&x| x <= 30.0).count();
    let pct_below_30 = (samples_below_30 as f64 / samples.len() as f64) * 100.0;

    println!("CPU Usage Analysis:");
    println!("  Average: {:.2}%", avg_cpu);
    println!("  Peak: {:.2}%", max_cpu);
    println!("  p95: {:.2}%", p95_cpu);
    println!("  % samples ≤30%: {:.1}%", pct_below_30);

    // Assertions
    assert!(avg_cpu <= 30.0, "Average CPU {:.2}% exceeds 30% target", avg_cpu);
    assert!(max_cpu <= 50.0, "Peak CPU {:.2}% exceeds 50% hard limit", max_cpu);
    assert!(p95_cpu <= 35.0, "p95 CPU {:.2}% exceeds 35% sustained target", p95_cpu);
    assert!(pct_below_30 >= 90.0, "Only {:.1}% of samples ≤30% (target: ≥90%)", pct_below_30);
}
```

### Manual Verification with `top`

```bash
# SSH to Pi Zero 2W
ssh pi@wkmp-pi.local

# Find wkmp-ap process
ps aux | grep wkmp-ap

# Monitor CPU with top (batch mode, 1-second intervals, 600 samples)
top -b -d 1 -n 600 -p [PID] > cpu_log.txt

# Analyze log
grep wkmp-ap cpu_log.txt | awk '{sum+=$9; if($9>max) max=$9; count++} END {print "Avg:", sum/count "% | Max:", max "%"}'
```

---

## Pass Criteria

**Test PASSES if ALL of the following are true:**

1. ✅ Average CPU ≤30% over 10-minute test
2. ✅ Peak CPU ≤50% (hard limit, no sample exceeds)
3. ✅ p95 CPU ≤35% (sustained performance acceptable)
4. ✅ ≥90% of 1-second samples ≤30%
5. ✅ No audio dropouts during test (verified by event log)

---

## Fail Criteria

**Test FAILS if ANY of the following are true:**

1. ❌ Average CPU >30%
2. ❌ Any single sample >50%
3. ❌ p95 CPU >35%
4. ❌ <90% of samples ≤30%
5. ❌ Audio dropouts or glitches occur
6. ❌ Service crashes or restarts during test

---

## Test Data

### Test Audio Files

Location: `/home/pi/wkmp_test_data/`

**File Distribution:**
- 50 files: FLAC 16-bit 44.1kHz, 320kbps equivalent, ~3-5 minutes each
- 30 files: MP3 320kbps 44.1kHz, ~3-5 minutes each
- 20 files: Opus 128kbps 44.1kHz, ~3-5 minutes each

**Total Duration:** ~6 hours of audio (ensures variety, no repetition during 10-minute test)

---

## Dependencies

- **Hardware:** Raspberry Pi Zero 2W (ARM Cortex-A53, 4 cores, 512MB RAM)
- **OS:** Raspberry Pi OS Lite (minimal install, no desktop)
- **USB Audio Adapter:** Any USB audio adapter with Linux support
- **Monitoring Tools:** `top`, `/proc/[pid]/stat`, Rust test harness
- **Network:** SSH access for monitoring and control

---

## Notes

- **Baseline Measurement Critical:** Must subtract idle CPU to isolate wkmp-ap contribution
- **Pi Zero 2W Thermal Throttling:** Monitor CPU temperature during test
  - If temp >70°C, throttling may occur → invalidates test
  - Use: `vcgencmd measure_temp` to monitor
- **Crossfade Impact:** Expect CPU spikes (5-10% increase) during crossfade start
  - This is acceptable if brief (<2 seconds)
- **Decode Formats:** FLAC is most CPU-intensive, so 50% FLAC provides realistic worst-case
- **Regression Testing:** Run this test before each release to catch performance regressions

---

**Status:** Ready for implementation
**Last Updated:** 2025-10-26
