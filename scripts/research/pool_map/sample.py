#!/usr/bin/env python3
"""2026-10-31: One bounded public pool-map sample; saved-response replay."""
import argparse,base64,json,time,urllib.request,urllib.error,subprocess,sys
from pathlib import Path
from decimal import Decimal
from core import D,decimal,pair,analyze,first_failure,mint_check,depth
ROOT=Path(__file__).resolve().parents[3]
def dump(path,data):path.write_text(json.dumps(data,default=str,sort_keys=True,indent=2)+'\n')
class Budget:
 def __init__(self,start=None):self.start=time.monotonic() if start is None else start;self.public=0;self.rpc=0
 def check(self,kind=None):
  if time.monotonic()-self.start+(20 if kind else 0)>=1190:raise RuntimeError('STOP: collection time limit')
  if kind=='public':
   if self.public>=300:raise RuntimeError('STOP: public request limit')
   self.public+=1
  if kind=='rpc':
   if self.rpc>=20:raise RuntimeError('STOP: RPC call limit')
   self.rpc+=1
 def wait(self,seconds):
  if time.monotonic()-self.start+seconds>=1190:raise RuntimeError('STOP: projected collection exceeds time limit')
  time.sleep(seconds)
class Source:
 def __init__(self,root,budget):self.root=root;self.b=budget;self.partial=[];self.last={};self.n=0
 def get(self,url):
  host=urllib.request.urlparse(url).hostname if False else url.split('/')[2]
  spacing=2.2 if 'geckoterminal' in host else 1.1
  for attempt in range(3):
   self.b.wait(max(0,spacing-(time.monotonic()-self.last.get(host,0))));self.b.check('public');self.last[host]=time.monotonic();self.n+=1
   status=0;headers={};raw=b''
   try:
    req=urllib.request.Request(url,headers={'Accept':'application/json','User-Agent':'MEVana-read-only-research/19'})
    with urllib.request.urlopen(req,timeout=15) as response:status=response.status;headers=dict(response.headers);raw=response.read(10000001)
   except urllib.error.HTTPError as e:status=e.code;headers=dict(e.headers);raw=e.read(10000001)
   except Exception:pass
   dump(self.root/f'http-{self.n:03}.json',{'url':url,'status':status,'collected_epoch':int(time.time()),'body':raw.decode('utf8','replace')})
   if status==200 and len(raw)<=10000000:
    try:return json.loads(raw,parse_float=Decimal)
    except Exception:pass
   if status==429 or status>=500:
    if attempt<2:
     retry=headers.get('Retry-After','5')
     try:wait=max(5,int(retry))
     except ValueError:
      from email.utils import parsedate_to_datetime
      wait=max(5,int(parsedate_to_datetime(retry).timestamp()-time.time()))
     self.b.wait(wait);continue
   self.partial.append({'source':host,'status':status,'url':url});return None
 def rpc(self,keys):
  result={}
  for i in range(0,len(keys),100):
   self.b.check('rpc');out=self.root/f'rpc-{self.b.rpc:02}.json'
   binary=Path('/Users/michaelinman/Repos/.cargo-target/mevana/debug/pool_snapshot')
   subprocess.run([str(binary),'--keys',*keys[i:i+100],'--output',str(out)],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,timeout=20)
   for r in json.loads(out.read_text())['accounts']:result[r['address']]=r['account']
  return result
def gt(data):
 out=[]
 for x in (data or {}).get('data',[]):
  try:
   a=x['attributes'];rel=x['relationships'];m0=rel['base_token']['data']['id'].removeprefix('solana_');m1=rel['quote_token']['data']['id'].removeprefix('solana_')
   from datetime import datetime
   created=int(datetime.fromisoformat(a['pool_created_at'].replace('Z','+00:00')).timestamp()) if a.get('pool_created_at') else None
   out.append({'address':a['address'],'venue':rel['dex']['data']['id'],'pool_type':None,'mint0':m0,'mint1':m1,'price':a.get('base_token_price_quote_token'),'base_usd':a.get('base_token_price_usd'),'quote_usd':a.get('quote_token_price_usd'),'liquidity':a.get('reserve_in_usd'),'volume':a.get('volume_usd',{}).get('h24'),'fee_bps':None,'created':created,'source':'GeckoTerminal','collected_epoch':int(time.time()),'evidence_kind':'Reported'})
  except (KeyError,ValueError,TypeError):continue
 return out
