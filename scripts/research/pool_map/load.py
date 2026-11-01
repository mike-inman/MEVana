#!/usr/bin/env python3
"""2026-10-31: Emit a repeatable single transaction from saved pool-map results."""
import json,sys
from pathlib import Path
root=Path(sys.argv[1]);manifest=json.loads((root/'manifest.json').read_text());bundle=json.loads((root/'bundle.json').read_text());result=json.loads((root/'replay.json').read_text())
def q(v):return "'"+str(v).replace("'","''")+"'"
def row(table,key,value):
 cols='run_id,evidence_kind,data';values=q(manifest['run_id'])+",'Reported',"+q(json.dumps(value,sort_keys=True))+'::jsonb'
 if key:cols+=','+key[0];values+=','+q(key[1])
 print(f'INSERT INTO research.{table} ({cols}) VALUES ({values});')
print('BEGIN;');print('DELETE FROM research.pool_map_runs WHERE run_id='+q(manifest['run_id'])+';')
row('pool_map_runs',None,manifest)
new={p['address'] for p in result['new_pools']}
for p in bundle['pools']:
 row('pool_map_pools',('pool_id',p['address']),dict(p,new_pool=p['address'] in new,samples=[{'epoch':t,'price':sample.get(p['address']),'evidence_kind':'Reported'} for t,sample in zip(bundle['sample_times'],bundle['samples'])]))
for p in result['pairs']:
 row('pair_requirements',('pair_id',p['pair']),p)
 if not p['failed_step']:row('pair_clues',('pair_id',p['pair']),p)
print('COMMIT;')
