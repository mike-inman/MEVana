import { useStore } from '../state/useStore'
export function SwapsPanel(){
  const swaps = useStore(s=>s.swaps)
  return (
    <div className="space-y-2 overflow-auto max-h-[64vh] pr-1">
      {swaps.map((s)=> (
        <div key={s.sig} className="card p-2 grid grid-cols-12 gap-2 items-center">
          <div className="col-span-2 text-xs text-muted">{new Date(s.ts*1000).toLocaleTimeString()}</div>
          <div className="col-span-3 font-mono text-xs">{s.sig.slice(0,10)}…</div>
          <div className="col-span-2 text-xs">{s.dex.toUpperCase()}</div>
          <div className="col-span-3 text-sm truncate">{s.token_in} → {s.token_out}</div>
          <div className="col-span-2 font-mono text-sm text-right">{s.amount_in ?? '?'} → {s.amount_out ?? '?'}</div>
        </div>
      ))}
    </div>
  )
}