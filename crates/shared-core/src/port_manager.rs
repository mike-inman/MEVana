use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::net::{SocketAddr, TcpListener};
use std::time::SystemTime;

/// Port allocation status and reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortAllocation {
    pub service_name: String,
    pub requested_port: u16,
    pub allocated_port: u16,
    pub status: PortStatus,
    pub timestamp: u64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PortStatus {
    Success,     // Got requested port
    Reallocated, // Had to use different port
    Failed,      // Couldn't allocate any port in range
}

/// Smart port manager that prevents conflicts and reports allocations
pub struct PortManager {
    allocations: HashMap<String, PortAllocation>,
    port_ranges: HashMap<String, (u16, u16)>, // service_type -> (min, max)
    allocation_file: String,
}

impl Default for PortManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PortManager {
    pub fn new() -> Self {
        let mut port_ranges = HashMap::new();

        // Define port ranges by service type
        port_ranges.insert("http".to_string(), (8080, 8099));
        port_ranges.insert("websocket".to_string(), (8100, 8119));
        port_ranges.insert("metrics".to_string(), (8120, 8139));
        port_ranges.insert("grpc".to_string(), (9000, 9019));
        port_ranges.insert("database".to_string(), (5432, 5449));
        port_ranges.insert("cache".to_string(), (6379, 6399));
        port_ranges.insert("message".to_string(), (9092, 9109)); // Kafka, etc.

        let mut manager = Self {
            allocations: HashMap::new(),
            port_ranges,
            allocation_file: "/tmp/mevana_port_allocations.json".to_string(),
        };

        manager.load_existing_allocations();
        manager
    }

    /// Allocate a port for a service, handling conflicts automatically
    pub fn allocate_port(
        &mut self,
        service_name: &str,
        service_type: &str,
        preferred_port: Option<u16>,
    ) -> PortAllocation {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Get port range for service type
        let (min_port, max_port) = self
            .port_ranges
            .get(service_type)
            .copied()
            .unwrap_or((8080, 8999)); // Default range

        // Determine preferred port
        let preferred = preferred_port.unwrap_or(min_port);

        // Try preferred port first
        if preferred >= min_port && preferred <= max_port && self.is_port_available(preferred) {
            let allocation = PortAllocation {
                service_name: service_name.to_string(),
                requested_port: preferred,
                allocated_port: preferred,
                status: PortStatus::Success,
                timestamp,
                message: format!("✅ {} allocated preferred port {}", service_name, preferred),
            };

            self.allocations
                .insert(service_name.to_string(), allocation.clone());
            self.save_allocations();
            return allocation;
        }

        // Preferred port not available, find next free port in range
        for port in min_port..=max_port {
            if self.is_port_available(port) {
                let allocation = PortAllocation {
                    service_name: service_name.to_string(),
                    requested_port: preferred,
                    allocated_port: port,
                    status: PortStatus::Reallocated,
                    timestamp,
                    message: format!(
                        "⚠️  {} reallocated from {} to {} (conflict resolved)",
                        service_name, preferred, port
                    ),
                };

                self.allocations
                    .insert(service_name.to_string(), allocation.clone());
                self.save_allocations();
                return allocation;
            }
        }

        // No ports available in range
        let allocation = PortAllocation {
            service_name: service_name.to_string(),
            requested_port: preferred,
            allocated_port: 0,
            status: PortStatus::Failed,
            timestamp,
            message: format!(
                "❌ {} failed to allocate port - range {}-{} exhausted",
                service_name, min_port, max_port
            ),
        };

        self.allocations
            .insert(service_name.to_string(), allocation.clone());
        self.save_allocations();
        allocation
    }

