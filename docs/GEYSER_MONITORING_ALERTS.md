# Geyser Stream Monitoring & Critical Alerts
Last updated: 2025-09-15

**Last Updated**: 2025-09-14
**Priority**: 🚨 **CRITICAL** - Production Safety System
**Context**: Prevent silent Geyser failures that could stop MEV detection

---

## 🎯 **CRITICAL REQUIREMENTS**

### **Core Monitoring Metrics**
The system already exports these key metrics for Geyser monitoring:

1. **`multi_geyser_stream_healthy{stream_id="N"}`** - Health status (1=healthy, 0=failed)
2. **`multi_geyser_stream_messages_total{stream_id="N"}`** - Message count per stream
3. **`multi_geyser_stream_seconds_since_last_message{stream_id="N"}`** - Time since last message
4. **`multi_geyser_dex_messages_total{dex="orca|raydium"}`** - DEX-specific message flow

---

## 🚨 **ALERT CONDITIONS**

### **1. Stream Failure Alerts (CRITICAL)**
```yaml
# Stream completely down
alert: GeyserStreamDown
expr: multi_geyser_stream_healthy == 0
for: 30s
severity: critical
summary: "Geyser stream {{$labels.stream_id}} is DOWN"

# No messages received in 60 seconds
alert: GeyserStreamStalled
expr: multi_geyser_stream_seconds_since_last_message > 60
for: 1m
severity: critical
summary: "Geyser stream {{$labels.stream_id}} stalled for {{$value}} seconds"
```

### **2. Message Flow Alerts (HIGH)**
```yaml
# Message rate dropped significantly
alert: GeyserMessageRateDropped
expr: rate(multi_geyser_stream_messages_total[5m]) < 0.1
for: 2m
severity: high
summary: "Geyser stream {{$labels.stream_id}} message rate critically low"

# No DEX messages in 5 minutes (trading stopped)
alert: GeyserDexMessagesStopped
expr: increase(multi_geyser_dex_messages_total[5m]) == 0
for: 5m
severity: high
summary: "No {{$labels.dex}} messages received in 5 minutes"
```

### **3. Connection Stability Alerts (MEDIUM)**
```yaml
# Multiple stream failures
alert: GeyserMultipleStreamFailures
expr: sum(multi_geyser_stream_healthy) < 2
for: 1m
severity: medium
summary: "Less than 2 Geyser streams healthy - redundancy compromised"
```

---

## 📱 **ALERTING IMPLEMENTATION**

### **Phase 1: Immediate (Use existing metrics)**
Create Prometheus alerting rules that monitor existing metrics:

```yaml
# File: infrastructure/prometheus/alerts/geyser_alerts.yml
groups:
  - name: geyser_critical
    interval: 30s
    rules:
      - alert: GeyserStreamDown
        expr: multi_geyser_stream_healthy == 0
        for: 30s
        labels:
          severity: critical
          component: geyser
        annotations:
          summary: "🚨 CRITICAL: Geyser stream {{$labels.stream_id}} is DOWN"
          description: "Stream {{$labels.stream_id}} health check failed. MEV detection may be compromised."

      - alert: GeyserStreamStalled
        expr: multi_geyser_stream_seconds_since_last_message > 60
        for: 1m
        labels:
          severity: critical
          component: geyser
        annotations:
          summary: "🚨 CRITICAL: Geyser stream {{$labels.stream_id}} stalled"
          description: "No messages received for {{$value}} seconds. Immediate investigation required."
```

### **Phase 2: Enhanced Monitoring**
Add specific Geyser connection metrics to the code:

```rust
// Additional metrics to add to hotpath_arb_engine
static GEYSER_CONNECTION_STATUS: Lazy<IntGaugeVec> = Lazy::new(|| {
    IntGaugeVec::new(
        Opts::new("geyser_connection_status", "Geyser connection status")
            .const_label("component", "geyser"),
        &["endpoint", "status"]
    ).unwrap()
});

static GEYSER_RECONNECT_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    IntCounterVec::new(
        Opts::new("geyser_reconnect_total", "Total Geyser reconnection attempts"),
        &["endpoint", "reason"]
    ).unwrap()
});

static GEYSER_LATENCY_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    HistogramVec::new(
        histogram_opts!("geyser_message_latency_seconds", "Geyser message processing latency"),
        &["stream_id", "message_type"]
    ).unwrap()
});
```

---

## 🔔 **NOTIFICATION CHANNELS**

### **Immediate Setup Options**

