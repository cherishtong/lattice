use std::time::Instant;

use lattice_content::SourcePack;
use lattice_query::SourceIndex;

fn main() {
    let started = Instant::now();
    let pack = SourcePack::discover().expect("CPython source pack");
    let tree = pack.scan().expect("source tree");
    println!(
        "scanned {} files in {:?}",
        tree.file_count,
        started.elapsed()
    );

    let index_started = Instant::now();
    let loaded = SourceIndex::load_or_build(&pack, &tree).expect("symbol index");
    println!(
        "indexed {} symbols in {:?} (cache loaded={}, persisted={})",
        loaded.index.symbol_count(),
        index_started.elapsed(),
        loaded.loaded_from_cache,
        loaded.persisted
    );
    let index = loaded.index;

    for query in [
        "list_resize",
        "PyList_New",
        "ArrayList",
        "HashMap",
        "Object",
        "listobject",
    ] {
        let results = index.search(query, 5, 3);
        println!(
            "{query}: {} symbols, {} files; first={:?}",
            results.symbols.len(),
            results.files.len(),
            results
                .symbols
                .first()
                .map(|symbol| (&symbol.name, &symbol.path, symbol.start_line))
        );
    }
}
