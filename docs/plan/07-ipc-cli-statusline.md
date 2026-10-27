# Adım 07 — IPC, CLI ve statusline köprüsü

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1 | 06 | 1-2 oturum | ✅ |

## Amaç

Birincil limit kaynağı olan **statusline köprüsünü (S1)** devreye almak. Bu adım üç parçadan oluşuyor:

1. Uygulama içinde güvenli bir IPC sunucusu.
2. Hızlı bir konsol CLI'ı (`tokenradar.exe`).
3. Kullanıcının mevcut statusline'ını bozmadan kurulum ve kaldırma.

Uygulama kapalıyken bile veri kaybolmaz: CLI veriyi spool dosyasına yazar.

## Önce oku

- `docs/data-sources.md` S1
- `docs/architecture.md` §11, §13
- `docs/decisions.md`: ADR-005, ADR-009, ADR-017
- `docs/performance-budget.md` (statusline gecikmesi, CLI boyutu)

## Kapsam

- `core::ipc`: protokol tipleri (sürümlü), framing, istemci yardımcıları.
- Uygulama tarafı: IPC sunucusu (`interprocess`, tokio) ve kullanıcıya özel DACL.
- `tokenradar-cli` komutları: `statusline`, `status [--json]`, `limits [--json]`, `sessions [--json]`, `statusline install`, `statusline uninstall`.
- Spool: yazma (CLI) ve içe aktarma (uygulama başlangıcında ve periyodik).
- Statusline çıktı biçimlendirici (EN/TR, ANSI, `COLUMNS` ile genişlik uyumu).

## Kapsam dışı

- Ayarlar UI'ındaki kurulum diyaloğu (Adım 11). Burada yalnızca CLI ve çekirdek API'si yazılır, UI bu API'yi çağıracak.
- `doctor` komutu (Adım 11).

## Görevler

### IPC

- [x] Protokol (`core::ipc::proto`):
  - Satır sonuyla ayrılmış JSON.
  - `Request { v: u8, id: u32, op: Op }` ve `Response { v, id, ok, data | error }`.
  - `Op` değerleri: `Hello { client, version }`, `StatuslinePush(StatuslinePayload)` (yanıt beklenmez), `StatusGet`, `LimitsGet`, `SessionsLive`.
  - **İleri uyumluluk:** Bilinmeyen `op` → `error: "unsupported"`.
