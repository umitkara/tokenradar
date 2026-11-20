# Performans Bütçesi

> Bu bütçeler **sürüm kapısıdır**. Bir sürüm, ilgili ölçümler bütçe içinde kalmadıkça yayınlanmaz.
> Ölçümler `scripts/measure-*.ps1` ile alınır. Sonuçlar `docs/perf/` altına tarih ve sürümle birlikte kaydedilir.
> **Referans makine:** Windows 11, 8+ çekirdek, NVMe SSD. Referans veri seti: ~11,7 GB / ~5.200 dosya / ~330 bin assistant satırı.

## Bellek (private working set — tüm alt süreçler dahil)

| Durum | Bütçe | Not |
|---|---|---|
| Yalnızca tray (pencere yok, indeksleme bitmiş) | **≤ 25 MB** | Ana hedef. "Görünmez ayak izi" vaadi bu değere dayanıyor. |
| Tray + ilk indeksleme sürüyor | ≤ 120 MB (tepe) | İndeksleme bitince ≤ 25 MB'a dönmeli. Adım 12o: Nerd Font kurulumu (tek seferlik, kullanıcı başlatır) xz çıkarması için 0,3 s boyunca tepe private 102 MiB ekler, ardından geri verir (ADR-055). Adım 02: 22 MiB'lık satırlar biriktirilirse 4 worker ~165 MiB'a çıkıyor. Adım 04: 4 MiB üstü satırlar biriktirilmeden atlanıyor; indeksleyici süreci tepe private 50-57 MiB (tray ve UI hariç). |
| Panel açık | ≤ 130 MB | WebView2 süreçleri dahil |
| Ana pencere açık (v0.2) | ≤ 180 MB | Büyük listeler sanallaştırılmış. Adım 15b (Genel Bakış): en fazla 129,2 MB |
| Tüm pencereler kapandıktan `warm_ttl + 10 s` sonra | ≤ 30 MB | **Adım 02'de doğrulandı:** WebView2 süreçleri `destroy()` sonrası ~0,4 s'de çıkıyor, 60 s sonra 3,6 MB (ADR-023). |
| Widget açık (v0.3) | Tray + ≤ 8 MB | Baştan native çizilir: `tiny-skia` + layered window, WebView yok (ADR-058). Yüzen ve görev çubuğu modu için aynı bütçe. Adım 18 (yüzen): +1,73 MB. |

## CPU

| Durum | Bütçe |
|---|---|
| Boşta (oturum yok) | 10 dk ortalaması < %0,1. Saniyede gereksiz uyanma (wakeup) yok. |
| Açık ama boştaki oturum, canlı yenileme 5 sn (Adım 12r) | Bütçe değil, kayıt: yenileme kullanıcının seçimidir ve yalnızca oturum açıkken çalışır. Aynı payload store'a yazılmaz (heartbeat hariç). Sentetik (10 dk): kapalı 14,4 uyanma/dk ve %0,002; 5 sn 64,9/dk ve %0,010, store'a yalnızca heartbeat; `git_status` ile 68,2/dk ve %0,013, artı ≈ 126 `git status` süreci (`docs/perf/2026-11-11-live-refresh-git-status.md`). Gerçek oturumla ölçüm kullanıcıda. |
| Aktif oturumlar yazarken | Debounce partisi başına ≤ 50 ms CPU, ortalama < %1 |
| İlk indeksleme | En fazla `min(4, çekirdek/2)` thread. Son 7 günün dosyaları normal, eskileri arka plan önceliğinde (hibrit, ADR-026). |

## Gecikme

