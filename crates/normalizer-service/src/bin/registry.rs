// crates/normalizer-service/src/bin/registry.rs
mod generated {
    // emitted by build.rs from normalizer-codegen
    include!(concat!(env!("OUT_DIR"), "/registry.rs"));
}

fn main() {
    let regs = generated::registry();
    println!("normalizer-service: {} decoder(s) registered", regs.len());
    for e in regs {
        println!(" - program_id: {}", e.program_id());
    }
}
