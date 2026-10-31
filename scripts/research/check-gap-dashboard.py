#!/usr/bin/env python3
"""2026-10-31: Static contract check for the provisioned Indicative survey dashboard."""
import json
from pathlib import Path

root = Path(__file__).resolve().parents[2]
path = root / "infrastructure/research/grafana/dashboards/mevana-price-gap-survey.json"
dashboard = json.loads(path.read_text())
assert dashboard["title"] == "MEVana Price-Gap Survey"
assert dashboard["editable"] is False
panels = dashboard["panels"]
assert len(panels) == 8
assert panels[0]["type"] == "text"
assert "Indicative mid-price gaps only. Not executable and not profit." in panels[0]["options"]["content"]
for panel in panels[1:]:
    assert panel["type"] == "table", panel["title"]
    assert panel["fieldConfig"]["defaults"]["noValue"] == "Not measured"
    assert panel["options"]["showHeader"] is True
    sql = panel["targets"][0]["rawSql"]
    assert "Indicative" in sql or "evidence_kind" in sql
    assert "profit" not in sql.lower()
    assert "COALESCE(median_net" not in sql
    assert panel["targets"][0]["format"] == "table"
    assert panel["fieldConfig"]["defaults"]["color"]["fixedColor"] == "#7aa5d2"
for panel in (panels[1], panels[2], panels[4]):
    assert 'AS "time"' in panel["targets"][0]["rawSql"]
assert "color-background" in json.dumps(panels[4])
print("Gap dashboard contract: 8 panels, Indicative labels, neutral colors and explicit unknowns")
