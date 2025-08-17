import type { PriceSnapshot, NormalizedSwap } from '../state/useStore'

const pairs = [
  { dex: 'Orca', pair: 'SOL/USDC', price: 175.23 },
  { dex: 'Orca', pair: 'BONK/USDC', price: 0.000021 },
  { dex: 'Orca', pair: 'WIF/USDC', price: 3.14 },
  { dex: 'Orca', pair: 'JUP/USDC', price: 1.05 },
  { dex: 'Orca', pair: 'PYTH/USDC', price: 0.45 },
] as const

function jitter(x: number, pct = 0.004) {
  const r = (Math.random() * 2 - 1) * pct
  return Math.max(0, x * (1 + r))
}

export function startMockFeed(
  upsertPrice: (p: PriceSnapshot) => void,
  pushSwap: (s: NormalizedSwap) => void
) {
  let seed = pairs.map((p) => ({ ...p }))

  const priceTimer = setInterval(() => {
    const ts = Math.floor(Date.now() / 1000)
    seed = seed.map((p) => ({ ...p, price: jitter(p.price) }))
    for (const p of seed) {
      const snap: PriceSnapshot = {
        ts,
        dex: p.dex,
        pair: p.pair,
        price: p.price,
        src_sig: 'mock_sig_' + Math.random().toString(36).slice(2, 8),
        window_secs: 5,
      }
      upsertPrice(snap)
    }
    // occasionally emit a swap
    if (Math.random() < 0.7) {
      const pick = seed[Math.floor(Math.random() * seed.length)]
      const amtIn = +(Math.random() * 5 + 0.1).toFixed(4)
      const amtOut = +(amtIn * pick.price).toFixed(4)
      const swap: NormalizedSwap = {
        ts,
        slot: 0,
        sig: 'mock_tx_' + Math.random().toString(36).slice(2, 10),
        dex: pick.dex.toLowerCase(),
        token_in: pick.pair.split('/')[0],
        amount_in: amtIn,
        token_out: pick.pair.split('/')[1],
        amount_out: amtOut,
      }
      pushSwap(swap)
    }
  }, 700)

  return () => clearInterval(priceTimer)
}
