#!/usr/bin/env python3
"""
Debug Transaction Data
Examine the actual transaction data structure to understand parsing issues
"""

import redis
import json
from datetime import datetime

def main():
    # Connect to Redis
    r = redis.Redis(host='127.0.0.1', port=6379, decode_responses=True)
    
    print("🔍 Transaction Data Structure Debug")
    print("="*60)
    
    # Get first Orca transaction
    orca_transactions = r.lrange('orca_transactions_list', 0, 0)  # Get first transaction
    
    if orca_transactions:
        transaction = json.loads(orca_transactions[0])
        print(f"\n📊 Transaction Structure:")
        print(f"  Slot: {transaction.get('slot', 'N/A')}")
        print(f"  Signature: {transaction.get('signature', 'N/A')[:20]}...")
        print(f"  Block Time: {datetime.fromtimestamp(transaction.get('block_time', 0)).strftime('%Y-%m-%d %H:%M:%S')}")
        
        # Check the actual transaction data
        if 'transaction' in transaction:
            tx_data = transaction['transaction']
            print(f"\n📄 Transaction Data Keys: {list(tx_data.keys()) if isinstance(tx_data, dict) else 'Not a dict'}")
            
            if 'message' in tx_data:
                message = tx_data['message']
                print(f"\n📨 Message Keys: {list(message.keys()) if isinstance(message, dict) else 'Not a dict'}")
                
                if 'instructions' in message:
                    instructions = message['instructions']
                    print(f"\n📋 Instructions Count: {len(instructions) if isinstance(instructions, list) else 'Not a list'}")
                    
                    if isinstance(instructions, list) and instructions:
                        print(f"\n📋 First Instruction:")
                        print(json.dumps(instructions[0], indent=2))
                        
                        # Check if this is a Whirlpool instruction
                        if 'program_id_index' in instructions[0]:
                            program_id_index = instructions[0]['program_id_index']
                            account_keys = message.get('account_keys', [])
                            if program_id_index < len(account_keys):
                                program_id = account_keys[program_id_index]
                                print(f"\n🔍 Program ID: {program_id}")
                                
                                # Check if this is Whirlpool program
                                whirlpool_program = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc"
                                if program_id == whirlpool_program:
                                    print(f"✅ This is a Whirlpool transaction!")
                                else:
                                    print(f"❌ Not a Whirlpool transaction (expected: {whirlpool_program})")
                
                if 'account_keys' in message:
                    account_keys = message['account_keys']
                    print(f"\n🔑 Account Keys Count: {len(account_keys) if isinstance(account_keys, list) else 'Not a list'}")
                    if isinstance(account_keys, list) and account_keys:
                        print(f"First few account keys: {account_keys[:3]}")
        
        # Check if there's meta data with parsed information
        if 'meta' in transaction:
            meta = transaction['meta']
            print(f"\n📊 Meta Keys: {list(meta.keys()) if isinstance(meta, dict) else 'Not a dict'}")
            
            if 'inner_instructions' in meta:
                inner_instructions = meta['inner_instructions']
                print(f"\n📋 Meta Inner Instructions Count: {len(inner_instructions) if isinstance(inner_instructions, list) else 'Not a list'}")
                if isinstance(inner_instructions, list) and inner_instructions:
                    print(f"First meta inner instruction: {json.dumps(inner_instructions[0], indent=2)}")
        
    else:
        print("❌ No Orca transactions found")

if __name__ == "__main__":
    main() 