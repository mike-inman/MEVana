#!/usr/bin/env python3
"""2026-10-31: Validate the local Handoff 17 stack without credentials in output."""
import csv
import io
import json
import subprocess
import urllib.request
import uuid
from decimal import Decimal
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
COMPOSE = ['docker', 'compose', '--env-file', str(ROOT / 'infrastructure/research.env'), '-p', 'mevana-research', '-f', str(ROOT / 'infrastructure/docker-compose.research.yml'), 'exec', '-T', 'postgres']
PSQL = COMPOSE + ['psql', '-U', 'postgres', '-d', 'research', '-A', '-F', ',', '-t', '-c']
API = 'http://127.0.0.1:3010'
SELECTED = 'replay-v2-463353643-usdc-confirmed-l1-tip10000'
V1 = 'replay-v2-463353643-a-confirmed-l1-tip10000'
TABLES = ('experiments', 'policy_results', 'policy_rejections', 'shape_results', 'shape_rejections', 'pool_direction_probes')


def query(sql):
    result = subprocess.run(PSQL + [sql], capture_output=True, text=True, check=True)
    return list(csv.reader(io.StringIO(result.stdout)))


def admin(sql):
    return subprocess.run(PSQL + [sql], capture_output=True, text=True)


def reader(sql):
    return subprocess.run(COMPOSE + ['sh', '-c',
        'PGPASSWORD="$RESEARCH_READER_PASSWORD" exec psql -h 127.0.0.1 -U research_reader -d research -v ON_ERROR_STOP=1 -tAc "$1"',
        'sh', sql], capture_output=True, text=True)


def snapshot():
    return {table: query(f"SELECT count(*), md5(string_agg(to_jsonb(t)::text, '|' ORDER BY to_jsonb(t)::text)) FROM research.{table} t")[0]
            for table in TABLES}


coverage = query("SELECT result_id,quote_symbol,evaluable_candidates,candidates_evaluated,evaluable_share_pct FROM research.v_policy_evidence WHERE policy_kind='FixedUniverse' ORDER BY result_id")
expected = [
    ['replay-v2-463353643-a-confirmed-l1-tip10000', 'USDC', '66870', '67050', '99.73'],
    ['replay-v2-463353643-b-confirmed-l1-tip10000', 'USDT', '20061', '44700', '44.88'],
    ['replay-v2-463353643-c-confirmed-l1-tip10000', 'USDT', '4458', '4470', '99.73'],
    [SELECTED, 'USDC', '258564', '339720', '76.11'],
]
for row in coverage:
    print('coverage', *row)
assert coverage == expected, 'handoff coverage differs'
triangle = query("SELECT quote_symbol,evaluable_candidates,candidates_evaluated,evaluable_share_pct FROM research.v_shape_evidence WHERE shape='triangle' AND policy_kind='FixedUniverse'")
print('triangle', *triangle[0])
assert triangle == [['USDC', '187236', '268200', '69.81']], 'handoff triangle coverage differs'
profits = query(f"SELECT policy_kind,simulated_net_profit_quote::text,missed_candidates IS NULL,realized_net_profit_quote_atoms IS NULL FROM research.v_policy_evidence WHERE result_id='{SELECTED}' ORDER BY policy_kind")
amounts = {row[0]: Decimal(row[1]) for row in profits}
assert amounts['FixedUniverse'] == Decimal('-0.005191'), 'FixedUniverse profit differs'
assert amounts['SimpleRules'] == Decimal('-0.002269'), 'SimpleRules profit differs'
assert next(row[1] for row in profits if row[0] == 'FixedUniverse') == '-0.005191', 'FixedUniverse profit is padded'
assert next(row[1] for row in profits if row[0] == 'SimpleRules') == '-0.002269', 'SimpleRules profit is padded'
zero_profit = query(f"SELECT simulated_net_profit_quote::text FROM research.v_shape_evidence WHERE result_id='{SELECTED}' AND shape='triangle' AND policy_kind='FixedUniverse'")
assert zero_profit == [['0']], 'zero profit is padded or hidden'
assert all(row[2:] == ['t', 't'] for row in profits), 'unmeasured value differs'
print('profits', 'FixedUniverse', amounts['FixedUniverse'], 'SimpleRules', amounts['SimpleRules'], 'triangle zero', zero_profit[0][0], 'missed/realized NULL')

