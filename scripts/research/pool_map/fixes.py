"""2026-11-01: Handoff 19a coverage and same-pool on-chain comparisons."""
from decimal import Decimal as D
from itertools import combinations
BOND={'pump-fun','pumpfun','raydium-launchlab','meteoradbc','meteora-dbc'}
AMM={'pumpswap','orca','raydium','meteora','raydium-clmm','meteora-damm-v2','pancakeswap','pancakeswap-v3-solana','fluxbeam'}
def merge(pools,items):
 for p in items:
  old=pools.get(p['address'])
  if old and {old['mint0'],old['mint1']}!={p['mint0'],p['mint1']}:raise ValueError('pool mint disagreement')
  p['venue_type']='BondingCurve' if p['venue'] in BOND else 'AMM' if p['venue'] in AMM else 'Unknown';pools[p['address']]=p
 return pools
def token_order(pools,controls,stable):
 rank={m:D(0) for key in controls for m in key.split('|')}
 for p in pools:
  if p.get('liquidity') is None or D(p['liquidity'])<20000:continue
  for m in [p['mint0'],p['mint1']]:
   if m not in stable:rank[m]=max(rank.get(m,D(0)),D(p['liquidity']))
 return sorted(rank,key=lambda m:(-rank[m],m))
def onchain(rows,bundle):
 pools=bundle['pools'];rounds=bundle['chain_rounds']
 for r in rows:
  key=r['pair'];eligible=[p for p in pools if p.get('venue_type')=='AMM' and '|'.join(sorted([p['mint0'],p['mint1']]))==key and p.get('liquidity') is not None and D(p['liquidity'])>=20000 and p.get('created') is not None and bundle['sample_epoch']-int(p['created'])>=7*86400]
  r.update(onchain_gap_bps=None,fee_kind='Not measured',above_fee=None,bin_step_bps=None,deepest_depth=None,shallowest_depth=None,deepest_venue=None,shallowest_venue=None)
  symbols=bundle.get('symbols',{});r['symbol']=' / '.join(symbols.get(m,m[:6]+'…') for m in key.split('|'));r['freeze_flag']='; '.join(f"{symbols.get(m,m[:6])}: {v['freeze']}" for m,v in r['freeze_flags'].items());r['mint_check']='; '.join(f"{symbols.get(m,m[:6])}: {v['reason'] or ('Pass' if v['safe'] else 'Not measured')}" for m,v in r['freeze_flags'].items())
  estimates=[(D(bundle['depths'][p['address']]),p['venue']) for p in eligible if bundle.get('depths',{}).get(p['address']) is not None]
  if estimates:r['deepest_depth'],r['deepest_venue']=max(estimates);r['shallowest_depth'],r['shallowest_venue']=min(estimates)
  observations=[];passes=[];unknown=False
  for a,b in combinations(eligible,2):
   for round_ in rounds:
    x=round_.get(a['address']);y=round_.get(b['address'])
    if not x or not y or abs(x['slot']-y['slot'])>25:unknown=True;continue
    pa=D(x['price_scaled'])/D(10**24);pb=D(y['price_scaled'])/D(10**24)
    if x['mint0']!=key.split('|')[0]:pa=1/pa
    if y['mint0']!=key.split('|')[0]:pb=1/pb
    gap=abs(pa-pb)*20000/(pa+pb);fee=None if x['fee_micro_bps'] is None or y['fee_micro_bps'] is None else D(x['fee_micro_bps']+y['fee_micro_bps'])/1000000
    kind='MinimumFeeOnly' if 'MinimumBaseFee' in [x['fee_kind'],y['fee_kind']] else 'Measured'
    obs=(gap,fee,kind,max([v for v in (x.get('bin_step_bps'),y.get('bin_step_bps')) if v is not None],default=None));observations.append(obs)
    if fee is None:unknown=True
    elif gap>fee:passes.append(obs)
  if observations:
   chosen=max(observations,key=lambda x:x[0])
   r['onchain_gap_bps'],r['combined_fee_bps'],kind,r['bin_step_bps']=chosen
   r['fee_kind']=kind if chosen[1] is not None else 'Not measured';r['above_fee']=chosen[0]>chosen[1] if chosen[1] is not None else None
  if r['failed_step'] in (None,'R5'):
   r['failed_step']=None;r['reason']=None
   if r['above_fee'] is True:pass
   elif not observations or r['combined_fee_bps'] is None:r['reason']='Not measured';r['fee_kind']='Not measured'
   else:r['failed_step']='R5';r['reason']='onchain_fee_not_below_gap'

def possible_cap(count):
 return count>=30

def reserve_token(token,public,skipped):
 if public>=200:
  skipped.append(token);return False
 return True