| İşlem | Bütçe |
|---|---|
| Uygulama başlangıcından tray ikonunun görünmesine | ≤ 300 ms |
| Panel açılışı — sıcak (warm_ttl içinde) | ≤ 50 ms |
| Panel açılışı — soğuk (webview oluşturma dahil) | ≤ 500 ms. ADR-037 (önce 400 ms). v0.1.0: medyan 440 ms (`docs/perf/2026-10-28-v0.1.0.md`) |
| Ana pencere açılışı — soğuk | ≤ 600 ms. Adım 15b: ikinci süreç başlatma dahil medyan 434 ms, webview oluşturmadan ilk boyamaya 72,6 ms, veri boyanması 161 ms (`docs/perf/2026-11-20-main-window.md`) |
| Ana pencere rota geçişi | ≤ 100 ms. Adım 15b: medyan 9,5 ms, Genel Bakış'a veriyle 42,7 ms |
| JSONL'e yeni satır yazılmasından UI/tray güncellemesine | ≤ 2 s (debounce dahil) |
| `tokenradar statusline` (daemon açık) | p95 ≤ 30 ms. Adım 07: 13,3 ms (`docs/perf/2026-10-27-statusline.md`). Adım 12c: 12,0-13,4 ms (`docs/perf/2026-11-07-statusline-v2.md`). Adım 12i: 15,0-16,2 ms, main 13,8-14,9 ms aynı oturumda (`docs/perf/2026-11-08-statusline-v3.md`). Adım 12k: 12,6-12,8 ms, main 12,6-13,2 ms aynı oturumda; Split + `COLUMNS=120` 12,9-13,1 ms (`docs/perf/2026-11-08-statusline-layout.md`). Adım 12m: 12,3-12,4 ms, main 12,2-12,4 ms aynı oturumda; yeni öğelerle 12,4-13,0 ms (`docs/perf/2026-11-08-statusline-enrichment.md`). Adım 12n: 13,5-13,6 ms, main 13,4-13,6 ms aynı oturumda; `compact` öğesiyle 13,5-17,0 ms, bir koşuda tekrarlanmayan 31,7 ms (`docs/perf/2026-11-08-statusline-compact.md`). Adım 12q: 14,1-17,7 ms, main 13,8-17,2 ms aynı oturumda; yeni öğeli iki satırla 13,6-18,5 ms (`docs/perf/2026-11-11-statusline-payload-fields.md`). Adım 12r: 20,3-20,7 ms, main 15,6-24,3 ms aynı oturumda; `git_status` öğesiyle (gerçek git, bu depo) 20,6-21,6 ms, main 22,0-22,4 ms (`docs/perf/2026-11-11-live-refresh-git-status.md`) |
| `tokenradar statusline` (daemon kapalı, spool) | p95 ≤ 40 ms. Adım 07: 25,5 ms. Adım 12c: 25,0-26,9 ms. Adım 12i: 29,1-29,9 ms. Adım 12k: 23,5-25,5 ms, main 24,4-26,9 ms aynı oturumda. Adım 12m: 21,3-24,3 ms, main 23,6-32,1 ms aynı oturumda; yeni öğelerle 24,7-30,7 ms, bir koşuda tekrarlanmayan 62,2 ms. Adım 12n: 26,5-27,0 ms, main 24,6-25,5 ms aynı oturumda; `compact` öğesiyle 26,4-28,6 ms, bir koşuda tekrarlanmayan 75,2 ms. Adım 12q: 26,7-27,5 ms, main 28,7-30,1 ms aynı oturumda; yeni öğeli iki satırla 28,0-31,5 ms, main'de bir koşuda tekrarlanmayan 86,7 ms. Adım 12r: 31,5-33,2 ms, main 27,8-32,5 ms aynı oturumda; `git_status` öğeli satırda 11 koşunun medyanı 33,0 ms (main 10 koşuda 30,7 ms), 12r'nin 3 koşusu tümüyle yavaşladı (p95 59-65 ms, p50 de yükseldi), main'de görülmedi; nedeni açık konu |

## İndeksleme

| İşlem | Bütçe |
|---|---|
| İlk indeksleme — referans veri seti, soğuk disk önbelleği | ≤ 60 s. Adım 04 (parse + SQLite, hibrit öncelik): **sıcak önbellekte 3,7-6,5 s**. Soğuk ölçülemedi; Adım 02'nin önbelleksiz okuma hızlarından tahmin **≈ 10 s** (`docs/perf/2026-10-27-indexer.md`). Adım 12n: veri seti büyüdü (11,58 GiB), sıcak önbellekte 14,5-15,2 s, main ile aynı. Şema 1 → 2 yükseltmesi her dosyayı bir kez yeniden okur: 14,5 s (`docs/perf/2026-11-08-statusline-compact.md`). |
| İlk indeksleme — kullanılabilir ilk görünüm (mevcut pencere + bu hafta) | ≤ 5 s (en yeni dosyalardan başlanır). Adım 04: **sıcak önbellekte 1,1-2,4 s**, soğuk tahmini **≈ 2-3 s**. Yeniden başlatma sonrası soğuk ölçüm açık iş. |
| Yeniden başlatma sonrası artımlı tarama (değişmemiş 5.200 dosya) | ≤ 500 ms (yalnızca metadata/stat). Adım 02: dizin yürüyüşü + `DirEntry::metadata` 177 ms. Adım 04 (envanter + `source_files` karşılaştırması): 107-123 ms, sıcak metadata önbelleği. |

