// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Reading what the kernel says, as text, without being welded to a kernel.
//!
//! Every function here parses a string and returns a number or nothing. Nothing opens a file, so all
//! of it runs in an ordinary unit test on any machine — the same discipline that pulled arithmetic
//! out of components, CSS out of the compiler's blind spot, and the disclosure rule out of the D-Bus
//! adapter. The one thing that touches `/proc` is `read_to_string`, and it lives in the caller.
//!
//! Every parser returns `Option` and none of them guesses. A `/proc` file that is missing, empty, or
//! shaped differently on some kernel produces *no reading*, which the window handles by having one
//! fewer sample. A parser that returned zero for an unreadable file would put a fabricated number
//! into a baseline, and a baseline is exactly where a fabricated number does the most damage: it
//! moves what the host believes is ordinary about itself.

/// The one-minute load average from `/proc/loadavg`.
#[must_use]
pub fn load_average(contents: &str) -> Option<f64> {
    contents.split_whitespace().next()?.parse().ok()
}

/// The share of memory in use, from `/proc/meminfo`.
///
/// Uses `MemAvailable` rather than `MemFree`. Free memory on Linux measures almost nothing a person
/// cares about — a healthy host keeps very little free because the rest is cache it can drop — and a
/// detector watching it would report every warm cache as an emergency.
#[must_use]
pub fn memory_used(contents: &str) -> Option<f64> {
    let total = meminfo_field(contents, "MemTotal:")?;
    let available = meminfo_field(contents, "MemAvailable:")?;
    if total <= 0.0 {
        return None;
    }
    Some(((total - available) / total).clamp(0.0, 1.0))
}

/// The share of swap in use, from `/proc/meminfo`.
///
/// A host with no swap configured has no reading rather than a reading of zero: *there is no swap*
/// and *swap is empty* are different facts, and only one of them can later become alarming.
#[must_use]
pub fn swap_used(contents: &str) -> Option<f64> {
    let total = meminfo_field(contents, "SwapTotal:")?;
    let free = meminfo_field(contents, "SwapFree:")?;
    if total <= 0.0 {
        return None;
    }
    Some(((total - free) / total).clamp(0.0, 1.0))
}

/// One `/proc/meminfo` field, in kibibytes.
fn meminfo_field(contents: &str, field: &str) -> Option<f64> {
    contents
        .lines()
        .find(|line| line.starts_with(field))?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}

/// The `some avg10` figure from a `/proc/pressure/*` file.
///
/// `some` rather than `full`: `some` is the share of time at least one task was stalled, which is
/// what a person experiences as the machine being slow. `full` is the share where *everything* was
/// stalled, which on a working host is usually zero right up until it is a catastrophe.
#[must_use]
pub fn pressure_some_avg10(contents: &str) -> Option<f64> {
    let line = contents.lines().find(|line| line.starts_with("some "))?;
    for field in line.split_whitespace() {
        if let Some(value) = field.strip_prefix("avg10=") {
            return value.parse().ok();
        }
    }
    None
}

/// The share of a filesystem in use, from block counts.
///
/// Uses the blocks available to an unprivileged process rather than the free blocks, because the
/// reserved portion is not usable by the services that will fail when it runs out. A host reported
/// at 95% by this measure and 90% by `df` is not being alarmist; it is describing the number that
/// will actually stop something.
#[must_use]
pub fn filesystem_used(total_blocks: u64, available_blocks: u64) -> Option<f64> {
    if total_blocks == 0 {
        return None;
    }
    #[allow(
        clippy::cast_precision_loss,
        reason = "a share of a filesystem is compared against a threshold, not counted with"
    )]
    let share = 1.0 - (available_blocks as f64 / total_blocks as f64);
    Some(share.clamp(0.0, 1.0))
}

