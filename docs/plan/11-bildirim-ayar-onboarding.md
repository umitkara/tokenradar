# Adım 11 — Bildirimler, ayarlar ve onboarding

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1 | 10 | 3 oturum | ✅ 11a · 11b · 11c (açık işler Adım 12 ön koşulu) |

> Bu adım büyük. Gerekirse **11a (bildirimler + ayarlar altyapısı)** ve **11b (ayarlar UI + onboarding + doctor)** olarak iki oturuma bölün. Bölünürse devir notu her iki parça için ayrı ayrı yazılır.

## Amaç

v0.1'i kullanıcıya hazır hale getirmek:
- Akıllı ve gürültüsüz bildirimler: eşik, projeksiyon ve reset.
- Eksiksiz bir ayarlar penceresi.
- Statusline köprüsünü ve API onayını açıkça anlatan bir ilk çalıştırma deneyimi.
- Tanılama paketi (`doctor`).

## Önce oku

- `docs/architecture.md` §5 (Notifier), §10, §11 (install/uninstall), §13, §14
- `docs/design-language.md` §7, §8, §9
- `docs/decisions.md`: ADR-005, ADR-006, ADR-009, ADR-013
- Adım 07'nin devir notu (statusline kurulum API'si)

## Kapsam

- **`Notifier`** (core):
  - Kurallar, histerezis ve tekilleştirme (`notification_log`).
  - Susturma ve sessiz saatler.
  - Yerelleştirilmiş metinler (Rust i18n).
- **Bildirim gönderimi:** `tauri-plugin-notification`. Tıklayınca panel açılır.
- **Config:** `config.toml` şeması (`schema_version`), doğrulama, canlı uygulama (`SettingsChanged` olayı), migration.
- **Ayarlar penceresi (`settings.html`):** Sekmeler: Genel · Veri kaynakları · Bildirimler · Gizlilik ve veri · Hakkında.
- **Onboarding penceresi (`onboarding.html`):** İlk çalıştırmada açılır, 4 adımdan oluşur.
- **Autostart:** `tauri-plugin-autostart`.
- **Tray görünürlük yardımcısı:** Windows 11'de ikonu taşma menüsünden çıkarıp görev çubuğuna sabitleme (kullanıcı onayıyla).
- **`tokenradar doctor` ve ayarlardaki "Tanılama paketi oluştur" butonu.**

## Kapsam dışı

- Periyodik özet bildirimi. Analitik gerektirdiği için Adım 17'de yapılacak.
- Çoklu hesap UI'ı (Adım 19). Ancak ayar şeması hesap listesini **şimdiden** desteklemeli.

## Görevler

### Bildirimler

- [x] Kural motoru (`notifier.rs`). Girdi olarak `LimitEngine` olaylarını alır.
  - **Eşik kuralları:** Her limit türü için ayrı bir eşik listesi tutulur (varsayılan `[75, 90, 100]`). Bir eşik, **pencere başına bir kez** tetiklenir. Tekilleştirme anahtarı: `rule:kind:scope:resets_at:threshold`. Yeni pencere başlayınca kurallar yeniden silahlanır.
  - **Projeksiyon kuralı:** `will_hit_before_reset && eta_100 ≤ 60 dk && confidence ≥ orta` koşulunda tetiklenir. Pencere başına en fazla 1 kez. Ancak eta 15 dakikanın altına düşerse ikinci bir "son uyarı" verilebilir (ayarlanabilir).
  - **Reset kuralı:** Yalnızca o pencerede tepe değer ≥ `reset_notify_min` olduysa (varsayılan %80) veya limite takılındıysa tetiklenir. Böylece her 5 saatte bir anlamsız bildirim gelmez.
  - **Haftalık limitler:** Aynı kurallar uygulanır, ama metinler ve eşikler ayrıdır.
  - **Susturma ve sessiz saatler:** Bu sürelerde tetiklenen bildirimler kaydedilir ama gösterilmez. Susturma bitince yalnızca **hâlâ geçerli** olanlar tek bir özet bildirim olarak gösterilir.
- [x] Bildirim metinleri (EN/TR `.ftl`). Örnekler:
  - "5 saatlik pencere %90'a ulaştı — 1 sa 12 dk sonra yenilenir"
  - "Bu hızla 38 dk içinde dolacak, reset 1 sa 52 dk sonra"
  - "5 saatlik pencere yenilendi — tam kapasite"
