use shared_core::port_manager::PortManager;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 7 {
        eprintln!(
            "Usage: {} --service <name> --type <type> --preferred <port>",
            args[0]
        );
        std::process::exit(1);
    }

    let mut service_name = "";
    let mut service_type = "";
    let mut preferred_port = 8080u16;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--service" => {
                service_name = &args[i + 1];
                i += 2;
            }
            "--type" => {
                service_type = &args[i + 1];
                i += 2;
            }
            "--preferred" => {
                preferred_port = args[i + 1].parse().unwrap_or(8080);
                i += 2;
            }
            _ => i += 1,
        }
    }

    let mut port_manager = PortManager::new();
    let allocation = port_manager.allocate_port(service_name, service_type, Some(preferred_port));

    // Print the message for logging
    eprintln!("{}", allocation.message);

    // Print just the port number for scripts to capture
    println!("{}", allocation.allocated_port);
}
