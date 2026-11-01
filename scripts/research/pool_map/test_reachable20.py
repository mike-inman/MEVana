"""2026-11-01: Synthetic boundaries plus unmodified public account layouts."""
import copy,json,unittest
from pathlib import Path
from unittest.mock import patch
from dlmm import *
from reachable20 import preflight,SEED,outcome,encoded
from fixes import onchain

class Tests(unittest.TestCase):
 def pool(self):
  return dict(active_id=0,status=0,pair_type=0,token_mint_x_program_flag=0,token_mint_y_program_flag=0,bin_step=10,
   parameters=dict(base_factor=100,base_fee_power_factor=0,variable_fee_control=100000,max_volatility_accumulator=1000000,filter_period=10,decay_period=100,reduction_factor=5000,collect_fee_mode=0,function_type=0),
   v_parameters=dict(volatility_accumulator=20000,volatility_reference=10000,index_reference=0,last_update_timestamp=100))
 def bin(self,x=0,y=0,price=Q):return dict(amount_x=x,amount_y=y,price=price,open_order_amount=0,processed_order_remaining_amount=0)
 def test_idl_offsets_and_real_pool(self):
  self.assertEqual(IDL['metadata']['version'],'0.12.0')
  fields,size,_=layout('LbPair');self.assertEqual(size+8,904)
  self.assertEqual(fields['v_parameters'][0]+layout('VariableParameters')[0]['last_update_timestamp'][0]+8,56)
  self.assertEqual(fields['active_id'][0]+8,76);self.assertEqual(fields['bin_step'][0]+8,80)
  self.assertEqual(layout('BinArray')[0]['bins'][0]+8,56);self.assertEqual(layout('Bin')[1],144);self.assertEqual(layout('BinArray')[1]+8,10136)
  fixture=Path(__file__).resolve().parents[3]/'crates/gap-survey/tests/fixtures/meteora_sol_usdc.bin'
  p=decode_bytes(fixture.read_bytes(),'LbPair');self.assertEqual(p['active_id'],-5865);self.assertEqual(p['bin_step'],4)
  self.assertGreater(p['v_parameters']['last_update_timestamp'],1700000000)
  with self.assertRaises(MissingState):decode_bytes(fixture.read_bytes()[:-1],'LbPair')
 def test_h20_real_account_fixtures(self):
  import hashlib,struct
  root=Path(__file__).parent/'fixtures'
  for record in json.loads((root/'h20-public-accounts.json').read_text()):
   raw=(root/record['file']).read_bytes();self.assertEqual(hashlib.sha256(raw).hexdigest(),record['sha256'])
   decoded=decode_bytes(raw,record['kind'])
   if record['kind']=='LbPair':
    self.assertEqual(decoded['active_id'],record['active_id']);self.assertEqual(decoded['active_id'],struct.unpack_from('<i',raw,76)[0])
    self.assertEqual(decoded['v_parameters']['last_update_timestamp'],struct.unpack_from('<q',raw,56)[0])
   else:
    self.assertEqual(decoded['index'],record['index']);self.assertEqual(decoded['lb_pair'],record['lb_pair']);self.assertEqual(len(decoded['bins']),70)
    self.assertEqual(decoded['bins'][0]['amount_x'],struct.unpack_from('<Q',raw,56)[0])
    self.assertEqual(decoded['bins'][0]['amount_y'],struct.unpack_from('<Q',raw,64)[0])
    self.assertTrue(any(b['amount_x'] or b['amount_y'] for b in decoded['bins']))
 @unittest.skipUnless((SEED.parent.parent/'sample-20261101-20/result.json').exists(),'local saved run not installed')
 def test_saved_run_replay_bytes(self):
  from reachable20 import replay,DATA,RUN
  root=DATA/RUN
  self.assertEqual(encoded(replay(root)),(root/'result.json').read_bytes())
 def test_real_jup_met_fee_example(self):
  p=decode_bytes((Path(__file__).parent/'fixtures/h20-lb-pair.bin').read_bytes(),'LbPair')
  self.assertEqual(p['bin_step'],50);self.assertEqual(p['parameters']['base_factor'],10000)
  # After the 1,200-second decay: reference resets to active 18. At liquid bin
  # 17, vol=10,000. base=10,000*50*10=5,000,000;
  # variable=ceil(10,000*(10,000*50)^2/1e11)=25,000.
  v=fee_reference(p,p['v_parameters']['last_update_timestamp']+1200)
  self.assertEqual(fee_rate(p,v,17),(5025000,25000,10000))
 def test_hand_fees_and_decay(self):
  p=self.pool();v=fee_reference(p,105);rate,var,vol=fee_rate(p,v,0)
  # base=100*10*10=10000; variable=ceil(100000*(10000*10)^2/1e11)=10000.
  self.assertEqual((rate,var,vol),(20000,10000,10000))
  v=fee_reference(p,110);self.assertEqual(v['volatility_reference'],10000)
  v=fee_reference(p,200);self.assertEqual(fee_rate(p,v,0),(10000,0,0))
  self.assertEqual(fee_rate(p,v,1),(20000,10000,10000))
 def test_one_sided_empty_and_multi_bin(self):
  p=self.pool();p['parameters']['base_factor']=0;p['parameters']['variable_fee_control']=0
  bins={0:self.bin(x=500),-1:self.bin(),-2:self.bin(y=40),-3:self.bin(y=60),-4:self.bin(y=100)}
  self.assertEqual(nearest(p,bins,True),2);self.assertEqual(nearest(p,bins,False),0)
  q=walk(p,bins,150,True,200);self.assertEqual(q['output_atoms'],150);self.assertEqual([b['bin_id'] for b in q['bins']],[-2,-3,-4])
  with self.assertRaises(MissingState):walk(p,bins,201,True,200)
  self.assertEqual(walk(p,bins,100,False,200)['output_atoms'],100)
 def test_permissionless_versions_and_activation(self):
  p=self.pool();bins={0:self.bin(x=1000,y=1000)}
  for typ in (0,2,3):
   p['pair_type']=typ;self.assertGreater(walk(p,bins,100,True,200)['output_atoms'],0)
  p['pair_type']=1
  with self.assertRaises(MissingState):walk(p,bins,100,True,200)
  p.update(pair_type=3,activation_point=501,activation_type=0)
  with self.assertRaises(MissingState):walk(p,bins,100,True,200,slot=500)
  self.assertGreater(walk(p,bins,100,True,200,slot=501)['output_atoms'],0)
 def test_fee_rounding_and_only_y(self):
  p=self.pool();p['parameters'].update(base_factor=10000,variable_fee_control=0,collect_fee_mode=1)
  q=walk(p,{0:self.bin(y=100000,x=100000)},1000,True,200)
  self.assertEqual(q['output_atoms'],999);self.assertEqual(q['fee_token'],'output')
  q=walk(p,{0:self.bin(y=100000,x=100000)},1000,False,200)
  self.assertEqual(q['output_atoms'],999);self.assertEqual(q['fee_token'],'input')
 def test_reachable_exact_boundaries(self):
  self.assertEqual(outcome(100,101,False,[100,125]),'Reachable')
  self.assertEqual(outcome(100,100,False,[100,125]),'NotReachable')
  self.assertEqual(outcome(100,99,False,[100,125]),'NotReachable')
  self.assertEqual(outcome(100,101,False,[100,126]),'Not measured')
  self.assertEqual(outcome(100,101,True,[100,125]),'Indicative')
  self.assertEqual(outcome(100,None,False,[100]),'Not measured')
 @unittest.skipUnless(SEED.exists(),'local saved seed not installed')
 def test_budget_before_reader(self):
  seed=json.loads(SEED.read_text());p=preflight(seed);self.assertLessEqual(p['rpc_upper_bound'],15)
  with patch('subprocess.run') as call:
   with self.assertRaisesRegex(RuntimeError,'preflight'):preflight(seed,array_cap=1000)
   call.assert_not_called()
 def test_r5_smaller_cheaper_gap(self):
  pools=[dict(address=k,mint0='A',mint1='B',liquidity='20000',created=0,venue='Meteora',venue_type='AMM') for k in 'abc']
  # a-b largest gap but expensive; a-c smaller gap and cheap.
  prices={'a':(10000,0),'b':(10100,200),'c':(10050,0)}
  states={k:dict(price_scaled=str(v[0]*10**20),mint0='A',slot=100,fee_micro_bps=v[1]*1000000,fee_kind='Measured',bin_step_bps=10) for k,v in prices.items()}
  row=dict(pair='A|B',freeze_flags={},failed_step=None)
  onchain([row],dict(pools=pools,chain_rounds=[states],sample_epoch=7*86400,depths={}))
  self.assertIsNone(row['failed_step']);self.assertTrue(row['above_fee']);self.assertEqual(row['compared_pool_ids'],['a','c'])
 def test_unknown_limit_orders(self):
  b=self.bin(y=10);b['open_order_amount']=1
  with self.assertRaises(MissingState):liquidity(b,True)
 @unittest.skipUnless(SEED.exists(),'local saved seed not installed')
 def test_plan_bytes_stable(self):
  seed=json.loads(SEED.read_text());self.assertEqual(encoded(preflight(seed)),encoded(preflight(seed)))
if __name__=='__main__':unittest.main()