- [x] Windows toast notları: Bildirimlerin uygulama adı ve ikonuyla görünmesi için AppUserModelID gerekir; installer (Adım 12) Başlat menüsü kısayolunu bu ID ile oluşturur. Geliştirme modunda başlık "PowerShell" veya benzeri olabilir; bunu belgele. Portable mod için de davranışı belgele.
- [x] Testler (sahte saat): Eşik geçişi; pencere başına tek tetikleme; yeni pencerede yeniden silahlanma; projeksiyon histerezisi; susturma sonrası özet; reset filtresi.

### Config ve ayarlar

- [x] `config.toml` şeması:
  - `schema_version`
  - `[general]`: `locale` (system/en/tr), `theme`, `autostart`, `panel_warm_ttl`
  - `[[accounts]]`: provider, config_dir, label, `api_consent { granted, granted_at, consent_version }`, `statusline { installed, previous_command, chain, chain_position }`
  - `[notifications]`: eşikler (her tür için), `projection` (bool), `final_warning_min`, `reset_notify_min`, `quiet_hours { start, end }`
  - `[data]`: `raw_retention_days`, `update_check`
  - `[advanced]`: `log_level`
  - Bilinmeyen alanlar yazımda **korunur** (`toml_edit`).
- [x] Ayarlar penceresi:
  - **Genel:** dil (anlık uygulanır), tema, Windows ile başlat, panelin sıcak tutulma süresi.
  - **Veri kaynakları:**
    - Claude dizini: algılanan yol, oturum ve dosya sayısı, "Değiştir…" butonu.
    - **Statusline köprüsü:** durum (kurulu / değil / son görülme). "Kur…" butonu `settings.json` diff'ini bir diyalogda gösterir ve onay alır. "Zincirleme" seçeneği mevcut komutu korur. Mevcut komut usage API'sini çağırıyorsa (örneğin `statusline.py`) bilgi notu gösterilir. "Kaldır" seçeneği.
    - **API erişimi:** Kapalı/açık anahtarı ve açık dilli bir açıklama: ne okunur, ne okunmaz, nereye gider, refresh yapılmaz, istediğin zaman kapatabilirsin. Açmak için açık onay butonu gerekir; onay zaman damgası ve metin sürümüyle saklanır. Ayrıca polling durumu: son istek, sonraki istek, geri çekilme.
  - **Bildirimler:** eşik çipleri (ekle/sil), projeksiyon, son uyarı, reset bildirimi eşiği, sessiz saatler, "Test bildirimi gönder" butonu.
  - **Gizlilik ve veri:**
    - Veritabanı boyutu, ham olay saklama süresi.
    - "Tüm TokenRadar verisini sil" (çift onay; Claude Code dosyalarına **dokunulmaz**).
    - Ağ özeti: "Bu uygulama yalnızca şu adreslere bağlanır: …"
    - "Tanılama paketi oluştur".
  - **Hakkında:** sürüm, güncelleme kontrolü (updater Adım 12'de bağlanacak, şimdilik yer tutucu), lisanslar (üçüncü parti lisans listesi build sırasında üretilir), GitHub linki.
- [x] Panel ve tray, ayar değişikliklerini anında yansıtır (`SettingsChanged` olayı).

### Onboarding

- [x] İlk çalıştırma tespiti: `kv.onboarding_done` yoksa onboarding açılır. Tray'den tekrar açılabilir.
- [x] Adımlar:
  1. **Hoş geldin:** dil seçimi (algılanan dil önceden seçili) ve tek cümlelik değer önerisi.
  2. **Claude Code bulundu:** dizin, oturum ve proje sayısı. İndeksleme bu adımda arka planda başlar ve ilerleme halkası gösterilir. Claude Code bulunamazsa yol seçme ekranı açılır.
  3. **Limit verisi:**
     - Statusline köprüsü kartı ("önerilen", diff önizlemesi, tek tık kurulum).
     - API kartı (opsiyonel, ayrıntılı açıklama, onay).
     - İkisi de atlanabilir. Atlanırsa sonuçları açıkça yazılır: "limit yüzdeleri görünmez; yalnızca token ve maliyet takibi yapılır".
  4. **Tray ipucu ve başlatma:**
     - Windows 11 taşma menüsünü gösteren küçük bir görsel.
     - "Simgeyi her zaman göster" butonu (aşağıdaki yardımcı).
     - "Windows ile başlat" seçeneği (varsayılan açık).
     - Bitti.
- [x] **Tray sabitleme yardımcısı:** Windows 11'de `HKCU\Control Panel\NotifyIconSettings\<id>\IsPromoted = 1`. Bu yarı-belgelenmiş bir mekanizma; uygulamamıza ait kaydı `ExecutablePath` alanıyla bul. Yalnızca kullanıcı butona basınca yazılır. Başarısız olursa manuel talimat gösterilir. Davranış ADR olarak belgelenir.

### Doctor

- [x] `tokenradar doctor [--out <zip>]` ve ayarlardaki buton. Pakete girenler:
  - Sürümler (uygulama, Claude Code son görülen sürüm, WebView2, işletim sistemi)
  - Yollar, dosya ve olay sayıları, DB boyutu
  - Son 200 log satırı (**redaksiyon testi yapılmış**)
  - Config (sırlar hariç)
  - Polling durumu, şema sapma sayaçları, son performans ölçümleri
  
  Token, içerik ve proje yolları paket dışında kalır; yollar hash'lenir.
- [x] Test: Doctor çıktısında `sk-ant-` deseni ve fixture'lardaki örnek içerik metinleri bulunmamalı.

## Teknik notlar ve tuzaklar

- **Ayar penceresi ve bellek:** Ayarlar penceresinin `warm_ttl` değeri 0'dır; kapanınca hemen yok edilir. Panel ve ayarlar aynı anda açıksa bellek bütçesi "ana pencere" satırına göre değerlendirilir.
- **API onay metni sürümlüdür:** Metin değişirse (örneğin yeni bir veri alanı eklenirse) onay yeniden istenir.
- **Diff diyaloğu:** JSON diff'i satır bazlı ve renkli gösterilir. Kullanıcı neyin değiştiğini **görmeden** hiçbir şey yazılmaz.

## Kabul kriterleri

- [ ] Bildirim kuralları testleri geçer. Gerçek kullanımda bir gün boyunca gürültü değerlendirmesi yapılır: tetiklenen bildirimler listelenir ve gereksiz olan kalmadığı devir notunda belirtilir.
- [ ] Onboarding sıfır kurulumdan (`%LOCALAPPDATA%\TokenRadar` silinmiş durumda) sonuna kadar akar. Statusline kurulumu diff onayıyla çalışır ve panelde canlı veri görünür.
- [x] Ayar değişiklikleri yeniden başlatma gerektirmeden uygulanır (dil, tema, eşikler).
- [x] "Tüm verileri sil" yalnızca TokenRadar verisini siler. Claude Code dizini dokunulmadan kalır.
- [x] Doctor paketinde sır, içerik veya açık yol bulunmaz.

## Doğrulama

```powershell
cargo test -p tokenradar-core notifier config
pnpm -C app test
Remove-Item -Recurse "$env:LOCALAPPDATA\TokenRadar" -WhatIf   # önce WhatIf; yedek alıp gerçekten sil, onboarding'i test et
pnpm dev
.\target\release\tokenradar.exe doctor --out "$env:TEMP\tr-doctor.zip"
```

## Devir notu

### 11a — Bildirimler ve ayar altyapısı

**Tarih:** 2026-10-27 · **Dal:** `step/11a-notifier-config` · **Karar:** ADR-033

#### Yapılanlar
- **Notifier (`crates/tokenradar-core/src/notifier/`):**
  - Duruma dayalı çalışır (ADR-033): her snapshot'ta `evaluate`, reset için `on_reset`. Saf bir yapıdır; ne gösterileceğini ve ne kaydedileceğini döndürür.
  - Kurallar: eşik (5 saatlik ve haftalık için ayrı listeler, pencere başına bir kez, ±5 dk `resets_at` toleransı), projeksiyon (ETA ≤ 60 dk, güven `Low` değil), son uyarı (`final_warning_min`), reset (`reset_notify_min` filtresi). Tahmin kaynaklı limitler bildirim üretmez.
  - Birden çok eşik aynı anda geçilirse yalnızca en yüksek eşik gösterilir, diğerleri sessizce kaydedilir.
  - Susturma ve sessiz saatler (gece yarısını geçebilir): bildirimler bekletilir. İkisi de bitince hâlâ geçerli olanlar güncel sayılarla gösterilir; birden fazlaysa tek özet bildirim olur.
  - `prefs.rs`: `NotifyPrefs` ve `QuietHours`, `normalized()` (eşikler 1–100, sıralı, tekil, en fazla 5; `final_warning_min` ≤ 59; `HH:MM` biçimi).
  - Metinler: `locales/{en,tr}/notify.ftl` (`Domain::Notify`). TR ekleri `TR_SUFFIX` ile yazılır: "%90'a", "Cum 10:59'da".
- **Runtime:**
  - `CoreConfig.notify: Option<NotifySetup { prefs, locale, clock }>`. Notifier `core-limits` thread'inin içinde kurulur: Fluent bundle `Send` değil, bu yüzden `LimitsCtx` de thread içinde oluşturuluyor.
  - Açılışta notifier son 8 günün `notification_log` kayıtlarını ve `kv.notify_snoozed_until`'ı okur. `CoreEvent::Notify(Notice)` yayınlar. `limits_loop`, susturmanın veya sessiz saatlerin bitişinde uyanır.
  - `CoreHandle::set_notify_prefs`, `set_notify_locale`, `set_muted_until`, `set_retention_days` (`RetentionScheduler` artık `AtomicU32` tutuyor).
  - `store/notifications.rs`: `notifications_since` ve `record_notifications` (`ON CONFLICT DO NOTHING`, 30 günden eski kayıtları siler).
- **Config v2 (`config.rs`):**
  - Bölümler: `[notifications]`, `[data]`, `[advanced]`, `[general].panel_warm_ttl`, `[[accounts]].api_consent`. Her bölüm hoşgörülü okunur.
  - Yazıcılar: `save_notifications`, `save_data`, `save_advanced`, `save_panel_warm_ttl`, `save_api_consent(dir, …)`. Hepsi normalize eder ve bilinmeyen anahtarları korur.
  - `migrate(path, primary_dir)`: v1 → v2, eski `[api_consent]` birincil hesaba taşınır.
  - `api_consent_granted(dir)` artık hesaba göre çalışıyor (imza değişti).
- **Saat:** `LocalClock`, `LocalTime` ve `UtcClock` core'daki `clock.rs`'ye taşındı. Tray'in `SystemClock`'u bu trait'i uygular.
- **Uygulama:**
  - `notify.rs`: toast gösterir; tıklama (`tap`) paneli açar.
  - `logging.rs`: `<data>/logs/tokenradar.YYYY-MM-DD.log`, günlük döner, 7 dosya, açılışta 20 MB sınırı, seviye canlı değişir.
  - Autostart: `tauri-plugin-autostart`, değer adı `TokenRadar`.
  - `core_bridge`: açılışta migration çalışır; birincil hesabın onayı, saklama süresi ve `NotifySetup` core'a geçirilir. `with_core` yardımcısı eklendi.
  - `prefs.rs`: `PrefsView` / `PrefsChanged` / `get_prefs` (notifications, data, log_level, panel_warm_ttl_s). `SettingsView` yalnızca görünüm alanlarıyla kaldı.
  - Komutlar: `set_notification_prefs`, `set_data_prefs`, `set_log_level`, `set_panel_warm_ttl`, `get_autostart`, `set_autostart`, `send_test_notification`. `set_locale` notifier'ın dilini de değiştirir.
  - Tray susturma değişikliğini `Sink::mute_changed` ile core'a iletir. Panelin `warm_ttl` değeri ayardan gelir.
- **Testler:** core 237 (notifier 24, config 15, runtime'da 2 yeni: yeniden başlatmada tekrar yok, kayıtlı susturma bekletir ve kaldırınca bırakır), app 46 + 2 ignored (log budama testi), CLI 17, i18n 16 + 6, frontend 53. `svelte-check` 0 hata.

#### Gerçek doğrulama (2026-10-27)
- **Toast:** `pnpm tauri dev` ile, `target\debug\portable` işaretiyle çalıştırıldı (veri `target\debug\data`'da kaldı, gerçek `config.toml`'ye dokunulmadı). `send_test_notification` CDP üzerinden çağrıldı ve kullanıcı toast'u ekranda gördüğünü doğruladı ("Windows PowerShell" adıyla). Toast'a tıklayınca panelin açılması elle denenmedi.
- **Autostart:** CDP ile `get_autostart` → `false`, `set_autostart(true)` → `true` (HKCU `Run`'a `TokenRadar` değeri yazıldı), `set_autostart(false)` → `false` (değer silindi). `auto-launch`'ın `StartupApproved\Run` altına yazdığı ve kapatınca silmediği `TokenRadar` değeri elle kaldırıldı. Kayıt defteri test öncesindeki haline döndü.
- **Migration:** Portable veri dizinindeki eski bir v1 dosyası v2'ye geçirildi (log: "config.toml migrated to schema v2").
- **Log dosyası:** Oluşuyor. İlk sürümde appender dizin yokken eski dosyaları budamaya çalışıp hata yazıyordu; dizin artık önceden oluşturuluyor.
- **Boyut:** `tokenradar-app.exe` release 11.868.672 B (11,32 MiB); Adım 10'da 11.074.048 B idi (+776 KiB: bildirim, autostart, tracing-appender). Bütçe ≤ 15 MB ✅.

#### Sapmalar
- Adım 11a ve 11b olarak bölündü.
- Notifier olaylarla değil, durumla besleniyor. Pencere eşleşmesinde ±5 dk tolerans var. Aynı anda birden çok eşik geçilirse yalnızca en yüksek eşik gösteriliyor. (ADR-033)
- `[general].autostart` alanı yok; gerçeğin kaynağı işletim sistemi (ADR-033).
- Ayarlar iki görünüme ayrıldı: `SettingsView` (görünüm, açılış betiği) ve `PrefsView` (ayarlar penceresi).
- Planın doğrulama komutu `cargo test -p tokenradar-core notifier config` geçersiz (cargo tek filtre alıyor). Bunun yerine `cargo test -p tokenradar-core notifier` ve `cargo test -p tokenradar-core config` ayrı ayrı çalıştırıldı.
- "Test bildirimi" susturmayı ve sessiz saatleri yok sayar (bilinçli: kullanıcı açıkça istedi).

#### 11b için notlar
- **Kapsam:**
  - Ayarlar penceresinin sekmeleri (komutlar ve `PrefsView` hazır)
  - Onboarding
  - Tray sabitleme yardımcısı
  - `tokenradar doctor` ve "Tanılama paketi" butonu (son 200 log satırı `logging.rs`'nin dosyasından gelir)
  - API onayı UI'ı (`config::save_api_consent` + `CoreHandle::set_api_consent`)
  - Statusline kurulum diyaloğu
- **Bir günlük gürültü değerlendirmesi:**
  - Uygulamayı **`target\release` içinden** çalıştırın. Portable kopya veya başka bir konum uygulama kimliğini kullanır; bu kimlik kayıtlı olmadığı için Windows toast'u hata vermeden göstermeyebilir ve test yanlışlıkla "bildirim yok" sonucu verir.
  - Önce "Test bildirimi gönder" ile görünürlüğü doğrulayın.
  - Tetiklenen bildirimler log dosyasında `notification` satırları olarak (`rule=…`) ve `notification_log` tablosunda bulunur. Listeyi oradan çıkarın; toplamları ve yolları commit etmeyin.
- **Toast tıklaması:** Panelin açıldığı elle doğrulanmalı. Bildirim Merkezi'nden tıklama Adım 12'ye kadar çalışmaz.
- **Log kapsamı:** Uygulamada hâlâ çok sayıda `eprintln!` var. Bunlar log dosyasına düşmüyor. Doctor için önemli olanlar `tracing`'e taşınmalı.
- **Bellek:** Yeni eklentiler ve log thread'i boşta tray belleğini (≤ 25 MB bütçe) etkileyebilir. Gerçek boşta ölçümü kullanıcı yapmalı (agent oturumu transcript yazar). 11b'de `measure-memory.ps1` ile ölçün.
- **Çoklu hesap:** Tüm poller'lar birincil hesabın onayını kullanıyor; hesap başına onay Adım 19'da bağlanacak.
- **Eklenti izinleri:** Bildirim ve autostart eklentilerinin JS komutlarına capability verilmedi; frontend yalnızca bizim komutlarımızı kullanır.

### 11b — Ayarlar UI ve doctor

**Tarih:** 2026-10-27 · **Dal:** `step/11b-settings-doctor` · **Karar:** ADR-034

#### Yapılanlar
- **Bölme:** Kullanıcıyla 11b + 11c olarak kararlaştırıldı. 11b: ayarlar penceresi, statusline diyaloğu, API onayı, veri silme, doctor. 11c: onboarding, tray sabitleme yardımcısı, gürültü değerlendirmesi, bellek ölçümü.
- **Core:**
  - `doctor` modülü: stored zip ve CRC32 (`zip.rs`), redaksiyon (`redact.rs`), Windows sürümü ve WebView2 sürümü kayıt defterinden (`sys.rs`). `live_from_status` (CLI) ve `live_from_snapshot` (uygulama).
  - `store/stats.rs`: `AccountStats` (oturum, proje, dosya, olay, son Claude Code sürümü, son etkinlik) ve tablo sayıları.
  - `config`: `claude_config_dir`, `reset_keeping_statusline`, `save_api_consent_now`.
  - `providers::claude::discover_configured`: seçilen dizin birincil olur, diğerleri izlenmeye devam eder.
- **CLI:** `tokenradar doctor [-o <zip>]`. `[profile.release-cli]` (`opt-level = "s"`). `measure-statusline.ps1 -Profile`.
- **Uygulama (Rust):**
  - `sources.rs`: veri kaynakları görünümü, statusline plan/uygula/at (token'lı), API onayı, klasör seçici ve sıfırlama.
  - `privacy.rs`: depolama, hakkında, sabit bağlantılar, tüm verileri silme, tanılama paketi.
  - `core_bridge::{stop, restart}` ve `TrayMsg::CoreStopped`.
  - Hata `eprintln!` çağrıları `tracing`'e taşındı.
  - Eklentiler: dialog ve opener, yalnızca Rust'tan.
  - Pencere boyutu 800×600 (en az 640×480).
- **Uygulama (Svelte):**
  - `pages/settings/`: beş sekme (dikey tablist, ok tuşları), `StatuslineDialog`, `ThresholdChips`, `Field`.
  - Saf mantık `view.ts`'de (sekme hedefi, boyut birimi, eşik kuralları, API durum satırları); durum `state.svelte.ts`'de.
  - Bileşenler: `lib/components/Select.svelte`; `Toggle`'a `labelHidden` eklendi.
  - Metinler: `locales/{en,tr}/settings.ftl`.
- **Testler:** core 251, app 49 + 2 ignored, CLI 17 + 2, i18n 16 + 6, frontend 64. `svelte-check` 0 uyarı. Clippy ve biome temiz.

#### Gerçek doğrulama (2026-10-27, portable `target\debug`, CDP)
- Beş sekme EN ve TR'de çizildi. Dil değişikliği yeniden başlatmadan uygulandı.
- Statusline diyaloğu gerçek `settings.json` için diff gösterdi. Zincir seçeneği açıkken önceki komut korunuyor ve not çıkıyor; kapalıyken yalnızca `command` satırı değişiyor. Git Bash uyarısı ters eğik çizgili komutta göründü. Diyalog **uygulanmadan** kapatıldı; `settings.json`'a yazılmadı.
- "Tüm verileri sil" (portable veride): onay kutusu işaretlenene kadar düğme kapalıydı. Silme sonrası veritabanı yeniden oluştu ve indeksleme baştan başladı. Loglar boşaldı. `config.toml` kaldırıldı (köprü kurulu değildi). Ayarlar varsayılana döndü.
- `tokenradar doctor` (portable kopya): `report.json`'da yollar `path#…`; token, içerik ve açık yol yok.
- Panel paketi değişmedi (JS 45,4 KB / 60 KB gzip). Uygulama 12,04 MiB, CLI 2,17 MiB (`docs/perf/2026-10-27-cli-profile.md`).

#### Doğrulanmadı (kullanıcıya veya sonraki oturuma kalan)
- **Yerel diyaloglar:** CDP ile sürülemiyor; elle denenmeli.
  - Klasör seçici: geçerli, geçersiz ve `.claude` olmayan klasör.
  - Tanılama paketinin kaydetme diyaloğu ve Explorer'da gösterme.
- **Gerçek statusline kurulumu ve kaldırma:** Ayarlardan, gerçek `settings.json` üzerinde yapılmadı. Kullanıcı onayı gerekir. Adım 07'deki CLI kurulumu aynı `Plan::apply`'ı kullanıyor.
- **API onayını açma:** Gerçek ağ isteği başlatacağı için denenmedi.

#### Plandan sapmalar
- Seçilen Claude dizini diğerlerinin yerine geçmiyor; birincil oluyor (ADR-034).
- Doctor'a performans ölçümleri girmedi; uygulama bunları çalışma anında tutmuyor (ADR-034).
- Usage API bilgi notu, kullanıcının betiği okunmadan genel bir not olarak gösteriliyor (ADR-034).
- Üçüncü parti lisans listesi Adım 12'ye kaldı. Güncelleme denetimi anahtarı saklanıyor, ama denetim Adım 12'de bağlanacak.

#### 11c için notlar
- **Onboarding:**
  - "Tüm verileri sil" sonrası onboarding yeniden açılmalı. Veritabanı yeni olduğu için `kv.onboarding_done` zaten yok.
  - Statusline ve API kartları `plan_statusline` / `apply_statusline` / `set_api_consent` komutlarını ve `StatuslineDialog`'u yeniden kullanabilir.
  - Klasör seçimi `pick_claude_dir` ile yapılabilir.
- **Tray sabitleme yardımcısı:** ADR gerekiyor (planda yazıyor).
- **Gürültü değerlendirmesi ve toast tıklaması:** 11a notları geçerli (`target\release` içinden çalıştırın).
- **Bellek:** Ayarlar penceresi ve iki yeni eklenti ile boşta tray belleği ölçülmeli (`measure-memory.ps1`). Gerçek boşta ölçümü kullanıcı yapar.
- **Adım 12:** `tauri.conf.json`'daki `beforeBuildCommand` CLI'yı `--profile release-cli` ile derlemeli; lisans listesi build'de üretilmeli.

### 11c — Onboarding ve tray sabitleme

**Tarih:** 2026-10-27 · **Dal:** `step/11c-onboarding` · **Karar:** ADR-035

#### Yapılanlar
- **Onboarding (Rust):** `onboarding.rs`.
  - `kv.onboarding_done` yoksa çekirdek her başladığında pencere açılır, "Tüm verileri sil" sonrası dahil.
  - Bayrak pencere yok edilince yazılır; X ile kapatmak da tamamlandı sayılır.
  - Tray menüsüne "Başlangıç rehberi…" eklendi.
  - `TOKENRADAR_SKIP_ONBOARDING=1` açılışı atlar.
- **Tray sabitleme (Rust):** `tray_pin.rs`.
  - `NotifyIconSettings` alt anahtarları `ExecutablePath` ile eşlenir; known-folder GUID'leri açılır.
  - Eşleşen tüm anahtarlara `IsPromoted = 1` yazılır ve değer geri okunur.
  - Durumlar: `shown`, `hidden`, `unknown`, `unsupported`.
  - Yedek yol: `ms-settings:taskbar`.
- **Komutlar:** `get_tray_pin`, `pin_tray_icon`, `open_taskbar_settings`, `finish_onboarding`.
- **Svelte:** `pages/onboarding/`, 4 adım.
  - Hoş geldin: dil seçimi.
  - Claude: dizin, sayılar, canlı ilerleme halkası, klasör seçici.
  - Limitler: `StatuslineDialog` ve ortak `ApiConsent`. Kaynak yoksa "İleri" düğmesi "Atla" olur.
  - Tray: SVG şema, sabitleme düğmesi ve "Windows ile başlat" (varsayılan açık).
  - Saf mantık `view.ts`'de.
- **Ortak parçalar:**
  - `ApiConsent.svelte` `SourcesTab`'dan çıkarıldı.
  - `lib/progress.ts` (`indexPercent`) panel ile paylaşılıyor.
  - Sayfa başına `catalog.ts`; tek `catalogs.ts` silindi. Panel JS 45,4 → 38,5 KB gzip.
- **Metinler:** `locales/{en,tr}/onboarding.ftl`, `tray-menu-onboarding`.
- **Testler:** core 251, app 52 + 2 ignored, CLI 17 + 2, i18n 16 + 6, frontend 76. `svelte-check` 0/0. Clippy, fmt ve biome temiz.
- **Ölçüm:** `docs/perf/2026-10-27-onboarding.md` (sentetik).
  - Boşta tray belleği 7,3 MB (≤ 25 MB).
  - Onboarding açıkken bellek 80 MB (≤ 130 MB).
  - Uygulama ikilisi 12,08 MiB (≤ 15 MB).

#### Gerçek doğrulama (2026-10-27, portable `target\debug`, CDP)
- Boş veriyle onboarding kendiliğinden açıldı. Dil EN → TR canlı değişti.
- 2. adım gerçek dizini ve sayıları gösterdi. İlerleme halkası canlı ilerledi ve "Mevcut pencere ve bu hafta hazır" satırı çıktı.
- 3. adım:
  - Statusline diyaloğu diff ve zincir seçeneğiyle açıldı, "Vazgeç" ile kapatıldı.
  - `~/.claude/settings.json` değişmedi; mtime Adım 07'den.
  - Kaynak olmadan "Atla" ve sonuç metni göründü.
- 4. adım:
  - Durum `hidden` iken düğme `IsPromoted=1` yazdı. Durum `shown` oldu ve simge görev çubuğuna canlı geldi.
  - Değer silinince simge canlı olarak taşmaya döndü. Kayıt eski haline getirildi.
- "Bitir" (autostart kapalı) pencereyi kapattı. `onboarding_done` yazıldı; HKCU `Run` kaydı oluşmadı.
- Yeniden başlatmada onboarding açılmadı. `delete_all_data` sonrası yeniden açıldı.
- Koyu tema ve EN'de 4. adım kontrol edildi.

#### Açık işler (Adım 12 ön koşulu, kullanıcıyla kararlaştırıldı)
- **Bir günlük gürültü değerlendirmesi** (kabul kriteri): uygulamayı `target\release` içinden bir gün çalıştırın. Tetiklenen bildirimleri listeleyin ve gereksiz olanları not edin. Toast tıklaması da bu sırada denenmeli.
- **Gerçek boşta bellek ölçümü:** Claude Code kapalıyken kullanıcı yapar. Komut `docs/perf/2026-10-27-onboarding.md`'de.
- **Gerçek statusline kurulumu ve panelde canlı veri** (kabul kriteri): onboarding'den, diff onayıyla yapılmalı. Agent oturumunda kullanıcının `settings.json`'ına yazılmadı.
- **Elle denenecekler:**
  - Tray menüsünden "Başlangıç rehberi…" (menü tıklaması CDP ile sürülemiyor; aynı `onboarding::show` yolu `delete_all_data` sonrası doğrulandı).
  - Klasör seçici ve onboarding'i X ile kapatma.

#### Plandan sapmalar
- İndeksleme 2. adımda değil, uygulama açılınca başlıyor; 2. adım yalnızca ilerlemeyi gösterir (ADR-035).
- Pencereyi X ile kapatmak tamamlandı sayılır (ADR-035).
- Taşma menüsü görseli çizilmiş bir SVG şema (ADR-035).
- Kabul kriterindeki "`%LOCALAPPDATA%\TokenRadar` silinmiş durumda" portable veriyle denendi; gerçek veri klasörü silinmedi.

#### Adım 12 için notlar
- **Kurulum ve veri klasörü çakışabilir:**
  - NSIS per-user kurulum büyük olasılıkla `%LOCALAPPDATA%\TokenRadar`'a kurar. Bu, veri klasörüyle aynıdır.
  - O durumda bu adımın doğrulama komutu (`Remove-Item -Recurse "$env:LOCALAPPDATA\TokenRadar"`) uygulamayı da siler. Kaldırıcı da veriyi götürebilir.
  - Kurulum klasörü (ör. `%LOCALAPPDATA%\Programs\TokenRadar`) ile veri klasörünü ayırmak baştan karara bağlanmalı. Tauri'nin varsayılanı sürüme göre doğrulanmalı.
- **Tray sabitleme:** Kurulu exe'nin yolu değişince Explorer yeni bir `NotifyIconSettings` anahtarı açar; güncelleme sonrası simge yeniden taşmaya düşebilir. Updater (Adım 12) sonrası davranış denenmeli.
- **Toast:** AppUserModelID ve Başlat menüsü kısayolu (11a notu) hâlâ geçerli.
- **Build:** `beforeBuildCommand` + `release-cli` ve lisans listesi (11b notu) hâlâ geçerli.
