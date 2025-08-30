fn main() {
    let regs = normalizer_generated::registry::registry();
    println!("extractors={}", regs.len());
    for ex in regs {
        println!("{}", ex.program_id());
    }
}
