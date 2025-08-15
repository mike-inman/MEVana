import { PricesPanel } from './components/PricesPanel'
import { SwapsPanel } from './components/SwapsPanel'
import { WsStatus } from './components/WsStatus'

export default function App() {
  return (
    <div className="min-h-dvh grid grid-cols-12 gap-4 p-4 bg-[var(--bg)]">
      <header className="col-span-12 card p-3 flex items-center justify-between">
        <div className="text-sm tracking-wide text-muted">MEVana</div>
        <div className="flex items-center gap-3">
          <span className="text-xs text-muted">Neon&nbsp;Carbon</span>
          <WsStatus />
        </div>
      </header>

      <section className="col-span-12 lg:col-span-7 card p-3">
        <PricesPanel />
      </section>

      <aside className="col-span-12 lg:col-span-5 card p-3">
        <SwapsPanel />
      </aside>
    </div>
  )
}