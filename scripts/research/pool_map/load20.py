#!/usr/bin/env python3
"""2026-11-01 CST: Transactional load of H20 without replacing older samples."""
import json,sys
from pathlib import Path
from reachable20 import replay,encoded
root=Path(sys.argv[1]);result=replay(root)
if encoded(result)!=(root/'result.json').read_bytes():raise ValueError('replay mismatch')
def q(v):return "'"+str(v).replace("'","''")+"'"
def data(v):return q(json.dumps(v,sort_keys=True))+'::jsonb'
run=q(result['run_id'])
print('BEGIN;')
print('DELETE FROM research.reachable_gap_runs WHERE run_id='+run+';')
print("INSERT INTO research.reachable_gap_runs VALUES ("+run+",'Modeled',"+data(result)+');')
for r in result['summary']:print('INSERT INTO research.reachable_gap_results VALUES ('+','.join([run,q(r['pool_buy']),q(r['pool_sell']),str(r['size_usd']),"'Modeled'",data(r)])+');')
for key in result['plan']['meteora_keys']:
 labels=[round_['pool_labels'][key] for round_ in result['rounds']]
 print('INSERT INTO research.reachable_gap_pools VALUES ('+','.join([run,q(key),"'OnChain'",data(dict(labels[-1],round_labels=labels))])+');')
print('COMMIT;')