def dex(data):
 out=[]
 for x in data if isinstance(data,list) else (data or {}).get('pairs',[]) or []:
  if x.get('chainId')!='solana':continue
  try:
   native=decimal(x.get('priceNative'));usd=decimal(x.get('priceUsd'))
   out.append({'address':x['pairAddress'],'venue':x['dexId'],'pool_type':','.join(x.get('labels') or []) or None,'mint0':x['baseToken']['address'],'mint1':x['quoteToken']['address'],'price':x.get('priceNative'),'base_usd':x.get('priceUsd'),'quote_usd':str(usd/native) if usd is not None and native else None,'liquidity':x.get('liquidity',{}).get('usd'),'volume':x.get('volume',{}).get('h24'),'fee_bps':None,'created':int(x['pairCreatedAt'])//1000 if x.get('pairCreatedAt') else None,'source':'DexScreener','collected_epoch':int(time.time()),'evidence_kind':'Reported'})
  except (KeyError,ValueError,TypeError):continue
 return out
def enrich(short,chain,mints):
 depths={}
 for p in short:
  account=chain.get(p['address']);p['onchain_pool_owner']=account.get('owner') if account else None
  if not account:continue
  raw=base64.b64decode(account['data'][0]);owner=account['owner'];d0=mint_check(mints.get(p['mint0']))['decimals'];d1=mint_check(mints.get(p['mint1']))['decimals']
  if owner=='whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc' and len(raw)==653:
   # Whirlpool token mint order may be opposite the reported pool order.
   liq=int.from_bytes(raw[49:65],'little');sqrt=int.from_bytes(raw[65:81],'little');fee=int.from_bytes(raw[45:47],'little')/D(100)
   mint1=raw[181:213]
  elif owner=='CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK' and len(raw)==1544:
   liq=int.from_bytes(raw[237:253],'little');sqrt=int.from_bytes(raw[253:269],'little');fee=None;mint1=raw[105:137]
  else:continue
  # Match mint bytes without adding a base58 dependency.
  def decode58(v):
   n=0
   for c in v:n=n*58+'123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz'.index(c)
   return b'\0'*(len(v)-len(v.lstrip('1')))+n.to_bytes((n.bit_length()+7)//8,'big')
  reverse=mint1!=decode58(p['mint1']);dec=d0 if reverse else d1;usd=p.get('base_usd') if reverse else p.get('quote_usd')
  if dec is not None:depths[p['address']]=depth(liq,sqrt,dec,usd)
  if fee is not None:p['fee_bps']=str(fee);p['fee_evidence_kind']='OnChain'
 return depths
def collect(run_id):
 root=Path.home()/'MEVanaData/pool-map'/run_id;root.mkdir(parents=True,exist_ok=False);b=Budget();s=Source(root,b)
 manifest={'run_id':run_id,'evidence_kind':'Reported','started_epoch':int(time.time()),'public_requests':0,'rpc_calls':0,'partial_sources':[],'limits_reached':[]}
 try:
  pools={}
  def add(items):
   for p in items:
    old=pools.get(p['address'])
    if old and {old['mint0'],old['mint1']}!={p['mint0'],p['mint1']}:raise RuntimeError('pool mint disagreement between sources')
    pools[p['address']]=p
  base='https://api.geckoterminal.com/api/v2/networks/solana'
  listing=s.get(base+'/dexes');ids=[x['id'] for x in (listing or {}).get('data',[]) if any(k in x['id'].lower() for k in ['orca','raydium','meteora','pump'])]
  for ident in ids[:12]:add(gt(s.get(base+'/dexes/'+ident+'/pools')))
  add(gt(s.get(base+'/pools')));add(gt(s.get(base+'/new_pools')))
  for query in ['SOL','USDC','raydium','meteora','pumpswap','orca']:add(dex(s.get('https://api.dexscreener.com/latest/dex/search?q='+query)))
  recent=s.get('https://api.dexscreener.com/token-profiles/latest/v1')
  tokens=[x['tokenAddress'] for x in recent or [] if x.get('chainId')=='solana'][:10]
  tokens+=list(dict.fromkeys(p['mint0'] for p in pools.values()))[:20]
  for token in list(dict.fromkeys(tokens)):add(dex(s.get('https://api.dexscreener.com/token-pairs/v1/solana/'+token)))
  if not pools:raise RuntimeError('STOP: no public source works')
  epoch=int(time.time());controls_data=json.loads((ROOT/'crates/gap-survey/allowlist.json').read_text())
  controls=sorted({pair(p['mint0'],p['mint1']) for p in controls_data['pools']})
  grouped={}
  for p in pools.values():grouped.setdefault(pair(p['mint0'],p['mint1']),[]).append(p)
  eligible=[]
  for key,ps in grouped.items():
   _,_,old=first_failure(ps,epoch,{})
   if len(old)>=2:eligible.append((key,old))
  eligible.sort(key=lambda x:(x[0] not in controls,-max(D(p['liquidity']) for p in x[1]),x[0]))
  eligible=eligible[:40];short=[p for _,ps in eligible for p in sorted(ps,key=lambda p:-D(p['liquidity']))[:3]]
  mints=s.rpc(sorted({p[k] for p in short for k in ['mint0','mint1']})) if short else {}
  samples=[];sample_times=[];start=time.monotonic()
  for round_index in range(3):
   b.wait(max(0,start+round_index*300-time.monotonic()));values={};sample_times.append(int(time.time()))
   addresses=[p['address'] for p in short]
   for i in range(0,len(addresses),30):
    ids=','.join(addresses[i:i+30]);items=dex(s.get('https://api.dexscreener.com/latest/dex/pairs/solana/'+ids))
    for p in items:
     original=pools[p['address']];v=decimal(p['price'])
     if v is not None and v>0:values[p['address']]=str(v if p['mint0']==original['mint0'] else 1/v)
   samples.append(values)
  chain=s.rpc([p['address'] for p in short]) if short else {};depths=enrich(short,chain,mints)
  bundle={'run_id':run_id,'sample_epoch':epoch,'pools':sorted(pools.values(),key=lambda p:p['address']),'controls':controls,'mints':mints,'samples':samples,'sample_times':sample_times,'depths':depths,'onchain_evidence_kind':'OnChain'}
  dump(root/'bundle.json',bundle);dump(root/'result.json',analyze(json.loads((root/'bundle.json').read_text())))
 except Exception as e:
  manifest['limits_reached'].append(str(e) if str(e).startswith('STOP:') else 'collection_failed');raise
 finally:
  manifest.update(public_requests=b.public,rpc_calls=b.rpc,partial_sources=s.partial,finished_epoch=int(time.time()),elapsed_seconds=str(D(str(time.monotonic()-b.start))))
  dump(root/'manifest.json',manifest)
 return root
def rebuild(root):
 original=json.loads((root/'bundle.json').read_text());pools={};samples=[{} for _ in range(3)]
 for file in sorted(root.glob('http-*.json')):
  envelope=json.loads(file.read_text())
  if envelope['status']!=200:continue
  data=json.loads(envelope['body'],parse_float=Decimal);url=envelope['url']
  if '/latest/dex/pairs/solana/' in url:continue
  items=gt(data) if 'geckoterminal' in url else dex(data) if 'token-profiles' not in url else []
  for item in items:
   item['collected_epoch']=envelope['collected_epoch'];pools[item['address']]=item
 for old in original['pools']:
  pools[old['address']]['collected_epoch']=old['collected_epoch']
 for file in sorted(root.glob('http-*.json')):
  envelope=json.loads(file.read_text())
  if envelope['status']!=200 or '/latest/dex/pairs/solana/' not in envelope['url']:continue
  i=min(range(3),key=lambda n:abs(original['sample_times'][n]-envelope['collected_epoch']))
  for p in dex(json.loads(envelope['body'],parse_float=Decimal)):
   old=pools[p['address']];v=decimal(p['price'])
   if {p['mint0'],p['mint1']}!={old['mint0'],old['mint1']}:raise ValueError('sample mint mismatch')
   if v is not None and v>0:samples[i][p['address']]=str(v if p['mint0']==old['mint0'] else 1/v)
 accounts={}
 for file in sorted(root.glob('rpc-*.json')):
  for item in json.loads(file.read_text())['accounts']:accounts[item['address']]=item['account']
 short=[pools[p['address']] for p in original['pools'] if 'onchain_pool_owner' in p]
 mints={key:accounts[key] for key in original['mints']}
 depths=enrich(short,accounts,mints)
 return dict(original,pools=sorted(pools.values(),key=lambda p:p['address']),samples=samples,mints=mints,depths=depths)
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('mode',choices=['collect','replay']);p.add_argument('path');args=p.parse_args()
 if args.mode=='collect':print(collect(args.path))
 else:
  root=Path(args.path);dump(root/'replay.json',analyze(rebuild(root)))
