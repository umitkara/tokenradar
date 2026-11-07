# Panel lifecycle: 2026-11-07-panel-detail

Runs: 3; scene: ok; sampling every 250 ms; exe: tokenradar-app.exe

| Budget | Value | Limit | |
|---|---|---|---|
| Cold open (median of all rounds) | 379.6 ms | 500 ms | ok |
| Warm open (median of all rounds) | 47.2 ms | 50 ms | ok |
| Panel open, private WS (max of round medians) | 82.2 MB | 130 MB | ok |
| warm_ttl + 10 s, private WS (max) | 80.4 MB | 30 MB | OVER |

| Value | Median | Run 1 | Run 2 | Run 3 |
|---|---|---|---|---|
| baseline_ws | 6.4 | 6.5 | 6.4 | 6.4 |
| cold1_ms | 377.2 | 409.9 | 377.2 | 372.2 |
| warm1_ms | 49.9 | 51.7 | 49.9 | 48.0 |
| open1_ws | 76.3 | 76.3 | 53.1 | 80.6 |
| hidden1_ws | 55.0 | 55.0 | 53.8 | 56.6 |
| ttl1_ws | 8.2 | 8.2 | 8.3 | 8.1 |
| cold2_ms | 379.6 | 412.7 | 379.6 | 346.9 |
| warm2_ms | 46.6 | 46.6 | 47.2 | 45.8 |
| open2_ws | 80.5 | 76.6 | 80.5 | 80.5 |
| hidden2_ws | 58.6 | 57.0 | 60.2 | 58.6 |
| ttl2_ws | 10.4 | 10.4 | 10.7 | 10.2 |
| cold3_ms | 389.6 | 389.6 | 375.6 | 515.2 |
| warm3_ms | 45.8 | 34.0 | 56.4 | 45.8 |
| open3_ws | 81.6 | 68.0 | 81.6 | 82.2 |
| hidden3_ws | 58.0 | 58.0 | 57.6 | 58.4 |
| ttl3_ws | 10.8 | 80.4 | 10.8 | 10.4 |
| peak_ws | 86.4 | 85.5 | 86.4 | 87.5 |
