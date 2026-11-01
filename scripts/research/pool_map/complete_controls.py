"""2026-11-01: Finish missing control-token coverage within the same run's ledger and deadline."""
from pathlib import Path
import sys,json,time
from sample import Source,dump
from collect19a import Limits
root=Path(sys.argv[1]);manifest=json.load(open(root/'manifest.json'));meta=json.load(open(root/'metadata.json'))
b=Limits();b.start=time.monotonic()-(time.time()-manifest['started_epoch']);b.public=manifest['public_requests'];b.rpc=manifest['rpc_calls'];source=Source(root,b);source.n=max(int(p.stem.split('-')[1]) for p in root.glob('http-*.json'))
urls={json.load(open(p))['url'] for p in root.glob('http-*.json')}
for token in sorted({m for key in meta['controls'] for m in key.split('|')}):
 url='https://api.dexscreener.com/token-pairs/v1/solana/'+token
 if url in urls:continue
 data=source.get(url)
 meta['coverage_tokens'].append(token)
 if isinstance(data,list) and len(data)>=30:meta['capped_tokens'].append({'mint':token,'returned':len(data),'possible_cap':True})
manifest.update(public_requests=b.public,partial_sources=manifest['partial_sources']+source.partial,finished_epoch=int(time.time()),elapsed_seconds=str(time.time()-manifest['started_epoch']))
dump(root/'metadata.json',meta);dump(root/'manifest.json',manifest)
