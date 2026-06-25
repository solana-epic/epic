# EPIC Known Issues

| ID | Component | Severity | Description | Status |
|---|---|---|---|---|
| KI-001 | Parser | Medium | `epic audit .` without flags ignores `fixtures` folders, preventing dummy-vuln detections unless `--all` is passed. | Open |
| KI-002 | CLI | Minor | Execution timings for AST Build and Rule Engine are statically simulated rather than extracting raw ticks from `parser-v2`. | Open |