## Disk ve boyut

| Öğe | Bütçe |
|---|---|
| Installer (NSIS) | ≤ 12 MB |
| `tokenradar-app.exe` | ≤ 15 MB. Adım 11b: 12,04 MiB. Adım 12o: 13,81 MiB (`lzma-rust2`, `ttf-parser`, +314 KiB). Adım 18: 14,22 MiB (DirectWrite/Direct2D/WIC özellikleri dahil) |
| `tokenradar.exe` (CLI) | ≤ 3 MB. Adım 07: 2,95 MiB. Adım 11b: doctor ile `--release` 3,14 MiB; `release-cli` profili (`opt-level = "s"`, ADR-034) ile 2,17 MiB. Adım 12c: 2,21 MiB. Adım 12i: 2,30 MiB. Adım 12k: 2,33 MiB (`unicode-width`, +28,5 KiB). Adım 12m: 2,35 MiB. Adım 12n: 2,37 MiB. Adım 12o: 2,37 MiB (+1,5 KiB). Adım 12p: 2,37 MiB (simge setleri ve iki bar stili, +5 KiB). Adım 12q: 2,51 MiB, main 2,46 MiB (yeni payload alanları, sekiz öğe, on bir biçim, +52 KiB) |
| Frontend bundle (panel sayfası, gzip) | ≤ 60 KB JS + ≤ 15 KB CSS |
| Frontend bundle (ana pencere, gzip) | ≤ 180 KB JS |
| Veritabanı büyümesi | ≤ 30 MB/ay (referans kullanım). Ham olaylar 90 gün sonra silinir, `usage_5m` kalıcıdır. |
| Loglar | ≤ 20 MB toplam |

## Ağ

| Öğe | Bütçe |
|---|---|
| S2 çağrıları | En fazla 1/dk (sert sınır). Taze S1 verisi varken tipik olarak 15 dk'da 1 |
| Güncelleme kontrolü | Günde ≤ 1 |

## Ölçüm yöntemi

- **Bellek:** `scripts/measure-memory.ps1`, `tokenradar-app` sürecini ve **tüm alt süreçlerini** (`msedgewebview2.exe` ağacı) ebeveyn bağlantıları üzerinden bulur. Private working set (bütçenin esası), private bytes (`PrivateMemorySize64`), working set ve CPU süresi toplamlarını CSV'ye yazar. Senaryo adımları: başlat → 60 s bekle → paneli aç → 10 s → paneli kapat → `warm_ttl + 10 s` → tekrar ölç. Pencere yaşam döngüsü senaryosu için `scripts/spike-lifecycle.ps1` (Adım 02) kullanılır.
- **CPU:** `scripts/measure-threads.ps1 -DurationSec 600`, boşta CPU'yu thread başına context switch (uyanma) ve CPU döngüsü (`QueryThreadCycleTime`) olarak ölçer. `measure-memory.ps1`'in CPU sütunu 15,6 ms'lik saat tikiyle örneklenir; kısa ve sık uyanmaları doğru göstermez, yalnızca kaba göstergedir (ADR-038).
- **İndeksleme:** `cargo bench -p tokenradar-core --bench indexer` (sentetik) ve `tokenradar-app --measure-index <dir>` (gerçek veri). Soğuk ölçümden önce dosya önbelleği temizlenir (RAMMap "Empty Standby List" veya yeniden başlatma).
- **Statusline:** `scripts/measure-statusline.ps1`, 200 çağrı yapıp p50/p95 değerlerini raporlar.
- Sonuçlar `docs/perf/YYYY-MM-DD-<sürüm>.md` dosyasına tablo olarak işlenir. Gerileme olursa bir issue açılır.
