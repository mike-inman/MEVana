Last updated: 2025-09-15

## Ops Runbook (AI‑friendly)

Date: 2025-08-19

### Disk Pressure Playbook
1) Identify heavy topics (Kafka):
   - `docker exec kafka bash -lc 'du -sh /var/lib/kafka/data/* | sort -h | tail -n 30'`
2) Temporarily reduce retention (example):
   - `docker exec kafka kafka-configs --bootstrap-server localhost:9092 --alter --entity-type topics --entity-name <topic> --add-config retention.ms=600000,retention.bytes=536870912,segment.ms=60000`
3) Confirm:
   - `docker exec kafka kafka-topics --bootstrap-server localhost:9092 --describe --topic <topic>`
4) Watch reclaim:
   - `docker system df -v | cat`
   - Repeat step 1
5) Restore sane retention after pressure subsides.

### ZooKeeper Maintenance
- Autopurge enabled via compose env:
  - `ZOOKEEPER_AUTOPURGE_SNAP_RETAIN_COUNT=3`
  - `ZOOKEEPER_AUTOPURGE_PURGE_INTERVAL=24`
- Health:
  - `docker logs zookeeper | tail -n 100`

### Service Health
- Kafka: `docker logs kafka | tail -n 200`
- Kafka-UI: http://localhost:8080
- Prometheus: http://localhost:9090
- Grafana: http://localhost:3000

### Common Commands
- List topics: `docker exec kafka kafka-topics --bootstrap-server localhost:9092 --list`
- Consume sample: `docker exec kafka kafka-console-consumer --bootstrap-server localhost:9092 --topic <topic> --from-beginning --max-messages 1`


