import { create } from 'zustand'

export interface PriceSnapshot {
  ts: number
  dex: string
  pair: string
  price: number
  src_sig: string
  window_secs: number
}

export interface NormalizedSwap {
  ts: number
  slot: number
  sig: string
  dex: string
  token_in?: string
  amount_in?: number
  token_out?: string
  amount_out?: number
}

type State = {
  prices: Record<string, PriceSnapshot>
  priceHistory: Record<string, number[]>
  swaps: NormalizedSwap[]
  useV2: boolean
  upsertPrice: (p: PriceSnapshot) => void
  pushSwap: (s: NormalizedSwap) => void
  setUseV2: (v: boolean) => void
}

export const useStore = create<State>((set, get) => ({
  prices: {},
  priceHistory: {},
  swaps: [],
  useV2: true,
  upsertPrice: (p) =>
    set((st) => {
      const key = `${p.dex}:${p.pair}`
      const hist = st.priceHistory[key] ?? []
      const next = [...hist.slice(-59), p.price]
      return {
        prices: { ...st.prices, [key]: p },
        priceHistory: { ...st.priceHistory, [key]: next },
      }
    }),
  pushSwap: (s) => set((st) => ({ swaps: [s, ...st.swaps].slice(0, 500) })),
  setUseV2: (v) => set(() => ({ useV2: v })),
}))
