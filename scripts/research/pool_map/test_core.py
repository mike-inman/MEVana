"""2026-10-31: Synthetic offline boundary fixtures; no market evidence."""
import unittest,base64,time,json
from core import *
from sample import Budget,Source
class Tests(unittest.TestCase):
 def account(self,freeze=False,ext=None):
  raw=bytearray(82);raw[45]=1;raw[44]=6;raw[46:50]=(int(freeze)).to_bytes(4,'little')
  if ext is not None:
   raw.extend(bytes(83));raw.append(1);typ,value=ext;raw.extend(typ.to_bytes(2,'little')+len(value).to_bytes(2,'little')+value)
  return {'owner':T22 if ext else SPL,'data':[base64.b64encode(raw).decode(),'base64']}
 def pool(self,**kw):
  p=dict(mint0='a',mint1='b',liquidity='20000',created=0,address='p',fee_bps=None,volume='100');p.update(kw);return p
 def test_grouping(self):self.assertEqual(pair('SOL','b'),pair('b',SOL))
 def test_requirements(self):
  now=7*86400;p=self.pool();q=self.pool(address='q');mint={'a':self.account(),'b':self.account()}
  self.assertEqual(first_failure([p],now,mint)[:2],('R1','fewer_than_two_pools'))
  self.assertEqual(first_failure([p,self.pool(liquidity=None)],now,mint)[1],'liquidity_unknown')
  self.assertEqual(first_failure([p,q],now-1,mint)[0],'R3')
  self.assertEqual(first_failure([p,q],now,mint)[0],None)
  self.assertEqual(first_failure([p,q],now,{})[0],'R4')
 def test_mint_fixtures(self):
  self.assertTrue(mint_check(self.account())['safe'])
  self.assertFalse(mint_check(self.account(True))['safe'])
  self.assertTrue(mint_check(self.account(True),True)['safe'])
  for typ,v in [(1,b'\0'*108),(6,b'\2'),(9,b''),(12,b'\1'*32),(14,b'\1'*64)]:self.assertFalse(mint_check(self.account(ext=(typ,v)))['safe'])
 def test_depth(self):self.assertEqual(depth(1000,2**64,0,'2'),D(2000)*(D('1.01').sqrt()-1))
 def test_limits(self):
  b=Budget();b.public=300
  with self.assertRaises(RuntimeError):b.check('public')
  b.rpc=20
  with self.assertRaises(RuntimeError):b.check('rpc')
  with self.assertRaises(RuntimeError):Budget(time.monotonic()-1200).check()
 def test_determinism_unknowns(self):
  bundle={'run_id':'fixed','pools':[self.pool(),self.pool(address='q')],'controls':[],'sample_epoch':7*86400,'mints':{'a':self.account(),'b':self.account()},'samples':[{}, {}, {}]}
  a=analyze(bundle);self.assertEqual(a,analyze(bundle));self.assertIsNone(a['pairs'][0]['combined_fee_bps']);self.assertEqual(a['pairs'][0]['reason'],'Not measured')
 def test_r5_boundary_and_first_failure(self):
  p=self.pool(fee_bps='0');q=self.pool(address='q',fee_bps='0')
  b={'run_id':'fixed','pools':[p,q],'controls':[],'sample_epoch':7*86400,'mints':{'a':self.account(),'b':self.account()},'samples':[{'p':'1','q':'1'}]*3}
  self.assertEqual(analyze(b)['pairs'][0]['failed_step'],'R5')
  b['pools'][1]['liquidity']='19999.999'
  self.assertEqual(analyze(b)['pairs'][0]['failed_step'],'R2')
 def test_429_retry_cap(self):
  import tempfile,urllib.error
  from unittest.mock import patch
  from pathlib import Path
  with tempfile.TemporaryDirectory() as directory:
   b=Budget();b.wait=lambda seconds:None
   source=Source(Path(directory),b)
   with patch('urllib.request.urlopen',side_effect=urllib.error.HTTPError('https://api.dexscreener.com',429,'limit',{},None)):
    self.assertIsNone(source.get('https://api.dexscreener.com/test'))
   self.assertEqual(b.public,3);self.assertEqual(len(source.partial),1)
 def test_source_has_no_execution_or_stream_path(self):
  from pathlib import Path
  source=Path(__file__).with_name('sample.py').read_text()
  for forbidden in ['send_transaction','sign_transaction','SubscribeRequest','GrpcSource','wallet-vault']:
   self.assertNotIn(forbidden,source)
if __name__=='__main__':unittest.main()
