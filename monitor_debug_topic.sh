#!/bin/bash
# Monitor Debug Topic for Whirlpool Transactions
# This script monitors the normalizer-debug-full-tx topic and looks for Whirlpool transactions

echo "🔍 Monitoring debug topic for Whirlpool transactions..."
echo "Press Ctrl+C to stop"
echo ""

# Monitor the debug topic and look for Whirlpool transactions
docker exec kafka kafka-console-consumer \
  --bootstrap-server localhost:9092 \
  --topic normalizer-debug-full-tx \
  --from-beginning \
  --max-messages 50 | \
  jq -r 'select(.transaction.message.instructions[].program_id | contains("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc")) | 
    "=== WHIRLPOOL TRANSACTION ===" + "\n" +
    "Signature: " + .signature + "\n" +
    "Slot: " + (.slot | tostring) + "\n" +
    "Timestamp: " + .timestamp + "\n" +
    "Instructions: " + (.transaction.message.instructions | length | tostring) + "\n" +
    "Program IDs: " + (.transaction.message.instructions | map(.program_id) | unique | join(", ")) + "\n" +
    "Log Messages: " + (.transaction.meta.log_messages | join(" | ")) + "\n" +
    "=== END ===" + "\n"'
