# Adım 12m — Statusline zenginleştirme: gecikme ve CLI boyutu

**Tarih:** 2026-11-08 · **Dal:** `step/12m-statusline-zenginlestirme` · **Karar:** ADR-053 · **Araç:** `scripts/measure-statusline.ps1`

## Özet

| Ölçüt | 12m, varsayılan config (iki koşu) | `main` (iki koşu, aynı oturum) | 12m, zengin config (dört koşu) | Bütçe |
|---|---|---|---|---|
| p95, uygulama açık (IPC) | **12,3 / 12,4 ms** (p50 11,6 / 11,6) | 12,2 / 12,4 ms (p50 11,6 / 11,5) | 12,4 / 12,4 / 13,0 / 12,6 ms (p50 11,7–11,8) | ≤ 30 ms ✅ |
| p95, uygulama kapalı (spool) | **24,3 / 21,3 ms** (p50 19,8 / 18,1) | 32,1 / 23,6 ms (p50 21,7 / 19,4) | 24,9 / 62,2 / 30,7 / 24,7 ms (p50 19,5–25,2) | ≤ 40 ms ✅ (bkz. not) |
| `tokenradar.exe` (`release-cli`) | **2.459.136 B = 2,35 MiB** | 2.440.704 B = 2,33 MiB | — | ≤ 3 MB ✅ |

**Sonuç:**
- Yeni öğeler, biçimler ve alanlar gecikmede ölçülebilir bir fark yaratmıyor. Varsayılan config'de 12m ile `main` arasındaki farklar koşular arasındaki gürültüden küçük.
- Boyut 18.432 B (18 KiB, %0,8) arttı: yeni öğelerin render kodu, Windows saat dilimi çağrısı ve yeni Fluent anahtarları.
- **Not:** Zengin config'in ikinci koşusunda "kapalı" p95 62,2 ms çıktı (p50 25,2, max 70,3). Hemen ardından iki koşu 30,7 ve 24,7 ms verdi; aynı config'le `main` 24,7 ms. Tek seferlik bir yük olarak değerlendirildi (12i'deki 68,9 ms ve 12k'daki 55,7 ms gibi). "Açık" mod aynı koşuda 12,4 ms idi; yani satırın çizimi değil, spool yolundaki disk ya da süreç başlatma gecikmesi.

## Makine ve yöntem

| Öğe | Değer |
|---|---|
| CPU | Intel Core i5-13420H (8 çekirdek / 12 thread) |
| Build | `cargo build --profile release-cli -p tokenradar-cli` (ADR-034) |
| Komut | `.\scripts\measure-statusline.ps1 -NoBuild [-Cli <main exe>] [-Config rich.toml]`; mod başına 200 ölçülen çağrı, 10 ısınma çağrısı |

- **A/B aynı oturumda koşuldu.** `main` (23eac48) ayrı bir git worktree'de aynı profille derlendi. Sıra main/12m/main/12m idi; ardından dört zengin koşu ve `main` ile bir zengin koşu geldi.
- **Sentetik payload genişletildi:** `session_name`, `output_style.name`, `cost.total_duration_ms`, `context_window.context_window_size` ve `context_window.current_usage` eklendi. Değerler sentetik.
- **Varsayılan koşularda** `[statusline]` tablosu yoktu; satır 12k ile aynıydı: `5h 49% ⏱60% · 1h59m │ 7d 41.2% ⏱57.1% · 2d23h │ ctx 38% │ API $4.12`.
- **Zengin config** yeni öğelerin hepsini ve yeni alanları kullandı: `seven_day` (`reset_format = "clock"`, `bar_width = 12`, `bar_style = "dots"`), `daily_budget`, `context` (`tokens_left`), `burn_rate`, `cache_ratio`, `flex`, `session_name`, `output_style`. Böylece her çağrı yerel saat çevrimi ve yeni türetimlerden geçti. Ölçülen satır: `7d 41.2% ⏱57.1% · Wed 22:02 │ 20%/day │ ctx 124k left │ API $12.01/h │ cache 82% synthetic sample │ default`.
  - Betik `NO_COLOR=1` ile çalıştığı için çubuk çizilmiyor (ADR-049); 12 hücrelik noktalı çubuk ölçüme dahil değil. Çubuk yalnızca hücre dizisidir, maliyeti ihmal edilebilir.
  - `COLUMNS` verilmedi; `flex` tek boşluk oldu ve `fit()` atlandı.
  - Aynı config'le `main`'in CLI'ı yeni öğeleri ve alanları tanımadı, esnek okuma bunları atladı: `7d 41.2% ⏱57.1% · 2d23h │ ctx 38%`. Eski build uyumunun beklenen davranışı.
- **Renk ölçüme dahil değil** (`NO_COLOR=1`).
- **Kurulu uygulama ölçüm boyunca kapatıldı** (kullanıcı onayıyla). "Kapalı" modda her koşuda 200, "açık" modda 0 spool satırı sayıldı. Ölçüm portable kopyayla yapıldı; gerçek spool ve pipe'a dokunulmadı.
