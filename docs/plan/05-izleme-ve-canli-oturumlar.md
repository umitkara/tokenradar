# Adım 05 — Dosya izleme, canlı oturumlar ve olay veri yolu

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1 | 04 | 1 oturum | ✅ |

## Amaç

Çekirdeği "yaşayan" bir sisteme dönüştürmek:
- JSONL dosyalarındaki değişiklikler ≤ 2 s içinde indekslenir.
- Çalışan Claude Code oturumları canlı izlenir (busy/idle).
- Tüm değişiklikler tipli bir `EventBus` üzerinden yayınlanır.
- `CoreRuntime` Tauri uygulamasının içinde başlatılır.

## Önce oku

- `docs/architecture.md` §2 (başlangıç sırası), §5, §15
- `docs/data-sources.md` S3, S4, S5
- `docs/performance-budget.md` (CPU ve gecikme bölümleri)

## Kapsam

- `watcher/`: `notify` + debouncer; kirli dosya kuyruğu; taşma ve kaçırılan olaylara karşı periyodik stat taraması.
- `live_sessions/`: `sessions/*.json` okuma, pid ve oluşturma zamanı doğrulaması, durum değişikliklerinin izlenmesi.
- `.credentials.json` **yalnızca mtime değişikliği** için izlenir (Adım 06'daki 401 kurtarması için). İçerik okunmaz.
- `events.rs`: `CoreEvent` enum'u + `EventBus` (`tokio::sync::broadcast`).
- `runtime.rs`: `CoreRuntime::start(config) -> CoreHandle` ve düzenli kapanış (graceful shutdown).
- Tauri entegrasyonu:
  - `CoreRuntime` uygulama başlangıcında çalışır.
  - Geçici olarak, olayların tray tooltip'ine yazıldığı basit bir debug gösterimi eklenir.

## Kapsam dışı

- Limit kaynakları (Adım 06), UI bileşenleri (Adım 09-10), tray ikon çizimi (Adım 08).

## Görevler

- [x] `CoreEvent` tasarımı (serde + specta):
  - `IndexProgress`, `IndexMilestone(RecentReady | Complete)`
  - `UsageAppended { account, sessions: Vec<SessionId>, from_ts, to_ts }`
  - `LiveSessionsChanged(Vec<LiveSession>)`
  - `CredentialsChanged { account }`
  - `SchemaDrift { source, unknown_count }`
  - İleride eklenecekler için `#[non_exhaustive]`.
- [x] `EventBus`: kapasite 256. Gecikmeli (lagging) alıcılar için politika: alıcı `Lagged` hatası alırsa tam durum tazelemesi ister. Bu politika bir örnek testle gösterilsin.
- [x] `watcher`:
  - İzlenecekler: `<config>/projects` (özyinelemeli), `<config>/sessions` ve `<config>/.credentials.json`'ın üst dizini (dosya adına göre filtrelenir).
  - Debounce süresi 750 ms. Aynı dosya için gelen olaylar birleştirilir. Kuyruk `HashSet<PathBuf>` olarak tutulur.
  - Yalnızca `.jsonl` uzantılı ve `classify_path` ile kabul edilen dosyalar indeksleyiciye gider.
  - Taşma (`ReadDirectoryChangesW` buffer overflow → `notify` "rescan" olayı) durumunda tam stat taraması yapılır.
  - **Güvenlik ağı:** 10 dakikada bir stat taraması yapılır (yalnızca metadata okunur, maliyeti düşük).
- [x] `live_sessions`:
  - `sessions/*.json` dosyaları parse edilir (`.key` dosyalarına **dokunulmaz**).
  - Pid canlılığı Windows'ta `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` ve `GetProcessTimes` ile doğrulanır. Oluşturma zamanı (FILETIME) dosyadaki `procStart` ile karşılaştırılır; bu, **pid yeniden kullanımına** karşı korur. `procStart` değeri FILETIME (100 ns, 1601 tabanlı) biçiminde görünüyor; Adım 04'teki gibi önce gerçek veride doğrula.
  - Ölü süreçlere ait dosyalar listeden çıkarılır, ama silinmez (dosya Claude Code'a ait).
  - Çıktı: `LiveSession { account, session_uuid, pid, cwd, name, status, started_at, status_updated_at }` ve DB'deki `SessionId` eşlemesi (varsa).
  - Durum değişimi olayı yalnızca liste veya durum gerçekten değiştiğinde yayınlanır.
- [x] İndeksleyici entegrasyonu: Watcher → kirli dosyalar → `indexer.index_files(..)` → `UsageAppended` olayı. Başlangıçta `full_scan()` 2 s gecikmeyle çalışır (ADR'deki başlangıç sırası).
- [x] `CoreRuntime`:
  - Tokio runtime (Tauri'nin runtime'ı kullanılabilir).
  - Görevler: watcher, indexer worker'ları, live-session izleyici, saklama zamanlayıcısı (günlük).
  - `CoreHandle`: `subscribe()`, `query(...)` (okuma API'leri için cephe), `shutdown()`.
- [x] Tauri tarafı: `setup` içinde `CoreRuntime::start`. `RunEvent::Exit` olayında `shutdown` çağrılır (en fazla 2 s beklenir). Geçici debug: tray tooltip'inde "N canlı oturum · son indeks HH:MM:SS".
- [x] Testler:
  - Geçici dizinde dosyaya satır ekle → `UsageAppended` ≤ 2 s içinde gelir (CI'da zaman toleransı geniş tutulur).
  - Birleştirme: 100 hızlı yazma → tek bir indeksleme turu.
  - Canlı oturum: sahte bir oturum dosyası + mevcut olmayan bir pid → listeye girmez.
  - Kapanış: `shutdown` sonrası hiçbir thread çalışmaya devam etmez.
- [x] **Ölçüm:**
  - Bir Claude Code oturumu aktifken 10 dakika boyunca CPU ortalaması.
  - Boşta 10 dakika CPU.
  - Sonuçları `docs/perf/` altına kaydet.

## Teknik notlar ve tuzaklar

- Claude Code streaming sırasında aynı dosyaya saniyede birkaç kez yazar. Debounce olmadan her yazma bir indeksleme turu tetikler ve CPU bütçesi aşılır.
- `notify` Windows'ta özyinelemeli izleme için tek bir `ReadDirectoryChangesW` handle'ı kullanır. 1.500 alt dizin bu yapıda sorun çıkarmaz.
- Tauri uygulaması pencere açmadan çalışırken bile olay döngüsü aktiftir. Çekirdek görevleri Tauri'nin ana thread'ini **asla** bloklamamalı.
- `sessions/` dizinindeki dosya adı pid'dir. Pid formatı ve alanlar sürümler arasında değişebilir; parser toleranslı olmalı.

## Kabul kriterleri

- [x] Aktif bir Claude Code oturumu yazarken ≤ 2 s içinde `UsageAppended` olayı gelir (manuel test, zaman damgalı log). _Entegrasyon testinde 685 ms; gerçek veride `TOKENRADAR_DEBUG_EVENTS` günlüğüyle izlendi._
- [ ] Boşta CPU 10 dakika ortalaması < %0,1. Aktif oturum varken ortalama < %1. _Aktif %0,038 ✅, boşta yalnızca sentetik %0,013; gerçek boşta ölçümü açık._
- [x] Canlı oturum listesi Claude Code oturumu açılıp kapandıkça doğru güncellenir. Durum değişimi (busy ↔ idle) ≤ 2 s içinde yansır.
- [ ] Uygulamadan çıkışta süreç ≤ 2 s içinde sonlanır. _Çekirdek kapanışı testle doğrulandı; tray "Quit" ile manuel kontrol açık._

## Doğrulama

```powershell
cargo test -p tokenradar-core watcher live_sessions runtime
pnpm dev   # Claude Code'da bir oturum aç, tray tooltip'ini izle
.\scripts\measure-memory.ps1 -ProcessName tokenradar-app -DurationSec 600 -Cpu
```

## Devir notu

**Tarih:** 2026-10-27 · **Dal:** `step/05-watcher-live-sessions` · **Rapor:** `docs/perf/2026-10-27-watcher.md` · **Karar:** ADR-027

### Yapılanlar
- **`events.rs`:** `CoreEvent` (`#[non_exhaustive]`, serde `type`/`data` etiketli, specta). Varyantlar: `IndexProgress`, `IndexMilestone` (`RecentReady | Complete`), `UsageAppended`, `LiveSessionsChanged`, `CredentialsChanged`, `SchemaDrift`. `EventBus` (`broadcast`, kapasite 256), `Subscription::{recv, blocking_recv}` ve `Delivery::{Event, Resync}`. Gecikmeli alıcı politikası örnek bir testle gösterildi (`a_lagging_subscriber_resyncs_from_a_full_query`).
- **`watcher/`:** Sabit pencereli `Debouncer` (750 ms), `HotFiles` (15 dk TTL, 30 s'de bir `stat`), `FsWatcher` (`notify` sarmalayıcı; `ensure`, `forget`, `is_watched`).
- **`live_sessions/`:**
  - `parse_session_file` toleranslı: pid ve `procStart` hem sayı hem string olabilir, bilinmeyen alanlar atlanır.
  - `is_live` → `procStart` ile FILETIME eşleşmesi (1 ms tolerans); `procStart` yoksa `startedAt` penceresi.
  - `probe` → `OpenProcess` + `GetExitCodeProcess` + `GetProcessTimes`.
  - `LiveSessionTracker`: yalnızca durum gerçekten değişince "değişti" der; `SessionId` eşlemesi önbelleklenir.
  - `.key` dosyaları açılmaz. `pidDomain` yerel değilse dosya atlanır.
- **`runtime.rs`:**
  - `CoreRuntime::start(&CoreConfig) -> CoreHandle` hesapları upsert eder.
  - Thread'ler: `core-watch` (tüm hesaplar), hesap başına `core-index-<id>`, `retention`.
  - `CoreHandle::{subscribe, store, status, shutdown}`. `CoreStatus`, `Resync` sonrası yeniden yüklenecek tam durumdur.
  - İlk tam tarama 2 s gecikmeli başlar, sonra `Complete` gelir. Sonraki geçişler artımlıdır, kuyruğa biriken işler tek geçişte birleştirilir.
  - `.credentials.json` yalnızca mtime ile izlenir.
- **İndeksleyici:** `set_cancel` (kapanışta dosya sınırında durur, tutarlılık korunur). `IndexReport.{sessions, ts_range, cancelled}`. `index_files` artık yalnızca verilen yolların satırlarını okur (`Store::source_files_at`). `Store::session_id_by_uuid` eklendi.
- **Tauri (`core_bridge.rs`):**
  - Tray kurulduktan sonra çekirdek ayrı bir thread'de açılır; ana thread diske beklemez.
  - `RunEvent::Exit` olayında `shutdown(2 s)` çağrılır.
  - Geçici tooltip: `TokenRadar · N live · indexed HH:MM:SS` (yerel saat `SystemTimeToTzSpecificLocalTime` ile).
  - `TOKENRADAR_DEBUG_EVENTS=1` ile stderr'e zaman damgalı olay günlüğü yazılır (yalnızca sayılar, yol veya kimlik yok).
- **Testler:** 24 yeni test. Gerçek watcher'la uçtan uca testler: satır ekleme → `UsageAppended`, 100 yazma → tek geçiş, canlı oturum aç/değiştir/kapat, credentials mtime, kapanış (thread'ler + bus kapanır + DB exclusive açılabilir), başlangıç gecikmesinin kesilmesi.

### Sapmalar
- **ADR-027 (kullanıcı kararı):**
  - Tokio runtime yerine std thread'ler + `tokio::sync`.
  - `notify` 8.2.1 Windows'ta buffer taşmasını bildirmiyor; planın "rescan olayı" dalı bu platformda hiç tetiklenmez. Telafi olarak sıcak dosya taraması eklendi.
- Debounce, plandaki "birleştir" tanımını **sabit pencere** olarak uygular. Kayan pencere sürekli yazmada hiç kapanmazdı.
- Tooltip metni İngilizce ve kodda sabit (`TODO(step-08)`). Fluent yüklemesi Adım 08'de geliyor; plan Türkçe bir örnek vermişti.
- `UsageAppended` ilk tam taramada yayınlanmaz (binlerce oturum listelenirdi); onun yerine `IndexMilestone::Complete` gelir. `IndexProgress` yalnızca tam taramalarda yayınlanır.
- `CoreEvent` varyantları hesap içerir (`IndexProgress { account, progress }` vb.); `LiveSessionsChanged` tüm hesapların birleşik listesini taşır.
- `deny.toml` izin listesine `CC0-1.0` eklendi (`notify`).
- Plandaki doğrulama komutları birebir çalışmıyor:
  - `cargo test -p tokenradar-core watcher live_sessions runtime`: cargo tek bir filtre kabul eder.
  - `measure-memory.ps1 -Cpu`: böyle bir parametre yok; CPU zaten her zaman ölçülüyor.

### Açık konular
- **Gerçek boşta CPU ölçümü yapılmadı.** Ölçümü yapan oturum kendisi bir Claude Code oturumuydu ve transcript'e sürekli yazıyordu. Bunun yerine sessiz bir fixture dizinine karşı **sentetik** bir ölçüm alındı: %0,013. Claude Code kapalıyken 10 dakikalık bir ölçüm alınmalı (komut `docs/perf/2026-10-27-watcher.md` içinde).
- Aktif ölçüm (%0,038) karma kullanımı temsil ediyor: oturum 10 dakikanın ~7,5'inde boştaydı.
- Uygulamadan çıkışın ≤ 2 s sürdüğü yalnızca çekirdek düzeyinde test edildi (`shutdown_stops_every_thread_in_time`). Tray menüsünden "Quit" ile manuel kontrol yapılmadı.
- Tray tooltip'i ekranda gözle kontrol edilmedi. Olay günlüğüyle aynı veriyi gösteriyor.
- `tracing` için uygulamada henüz bir subscriber yok; çekirdeğin uyarıları görünmüyor. `doctor` ve log dosyası işi ileriki bir adımda.
- macOS/Linux'ta `probe` her zaman `Unknown` döner (Adım 21).

### Sonraki adıma notlar (Adım 06)
- 401 kurtarması için `CoreEvent::CredentialsChanged { account }` hazır. Olay yalnızca mtime değiştiğinde gelir; dosya içeriği okunmaz.
- Yeni uzun ömürlü görevler (`UsageApiPoller` gibi) `runtime.rs`'e adlandırılmış bir thread olarak eklenmeli ve `CoreHandle::stop_all` içinde durdurulmalıdır. Zamanlayıcılarda `recv_timeout` kalıbı kullanılmalı ki durdurma anında olsun.
- Limit snapshot'ları için yeni bir `CoreEvent` varyantı eklemek serbest (`#[non_exhaustive]`).
- `CoreStatus`, Tauri tarafının `Resync`'te yeniden yüklediği tam durumdur; limit durumu da buraya eklenmeli.
