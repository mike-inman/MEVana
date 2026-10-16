# Slack Alerting Setup Guide for MEVana

**Last Updated:** September 27, 2025
**Status:** ⚠️ Manual Configuration Required

## Current Issues

The automated Grafana alerting provisioning is not working due to:
1. Docker compose not mounting the alerting directory properly
2. Alert rule format incompatibility between Prometheus and Grafana
3. Environment variable substitution not working for Slack webhook

## Working Slack Webhook

```
https://hooks.slack.com/services/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
```

**Verified working:** This webhook successfully sends messages to #mevana-alerts channel.

Test command:
```bash
curl -X POST https://hooks.slack.com/services/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 \
  -H 'Content-Type: application/json' \
  -d '{"text":"Test alert from MEVana"}'
```

## Manual Setup Instructions

### Step 1: Access Grafana
1. Navigate to: http://149.28.56.125:3000
2. Login: admin / <ADMIN_PASSWORD>

### Step 2: Configure Contact Point
1. Go to **Alerting** → **Contact points**
2. Click **+ Add contact point**
3. Configure:
   - Name: `slack-mevana`
   - Type: `Slack`
   - Webhook URL: `https://hooks.slack.com/services/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`
   - Optional settings:
     - Title: `MEVana Alert: {{ .GroupLabels.alertname }}`
     - Text Body:
     ```
     {{ range .Alerts }}
     Status: {{ .Status }}
     {{ .Annotations.summary }}
     {{ end }}
     ```
4. Click **Test** to verify
5. Click **Save contact point**

### Step 3: Create Alert Rules
1. Go to **Alerting** → **Alert rules**
2. Click **+ Create alert rule**
3. Example: Service Down Alert
   - Name: `Service Down Alert`
   - Query:
     ```
     Query A: up{job="hotpath-arb-engine"} == 0
     ```
   - Condition: WHEN last() OF A IS BELOW 1
   - Evaluation: Every 1m for 2m
   - Labels: `severity=critical`
   - Annotations:
     - Summary: `Service {{ $labels.job }} is down`
   - Contact point: `slack-mevana`

### Step 4: Set Notification Policy
1. Go to **Alerting** → **Notification policies**
2. Edit the default policy
3. Set default contact point to: `slack-mevana`
4. Save

## Recommended Alerts to Create

### 1. Service Health Alerts
```promql
# Service down
up{job=~"hotpath-arb-engine|normalizer-service|executor-service"} == 0

# High memory usage
process_resident_memory_bytes > 2000000000  # 2GB

# High CPU usage
rate(process_cpu_seconds_total[5m]) > 0.8
```

### 2. Trading Performance Alerts
```promql
# No arbitrage opportunities in 5 minutes
rate(mevana_arb_opportunities_total[5m]) == 0

# Low profit margin
mevana_realized_profit_lamports_total < 1000000  # Less than 0.001 SOL

# High slippage
rate(mevana_profit_slippage_lamports_total[5m]) > 10000000
```

### 3. Infrastructure Alerts
```promql
# Kafka lag
kafka_consumer_lag > 1000

# Database connection issues
pg_up == 0

# Redis memory full
redis_memory_used_bytes / redis_memory_max_bytes > 0.9
```

## Files That Need Fixing

### 1. Docker Compose (infrastructure/docker-compose.server.yml)
Add this line to the grafana service volumes:
```yaml
- ./grafana/provisioning/alerting:/etc/grafana/provisioning/alerting:ro
```

### 2. Remove Incompatible Alert Files
These files use Prometheus format, not Grafana format:
- `/opt/MEVana/infrastructure/grafana/provisioning/alerting/mevana-alerts.yml`
- `/opt/MEVana/infrastructure/grafana/provisioning/alerting/mevana-financial-alerts.yml`

Move them to backup:
```bash
ssh root@149.28.56.125
cd /opt/MEVana/infrastructure/grafana/provisioning/alerting/
mv *.yml *.yml.bak
```

### 3. Contact Points File
The file `contact-points.yaml` should have the webhook hardcoded (not use env var):
```yaml
apiVersion: 1
contactPoints:
  - orgId: 1
    name: slack-primary
    receivers:
      - uid: slack-mevana
        type: slack
        settings:
          url: https://hooks.slack.com/services/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
          title: "MEVana Alert"
        disableResolveMessage: false
```

## Testing Alerts

Once configured, you can test by:
1. Stopping a service: `systemctl stop normalizer-service`
2. Wait 2 minutes
3. Check #mevana-alerts Slack channel
4. Restart service: `systemctl start normalizer-service`

## Troubleshooting

### Check Grafana logs:
```bash
docker logs mevana-grafana-1 2>&1 | grep -i "slack\|alert"
```

### Test webhook directly:
```bash
curl -X POST <webhook_url> -H 'Content-Type: application/json' \
  -d '{"text":"Direct test"}'
```

### Verify contact points are loaded:
```bash
curl -u admin:<ADMIN_PASSWORD> http://localhost:3000/api/v1/provisioning/contact-points
```

## Future Improvements

1. Fix docker-compose.server.yml to properly mount alerting directory
2. Convert Prometheus alert rules to Grafana format
3. Use secrets management for Slack webhook URL
4. Create terraform/ansible scripts for automated provisioning
5. Add more sophisticated alert routing based on severity

## Notes

- The webhook URL is currently hardcoded for simplicity
- Grafana's environment variable substitution `${SLACK_WEBHOOK_URL}` doesn't work reliably
- Alert provisioning via YAML files requires exact Grafana format, not Prometheus format
- Manual configuration through UI is more reliable than file provisioning currently