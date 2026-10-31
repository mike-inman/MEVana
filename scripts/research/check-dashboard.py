#!/usr/bin/env python3
"""2026-10-31: Static evidence and browser-display guards for the dashboard."""
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DASHBOARD = ROOT / 'infrastructure/research/grafana/dashboards/mevana-research-evidence.json'
data = json.loads(DASHBOARD.read_text())
text = DASHBOARD.read_text()
panels = {panel['id']: panel for panel in data['panels']}
assert data['title'] == 'MEVana Research Evidence'
assert data['schemaVersion'] == 39
assert data['editable'] is False
assert set(panels) == set(range(1, 9))
assert 'Simulated research only. About eight minutes of capture. No realized results.' in panels[1]['options']['content']
assert all(p['fieldConfig']['defaults']['noValue'] == 'Not measured' for p in list(panels.values())[1:])
assert all(p['datasource']['uid'] == 'research-postgres' for p in list(panels.values())[1:])
assert not re.search(r'COALESCE|connectNulls|null.as.zero|profitable|successful|winner|best|recommended', text, re.I)
assert not re.search(r'"(allValue|includeAll)"\s*:\s*(".*"|true)', text, re.I)
assert all(not p.get('transformations') for p in panels.values())
assert all('${experiment:sqlstring}' in panels[i]['targets'][0]['rawSql'] for i in range(5, 9))
assert all(not re.search(r'\bseed_loaded_at\s+AS\s+time\b', p['targets'][0]['rawSql'], re.I)
           for p in list(panels.values())[1:])

variable = data['templating']['list'][0]
assert len(data['templating']['list']) == 1
assert variable['includeAll'] is False and variable['multi'] is False
assert variable['current']['value'] == 'replay-v2-463353643-usdc-confirmed-l1-tip10000'
assert 'scope_label AS __text' in variable['query']

# Multi-row stat and bar-gauge panels must render values per row; a bar chart is also valid.
for panel in panels.values():
    if panel['type'] in ('stat', 'bargauge'):
        assert panel['options'].get('reduceOptions', {}).get('values') is True, panel['title']
assert panels[3]['type'] == 'barchart' and panels[3]['gridPos']['w'] == 24
assert panels[3]['fieldConfig']['defaults']['decimals'] == 2
assert 'scope_label' in panels[3]['targets'][0]['rawSql']
assert panels[4]['gridPos']['w'] == 24 and panels[4]['gridPos']['h'] >= 16
assert "FM990.00" in panels[4]['targets'][0]['rawSql']
assert "FM990.00" in panels[7]['targets'][0]['rawSql']
assert 'jsonb_array_elements_text' in panels[2]['targets'][0]['rawSql']
assert 'Not measured for this experiment' in panels[7]['targets'][0]['rawSql']
assert 'Not measured for this experiment' in panels[8]['targets'][0]['rawSql']
assert panels[8]['type'] == 'table'
assert 'Diagnostic probe: raw input amounts, not chained route amounts' in panels[8]['title']
# Outcomes are exhaustive per probe row, so an outcome that did not occur is an exact 0, not unmeasured.
assert 'FILTER' not in panels[8]['targets'][0]['rawSql']
assert 'seed_loaded_at' not in panels[8]['targets'][0]['rawSql']
assert all('FM999,999,999,999' in panels[i]['targets'][0]['rawSql'] for i in (4, 6, 7, 8))

reasons = ('below_threshold', 'insufficient_state', 'tick_array_absent', 'state_gap')
overrides = panels[5]['fieldConfig']['overrides']
colors = {}
for reason in reasons:
    match = [o for o in overrides if o['matcher'] == {'id': 'byName', 'options': reason}]
    assert len(match) == 1
    color = next(prop['value'] for prop in match[0]['properties'] if prop['id'] == 'color')
    assert color['mode'] == 'fixed'
    colors[reason] = color['fixedColor']
assert len(set(colors.values())) == len(reasons)
assert not any(color.lower() in ('#00ff00', '#ff0000', '#2ef2a6', '#ef4444') for color in colors.values())
assert panels[5]['options']['orientation'] == 'horizontal'
assert panels[5]['options']['stacking']['mode'] == 'normal'

# Explicit table widths must fit a 1500px viewport with room for panel padding.
for number in (2, 4, 6, 7, 8):
    widths = [prop['value'] for override in panels[number]['fieldConfig']['overrides']
              for prop in override['properties'] if prop['id'] == 'custom.width']
    assert widths and sum(widths) <= 1400, (number, sum(widths))

schema = (ROOT / 'infrastructure/research/schema.sql').read_text()
assert schema.count('trim_scale(') >= 6
print('Dashboard evidence guard: PASS (8 panels, default universe, 12 coverage rows, scoped diagnostics)')