- [x] Pipe adı: `\\.\pipe\tokenradar-<sha256(kullanıcı SID)[..12]>`. SID, `GetTokenInformation(TokenUser)` ile alınır.
- [x] **Güvenlik:**
  - Sunucu pipe'ı açık bir security descriptor ile oluşturulur: SDDL `D:P(A;;GA;;;<UserSID>)`; gerekiyorsa SYSTEM eklenir.
  - `PIPE_REJECT_REMOTE_CLIENTS` bayrağı set edilir.
  - `interprocess` kütüphanesinin bu ayarları destekleyip desteklemediği doğrulanmalı. Desteklemiyorsa `windows-sys` ile pipe oluşturulup tokio'ya sarılır (gerekçeli `unsafe`).
  - Erişim izinleri bir testle doğrulanmalı (en azından SDDL'nin okunup karşılaştırılması).
- [x] Sunucu: eşzamanlı istemciler, istemci başına 2 s okuma zaman aşımı, en fazla 1 MB mesaj boyutu. Hatalı mesaj bağlantıyı kapatır ama sunucuyu etkilemez.

### Statusline akışı

- [x] `StatuslinePayload`: S1 JSON'unun ihtiyaç duyulan alt kümesi:
  - `session_id`, `session_name`, `transcript_path`, `cwd`
  - `model.id`, `cost.total_cost_usd`, `context_window.used_percentage`
  - `rate_limits.{five_hour,seven_day}.{used_percentage,resets_at}`, `version`
  
  Ham JSON **saklanmaz**, yalnızca bu alanlar alınır.
- [x] Uygulama tarafı: `StatuslinePush` → `LimitObservation` (`source: Statusline`, `session_uuid`) → `LimitEngine::ingest`. Hesap eşlemesi `transcript_path` önekinin hesabın `config_dir` değeriyle eşleşmesine göre yapılır (v0.3'te çoklu hesap için kritik; şimdiden doğru yazılsın).
- [x] `tokenradar statusline` (CLI):
  1. stdin'i oku (en fazla 1 MB, 200 ms zaman aşımı).
  2. Pipe'a 50 ms zaman aşımıyla bağlan, `Hello` gönder (sürüm), ardından `StatuslinePush` gönder ve `StatusGet` ile zenginleştirme verisini al (oturumun ≈ pay değeri, projeksiyon).
  3. Satırı biçimlendir. Varsayılan şablon: `◔ 5s %72 · 1sa48dk ▲38dk │ 7g %34 │ ≈%9 bu oturum`. Dil config'den okunur, renkler eşiklere göre seçilir, `COLUMNS` dar ise kısaltılmış biçim kullanılır.
  4. Daemon yoksa: `spool/statusline-YYYYMMDD.jsonl` dosyasına tek satır ekle (dosya kilidi, günlük en fazla 5 MB, 14 gün saklama) ve satırı yalnızca stdin verisiyle biçimlendir.
  5. `--chain`: config'deki `previous_command`, **aynı stdin ile** çalıştırılır (300 ms zaman aşımı). Çıktısı bizim satırımızın önüne veya arkasına eklenir (config: `chain_position`).
- [x] Spool içe aktarma: Uygulama başlangıcında ve 5 dakikada bir `spool/*.jsonl` dosyaları okunur. Gözlemler zaman damgalarıyla `ingest` edilir, ardından dosyalar silinir (içe aktarma idempotent: aynı `observed_at` + session tekrarlanmaz).
- [x] `statusline install`:
  1. `settings.json` dosyasının yolunu bul (`CLAUDE_CONFIG_DIR` veya `~/.claude`).
  2. Mevcut `statusLine` değerini oku.
  3. Varsa `previous_command` olarak config'e kaydet.
  4. Yeni değeri hazırla: `{"type":"command","command":"<CLI tam yolu> statusline[ --chain]"}`. `refreshInterval` eklenmez; olay güdümlü güncellemeler yeterli.
  5. **Diff göster** (CLI: unified diff, `--yes` olmadan onay ister).
  6. Yedek al: `settings.json.tokenradar-<ts>.bak`.
  7. Atomik yaz: geçici dosya + `ReplaceFileW`/rename. Anahtar sırası korunur (`serde_json` `preserve_order`), girinti 2 boşluk.
- [x] `statusline uninstall`: `previous_command` geri yazılır, kayıt yoksa `statusLine` anahtarı silinir. Yine diff gösterilir ve yedek alınır.
- [x] **Windows'ta komut çalıştırma doğrulaması:** Claude Code'un statusline komutunu Windows'ta hangi kabukla çalıştırdığı (cmd, PowerShell, Git Bash) belirlenmeli. Mevcut kurulumdaki örnek `python C:\Users\…\.claude\statusline.py` ters bölülerle yazılmış ve çalışıyor. **Boşluk içeren yollar** (`C:\Users\Ad Soyad\…`) için doğru tırnaklama gerçek bir Claude Code oturumunda test edilmeli. Bulgu `data-sources.md` S1'e eklenmeli.

### Diğer CLI komutları

- [x] `status`, `limits`, `sessions`: Daemon varsa IPC üzerinden, yoksa SQLite'tan **salt-okunur** (`SQLITE_OPEN_READ_ONLY`, WAL okuyucusu) veri alınır. `--json` makine çıktısı (kararlı şema, `"schema": 1`) verir. Varsayılan çıktı insan okuması için hizalı bir tablodur.
- [x] CLI i18n: `tokenradar-i18n` crate'i, `.ftl` dosyalarını `include_str!` ile gömer. Dil config'den, yoksa sistem dilinden seçilir.
- [x] CLI boyutu ≤ 3 MB. Bunun için minimal bağımlılık kullanılır: `tokio` kullanılmaz, IPC istemcisi senkron çalışır. HTTP ve Tauri bağımlılığı yoktur.

### Ölçüm ve test

- [x] `scripts/measure-statusline.ps1`: 200 çağrı (örnek stdin ile) → p50/p95 değerleri. Ölçüm, daemon açıkken ve kapalıyken ayrı yapılır.
- [x] Testler:
  - Protokol serde gidiş-dönüşü (round-trip)
  - Sürüm uyuşmazlığı
  - Hatalı mesaj
  - Spool yazma ve içe aktarma idempotensi
  - settings.json düzenleme: anahtar sırasının korunması, önceki komutun saklanması, uninstall ile geri dönüş
  - Biçimlendirici için EN/TR snapshot'ları ve dar genişlik
  - Zincirleme modunda zaman aşımı

## Teknik notlar ve tuzaklar

- Claude Code yeni bir güncelleme geldiğinde **çalışmakta olan statusline script'ini iptal eder**. CLI ne kadar hızlı olursa veri kaybı o kadar az olur. Bu yüzden push işlemi **ilk iş** olarak yapılır, biçimlendirme sonra gelir.
- GUI alt sistemli bir exe statusline için uygun değildir (konsol yok). Statusline için yalnızca `tokenradar.exe` (console) kullanılır.
- Windows'un varsayılan named pipe DACL'i *Everyone* ve *Anonymous* gruplarına okuma izni verir. Açık bir security descriptor şarttır.
- `settings.json` dosyasını Claude Code çalışırken değiştirmek güvenlidir; Claude Code ayarları yeniden yükler. Yine de atomik yazma zorunludur.
- Kullanıcının `statusline.py` dosyası usage API'sini 15 dakikalık cache ile çağırıyor. Zincirleme modda bu çağrılar devam eder. Kurulum diyaloğu (Adım 11) bu durumu fark edip kullanıcıya *"TokenRadar bu bilgiyi zaten sağlıyor, eski script'i zincirden çıkarabilirsiniz"* önerisinde bulunabilir.

## Kabul kriterleri

- [x] Gerçek bir Claude Code oturumunda statusline TokenRadar satırını gösterir ve tray/uygulama yüzdeleri her yanıttan sonra güncellenir.
- [x] `statusline` p95 ≤ 30 ms (daemon açık) ve ≤ 40 ms (daemon kapalı).
- [x] Uygulama kapalıyken 10 yanıt üretilip uygulama açıldığında, spool'daki 10 gözlem snapshot tablosuna aktarılır.
- [x] `install` → `uninstall` döngüsü sonrası `settings.json` dosyası, anahtar sırası dahil orijinaliyle **bayt düzeyinde aynıdır** (girinti farkı olursa belgelenir).
- [x] Pipe yalnızca mevcut kullanıcı tarafından açılabilir.

## Doğrulama

```powershell
cargo test -p tokenradar-core ipc
cargo test -p tokenradar-cli
cargo build --release -p tokenradar-cli; (Get-Item .\target\release\tokenradar.exe).Length
Get-Content testdata\statusline\sample.json | .\target\release\tokenradar.exe statusline
.\scripts\measure-statusline.ps1
.\target\release\tokenradar.exe statusline install   # diff'i incele, onayla; sonra uninstall
```

## Devir notu

**Tarih:** 2026-10-27 · **Dal:** `step/07-ipc-cli-statusline` · **Karar:** ADR-029 · **Ölçüm:** `docs/perf/2026-10-27-statusline.md`

### Yapılanlar
- **`tokenradar-i18n`:**
  - `Locale { En, Tr }` (`detect()` sistem dilini okur), `Domain::Cli`, `Localizer::{new, fmt, msg}`.
  - `NUM`/`PCT`/`DUR` Fluent fonksiyonları (`format.rs`): TR `1.234,5`, `%23,5`, `1 sa 48 dk` / `1sa48dk`; EN `1,234.5`, `23.5%`, `1h 48m` / `1h48m`.
  - `locales/{en,tr}/cli.ftl` dosyaları eklendi. İki dilin anahtar ve değişken eşliği testle denetleniyor.
- **`core::ipc`:**
  - `proto.rs`: satır JSON'u, `Request`/`Response`/`Op`, `Handler` trait'i, `handle()`.
  - `view.rs`: CLI'ın gördüğü kararlı DTO'lar (`StatusReply`, `LimitsReply`, `SessionsReply`, `"schema": 1`); `from_snapshot` ve `from_stored` dönüşümleri.
  - `security.rs`: SID, kullanıcıya özel SDDL, `dacl_sddl` (test için).
  - `server.rs`: tokio named pipe, `core-ipc` thread'i, `stop_channel()`.
  - `client.rs`: senkron istemci, `with_deadline`.
  - `pipe_name()` = `\\.\pipe\tokenradar-<sha256(SID)[..12 hex]>`.
- **`core::statusline`:**
  - `payload.rs`: seçici `StatuslinePayload` ve gözleme dönüşüm. `account_for` hesabı transcript yolunun en uzun önekiyle bulur.
  - `spool.rs`: UTC gün dosyaları, kilitli ekleme, 5 MiB / 14 gün, `claim` + `finish`.
  - `settings.rs`: cerrahi JSON üye düzenleyici ve unified diff.
  - `shell.rs`: Git Bash / PowerShell algılama, `mangles`, `exe_for_command` (8.3 kısa yol).
  - `install.rs`: `plan_install` / `plan_uninstall` → `Plan::{diff, apply}`, yedek, "arada değişti" kontrolü.
- **`config.rs`:**
  - Yeni alanlar: `[general] locale` ve `[[accounts]] … [accounts.statusline]`.
  - Yazma `toml_edit` ile yapılır; bilinmeyen anahtarlar ve yorumlar korunur.
  - `fsutil::atomic_write` eklendi.
- **Store:**
  - `Store::snapshot_exists` eklendi (spool idempotensi için).
  - `ReadOnlyStore` eklendi: `SQLITE_OPEN_READ_ONLY`; şema yoksa `None`, şema daha yeniyse hata.
  - `catalog::accounts` eklendi.
- **Runtime:**
  - `CoreConfig.{ipc, spool_dir}` alanları eklendi.
  - `runtime/bridge.rs`: `CoreIpc` handler'ı push'ları `LimitsMsg::Observe` olarak iletir; `import_spool` eklendi.
  - `core-limits` spool'u açılışta ve 5 dakikada bir içe aktarır.
  - `live_sessions::running_sessions` eklendi: store'suz tarama.
- **CLI (`tokenradar`):**
  - `statusline [--chain]`, `status|limits|sessions [--json]`, `statusline install [--chain|--no-chain] [--yes] [--config-dir]`, `statusline uninstall [--yes] [--config-dir]`.
  - Satır biçimi:
    - Çeyrek dilimli bir pasta glifi.
    - Eşik renkleri 75/90 (camgöbeği/sarı/kırmızı; `NO_COLOR` bunları kapatır).
    - `COLUMNS` dar geldiğinde sırayla pay, `▲`, reset ve 7g bölümleri düşer.
- **Uygulama:** `core_bridge.rs` pipe'ı açar (`tokenradar-app/<sürüm>`) ve spool dizinini verir.
- **Ölçüm:** `scripts/measure-statusline.ps1` ve `examples/ipc_serve.rs` eklendi.
- **Testler:**
  - core 201 test (ipc 19).
  - CLI 17 test: biçimlendiricinin EN/TR snapshot'ları ve dar genişlik; zincirde önce/sonra konumu, aynı stdin, zaman aşımı; spool'a karşı IPC yolu; raporlar.
  - i18n 9 test.

### Gerçek doğrulama (2026-10-27)
- Kurulum **kuru çalıştırma** ile başladı: terminal olmadan ve `--yes` olmadan yalnızca diff basıldı, dosya hash'i değişmedi. Ardından kullanıcı onayıyla `--no-chain` kuruldu.
- Claude Code 2.1.311 komutu her güncellemede çalıştırdı. Uygulama kapalıyken 8 okuma spool'a yazıldı.
- Kullanıcı uygulamayı açtı. Spool içe aktarıldı ve dizin boşaldı. Sonraki okumalar IPC'den geldi (`statusline_last_seen` 1 s önce). Limitlerin kaynağı `statusline` oldu, oturum payı geldi.
- `uninstall`: `settings.json`'un SHA-256 değeri kurulum öncesiyle **aynı** (`ace1f346…`). `config.toml`'daki `previous` kaydı temizlendi.
- `sample.json` uygulama açıkken IPC'ye gitti. Transcript yolu hiçbir hesapla eşleşmediği için okuma yok sayıldı.

### Sapmalar
- **ADR-029 (kullanıcı kararları):**
  - `interprocess` yerine `tokio` named pipe kullanıldı.
  - `preserve_order` ile yeniden yazmak yerine cerrahi bayt düzenlemesi yapıldı.
  - Fluent çekirdeği Adım 08'den bu adıma çekildi.
- **Satırın kaynağı:** Yüzde ve reset değerleri payload'dan, projeksiyon ve oturum payı uygulamadan gelir. Plan "satır stdin'den, zenginleştirme daemon'dan" diyordu; aynı fikir, ama hangi alanın kimden geldiği açıkça sabitlendi.
- **Varsayılan zincir:** Önceki bir komut varsa `install` zincirler. Kapatmak için `--no-chain` kullanılır.
- **Windows kabuğu:** Plan "ters bölülü örnek çalışıyor" diyordu. Git Bash ters bölüleri sildiği için bu büyük olasılıkla yanlıştı (`data-sources.md` S1). Kurulum, zincirlenecek bozuk komut için uyarı verir.
- **Doğrulama komutu:** `Get-Content testdata\statusline\sample.json | tokenradar.exe statusline` uygulama kapalıyken **gerçek spool'a yazar**. Bu oturumda bir kez yazdı ve elle temizlendi. Bundan sonra `portable` işaretli bir kopyada çalıştırılmalı.
- `cargo test -p tokenradar-core ipc` filtresi IPC testlerini çalıştırır. Runtime köprü testleri `runtime` filtresindedir.

### Açık konular
- **CLI boyutu:** 2,95 MiB, bütçeye yalnızca ~48 KiB pay kalıyor. `opt-level = "s"` 2,05 MiB veriyor. Seçenekler:
  - CLI için ayrı bir profil (`[profile.release-cli]`).
  - Çekirdekte özellik bayrakları (CLI `runtime`/`http`/`watcher` kodunu derlemesin).
  
  Hangisi seçilirse, Adım 13'ten önce kullanıcıyla karar verilmeli.
- **Boşluklu yollar:** Kısa yol ve tırnaklama birim testlerinde var, gerçek Claude Code'da denenmedi.
- **Zincir modu:** Gerçek ortamda denenmedi (kullanıcı `--no-chain` seçti). Zincirin p95'e etkisi ölçülmedi.
- **Kullanıcının `statusline.py`'si:** Git Bash altında büyük olasılıkla hiçbir şey basmıyor. Kullanıcı isterse yolu düz bölülerle düzeltebilir; TokenRadar komutu kendiliğinden değiştirmez.
- **Yedekler:** `~/.claude` altında iki `.bak` yedeği kaldı (kurulum ve kaldırma). Silinmedi. Adım 11'deki kurulum diyaloğu yedek temizliğini düşünebilir.
- **IPC yalnızca Windows'ta.** Diğer platformlarda CLI her zaman spool'a yazar (Adım 21).
- **Pipe çakışması:** Pipe başka bir süreç tarafından tutuluyorsa uygulama köprüsüz çalışır; bu yalnızca `tracing` uyarısında görünür. Henüz log dosyası olmadığı için bu uyarı görünmez (Adım 06'dan kalan açık konu). `doctor` (Adım 11) pipe durumunu göstermeli.
- **Spool zamanlaması:** Uygulama kapalıyken biriken okumalar en geç 5 dakikada bir içe aktarılır. Uygulama açıkken de spool'a düşen bir okuma (pipe meşgul) en fazla 5 dakika gecikebilir.

### Sonraki adıma notlar (Adım 08)
- `Localizer`, `NUM`/`PCT`/`DUR` ve `Locale::detect()` hazır. Tray metinleri için `Domain`'e yeni bir değer ekleyin ve `locales/{en,tr}/<domain>.ftl` dosyalarını oluşturun. Eşlik testi `Domain::ALL` üzerinden otomatik çalışır.
- TR sonek kuralları ve `i18n-vectors.json` hâlâ yapılacak.
- Tray yüzdeleri `CoreEvent::LimitsUpdated` ile gelir. Statusline okumaları artık bu akışa giriyor (`source: statusline`).
- Eşikler `limits::{WARN_AT, CRIT_AT}` (75/90). CLI aynı `Severity::of`'u kullanıyor; tray de onu kullanmalı.