#### **1. Webhook Alerts**
```bash
# Simple webhook notification
curl -X POST https://your-webhook-url \
  -H "Content-Type: application/json" \
  -d '{
    "alert": "GeyserStreamDown",
    "stream_id": "0",
    "severity": "critical",
    "timestamp": "'$(date -u +%Y-%m-%dT%H:%M:%SZ)'"
  }'
```

#### **2. Slack Integration**
```yaml
# Prometheus Alertmanager config
route:
  group_by: ['alertname']
  group_wait: 10s
  group_interval: 10s
  repeat_interval: 1m
  receiver: 'geyser-alerts'

receivers:
  - name: 'geyser-alerts'
    slack_configs:
      - api_url: 'YOUR_SLACK_WEBHOOK_URL'
        channel: '#mevana-alerts'
        title: '🚨 MEVana Geyser Alert'
        text: '{{ range .Alerts }}{{ .Annotations.summary }}{{ end }}'
```

#### **3. Telegram Alerts (RECOMMENDED)**
```bash
#!/bin/bash
# File: scripts/telegram_alert.sh
# Telegram Bot Setup:
# 1. Create bot with @BotFather on Telegram
# 2. Get bot token and chat ID

TELEGRAM_BOT_TOKEN="YOUR_BOT_TOKEN"
TELEGRAM_CHAT_ID="YOUR_CHAT_ID"

send_telegram_alert() {
    local message="$1"
    local severity="$2"

    # Add emoji based on severity
    case $severity in
        "critical") emoji="🚨" ;;
        "high") emoji="⚠️" ;;
        "medium") emoji="📢" ;;
        *) emoji="ℹ️" ;;
    esac

    # Send message
    curl -s -X POST "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/sendMessage" \
        -d "chat_id=${TELEGRAM_CHAT_ID}" \
        -d "text=${emoji} MEVana Alert: ${message}" \
        -d "parse_mode=HTML" \
        -d "disable_web_page_preview=true"
}

# Usage examples:
# send_telegram_alert "Geyser stream 0 is DOWN" "critical"
# send_telegram_alert "Stream recovered successfully" "medium"
```

#### **4. Telegram + Prometheus Integration**
Create a webhook receiver for Prometheus alerts:

```python
#!/usr/bin/env python3
# File: scripts/telegram_webhook.py
import requests
import json
from flask import Flask, request

app = Flask(__name__)

TELEGRAM_BOT_TOKEN = "YOUR_BOT_TOKEN"
TELEGRAM_CHAT_ID = "YOUR_CHAT_ID"

def send_telegram(message):
    url = f"https://api.telegram.org/bot{TELEGRAM_BOT_TOKEN}/sendMessage"
    data = {
        "chat_id": TELEGRAM_CHAT_ID,
        "text": message,
        "parse_mode": "HTML"
    }
    requests.post(url, data=data)

@app.route('/webhook', methods=['POST'])
def webhook():
    data = request.get_json()

    for alert in data.get('alerts', []):
        status = alert.get('status', 'unknown')
        alert_name = alert.get('labels', {}).get('alertname', 'Unknown')
        stream_id = alert.get('labels', {}).get('stream_id', 'N/A')

        if status == 'firing':
            emoji = "🚨" if "critical" in alert.get('labels', {}).get('severity', '') else "⚠️"
            message = f"{emoji} <b>ALERT FIRING</b>\n"
            message += f"Alert: {alert_name}\n"
            message += f"Stream: {stream_id}\n"
            message += f"Time: {alert.get('startsAt', 'Unknown')}"
        else:
            message = f"✅ <b>ALERT RESOLVED</b>\n"
            message += f"Alert: {alert_name}\n"
            message += f"Stream: {stream_id}"

        send_telegram(message)

    return "OK", 200

if __name__ == '__main__':
    app.run(host='0.0.0.0', port=5001)
```

#### **5. Email Notifications (Backup)**
```yaml
receivers:
  - name: 'geyser-critical'
    email_configs:
      - to: 'alerts@yourcompany.com'
        from: 'prometheus@mevana.com'
        subject: '🚨 CRITICAL: Geyser Stream Failure'
        body: |
          Alert: {{ .GroupLabels.alertname }}
          Stream: {{ .CommonLabels.stream_id }}
          Time: {{ .CommonAnnotations.timestamp }}

          {{ range .Alerts }}
          {{ .Annotations.description }}
          {{ end }}
```

---

## ⚡ **AUTOMATED RECOVERY**

### **Recovery Scripts**
Create automatic recovery mechanisms:

