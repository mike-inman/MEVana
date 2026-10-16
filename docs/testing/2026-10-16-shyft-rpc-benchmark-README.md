# SHYFT RPC benchmark usage

**Date:** 2026-10-16
**Purpose:** Run a paced, read-only `getSlot` comparison from the local Mac and the existing Vultr and Hetzner servers.

The script is `scripts/testing/shyft_rpc_benchmark.py`. It uses Python 3 and system `curl`; it does not need Python packages. It accepts `--server-label`, `--network`, `--samples`, `--interval`, `--warmup`, `--timeout`, `--output-dir`, and optional `--location`. The default run uses both networks, 30 measured requests per network, two warm-ups, a one-second interval and an eight-second timeout. Each curl process opens a fresh direct IPv4 HTTP/1.1 connection. Authenticated URLs are supplied through curl's standard-input config, never as command arguments or output.

Local run from the benchmark worktree, reading only `SHYFT_API_KEY` from the root checkout's untouched `.env`:

```sh
python3 scripts/testing/shyft_rpc_benchmark.py run \
  --server-label local --env-file /Users/michaelinman/Repos/MEVana/.env \
  --output-dir benchmark_results/shyft
```

On a remote host, use `--key-stdin` and pipe **only the API key** over SSH standard input. Do not pass the key as a shell argument, copy the full `.env`, or save a remote credential file. The operator can use a local Python subprocess that calls `load_key_from_env()` and passes the returned key as `input=` to `ssh`; neither the key nor its authenticated URL should be printed. The remote command should run the deployed benchmark script with `--server-label vultr` or `hetzner`, `--key-stdin`, and a private task-local output directory. Copy only the resulting JSON back to `benchmark_results/shyft/` and delete the task-local remote JSON after retrieval.

From the worktree, set `BENCH_SSH_TARGET` to the verified SSH target and `BENCH_LABEL` to `vultr` or `hetzner`, then run:

```sh
python3 - <<'PY'
import os, subprocess, sys
sys.path.insert(0, 'scripts/testing')
from shyft_rpc_benchmark import load_key_from_env
key = load_key_from_env('/Users/michaelinman/Repos/MEVana/.env')
command = ['ssh', '-o', 'BatchMode=yes', '-o', 'StrictHostKeyChecking=yes',
           os.environ['BENCH_SSH_TARGET'], 'python3',
           '/opt/mevana-latency-benchmark-20261016/shyft_rpc_benchmark.py',
           'run', '--server-label', os.environ['BENCH_LABEL'], '--network', 'both',
           '--samples', '30', '--warmup', '2', '--interval', '1', '--timeout', '8',
           '--output-dir', '/opt/mevana-latency-benchmark-20261016/shyft-results',
           '--key-stdin']
subprocess.run(command, input=key, text=True, check=True)
PY
```

Generate the report with:

```sh
python3 scripts/testing/shyft_rpc_benchmark.py compare --output-dir benchmark_results/shyft
```

`local.json`, `vultr.json`, `hetzner.json`, and `comparison.md` are ignored by Git. JSON contains response slots, safe error categories, resolved endpoint IPs, and successful timing statistics. Cumulative curl timings are DNS lookup, TCP connected, TLS ready, first byte and total duration. Separate stage timings are TCP connection, TLS handshake, server wait and download. Only validated HTTP 200 JSON-RPC responses with a numeric slot enter latency statistics. Authentication failures and rate limits stop the run without retries. RPC response timing is not Yellowstone Geyser event-delivery timing.
