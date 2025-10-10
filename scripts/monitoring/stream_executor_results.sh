#!/usr/bin/env bash
# Stream executor dry-run results with enriched analytics fields.
# Usage: stream_executor_results.sh [--from-beginning] [--limit N]

set -euo pipefail

BROKER="${KAFKA_BROKER:-localhost:29092}"
TOPIC="${EXECUTION_RESULTS_TOPIC:-execution.results.v1}"
CONSUMER_GROUP="${EXECUTOR_MONITOR_GROUP:-mevana.executor.analytics}"
KAFKA_CONTAINER="${KAFKA_CONTAINER:-mevana-kafka-1}"

FROM_BEGINNING=false
LIMIT=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --from-beginning)
      FROM_BEGINNING=true
      shift
      ;;
    --limit)
      LIMIT="$2"
      shift 2
      ;;
    --help|-h)
      cat <<USAGE
Stream enriched executor results from Kafka.

Environment overrides:
  KAFKA_BROKER            (default: localhost:29092)
  EXECUTION_RESULTS_TOPIC (default: execution.results.v1)
  EXECUTOR_MONITOR_GROUP  (default: mevana.executor.analytics)
  KAFKA_CONTAINER         (default: mevana-kafka-1)

Flags:
  --from-beginning   Consume from beginning of topic (default: latest)
  --limit N          Stop after N messages (handy for sampling)
USAGE
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required on the host" >&2
  exit 1
fi

KAFKA_CMD=("docker" "exec" "-i" "$KAFKA_CONTAINER" "kafka-console-consumer" "--bootstrap-server" "$BROKER" "--topic" "$TOPIC" "--group" "$CONSUMER_GROUP" "--property" "print.key=false")

if [[ "$FROM_BEGINNING" == true ]]; then
  KAFKA_CMD+=("--from-beginning")
fi

if [[ -n "$LIMIT" ]]; then
  KAFKA_CMD+=("--max-messages" "$LIMIT")
fi

"${KAFKA_CMD[@]}" 2>&1 \
  | sed '/^Processed a total/d' \
  | jq -r '
    select(type == "object")
    | {
        opportunity_id,
        execution_id,
        status,
        rejection_reason: .error_message,
        estimated_profit_sol: ((.estimated_profit_lamports // 0) / 1e9),
        gas_used,
        execution_time_ms,
        flash_loan_provider,
        flash_loan_fee_sol: ((.flash_loan_fee_lamports // 0) / 1e9),
        estimated_priority_fee_sol: ((.estimated_priority_fee_lamports // 0) / 1e9),
        estimated_cu_cost_sol: ((.estimated_cu_cost_lamports // 0) / 1e9),
        simulated_token_deltas: .simulated_token_deltas
      }
    | @json'
