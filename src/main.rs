use findlich::{index::Index, parse::parse};
use std::time::Instant;

fn main() {
    let input = std::env::args().skip(1).collect::<Vec<_>>().join(" ");

    let started = Instant::now();
    let index = Index::build();
    println!("Indexed {} files in {:?}", index.len(), started.elapsed());

    let q = parse(&input);
    println!("{q:?}");
    println!("  → {}\n", q.to_find_command());

    let started = Instant::now();
    let results = index.search(&q);
    println!("Searched in {:?}\n", started.elapsed());

    for f in results.iter().take(20) {
        println!("{}", f.path.display());
    }
    println!("\n{} files found", results.len());
}
