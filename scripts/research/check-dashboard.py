#!/usr/bin/env python3
"""2026-10-31: Static evidence-safety check for the provisioned dashboard."""
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DASHBOARD = ROOT / 'infrastructure/research/grafana/dashboards/mevana-research-evidence.json'
data = json.loads(DASHBOARD.read_text())
text = DASHBOARD.read_text()
panels = data['panels']
assert data['title'] == 'MEVana Research Evidence'
assert data['schemaVersion'] == 39
assert data['editable'] is False
assert len(panels) == 8
assert 'Simulated research only. About eight minutes of capture. No realized results.' in panels[0]['options']['content']
assert all(p['fieldConfig']['defaults']['noValue'] == 'Not measured' for p in panels[1:])
assert all(p['datasource']['uid'] == 'research-postgres' for p in panels[1:])
assert panels[-1]['type'] == 'heatmap'
assert 'Diagnostic probe: raw input amounts, not chained route amounts' in panels[-1]['title']
assert len(data['templating']['list']) == 1
assert data['templating']['list'][0]['includeAll'] is False
assert data['templating']['list'][0]['multi'] is False
assert all('${experiment:sqlstring}' in p['targets'][0]['rawSql'] for p in panels[4:])
assert not re.search(r'COALESCE|connectNulls|null.as.zero|profitable|successful|winner|best|recommended', text, re.I)
assert not re.search(r'"(allValue|includeAll)"\s*:\s*(".*"|true)', text, re.I)
assert all(not p.get('transformations') for p in panels)
print('Dashboard evidence guard: PASS (8 panels, scoped variable, null and wording rules)')
