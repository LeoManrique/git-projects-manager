//! Times `Scanner::scan_folder` so scanner changes can be measured instead of
//! argued about. Run with `just bench-scan <path>` (add `local` for an offline
//! scan). Prints the bucket counts too, so a change that makes a scan faster by
//! quietly finding fewer repos is visible.

use gpm_core::domain::scanner::Scanner;
use std::time::Instant;

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: timescan <path> [local]");
        std::process::exit(2);
    };
    let only_local_checks = args.next().is_some_and(|a| a == "local");

    println!(
        "scanning {path} ({})",
        if only_local_checks { "local only" } else { "online" }
    );
    for run in 1..=3 {
        let started = Instant::now();
        // Uninitialized detection stays on: it is a second walk of the tree, so
        // leaving it out would time a scan the app does not run by default.
        let r = Scanner::new().scan_folder(std::path::Path::new(&path), only_local_checks, true);
        println!(
            "run {run}: {:.2}s  repos={} clean={} changed={} unpushed={} unpulled={} \
             errors={} uninitialized={} unpublished={} remote_not_found={}",
            started.elapsed().as_secs_f64(),
            r.total_repositories,
            r.clean.len(),
            r.with_changes.len(),
            r.with_unpushed.len(),
            r.with_unpulled.len(),
            r.errors.len(),
            r.uninitialized.len(),
            r.unpublished.len(),
            r.remote_not_found.len(),
        );
    }
}
