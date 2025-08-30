import { PricesPanel } from './components/PricesPanel'
import { SwapsPanel } from './components/SwapsPanel'
import { WsStatus } from './components/WsStatus'
import { PoolPanel } from './components/PoolPanel'

export default function App() {
  return (
    <div className="min-h-dvh grid grid-cols-12 gap-3 p-3 bg-[var(--bg)]">
      <header className="col-span-12 card p-3 flex items-center justify-between">
        <div className="text-sm tracking-wide text-muted">MEVana</div>
        <div className="flex items-center gap-3">
          <span className="text-xs text-muted">Neon&nbsp;Carbon</span>
          <WsStatus />
        </div>
      </header>

      <section className="col-span-12 lg:col-span-7 card p-3 overflow-hidden">
        <PricesPanel />
      </section>

      <aside className="col-span-12 lg:col-span-5 space-y-3 overflow-hidden">
        <div className="card p-3 overflow-hidden"><SwapsPanel /></div>
        <div className="card p-3 overflow-hidden"><PoolPanel /></div>
      </aside>
    </div>
  )
}