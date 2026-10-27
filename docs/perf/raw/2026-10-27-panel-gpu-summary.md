# Panel lifecycle: 2026-10-27-panel-gpu

Runs: 3; scene: ok; sampling every 250 ms; exe: tokenradar-app.exe

| Budget | Value | Limit | |
|---|---|---|---|
| Cold open (median of all rounds) | 432.5 ms | 400 ms | OVER |
| Warm open (median of all rounds) | 46.2 ms | 50 ms | ok |
| Panel open, private WS (max of round medians) | 121.6 MB | 130 MB | ok |
| warm_ttl + 10 s, private WS (max) | 4.8 MB | 30 MB | ok |

| Value | Median | Run 1 | Run 2 | Run 3 |
|---|---|---|---|---|
| baseline_ws | 1.9 | 1.9 | 1.9 | 1.9 |
| cold1_ms | 462.3 | 462.3 | 463.5 | 430.7 |
| warm1_ms | 46.2 | 52.4 | 46.2 | 40.2 |
| open1_ws | 118.6 | 120.5 | 118.6 | 118.1 |
| hidden1_ws | 92.8 | 95.1 | 92.3 | 92.8 |
| ttl1_ws | 4.1 | 4.1 | 3.9 | 4.1 |
| cold2_ms | 421.6 | 406.5 | 459.4 | 421.6 |
| warm2_ms | 40.0 | 58.1 | 35.1 | 40.0 |
| open2_ws | 118.3 | 121.6 | 118.3 | 93.5 |
| hidden2_ws | 94.9 | 99.1 | 94.9 | 92.8 |
| ttl2_ws | 4.5 | 4.5 | 4.4 | 4.5 |
| cold3_ms | 432.5 | 390.7 | 982.6 | 432.5 |
| warm3_ms | 47.5 | 48.3 | 37.6 | 47.5 |
| open3_ws | 118.0 | 118.0 | 96.9 | 118.6 |
| hidden3_ws | 94.4 | 98.5 | 94.4 | 92.3 |
| ttl3_ws | 4.6 | 4.8 | 4.5 | 4.6 |
| peak_ws | 128.3 | 131.6 | 128.2 | 128.3 |
