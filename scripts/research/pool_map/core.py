"""2026-10-31: Exact, offline pool-map requirements and mint checks."""
from decimal import Decimal, getcontext
import base64, hashlib, random
getcontext().prec = 70
D = Decimal
SOL='So11111111111111111111111111111111111111112'
SPL='TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'
T22='TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb'
EXEMPT={'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v':'USDC: issuer freeze authority is disclosed; mainstream control stablecoin', 'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB':'USDT: issuer freeze authority is disclosed; mainstream control stablecoin'}
def decimal(v):
    if v is None: return None
    try:
        x=D(str(v)); return x if x.is_finite() else None
    except Exception: return None
def pair(a,b): return '|'.join(sorted([SOL if x=='SOL' else x for x in (a,b)]))
def mint_check(account, exempt=False):
    if not account or account.get('owner') not in (SPL,T22): return {'safe':False,'reason':'mint_unreadable','freeze':None,'decimals':None}
    try:
        raw=base64.b64decode(account['data'][0],validate=True)
        if len(raw)<82 or raw[45]!=1: raise ValueError()
        freeze=int.from_bytes(raw[46:50],'little')!=0
        reason='freeze_authority' if freeze and not exempt else None
        if account['owner']==T22:
            if len(raw)>82:
                if len(raw)<166 or raw[165]!=1: raise ValueError()
                i=166
                while i+4<=len(raw):
                    typ=int.from_bytes(raw[i:i+2],'little'); n=int.from_bytes(raw[i+2:i+4],'little'); i+=4
                    if i+n>len(raw): raise ValueError()
                    value=raw[i:i+n]; i+=n
                    if typ in (1,9,12,14) or (typ==6 and value==b'\x02'): reason='token2022_restricted_extension'
                if any(raw[i:]): raise ValueError()
        return {'safe':reason is None,'reason':reason,'freeze':freeze,'decimals':raw[44]}
    except Exception: return {'safe':False,'reason':'mint_unreadable','freeze':None,'decimals':None}
def depth(liquidity,sqrt_price,decimals_quote,quote_usd):
    if quote_usd is None:return None
    return D(liquidity)*D(sqrt_price)/D(2**64)*(D('1.01').sqrt()-1)/D(10**decimals_quote)*D(quote_usd)
def first_failure(pools,now,mints):
    if len(pools)<2:return 'R1','fewer_than_two_pools',[]
    eligible=[p for p in pools if decimal(p.get('liquidity')) is not None and D(p['liquidity'])>=20000]
    if len(eligible)<2:return 'R2','liquidity_unknown' if any(p.get('liquidity') is None for p in pools) else 'liquidity_below_floor',[]
    old=[p for p in eligible if p.get('created') is not None and now-int(p['created'])>=7*86400]
    if len(old)<2:return 'R3','age_unknown' if any(p.get('created') is None for p in eligible) else 'NewPool',[]
    for mint in (old[0]['mint0'],old[0]['mint1']):
        check=mint_check(mints.get(mint),mint in EXEMPT)
        if not check['safe']:return 'R4',check['reason'],old
    return None,None,old
