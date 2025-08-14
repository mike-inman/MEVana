#!/usr/bin/env python3
"""
Test data producer for arbitrage engine
Produces mock swap events to Kafka topics
"""

import json
import time
import random
from kafka import KafkaProducer
from datetime import datetime

def create_producer():
    """Create Kafka producer"""
    return KafkaProducer(
        bootstrap_servers=['localhost:9092'],
        value_serializer=lambda v: json.dumps(v).encode('utf-8'),
        key_serializer=lambda k: k.encode('utf-8') if k else None
    )

def generate_swap_event(dex, base_price, variation=0.02, reverse=False):
    """Generate a mock swap event"""
    # Add some price variation
    price = base_price * (1 + random.uniform(-variation, variation))
    
    if reverse:
        # Reverse swap: SOL -> USDC
        amount_in = 4.0  # 4 SOL
        amount_out = amount_in * price
        return {
            "dex": dex,
            "token_in": "SOL",
            "token_out": "USDC", 
            "amount_in": amount_in,
            "amount_out": amount_out
        }
    else:
        # Standard swap: 100 USDC -> SOL
        amount_in = 100.0
        amount_out = amount_in / price
        return {
            "dex": dex,
            "token_in": "USDC",
            "token_out": "SOL", 
            "amount_in": amount_in,
            "amount_out": amount_out
        }

def main():
    print("🚀 Starting test data producer...")
    print("📡 Producing swap events to raydium-swaps and orca-swaps topics")
    print("💰 Base SOL price: ~$25.00 with 2% variation")
    print("⏰ Producing events every 2 seconds...")
    print("Press Ctrl+C to stop")
    print()
    
    producer = create_producer()
    
    # Base prices for different DEXes
    base_prices = {
        "Raydium": 25.0,
        "Orca": 25.1  # Slightly different to create arbitrage opportunities
    }
    
    try:
        while True:
            timestamp = datetime.now().isoformat()
            
            # Generate swap events for each DEX
            for dex, base_price in base_prices.items():
                # Generate both directions to create arbitrage opportunities
                swap_event = generate_swap_event(dex, base_price, reverse=False)
                topic = f"{dex.lower()}-swaps"
                
                # Send to Kafka
                future = producer.send(topic, value=swap_event)
                future.get(timeout=10)  # Wait for send to complete
                
                print(f"[{timestamp}] 📊 {dex}: {swap_event['amount_in']} {swap_event['token_in']} -> {swap_event['amount_out']:.4f} {swap_event['token_out']} (price: ${base_price:.2f})")
                
                # Generate reverse swap for arbitrage
                reverse_swap = generate_swap_event(dex, base_price, reverse=True)
                future = producer.send(topic, value=reverse_swap)
                future.get(timeout=10)
                
                print(f"[{timestamp}] 📊 {dex}: {reverse_swap['amount_in']} {reverse_swap['token_in']} -> {reverse_swap['amount_out']:.4f} {reverse_swap['token_out']} (price: ${base_price:.2f})")
            
            print("-" * 80)
            time.sleep(2)
            
    except KeyboardInterrupt:
        print("\n🛑 Stopping test data producer...")
    finally:
        producer.close()
        print("✅ Test data producer stopped")

if __name__ == "__main__":
    main() 