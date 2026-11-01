"""2026-11-01: Read-only DLMM model; pinned IDL and SDK 2f540c21.

Amounts and fees are integers. This module has no transport or trading authority.
C layout follows Solana's 8-byte u128 alignment, including the explicit IDL padding.
"""
import base64, hashlib, json
from pathlib import Path

IDL=json.loads((Path(__file__).parent/'fixtures/dlmm-idl-types.json').read_text())
TYPES={t['name']:t['type'] for t in IDL['types']}
METEORA='LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo'
ORCA='whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc'
Q=1<<64
FEE_PRECISION=10**9
ALPHABET='123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz'
class MissingState(ValueError):pass

def b58(data):
 n=int.from_bytes(data,'big');s=''
 while n:n,r=divmod(n,58);s=ALPHABET[r]+s
 return '1'*(len(data)-len(data.lstrip(b'\0')))+s

def un58(s):
 n=0
 for c in s:n=n*58+ALPHABET.index(c)
 return b'\0'*(len(s)-len(s.lstrip('1')))+(n.to_bytes((n.bit_length()+7)//8,'big') if n else b'')

def on_curve(raw):
 # Same compressed Edwards-Y validity predicate needed for public PDA derivation.
 p=2**255-19;y=int.from_bytes(raw,'little')&((1<<255)-1)
 if y>=p:return False
 d=-121665*pow(121666,p-2,p)%p
 u=(y*y-1)%p;v=(d*y*y+1)%p
 if v==0:return False
 x2=u*pow(v,p-2,p)%p
 return x2==0 or pow(x2,(p-1)//2,p)==1

def bin_address(pool,index):
 key=un58(pool);program=un58(METEORA)
 if len(key)!=32:raise ValueError('invalid pool pubkey')
 for bump in range(255,-1,-1):
  h=hashlib.sha256(b'bin_array'+key+index.to_bytes(8,'little',signed=True)+bytes([bump])+program+b'ProgramDerivedAddress').digest()
  if not on_curve(h):return b58(h)
 raise ValueError('PDA derivation failed')

def shape(t):
 if isinstance(t,str):
  if t=='pubkey':return 32,1
  size=int(t[1:])//8;return size,min(size,8)
 if 'array' in t:
  a,n=t['array'];size,align=shape(a);return size*n,align
 return layout(t['defined']['name'])[1:]

def layout(name):
 fields={};offset=0;align=1
 for f in TYPES[name]['fields']:
  size,a=shape(f['type']);offset=(offset+a-1)//a*a
  fields[f['name']]=(offset,f['type']);offset+=size;align=max(align,a)
 return fields,(offset+align-1)//align*align,align

def read(raw,t,offset):
 if isinstance(t,str):
  size,_=shape(t);v=raw[offset:offset+size]
  if len(v)!=size:raise MissingState('truncated field')
  return b58(v) if t=='pubkey' else int.from_bytes(v,'little',signed=t.startswith('i'))
 if 'array' in t:
  a,n=t['array'];size,_=shape(a);return [read(raw,a,offset+i*size) for i in range(n)]
 name=t['defined']['name'];fields,_,_=layout(name)
 return {k:read(raw,typ,offset+off) for k,(off,typ) in fields.items()}

def decode_bytes(raw,name):
 disc=next(a['discriminator'] for a in IDL['accounts'] if a['name']==name)
 if len(raw)!=layout(name)[1]+8 or raw[:8]!=bytes(disc):raise MissingState('account length/discriminator mismatch')
 return read(raw,{'defined':{'name':name}},8)

def decode_account(account,name):
 if not account or account.get('owner')!=METEORA or account.get('executable'):raise MissingState('missing or wrong-owner account')
 return decode_bytes(base64.b64decode(account['data'][0],validate=True),name)

def bin_indices(pool,each_side=12):
 active=pool['active_id']//70;bitmap=sum(v<<(64*i) for i,v in enumerate(pool['bin_array_bitmap']))
 if not -512<=active<512:raise MissingState('bitmap extension required')
 initialized=[i for i in range(-512,512) if bitmap>>(i+512)&1]
 return sorted({active,*[i for i in initialized if i<active][-each_side:],*[i for i in initialized if i>active][:each_side]})

def bins_from_arrays(pool_key,pool,accounts):
 bins={};bitmap=sum(v<<(64*i) for i,v in enumerate(pool['bin_array_bitmap']))
 for index in bin_indices(pool):
  addr=bin_address(pool_key,index);account=accounts.get(addr)
  if account is None:
   # A requested, absent array is only known empty when its bitmap bit is clear.
   if bitmap>>(index+512)&1:continue
   for j in range(70):bins[index*70+j]=None
   continue
  arr=decode_account(account,'BinArray')
  if arr['index']!=index or arr['lb_pair']!=pool_key or arr['version']>3:raise MissingState('array identity/version mismatch')
  for j,b in enumerate(arr['bins']):bins[index*70+j]=b
 # Uninitialized internal arrays are proven empty, never unknown initialized arrays.
 for index in range(-512,512):
  if not (bitmap>>(index+512)&1):
   for j in range(70):bins.setdefault(index*70+j,None)
 return bins

def fee_reference(pool,now):
 s=pool['parameters'];v=dict(pool['v_parameters']);elapsed=now-v['last_update_timestamp']
 if elapsed<0:raise MissingState('future variable timestamp')
 if elapsed>=s['filter_period']:
  v['index_reference']=pool['active_id']
  v['volatility_reference']=v['volatility_accumulator']*s['reduction_factor']//10000 if elapsed<s['decay_period'] else 0
 return v

def fee_rate(pool,v,bin_id):
 s=pool['parameters'];vol=min(v['volatility_reference']+abs(v['index_reference']-bin_id)*10000,s['max_volatility_accumulator'])
 base=s['base_factor']*pool['bin_step']*10*10**s['base_fee_power_factor']
 variable=ceildiv(s['variable_fee_control']*(vol*pool['bin_step'])**2,100_000_000_000)
 return min(base+variable,100_000_000),variable,vol

def ceildiv(a,b):return (a+b-1)//b

def liquidity(bin_,swap_y):
 if bin_ is None:return 0
 # Limit-order features need separate exact fill rounding. Fail closed rather than
 # silently counting only MM reserves as total available liquidity.
 if bin_['open_order_amount'] or bin_['processed_order_remaining_amount']:
  raise MissingState('limit-order liquidity unsupported')
 return bin_['amount_y'] if swap_y else bin_['amount_x']

def nearest(pool,bins,swap_y):
 active=pool['active_id'];step=-1 if swap_y else 1
 for distance in range(71680):
  i=active+step*distance
  if i not in bins:return None
  if liquidity(bins[i],swap_y)>0:return distance
 return None

def walk(pool,bins,amount,swap_y,now,slot=None):
 if amount<=0:raise MissingState('zero input')
 s=pool['parameters']
 activation=pool.get('activation_point',0)
 if activation:
  point=now if pool.get('activation_type')==1 else slot if pool.get('activation_type')==0 else None
  if point is None or point<activation:raise MissingState('pool not yet activated or activation unknown')
 if pool['status']!=0 or pool['pair_type'] not in (0,2,3) or pool['token_mint_x_program_flag'] or pool['token_mint_y_program_flag']:
  raise MissingState('disabled, permissioned or Token-2022 pool unsupported')
 if s['collect_fee_mode'] not in (0,1) or s['function_type'] not in (0,1,2):raise MissingState('unknown fee/function mode')
 v=fee_reference(pool,now);remaining=amount;out=0;fees=0;used=[];i=pool['active_id'];step=-1 if swap_y else 1
 fee_on_input=s['collect_fee_mode']==0 or not swap_y
 while remaining:
  if i not in bins:raise MissingState('liquidity outside observed arrays')
  b=bins[i];reserve=liquidity(b,swap_y)
  if reserve:
   price=b['price']
   if not price:raise MissingState('zero price in liquid bin')
   rate,variable,vol=fee_rate(pool,v,i)
   net=remaining-ceildiv(remaining*rate,FEE_PRECISION) if fee_on_input else remaining
   needed=ceildiv(reserve*Q,price) if swap_y else ceildiv(reserve*price,Q)
   consumed=min(net,needed)
   gross=ceildiv(consumed*FEE_PRECISION,FEE_PRECISION-rate) if fee_on_input and consumed<net else remaining
   if not fee_on_input:gross=consumed
   output=reserve if consumed>=needed else (consumed*price//Q if swap_y else consumed*Q//price)
   fee=gross-consumed if fee_on_input else ceildiv(output*rate,FEE_PRECISION)
   if not fee_on_input:output-=fee
   if gross<=0 or gross>remaining:raise MissingState('invalid fill')
   remaining-=gross;out+=output;fees+=fee
   used.append(dict(bin_id=i,input_atoms=gross,output_atoms=output,fee_atoms=fee,fee_rate=rate,variable_rate=variable,volatility_accumulator=vol))
  i+=step
  if len(used)>10000:raise MissingState('walk bound')
 return dict(input_atoms=amount,output_atoms=out,fee_atoms=fees,fee_token='input' if fee_on_input else 'output',bins=used,evidence_kind='Modeled')
