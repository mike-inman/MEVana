"""2026-11-01: Synthetic all-pair planning, unchecked outcomes and bin labels."""
import unittest
from unittest.mock import patch
from collect19b import plan
from core import analyze
from fixes import onchain
from decimal import Decimal as D
class Tests(unittest.TestCase):
 def seed(self,n):
  pools=[]
  for i in range(n):
   for j in range(3):pools.append(dict(address=f'{i}-{j}',mint0=f'A{i}',mint1=f'B{i}',liquidity='20000',created=0,venue_type='AMM',venue='orca'))
  return dict(pools=pools,controls=['A0|B0'])
 def test_no_twenty_pair_cap(self):
  p=plan(self.seed(25),7*86400);self.assertEqual(len(p['planned_pairs']),25);self.assertEqual(len(p['short_addresses']),75)
 def test_preflight_exceeds_limit_before_read(self):
  with patch('sample.Source.rpc') as read:
   with self.assertRaisesRegex(RuntimeError,'no live reads'):plan(self.seed(300),7*86400)
   read.assert_not_called()
 def test_notchecked_is_not_mint_rejection(self):
  seed=self.seed(1);b=dict(seed,version='19b',run_id='test',sample_epoch=7*86400,mints={},planned_pairs=[],mint_requested=[],samples=[{}, {}, {}],depths={},chain_rounds=[])
  r=analyze(b)['pairs'][0];self.assertTrue(r['not_checked']);self.assertIsNone(r['failed_step']);self.assertEqual(r['reason'],'NotChecked')
  b.update(planned_pairs=['A0|B0'],mint_requested=['A0','B0']);r=analyze(b)['pairs'][0];self.assertFalse(r['not_checked']);self.assertEqual(r['failed_step'],'R4');self.assertEqual(r['reason'],'mint_unreadable')
 def test_bin_exact_boundary_does_not_change_r5(self):
  pools=[dict(address=k,mint0='A',mint1='B',liquidity='20000',created=0,venue_type='AMM',venue='Meteora') for k in ['a','b']]
  def result(step):
   x=dict(mint0='A',slot=100,price_scaled=str(10**24),fee_micro_bps=0,fee_kind='MinimumBaseFee',bin_step_bps=step)
   y=dict(x,price_scaled=str(3*10**24));r=dict(pair='A|B',freeze_flags={},failed_step=None)
   onchain([r],dict(pools=pools,chain_rounds=[{'a':x,'b':y}],sample_epoch=7*86400,depths={}));return r
  a=result(10000);b=result(9999);c=result(None)
  self.assertTrue(a['gap_within_bin_step']);self.assertFalse(b['gap_within_bin_step']);self.assertIsNone(c['gap_within_bin_step'])
  self.assertEqual(a['failed_step'],b['failed_step']);self.assertEqual(a['above_fee'],b['above_fee'])
if __name__=='__main__':unittest.main()
