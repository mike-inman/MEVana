import { useWS } from '../hooks/useWs'
import { useStore } from '../state/useStore'
import { PriceCard } from './PriceCard'

export function PricesPanel() {
  useWS()
  const entries = Object.entries(useStore((s) => s.prices))
  return (
    <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-6 gap-2">
      {entries.map(([key, p]) => (
        <PriceCard key={key} p={p} />
      ))}
    </div>
  )
}
