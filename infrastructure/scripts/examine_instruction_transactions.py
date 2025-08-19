#!/usr/bin/env python3
"""
Examine Instruction Transactions
Look at transactions that have instructions to understand what programs they're calling
"""

import redis
import json
from datetime import datetime

def main():
    # Connect to Redis
    r = redis.Redis(host='127.0.0.1', port=6379, decode_responses=True)
    
    print("🔍 Examining Transactions with Instructions")
    print("="*60)
    
    # Get multiple Orca transactions
    orca_transactions = r.lrange('orca_transactions_list', 0, 19)  # Get first 20 transactions
    
    if orca_transactions:
        print(f"📊 Analyzing {len(orca_transactions)} transactions...")
        
        instruction_count = 0
        for i, transaction_json in enumerate(orca_transactions):
            transaction = json.loads(transaction_json)
            
            # Check transaction structure
            if 'transaction' in transaction:
                tx_data = transaction['transaction']
                if 'message' in tx_data:
                    message = tx_data['message']
                    instructions = message.get('instructions', [])
                    
                    if len(instructions) > 0:
                        instruction_count += 1
                        print(f"\n✅ Transaction {i+1} has {len(instructions)} instructions!")
                        print(f"  Slot: {transaction.get('slot', 'N/A')}")
                        print(f"  Signature: {transaction.get('signature', 'N/A')[:20]}...")
                        
                        # Check each instruction
                        for j, instruction in enumerate(instructions):
                            if 'program_id_index' in instruction:
                                program_id_index = instruction['program_id_index']
                                account_keys = message.get('account_keys', [])
                                if program_id_index < len(account_keys):
                                    program_id = account_keys[program_id_index]
                                    print(f"  Instruction {j+1}: Program ID = {program_id}")
                                    
                                    # Check for known programs
                                    known_programs = {
                                        "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc": "Whirlpool",
                                        "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8": "Raydium AMM",
                                        "CAMMCzo5YL8w4VFF8KVHrK22GGUQp5VpKqKjqKqKqKqKqKqK": "Raydium CLMM",
                                        "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJBk": "Jupiter",
                                        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA": "Token Program",
                                        "ComputeBudget111111111111111111111111111111": "Compute Budget"
                                    }
                                    
                                    if program_id in known_programs:
                                        print(f"    🔍 Known Program: {known_programs[program_id]}")
                                    else:
                                        print(f"    ❓ Unknown Program")
                                    
                                    # Show instruction data length
                                    data_length = len(instruction.get('data', []))
                                    print(f"    📊 Data Length: {data_length} bytes")
                                    
                                    # Show account indices
                                    account_indices = instruction.get('accounts', [])
                                    print(f"    🔑 Account Indices: {account_indices}")
                                    
                                    # Show actual account addresses for first few
                                    if len(account_indices) > 0 and len(account_keys) > 0:
                                        print(f"    🔑 Account Addresses:")
                                        for k, idx in enumerate(account_indices[:3]):  # Show first 3
                                            if idx < len(account_keys):
                                                print(f"      {k+1}: {account_keys[idx]}")
                                        if len(account_indices) > 3:
                                            print(f"      ... and {len(account_indices) - 3} more")
                        
                        # Show meta information
                        if 'meta' in transaction:
                            meta = transaction['meta']
                            status = meta.get('status', {})
                            print(f"  📊 Status: {status}")
                            
                            if 'log_messages' in meta:
                                logs = meta['log_messages']
                                if logs:
                                    print(f"  📝 Logs (first 3):")
                                    for k, log in enumerate(logs[:3]):
                                        print(f"    {k+1}: {log}")
        
        print(f"\n📈 Summary:")
        print(f"  Total transactions analyzed: {len(orca_transactions)}")
        print(f"  Transactions with instructions: {instruction_count}")
        print(f"  Percentage with instructions: {(instruction_count/len(orca_transactions)*100):.1f}%")
        
    else:
        print("❌ No Orca transactions found")

if __name__ == "__main__":
    main() 