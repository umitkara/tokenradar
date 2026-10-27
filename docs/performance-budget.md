# Performans Bütçesi

> Bu bütçeler **sürüm kapısıdır**. Bir sürüm, ilgili ölçümler bütçe içinde kalmadıkça yayınlanmaz.
> Ölçümler `scripts/measure-*.ps1` ile alınır. Sonuçlar `docs/perf/` altına tarih ve sürümle birlikte kaydedilir.
> **Referans makine:** Windows 11, 8+ çekirdek, NVMe SSD. Referans veri seti: ~11,7 GB / ~5.200 dosya / ~330 bin assistant satırı.

## Bellek (private working set — tüm alt süreçler dahil)

| Durum | Bütçe | Not |
|---|---|---|
| Yalnızca tray (pencere yok, indeksleme bitmiş) | **≤ 25 MB** | Ana hedef. "Görünmez ayak izi" vaadi bu değere dayanıyor. |
| Tray + ilk indeksleme sürüyor | ≤ 120 MB (tepe) | İndeksleme bitince ≤ 25 MB'a dönmeli. Adım 02: 22 MiB'lık satırlar biriktirilirse 4 worker ~165 MiB'a çıkıyor. Uzun satırlar biriktirilmeden atlanmalı (Adım 04). |
| Panel açık | ≤ 130 MB | WebView2 süreçleri dahil |
| Ana pencere açık (v0.2) | ≤ 180 MB | Büyük listeler sanallaştırılmış |
| Tüm pencereler kapandıktan `warm_ttl + 10 s` sonra | ≤ 30 MB | **Adım 02'de doğrulandı:** WebView2 süreçleri `destroy()` sonrası ~0,4 s'de çıkıyor, 60 s sonra 3,6 MB (ADR-023). |
| Widget açık (v0.3) | Tray + ≤ 45 MB | Aşılırsa native çizim yedek planı devreye girer (ADR-011) |

## CPU

| Durum | Bütçe |
|---|---|
| Boşta (oturum yok) | 10 dk ortalaması < %0,1. Saniyede gereksiz uyanma (wakeup) yok. |
| Aktif oturumlar yazarken | Debounce partisi başına ≤ 50 ms CPU, ortalama < %1 |
| İlk indeksleme | Arka plan önceliğinde, en fazla `min(4, çekirdek/2)` thread |

## Gecikme

| İşlem | Bütçe |
|---|---|
| Uygulama başlangıcından tray ikonunun görünmesine | ≤ 300 ms |
| Panel açılışı — sıcak (warm_ttl içinde) | ≤ 50 ms |
| Panel açılışı — soğuk (webview oluşturma dahil) | ≤ 400 ms |
| Ana pencere açılışı — soğuk | ≤ 600 ms |
| JSONL'e yeni satır yazılmasından UI/tray güncellemesine | ≤ 2 s (debounce dahil) |
| `tokenradar statusline` (daemon açık) | p95 ≤ 30 ms |
| `tokenradar statusline` (daemon kapalı, spool) | p95 ≤ 40 ms |

## İndeksleme

| İşlem | Bütçe |
|---|---|
| İlk indeksleme — referans veri seti, soğuk disk önbelleği | ≤ 60 s. Adım 02'de yalnızca parse 12,6 s ölçüldü (arka plan önceliği, 4 worker). SQLite yazımıyla birlikte Adım 04'te doğrulanır. |
| İlk indeksleme — kullanılabilir ilk görünüm (mevcut pencere + bu hafta) | ≤ 5 s (en yeni dosyalardan başlanır) |
| Yeniden başlatma sonrası artımlı tarama (değişmemiş 5.200 dosya) | ≤ 500 ms (yalnızca metadata/stat). Adım 02: dizin yürüyüşü + `DirEntry::metadata` 177 ms. |

## Disk ve boyut

| Öğe | Bütçe |
|---|---|
| Installer (NSIS) | ≤ 12 MB |
| `tokenradar-app.exe` | ≤ 15 MB |
| `tokenradar.exe` (CLI) | ≤ 3 MB |
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
- **CPU:** Aynı script, 10 dakikalık boşta CPU süresi farkını ölçer.
- **İndeksleme:** `cargo bench -p tokenradar-core --bench indexer` (sentetik) ve `tokenradar-app --measure-index <dir>` (gerçek veri). Soğuk ölçümden önce dosya önbelleği temizlenir (RAMMap "Empty Standby List" veya yeniden başlatma).
- **Statusline:** `scripts/measure-statusline.ps1`, 200 çağrı yapıp p50/p95 değerlerini raporlar.
- Sonuçlar `docs/perf/YYYY-MM-DD-<sürüm>.md` dosyasına tablo olarak işlenir. Gerileme olursa bir issue açılır.
