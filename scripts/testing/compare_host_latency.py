#!/usr/bin/env python3
"""Read-only DNS, TCP, TLS and Solana RPC timing probe (standard library only)."""

import argparse
import csv
import datetime as dt
import json
import os
import socket
import ssl
import sys
import time
from urllib.parse import urlsplit

FIELDS = ["server_label", "utc_time", "endpoint", "trial", "metric", "value", "unit", "status"]
METRICS = ("dns_ms", "tcp_ms", "tls_ms", "http_total_ms")


def utc_now():
    return dt.datetime.now(dt.timezone.utc).isoformat(timespec="milliseconds")


def percentile(values, percent):
    ordered = sorted(values)
    if not ordered:
        return None
    rank = (len(ordered) - 1) * percent / 100
    lo = int(rank)
    hi = min(lo + 1, len(ordered) - 1)
    return ordered[lo] + (ordered[hi] - ordered[lo]) * (rank - lo)


def probe(url, timeout, rpc):
    parsed = urlsplit(url)
    if parsed.scheme != "https" or not parsed.hostname or parsed.username or parsed.password:
        raise ValueError("target must be an HTTPS URL without embedded credentials")
    host, port = parsed.hostname, parsed.port or 443
    timings = {}
    start = time.monotonic()
    addresses = socket.getaddrinfo(host, port, type=socket.SOCK_STREAM)
    timings["dns_ms"] = (time.monotonic() - start) * 1000
    if not addresses:
        raise OSError("DNS returned no addresses")
    # First successful address is recorded locally for anycast/path diagnosis.
    last_error = None
    for family, kind, proto, _, address in addresses:
        sock = socket.socket(family, kind, proto)
        sock.settimeout(timeout)
        try:
            start = time.monotonic()
            sock.connect(address)
            timings["tcp_ms"] = (time.monotonic() - start) * 1000
            break
        except OSError as exc:
            last_error = exc
            sock.close()
    else:
        raise last_error or OSError("TCP connect failed")
    with sock:
        start = time.monotonic()
        with ssl.create_default_context().wrap_socket(sock, server_hostname=host) as tls:
            timings["tls_ms"] = (time.monotonic() - start) * 1000
            if not rpc:
                return timings, address[0], "ok"
            body = b'{"jsonrpc":"2.0","id":1,"method":"getHealth"}'
            path = parsed.path or "/"
            if parsed.query:
                path += "?" + parsed.query
            host_header = host if port == 443 else f"{host}:{port}"
            request = (
                f"POST {path} HTTP/1.1\r\nHost: {host_header}\r\n"
                "Content-Type: application/json\r\nConnection: close\r\n"
                f"Content-Length: {len(body)}\r\n\r\n"
            ).encode("ascii") + body
            start = time.monotonic()
            tls.sendall(request)
            chunks = []
            total = 0
            while True:
                chunk = tls.recv(65536)
                if not chunk:
                    break
                total += len(chunk)
                if total > 1024 * 1024:
                    raise ValueError("HTTP response exceeds 1 MiB")
                chunks.append(chunk)
            timings["http_total_ms"] = (time.monotonic() - start) * 1000
            response = b"".join(chunks)
            head, sep, payload = response.partition(b"\r\n\r\n")
            status_line = head.split(b"\r\n", 1)[0]
            if not sep or not status_line.startswith(b"HTTP/"):
                raise ValueError("invalid HTTP response")
            code = int(status_line.split()[1])
            if code != 200:
                return timings, address[0], f"http_{code}"
            if b"transfer-encoding: chunked" in head.lower():
                size_line, _, remainder = payload.partition(b"\r\n")
                payload = remainder[: int(size_line.split(b";", 1)[0], 16)]
            data = json.loads(payload)
            if data.get("error") or data.get("result") != "ok":
                return timings, address[0], "rpc_error"
            return timings, address[0], "ok"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--server-label", required=True, choices=("vultr", "hetzner"))
    parser.add_argument("--endpoint-env", default="GEYSER_ENDPOINT")
    parser.add_argument("--rpc-url-env", default="SHYFT_RPC_URL")
    parser.add_argument("--samples", type=int, default=30)
    parser.add_argument("--warmup", type=int, default=2)
    parser.add_argument("--timeout", type=float, default=5)
    parser.add_argument("--trial", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--addresses-output", help="Local-only address log; do not commit")
    args = parser.parse_args()
    if args.samples < 1 or args.warmup < 0 or args.timeout <= 0:
        parser.error("samples must be positive, warmup nonnegative and timeout positive")
    targets = [("geyser", os.getenv(args.endpoint_env), False)]
    if os.getenv(args.rpc_url_env):
        targets.append(("rpc", os.environ[args.rpc_url_env], True))
    if not targets[0][1]:
        parser.error(f"{args.endpoint_env} is required")
    for _, url, _ in targets:
        parsed = urlsplit(url)
        if parsed.scheme != "https" or not parsed.hostname or parsed.username or parsed.password:
            parser.error("targets must be HTTPS URLs without embedded credentials")
    rows = []
    counts = {}
    for name, url, rpc in targets:
        host = urlsplit(url).hostname
        for index in range(-args.warmup, args.samples):
            stamp = utc_now()
            timings = {}
            address = ""
            try:
                timings, address, status = probe(url, args.timeout, rpc)
            except (OSError, ValueError, json.JSONDecodeError, ssl.SSLError) as exc:
                status = type(exc).__name__.lower()
            if args.addresses_output and address:
                with open(args.addresses_output, "a", encoding="utf-8") as addresses_file:
                    addresses_file.write(f"{stamp},{args.server_label},{name},{address}\n")
            if index < 0:
                continue
            counts[(name, status)] = counts.get((name, status), 0) + 1
            for metric in METRICS:
                if metric == "http_total_ms" and not rpc:
                    continue
                rows.append({"server_label": args.server_label, "utc_time": stamp,
                             "endpoint": host, "trial": args.trial, "metric": metric,
                             "value": f"{timings[metric]:.3f}" if metric in timings and status == "ok" else "",
                             "unit": "ms", "status": status})
        print(f"{name} ({host}): " + ", ".join(f"{status}={count}" for (target, status), count in counts.items() if target == name))
        for metric in METRICS:
            values = [float(row["value"]) for row in rows if row["endpoint"] == host and row["metric"] == metric and row["value"]]
            if values:
                print(f"  {metric}: n={len(values)} p50={percentile(values, 50):.3f} p95={percentile(values, 95):.3f} p99={percentile(values, 99):.3f} ms")
    exists = os.path.exists(args.output) and os.path.getsize(args.output) > 0
    with open(args.output, "a", newline="", encoding="utf-8") as output:
        writer = csv.DictWriter(output, fieldnames=FIELDS)
        if not exists:
            writer.writeheader()
        writer.writerows(rows)
    return 0 if all(counts.get((name, "ok"), 0) == args.samples for name, _, _ in targets) else 1


if __name__ == "__main__":
    sys.exit(main())