assert reader('SELECT count(*) FROM research.experiments').returncode == 0
for sql in ["INSERT INTO research.experiments(result_id) VALUES ('denied')",
            "UPDATE research.experiments SET status='denied' WHERE false",
            'DELETE FROM research.experiments WHERE false', 'CREATE TABLE research.denied(x int)']:
    assert reader(sql).returncode != 0, 'reader write unexpectedly allowed'
probe = 'research_reader_probe_' + uuid.uuid4().hex
assert admin(f'CREATE TABLE public.{probe}(x int)').returncode == 0
try:
    assert reader(f'SELECT * FROM public.{probe}').returncode != 0, 'reader outside-schema read unexpectedly allowed'
finally:
    assert admin(f'DROP TABLE public.{probe}').returncode == 0
future = 'future_reader_probe_' + uuid.uuid4().hex
assert admin(f'CREATE TABLE research.{future}(x int)').returncode == 0
try:
    assert reader(f'SELECT count(*) FROM research.{future}').returncode == 0, 'future SELECT grant missing'
finally:
    assert admin(f'DROP TABLE research.{future}').returncode == 0
print('reader_access PASS')

# Inspect names only. Never print or log any environment value.
environment = subprocess.run(['docker', 'inspect', '--format', '{{json .Config.Env}}', 'mevana-research-grafana'],
                             capture_output=True, text=True, check=True)
env_names = {entry.split('=', 1)[0] for entry in json.loads(environment.stdout)}
assert 'POSTGRES_PASSWORD' not in env_names, 'Grafana received Postgres superuser password'
assert {'RESEARCH_READER_PASSWORD', 'GF_SECURITY_ADMIN_PASSWORD'} <= env_names
print('grafana_environment_names PASS')

with urllib.request.urlopen(API + '/api/dashboards/uid/mevana-research-evidence', timeout=10) as response:
    dashboard = json.load(response)['dashboard']
assert dashboard['title'] == 'MEVana Research Evidence' and len(dashboard['panels']) == 8
assert dashboard['templating']['list'][0]['current']['value'] == SELECTED
for experiment, label, expected_rows in ((SELECTED, 'universe', [4, 12, 12, 3, 3, 6, 26]),
                                          (V1, 'v1', [4, 12, 12, 3, 3, 1, 1])):
    for panel, expected_count in zip(dashboard['panels'][1:], expected_rows):
        sql = panel['targets'][0]['rawSql'].replace('${experiment:sqlstring}', "'" + experiment + "'")
        payload = {'queries': [{'refId': 'A', 'datasource': {'uid': 'research-postgres', 'type': 'postgres'},
            'rawSql': sql, 'format': 'table', 'intervalMs': 1000, 'maxDataPoints': 1000}], 'from': 'now-1h', 'to': 'now'}
        request = urllib.request.Request(API + '/api/ds/query', json.dumps(payload).encode(), {'Content-Type': 'application/json'}, method='POST')
        with urllib.request.urlopen(request, timeout=15) as response:
            result = json.load(response)['results']['A']
        frames = result.get('frames', [])
        rows = sum(len(frame.get('data', {}).get('values', [[]])[0]) for frame in frames)
        assert result.get('status') == 200 and not result.get('error') and rows == expected_count, f"Grafana panel {panel['id']} failed for {label}: {rows}"
        fields = [field['name'] for field in frames[0]['schema']['fields']]
        values = frames[0]['data']['values']
        if panel['id'] in (7, 8) and label == 'v1':
            assert values[0][0] == 'Not measured for this experiment'
        if panel['id'] == 6 and label == 'universe':
            simulated = values[fields.index('Simulated net profit')]
            assert '-0.005191 USDC' in simulated and '-0.002269 USDC' in simulated
        if panel['id'] == 4:
            shares = values[fields.index('Share')]
            assert all(share.endswith('%') and len(share.split('.')[1]) == 3 for share in shares)
        print('grafana_panel', label, panel['id'], 'rows', rows)

before = snapshot()
load = subprocess.run([str(ROOT / 'scripts/research/load-seed.sh')], capture_output=True, text=True)
assert load.returncode == 0, 'second load failed'
assert snapshot() == before, 'rows changed after second load'
print('idempotence PASS', 'counts', ','.join(f'{t}={before[t][0]}' for t in TABLES))
print('LIVE_VALIDATION_PASS')
