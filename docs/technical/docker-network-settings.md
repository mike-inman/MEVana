# Docker Network Settings Documentation
*Created: 2025-01-27*

## Overview
This document captures the network configuration settings from the MEVana Docker Compose infrastructure. These settings are preserved and should not be changed without careful consideration.

## Network Configuration

### Primary Network: `mevana-network`
```yaml
networks:
  mevana-network:
    driver: bridge
    ipam:
      config:
        - subnet: 172.20.0.0/16
          gateway: 172.20.0.1
    driver_opts:
      com.docker.network.bridge.name: mevana-br0
      com.docker.network.bridge.enable_icc: "true"
      com.docker.network.bridge.enable_ip_masquerade: "true"
      com.docker.network.bridge.host_binding_ipv4: "0.0.0.0"
      com.docker.network.bridge.mtu: "1500"
      com.docker.network.driver.mtu: "1500"
```

### Network Details
- **Driver**: Bridge network for container communication
- **Subnet**: 172.20.0.0/16 (65,534 available IP addresses)
- **Gateway**: 172.20.0.1
- **Bridge Name**: mevana-br0 (custom bridge interface)
- **Inter-Container Communication**: Enabled (icc: true)
- **IP Masquerading**: Enabled for external connectivity
- **MTU**: 1500 bytes (standard Ethernet frame size)

## Port Mappings

### Infrastructure Services
- **PostgreSQL**: 5432:5432
- **Zookeeper**: 2181:2181
- **Kafka**: 9093:9093 (external), 9101:9101 (JMX)
- **Redis**: 6379:6379
- **Prometheus**: 9090:9090
- **Grafana**: 3000:3000

## Network Security Considerations

### Bridge Network Benefits
- **Isolation**: Containers can communicate but are isolated from host network
- **Port Mapping**: Controlled exposure of services to host
- **DNS Resolution**: Automatic service discovery within the network

### Security Features
- **ICC Enabled**: Containers can communicate with each other
- **IP Masquerading**: Outbound traffic appears to come from host
- **Custom Bridge**: Dedicated network interface for MEVana services

## Usage Guidelines

### Do Not Change
- **Subnet Range**: 172.20.0.0/16 is reserved for MEVana
- **Gateway IP**: 172.20.0.1 is the network gateway
- **Bridge Name**: mevana-br0 is the custom bridge interface
- **MTU Settings**: 1500 is optimal for most network configurations

### Can Be Modified
- **Port Mappings**: Can be adjusted if conflicts arise
- **Additional Networks**: Can add overlay networks for complex deployments
- **Network Aliases**: Can add custom DNS aliases for services

## Troubleshooting

### Common Issues
1. **Port Conflicts**: Check if ports are already in use on host
2. **Network Connectivity**: Verify bridge network is created properly
3. **DNS Resolution**: Ensure containers can resolve each other's names

### Debug Commands
```bash
# Check network status
docker network ls
docker network inspect mevana-network

# Check bridge interface
ip addr show mevana-br0

# Test connectivity between containers
docker exec -it <container_name> ping <other_container_name>
```

## Future Considerations

### Scaling
- Current subnet supports up to 65,534 containers
- Can add additional networks for different service tiers
- Consider overlay networks for multi-host deployments

### Monitoring
- Network metrics available through Docker stats
- Prometheus can collect network performance data
- Grafana dashboards can visualize network traffic patterns
