# Adım 11c — Onboarding, boşta bellek (sentetik) ve boyut

**Tarih:** 2026-10-27 · **Sürüm:** 0.0.1 (`step/11c-onboarding`) · **Karar:** ADR-035

## Özet

| Ölçüt | Sonuç | Bütçe |
|---|---|---|
| Bellek, yalnızca tray, sentetik veri (180 s, release) | private working set **7,31–7,36 MB**, private bytes 8,6 MB, 1 süreç | ≤ 25 MB ✅ |
| CPU, aynı koşu | 180 s'de 0,016 s (0,031 → 0,047 s) | boşta ~0 ✅ |
| Bellek, onboarding penceresi açık (45 s) | private working set tepe **80,1 MB**, sonda 77,5 MB, 7 süreç (6 WebView2) | ≤ 130 MB ✅ |
| Panel JS (gzip) | **38,5 KB** (11b: 45,4 KB) · CSS 4,0 KB | ≤ 60 KB ✅ |
| Onboarding JS (gzip) | giriş 7,3 KB; ayarlarla paylaşılan `StatuslineDialog` ve ortak chunk'larla toplam ~47 KB | — |
| Uygulama ikilisi (release) | 12.672.000 B (12,08 MiB); 11b: 12.623.360 B | ≤ 15 MB ✅ |

**Sentetik:** Bu ölçümler gerçek kullanım değildir. Gerçek boşta ölçümünü kullanıcı yapar (aşağıda).

## Yöntem

- `target\release\tokenradar-app.exe` geçici bir klasöre `portable` işaretiyle kopyalandı. Veri o klasörün `data\` alt dizinine yazılır.
- `USERPROFILE` geçici bir eve yönlendirildi; `.claude\projects` `testdata/claude/projects`'ten kopyalandı. Keşif `USERPROFILE`'ı okuduğu için (`providers/claude/mod.rs`) gerçek `~/.claude` görülmedi.
- **Boşta:** `TOKENRADAR_SKIP_ONBOARDING=1`. `scripts/measure-memory.ps1 -IntervalMs 1000 -DurationSec 180`. İndeksleme fikstürlerde hemen bitiyor; WebView2 süreci hiç başlamadı.
- **Onboarding açık:** Veri silinip ortam değişkeni olmadan başlatıldı; onboarding kendiliğinden açıldı. 45 s ölçüldü, sonra süreç kapatıldı.
- Ham veri: `raw/2026-10-27-onboarding-idle-synthetic.csv`, `raw/2026-10-27-onboarding-open-synthetic.csv`.

## Yorum

- 11a/11b'de eklenen bildirim, autostart, dialog ve opener eklentileri ile log thread'i boşta belleği belirgin artırmadı: tray-only private working set 7,3 MB.
- Onboarding ve ayarlar pencerelerinin `warm_ttl` değeri 0'dır; kapanınca WebView2 süreçleri çıkar (Adım 02: ~0,4 s). Bu ölçümde kapatma senaryosu tekrarlanmadı.
- Gerçek veride indeksleme bittikten sonraki boşta bellek, veritabanı boyutuna ve canlı oturum sayısına bağlıdır; bu koşu bunu kapsamaz.

## Kullanıcının yapacağı gerçek boşta ölçümü

Agent oturumu açıkken Claude Code transcript yazdığı için ölçüm boşta sayılmaz. Bütün Claude Code oturumları kapalıyken:

```powershell
$p = Start-Process .\target\release\tokenradar-app.exe -PassThru
Start-Sleep 120   # indeksleme bitsin
.\scripts\measure-memory.ps1 -RootPid $p.Id -IntervalMs 1000 -DurationSec 600 -Out "$env:TEMP\tr-idle.csv"
```

Sonuçtan yalnızca bellek ve CPU özetini kaydedin; token toplamları, sayılar ve yollar git'e girmez.
