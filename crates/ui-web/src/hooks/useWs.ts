import { useEffect } from 'react'
import { useStore } from '../state/useStore'
import { startMockFeed } from '../mock/mockFeed'

export function useWS() {
  const upsertPrice = useStore((s) => s.upsertPrice)
  const pushSwap = useStore((s) => s.pushSwap)
  const useV2 = useStore((s) => s.useV2)
  const mock = (import.meta.env as any)["VITE_MOCK"] === '1'

  useEffect(() => {
    if (mock) {
      // explicit mock mode
      return startMockFeed(upsertPrice, pushSwap)
    }

    // real WS with fallback-to-mock if connect fails quickly
    let usingMock = false
    let stopMock: (() => void) | null = null
    const startFallback = () => {
      if (!usingMock) {
        usingMock = true
        stopMock = startMockFeed(upsertPrice, pushSwap)
      }
    }

    const priceWS = new WebSocket('ws://localhost:8081/ws/prices')
    const swapsPath = useV2 ? '/ws/swaps.v2' : '/ws/swaps'
    const swapWS = new WebSocket('ws://localhost:8081' + swapsPath)

    const failTimer = setTimeout(startFallback, 2000)

    priceWS.onopen = () => clearTimeout(failTimer)
    swapWS.onopen = () => clearTimeout(failTimer)

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

    priceWS.onerror = startFallback
    swapWS.onerror = startFallback

    return () => {
      clearTimeout(failTimer)
      try { priceWS.close() } catch {}
      try { swapWS.close() } catch {}
      if (stopMock) stopMock()
    }
  }, [upsertPrice, pushSwap, mock, useV2])
}
