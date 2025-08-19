#!/usr/bin/env python3
"""
Redis Price Monitor
Monitors and visualizes price data from Redis for cross-DEX comparison
"""

import redis
import json
import time
import sys
from datetime import datetime
from typing import Dict, List, Any
from collections import defaultdict

class RedisPriceMonitor:
    def __init__(self, redis_host: str = '127.0.0.1', redis_port: int = 6379):
        self.r = redis.Redis(host=redis_host, port=redis_port, decode_responses=True)
        self.price_data = defaultdict(list)
        self.token_pairs = set()
        
    def connect(self) -> bool:
        """Test Redis connection"""
        try:
            self.r.ping()
            print(f"✅ Connected to Redis at {self.r.connection_pool.connection_kwargs['host']}:{self.r.connection_pool.connection_kwargs['port']}")
            return True
        except Exception as e:
            print(f"❌ Failed to connect to Redis: {e}")
            return False
    
    def get_price_lists(self) -> Dict[str, List[Dict]]:
        """Get all price data from Redis lists"""
        price_lists = {}
        
        # Get Orca prices
        orca_prices = self.r.lrange('orca_prices_list', 0, -1)
        if orca_prices:
            price_lists['orca'] = [json.loads(price) for price in orca_prices]
        
        # Get Raydium prices
        raydium_prices = self.r.lrange('raydium_prices_list', 0, -1)
        if raydium_prices:
            price_lists['raydium'] = [json.loads(price) for price in raydium_prices]
        
        # Get cross-DEX comparison data
        cross_dex = self.r.lrange('cross_dex_comparison', 0, -1)
        if cross_dex:
            price_lists['cross_dex'] = [json.loads(comp) for comp in cross_dex]
        
        return price_lists
    
    def get_transaction_lists(self) -> Dict[str, List[Dict]]:
        """Get transaction data from Redis lists"""
        transaction_lists = {}
        
        # Get Orca transactions
        orca_txns = self.r.lrange('orca_transactions_list', 0, -1)
        if orca_txns:
            transaction_lists['orca'] = [json.loads(txn) for txn in orca_txns]
        
        # Get Raydium transactions
        raydium_txns = self.r.lrange('raydium_transactions_list', 0, -1)
        if raydium_txns:
            transaction_lists['raydium'] = [json.loads(txn) for txn in raydium_txns]
        
        return transaction_lists
    
    def analyze_price_data(self, price_lists: Dict[str, List[Dict]]) -> Dict[str, Any]:
        """Analyze price data for insights"""
        analysis = {
            'total_prices': 0,
            'dex_counts': {},
            'token_pairs': set(),
            'price_ranges': {},
            'recent_activity': {}
        }
        
        for dex, prices in price_lists.items():
            if dex == 'cross_dex':
                continue
                
            analysis['dex_counts'][dex] = len(prices)
            analysis['total_prices'] += len(prices)
            
            if prices:
                # Get token pairs
                for price in prices:
                    token_pair = f"{price.get('token_in', '')[:8]}.../{price.get('token_out', '')[:8]}..."
                    analysis['token_pairs'].add(token_pair)
                
                # Get price ranges
                valid_prices = [p.get('price', 0) for p in prices if p.get('price') is not None]
                if valid_prices:
                    analysis['price_ranges'][dex] = {
                        'min': min(valid_prices),
                        'max': max(valid_prices),
                        'avg': sum(valid_prices) / len(valid_prices)
                    }
                
                # Get recent activity (last 10 minutes)
                current_time = int(time.time())
                recent_prices = [p for p in prices if current_time - p.get('timestamp', 0) < 600]
                analysis['recent_activity'][dex] = len(recent_prices)
        
        return analysis
    
    def display_price_comparison(self, price_lists: Dict[str, List[Dict]]):
        """Display cross-DEX price comparison"""
        if 'cross_dex' not in price_lists:
            print("📊 No cross-DEX comparison data available")
            return
        
        cross_dex_data = price_lists['cross_dex']
        if not cross_dex_data:
            print("📊 No cross-DEX comparison data available")
            return
        
        print("\n" + "="*80)
        print("🔄 CROSS-DEX PRICE COMPARISON")
        print("="*80)
        
        # Group by token pair
        token_pair_data = defaultdict(list)
        for data in cross_dex_data:
            token_pair = data.get('token_pair', 'Unknown')
            token_pair_data[token_pair].append(data)
        
        for token_pair, prices in token_pair_data.items():
            print(f"\n💰 Token Pair: {token_pair}")
            print("-" * 60)
            
            # Group by DEX
            dex_prices = defaultdict(list)
            for price in prices:
                dex = price.get('dex', 'Unknown')
                dex_prices[dex].append(price)
            
            for dex, dex_data in dex_prices.items():
                if dex_data:
                    latest = dex_data[0]  # Most recent
                    price_val = latest.get('price', 0)
                    volume = latest.get('volume', 0)
                    timestamp = datetime.fromtimestamp(latest.get('timestamp', 0)).strftime('%H:%M:%S')
                    
                    print(f"  {dex.upper():<10} | Price: {price_val:<12.6f} | Volume: {volume:<12} | Time: {timestamp}")
    
    def display_summary(self, analysis: Dict[str, Any]):
        """Display summary statistics"""
        print("\n" + "="*80)
        print("📈 PRICE DATA SUMMARY")
        print("="*80)
        
        print(f"📊 Total Price Records: {analysis['total_prices']}")
        print(f"🔄 Token Pairs Tracked: {len(analysis['token_pairs'])}")
        
        print("\n📊 DEX Activity:")
        for dex, count in analysis['dex_counts'].items():
            recent = analysis['recent_activity'].get(dex, 0)
            print(f"  {dex.upper():<10} | Total: {count:<6} | Recent (10min): {recent}")
        
        print("\n💰 Price Ranges:")
        for dex, ranges in analysis['price_ranges'].items():
            print(f"  {dex.upper():<10} | Min: {ranges['min']:<10.6f} | Max: {ranges['max']:<10.6f} | Avg: {ranges['avg']:<10.6f}")
        
        if analysis['token_pairs']:
            print(f"\n🪙 Token Pairs: {', '.join(list(analysis['token_pairs'])[:5])}")
            if len(analysis['token_pairs']) > 5:
                print(f"   ... and {len(analysis['token_pairs']) - 5} more")
    
    def monitor_live(self, interval: int = 5):
        """Monitor Redis data in real-time"""
        print("🔍 Starting live Redis price monitoring...")
        print("⏹️  Press Ctrl+C to stop")
        print("-" * 60)
        
        last_counts = {}
        
        try:
            while True:
                # Get current data
                price_lists = self.get_price_lists()
                transaction_lists = self.get_transaction_lists()
                
                # Analyze data
                analysis = self.analyze_price_data(price_lists)
                
                # Check for new data
                current_time = datetime.now().strftime("%H:%M:%S")
                has_new_data = False
                
                for dex, count in analysis['dex_counts'].items():
                    if dex not in last_counts or count > last_counts[dex]:
                        has_new_data = True
                        last_counts[dex] = count
                
                if has_new_data:
                    print(f"\n[{current_time}] 🔄 New data detected!")
                    self.display_summary(analysis)
                    self.display_price_comparison(price_lists)
                else:
                    print(f"[{current_time}] ⏳ No new data...", end='\r')
                
                time.sleep(interval)
                
        except KeyboardInterrupt:
            print("\n\n🛑 Monitoring stopped by user")
    
    def show_current_state(self):
        """Show current state of Redis data"""
        print("🔍 Current Redis Data State")
        print("="*60)
        
        # Get all data
        price_lists = self.get_price_lists()
        transaction_lists = self.get_transaction_lists()
        
        # Show transaction counts
        print("\n📥 Transaction Data:")
        for dex, txns in transaction_lists.items():
            print(f"  {dex.upper():<10} | Transactions: {len(txns)}")
        
        # Show price data
        print("\n💰 Price Data:")
        for dex, prices in price_lists.items():
            if dex != 'cross_dex':
                print(f"  {dex.upper():<10} | Price Records: {len(prices)}")
        
        if 'cross_dex' in price_lists:
            print(f"  CROSS-DEX   | Comparisons: {len(price_lists['cross_dex'])}")
        
        # Analyze and display
        if any(len(prices) > 0 for prices in price_lists.values()):
            analysis = self.analyze_price_data(price_lists)
            self.display_summary(analysis)
            self.display_price_comparison(price_lists)
        else:
            print("\n❌ No price data found in Redis")
            print("💡 Make sure the price parser service is running and processing data")

def main():
    import argparse
    
    parser = argparse.ArgumentParser(description='Monitor Redis price data')
    parser.add_argument('--host', default='127.0.0.1', help='Redis host')
    parser.add_argument('--port', type=int, default=6379, help='Redis port')
    parser.add_argument('--live', action='store_true', help='Monitor live data')
    parser.add_argument('--interval', type=int, default=5, help='Monitoring interval in seconds')
    
    args = parser.parse_args()
    
    monitor = RedisPriceMonitor(args.host, args.port)
    
    if not monitor.connect():
        sys.exit(1)
    
    if args.live:
        monitor.monitor_live(args.interval)
    else:
        monitor.show_current_state()

if __name__ == "__main__":
    main() 