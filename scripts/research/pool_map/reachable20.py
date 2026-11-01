#!/usr/bin/env python3
"""2026-11-01: Handoff 20 immutable preflight, bounded collection and offline replay."""
import argparse,base64,hashlib,itertools,json,subprocess,time
from pathlib import Path
from decimal import Decimal as D, getcontext
from dlmm import *
from core import EXEMPT,SOL,SPL,pair,mint_check,first_failure,analyze
from sample import dump
getcontext().prec=80
RUN='sample-20261101-20'
ROOT=Path(__file__).resolve().parents[3]
DATA=Path.home()/'MEVanaData/pool-map'
SEED=DATA/'sample-20261101-19b/bundle.json'
BIN=Path('/Users/michaelinman/Repos/.cargo-target/mevana/debug/pool_snapshot')
SIZES=(100,1000)
CONTROL_SYMBOLS={'Bonk / SOL','Bonk / USDC','USDC / JUP','JUP / SOL'}


def encoded(value):return (json.dumps(value,default=str,sort_keys=True,indent=2)+'\n').encode()
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def eligible(b,key):
 ps=[p for p in b['pools'] if p['venue_type']=='AMM' and pair(p['mint0'],p['mint1'])==key]
 failed,_,old=first_failure(ps,b['sample_epoch'],b['mints'])
 return [] if failed else old

def clears(b,a,c,key):
 for r in b['chain_rounds']:
  x,y=r.get(a['address']),r.get(c['address'])
  if not x or not y or abs(x['slot']-y['slot'])>25:continue
  if x['fee_micro_bps'] is None or y['fee_micro_bps'] is None:continue
  px=D(x['price_scaled']);py=D(y['price_scaled'])
  if x['mint0']!=key.split('|')[0]:px=D(10**48)/px
  if y['mint0']!=key.split('|')[0]:py=D(10**48)/py
  if abs(px-py)*20000/(px+py)>D(x['fee_micro_bps']+y['fee_micro_bps'])/1000000:return True
 return False

def preflight(seed,array_cap=25):
 rows=analyze(seed)['pairs'];selected=[]
 for row in rows:
  control=row['symbol'] in CONTROL_SYMBOLS
  if row['failed_step'] not in (None,'R5') or not (control or row['above_fee']):continue
  for a,b in itertools.combinations(sorted(eligible(seed,row['pair']),key=lambda p:p['address']),2):
   if clears(seed,a,b,row['pair']):selected.append(dict(pair=row['pair'],symbol=row['symbol'],control=control,pools=[a['address'],b['address']]))
 symbols={p['symbol'] for p in selected}
 expected={'PENGU / USDC','HYPE / USDC','USDT / FO','WBTC / USDC','JUP / MET'}|CONTROL_SYMBOLS
 if symbols!=expected:raise RuntimeError('STOP: saved-map scope differs from handoff')
 pool_map={p['address']:p for p in seed['pools']};keys=sorted({a for p in selected for a in p['pools']})
 # Declared nominal USD inputs: stablecoin=$1; SOL from saved-map Orca SOL/USDC;
 # JUP from an already in-scope JUP/USDC pool. They are valuation inputs only.
 symbols_by_mint=seed['symbols'];usdc=next(m for m in EXEMPT if EXEMPT[m].startswith('USDC:'));jup=next(m for m,s in symbols_by_mint.items() if s=='JUP')
 refs={}
 for mint in (SOL,jup):
  choices=[p for p in seed['pools'] if {p['mint0'],p['mint1']}=={mint,usdc} and (p['venue']=='orca' if mint==SOL else p['address'] in keys)]
  if not choices:raise RuntimeError('STOP: missing saved-map USD reference')
  refs[mint]=min(choices,key=lambda p:p['address'])['address']
 keys=sorted(set(keys)|set(refs.values()));mints=sorted({m for a in keys for m in (pool_map[a]['mint0'],pool_map[a]['mint1'])})
 meteora=[a for a in keys if pool_map[a]['venue']=='meteora'];other=[a for a in keys if pool_map[a]['venue'] not in ('meteora','orca')]
 if other:raise RuntimeError('STOP: unsupported scoped venue')
 batch=lambda n:(n+99)//100
 calls=2*(batch(len(keys)+len(mints))+batch(len(meteora)*array_cap))
 if calls>15:raise RuntimeError(f'STOP: preflight needs {calls} calls; no reads made')
 if calls*8+120>=280:raise RuntimeError('STOP: projected collection duration exceeds safety bound')
 return dict(run_id=RUN,pairs=selected,pool_keys=keys,mint_keys=mints,meteora_keys=meteora,array_cap_per_pool=array_cap,rpc_upper_bound=calls,public_requests=0,geyser_bytes=0,sizes_usd=list(SIZES),max_slot_spread=25,reachability='output_atoms > input_atoms; full modeled legs only',usd_references=refs,usdc=usdc,jup=jup,valuation='USDC/USDT nominal $1; SOL and JUP from declared same-round nominal pool prices, price inputs only',orca='Indicative: tick arrays are not part of this bounded account plan')

