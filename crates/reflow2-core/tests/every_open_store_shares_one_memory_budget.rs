//! Every store one process opens draws on ONE memory budget.
//!
//! `req:one-open-design-costs-a-deliberate-amount-of-memory`. Left to RocksDB's
//! defaults, each store brought its own write buffers and block caches, so a
//! server holding many designs cost the sum of every store's defaults — a
//! number nobody chose. Since 2026-09-27 the process sets one budget
//! (`set_store_memory_budget`, `--store-memory`) and every store's memtables are
//! charged to it.
//!
//! ⭐ ONE TEST IN ITS OWN BINARY, ON PURPOSE. The budget is process-wide, so a
//! test sharing a process with other tests that open and flush stores would read
//! their memtables too. Alone here, a rise after writing to the second store can
//! only be that store's memtable landing on the same budget as the first.

#![cfg(feature = "rocksdb")]

use reflow2_core::DesignGraph;

fn store(tag: &str) -> String {
    let dir = std::env::temp_dir().join(format!(
        "reflow2-budget-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    dir.to_str().expect("temp path is utf-8").to_string()
}

fn write(graph: &mut DesignGraph, prefix: &str) {
    let statement = "A statement long enough to put real bytes in the memtable. ".repeat(8);
    for i in 0..400 {
        graph
            .add_requirement(
                &format!("req:{prefix}-{i}"),
                &format!("{prefix} {i}"),
                &statement,
            )
            .expect("a requirement is written");
    }
}

#[test]
fn every_store_this_process_opens_is_charged_to_one_write_buffer_budget() {
    let chosen = 64 * 1024 * 1024;
    assert_eq!(
        reflow2_core::set_store_memory_budget(chosen),
        chosen,
        "set before any store opens, the budget is the one chosen"
    );

    let (_, before) = reflow2_core::store_memory_usage();
    let (path_a, path_b) = (store("a"), store("b"));
    let mut a = DesignGraph::open_rocksdb(&path_a).expect("store a opens");
    write(&mut a, "a");
    let (_, after_a) = reflow2_core::store_memory_usage();
    assert!(
        after_a > before,
        "writing to one store is charged to the shared budget ({before} -> {after_a})"
    );

    let mut b = DesignGraph::open_rocksdb(&path_b).expect("store b opens");
    write(&mut b, "b");
    let (_, after_b) = reflow2_core::store_memory_usage();
    assert!(
        after_b > after_a,
        "a SECOND store's memtable lands on the SAME budget, not one of its own \
         ({after_a} -> {after_b})"
    );

    assert_eq!(
        reflow2_core::set_store_memory_budget(1),
        chosen,
        "once a store is open the budget is fixed, and a late setter is told what is in force"
    );

    drop(a);
    drop(b);
    let _ = std::fs::remove_dir_all(&path_a);
    let _ = std::fs::remove_dir_all(&path_b);
}
