#!/usr/bin/env python3
"""
Redis Channel Bridge
Subscribes to orca.transactions channel and stores data in orca_transactions_list
so it can be viewed in Redis UI.
"""

import redis
import json
import time
import sys
from datetime import datetime

def main():
    # Connect to Redis
    r = redis.Redis(host='127.0.0.1', port=6379, decode_responses=True)
    
    print("🔗 Redis Channel Bridge Started")
    print("📡 Subscribing to orca.transactions channel...")
    print("💾 Storing data in orca_transactions_list")
    print("🌐 View data in Redis UI at http://localhost:8083")
    print("⏹️  Press Ctrl+C to stop")
    print("-" * 50)
    
    try:
        # Test Redis connection
        print(f"✅ Redis connection test: {r.ping()}")
        
        # Subscribe to the channel
        pubsub = r.pubsub()
        pubsub.subscribe('orca.transactions')
        
        # Keep track of message count
        message_count = 0
        
        for message in pubsub.listen():
            if message['type'] == 'subscribe':
                print(f"✅ Subscribed to {message['channel']}")
                continue
                
            if message['type'] == 'message':
                message_count += 1
                timestamp = datetime.now().strftime("%H:%M:%S")
                
                try:
                    # Parse the JSON data
                    data = json.loads(message['data'])
                    signature = data.get('signature', 'UNKNOWN')[:20] + '...'
                    slot = data.get('slot', 'UNKNOWN')
                    
                    # Store in Redis list (limit to last 100 messages)
                    result = r.lpush('orca_transactions_list', message['data'])
                    r.ltrim('orca_transactions_list', 0, 99)  # Keep only last 100
                    
                    # Verify the data was stored
                    list_length = r.llen('orca_transactions_list')
                    
                    print(f"[{timestamp}] #{message_count:04d} | Slot: {slot} | Sig: {signature} | List length: {list_length}")
                    
                    # Debug: show first few items in list every 10 messages
                    if message_count % 10 == 0:
                        first_item = r.lindex('orca_transactions_list', 0)
                        if first_item:
                            first_data = json.loads(first_item)
                            first_sig = first_data.get('signature', 'N/A')[:20] + '...'
                            print(f"    📋 First item in list: {first_sig}")
                    
                except json.JSONDecodeError as e:
                    print(f"[{timestamp}] #{message_count:04d} | Error parsing JSON: {e}")
                except Exception as e:
                    print(f"[{timestamp}] #{message_count:04d} | Error: {e}")
                    import traceback
                    traceback.print_exc()
                    
    except KeyboardInterrupt:
        print("\n🛑 Bridge stopped by user")
    except Exception as e:
        print(f"❌ Error: {e}")
        import traceback
        traceback.print_exc()
    finally:
        pubsub.close()
        r.close()

if __name__ == "__main__":
    main() 