class Reader:
 def __init__(self,root):self.root=root;self.start=time.monotonic();self.calls=0
 def read(self,keys):
  out={};files=[]
  for i in range(0,len(keys),100):
   if self.calls>=15 or time.monotonic()-self.start+8>=280:raise RuntimeError('STOP: hard collection budget')
   self.calls+=1;path=self.root/f'rpc-{self.calls:02}.json'
   subprocess.run([str(BIN),'--keys',*keys[i:i+100],'--output',str(path)],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,timeout=8)
   snap=json.loads(path.read_text());files.append(path.name)
   for a in snap['accounts']:out[a['address']]={'account':a['account'],'slot':snap['contextSlot']}
  return out,files

def raw(account):return base64.b64decode(account['data'][0],validate=True)
def pool_price(account):
 if account['owner']==METEORA:
  p=decode_account(account,'LbPair');return p['token_x_mint'],p['token_y_mint'],(D(10000+p['bin_step'])/10000)**p['active_id']
 if account['owner']==ORCA:
  b=raw(account)
  if len(b)!=653 or b[:8]!=hashlib.sha256(b'account:Whirlpool').digest()[:8]:raise MissingState('invalid Whirlpool')
  return b58(b[101:133]),b58(b[181:213]),D(int.from_bytes(b[65:81],'little'))**2/D(Q**2)
 raise MissingState('unsupported pool')

def valuation(plan,accounts,decimals):
 prices={m:D(1) for m in decimals if m in EXEMPT}
 for mint,key in plan['usd_references'].items():
  x,y,p=pool_price(accounts[key]['account']);p=p*D(10)**(decimals[x]-decimals[y]);prices[mint]=p if mint==x else 1/p
 return prices

def outcome(input_,output,indicative,slots):
 if not slots or max(slots)-min(slots)>25 or output is None:return 'Not measured'
 if indicative:return 'Indicative'
 return 'Reachable' if output>input_ else 'NotReachable'

