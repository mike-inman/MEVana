import { useEffect, useRef, useState } from 'react'
import { useStore } from '../state/useStore'

export function WsStatus(){
  const [ok, setOk] = useState(false)
  const retryRef = useRef<number | null>(null)
  const useV2 = useStore(s=>s.useV2)
  const setUseV2 = useStore(s=>s.setUseV2)

  useEffect(() => {
    let ws: WebSocket | null = null
    let stopped = false

    const connect = () => {
      if (stopped) return
      const url = 'ws://127.0.0.1:8081/ws/ping' // try 127.0.0.1 to dodge any IPv6 oddities
      console.log('[WS] connecting', url)
      ws = new WebSocket(url)

      ws.onopen = () => { console.log('[WS] open'); setOk(true) }
      ws.onmessage = (e) => { /* 'ok' heartbeat every 3s */ }
      ws.onerror = (e) => { console.warn('[WS] error', e) }
      ws.onclose = () => {
        console.warn('[WS] closed')
        setOk(false)
        retryRef.current = window.setTimeout(connect, 1500)
      }
    }

    connect()
    return () => {
      stopped = true
      if (retryRef.current) clearTimeout(retryRef.current)
      ws?.close()
    }
  }, [])

  return (
    <div className="flex items-center gap-2">
      <div className={`px-2 py-1 rounded-full text-xs ${ok?'bg-green-500/20 text-green-300':'bg-red-500/20 text-red-300'}`}>
        {ok ? 'live' : 'offline'}
      </div>
      <button
        className="px-2 py-1 rounded text-xs bg-[var(--panel-2)] hover:opacity-80"
        onClick={()=>setUseV2(!useV2)}
        title="Toggle swaps source"
      >{useV2? 'v2' : 'v1'}</button>
    </div>
  )
}
