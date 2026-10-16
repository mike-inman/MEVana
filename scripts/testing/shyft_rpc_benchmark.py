#!/usr/bin/env python3
"""Paced, read-only SHYFT getSlot benchmark using Python and system curl."""

import argparse
import datetime as dt
import json
import math
import re
import statistics
import subprocess
import sys
import time
from pathlib import Path
from urllib.parse import quote


ENDPOINTS = {
    "devnet": "https://devnet-rpc.shyft.to/",
    "mainnet": "https://rpc.shyft.to/",
}
RPC_REQUEST = '{"jsonrpc":"2.0","id":1,"method":"getSlot"}'
CUMULATIVE = ("dns_lookup_ms", "tcp_connect_ms", "tls_ready_ms", "first_byte_ms", "total_ms")
STAGES = ("tcp_stage_ms", "tls_stage_ms", "server_wait_ms", "download_ms")
TIMINGS = CUMULATIVE + STAGES
MARKER = "\n__MEVANA_CURL_TIMING__"


def utc_now():
    return dt.datetime.now(dt.timezone.utc).isoformat(timespec="seconds")


def load_key_from_env(path):
    """Read one dotenv assignment literally, without executing or interpolating it."""
    found = None
    for line in Path(path).read_text(encoding="utf-8").splitlines():
        match = re.match(r"^\s*(?:export\s+)?SHYFT_API_KEY\s*=\s*(.*)$", line)
        if not match:
            continue
        value = match.group(1).strip()
        if value.startswith(("'", '"')):
            marker = value[0]
            end = value.find(marker, 1)
            if end < 0 or value[end + 1 :].strip() not in ("",) and not value[end + 1 :].lstrip().startswith("#"):
                raise ValueError("invalid SHYFT_API_KEY assignment")
            value = value[1:end]
        else:
            value = re.split(r"\s+#", value, maxsplit=1)[0].strip()
        if found is not None:
            raise ValueError("duplicate SHYFT_API_KEY assignment")
        found = value
    if not found or found.startswith("<") or "replace-with" in found.lower():
        raise ValueError("usable SHYFT_API_KEY not found")
    if any(char.isspace() for char in found):
        raise ValueError("invalid SHYFT_API_KEY assignment")
    return found


def percentile(values, pct):
    values = sorted(values)
    if not values:
        return None
    index = (len(values) - 1) * pct / 100
    low = math.floor(index)
    high = math.ceil(index)
    return values[low] + (values[high] - values[low]) * (index - low)


def stats(values):
    if not values:
        return None
    return {"min": round(min(values), 3), "average": round(statistics.mean(values), 3),
            "p50": round(percentile(values, 50), 3), "p95": round(percentile(values, 95), 3),
            "p99": round(percentile(values, 99), 3), "max": round(max(values), 3),
            "stddev": round(statistics.pstdev(values), 3)}


def curl_sample(network, key, timeout):
    """Return a sanitized sample; no authenticated URL is passed as an argv value."""
    endpoint = ENDPOINTS[network]
    url = endpoint + "?api_key=" + quote(key, safe="")
    config = 'url = "' + url + '"\n'
    writeout = MARKER + "%{time_namelookup}|%{time_connect}|%{time_appconnect}|%{time_starttransfer}|%{time_total}|%{http_code}|%{remote_ip}"
    command = ["curl", "--silent", "--show-error", "--no-progress-meter", "--noproxy", "*",
               "--ipv4", "--http1.1", "--request", "POST", "--header", "Content-Type: application/json",
               "--data-binary", RPC_REQUEST, "--connect-timeout", "4", "--max-time", str(timeout),
               "--retry", "0", "--config", "-", "--write-out", writeout]
    try:
        completed = subprocess.run(command, input=config, text=True, capture_output=True,
                                   timeout=timeout + 2, check=False)
    except (subprocess.TimeoutExpired, OSError) as exc:
        return {"utc_time": utc_now(), "success": False, "error": type(exc).__name__.lower(),
                "http_status": None, "slot": None, "resolved_ip": None, "timings_ms": None}
    result = {"utc_time": utc_now(), "success": False, "error": None,
              "http_status": None, "slot": None, "resolved_ip": None, "timings_ms": None}
    body, sep, suffix = completed.stdout.rpartition(MARKER)
    if not sep:
        result["error"] = f"curl_exit_{completed.returncode}" if completed.returncode else "missing_timing_data"
        return result
    parts = suffix.split("|")
    if len(parts) != 7:
        result["error"] = "invalid_timing_data"
        return result
    try:
        dns, tcp, tls, first, total = (float(value) * 1000 for value in parts[:5])
        result["http_status"] = int(parts[5])
    except ValueError:
        result["error"] = "invalid_timing_data"
        return result
    ip = parts[6].strip()
    result["resolved_ip"] = ip if re.fullmatch(r"[0-9.]+", ip) else None
    if completed.returncode:
        result["error"] = f"curl_exit_{completed.returncode}"
        return result
    if result["http_status"] != 200:
        result["error"] = f"http_{result['http_status']}"
        return result
    try:
        response = json.loads(body)
    except json.JSONDecodeError:
        result["error"] = "invalid_json"
        return result
    if not isinstance(response, dict) or response.get("jsonrpc") != "2.0" or response.get("id") != 1:
        result["error"] = "invalid_rpc_envelope"
        return result
    if "error" in response:
        result["error"] = "rpc_error"
        return result
    slot = response.get("result")
    if isinstance(slot, bool) or not isinstance(slot, int) or slot < 0:
        result["error"] = "invalid_slot"
        return result
    result["slot"] = slot
    result["timings_ms"] = {
        "dns_lookup_ms": round(dns, 3), "tcp_connect_ms": round(tcp, 3),
        "tls_ready_ms": round(tls, 3), "first_byte_ms": round(first, 3),
        "total_ms": round(total, 3), "tcp_stage_ms": round(max(0, tcp - dns), 3),
        "tls_stage_ms": round(max(0, tls - tcp), 3),
        "server_wait_ms": round(max(0, first - tls), 3),
        "download_ms": round(max(0, total - first), 3),
    }
    result["success"] = True
    return result


