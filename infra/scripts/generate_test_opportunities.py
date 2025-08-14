#!/usr/bin/env python3
"""
Generate Test Arbitrage Opportunities
Creates mock arbitrage opportunities for testing validation logic
"""

import json
import time
import random
from datetime import datetime, timedelta
from typing import List, Dict

def generate_mock_opportunities() -> List[Dict]:
    """Generate realistic mock arbitrage opportunities"""
    
    # Test token pairs
    token_pairs = [
        "SOL/USDC",
        "SOL/USDT", 
        "RAY/USDC",
        "SRM/USDC",
        "ORCA/USDC",
        "MNGO/USDC",
        "BONK/USDC",
        "JUP/USDC"
    ]
    
    # DEX pairs
    dex_pairs = [
        ("Raydium", "Orca"),
        ("Orca", "Raydium"),
        ("Raydium", "Serum"),
        ("Serum", "Orca"),
        ("Jupiter", "Raydium"),
        ("Orca", "Jupiter")
    ]
    
    opportunities = []
    
    for i in range(20):  # Generate 20 opportunities
        # Random token pair
        token_pair = random.choice(token_pairs)
        base_token, quote_token = token_pair.split("/")
        
        # Random DEX pair
        dex1, dex2 = random.choice(dex_pairs)
        
        # Generate realistic prices
        base_price = random.uniform(0.1, 1000.0)  # Realistic token prices
        
        # Create price difference for arbitrage
        price_diff_pct = random.uniform(0.1, 2.0)  # 0.1% to 2% difference
        dex1_price = base_price
        dex2_price = base_price * (1 + price_diff_pct / 100)
        
        # Calculate profit percentage
        profit_pct = price_diff_pct - 0.5  # Subtract fees
        
        # Only include profitable opportunities
        if profit_pct > 0.05:  # Minimum 0.05% profit
            opportunity = {
                "id": f"opp_{i+1:03d}",
                "token_pair": token_pair,
                "base_token": base_token,
                "quote_token": quote_token,
                "dex1": dex1,
                "dex2": dex2,
                "dex1_price": round(dex1_price, 6),
                "dex2_price": round(dex2_price, 6),
                "profit_percentage": round(profit_pct, 4),
                "profit_usd": round(profit_pct * base_price / 100, 6),
                "volume_24h": random.uniform(1000, 1000000),
                "liquidity": random.uniform(10000, 10000000),
                "confidence_score": random.uniform(0.7, 0.95),
                "created_at": (datetime.now() - timedelta(minutes=random.randint(1, 60))).isoformat(),
                "status": "active",
                "execution_time_ms": random.randint(50, 500),
                "gas_estimate": random.randint(5000, 15000),
                "slippage_estimate": random.uniform(0.1, 1.0)
            }
            opportunities.append(opportunity)
    
    # Sort by profit percentage (highest first)
    opportunities.sort(key=lambda x: x["profit_percentage"], reverse=True)
    
    return opportunities

def save_opportunities_to_file(opportunities: List[Dict], filename: str = "test_opportunities.json"):
    """Save opportunities to JSON file"""
    with open(filename, 'w') as f:
        json.dump({
            "opportunities": opportunities,
            "generated_at": datetime.now().isoformat(),
            "count": len(opportunities)
        }, f, indent=2)
    
    print(f"✅ Saved {len(opportunities)} opportunities to {filename}")

def create_mock_api_response(opportunities: List[Dict]) -> Dict:
    """Create a mock API response format"""
    return {
        "success": True,
        "opportunities": opportunities,
        "timestamp": datetime.now().isoformat(),
        "total_count": len(opportunities),
        "active_count": len([o for o in opportunities if o["status"] == "active"]),
        "avg_profit": sum(o["profit_percentage"] for o in opportunities) / len(opportunities) if opportunities else 0
    }

def main():
    """Main function to generate test opportunities"""
    print("🧪 Generating mock arbitrage opportunities for testing...")
    
    # Generate opportunities
    opportunities = generate_mock_opportunities()
    
    # Save to file
    save_opportunities_to_file(opportunities)
    
    # Create API response
    api_response = create_mock_api_response(opportunities)
    
    # Save API response format
    with open("test_api_response.json", 'w') as f:
        json.dump(api_response, f, indent=2)
    
    print(f"✅ Generated {len(opportunities)} opportunities")
    print(f"📊 Average profit: {api_response['avg_profit']:.4f}%")
    print(f"📈 Active opportunities: {api_response['active_count']}")
    
    # Show top 5 opportunities
    print("\n🏆 Top 5 Opportunities:")
    for i, opp in enumerate(opportunities[:5]):
        print(f"  {i+1}. {opp['token_pair']} ({opp['dex1']} → {opp['dex2']})")
        print(f"     Profit: {opp['profit_percentage']:.4f}% | Price: ${opp['dex1_price']:.6f} → ${opp['dex2_price']:.6f}")
        print(f"     Confidence: {opp['confidence_score']:.2f} | Gas: {opp['gas_estimate']} lamports")
        print()

if __name__ == "__main__":
    main() 