import { useEffect, useState } from 'react'

type PoolInfo = {
  pool_id: string
  tick_index?: number
  sqrt_price_x64?: string
  token_mint_a?: string
  token_mint_b?: string
}

export function PoolPanel(){
  const [pool, setPool] = useState<PoolInfo | null>(null)

  // Placeholder: we will wire this to a REST/WS source later
  useEffect(()=>{
    setPool(null)
  },[])

  return (
    <div className="space-y-2">
      <div className="text-sm text-muted">Pool</div>
      {pool ? (
        <div className="card p-2 text-sm">
          <div className="font-mono">{pool.pool_id}</div>
          <div>tick: {pool.tick_index ?? '—'}</div>
          <div>sqrt_price_x64: {pool.sqrt_price_x64 ?? '—'}</div>
          <div>token A: {pool.token_mint_a ?? '—'}</div>
          <div>token B: {pool.token_mint_b ?? '—'}</div>
        </div>
      ) : (
        <div className="text-xs text-muted">No pool selected</div>
      )}
    </div>
  )
}


