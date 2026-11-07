# Adım 12c — Statusline v2 gecikmesi ve CLI boyutu

**Tarih:** 2026-11-07 · **Dal:** `step/12c-statusline-v2` · **Karar:** ADR-042 · **Araç:** `scripts/measure-statusline.ps1`

## Özet

| Ölçüt | 12c (iki koşu) | `main` / 12b (iki koşu, aynı oturum) | Bütçe |
|---|---|---|---|
| p95, uygulama açık (IPC) | **12,0 / 13,4 ms** (p50 11,3 / 11,5) | 12,2 / 13,7 ms (p50 11,4 / 11,5) | ≤ 30 ms ✅ |
| p95, uygulama kapalı (spool) | **25,0 / 26,9 ms** (p50 19,9 / 20,3) | 25,0 / 24,9 ms (p50 20,7 / 20,0) | ≤ 40 ms ✅ |
| `tokenradar.exe` (`release-cli`) | **2.312.704 B = 2,21 MiB** | 2.294.272 B = 2,19 MiB | ≤ 3 MB ✅ |

**Sonuç:** Yeni satır modeli ve `[statusline]` okuması ölçülebilir bir gecikme eklemiyor. Fark, iki koşu arasındaki gürültünün içinde kalıyor (±2 ms). Boyut 18 KiB arttı.

## Makine ve yöntem

| Öğe | Değer |
|---|---|
| CPU | Intel Core i5-13420H (8 çekirdek / 12 thread) |
| Build | `cargo build --profile release-cli -p tokenradar-cli` (ADR-034) |
| Komut | `.\scripts\measure-statusline.ps1 -NoBuild [-Cli <eski exe>]`: mod başına 200 ölçülen çağrı ve 10 ısınma çağrısı |

- **A/B karşılaştırması aynı oturumda yapıldı.** `main` ayrı bir git worktree'de aynı profille derlendi. Ölçümler eski/yeni/eski/yeni sırasıyla art arda koşuldu. Betiğe bunun için `-Cli` parametresi eklendi. Betik artık `[CmdletBinding()]` kullanıyor, böylece yanlış yazılmış bir parametre sessizce yok sayılmıyor, hata veriyor.
- **Payload'a eklenenler:** `model.display_name`, `cost.total_cost_usd` ve `context_window.used_percentage`. Böylece yeni segmentler de ölçülen yoldan geçiyor. Ölçülen satır: `5h 49% · 1h59m │ 7d 41.2% · 2d23h │ ctx 38% │ API $4.12`.
- **Renk ve bar ölçüme dahil değil.** Betik `NO_COLOR=1` ile çalışıyor, bu yüzden bar ve ANSI kodları ölçülmedi. Bar 8 karakterlik bir döngü, maliyeti ihmal edilebilir.
- **Kurulu uygulama ölçüm boyunca kapatıldı (kullanıcı onayıyla, ~10 dk).** CLI'nin portable kopyası da kullanıcının gerçek pipe'ına bağlanır; pipe adı kullanıcı SID'inden türetiliyor ve bir override yok. Uygulama açık kalsaydı sentetik okumalar gerçek `limit_snapshots` tablosuna giderdi. "Kapalı" modda 200, "açık" modda 0 spool satırı sayıldı; yani okumalar gerçek uygulamaya değil, ölçüm ortamına gitti.

## Config okumanın maliyeti

CLI, `config.toml`'u zaten her çağrıda tümüyle okuyor (dil, zincir ayarları). `[statusline]` aynı ayrıştırmaya eklenen bir tablo. Ayrı bir dosya okuması ya da ayrı bir ayrıştırma yok. Planın "gerekirse yalnızca `[statusline]` tablosu ayrıştırılır" seçeneğine bu yüzden gerek kalmadı.
