#!/usr/bin/env python3
"""
Redis Bridge for Solana Trading Platform
Subscribes to orca.transactions channel and stores data in Redis lists
Also extracts price information for DEX comparisons
"""

import redis
import json
import time
import sys
from datetime import datetime
from typing import Dict, Any, Optional

class RedisBridge:
    def __init__(self, redis_host: str = 'redis', redis_port: int = 6379):
        self.redis_host = redis_host
        self.redis_port = redis_port
        self.r = redis.Redis(host=redis_host, port=redis_port, decode_responses=True)
        self.message_count = 0
        self.price_count = 0
        
    def connect(self) -> bool:
        """Test Redis connection"""
        try:
            self.r.ping()
            print(f"✅ Connected to Redis at {self.redis_host}:{self.redis_port}")
            return True
        except Exception as e:
            print(f"❌ Failed to connect to Redis: {e}")
            return False
    
    def extract_price_info(self, transaction_data: Dict[str, Any]) -> Optional[Dict[str, Any]]:
        """Extract price information from transaction data"""
        try:
            # Look for swap instructions in the parsed data
            instructions = transaction_data.get('parsed_instructions', {})
            
            # Check for Whirlpool swap instructions
            if 'inner_instructions' in instructions:
                for instruction in instructions['inner_instructions']:
                    if instruction.get('name') == 'swap':
                        # Extract token amounts and addresses
                        accounts = instruction.get('accounts', [])
                        data = instruction.get('data', {})
                        
                        # Look for token amounts in the data
                        if 'Swap' in data:
                            swap_data = data['Swap']
                            return {
                                'timestamp': transaction_data.get('block_time'),
                                'slot': transaction_data.get('slot'),
                                'signature': transaction_data.get('signature'),
                                'dex': 'orca',
                                'swap_type': 'whirlpool',
                                'amount_in': swap_data.get('amount_in'),
                                'amount_out': swap_data.get('amount_out'),
                                'token_in': self._extract_token_address(accounts, 'token_in'),
                                'token_out': self._extract_token_address(accounts, 'token_out'),
                                'raw_data': swap_data
                            }
            
            # Check for Jupiter swap instructions
            if 'compiled_instructions' in instructions:
                for instruction in instructions['compiled_instructions']:
                    if instruction.get('name') == 'swap':
                        return {
                            'timestamp': transaction_data.get('block_time'),
                            'slot': transaction_data.get('slot'),
                            'signature': transaction_data.get('signature'),
                            'dex': 'orca',
                            'swap_type': 'jupiter',
                            'raw_data': instruction
                        }
                        
        except Exception as e:
            print(f"⚠️  Error extracting price info: {e}")
        
        return None
    
    def _extract_token_address(self, accounts: list, token_type: str) -> Optional[str]:
        """Extract token address from accounts"""
        for account in accounts:
            if account.get('name') == token_type:
                return account.get('pubkey')
        return None
    
    def store_transaction_data(self, data: str):
        """Store transaction data in Redis list"""
        try:
            # Store in orca_transactions_list (limit to last 1000)
            self.r.lpush('orca_transactions_list', data)
            self.r.ltrim('orca_transactions_list', 0, 999)
            
            # Parse the data to extract price info
            transaction_data = json.loads(data)
            price_info = self.extract_price_info(transaction_data)
            
            if price_info:
                # Store price info in orca_prices_list (limit to last 500)
                price_data = json.dumps(price_info)
                self.r.lpush('orca_prices_list', price_data)
                self.r.ltrim('orca_prices_list', 0, 499)
                self.price_count += 1
                
                # Also store in a time-series format for easy comparison
                timestamp = price_info.get('timestamp', int(time.time()))
                time_key = f"orca_prices:{timestamp}"
                self.r.setex(time_key, 3600, price_data)  # Expire after 1 hour
            
            self.message_count += 1
            
        except Exception as e:
            print(f"❌ Error storing data: {e}")
    
    def run(self):
        """Main bridge loop"""
        print("🔗 Redis Bridge for Solana Trading Platform")
        print("📡 Subscribing to orca.transactions channel...")
        print("💾 Storing data in orca_transactions_list")
        print("💰 Extracting price info to orca_prices_list")
        print("🌐 View data in Redis UI at http://localhost:8083")
        print("⏹️  Press Ctrl+C to stop")
        print("-" * 60)
        
        if not self.connect():
            return
        
        try:
            # Subscribe to the channel
            pubsub = self.r.pubsub()
            pubsub.subscribe('orca.transactions')
            
            print("✅ Subscribed to orca.transactions channel")
            print("⏳ Waiting for messages...")
            print("-" * 60)
            
            for message in pubsub.listen():
                if message['type'] == 'subscribe':
                    print(f"✅ Subscribed to {message['channel']}")
                    continue
                    
                if message['type'] == 'message':
                    timestamp = datetime.now().strftime("%H:%M:%S")
                    
                    try:
                        # Parse the JSON data
                        data = json.loads(message['data'])
                        signature = data.get('signature', 'UNKNOWN')[:20] + '...'
                        slot = data.get('slot', 'UNKNOWN')
                        
                        # Store the data
                        self.store_transaction_data(message['data'])
                        
                        # Print status
                        print(f"[{timestamp}] #{self.message_count:04d} | Slot: {slot} | Sig: {signature} | Prices: {self.price_count}")
                        
                    except json.JSONDecodeError as e:
                        print(f"[{timestamp}] #{self.message_count:04d} | Error parsing JSON: {e}")
                    except Exception as e:
                        print(f"[{timestamp}] #{self.message_count:04d} | Error: {e}")
                        
        except KeyboardInterrupt:
            print("\n🛑 Bridge stopped by user")
        except Exception as e:
            print(f"❌ Error: {e}")
        finally:
            pubsub.close()
            self.r.close()

if __name__ == "__main__":
    bridge = RedisBridge()
    bridge.run() 