```bash
#!/bin/bash
# File: scripts/geyser_recovery.sh

GEYSER_ENDPOINT="https://grpc.ny.shyft.to"
X_TOKEN="<SHYFT_X_TOKEN>"

recover_geyser_stream() {
    local stream_id=$1
    echo "🔄 Attempting to recover Geyser stream $stream_id..."

    # Kill existing process
    pkill -f "hotpath_arb_engine.*$stream_id" || true
    sleep 5

    # Restart with exponential backoff
    MULTI_ENDPOINT_MODE=true \
    PAPER_TRADING=true \
    DATABASE_URL="postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb" \
    GEYSER_ENDPOINT="$GEYSER_ENDPOINT" \
    X_TOKEN="$X_TOKEN" \
    METRICS_PORT=8088 \
    RUST_LOG=info \
    nohup ./target/release/hotpath_arb_engine > logs/recovery_$stream_id.log 2>&1 &

    echo "✅ Recovery attempt completed for stream $stream_id"
}

# Check all streams and recover failed ones
for stream_id in 0 1 2; do
    health=$(curl -s "http://localhost:8088/metrics" | grep "multi_geyser_stream_healthy{stream_id=\"$stream_id\"}" | awk '{print $2}')
    if [[ "$health" == "0" ]]; then
        recover_geyser_stream $stream_id
    fi
done
```

### **Watchdog Service**
```bash
#!/bin/bash
# File: scripts/geyser_watchdog.sh
# Run this as a systemd service or cron job

ALERT_THRESHOLD=120  # seconds
CHECK_INTERVAL=30    # seconds

while true; do
    # Check each stream
    for stream_id in 0 1 2; do
        seconds_since_last=$(curl -s "http://localhost:8088/metrics" | \
            grep "multi_geyser_stream_seconds_since_last_message{stream_id=\"$stream_id\"}" | \
            awk '{print $2}')

        if (( $(echo "$seconds_since_last > $ALERT_THRESHOLD" | bc -l) )); then
            echo "🚨 Stream $stream_id stalled for $seconds_since_last seconds - triggering recovery"
            ./scripts/geyser_recovery.sh
            # Send immediate notification
            curl -X POST YOUR_WEBHOOK_URL -d "{\"alert\":\"Stream $stream_id recovered\"}"
        fi
    done

    sleep $CHECK_INTERVAL
done
```

---

## 📊 **MONITORING DASHBOARD**

### **Grafana Dashboard Panels**
Add these panels to monitor Geyser health:

```json
{
  "title": "🚨 Geyser Stream Health",
  "type": "stat",
  "targets": [{
    "expr": "multi_geyser_stream_healthy",
    "legendFormat": "Stream {{stream_id}}"
  }],
  "fieldConfig": {
    "defaults": {
      "thresholds": {
        "steps": [
          {"color": "red", "value": null},
          {"color": "green", "value": 1}
        ]
      },
      "mappings": [
        {"options": {"0": {"text": "🔴 DOWN"}}, "type": "value"},
        {"options": {"1": {"text": "🟢 HEALTHY"}}, "type": "value"}
      ]
    }
  }
}
```

---

## 🔧 **IMMEDIATE IMPLEMENTATION**

### **Step 1: Add Alert Rules to Prometheus**
```bash
# Create alert rules directory
mkdir -p /opt/MEVana/infrastructure/prometheus/rules

# Add Geyser alert rules
cat > /opt/MEVana/infrastructure/prometheus/rules/geyser_alerts.yml << 'EOF'
groups:
  - name: geyser_critical
    interval: 30s
    rules:
      - alert: GeyserStreamDown
        expr: multi_geyser_stream_healthy == 0
        for: 30s
        labels:
          severity: critical
        annotations:
          summary: "🚨 Geyser stream {{$labels.stream_id}} is DOWN"
EOF
```

### **Step 2: Update Prometheus Config**
Add to `prometheus.yml`:
```yaml
rule_files:
  - "/etc/prometheus/rules/*.yml"

alerting:
  alertmanagers:
    - static_configs:
        - targets:
          - alertmanager:9093
```

### **Step 3: Test Alert System**
```bash
# Simulate stream failure for testing
curl -X POST http://localhost:8088/test/simulate_stream_failure?stream_id=0

# Check if alert fires
curl http://localhost:9090/api/v1/alerts
```

---

## 🎯 **SUCCESS CRITERIA**

✅ **Alert fires within 30 seconds** of stream failure
✅ **Notifications sent immediately** to designated channels
✅ **Recovery attempted automatically** within 2 minutes
✅ **Dashboard shows real-time status** of all streams
✅ **Historical logging** of all failures and recoveries

---

## 📚 **RELATED DOCS**

- **PROMETHEUS_MONITORING_SETUP.md** - Base metrics architecture
- **DOCKER_DEPLOYMENT_ISSUES.md** - Deployment considerations
- **scripts/simple-deploy.sh** - Deployment automation
