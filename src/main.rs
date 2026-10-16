use findlich::{index::Index, parse::parse};
use std::time::Instant;

fn main() {
    let input = std::env::args().skip(1).collect::<Vec<_>>().join(" ");

    let started = Instant::now();
    let index = Index::build();
    println!("Indexed {} files in {:?}", index.len(), started.elapsed());
    for (reason, folders) in index.skipped() {
        println!("  skipped {folders} folders: {reason}");
    }
    println!("  (plus anything listed in .gitignore files)");

    let q = parse(&input);
    let plan = q.resolve();
    println!("{q:?}");
    println!("  → {}\n", q.to_find_command(&plan));

    let started = Instant::now();
    let result = index.search(&q, &plan, 20);
    println!("Searched in {:?}\n", started.elapsed());

    for f in &result.matches {
        println!("{}", f.path.display());
    }
    println!("\n{} files found", result.total);
}
