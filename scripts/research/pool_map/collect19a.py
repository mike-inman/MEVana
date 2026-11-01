#!/usr/bin/env python3
"""2026-11-01: One capped 19a sample; three chain rounds two minutes apart."""
import json,time,sys,subprocess
from pathlib import Path
from sample import Budget,Source,gt,dex,dump,enrich
from core import D,pair,EXEMPT,first_failure,analyze
from fixes import merge,token_order,possible_cap,reserve_token
ROOT=Path(__file__).resolve().parents[3]
BIN=Path('/Users/michaelinman/Repos/.cargo-target/mevana/debug')
class Limits(Budget):
 def check(self,kind=None):
  if time.monotonic()-self.start+(20 if kind else 0)>=580:raise RuntimeError('STOP: 10-minute collection limit')
  if kind=='public':
   if self.public>=220:raise RuntimeError('STOP: 220 public request limit')
   self.public+=1
  if kind=='rpc':
   if self.rpc>=10:raise RuntimeError('STOP: 10 RPC limit')
   self.rpc+=1
 def wait(self,seconds):
  if time.monotonic()-self.start+seconds>=580:raise RuntimeError('STOP: projected time limit')
  time.sleep(seconds)
def replay(root):
 m=json.load(open(root/'metadata.json'));pools={};samples=[{}, {}, {}]
 for file in sorted(root.glob('http-*.json')):
  e=json.load(open(file))
  if e['status']!=200:continue
  body=json.loads(e['body'],parse_float=D)
  if '/latest/dex/pairs/solana/' in e['url']:continue
  items=gt(body) if 'geckoterminal' in e['url'] else dex(body)
  for p in items:p['collected_epoch']=e['collected_epoch']
  merge(pools,items)
 for file in sorted(root.glob('http-*.json')):
  e=json.load(open(file))
  if e['status']!=200 or '/latest/dex/pairs/solana/' not in e['url']:continue
  i=min(range(3),key=lambda n:abs(m['sample_times'][n]-e['collected_epoch']))
  for p in dex(json.loads(e['body'],parse_float=D)):
   old=pools.get(p['address'])
   if old and {p['mint0'],p['mint1']}!={old['mint0'],old['mint1']}:raise ValueError('sample mint mismatch')
   if old and p.get('price') is not None and D(p['price'])>0:
    v=D(p['price']);samples[i][p['address']]=str(v if p['mint0']==old['mint0'] else 1/v)
 accounts={};rounds=[]
 for files in m['round_files']:
  for f in files:
   snap=json.load(open(root/f))
   for a in snap['accounts']:accounts[a['address']]={'account':a['account'],'slot':snap['contextSlot']}
  # Configuration read was saved separately after the first price round.
  if not rounds:
   for f in m['config_files']:
    snap=json.load(open(root/f))
    for a in snap['accounts']:accounts[a['address']]={'account':a['account'],'slot':snap['contextSlot']}
  input_=root/f'decode-input-{len(rounds)}.json';output=root/f'decode-output-{len(rounds)}.json';dump(input_,accounts)
  subprocess.run([str(BIN/'pool_prices'),str(input_),str(output)],check=True,timeout=10)
  rounds.append(json.load(open(output)))
 short=[pools[a] for a in m['short_addresses']];plain={k:v['account'] for k,v in accounts.items()};mint_accounts={k:plain.get(k) for k in m['mint_keys']}
 depths=enrich(short,plain,mint_accounts)
 symbols={}
 for p in pools.values():
  for i in [0,1]:
   if p.get('symbol'+str(i)):symbols[p['mint'+str(i)]]=p['symbol'+str(i)]
 symbols={**m['control_symbols'],**symbols}
 bundle=dict(m,pools=sorted(pools.values(),key=lambda p:p['address']),samples=samples,mints=mint_accounts,depths=depths,chain_rounds=rounds,symbols=symbols,onchain_evidence_kind='OnChain',version='19a')
 dump(root/'bundle.json',bundle);return analyze(json.loads((root/'bundle.json').read_text()))
