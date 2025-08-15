#!/usr/bin/env python3
import redis
import json

# Connect to Redis
r = redis.Redis(host='localhost', port=6379, decode_responses=True)

# Get recent transactions from orca_prices_list
transactions = r.lrange('orca_prices_list', 0, 9)  # Get last 10 transactions

print(f"Found {len(transactions)} transactions to publish")

# Publish each transaction to orca.transactions channel
for i, tx_json in enumerate(transactions):
    try:
        # Parse to validate JSON
        tx = json.loads(tx_json)
        # Publish to channel
        result = r.publish('orca.transactions', tx_json)
        print(f"Published transaction {i+1}: {tx.get('signature', 'unknown')[:8]}... (subscribers: {result})")
    except Exception as e:
        print(f"Error publishing transaction {i+1}: {e}")

print("Done publishing transactions") 