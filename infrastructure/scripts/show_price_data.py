#!/usr/bin/env python3
"""
Show Price Data Structure
Display the actual price data being stored in Redis
"""

import redis
import json
from datetime import datetime

def main():
    # Connect to Redis
    r = redis.Redis(host='127.0.0.1', port=6379, decode_responses=True)
    
    print("🔍 Redis Price Data Structure Analysis")
    print("="*60)
    
    # Get Orca price data
    orca_prices = r.lrange('orca_prices_list', 0, 2)  # Get first 3 records
    
    if orca_prices:
        print(f"\n💰 Orca Price Data (showing first 3 of {len(r.lrange('orca_prices_list', 0, -1))} records):")
        print("-" * 60)
        
        for i, price_json in enumerate(orca_prices):
            price_data = json.loads(price_json)
            print(f"\n📊 Record {i+1}:")
            print(f"  ID: {price_data.get('id', 'N/A')}")
            print(f"  Timestamp: {datetime.fromtimestamp(price_data.get('timestamp', 0)).strftime('%Y-%m-%d %H:%M:%S')}")
            print(f"  Slot: {price_data.get('slot', 'N/A')}")
            print(f"  Signature: {price_data.get('signature', 'N/A')[:20]}...")
            print(f"  DEX: {price_data.get('dex', 'N/A')}")
            print(f"  Token In: {price_data.get('token_in', 'N/A')}")
            print(f"  Token Out: {price_data.get('token_out', 'N/A')}")
            print(f"  Amount In: {price_data.get('amount_in', 'N/A')}")
            print(f"  Amount Out: {price_data.get('amount_out', 'N/A')}")
            print(f"  Price: {price_data.get('price', 'N/A')}")
            print(f"  Pool Address: {price_data.get('pool_address', 'N/A')}")
    else:
        print("❌ No Orca price data found")
    
    # Get cross-DEX comparison data
    cross_dex = r.lrange('cross_dex_comparison', 0, 2)  # Get first 3 records
    
    if cross_dex:
        print(f"\n🔄 Cross-DEX Comparison Data (showing first 3 of {len(r.lrange('cross_dex_comparison', 0, -1))} records):")
        print("-" * 60)
        
        for i, comp_json in enumerate(cross_dex):
            comp_data = json.loads(comp_json)
            print(f"\n📊 Comparison {i+1}:")
            print(f"  Timestamp: {datetime.fromtimestamp(comp_data.get('timestamp', 0)).strftime('%Y-%m-%d %H:%M:%S')}")
            print(f"  DEX: {comp_data.get('dex', 'N/A')}")
            print(f"  Token Pair: {comp_data.get('token_pair', 'N/A')}")
            print(f"  Price: {comp_data.get('price', 'N/A')}")
            print(f"  Volume: {comp_data.get('volume', 'N/A')}")
            print(f"  Signature: {comp_data.get('signature', 'N/A')[:20]}...")
    else:
        print("\n❌ No cross-DEX comparison data found")
    
    # Show all Redis keys related to prices
    print(f"\n🔑 All Redis Keys:")
    print("-" * 60)
    
    all_keys = r.keys("*price*")
    all_keys.extend(r.keys("*comparison*"))
    all_keys.extend(r.keys("*transaction*"))
    
    for key in sorted(all_keys):
        key_type = r.type(key)
        if key_type == 'list':
            length = r.llen(key)
            print(f"  {key} (list, {length} items)")
        elif key_type == 'string':
            print(f"  {key} (string)")
        else:
            print(f"  {key} ({key_type})")

if __name__ == "__main__":
    main() 