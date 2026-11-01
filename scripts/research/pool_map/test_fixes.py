"""2026-11-01: Synthetic 19a boundary tests."""
import unittest
from fixes import *
from collect19a import Limits
class Tests(unittest.TestCase):
 def test_merge_and_tokens(self):
  ps={};p=dict(address='p',mint0='a',mint1='b',liquidity='20000',venue='pump-fun');merge(ps,[p,p]);self.assertEqual(len(ps),1);self.assertEqual(ps['p']['venue_type'],'BondingCurve');self.assertEqual(token_order(list(ps.values()),[],{'b'}),['a'])
 def test_slots_reverse_and_fees(self):
  pools=[dict(address='a',mint0='A',mint1='B',liquidity='20000',created=0,venue='Orca',venue_type='AMM'),dict(address='b',mint0='B',mint1='A',liquidity='20000',created=0,venue='Meteora',venue_type='AMM')]
  x=dict(price_scaled=str(2*10**24),mint0='A',slot=100,fee_micro_bps=1000000,fee_kind='Measured',bin_step_bps=None)
  y=dict(price_scaled=str(10**24//2),mint0='B',slot=125,fee_micro_bps=1000000,fee_kind='MinimumBaseFee',bin_step_bps=10)
  bundle=dict(pools=pools,chain_rounds=[{'a':x,'b':y}],sample_epoch=7*86400,depths={'a':'100','b':'10'})
  def row():return dict(pair='A|B',freeze_flags={},failed_step=None)
  r=row();onchain([r],bundle);self.assertEqual(r['onchain_gap_bps'],0);self.assertEqual(r['fee_kind'],'MinimumFeeOnly');self.assertEqual(r['deepest_venue'],'Orca');self.assertEqual(r['shallowest_venue'],'Meteora')
  y['slot']=126;r=row();onchain([r],bundle);self.assertIsNone(r['onchain_gap_bps']);self.assertEqual(r['fee_kind'],'Not measured')
  y.update(slot=100,price_scaled=str(10**24//3),fee_kind='Measured');r=row();onchain([r],bundle);self.assertEqual(r['fee_kind'],'Measured');self.assertTrue(r['above_fee'])
 def test_bonding_pool_does_not_satisfy_r1(self):
  from core import analyze
  p=dict(address='a',mint0='A',mint1='B',liquidity='20000',created=0,venue='pumpfun',venue_type='BondingCurve')
  q=dict(p,address='b',venue='Orca',venue_type='AMM')
  b=dict(version='19a',run_id='test',pools=[p,q],controls=['A|B'],sample_epoch=7*86400,mints={},samples=[{}, {}, {}],depths={},chain_rounds=[])
  self.assertEqual(analyze(b)['pairs'][0]['failed_step'],'R1')
 def test_limits_and_skipped_token(self):
  b=Limits();b.public=220
  with self.assertRaises(RuntimeError):b.check('public')
  b.rpc=10
  with self.assertRaises(RuntimeError):b.check('rpc')
  self.assertFalse(possible_cap(29));self.assertTrue(possible_cap(30))
  skipped=[]
  self.assertFalse(reserve_token('token',b.public,skipped))
  self.assertEqual(skipped,['token'])
if __name__=='__main__':unittest.main()