/// How many units `systemctl list-units --failed` reported.
///
/// Counts the lines that look like a unit rather than trusting the summary line, which is absent
/// under `--no-legend` and worded differently across versions.
#[must_use]
pub fn failed_units(listing: &str) -> f64 {
    let count = listing
        .lines()
        .map(str::trim_start)
        .filter(|line| {
            // The bullet systemd prefixes failed units with, and its absence under --no-legend.
            let line = line.trim_start_matches(['●', '*', ' ']);
            line.split_whitespace()
                .next()
                .is_some_and(|first| first.contains('.') && !first.starts_with('-'))
        })
        .count();
    #[allow(
        clippy::cast_precision_loss,
        reason = "a host with more failed units than f64 can count exactly has larger problems"
    )]
    let count = count as f64;
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    const MEMINFO: &str = "MemTotal:        4025536 kB\nMemFree:          198372 kB\nMemAvailable:    1610214 kB\nBuffers:           51236 kB\nCached:          1502100 kB\nSwapTotal:       2097148 kB\nSwapFree:        1887436 kB\n";

    const PRESSURE: &str = "some avg10=12.43 avg60=8.21 avg300=3.02 total=1284920\nfull avg10=0.00 avg60=0.00 avg300=0.00 total=0\n";

    #[test]
    fn memory_in_use_is_measured_against_what_is_available_not_what_is_free() {
        // A healthy host keeps very little free because the rest is droppable cache. A detector
        // watching MemFree would report every warm cache as an emergency: free here is 4.9%, and
        // available is 40%.
        let used = memory_used(MEMINFO).expect("a well-formed meminfo");
        assert!((used - 0.6).abs() < 0.01, "{used}");

        let by_free = 1.0 - (198_372.0 / 4_025_536.0);
        assert!(
            by_free > 0.94,
            "the two measures should disagree sharply, or this test proves nothing"
        );
    }

    #[test]
    fn a_host_with_no_swap_has_no_reading_rather_than_a_reading_of_zero() {
        // "There is no swap" and "swap is empty" are different facts, and only one of them can
        // later become alarming.
        let swapless =
            "MemTotal: 4025536 kB\nSwapTotal:             0 kB\nSwapFree:              0 kB\n";
        assert_eq!(swap_used(swapless), None);
        assert!(swap_used(MEMINFO).is_some());
    }

    #[test]
    fn pressure_is_read_from_some_and_not_from_full() {
        // `full` on a working host is zero right up until it is a catastrophe; `some` is what a
        // person experiences as the machine being slow.
        let some = pressure_some_avg10(PRESSURE).expect("a well-formed pressure file");
        assert!((some - 12.43).abs() < f64::EPSILON);
    }

    #[test]
    fn anything_this_cannot_read_produces_no_reading_at_all() {
        // A parser returning zero for an unreadable file would put a fabricated number into a
        // baseline, which is where a fabricated number does the most damage: it moves what the host
        // believes is ordinary about itself.
        for unreadable in ["", "\n", "garbage", "MemTotal: not-a-number kB"] {
            assert_eq!(memory_used(unreadable), None, "{unreadable:?}");
            assert_eq!(swap_used(unreadable), None, "{unreadable:?}");
            assert_eq!(pressure_some_avg10(unreadable), None, "{unreadable:?}");
        }
        assert_eq!(load_average(""), None);
        assert_eq!(filesystem_used(0, 0), None);
    }

    #[test]
    fn a_kernel_without_pressure_accounting_simply_says_nothing() {
        // Pressure stall information is a configuration option. On a kernel built without it the
        // file is absent, and the honest result is one fewer subject rather than a zero that reads
        // as a perfectly calm machine.
        assert_eq!(pressure_some_avg10("full avg10=0.00\n"), None);
    }

    #[test]
    fn the_load_average_is_the_first_field() {
        assert_eq!(load_average("0.42 0.51 0.60 1/238 9182\n"), Some(0.42));
    }

    #[test]
    fn a_filesystem_is_measured_by_what_an_ordinary_process_can_still_use() {
        // The reserved portion is not usable by the services that will fail when it runs out.
        let used = filesystem_used(1000, 50).expect("a real filesystem");
        assert!((used - 0.95).abs() < f64::EPSILON);
    }

    #[test]
    fn failed_units_are_counted_from_the_listing_and_not_from_its_summary() {
        let listing = "● postgresql.service loaded failed failed PostgreSQL RDBMS\n● nginx.service   loaded failed failed A high performance web server\n";
        assert!((failed_units(listing) - 2.0).abs() < f64::EPSILON);
        assert!((failed_units("") - 0.0).abs() < f64::EPSILON);
        // The legend systemd prints after the units must not be counted as units.
        let with_legend = format!(
            "{listing}\nLOAD   = Reflects whether the unit definition was properly loaded.\n2 loaded units listed.\n"
        );
        assert!((failed_units(&with_legend) - 2.0).abs() < f64::EPSILON);
    }
}
