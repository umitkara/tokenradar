# Adım 12n — Compact sayısı: indeksleyici, şema yükseltmesi, statusline gecikmesi ve CLI boyutu

**Tarih:** 2026-11-08 · **Dal:** `step/12n-statusline-compact` · **Karar:** ADR-054 · **Araçlar:** `examples/index_real.rs`, `examples/compact_scan.rs`, `scripts/measure-statusline.ps1`

## Özet

| Ölçüt | 12n | `main` (aynı oturum) | Bütçe |
|---|---|---|---|
| Tam indeksleme, sıcak önbellek (iki koşu) | 14,8 / 15,1 s (CPU 9,8 / 10,4 s) | 15,2 / 14,5 s (CPU 9,2 / 9,6 s) | ≤ 60 s ✅ |
| İlk kullanılabilir görünüm (RecentReady) | 0,64 / 0,62 s | 0,86 / 0,60 s | ≤ 5 s ✅ |
| İndeksleyici tepe private bellek | 58,1 / 51,1 MiB | 59,6 / 56,6 MiB | ≤ 120 MB ✅ |
| Şema 1 → 2 yükseltmesi (main DB'si üstüne 12n) | 14,5 s (CPU 9,5 s), her canlı dosya yeniden okundu; tepe private 61,5 MiB | — | ≤ 60 s ✅ |
| DB boyutu farkı (checkpoint sonrası) | +192 KiB | — | — |
| Statusline p95, uygulama açık, varsayılan config | **13,5 / 13,6 ms** | 13,6 / 13,4 ms | ≤ 30 ms ✅ |
| Statusline p95, uygulama kapalı, varsayılan config | **27,0 / 26,5 ms** | 25,5 / 24,6 ms | ≤ 40 ms ✅ |
| Statusline p95, açık, `compact` config (beş koşu) | 13,6 / 13,8 / 31,7 / 17,0 / 13,5 ms | 13,3 ms (öğeyi tanımıyor) | ≤ 30 ms ✅ (bkz. not) |
| Statusline p95, kapalı, `compact` config (beş koşu) | 26,4 / 27,8 / 75,2 / 26,9 / 28,6 ms | 29,1 ms | ≤ 40 ms ✅ (bkz. not) |
| `tokenradar.exe` (`release-cli`) | **2.481.664 B = 2,37 MiB** | 2.459.136 B = 2,35 MiB | ≤ 3 MB ✅ |

**Sonuç:**
- **İndeksleyici:** compact satırlarının ayrıştırılması ve yazılması süreyi değiştirmiyor; farklar koşular arası gürültü içinde. CPU ortalamada yaklaşık 0,7 s (%7) arttı. Sebebi satır başına bir `memmem` araması daha ve gürültü. Süre paralel iş parçacıklarında gizleniyor.
- **Yükseltme:** migration bütün canlı dosyaların okuma konumunu sıfırlıyor. Bu yüzden ilk açılış, bir tam indeksleme kadar sürüyor (14,5 s). Yükseltilmiş DB'nin parmak izi (`52d288de45e2b7ec`) aynı anda koşan 12n tam indekslemesiyle eşit; yani mevcut veriye dokunulmuyor, yeniden okumada tekrar eklenen kayıtlar dedup'ta düşüyor.
- **Statusline:** yeni öğe ve ayar dosyası okuma, gecikmede ölçülebilir fark yaratmıyor. Ayar dosyaları yalnızca `compact` öğesi config'te varsa okunuyor; varsayılan config'te hiç okunmuyor. Varsayılan "kapalı" modda 12n'in 1–2 ms yüksek görünmesi, 12m'de `main`'in kendi koşuları arasında görülen 23,6–32,1 ms aralığından küçük.
- **Boyut:** +22.528 B (22 KiB, %0,9): `auto_compact` modülü, `serde_json::Value` ile ayar okuma ve yeni Fluent anahtarları.
- **Not:** `compact` config'in üçüncü koşusunda iki mod birlikte yükseldi: açık p95 31,7 ms (p50 12,8), kapalı p95 75,2 ms (p50 26,1). İki ek koşu 17,0 / 13,5 ms ve 26,9 / 28,6 ms verdi. p50'ler değişmediği ve iki mod aynı koşuda birlikte saptığı için bu, çizimden değil, makinedeki geçici yükten geliyor (12i'deki 68,9 ms, 12k'daki 55,7 ms ve 12m'deki 62,2 ms gibi). Açık moddaki 31,7 ms bütçeyi 1,7 ms aşıyor ve tekrarlanmadı.

## Backfill kararının ölçümü

Eski transcript'lerdeki compact olaylarını almanın iki yolu ölçüldü (ham çıktı: `raw/2026-11-08-compact-scan.txt`):

| Yol | Süre | Not |
|---|---|---|
| Yalnız compact satırlarını arayan ayrı tarama (`compact_scan`, tek iş parçacığı, `memmem`) | 16,7 / 16,9 / 17,3 s (0,67–0,69 GiB/s) | Ayrı bir kod yolu ve ayrı bir durum gerektirir |
| Migration'da okuma konumlarını sıfırlayıp tam yeniden indeksleme | 14,5 s | Mevcut indeksleyiciyle aynı yol, dedup idempotent |

Ayrı tarama daha hızlı değil. Disk okuması iki yolda da baskın; tam yeniden indeksleme ise paralel. Kullanıcı tam yeniden indekslemeyi seçti (ADR-054).

## Makine ve yöntem

| Öğe | Değer |
|---|---|
| CPU | Intel Core i5-13420H (8 çekirdek / 12 thread) |
| Veri seti | Kullanıcının gerçek Claude Code dizini: 4.526 dosya, 11,58 GiB |
| İndeksleyici | `cargo run --release -p tokenradar-core --example index_real`, geçici DB'ye; sıra main/12n/main/12n, ardından main DB'sinin kopyası üstüne 12n `--keep` |
| Statusline | `cargo build --profile release-cli -p tokenradar-cli` (ADR-034); `.\scripts\measure-statusline.ps1 -NoBuild [-Cli <main exe>] [-Config compact.toml]`, mod başına 200 ölçülen ve 10 ısınma çağrısı |

- **Gerçek DB'ye dokunulmadı.** İndeksleyici koşuları `%TEMP%` altındaki geçici DB'lere yazdı. Statusline ölçümü betiğin portable kopyasıyla yapıldı; spool, config ve DB geçici dizindeydi. Şema 2'ye yükseltilmiş bir DB'yi kurulu v0.1.0 açamaz (ADR-054). Bu yüzden kurulu uygulamanın DB'si yalnızca okunmadı, hiç açılmadı.
- **Parmak izleri koşudan koşuya değişiyor**, çünkü ölçüm sırasında bu oturum transcript yazmaya devam etti. Karşılaştırma aynı anda koşan çiftler arasında yapıldı. Ham çıktılarda token toplamı, olay sayısı ya da yol yok.
- **A/B aynı oturumda koşuldu.** `main` (8ecf722) ayrı bir git worktree'de aynı profille derlendi. Statusline sırası: main/12n/main/12n varsayılan, 12n `compact` ×5, main `compact` ×1.
- **Sentetik veri genişletildi:** betik, geçici config dizinine tek bir `compact_boundary` satırı (trigger `auto`) içeren bir transcript ve `{"autoCompactWindow": 300000}` içeren bir `settings.json` yazıyor. Böylece "açık" modda `status.get` oturumu bulup sayıyı okuyor, iki modda da CLI ayar dosyalarını okuyor.
- **Varsayılan koşularda** satır 12m ile aynıydı: `5h 49% ⏱60% · 1h59m │ 7d 41.2% ⏱57.1% · 2d23h │ ctx 38% │ API $4.12`.
- **`compact` config:** `seven_day`, ayraç, `context`, ayraç, `compact` (`format = "split"`).
  - Açık: `7d 41.2% ⏱57.1% · 2d23h │ ctx 38% │ ⟲1 auto 0 manual · 200k`.
  - Kapalı: `7d 41.2% ⏱57.1% · 2d23h │ ctx 38% │ ⟲ 200k`. Uygulama yokken sayı bilinmiyor; yalnızca ayar gösteriliyor.
  - 300k pencere, payload'daki 200k bağlam penceresiyle sınırlandı.
  - `main`'in CLI'ı `compact` öğesini tanımadı; esnek okuma (ADR-042) onu atladı: `7d 41.2% ⏱57.1% · 2d23h │ ctx 38%`.
- **Eski CLI, yeni sunucu:** `main` CLI'ı 12n `ipc_serve` ile "açık" modda 13,4–13,6 ms ve hatasız çalıştı. Yeni `SessionInfo.compacts` alanını tanımıyor, yok sayıyor.
- **Renk ölçüme dahil değil** (`NO_COLOR=1`).
- **Kurulu uygulama statusline ölçümü boyunca kapatıldı** (kullanıcı onayıyla), sonra yeniden başlatıldı. "Kapalı" modda her koşuda 200, "açık" modda 0 spool satırı sayıldı.

## Ham çıktılar

- `raw/2026-11-08-compact-indexer.txt`: indeksleyici A/B ve yükseltme koşusu.
- `raw/2026-11-08-compact-scan.txt`: ayrı compact taramasının üç koşusu.
- `raw/2026-11-08-compact-statusline.txt`: statusline A/B koşuları.
