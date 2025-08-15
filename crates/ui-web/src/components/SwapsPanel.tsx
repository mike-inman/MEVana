import { useStore } from '../state/useStore'
export function SwapsPanel(){
const swaps = useStore(s=>s.swaps)
return (
<div className="space-y-2 overflow-auto max-h-[72vh] pr-1">
{swaps.map((s)=> (
<div key={s.sig} className="card p-2 flex items-center justify-between">
<div className="text-sm text-muted">{new
Date(s.ts*1000).toLocaleTimeString()}</div>
<div className="font-mono">{s.sig.slice(0,8)}…</div>
<div className="text-sm">{s.dex.toUpperCase()}</div>
<div className="text-sm">{s.token_in} → {s.token_out}</div>
<div className="font-mono">{s.amount_in ?? '?'} → {s.amount_out ??
'?'}</div>
</div>
))}
</div>
)
}