use findlich::{parse::parse, search::search};

fn main() {
    let input = std::env::args().skip(1).collect::<Vec<_>>().join(" ");

    let q = parse(&input);
    println!("{q:?}");
    println!("  → {}\n", q.to_find_command());

    let results = search(&q);
    for path in results.iter().take(20) {
        println!("{}", path.display());
    }
    println!("\n{} files found", results.len());
}
