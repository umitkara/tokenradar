# Panel lifecycle: 2026-10-27-panel

Runs: 3; scene: ok; sampling every 250 ms; exe: tokenradar-app.exe

| Budget | Value | Limit | |
|---|---|---|---|
| Cold open (median of all rounds) | 406.9 ms | 400 ms | OVER |
| Warm open (median of all rounds) | 47.3 ms | 50 ms | ok |
| Panel open, private WS (max of round medians) | 76.6 MB | 130 MB | ok |
| warm_ttl + 10 s, private WS (max) | 4.5 MB | 30 MB | ok |

| Value | Median | Run 1 | Run 2 | Run 3 |
|---|---|---|---|---|
| baseline_ws | 1.9 | 1.9 | 1.9 | 1.9 |
| cold1_ms | 437.9 | 437.9 | 465.4 | 437.1 |
| warm1_ms | 50.8 | 50.8 | 29.3 | 56.2 |
| open1_ws | 73.7 | 73.6 | 73.7 | 74.0 |
| hidden1_ws | 53.6 | 53.6 | 53.2 | 55.8 |
| ttl1_ws | 4.0 | 4.1 | 3.5 | 4.0 |
| cold2_ms | 392.7 | 385.9 | 392.7 | 392.9 |
| warm2_ms | 37.3 | 36.0 | 54.7 | 37.3 |
| open2_ws | 74.0 | 74.0 | 73.2 | 74.1 |
| hidden2_ws | 54.1 | 54.1 | 52.2 | 55.5 |
| ttl2_ws | 4.3 | 4.3 | 3.7 | 4.5 |
| cold3_ms | 406.9 | 491.5 | 406.9 | 399.9 |
| warm3_ms | 47.3 | 43.3 | 52.2 | 47.3 |
| open3_ws | 73.3 | 76.6 | 73.3 | 51.1 |
| hidden3_ws | 53.1 | 53.9 | 53.1 | 51.9 |
| ttl3_ws | 4.2 | 4.4 | 4.0 | 4.2 |
| peak_ws | 80.5 | 80.5 | 80.3 | 80.6 |
