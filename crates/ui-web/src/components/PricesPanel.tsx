import { useWS } from '../hooks/useWs'
import { useStore } from '../state/useStore'
import { PriceCard } from './PriceCard'

export function PricesPanel() {
  useWS()
  const entries = Object.entries(useStore((s) => s.prices))
  return (
    <div className="grid grid-cols-2 gap-3">
      {entries.map(([key, p]) => (
        <PriceCard key={key} p={p} />
      ))}
    </div>
  )
}
