# 2026-11-01 CST
## ADDED Requirements
### Requirement: Any passing compared pool pair
R5 SHALL accept a pair when any compared eligible pool pair clears its own combined fee, even if a larger gap fails against higher fees.
#### Scenario: Smaller cheaper gap
- **WHEN** the largest gap fails but a smaller gap clears its combined fee
- **THEN** R5 SHALL pass using the passing comparison.
### Requirement: Complete review tables
The dashboard SHALL show all candidate and control rows at a 1500-pixel viewport and an empty first-failure value when nothing failed.
#### Scenario: Passing pair
- **WHEN** a pair has no failure
- **THEN** First failure SHALL be empty, rather than Not measured.
