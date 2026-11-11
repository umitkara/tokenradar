# Adım 12q — Statusline payload alanları: CLI boyutu ve statusline gecikmesi

**Tarih:** 2026-11-11 · **Dal:** `step/12q-statusline-payload-fields` · **Karar:** ADR-059 · **Araç:** `scripts/measure-statusline.ps1`

## Özet

| Ölçüt | 12q | `main` (aynı oturum) | Bütçe |
|---|---|---|---|
| `tokenradar.exe` (`release-cli`) | **2.631.168 B = 2,51 MiB** | 2.577.920 B = 2,46 MiB | ≤ 3 MB ✅ |
| Statusline p95, uygulama açık, varsayılan satır (iki koşu) | 17,7 / 14,1 ms | 17,2 / 13,8 ms | ≤ 30 ms ✅ |
| Statusline p95, uygulama kapalı, varsayılan satır (iki koşu) | 26,7 / 27,5 ms | 30,1 / 28,7 ms | ≤ 40 ms ✅ |
| Statusline p95, uygulama açık, yeni öğeli satır (iki koşu) | 13,6 / 18,5 ms | 13,6 / 22,6 ms | ≤ 30 ms ✅ |
| Statusline p95, uygulama kapalı, yeni öğeli satır (iki koşu) | 28,0 / 31,5 ms | 26,7 / 86,7 ms | ≤ 40 ms ✅ (bkz. not) |

**Sonuç:**
- **Boyut:** CLI +53.248 B (+52 KiB). Kaynakları yeni payload struct'ları ve esnek okuma (`lax`, `seq_len`), sekiz öğe türü ve on bir biçimin okunması, yazılması ve çizilmesi ve yeni Fluent metinleri (EN ve TR). Bütçenin %84'ü kullanılıyor.
- **Gecikme:** Varsayılan satırda 12q ile `main` arasında ölçülebilir fark yok. Sentetik payload artık yeni alanların hepsini taşıyor, yani okuma maliyeti iki koşuda da ölçülen yolda; `main` bu alanları tanımadığı için atlıyor.
- **Yeni öğeli satır:** 12q iki satır çiziyor (harcama limiti, efor, cache süresi, kaçırmalar, yeniden cache, depo, PR, eklenen klasörler). `main` bu öğeleri tanımıyor ve atlıyor, yani iş yükleri aynı değil. Fark yine gürültü düzeyinde.
- **Not:** `main`'in son "kapalı" koşusundaki 86,7 ms (p50 30,2 ms, max 133,8 ms) ve aynı koşunun "açık" 22,6 ms'si makinedeki geçici yükten geliyor. 12i–12p'deki tek koşuluk sapmalar gibi tekrarlanmadı; 12q'nun son koşusu (31,5 / 18,5 ms) da aynı zaman diliminde biraz yükseldi.

## Makine ve yöntem

| Öğe | Değer |
|---|---|
| CPU | Intel Core i5-13420H (8 çekirdek / 12 thread) |
| Derleme | `cargo build --profile release-cli -p tokenradar-cli`, iki tarafta da. `main` (e3eb871) ayrı bir git worktree'de, ayrı target dizininde. |
| Ölçüm | `.\scripts\measure-statusline.ps1 -NoBuild -Cli <exe> [-Config <detailed-12q.toml>]`, mod başına 200 ölçülen ve 10 ısınma çağrısı, `NO_COLOR=1` |
| Payload | Betiğin sentetik payload'ı; 12q ile `effort`, `thinking`, `fast_mode`, `prompt_cache` (kaçırma nedeniyle), `pr`, `workspace.{repo, added_dirs}` ve `rate_limits.spend_limit` eklendi. Adlar sahte (`example/alpha`). |
| Varsayılan satır | `5h 49% ⏱ 60% · 1h59m │ 7d 41.2% ⏱ 57.1% · 2d23h │ ctx 38% │ API $4.12` (iki tarafta birebir aynı) |
| Yeni öğeli satır | Sentetik bir `[statusline]`, `color = "always"`: `… │ $21/$50 · 11d23h` ve ikinci satırda `Opus 5.5 · high │ cache →21:57 1h │ 2 misses · tools changed │ if cold 45k │ example/alpha │ #123 ✓ │ Alpha +2` |

- **A/B aynı oturumda, sırayla koşuldu:** main/12q/main/12q varsayılan, ardından 12q/main/12q/main yeni öğeli satır.
- **Kurulu uygulama ölçüm boyunca kapatıldı** (kullanıcı onayıyla), sonra yeniden başlatıldı. "Kapalı" modda her koşuda 200, "açık" modda 0 spool satırı sayıldı.
- Ham çıktıdaki satırlar ANSI kodlarından arındırıldı.

## Ham çıktılar

- `raw/2026-11-11-statusline-payload-fields.txt`: boyutlar ve sekiz statusline koşusu.