    /// Check if a port is available
    fn is_port_available(&self, port: u16) -> bool {
        // Check if already allocated by us
        for allocation in self.allocations.values() {
            if allocation.allocated_port == port && allocation.status != PortStatus::Failed {
                return false;
            }
        }

        // Try to bind to the port
        TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], port))).is_ok()
    }

    /// Free a port when service stops
    pub fn free_port(&mut self, service_name: &str) -> Option<u16> {
        if let Some(allocation) = self.allocations.remove(service_name) {
            self.save_allocations();
            println!(
                "🔓 Freed port {} for {}",
                allocation.allocated_port, service_name
            );
            Some(allocation.allocated_port)
        } else {
            None
        }
    }

    /// Get current allocation for a service
    pub fn get_allocation(&self, service_name: &str) -> Option<&PortAllocation> {
        self.allocations.get(service_name)
    }

    /// Get all current allocations
    pub fn get_all_allocations(&self) -> Vec<&PortAllocation> {
        self.allocations.values().collect()
    }

    /// Generate deployment report
    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        report.push_str("🚀 MEVana Port Allocation Report\n");
        report.push_str("================================\n\n");

        for allocation in self.allocations.values() {
            report.push_str(&format!("{}\n", allocation.message));
        }

        report.push_str("\n📊 Current Port Usage:\n");
        for allocation in self.allocations.values() {
            if allocation.status != PortStatus::Failed {
                report.push_str(&format!(
                    "  • {}: port {}\n",
                    allocation.service_name, allocation.allocated_port
                ));
            }
        }

        // Show available ports in each range
        report.push_str("\n🔍 Available Ports by Service Type:\n");
        for (service_type, (min, max)) in &self.port_ranges {
            let available_count = (*min..=*max)
                .filter(|&port| self.is_port_available(port))
                .count();
            report.push_str(&format!(
                "  • {}: {} available ({}-{})\n",
                service_type, available_count, min, max
            ));
        }

        report
    }

    /// Save allocations to file for persistence
    fn save_allocations(&self) {
        if let Ok(json) = serde_json::to_string_pretty(&self.allocations) {
            let _ = fs::write(&self.allocation_file, json);
        }
    }

    /// Load existing allocations from file
    fn load_existing_allocations(&mut self) {
        if let Ok(content) = fs::read_to_string(&self.allocation_file) {
            if let Ok(allocations) = serde_json::from_str(&content) {
                self.allocations = allocations;
            }
        }
    }

    /// Kill process using a specific port (emergency cleanup)
    pub fn kill_port_process(&self, port: u16) -> Result<(), String> {
        #[cfg(unix)]
        {
            use std::process::Command;

            let output = Command::new("lsof")
                .args(["-t", "-i", &format!(":{}", port)])
                .output()
                .map_err(|e| format!("Failed to find process on port {}: {}", port, e))?;

            if !output.stdout.is_empty() {
                let pid = String::from_utf8_lossy(&output.stdout).trim().to_string();

                let kill_output = Command::new("kill")
                    .args(["-9", &pid])
                    .output()
                    .map_err(|e| format!("Failed to kill process {}: {}", pid, e))?;

                if kill_output.status.success() {
                    println!("💀 Killed process {} using port {}", pid, port);
                    Ok(())
                } else {
                    Err(format!("Failed to kill process {}", pid))
                }
            } else {
                Err(format!("No process found using port {}", port))
            }
        }

        #[cfg(not(unix))]
        {
            Err("Port cleanup not supported on this platform".to_string())
        }
    }
}

/// Environment variable helper for port allocation
pub fn get_port_with_fallback(
    service_name: &str,
    service_type: &str,
    env_var: &str,
    default_port: u16,
) -> (u16, String) {
    let mut port_manager = PortManager::new();

    // Check if port specified in environment
    let requested_port = std::env::var(env_var)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default_port);

    let allocation = port_manager.allocate_port(service_name, service_type, Some(requested_port));

    (allocation.allocated_port, allocation.message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_allocation() {
        let mut manager = PortManager::new();

        // Test successful allocation
        let allocation = manager.allocate_port("test-service", "http", Some(8085));
        assert!(allocation.allocated_port > 0);

        // Test conflict resolution
        let allocation2 = manager.allocate_port("test-service-2", "http", Some(8085));
        if allocation.allocated_port == 8085 {
            // First service got 8085, second should get different port
            assert_ne!(allocation2.allocated_port, 8085);
            assert_eq!(allocation2.status, PortStatus::Reallocated);
        }
    }

    #[test]
    fn test_port_ranges() {
        let mut manager = PortManager::new();

        let http_allocation = manager.allocate_port("http-service", "http", None);
        assert!(http_allocation.allocated_port >= 8080 && http_allocation.allocated_port <= 8099);

        let grpc_allocation = manager.allocate_port("grpc-service", "grpc", None);
        assert!(grpc_allocation.allocated_port >= 9000 && grpc_allocation.allocated_port <= 9019);
    }
}