def quote(key,input_mint,amount,accounts,now):
 a=accounts[key]['account']
 if a['owner']==METEORA:
  p=decode_account(a,'LbPair');swap_y=input_mint==p['token_x_mint']
  if input_mint not in (p['token_x_mint'],p['token_y_mint']):raise MissingState('wrong input mint')
  bins=bins_from_arrays(key,p,{k:v['account'] for k,v in accounts.items()});q=walk(p,bins,amount,swap_y,now,accounts[key]['slot'])
  needed=[bin_address(key,b['bin_id']//70) for b in q['bins']]
  q.update(indicative=False,output_mint=p['token_y_mint'] if swap_y else p['token_x_mint'],slots=[accounts[key]['slot']]+[accounts[k]['slot'] for k in needed],pool=key)
  return q
 x,y,p=pool_price(a);b=raw(a);fee=int.from_bytes(b[45:47],'little')
 # Standard-fee nominal Whirlpool estimate only: no tick state, impact unmeasured.
 if b[41:43]!=b[43:45]:raise MissingState('adaptive Whirlpool fees unsupported')
 net=amount-ceildiv(amount*fee,10**6);output=int(D(net)*p) if input_mint==x else int(D(net)/p)
 return dict(pool=key,input_atoms=amount,output_atoms=output,output_mint=y if input_mint==x else x,fee_atoms=amount-net,fee_token='input',evidence_kind='Indicative',indicative=True,slots=[accounts[key]['slot']],bins=[])

def replay(root):
 plan=json.loads((root/'plan.json').read_text());meta=json.loads((root/'metadata.json').read_text());rounds=[];stale={a:[] for a in plan['meteora_keys']}
 for round_no,info in enumerate(meta['rounds']):
  accounts={}
  for file in info['pool_files']+info['array_files']:
   snap=json.loads((root/file).read_text())
   for a in snap['accounts']:accounts[a['address']]={'account':a['account'],'slot':snap['contextSlot']}
  decimals={}
  for m in plan['mint_keys']:
   check=mint_check(accounts[m]['account'],m in EXEMPT)
   if not check['safe'] or accounts[m]['account']['owner']!=SPL:raise MissingState('unsafe or unsupported mint')
   decimals[m]=check['decimals']
  prices=valuation(plan,accounts,decimals);now=info['epoch'];pool_labels={}
  for key in plan['meteora_keys']:
   p=decode_account(accounts[key]['account'],'LbPair');bins=bins_from_arrays(key,p,{k:v['account'] for k,v in accounts.items()})
   try:lower=nearest(p,bins,True);upper=nearest(p,bins,False)
   except MissingState:lower=upper=None
   label=dict(pool=key,round=round_no+1,active_id=p['active_id'],slot=accounts[key]['slot'],empty_bins_to_y=lower,empty_bins_to_x=upper,last_update_age_seconds=now-p['v_parameters']['last_update_timestamp'],timestamp_semantics='variable fee parameters updated; not independent proof of last trade',volatility=p['v_parameters'],parameters=p['parameters'])
   pool_labels[key]=label;stale[key].append(p['active_id'])
  rows=[]
  for pair_ in plan['pairs']:
   mints=pair_['pair'].split('|');fund=next((m for m in mints if m in EXEMPT),SOL if SOL in mints else plan['jup']);usd=prices[fund]
   for size in SIZES:
    amount=int(D(size)*D(10)**decimals[fund]/usd)
    for order in (pair_['pools'],list(reversed(pair_['pools']))):
     r=dict(pair=pair_['pair'],symbol=pair_['symbol'],control=pair_['control'],pool_buy=order[0],pool_sell=order[1],size_usd=size,round=round_no+1,input_mint=fund,input_atoms=amount,usd_per_funding_token=str(usd),evidence_kind='Modeled',result='Not measured',reason=None,output_atoms=None,modeled_dollars=None,average_buy_price=None,average_sell_price=None,legs=[])
     try:
      if any(accounts[k]['account'] is None for k in order):raise MissingState('pool absent')
      first=quote(order[0],fund,amount,accounts,now);second=quote(order[1],first['output_mint'],first['output_atoms'],accounts,now)
      slots=first['slots']+second['slots']+[accounts[m]['slot'] for m in mints]+[accounts[k]['slot'] for k in plan['usd_references'].values()]
      if second['output_mint']!=fund:raise MissingState('route mint mismatch')
      r.update(result=outcome(amount,second['output_atoms'],first['indicative'] or second['indicative'],slots),output_atoms=second['output_atoms'],modeled_dollars=str(D(second['output_atoms']-amount)*usd/D(10)**decimals[fund]),legs=[first,second],slot_spread=max(slots)-min(slots))
      intermediate=D(first['output_atoms'])/D(10)**decimals[first['output_mint']]
      if intermediate:r['average_buy_price']=str(D(amount)/D(10)**decimals[fund]/intermediate);r['average_sell_price']=str(D(second['output_atoms'])/D(10)**decimals[fund]/intermediate)
      r['price_unit']='funding tokens per intermediate token, fees included'
      if r['result']=='Not measured':r['reason']='slot spread exceeds 25'
     except (MissingState,KeyError,TypeError,ZeroDivisionError) as e:r['reason']=str(e)
     rows.append(r)
  rounds.append(dict(round=round_no+1,epoch=now,pool_labels=pool_labels,rows=rows))
 for round_ in rounds:
  for k,v in round_['pool_labels'].items():v['price_changed_across_rounds']=len(set(stale[k]))>1
 # One dashboard row per full pool pair, size and direction; retain both rounds.
 groups={}
 for round_ in rounds:
  for r in round_['rows']:groups.setdefault((r['pool_buy'],r['pool_sell'],r['size_usd']),[]).append(r)
 summary=[]
 for _,rs in sorted(groups.items()):
  priority={'Reachable':3,'NotReachable':2,'Indicative':1,'Not measured':0};best=max(rs,key=lambda r:(priority[r['result']],D(r['modeled_dollars']) if r['modeled_dollars'] is not None else D('-Infinity')))
  summary.append(dict(best,round_results=[r['result'] for r in rs]))
 reachable={r['pair'] for r in summary if not r['control'] and r['result']=='Reachable'}
 return dict(run_id=RUN,evidence_kind='Modeled',plan=plan,rounds=rounds,summary=summary,non_control_reachable_count=len(reachable),limitations=['Before transaction, priority and tip costs; no execution claim','USD references are nominal price inputs, not additional route legs','Orca has no tick arrays in this plan and remains Indicative','Unknown liquidity beyond sampled initialized arrays remains Not measured'])

def collect():
 seed=json.loads(SEED.read_text());plan=preflight(seed);root=DATA/RUN
 root.mkdir(exist_ok=False);dump(root/'plan.json',plan);reader=Reader(root);meta=dict(run_id=RUN,seed_sha256=sha(SEED),started_epoch=int(time.time()),rounds=[])
 try:
  for i in range(2):
   target=reader.start+120*i
   # External runner invokes collection asynchronously; bounded two-minute wait only here.
   while time.monotonic()<target:time.sleep(min(1,target-time.monotonic()))
   accounts,files=reader.read(sorted(plan['pool_keys']+plan['mint_keys']));epoch=int(time.time());array_keys=[]
   for key in plan['meteora_keys']:
    p=decode_account(accounts[key]['account'],'LbPair');indices=bin_indices(p)
    if len(indices)>plan['array_cap_per_pool']:raise RuntimeError('STOP: arrays exceed preflight')
    array_keys.extend(bin_address(key,index) for index in indices)
   arrays,array_files=reader.read(sorted(set(array_keys)))
   meta['rounds'].append(dict(epoch=epoch,pool_files=files,array_files=array_files,array_keys=sorted(set(array_keys))))
  dump(root/'metadata.json',meta);dump(root/'result.json',replay(root))
 finally:
  dump(root/'manifest.json',dict(meta,rpc_calls=reader.calls,public_requests=0,geyser_bytes=0,elapsed_seconds=str(D(str(time.monotonic()-reader.start))),finished_epoch=int(time.time())))
 return root
if __name__=='__main__':
 parser=argparse.ArgumentParser();parser.add_argument('command',choices=['plan','collect','replay']);parser.add_argument('root',nargs='?',type=Path);a=parser.parse_args()
 if a.command=='plan':print(encoded(preflight(json.loads(SEED.read_text()))).decode(),end='')
 elif a.command=='collect':print(collect())
 else:dump(a.root/'replay20.json',replay(a.root))
