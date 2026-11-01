#!/usr/bin/env python3
"""2026-11-01: All eligible saved-map pairs; preflight before any live read."""
import json,time,sys,subprocess,hashlib
from pathlib import Path
from sample import Budget,Source,dump,dex,enrich
from core import D,pair,first_failure,analyze
from collect19a import BIN
RUN='sample-20261101-19b'
SEED=Path.home()/'MEVanaData/pool-map/sample-20261101-19a/bundle.json'
def plan(seed,epoch):
 groups={}
 for p in seed['pools']:
  if p['venue_type']=='AMM':groups.setdefault(pair(p['mint0'],p['mint1']),[]).append(p)
 selected=[];planned=[]
 for key,ps in sorted(groups.items()):
  _,_,old=first_failure(ps,epoch,{})
  if len(old)>=2 or key in seed['controls']:
   planned.append(key);selected+=sorted(old if len(old)>=2 else ps,key=lambda p:-(D(p['liquidity']) if p.get('liquidity') else D(0)))[:3]
 mints=sorted({m for p in selected for m in (p['mint0'],p['mint1'])}|{m for k in seed['controls'] for m in k.split('|')})
 pools=sorted({p['address'] for p in selected});batch=lambda n:(n+99)//100
 # Every pool could reference a distinct config; this bounds unknown configs before reading.
 rpc=batch(len(mints))+3*batch(len(pools))+batch(len(pools))
 public=3*((len(pools)+29)//30)
 if rpc>15:raise RuntimeError(f'STOP: plan needs {rpc} RPC calls, over 15; no live reads made')
 if public>40:raise RuntimeError(f'STOP: plan needs {public} public requests, over 40')
 if not set(seed['controls']).issubset(planned):raise RuntimeError('STOP: control missing from saved map')
 return dict(planned_pairs=planned,short_addresses=pools,mint_requested=mints,mint_keys=mints,rpc_upper_bound=rpc,public_normal_plan=public,config_account_upper_bound=len(pools))
class Limits(Budget):
 def check(self,kind=None):
  if time.monotonic()-self.start+(20 if kind else 0)>=460:raise RuntimeError('STOP: 8-minute collection limit')
  if kind=='public':
   if self.public>=40:raise RuntimeError('STOP: 40 public request limit')
   self.public+=1
  if kind=='rpc':
   if self.rpc>=15:raise RuntimeError('STOP: 15 RPC limit')
   self.rpc+=1
 def wait(self,seconds):
  if time.monotonic()-self.start+seconds>=460:raise RuntimeError('STOP: projected collection time limit')
  time.sleep(seconds)
def readfiles(root,files,accounts):
 for file in files:
  snap=json.load(open(root/file))
  for a in snap['accounts']:accounts[a['address']]={'account':a['account'],'slot':snap['contextSlot']}
def decode(root,accounts,name):
 source=root/(name+'-input.json');target=root/(name+'-output.json');dump(source,accounts)
 subprocess.run([str(BIN/'pool_prices'),str(source),str(target)],check=True,timeout=15)
 return json.load(open(target))
def replay(root):
 meta=json.load(open(root/'metadata.json'));seed=json.load(open(root/'seed.json'));pools={p['address']:p for p in seed['pools']};samples=[{}, {}, {}]
 for file in sorted(root.glob('http-*.json')):
  e=json.load(open(file))
  if e['status']!=200:continue
  if '/latest/dex/pairs/solana/' not in e['url']:raise ValueError('unexpected public endpoint')
  i=min(range(3),key=lambda n:abs(e['collected_epoch']-meta['sample_times'][n]))
  for p in dex(json.loads(e['body'],parse_float=D)):
   old=pools.get(p['address'])
   if not old:continue
   if {p['mint0'],p['mint1']}!={old['mint0'],old['mint1']}:raise ValueError('sample mints changed')
   if p['price'] is not None and D(p['price'])>0:samples[i][p['address']]=str(D(p['price']) if p['mint0']==old['mint0'] else 1/D(p['price']))
 accounts={};readfiles(root,meta['mint_files'],accounts);rounds=[]
 for i,files in enumerate(meta['round_files']):
  readfiles(root,files,accounts)
  if i==0:readfiles(root,meta['config_files'],accounts)
  rounds.append(decode(root,accounts,'round-'+str(i)))
 plain={k:v['account'] for k,v in accounts.items()};mints={k:plain.get(k) for k in meta['mint_requested']};short=[pools[a] for a in meta['short_addresses']];depths=enrich(short,plain,mints)
 bundle=dict(meta,pools=sorted(pools.values(),key=lambda p:p['address']),controls=seed['controls'],symbols=seed['symbols'],samples=samples,mints=mints,depths=depths,chain_rounds=rounds,version='19b',onchain_evidence_kind='OnChain')
 dump(root/'bundle.json',bundle);return analyze(json.loads((root/'bundle.json').read_text()))
def collect():
 seed=json.loads(SEED.read_text());epoch=int(time.time());planned=plan(seed,epoch)
 root=Path.home()/'MEVanaData/pool-map'/RUN;root.mkdir(parents=True,exist_ok=False);dump(root/'seed.json',seed);dump(root/'plan.json',planned)
 b=Limits();source=Source(root,b);meta=dict(planned,run_id=RUN,started_epoch=epoch,sample_epoch=epoch,evidence_kind='Reported',seed_sha256=hashlib.sha256(SEED.read_bytes()).hexdigest(),sample_times=[],round_files=[],mint_files=[],config_files=[],limits_reached=[],skipped_tokens=[],capped_tokens=seed.get('capped_tokens',[]))
 try:
  before=b.rpc;source.rpc(planned['mint_requested']);meta['mint_files']=[f'rpc-{n:02}.json' for n in range(before+1,b.rpc+1)];accounts={};readfiles(root,meta['mint_files'],accounts)
  start=time.monotonic()
  for i in range(3):
   b.wait(max(0,start+120*i-time.monotonic()));meta['sample_times'].append(int(time.time()));before=b.rpc;source.rpc(planned['short_addresses']);files=[f'rpc-{n:02}.json' for n in range(before+1,b.rpc+1)];meta['round_files'].append(files);readfiles(root,files,accounts)
   if i==0:
    prices=decode(root,accounts,'preconfig');configs=sorted({p['config'] for p in prices.values() if p.get('config')})
    # Configs cannot exceed the preflight's one-per-pool upper bound.
    assert len(configs)<=planned['config_account_upper_bound']
    before=b.rpc;source.rpc(configs);meta['config_files']=[f'rpc-{n:02}.json' for n in range(before+1,b.rpc+1)]
   for n in range(0,len(planned['short_addresses']),30):source.get('https://api.dexscreener.com/latest/dex/pairs/solana/'+','.join(planned['short_addresses'][n:n+30]))
  dump(root/'metadata.json',meta);dump(root/'result.json',replay(root))
 except Exception as e:
  meta['limits_reached'].append(str(e) if str(e).startswith('STOP:') else 'collection_failed');raise
 finally:
  meta.update(public_requests=b.public,rpc_calls=b.rpc,partial_sources=source.partial,finished_epoch=int(time.time()),elapsed_seconds=str(D(str(time.monotonic()-b.start))));dump(root/'manifest.json',meta)
 return root
if __name__=='__main__':
 if sys.argv[1]=='plan':print(json.dumps(plan(json.loads(SEED.read_text()),int(time.time())),indent=2))
 elif sys.argv[1]=='collect':print(collect())
 else:
  root=Path(sys.argv[2]);dump(root/'replay.json',replay(root))
