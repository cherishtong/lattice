use std::time::Instant;

use lattice_content::SourcePack;

fn main() {
    let pack = SourcePack::discover().expect("CPython source pack");
    let files = pack.source_files().expect("source files");
    let started = Instant::now();
    let mut parsed = 0usize;
    let mut reused = 0usize;
    let mut failed = 0usize;

    for path in &files {
        let Ok(source) = pack.read_text(path) else {
            failed += 1;
            continue;
        };
        let cache_path = pack.document_cache_path(path);
        match lattice_parser::ensure_document_cache(path, &source, &cache_path) {
            Ok(true) => parsed += 1,
            Ok(false) => reused += 1,
            Err(_) => failed += 1,
        }
    }

    println!(
        "documents: {} total, {} parsed, {} reused, {} failed in {:?}",
        files.len(),
        parsed,
        reused,
        failed,
        started.elapsed()
    );

    let cache_dir = pack.cache_dir().join("documents");
    let bytes: u64 = std::fs::read_dir(cache_dir)
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|entry| entry.metadata().ok())
                .filter(|metadata| metadata.is_file())
                .map(|metadata| metadata.len())
                .sum()
        })
        .unwrap_or(0);
    println!("document cache: {:.2} MiB", bytes as f64 / 1024.0 / 1024.0);
}
