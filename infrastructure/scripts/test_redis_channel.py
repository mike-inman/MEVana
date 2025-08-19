#!/usr/bin/env python3
"""
Test Redis Channel Subscription
Simple test to verify if we can receive messages from orca.transactions channel
"""

import redis
import json
import time
from datetime import datetime

def main():
    print("🧪 Testing Redis Channel Subscription")
    print("📡 Subscribing to orca.transactions channel...")
    print("-" * 50)
    
    # Connect to Redis
    r = redis.Redis(host='127.0.0.1', port=6379, decode_responses=True)
    
    try:
        # Test connection
        print(f"✅ Redis connection: {r.ping()}")
        
        # Subscribe to the channel
        pubsub = r.pubsub()
        pubsub.subscribe('orca.transactions')
        
        print("⏳ Waiting for messages (30 seconds)...")
        start_time = time.time()
        message_count = 0
        
        for message in pubsub.listen():
            current_time = time.time()
            if current_time - start_time > 30:  # 30 second timeout
                break
                
            if message['type'] == 'subscribe':
                print(f"✅ Subscribed to {message['channel']}")
                continue
                
            if message['type'] == 'message':
                message_count += 1
                timestamp = datetime.now().strftime("%H:%M:%S")
                print(f"[{timestamp}] Message #{message_count} received!")
                print(f"Data length: {len(message['data'])} characters")
                print(f"First 100 chars: {message['data'][:100]}...")
                print("-" * 30)
                
                # Try to parse JSON
                try:
                    data = json.loads(message['data'])
                    print(f"✅ JSON parsed successfully")
                    print(f"Signature: {data.get('signature', 'N/A')[:20]}...")
                    print(f"Slot: {data.get('slot', 'N/A')}")
                except json.JSONDecodeError as e:
                    print(f"❌ JSON parse error: {e}")
                
                print("-" * 30)
        
        print(f"\n📊 Summary: Received {message_count} messages in 30 seconds")
        
    except Exception as e:
        print(f"❌ Error: {e}")
    finally:
        pubsub.close()
        r.close()

if __name__ == "__main__":
    main() 