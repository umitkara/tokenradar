# Adım 12r — Canlı yenileme ve git durumu: CLI boyutu, statusline gecikmesi, boştaki oturum

**Tarih:** 2026-11-11 · **Dal:** `step/12r-live-refresh-git-status` · **Karar:** ADR-059 · **Araçlar:** `scripts/measure-statusline.ps1`, `scripts/measure-threads.ps1`

## Özet

| Ölçüt | 12r | `main` (aynı oturum) | Bütçe |
|---|---|---|---|
| `tokenradar.exe` (`release-cli`) | **2.662.400 B = 2,54 MiB** | 2.631.168 B = 2,51 MiB | ≤ 3 MB ✅ |
| Statusline p95, uygulama açık, varsayılan satır (iki koşu) | 20,7 / 20,3 ms | 15,6 / 24,3 ms | ≤ 30 ms ✅ |
| Statusline p95, uygulama kapalı, varsayılan satır (iki koşu) | 33,2 / 31,5 ms | 27,8 / 32,5 ms | ≤ 40 ms ✅ |
| Statusline p95, uygulama açık, `git_status` öğeli satır (iki koşu) | 20,6 / 21,6 ms | 22,0 / 22,4 ms | ≤ 30 ms ✅ |
| Statusline p95, uygulama kapalı, `git_status` öğeli satır (medyan) | 33,0 ms (11 koşu) | 30,7 ms (10 koşu) | ≤ 40 ms ✅ (bkz. not) |
| `git status --porcelain=v2` bu depoda (CLI beklemez) | p50 40,6 / p95 48,1 ms; `-uno` ile p50 37,5 / p95 67,3 ms | — | — |
| Açık ama boştaki oturum (sentetik, 10 dk): uyanma / CPU / store | kapalı: 14,4/dk, %0,002, 0 · 5 sn: 64,9/dk, %0,010, 4 · 5 sn + `git_status`: 68,2/dk, %0,013, 4 | — | kayıt (bkz. not) |

