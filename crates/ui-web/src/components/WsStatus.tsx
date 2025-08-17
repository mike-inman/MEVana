import { useEffect, useState } from 'react'

export function WsStatus() {
  const [ok, setOk] = useState(false)
  const mock = import.meta.env.VITE_MOCK === '1'

  useEffect(() => {
    if (mock) return
    const ws = new WebSocket('ws://localhost:8081/ws/ping')
    ws.onopen = () => setOk(true)
    ws.onclose = () => setOk(false)
    return () => ws.close()
  }, [mock])

  if (mock) {
    return (
      <div className="px-2 py-1 rounded-full text-xs bg-sky-500/20 text-sky-300">
        mock mode
      </div>
    )
  }

  return (
    <div
      className={`px-2 py-1 rounded-full text-xs ${
        ok ? 'bg-green-500/20 text-green-300' : 'bg-red-500/20 text-red-300'
      }`}
    >
      {ok ? 'live' : 'offline'}
    </div>
  )
}