def network_run(network, key, samples, interval, warmup, timeout):
    attempts = []
    warmup_errors = []
    aborted = None
    for index in range(warmup + samples):
        if index:
            time.sleep(interval)
        sample = curl_sample(network, key, timeout)
        if index < warmup:
            if not sample["success"]:
                warmup_errors.append(sample["error"])
        else:
            attempts.append(sample)
        if sample["error"] in ("http_401", "http_403", "http_429"):
            aborted = sample["error"]
            break
    successes = [item for item in attempts if item["success"]]
    errors = {}
    for item in attempts:
        if item["error"]:
            errors[item["error"]] = errors.get(item["error"], 0) + 1
    return {"network": network, "rpc_hostname": ENDPOINTS[network].split("/")[2],
            "requested_samples": samples, "measured_samples": len(attempts),
            "warmup_requested": warmup, "warmup_errors": warmup_errors,
            "successful_responses": len(successes), "failed_responses": len(attempts) - len(successes),
            "error_rate": round((len(attempts) - len(successes)) / len(attempts), 4) if attempts else None,
            "errors": errors, "aborted_reason": aborted,
            "resolved_endpoint_ips": sorted({item["resolved_ip"] for item in attempts if item["resolved_ip"]}),
            "timing_statistics_ms": {name: stats([item["timings_ms"][name] for item in successes]) for name in TIMINGS},
            "samples": attempts}


def run(args):
    if args.samples < 1 or args.warmup < 0 or args.interval < 0.5 or args.timeout < 2:
        raise ValueError("samples >= 1, warmup >= 0, interval >= 0.5 and timeout >= 2 are required")
    key = sys.stdin.buffer.read().decode("utf-8").strip() if args.key_stdin else load_key_from_env(args.env_file)
    if not key or any(char.isspace() for char in key):
        raise ValueError("invalid SHYFT API key")
    selected = ("devnet", "mainnet") if args.network == "both" else (args.network,)
    report = {"server_label": args.server_label, "utc_timestamp": utc_now(),
              "hosting_location": args.location, "method": "curl HTTP/1.1 fresh IPv4 connection per sample; direct, no proxy",
              "timing_semantics": {"cumulative_ms": list(CUMULATIVE), "individual_stages_ms": list(STAGES)},
              "interval_seconds": args.interval, "timeout_seconds": args.timeout,
              "networks": {}}
    for network in selected:
        if report["networks"]:
            time.sleep(args.interval)
        report["networks"][network] = network_run(network, key, args.samples, args.interval, args.warmup, args.timeout)
        if report["networks"][network]["aborted_reason"]:
            break
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True, mode=0o700)
    output_dir.chmod(0o700)
    output = output_dir / f"{args.server_label}.json"
    output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    output.chmod(0o600)
    print(f"Saved {args.server_label} result: {output}")
    for network, result in report["networks"].items():
        print(f"{network}: {result['successful_responses']}/{result['measured_samples']} successful; errors={result['errors']}")
    return 0 if all(result["successful_responses"] == args.samples for result in report["networks"].values()) and len(report["networks"]) == len(selected) else 1