def analyze(bundle):
    pools=bundle['pools']; grouped={}
    for p in pools:grouped.setdefault(pair(p['mint0'],p['mint1']),[]).append(p)
    for key in bundle['controls']:grouped.setdefault(key,[])
    rows=[];new=[]
    for p in pools:
        if p.get('created') is not None and bundle['sample_epoch']-int(p['created'])<7*86400:new.append(p)
    for key,ps in sorted(grouped.items()):
        step,reason,eligible=first_failure(ps,bundle['sample_epoch'],bundle['mints'])
        if step is None and 'onchain_evidence_kind' in bundle and sum('onchain_pool_owner' in p for p in eligible)<2:step,reason='R4','shortlist_not_checked'
        r={'pair':key,'control':key in bundle['controls'],'failed_step':step,'reason':reason,'pool_count':len(ps),'evidence_kind':'Reported','list':None,'turnover':None,'price_disagreement_bps':None,'sample_movement_bps':None,'liquidity_imbalance':None,'combined_fee_bps':None,'near_price_depth_usd_estimate':None,'freeze_flags':{m:mint_check(bundle['mints'].get(m),m in EXEMPT) for m in key.split('|')},'onchain_evidence_kind':'OnChain','exemptions':{m:EXEMPT[m] for m in key.split('|') if m in EXEMPT}}
        if step is None:
            values=[];movement=[]
            for sample in bundle['samples']:
                prices=[]
                for p in eligible:
                    v=decimal(sample.get(p['address']))
                    if v is not None and v>0:prices.append(v if p['mint0']==key.split('|')[0] else 1/v)
                if len(prices)>=2:values.append((max(prices)-min(prices))*20000/(max(prices)+min(prices)))
            for p in eligible:
                prices=[decimal(s.get(p['address'])) for s in bundle['samples']]
                prices=[v for v in prices if v is not None and v>0]
                if len(prices)==3:movement.append((max(prices)-min(prices))*20000/(max(prices)+min(prices)))
            fees=sorted(D(p['fee_bps']) for p in eligible if p.get('fee_bps') is not None)
            r['combined_fee_bps']=sum(fees[:2]) if len(fees)>=2 else None
            r['price_disagreement_bps']=max(values) if len(values)==3 else None
            r['sample_movement_bps']=max(movement) if movement else None
            liq=sorted([D(p['liquidity']) for p in eligible],reverse=True)
            r['liquidity']=liq[0];r['liquidity_imbalance']=liq[0]/liq[1]
            volumes=[decimal(p.get('volume')) for p in eligible]
            r['turnover']=sum(volumes)/sum(D(p['liquidity']) for p in eligible) if all(v is not None for v in volumes) else None
            estimates=[decimal(bundle.get('depths',{}).get(p['address'])) for p in eligible];estimates=[v for v in estimates if v is not None]
            r['near_price_depth_usd_estimate']=min(estimates) if estimates else None
            from itertools import combinations
            measured_pass=False;unknown_pair=False
            for a,b in combinations(eligible,2):
                gaps=[]
                for sample in bundle['samples']:
                    pa=decimal(sample.get(a['address']));pb=decimal(sample.get(b['address']))
                    if pa is None or pb is None or pa<=0 or pb<=0:continue
                    if a['mint0']!=key.split('|')[0]:pa=1/pa
                    if b['mint0']!=key.split('|')[0]:pb=1/pb
                    gaps.append(abs(pa-pb)*20000/(pa+pb))
                if a.get('fee_bps') is None or b.get('fee_bps') is None or len(gaps)!=3:
                    unknown_pair=True;continue
                if max(gaps)>D(a['fee_bps'])+D(b['fee_bps']):measured_pass=True
            if not measured_pass:
                if unknown_pair:r['reason']='Not measured'
                else:r['failed_step']='R5';r['reason']='fees_not_below_gap'
        if r['control']:r['list']='Control'
        rows.append(r)
    survivors=[r for r in rows if not r['failed_step'] and not r['control']]
    liqs=sorted(r['liquidity'] for r in survivors)
    median=liqs[(len(liqs)-1)//2] if liqs else None
    candidates=sorted([r for r in survivors if r['liquidity']>=median and r['turnover'] is not None],key=lambda r:(r['turnover'],r['pair']))[:30]
    for r in candidates:r['list']='Candidates'
    rest=sorted([r for r in survivors if r not in candidates],key=lambda r:r['pair'])
    random.Random(int(hashlib.sha256(bundle['run_id'].encode()).hexdigest(),16)).shuffle(rest)
    for r in rest[:10]:r['list']='Comparison'
    return {'run_id':bundle['run_id'],'evidence_kind':'Reported','pairs':rows,'new_pools':sorted(new,key=lambda p:p['address']),'median_liquidity':median}
