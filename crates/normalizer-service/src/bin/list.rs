// crates/normalizer-service/src/bin/list.rs
mod generated {
    include!(concat!(env!("OUT_DIR"), "/registry.rs"));
}

fn main() {
    // Prints the program_ids registered by the generated extractors
    let regs = generated::registry();
    println!("Registered program_ids:");
    for e in regs {
        println!(" - {}", e.program_id());
    }
}
