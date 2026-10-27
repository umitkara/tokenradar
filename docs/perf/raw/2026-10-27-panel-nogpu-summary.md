# Panel lifecycle: 2026-10-27-panel-nogpu

Runs: 3; scene: ok; sampling every 250 ms; exe: tokenradar-app.exe; browser args: --disable-gpu

| Budget | Value | Limit | |
|---|---|---|---|
| Cold open (median of all rounds) | 489.7 ms | 400 ms | OVER |
| Warm open (median of all rounds) | 39.5 ms | 50 ms | ok |
| Panel open, private WS (max of round medians) | 77.0 MB | 130 MB | ok |
| warm_ttl + 10 s, private WS (max) | 4.3 MB | 30 MB | ok |

| Value | Median | Run 1 | Run 2 | Run 3 |
|---|---|---|---|---|
| baseline_ws | 1.9 | 1.9 | 1.9 | 1.8 |
| cold1_ms | 477.3 | 404.3 | 477.3 | 511.0 |
| warm1_ms | 35.5 | 40.5 | 27.8 | 35.5 |
| open1_ws | 74.3 | 55.7 | 76.9 | 74.3 |
| hidden1_ws | 53.8 | 56.3 | 53.4 | 53.8 |
| ttl1_ws | 3.9 | 4.0 | 3.9 | 3.9 |
| cold2_ms | 489.7 | 388.1 | 489.7 | 562.0 |
| warm2_ms | 40.8 | 40.8 | 39.5 | 43.4 |
| open2_ws | 76.3 | 77.0 | 76.3 | 73.6 |
| hidden2_ws | 53.8 | 56.1 | 53.8 | 53.1 |
| ttl2_ws | 4.0 | 3.8 | 4.1 | 4.0 |
| cold3_ms | 492.7 | 538.5 | 492.7 | 376.1 |
| warm3_ms | 38.8 | 37.1 | 38.8 | 39.7 |
| open3_ws | 74.1 | 74.1 | 73.6 | 74.4 |
| hidden3_ws | 53.9 | 53.8 | 53.9 | 55.4 |
| ttl3_ws | 4.2 | 3.9 | 4.3 | 4.2 |
| peak_ws | 82.9 | 82.9 | 82.9 | 81.2 |
