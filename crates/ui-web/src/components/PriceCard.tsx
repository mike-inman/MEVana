import { useStore } from '../state/useStore'
import { motion } from 'framer-motion'
export function PriceCard({p}:{p:any}){
const hist = useStore(s=>s.priceHistory[`${p.dex}:${p.pair}`] ?? [])
return (
<motion.div layout className="card p-3 hover:shadow-neon transition">
<div className="flex items-center justify-between">
<div className="text-sm text-muted">{p.dex}</div>
<div className="text-xs text-muted">{new
Date(p.ts*1000).toLocaleTimeString()}</div>
</div>
<div className="text-lg font-semibold mt-1">{p.pair}</div>
<div className="text-2xl font-mono tabular-nums mt-2">{p.price.toFixed(5)}
</div>
<Sparkline points={hist} />
</motion.div>
)
}
function Sparkline({points}:{points:number[]}){
if(points.length<2) return <div className="h-8" />
const w=240,h=40; const max=Math.max(...points), min=Math.min(...points)
const norm = (v:number)=> h - (h*(v-min))/Math.max(1e-9,(max-min))
const step = w/(points.length-1)
const d = points.map((v,i)=> `${i? 'L':'M'} ${i*step} ${norm(v)}`).join(' ')
return (
<svg viewBox={`0 0 ${w} ${h}`} className="w-full h-10 mt-2">
<path d={d} fill="none" stroke="var(--accent)" strokeWidth="1.5" />
</svg>

)
}