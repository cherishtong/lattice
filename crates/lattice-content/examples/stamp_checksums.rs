use lattice_content::SourcePack;

fn main() {
    let packs = std::env::args()
        .nth(1)
        .map(|root| SourcePack::open(root.into()).map(|pack| vec![pack]))
        .unwrap_or_else(SourcePack::installed)
        .unwrap_or_else(|error| {
            eprintln!("{error}");
            std::process::exit(1);
        });
    for pack in packs {
        match pack.stamp_checksum() {
            Ok(checksum) => println!("{}: {checksum}", pack.name()),
            Err(error) => eprintln!("{}: {error}", pack.name()),
        }
    }
}
