#!/usr/bin/env python3
# /// script
# requires-python = ">=3.10"
# dependencies = ["grpcio==1.76.0", "protobuf==6.33.0"]
# ///
"""2026-10-29: bounded, read-only Yellowstone slot samples; no engine startup.

Uses a minimal protobuf descriptor for the stable slot subscription fields.
Unknown response fields are ignored by protobuf, not interpreted as slot data.
"""
import argparse
import json
import os
import sys
from datetime import datetime, timezone
from urllib.parse import urlsplit

import grpc
from google.protobuf import descriptor_pb2, descriptor_pool, message_factory


def response_type():
    schema = descriptor_pb2.FileDescriptorProto(name="slot_probe.proto", syntax="proto3")
    slot = schema.message_type.add(name="Slot")
    slot.field.add(name="slot", number=1, type=4, label=1)
    slot.field.add(name="parent", number=2, type=4, label=1)
    slot.field.add(name="status", number=3, type=5, label=1)
    update = schema.message_type.add(name="Update")
    update.field.add(name="slot", number=3, type=11, type_name=".Slot", label=1)
    pool = descriptor_pool.DescriptorPool()
    pool.Add(schema)
    return message_factory.GetMessageClass(pool.FindMessageTypeByName("Update"))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("provider", choices=["tatum", "sodae"])
    parser.add_argument("--count", type=int, default=10)
    parser.add_argument("--seconds", type=int, default=20)
    args = parser.parse_args()
    if not 1 <= args.count <= 100 or not 1 <= args.seconds <= 60:
        parser.error("count must be 1–100; seconds must be 1–60")
    if args.provider == "tatum":
        endpoint = os.environ.get("TATUM_GEYSER_URL", "https://solana-mainnet-grpc.gateway.tatum.io:443")
        token = os.environ.get("TATUM_API_KEY", "")
        header = "x-api-key"
    else:
        endpoint = os.environ.get("SODAE_YELLOWSTONE_URL", "https://ams.tls.sodae.io:443")
        token = os.environ.get("SODAE_TOKEN", "")
        header = "x-token"
        if not token:
            parser.error("Set SODAE_TOKEN locally after your Yellowstone trial is approved")
    url = urlsplit(endpoint)
    if url.scheme != "https" or not url.hostname or url.username or url.password or url.query or url.fragment or url.path not in ("", "/"):
        parser.error("Use an HTTPS endpoint without credentials, query parameters or path")
    target = f"{url.hostname}:{url.port or 443}"
    update_type = response_type()
    metadata = [(header, token)] if token else []
    # SubscribeRequest.slots["dev"] = {filter_by_commitment: true};
    # commitment = PROCESSED. Field numbers follow Yellowstone geyser.proto.
    request = b"\x12\x09\x0a\x03dev\x12\x02\x08\x01\x30\x00"
    received = 0
    with grpc.secure_channel(target, grpc.ssl_channel_credentials()) as channel:
        subscribe = channel.stream_stream("/geyser.Geyser/Subscribe")
        call = subscribe(iter([request]), metadata=metadata, timeout=args.seconds)
        try:
            for payload in call:
                update = update_type.FromString(payload)
                if not update.HasField("slot"):
                    continue
                print(json.dumps({"provider": args.provider, "received_at": datetime.now(timezone.utc).isoformat(),
                                  "slot": update.slot.slot, "parent": update.slot.parent,
                                  "status": update.slot.status}), flush=True)
                received += 1
                if received >= args.count:
                    break
        except grpc.RpcError as error:
            # Do not print provider error details: they can echo credentials.
            print(f"{args.provider}: gRPC {error.code().name}; {received} slot updates received", file=sys.stderr)
            return 1
        finally:
            call.cancel()
    if received < args.count:
        print(f"{args.provider}: stream ended after {received}/{args.count} updates", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
