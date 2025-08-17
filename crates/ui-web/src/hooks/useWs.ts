import { useEffect } from 'react'
import { useStore } from '../state/useStore'
import { startMockFeed } from '../mock/mockFeed'

export function useWS() {
  const upsertPrice = useStore((s) => s.upsertPrice)
  const pushSwap = useStore((s) => s.pushSwap)
  const useV2 = useStore((s) => s.useV2)
  const mock = (import.meta.env as any)["VITE_MOCK"] === '1'
  const allowFallback = (import.meta.env as any)["VITE_ALLOW_FALLBACK"] === '1'

  useEffect(() => {
    if (mock) {
      // explicit mock mode
      return startMockFeed(upsertPrice, pushSwap)
    }

    // real WS with fallback-to-mock if connect fails quickly
    let usingMock = false
    let stopMock: (() => void) | null = null
    const startFallback = () => {
      if (!allowFallback) { console.warn('[WS] fallback disabled'); return }
      if (!usingMock) {
        usingMock = true
        stopMock = startMockFeed(upsertPrice, pushSwap)
      }
    }

    const base = 'ws://127.0.0.1:8081'
    const priceWS = new WebSocket(base + '/ws/prices')
    const swapsPath = useV2 ? '/ws/swaps.v2' : '/ws/swaps'
    const swapWS = new WebSocket(base + swapsPath)

    const failTimer = setTimeout(startFallback, 5000)

    priceWS.onopen = () => { console.log('[WS] prices open'); clearTimeout(failTimer) }
    swapWS.onopen = () => { console.log('[WS] swaps open'); clearTimeout(failTimer) }

    priceWS.onmessage = (e) => {
      try {
        upsertPrice(JSON.parse(e.data))
      } catch {}
    }
    swapWS.onmessage = (e) => {
      try {
        pushSwap(JSON.parse(e.data))
      } catch {}
    }

    priceWS.onerror = () => { console.warn('[WS] prices error'); startFallback() }
    swapWS.onerror = () => { console.warn('[WS] swaps error'); startFallback() }

    return () => {
      clearTimeout(failTimer)
      try { priceWS.close() } catch {}
      try { swapWS.close() } catch {}
      if (stopMock) stopMock()
    }
  }, [upsertPrice, pushSwap, mock, useV2])
}
