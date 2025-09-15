#!/usr/bin/env python3
"""
MEVana Telegram Alert Webhook
Receives Prometheus alerts and sends them to Telegram

Setup:
1. Create Telegram bot with @BotFather
2. Get bot token and chat ID
3. Set environment variables:
   export TELEGRAM_BOT_TOKEN="your_bot_token"
   export TELEGRAM_CHAT_ID="your_chat_id"
4. Run: python3 telegram_webhook.py

For Prometheus Alertmanager integration, add to alertmanager.yml:
  receivers:
  - name: 'telegram-alerts'
    webhook_configs:
    - url: 'http://localhost:5001/webhook'
"""

import requests
import json
import os
from datetime import datetime
from flask import Flask, request

app = Flask(__name__)

# Configuration
TELEGRAM_BOT_TOKEN = os.getenv('TELEGRAM_BOT_TOKEN')
TELEGRAM_CHAT_ID = os.getenv('TELEGRAM_CHAT_ID')

if not TELEGRAM_BOT_TOKEN or not TELEGRAM_CHAT_ID:
    print("ERROR: Please set TELEGRAM_BOT_TOKEN and TELEGRAM_CHAT_ID environment variables")
    print("Example:")
    print("  export TELEGRAM_BOT_TOKEN='123456789:ABCdefGhIJklmnoPQRstUVwXYz'")
    print("  export TELEGRAM_CHAT_ID='-1001234567890'")
    exit(1)

def send_telegram_message(message):
    """Send message to Telegram chat"""
    url = f"https://api.telegram.org/bot{TELEGRAM_BOT_TOKEN}/sendMessage"

    data = {
        "chat_id": TELEGRAM_CHAT_ID,
        "text": message,
        "parse_mode": "HTML",
        "disable_web_page_preview": True
    }

    try:
        response = requests.post(url, data=data, timeout=10)
        if response.status_code == 200:
            print(f"✅ Telegram message sent successfully")
        else:
            print(f"❌ Failed to send Telegram message: {response.status_code}")
            print(response.text)
    except requests.exceptions.RequestException as e:
        print(f"❌ Error sending Telegram message: {e}")

def format_alert_message(alert):
    """Format Prometheus alert for Telegram"""
    status = alert.get('status', 'unknown')
    labels = alert.get('labels', {})
    annotations = alert.get('annotations', {})

    # Extract key information
    alertname = labels.get('alertname', 'Unknown Alert')
    severity = labels.get('severity', 'unknown')
    component = labels.get('component', 'unknown')
    stream_id = labels.get('stream_id', 'N/A')

    summary = annotations.get('summary', 'No summary available')
    description = annotations.get('description', 'No description available')

    # Choose emoji based on severity
    severity_emojis = {
        'critical': '🚨',
        'high': '⚠️',
        'medium': '📢',
        'low': 'ℹ️',
        'unknown': '❓'
    }

    emoji = severity_emojis.get(severity.lower(), '📢')

    # Format timestamp
    starts_at = alert.get('startsAt')
    if starts_at:
        try:
            timestamp = datetime.fromisoformat(starts_at.replace('Z', '+00:00'))
            time_str = timestamp.strftime('%Y-%m-%d %H:%M:%S UTC')
        except:
            time_str = starts_at
    else:
        time_str = 'Unknown'

    if status == 'firing':
        message = f"{emoji} <b>🚨 MEVana ALERT FIRING</b>\n\n"
        message += f"<b>Alert:</b> {alertname}\n"
        message += f"<b>Severity:</b> {severity.upper()}\n"
        message += f"<b>Component:</b> {component}\n"

        if stream_id != 'N/A':
            message += f"<b>Stream ID:</b> {stream_id}\n"

        message += f"<b>Time:</b> {time_str}\n\n"
        message += f"<b>Summary:</b>\n{summary}\n\n"
        message += f"<b>Description:</b>\n{description}"

        # Add action recommendations for critical alerts
        if severity.lower() == 'critical':
            message += f"\n\n<b>🔧 IMMEDIATE ACTIONS REQUIRED:</b>"
            message += f"\n• Check system status immediately"
            message += f"\n• Review logs: tail -f /opt/MEVana/logs/*.log"
            message += f"\n• Run recovery: ./scripts/geyser_recovery.sh"

            if 'geyser' in alertname.lower():
                message += f"\n• Verify Geyser endpoint connectivity"
                message += f"\n• Check MEV detection pipeline"

    else:  # resolved
        message = f"✅ <b>🟢 MEVana ALERT RESOLVED</b>\n\n"
        message += f"<b>Alert:</b> {alertname}\n"
        message += f"<b>Component:</b> {component}\n"

        if stream_id != 'N/A':
            message += f"<b>Stream ID:</b> {stream_id}\n"

        message += f"<b>Resolved at:</b> {time_str}\n\n"
        message += f"<b>Summary:</b> {summary}"

    return message

@app.route('/webhook', methods=['POST'])
def webhook():
    """Handle incoming Prometheus alert webhooks"""
    try:
        data = request.get_json()

        if not data:
            return "No JSON data received", 400

        alerts = data.get('alerts', [])

        if not alerts:
            return "No alerts in payload", 400

        print(f"📨 Received {len(alerts)} alert(s)")

        # Process each alert
        for alert in alerts:
            message = format_alert_message(alert)
            send_telegram_message(message)

            # Log alert for debugging
            alertname = alert.get('labels', {}).get('alertname', 'Unknown')
            status = alert.get('status', 'unknown')
            print(f"  📋 Processed: {alertname} ({status})")

        return "OK", 200

    except Exception as e:
        error_msg = f"❌ Error processing webhook: {e}"
        print(error_msg)

        # Send error notification
        try:
            send_telegram_message(f"🔧 <b>Webhook Error</b>\n\n{error_msg}")
        except:
            pass

        return "Internal Server Error", 500

@app.route('/test', methods=['GET'])
def test():
    """Test endpoint to verify Telegram integration"""
    test_message = f"""
🧪 <b>MEVana Telegram Bot Test</b>

<b>Time:</b> {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}
<b>Status:</b> ✅ Connected and working!

This is a test message to verify the Telegram integration is working correctly.
"""

    send_telegram_message(test_message)
    return "Test message sent to Telegram", 200

@app.route('/health', methods=['GET'])
def health():
    """Health check endpoint"""
    return {
        "status": "healthy",
        "service": "MEVana Telegram Webhook",
        "timestamp": datetime.now().isoformat(),
        "telegram_configured": bool(TELEGRAM_BOT_TOKEN and TELEGRAM_CHAT_ID)
    }, 200

if __name__ == '__main__':
    print("🤖 MEVana Telegram Alert Webhook Starting...")
    print(f"📱 Telegram Bot Token: {'✅ Configured' if TELEGRAM_BOT_TOKEN else '❌ Missing'}")
    print(f"💬 Telegram Chat ID: {'✅ Configured' if TELEGRAM_CHAT_ID else '❌ Missing'}")
    print("")
    print("🔗 Endpoints:")
    print("  • Webhook: POST /webhook (for Prometheus)")
    print("  • Test: GET /test (send test message)")
    print("  • Health: GET /health (status check)")
    print("")
    print("🚀 Starting webhook server on http://0.0.0.0:5001")

    app.run(host='0.0.0.0', port=5001, debug=False)