**Sonuç:**
- **Boyut:** CLI +31.232 B (+30,5 KiB): `git_status` öğesi ve üç biçimi, `countdown`, `untracked` anahtarı, spool kuyruk kontrolü ve yeni Fluent metinleri. Bütçenin %85'i kullanılıyor.
- **Uygulama açık:** 12r ile `main` aynı düzeyde, `git_status` ekliyken de. CLI git'i hiç beklemez; uygulama pipe yanıtını önbellekten verir, git arka planda en fazla 5 saniyede bir çalışır. `main` `git_status`'u tanımıyor ve atlıyor, yani iş yükleri aynı değil.
- **Uygulama kapalı:** Medyan p95 12r'de 33,0 ms, `main`'de 30,7 ms. Yeni olan tek iş spool dosyasının son 16 KiB'ının okunması (tekrar kontrolü).
- **Not (açık konu):** 12r'nin "kapalı" koşularından 3'ü (11'de) tümüyle yavaşladı: p95 59,2 / 65,0 / 65,1 ms, aynı koşularda p50 de 26–27 ms'ye çıktı. Tek tek çağrılar değil, koşunun tamamı yavaş; bu makine yükü epizoduna benziyor. Ama `main`'in 10 koşusunda görülmedi, bu yüzden gürültü diye kapatılmadı. Sonraki ölçüm adımında ya da spool kontrolünü kapatan bir A/B ile yeniden bakılmalı.
- **Spool tekrarı çalışıyor:** 12r'nin "kapalı" koşularında 200 değil 199 satır yazıldı. Betiğin son ısınma çağrısı ile ilk ölçülen çağrı aynı yüzdeyi aynı saniyede taşıyor (aynı içerik); ikincisi spool'a yazılmadı.
- **Boştaki oturum (sentetik):** 5 saniyelik yenilemede 126 çizim boyunca store'a yalnızca heartbeat yazdı: 10 dakikada 4 `limit_snapshots` satırı (iki heartbeat × iki pencere). Tekrarlar yazılmadı. Çekirdek süreci tek çekirdeğin %0,010'unu kullandı; uyanmaların çoğu çizim başına pipe turu (`core-ipc`, `core-limits`) ve API poller'ı.
- **`git_status` bedeli:** Çekirdeğin CPU'su %0,010'dan %0,013'e çıktı. 5 saniyelik TTL ve 5 saniyelik yenileme birleşince git neredeyse her çizimde çalışır: 10 dakikada ≈ 126 `git status`, her biri bu depoda ≈ 40 ms. `git.exe`'nin kendi CPU'su bu ölçüme girmez (ayrı süreç, thread ölçümü yalnızca çekirdek sürecini kapsar). Büyük depolarda `untracked = false` ya da 10/30 sn aralık önerilir.
- **Kapalıyken `core-ipc`:** Hiç istemci yokken 10 dakikada 131 uyanma (22 M döngü). Beklenen sıfıra yakın; 12r'de değişmeyen pipe sunucu döngüsünden geliyor olmalı. Sonraki ölçüm adımında `main` ile karşılaştırılmalı.
- **Gerçek boşta koşu kullanıcıda:** Ajan oturumu transcript yazdığı için boşta değildir; açık ama boştaki gerçek bir Claude Code oturumu ile 10 dakikalık ölçümü kullanıcı, 12r'yi içeren bir sürümle yapar.

## Makine ve yöntem

| Öğe | Değer |
|---|---|
| CPU | Intel Core i5-13420H (8 çekirdek / 12 thread) |
| Derleme | `cargo build --profile release-cli -p tokenradar-cli`, iki tarafta da. `main` (6e8d2ce) ayrı bir git worktree'de, ayrı target dizininde. |
| Statusline | `.\scripts\measure-statusline.ps1 -NoBuild -Cli <exe> [-Config <git-12r.toml> -Cwd <bu depo>] [-Mode off]`, mod başına 200 ölçülen ve 10 ısınma çağrısı, `NO_COLOR=1` |
| `git_status` satırı | Sentetik bir `[statusline]`: `five_hour`, `seven_day`, `context`, `git_branch`, `git_status` (ayraçsız). `-Cwd` bu depo, yani uygulama tarafında gerçek `git status` çalışıyor. |
| Git süresi | `git -C <depo> --no-optional-locks status --porcelain=v2 --branch -z [-uno]`, 5 ısınma + 50 ölçüm, PowerShell'den |
| Boştaki oturum | `ipc_serve` (gerçek `CoreRuntime`, geçici yapılandırma dizini ve veritabanı) + portable CLI kopyası, aynı payload'ı 5 saniyede bir çizen bir döngüyle (`Start-Job`). 20 sn yerleşme, ardından `scripts/measure-threads.ps1 -ProcessName ipc_serve -DurationSec 600`; store yazımı `limit_snapshots` satır sayısı farkıyla. `git_status` koşusunda öğeli yapılandırma ve bu depo. |

- **Kurulu uygulama ölçüm boyunca kapatıldı** (kullanıcı onayıyla), sonra yeniden başlatıldı.
- **A/B aynı oturumda, sırayla koşuldu:** main/12r/main/12r varsayılan, 12r/main/12r/main `git_status` satırı, ardından yalnızca "kapalı" mod için 5 ve 12 koşuluk iki dizi (ikincisi karışık sırayla).
- İlk 12r koşusu `main` arka planda derlenirken yapıldı (p95 açık 40,2, kapalı 80,6 ms) ve sayılmadı; ham çıktıda "Discarded" başlığıyla duruyor.
- Ham çıktıda dal adı ve git sayıları `<branch>` ve `<git status>` ile değiştirildi; ölçüm kayıtlarına repo adı, yol ya da dosya sayısı girmez.

## Ham çıktılar

- `raw/2026-11-11-live-refresh-git-status.txt`: boyutlar, statusline koşuları ve git süreleri.
- `raw/2026-11-11-live-refresh-git-status-idle.txt`: üç boşta koşusunun thread tabloları.
