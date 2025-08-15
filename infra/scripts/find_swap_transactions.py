#!/usr/bin/env python3
"""
Find Swap Transactions
Search through transactions to find actual swap transactions with instructions
"""

import redis
import json
from datetime import datetime

def main():
    # Connect to Redis
    r = redis.Redis(host='127.0.0.1', port=6379, decode_responses=True)
    
    print("🔍 Searching for Swap Transactions")
    print("="*60)
    
    # Get multiple Orca transactions
    orca_transactions = r.lrange('orca_transactions_list', 0, 9)  # Get first 10 transactions
    
    if orca_transactions:
        print(f"📊 Analyzing {len(orca_transactions)} transactions...")
        
        swap_found = False
        for i, transaction_json in enumerate(orca_transactions):
            transaction = json.loads(transaction_json)
            
            # Check transaction structure
            if 'transaction' in transaction:
                tx_data = transaction['transaction']
                if 'message' in tx_data:
                    message = tx_data['message']
                    instructions = message.get('instructions', [])
                    
                    if len(instructions) > 0:
                        print(f"\n✅ Transaction {i+1} has {len(instructions)} instructions!")
                        print(f"  Slot: {transaction.get('slot', 'N/A')}")
                        print(f"  Signature: {transaction.get('signature', 'N/A')[:20]}...")
                        
                        # Check if it's a Whirlpool transaction
                        for instruction in instructions:
                            if 'program_id_index' in instruction:
                                program_id_index = instruction['program_id_index']
                                account_keys = message.get('account_keys', [])
                                if program_id_index < len(account_keys):
                                    program_id = account_keys[program_id_index]
                                    
                                    # Check for Whirlpool program
                                    whirlpool_program = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc"
                                    if program_id == whirlpool_program:
                                        print(f"  🔍 Found Whirlpool instruction!")
                                        print(f"  Program ID: {program_id}")
                                        print(f"  Instruction data length: {len(instruction.get('data', []))}")
                                        swap_found = True
                                        break
                        
                        if swap_found:
                            break
                    else:
                        print(f"  Transaction {i+1}: No instructions")
        
        if not swap_found:
            print(f"\n❌ No swap transactions found in the first {len(orca_transactions)} transactions")
            print(f"💡 This suggests the stream-orca service might be capturing non-swap transactions")
            print(f"💡 Or the transactions might be filtered incorrectly")
            
            # Show what types of transactions we're getting
            print(f"\n🔍 Transaction Types Analysis:")
            for i, transaction_json in enumerate(orca_transactions[:3]):
                transaction = json.loads(transaction_json)
                if 'meta' in transaction:
                    meta = transaction['meta']
                    status = meta.get('status', {})
                    print(f"  Transaction {i+1}: Status = {status}")
                    
                    if 'log_messages' in meta:
                        logs = meta['log_messages']
                        if logs:
                            print(f"    First log: {logs[0]}")
    else:
        print("❌ No Orca transactions found")

if __name__ == "__main__":
    main() 