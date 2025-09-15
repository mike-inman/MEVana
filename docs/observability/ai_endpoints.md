# AI Integration Endpoints
Last updated: 2025-09-15

HTTP + WebSocket surfaces for AI agents.

## Endpoints
GET /ai/recent?limit=N  (N<=256)
GET /ai/advice?limit=N  (N<=100)
POST /ai/advice {"message":"text","tag":"optional"}
GET /ai/config  -> live snapshot of min_profit_bps & max_slippage_bps (atomic)
POST /ai/config/update {"min_profit_bps":40,"max_slippage_bps":120} (partial fields allowed)
POST /ai/config/reload  (histogram bucket config reload)
WS  /ws/arbs (stream arbitrage signals JSON)

## Auth
Set AI_API_TOKEN to require X-AI-Token header on mutating routes (advice/config update). Omit / empty -> open.

## Limits
limit must be >0 and <= MAX else 400.

## Persistence & Rotation
JSONL append-only logs under AI_LOG_DIR (default ./ai_logs):
- signals.jsonl
- advice.jsonl
Automatic rotation triggers on either:
1. File size >= AI_LOG_MAX_BYTES (default 10MB)
2. Day boundary (UTC day index change)
Old files shift with suffixes: `.1`, `.2`, ... up to AI_LOG_MAX_ROTATIONS (default 5). `.1` is most recent previous segment. A fresh base file is opened post-rotation.

Env vars:
- AI_LOG_DIR: directory (default ./ai_logs)
- AI_LOG_MAX_BYTES: size threshold (bytes, default 10485760)
- AI_LOG_MAX_ROTATIONS: retain count (default 5)
- AI_API_TOKEN: auth token (see Auth)
- ARB_MIN_PROFIT_BPS / ARB_MAX_SLIPPAGE_BPS: initial thresholds (runtime updates override via atomics)

## Runtime Threshold Semantics
The engine reads `min_profit_bps` & `max_slippage_bps` from global atomic variables. POST /ai/config/update mutates these instantly without restart. Missing fields leave existing values unchanged. GET /ai/config always reflects current atomics (not original env defaults).

## New Metrics (Diagnostics Feature)
- arb_edge_staleness_ms: age since last edge update (ms)
- arb_process_resident_bytes: resident memory bytes (best-effort)

## Examples
curl /ai/recent
curl -H "X-AI-Token:$AI_API_TOKEN" -d '{"message":"tighten thresholds"}' -H 'content-type: application/json' /ai/advice
curl -H "X-AI-Token:$AI_API_TOKEN" -d '{"min_profit_bps":75}' -H 'content-type: application/json' /ai/config/update
