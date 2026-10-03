//! The MCP tool surface, carved into the systems the design already names.
//!
//! BL-181. `service.rs` holds the service itself — the graph handle, the
//! constructors, the request shapes — and each module here holds one slice of
//! the tools, declaring its own `tool_router`. `ReflowService::new` sums them.
//!
//! The carving follows `dec:bl83a-functional-decomposition` ("reflow2's systems
//! are functional, not its file tree") rather than the crate layout, because a
//! file tree that disagrees with the design's own decomposition is exactly what
//! this split existed to fix.

pub mod ask;
pub mod assure;
pub mod built;
pub mod capture;
pub mod claims_tools;
pub mod coherence;
pub mod exchange;
pub mod ingest_tools;
pub mod operate_tools;
pub mod query;
pub mod skills_tools;
pub mod temporal_tools;

#[cfg(test)]
mod tests {
    /// EVERY FILE A TOOL WRITES PASSES THE READ-ONLY FILE GUARD.
    ///
    /// `export_graph` and `export_surface` are annotated read-only because they
    /// do not write the graph, so `write_lock` — the one wall read-only mode
    /// had — never saw them, and a read-only server wrote a file at any path
    /// a caller named (fact:root-cause-one-regex-cannot-separate-door-reads-because-the-read-set-is-not-in-the-command-and-not-served-2026-10-02).
    /// The graph guard is complete by construction (a write needs the guard);
    /// a file write needs no guard to happen, so this holds the count instead:
    /// in each tool module, as many `file_write_permitted` calls as file
    /// writes. A new tool that writes a file without the guard fails here.
    #[test]
    fn every_file_write_in_a_tool_module_is_guarded_by_read_only() {
        const WRITES: [&str; 9] = [
            "fs::write(",
            "File::create(",
            "OpenOptions::new(",
            "fs::copy(",
            "fs::rename(",
            "fs::remove_file(",
            "fs::remove_dir",
            "fs::create_dir",
            "chain_and_write(",
        ];
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tools");
        let mut checked = 0;
        for entry in std::fs::read_dir(&dir).expect("src/tools").flatten() {
            let path = entry.path();
            // This file holds the patterns themselves, as string literals.
            if path.extension().and_then(|e| e.to_str()) != Some("rs")
                || path.file_name().and_then(|n| n.to_str()) == Some("mod.rs")
            {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            let code: Vec<&str> = text
                .lines()
                .map(str::trim_start)
                .filter(|l| !l.starts_with("//"))
                .collect();
            let count = |pat: &str| code.iter().map(|l| l.matches(pat).count()).sum::<usize>();
            let writes: usize = WRITES.iter().map(|w| count(w)).sum();
            let guards = count("file_write_permitted(");
            checked += writes;
            assert!(
                guards >= writes,
                "{}: {writes} file write(s) and only {guards} `file_write_permitted` call(s). \
                 A tool that writes a file must ask `self.file_write_permitted(tool, path)?` \
                 first, or a read-only server writes it.",
                path.display()
            );
        }
        assert!(
            checked >= 2,
            "found only {checked} file writes: the scan is reading nothing"
        );
    }
}
