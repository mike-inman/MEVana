#!/usr/bin/env python3
"""
Quick API server to serve arbitrage data from PostgreSQL
"""
import json
import psycopg2
from http.server import HTTPServer, BaseHTTPRequestHandler
from urllib.parse import urlparse, parse_qs
from datetime import datetime

DB_CONFIG = {
    'host': 'localhost',
    'port': 5433,
    'database': 'mevdb',
    'user': 'mev',
    'password': '<POSTGRES_PASSWORD>'
}

class ArbitrageAPIHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        parsed_path = urlparse(self.path)
        
        if parsed_path.path == '/api/arbitrage/all':
            self.handle_all_records(parsed_path)
        elif parsed_path.path == '/api/arbitrage/summary':
            self.handle_summary()
        elif parsed_path.path == '/health':
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.end_headers()
            self.wfile.write(json.dumps({'status': 'healthy'}).encode())
        else:
            self.send_response(404)
            self.end_headers()
            
    def handle_all_records(self, parsed_path):
        params = parse_qs(parsed_path.query)
        limit = int(params.get('limit', ['1000'])[0])
        limit = min(limit, 10000)  # Max 10000 records
        
        try:
            conn = psycopg2.connect(**DB_CONFIG)
            cur = conn.cursor()
            
            cur.execute("""
                SELECT 
                    opportunity_id,
                    timestamp,
                    slot,
                    venue_sell,
                    venue_buy,
                    token_pair,
                    profit_bps,
                    amount_in,
                    expected_amount_out,
                    detection_time_us,
                    strategy_type
                FROM arbitrage_opportunities 
                ORDER BY timestamp DESC
                LIMIT %s
            """, (limit,))
            
            rows = cur.fetchall()
            records = []
            
            for row in rows:
                records.append({
                    'opportunity_id': row[0],
                    'timestamp': row[1].isoformat() if row[1] else None,
                    'slot': row[2],
                    'venue_sell': row[3],
                    'venue_buy': row[4],
                    'token_pair': row[5],
                    'profit_bps': row[6],
                    'amount_in': float(row[7]) if row[7] else 0,
                    'expected_amount_out': float(row[8]) if row[8] else 0,
                    'detection_time_us': row[9],
                    'strategy_type': row[10]
                })
            
            response = {
                'total': len(records),
                'limit': limit,
                'records': records
            }
            
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Access-Control-Allow-Origin', '*')
            self.end_headers()
            self.wfile.write(json.dumps(response, indent=2).encode())
            
            cur.close()
            conn.close()
            
        except Exception as e:
            self.send_response(500)
            self.send_header('Content-Type', 'application/json')
            self.end_headers()
            self.wfile.write(json.dumps({'error': str(e)}).encode())
    
    def handle_summary(self):
        try:
            conn = psycopg2.connect(**DB_CONFIG)
            cur = conn.cursor()
            
            cur.execute("""
                SELECT 
                    COUNT(*) as total_opportunities,
                    COUNT(DISTINCT venue_sell || '-' || venue_buy) as unique_venue_pairs,
                    AVG(profit_bps) as avg_profit_bps,
                    MAX(profit_bps) as max_profit_bps,
                    MIN(profit_bps) as min_profit_bps,
                    SUM(profit_bps) as total_profit_bps
                FROM arbitrage_opportunities
            """)
            
            row = cur.fetchone()
            
            summary = {
                'total_opportunities': row[0],
                'unique_venue_pairs': row[1],
                'avg_profit_bps': float(row[2]) if row[2] else 0,
                'max_profit_bps': row[3],
                'min_profit_bps': row[4],
                'total_profit_bps': row[5]
            }
            
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Access-Control-Allow-Origin', '*')
            self.end_headers()
            self.wfile.write(json.dumps(summary, indent=2).encode())
            
            cur.close()
            conn.close()
            
        except Exception as e:
            self.send_response(500)
            self.send_header('Content-Type', 'application/json')
            self.end_headers()
            self.wfile.write(json.dumps({'error': str(e)}).encode())

if __name__ == '__main__':
    server = HTTPServer(('0.0.0.0', 8090), ArbitrageAPIHandler)
    print('Starting API server on port 8090...')
    print('Available endpoints:')
    print('  http://149.28.56.125:8090/api/arbitrage/all?limit=1000')
    print('  http://149.28.56.125:8090/api/arbitrage/summary')
    print('  http://149.28.56.125:8090/health')
    server.serve_forever()