def collect(run):
 root=Path.home()/'MEVanaData/pool-map'/run;root.mkdir(parents=True,exist_ok=False);b=Limits();s=Source(root,b);meta={'run_id':run,'started_epoch':int(time.time()),'evidence_kind':'Reported','skipped_tokens':[],'capped_tokens':[],'limits_reached':[]}
 try:
  control=json.load(open(ROOT/'crates/gap-survey/allowlist.json'))['pools'];controls=sorted({pair(p['mint0'],p['mint1']) for p in control});symbols={}
  for p in control:
   for mint in [p['mint0'],p['mint1']]:symbols.setdefault(mint,mint[:6]+'…')
  symbols.update({'So11111111111111111111111111111111111111112':'SOL','EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v':'USDC','Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB':'USDT'})
  pools={};base='https://api.geckoterminal.com/api/v2/networks/solana'
  for endpoint in ['/pools','/new_pools']:merge(pools,gt(s.get(base+endpoint)))
  for q in ['SOL','USDC','orca','raydium','meteora','pumpswap']:merge(pools,dex(s.get('https://api.dexscreener.com/latest/dex/search?q='+q)))
  tokens=token_order(list(pools.values()),controls,set(EXEMPT));meta['coverage_tokens']=tokens
  for token in tokens:
   if not reserve_token(token,b.public,meta['skipped_tokens']):continue
   data=s.get('https://api.dexscreener.com/token-pairs/v1/solana/'+token);items=dex(data)
   if possible_cap(len(items)):meta['capped_tokens'].append({'mint':token,'returned':len(items),'possible_cap':True})
   merge(pools,items)
  if not pools:raise RuntimeError('STOP: no public source works')
  epoch=int(time.time());groups={}
  for p in pools.values():
   if p['venue_type']=='AMM':groups.setdefault(pair(p['mint0'],p['mint1']),[]).append(p)
  eligible=[]
  for key,ps in groups.items():
   _,_,old=first_failure(ps,epoch,{})
   if len(old)>=2:eligible.append((key,old))
  eligible.sort(key=lambda x:(x[0] not in controls,-max(D(p['liquidity']) for p in x[1]),x[0]));eligible=eligible[:20]
  short=[p for _,ps in eligible for p in sorted(ps,key=lambda p:-D(p['liquidity']))[:3]]
  mintkeys=sorted({p[k] for p in short for k in ['mint0','mint1']});keys=sorted(set(mintkeys+[p['address'] for p in short]));meta.update(controls=controls,control_symbols=symbols,sample_epoch=epoch,short_addresses=[p['address'] for p in short],mint_keys=mintkeys,sample_times=[],round_files=[],config_files=[])
  start=time.monotonic();configs=[]
  for n in range(3):
   b.wait(max(0,start+120*n-time.monotonic()));meta['sample_times'].append(int(time.time()));before=b.rpc;s.rpc(sorted(set(keys+configs)));meta['round_files'].append([f'rpc-{i:02}.json' for i in range(before+1,b.rpc+1)])
   if n==0:
    accounts={}
    for f in meta['round_files'][0]:
     snap=json.load(open(root/f))
     for a in snap['accounts']:accounts[a['address']]={'account':a['account'],'slot':snap['contextSlot']}
    dump(root/'predecode.json',accounts);subprocess.run([str(BIN/'pool_prices'),str(root/'predecode.json'),str(root/'preprices.json')],check=True,timeout=10)
    configs=sorted({p['config'] for p in json.load(open(root/'preprices.json')).values() if p.get('config')})
    if configs:
     before=b.rpc;s.rpc(configs);meta['config_files']=[f'rpc-{i:02}.json' for i in range(before+1,b.rpc+1)]
   for i in range(0,len(short),30):s.get('https://api.dexscreener.com/latest/dex/pairs/solana/'+','.join(p['address'] for p in short[i:i+30]))
  dump(root/'metadata.json',meta);dump(root/'result.json',replay(root))
 except Exception as e:
  meta['limits_reached'].append(str(e) if str(e).startswith('STOP:') else 'collection_failed');raise
 finally:
  meta.update(public_requests=b.public,rpc_calls=b.rpc,partial_sources=s.partial,finished_epoch=int(time.time()),elapsed_seconds=str(D(str(time.monotonic()-b.start))));dump(root/'manifest.json',meta)
 return root
if __name__=='__main__':
 if sys.argv[1]=='collect':print(collect(sys.argv[2]))
 else:
  root=Path(sys.argv[2]);dump(root/'replay.json',replay(root))
