# Adım 05 — Dosya izleme, canlı oturumlar ve olay veri yolu

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1 | 04 | 1 oturum | ⬜ |

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

- [ ] `CoreEvent` tasarımı (serde + specta):
  - `IndexProgress`, `IndexMilestone(RecentReady | Complete)`
  - `UsageAppended { account, sessions: Vec<SessionId>, from_ts, to_ts }`
  - `LiveSessionsChanged(Vec<LiveSession>)`
  - `CredentialsChanged { account }`
  - `SchemaDrift { source, unknown_count }`
  - İleride eklenecekler için `#[non_exhaustive]`.
- [ ] `EventBus`: kapasite 256. Gecikmeli (lagging) alıcılar için politika: alıcı `Lagged` hatası alırsa tam durum tazelemesi ister. Bu politika bir örnek testle gösterilsin.
- [ ] `watcher`:
  - İzlenecekler: `<config>/projects` (özyinelemeli), `<config>/sessions` ve `<config>/.credentials.json`'ın üst dizini (dosya adına göre filtrelenir).
  - Debounce süresi 750 ms. Aynı dosya için gelen olaylar birleştirilir. Kuyruk `HashSet<PathBuf>` olarak tutulur.
  - Yalnızca `.jsonl` uzantılı ve `classify_path` ile kabul edilen dosyalar indeksleyiciye gider.
  - Taşma (`ReadDirectoryChangesW` buffer overflow → `notify` "rescan" olayı) durumunda tam stat taraması yapılır.
  - **Güvenlik ağı:** 10 dakikada bir stat taraması yapılır (yalnızca metadata okunur, maliyeti düşük).
- [ ] `live_sessions`:
  - `sessions/*.json` dosyaları parse edilir (`.key` dosyalarına **dokunulmaz**).
  - Pid canlılığı Windows'ta `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` ve `GetProcessTimes` ile doğrulanır. Oluşturma zamanı (FILETIME) dosyadaki `procStart` ile karşılaştırılır; bu, **pid yeniden kullanımına** karşı korur. `procStart` değeri FILETIME (100 ns, 1601 tabanlı) biçiminde görünüyor; Adım 04'teki gibi önce gerçek veride doğrula.
  - Ölü süreçlere ait dosyalar listeden çıkarılır, ama silinmez (dosya Claude Code'a ait).
  - Çıktı: `LiveSession { account, session_uuid, pid, cwd, name, status, started_at, status_updated_at }` ve DB'deki `SessionId` eşlemesi (varsa).
  - Durum değişimi olayı yalnızca liste veya durum gerçekten değiştiğinde yayınlanır.
- [ ] İndeksleyici entegrasyonu: Watcher → kirli dosyalar → `indexer.index_files(..)` → `UsageAppended` olayı. Başlangıçta `full_scan()` 2 s gecikmeyle çalışır (ADR'deki başlangıç sırası).
- [ ] `CoreRuntime`:
  - Tokio runtime (Tauri'nin runtime'ı kullanılabilir).
  - Görevler: watcher, indexer worker'ları, live-session izleyici, saklama zamanlayıcısı (günlük).
  - `CoreHandle`: `subscribe()`, `query(...)` (okuma API'leri için cephe), `shutdown()`.
- [ ] Tauri tarafı: `setup` içinde `CoreRuntime::start`. `RunEvent::Exit` olayında `shutdown` çağrılır (en fazla 2 s beklenir). Geçici debug: tray tooltip'inde "N canlı oturum · son indeks HH:MM:SS".
- [ ] Testler:
  - Geçici dizinde dosyaya satır ekle → `UsageAppended` ≤ 2 s içinde gelir (CI'da zaman toleransı geniş tutulur).
  - Birleştirme: 100 hızlı yazma → tek bir indeksleme turu.
  - Canlı oturum: sahte bir oturum dosyası + mevcut olmayan bir pid → listeye girmez.
  - Kapanış: `shutdown` sonrası hiçbir thread çalışmaya devam etmez.
- [ ] **Ölçüm:**
  - Bir Claude Code oturumu aktifken 10 dakika boyunca CPU ortalaması.
  - Boşta 10 dakika CPU.
  - Sonuçları `docs/perf/` altına kaydet.

## Teknik notlar ve tuzaklar

- Claude Code streaming sırasında aynı dosyaya saniyede birkaç kez yazar. Debounce olmadan her yazma bir indeksleme turu tetikler ve CPU bütçesi aşılır.
- `notify` Windows'ta özyinelemeli izleme için tek bir `ReadDirectoryChangesW` handle'ı kullanır. 1.500 alt dizin bu yapıda sorun çıkarmaz.
- Tauri uygulaması pencere açmadan çalışırken bile olay döngüsü aktiftir. Çekirdek görevleri Tauri'nin ana thread'ini **asla** bloklamamalı.
- `sessions/` dizinindeki dosya adı pid'dir. Pid formatı ve alanlar sürümler arasında değişebilir; parser toleranslı olmalı.

## Kabul kriterleri

- [ ] Aktif bir Claude Code oturumu yazarken ≤ 2 s içinde `UsageAppended` olayı gelir (manuel test, zaman damgalı log).
- [ ] Boşta CPU 10 dakika ortalaması < %0,1. Aktif oturum varken ortalama < %1.
- [ ] Canlı oturum listesi Claude Code oturumu açılıp kapandıkça doğru güncellenir. Durum değişimi (busy ↔ idle) ≤ 2 s içinde yansır.
- [ ] Uygulamadan çıkışta süreç ≤ 2 s içinde sonlanır.

## Doğrulama

```powershell
cargo test -p tokenradar-core watcher live_sessions runtime
pnpm dev   # Claude Code'da bir oturum aç, tray tooltip'ini izle
.\scripts\measure-memory.ps1 -ProcessName tokenradar-app -DurationSec 600 -Cpu
```

## Devir notu

_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