def fmt(result, network, metric, stat):
    try:
        value = result["networks"][network]["timing_statistics_ms"][metric][stat]
    except (KeyError, TypeError):
        return None
    return value


def compare(args):
    directory = Path(args.output_dir)
    results = {}
    for label in ("vultr", "hetzner"):
        path = directory / f"{label}.json"
        if path.exists():
            results[label] = json.loads(path.read_text(encoding="utf-8"))
    metrics = [("Mainnet p50", "mainnet", "total_ms", "p50"),
               ("Mainnet p95", "mainnet", "total_ms", "p95"),
               ("Mainnet p99", "mainnet", "total_ms", "p99"),
               ("Devnet p50", "devnet", "total_ms", "p50"),
               ("Devnet p95", "devnet", "total_ms", "p95"),
               ("Devnet p99", "devnet", "total_ms", "p99"),
               ("Mainnet TCP connection p50", "mainnet", "tcp_stage_ms", "p50"),
               ("Mainnet TLS handshake p50", "mainnet", "tls_stage_ms", "p50")]
    lines = ["# SHYFT RPC Vultr and Hetzner comparison", "", f"**Date:** {utc_now()}",
             "**Purpose:** Compare read-only SHYFT getSlot response timing from both existing hosts.", "",
             "| Metric | Vultr | Hetzner | Better |", "| --- | ---: | ---: | --- |"]
    for label, network, metric, stat in metrics:
        values = [fmt(results.get(host, {}), network, metric, stat) for host in ("vultr", "hetzner")]
        better = "Pending" if None in values else ("Tie (<5%)" if abs(values[0] - values[1]) < 0.05 * min(values) else ("Vultr" if values[0] < values[1] else "Hetzner"))
        lines.append(f"| {label} (ms) | {values[0] if values[0] is not None else 'Pending'} | {values[1] if values[1] is not None else 'Pending'} | {better} |")
    error_rates = []
    for host in ("vultr", "hetzner"):
        networks = results.get(host, {}).get("networks", {})
        total = sum(value["measured_samples"] for value in networks.values())
        failures = sum(value["failed_responses"] for value in networks.values())
        error_rates.append(100 * failures / total if total else None)
    better = "Pending" if None in error_rates else ("Tie" if error_rates[0] == error_rates[1] else ("Vultr" if error_rates[0] < error_rates[1] else "Hetzner"))
    lines.append(f"| Error rate (all attempts) | {f'{error_rates[0]:.1f}%' if error_rates[0] is not None else 'Pending'} | {f'{error_rates[1]:.1f}%' if error_rates[1] is not None else 'Pending'} | {better} |")
    complete = (len(results) == 2 and all(
        set(result.get("networks", {})) == set(ENDPOINTS) and
        all(n["measured_samples"] == n["requested_samples"] and n["successful_responses"] > 0
            for n in result["networks"].values())
        for result in results.values()))
    lines += ["", f"**Status:** {'Both host runs complete.' if complete else 'Incomplete; no hosting recommendation yet.'}",
              "**Interpretation:** Compare successful request distributions and error counts. A 5% difference is shown as a timing tie; this is a descriptive threshold, not a statistical confidence interval.",
              "**Timing semantics:** Total, first byte, TLS ready, TCP connected and DNS lookup are cumulative curl times from request start. TCP connection and TLS handshake rows are individual stage durations obtained by subtraction.",
              "**Remaining uncertainty:** RPC getSlot timing does not measure Yellowstone Geyser event delivery or transaction landing.",
              "**Local baseline:** `local.json` is a workstation reference only and is excluded from the host comparison.", ""]
    local_path = directory / "local.json"
    if local_path.exists():
        local = json.loads(local_path.read_text(encoding="utf-8"))
        main_local = fmt(local, "mainnet", "total_ms", "p50")
        dev_local = fmt(local, "devnet", "total_ms", "p50")
        if main_local is not None and dev_local is not None:
            lines += [f"The Mac's local p50 was {main_local:.3f} ms on mainnet and {dev_local:.3f} ms on devnet.", ""]
    if complete:
        vultr, hetzner = results["vultr"], results["hetzner"]
        main_v = vultr["networks"]["mainnet"]["timing_statistics_ms"]["total_ms"]
        main_h = hetzner["networks"]["mainnet"]["timing_statistics_ms"]["total_ms"]
        dev_v = vultr["networks"]["devnet"]["timing_statistics_ms"]["total_ms"]
        dev_h = hetzner["networks"]["devnet"]["timing_statistics_ms"]["total_ms"]
        def advantage(fast, slow):
            return 100 * (slow - fast) / slow
        main_adv = {q: advantage(main_v[q], main_h[q]) for q in ("p50", "p95", "p99")}
        dev_adv = {q: advantage(dev_v[q], dev_h[q]) for q in ("p50", "p95", "p99")}
        all_valid = all(n["successful_responses"] == n["requested_samples"] for r in results.values() for n in r["networks"].values())
        preference = (all_valid and min(main_adv.values()) >= 10 and dev_adv["p95"] >= 10)
        main_ips_same = set(vultr["networks"]["mainnet"]["resolved_endpoint_ips"]) == set(hetzner["networks"]["mainnet"]["resolved_endpoint_ips"])
        lines += ["## Decision", "",
                  "1. **Best observed RPC path:** " + ("Vultr, which was faster at p50, p95 and p99 on both networks." if min(main_adv.values()) > 0 and min(dev_adv.values()) > 0 else "Mixed results; no single host won all six total-latency measures."),
                  f"2. **Size of difference:** Vultr mainnet total latency was {main_adv['p50']:.1f}% lower at p50, {main_adv['p95']:.1f}% lower at p95 and {main_adv['p99']:.1f}% lower at p99. Devnet differences were {dev_adv['p50']:.1f}%, {dev_adv['p95']:.1f}% and {dev_adv['p99']:.1f}% respectively. These are observed differences from one 30-sample window per host, not confidence intervals.",
                  f"3. **Tail behavior:** Vultr mainnet max/stddev were {main_v['max']:.1f}/{main_v['stddev']:.1f} ms; Hetzner were {main_h['max']:.1f}/{main_h['stddev']:.1f} ms. Vultr devnet max/stddev were {dev_v['max']:.1f}/{dev_v['stddev']:.1f} ms; Hetzner were {dev_h['max']:.1f}/{dev_h['stddev']:.1f} ms. Hetzner had more pronounced tail spikes in this window." if main_h["p99"] > main_v["p99"] and dev_h["p99"] > dev_v["p99"] else "3. **Tail behavior:** The p99 and maxima are mixed; inspect the raw samples before treating either host as more stable.",
                  "4. **Initial hosting preference:** Vultr for the measured SHYFT RPC path. Keep Hetzner as the comparison/backup host; this is a provisional choice from one sequential test window." if preference else "4. **Initial hosting preference:** Inconclusive under the conservative rule requiring full success and at least a 10% mainnet p50/p95/p99 plus devnet p95 advantage.",
                  "5. **Still unproven:** Live Yellowstone Geyser event-arrival latency, stream stability, and transaction landing speed were not measured.", "",
                  f"**Host locations:** Vultr: {vultr.get('hosting_location') or 'unknown'}; Hetzner: {hetzner.get('hosting_location') or 'unknown'}.",
                  f"**Run conditions:** Each host made two warm-up and 30 measured requests per network, paced {vultr.get('interval_seconds')} second(s) apart. The runs were sequential; Vultr began at {vultr.get('utc_timestamp')} and Hetzner at {hetzner.get('utc_timestamp')}.",
                  "**Resolved mainnet addresses:** " + ("Both hosts used the same address set." if main_ips_same else "The hosts used different address sets, so observed timing includes provider DNS/routing differences."),
                  ""]
    destination = directory / "comparison.md"
    destination.write_text("\n".join(lines), encoding="utf-8")
    print(f"Saved comparison: {destination}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    run_parser = sub.add_parser("run", help="run paced getSlot requests")
    run_parser.add_argument("--server-label", required=True, choices=("local", "vultr", "hetzner"))
    run_parser.add_argument("--network", choices=("both", "devnet", "mainnet"), default="both")
    run_parser.add_argument("--samples", type=int, default=30)
    run_parser.add_argument("--interval", type=float, default=1.0)
    run_parser.add_argument("--warmup", type=int, default=2)
    run_parser.add_argument("--timeout", type=float, default=8.0)
    run_parser.add_argument("--output-dir", default="benchmark_results/shyft")
    run_parser.add_argument("--location", help="hosting location, only if verified")
    credential = run_parser.add_mutually_exclusive_group(required=True)
    credential.add_argument("--env-file", help="local dotenv file containing SHYFT_API_KEY")
    credential.add_argument("--key-stdin", action="store_true", help="read only the API key from stdin")
    compare_parser = sub.add_parser("compare", help="write comparison.md from host JSON results")
    compare_parser.add_argument("--output-dir", default="benchmark_results/shyft")
    args = parser.parse_args()
    try:
        return run(args) if args.command == "run" else compare(args) or 0
    except (ValueError, OSError, UnicodeError) as exc:
        print(f"Benchmark error: {type(exc).__name__}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
