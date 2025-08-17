# Fast Mode (Low-Latency Settings)

Enable via `FAST_MODE=1` on services that support it.

## Producer
- linger.ms=0
- acks=1
- compression.type=lz4 (or none)
- enable.idempotence=false
- message.timeout.ms=100

## Consumer
- fetch.min.bytes=1
- fetch.wait.max.ms=1–2
- partition.assignment.strategy=cooperative-sticky

## Notes
- Use in the hot path only; UI/analytics can keep default settings.
- Ensure co-location with broker and minimal partitions for reduced hop latency.
