# Adım 06 — Limit motoru ve OAuth usage API istemcisi

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1 | 03, 05 | 1-2 oturum | ⬜ |

## Amaç

Uygulamanın kalbini kurmak: limit gözlemlerini birleştiren (füzyon), snapshot ve pencere kayıtlarını tutan, reset'leri algılayan, burn rate ve projeksiyon hesaplayan `LimitEngine`. Buna ek olarak, **açık onayla** çalışan ve token'ı salt-okunur kullanan OAuth usage API istemcisi (S2).

Bu adımın sonunda **% geçmişi kaydı başlar**. Bu kayıt, v0.2'deki kalibrasyonun ham maddesidir.

## Önce oku

- `docs/data-sources.md` S1, S2, S5 ve "Kaynak füzyonu"
- `docs/architecture.md` §7, §8, §9.3 (v0.1 atıfı), §10, §13
- `docs/decisions.md`: ADR-005, ADR-006, ADR-008, ADR-013
- `testdata/api/claude-usage-2026-10-26-limits-array.json`

## Kapsam

- `limits/engine.rs`: füzyon, güncel durum, bayatlık, reset algılama, `limit_snapshots` ve `limit_windows` yazımı.
- `limits/rate.rs`: burn rate (Theil–Sen) ve projeksiyon.
- `limits/share.rs`: v0.1 orantılı atıf. Mevcut pencere içinde oturum başına `≈ %` ve maliyet payı.
- `providers/claude/usage_api.rs`: toleranslı parser (önce `limits[]`, yoksa düz alanlar), istemci, zamanlayıcı ve geri çekilme mantığı.
- `providers/claude/credentials.rs`:
  - Plan meta verisi (`subscriptionType`, `rateLimitTier`) onay gerektirmeden okunur.
  - `accessToken` yalnızca onay varsa ve istek anında okunur.
  - `refreshToken` **hiçbir koşulda** okunmaz.
- Onay durumu: config içinde `api_consent: { granted: bool, granted_at, version }`. UI Adım 11'de yapılacak; bu adımda config dosyası veya geliştirici komutuyla açılır.
- `PanelSnapshot` görünüm modeli: Adım 10'daki panelin ihtiyaç duyduğu her şey (bkz. aşağıdaki görev).
- `CoreEvent::LimitsUpdated(PanelSnapshot)`.

## Kapsam dışı

- Statusline köprüsü ve IPC (Adım 07). Bu adımda `LimitEngine::ingest(LimitObservation)` API'si hazırlanır, S1 bu API'ye Adım 07'de bağlanır.
- Kalibre edilmiş k ve aralık bazlı atıf (Adım 13-14).
- Bildirim kuralları (Adım 11). Ancak `LimitEngine` gerekli sinyalleri (eşik geçişi, projeksiyon, reset) **olay olarak** yayınlar.

## Görevler

- [x] **Parser** (`usage_api.rs`):
  - `limits[]` → `LimitObservation` listesi. `kind` değerleri `LimitKind` enum'una eşlenir; bilinmeyen `kind` için `Other(String)`. `scope.model.display_name` → `Scope::Model`.
  - `limits[]` yoksa veya boşsa: `five_hour` → `Session`, `seven_day` → `WeeklyAll`, `seven_day_opus/sonnet/...` (null değilse) → `WeeklyScoped(model)`.
  - `seven_day_breakdown` → `surface_breakdown` tablosu.
  - `extra_usage` ve `spend` → özet struct. UI'da yalnızca "extra usage etkin mi?" bilgisi gösterilecek.
  - Kod adlı ve bilinmeyen alanlar sessizce yok sayılır. **Fixture testleri:** gerçek yanıt, sentetik düz şema, boş `limits`, bilinmeyen `kind`, eksik `resets_at`.
- [x] **İstemci:**
  - `reqwest` (rustls), zaman aşımı 10 s, `User-Agent: TokenRadar/<sürüm>`.
  - Başlıklar `data-sources.md` S2'deki gibi.
  - Token `Secret<String>` içinde tutulur ve istek biter bitmez drop edilir.
- [x] **Zamanlayıcı:**
  - `architecture.md` §10'daki tabloya göre çalışır. Girdiler: S1 tazeliği, canlı oturum durumları, son başarılı istek, `resets_at` zamanları.
  - **Sahte saat** (`Clock` trait'i) ile test edilebilir olmalı.
  - Sert sınır: 60 s'de en fazla 1 istek. Jitter ±%10.
- [x] **Hata durumları:**
  - 401 → `TokenStale`; bir sonraki `CredentialsChanged` olayına (Adım 05) kadar istek atılmaz.
  - 429/5xx → üstel geri çekilme (60 s → 30 dk), `Retry-After` başlığına uyulur.
  - Ağ hatası → çevrimdışı modu.
  - Hiçbir hata uygulamayı durdurmaz. Son hata ve hata sayacı `doctor` çıktısı için saklanır.
- [x] **Füzyon ve durum** (`engine.rs`):
  - Her `(account, kind, scope)` için güncel gözlem tutulur. Öncelik kuralı: aynı 30 s içinde `Statusline > Api > Estimate`.
  - `Estimate` dışındaki gözlemler `limit_snapshots` tablosuna yazılır. Tekrar yazmayı önlemek için aynı değer ve `resets_at` ile 60 s içinde gelen gözlemler atlanır, ama zaman serisi için en az 5 dakikada bir "kalp atışı" snapshot'ı yazılır.
  - `limit_windows`: yeni bir `resets_at` → yeni pencere satırı. `peak_percent` güncellenir. `percent ≥ 100` → `hit_limit = 1`.
  - **Reset algılama:** `now > resets_at` ve yeni gözlem yoksa yerel değer %0'a çekilir (`Estimate`, neden: reset) ve `LimitEvent::Reset` olayı yayınlanır.
  - **Bayatlık seviyeleri:** taze (< 10 dk), eski (10-30 dk), bayat (> 30 dk).
- [x] **Burn rate ve projeksiyon** (`rate.rs`):
  - Mevcut penceredeki son 30 dakikanın gözlemleri (≥ 3 nokta) üzerinden Theil–Sen eğimi (%/dk) hesaplanır.
  - Nokta sayısı yetersizse yedek yöntem kullanılır: `k0 × son 15 dk maliyet hızı`. Burada `k0 = p_now / C_pencere` (`architecture.md` §9.2 prior).
  - Çıktılar: `rate_per_min`, `eta_100: Option<Duration>`, `will_hit_before_reset: bool`, `projected_at_reset: f64`, `confidence`.
- [x] **v0.1 payı** (`share.rs`): Mevcut 5 saatlik pencere için oturum başına (subagent'lar ana oturuma eklenerek):
  - `C_oturum`
  - `pay = p_now × C_oturum / C_pencere`
  - Son 5 dakikalık token hızı
  
  Snapshot hiç yoksa yüzde gösterilmez; yalnızca maliyet ve token verilir. **Sahte yüzde uydurulmaz** (vizyon ilke 1).
- [x] **`PanelSnapshot`** (specta tipi):
  - `account` (etiket, plan, tier)
  - `limits: Vec<LimitView>`: kind, scope, percent, resets_at, severity, source, freshness, rate, projection
  - `live_sessions: Vec<SessionShareView>`: ad, proje, durum, `≈ %` pay, maliyet, token/dk
  - `index_state`
  - `api_state`: kapalı / aktif / token bayat / geri çekilmede / hata
  - `statusline_state`: hiç görülmedi / son görülme zamanı
  - `generated_at`
- [x] `CoreEvent::LimitsUpdated` olayı yalnızca görünür bir değişiklik olduğunda yayınlanır: yuvarlanmış yüzde, durum, kaynak veya tazelik değişimi.
- [x] Geliştirici kolaylığı: `TOKENRADAR_API_CONSENT=1` ortam değişkeni yalnızca debug derlemede onayı simüle eder.
- [x] Testler:
  - Parser fixture'ları
  - Zamanlayıcı (sahte saat): tüm durum geçişleri, geri çekilme, 401 kurtarma
  - Füzyon öncelik kuralı
  - Pencere geçişi
  - Reset algılama
  - Theil–Sen: bilinen eğim + gürültü
  - Pay toplamının `p_now`'a eşit olması (korunum)
- [x] Gerçek doğrulama (yerel): Onay açıkken uygulamayı 30 dk çalıştır. Snapshot'ların aralıkları beklenen polling politikasına uymalı, gösterilen yüzdeler `/usage` çıktısıyla ±1 puan içinde eşleşmeli.

## Teknik notlar ve tuzaklar

- **Kimlik bilgisi dosyasını okurken** `accessToken` ve `refreshToken` alanları `IgnoredAny` ile atlanan bir struct kullanılır. Böylece plan meta verisi okunurken token belleğe hiç girmez. Token gerektiğinde **ayrı bir struct** yalnızca `accessToken` alanını okur.
- `resets_at` değerleri S2'de ISO 8601, S1'de epoch saniye biçimindedir. İkisi de jiff `Timestamp` tipine normalize edilir.
- `limits[].percent` tam sayıdır, düz alanlardaki `utilization` ise float'tır (ama gözlemde tam sayı değerli). Eşitlik karşılaştırmalarında tolerans kullan.
- S2 yanıtında `ratelimit` başlıkları yok. Polling'in nezaketi tamamen bizim sorumluluğumuzda.
- Tek bir hesap için birden fazla `weekly_scoped` olabilir (her model için bir tane). UI listesi dinamik olmalı.

## Kabul kriterleri

- [x] Onay kapalıyken **hiçbir ağ isteği atılmaz**. Bu, bir testle doğrulanır: istemci mock'u çağrı sayısını kontrol eder.
- [x] Onay açıkken polling politikası tabloya uyar. 30 dakikalık gerçek çalıştırmada istek sayısı beklenen aralıktadır. _30 dk'da 7 istek, aralıklar 4:48–5:29 (Idle politikası: 5 dk ±%10)._
- [x] Gerçek ortamda `/usage` ile ±1 puan uyum sağlanır. _Üç limitte de 0 puan fark; reset saatleri dakikasına kadar aynı._
- [x] Token hiçbir log satırında, hata mesajında veya `PanelSnapshot` içinde görünmez. Log'larda `sk-ant-` deseni aranır ve bulunmamalıdır. _Log dosyası henüz yok (aşağıda açık konular); arama gerçek çalıştırmanın stderr'ine ve `--nocapture` test çıktısına yapıldı: 0 eşleşme._
- [x] Tüm testler geçer.

## Doğrulama

```powershell
cargo test -p tokenradar-core limits usage_api credentials
$env:TOKENRADAR_API_CONSENT="1"; pnpm dev
Select-String -Path "$env:LOCALAPPDATA\TokenRadar\logs\*.log" -Pattern "sk-ant-"   # sonuç boş olmalı
```

## Devir notu

**Tarih:** 2026-10-27 · **Dal:** `step/06-limit-engine-api` · **Karar:** ADR-028

### Yapılanlar
- **`clock.rs`:** `Clock` trait'i, `SystemClock`, testler için `ManualClock`.
- **`config.rs`:** `AppConfig { schema_version, api_consent }`. `ApiConsent { granted, granted_at, version }`; geçerli sayılması için `version == API_CONSENT_VERSION` (1) olmalı. Dosya yoksa varsayılan (onay yok). `TOKENRADAR_API_CONSENT=1` yalnızca debug derlemede onay sayılır.
- **`providers/claude/credentials.rs`:**
  - `read_plan` → `PlanInfo { subscription_type, rate_limit_tier }`. Kullanılan struct'ta token alanı yok; token belleğe hiç girmez.
  - `read_access_token` → `Secret<String>`. Yalnızca `accessToken` alanı tanımlı, buffer `Zeroizing`, 64 KB sınırı. `refreshToken` hiçbir yerde tanımlı değil.
  - Hata mesajları serde mesajını değil yalnızca kategori ve satır/sütunu taşır (serde değerleri tırnak içinde alıntılayabiliyor).
- **`providers/claude/usage_api/`:**
  - `parse.rs`: önce `limits[]`, yoksa düz alanlar (`five_hour`, `seven_day`, `seven_day_<model>`). `SurfaceBreakdown`, `ExtraUsage`. Bilinmeyen `kind` → `Other`.
  - `client.rs`: `reqwest` + `rustls`, ilk istekte kurulan `current_thread` runtime. 10 s zaman aşımı, 1 MB gövde sınırı, `set_sensitive(true)` Authorization başlığı, 100 ms'de bir iptal yoklaması.
  - `schedule.rs`: `PollScheduler` (§10 tablosu, 60 s sert alt sınır, ±%10 jitter). `ApiState { Off, Active, TokenStale, Backoff, Offline, Error }`, `PollDiagnostics` (istek/hata sayacı, son hata, son başarı).
  - `mod.rs`: `UsageApiPoller` thread gövdesi. Token istekten hemen önce okunur ve yanıta bakılmadan drop edilir.
- **`limits/`:**
  - `engine.rs`: füzyon (30 s içinde Statusline > Api > Estimate), heartbeat'li kalıcılık, ±5 dk toleranslı pencere eşleşmesi, reset algılama (başlangıçta sessiz), eşik olayları (75/90/100), projeksiyon olayları, `poll_inputs`, `snapshot`.
  - `rate.rs`: Theil–Sen (son 30 dk, ≥ 3 nokta; ≥ 5 nokta yüksek güven), yedek `k0 × 15 dk maliyet hızı` (düşük güven).
  - `share.rs`: `p_now × C_s / C_pencere`, subagent'lar köke eklenir; korunum proptest'i.
  - `view.rs`: `PanelSnapshot`, `LimitView`, `SessionShareView`, `ApiStatusView`, `VisibleKey`, `LimitEvent`.
- **`store/windows.rs`:** `upsert_window`, `insert_breakdown`, `root_usage_between`, `cost_between`, `session_labels`. `Writer::record_snapshot` snapshot'ı ve pencereyi tek transaction'da yazar.
- **`runtime.rs`:** `core-limits` thread'i ve hesap başına `core-api-<id>`. `CoreEvent::LimitsUpdated(Box<PanelSnapshot>)` ve `CoreEvent::Limit(LimitEvent)`. `CoreStatus.limits`. `CoreHandle::{ingest_limit, panel_opened, set_api_consent}`. Snapshot en fazla 2 s'de bir yeniden kurulur; boşta thread'ler bir sonraki reset/tazelik adımına kadar uyur.
- **Tauri (`core_bridge.rs`):** `config.toml`'dan onay okunur (okunamazsa onay yok sayılır). `TOKENRADAR_DEBUG_EVENTS=1` günlüğüne `limits_updated` ve `limit_*` satırları eklendi (yalnızca yüzde, kaynak, durum, zaman).
- **Testler:** core'da toplam 155 test (limits 26, usage_api 21, credentials 8, yeni runtime testi `a_limit_reading_reaches_the_panel_snapshot`). Token sızıntısı testi `tracing` çıktısını TRACE seviyesinde yakalar ve token'ın loglarda, hata mesajlarında ve `PollUpdate`'te olmadığını doğrular.

### Gerçek doğrulama (2026-10-27, 30 dk, onay açık)
- **İstekler:** 30 dakikada 7 istek, 0 hata. Aralıklar 4:48, 4:48, 4:54, 5:29 (+ biri DB yazım zamanından çıkarıldı). Hepsi Idle politikasının (5 dk ±%10) içinde. Statusline henüz yok (Adım 07), oturum çoğunlukla Idle göründü; Busy (2 dk) hiç devreye girmedi.
- **`/usage` karşılaştırması:** üç limitte (5 saat, haftalık, haftalık model) **0 puan fark**; reset saatleri dakikasına kadar aynı. Kişisel yüzdeler bilerek buraya yazılmadı.
- **Heartbeat:** 7 istekten 4'ü yazıldı (12 satır = 4 × 3 limit); aynı değer 5 dk dolmadan tekrar yazılmadı. 3 pencere satırı.
- **`resets_at` kayması:** API aynı pencere için `07:59:59.6xx` ile `08:00:00.3xx` arasında değerler döndürdü. ±5 dk tolerans bunları tek pencerede tuttu.
- **Token taraması:** gerçek çalıştırmanın stderr'i (95 satır) ve `cargo test -p tokenradar-core -- --nocapture` çıktısı (171 satır) içinde `sk-ant-`: 0. stderr'de `Bearer|authorization|accessToken`: 0. Kaynakta sahte `sk-ant-` token'ı 9 kez test girdisi olarak geçiyor, yani arama boşa geçmiyor.

### Sapmalar
- **ADR-028 (HTTP ve çevrimdışı davranış kullanıcı kararı):** yerel `current_thread` runtime'lı `reqwest`; işletim sistemi ağ olayları yerine geri çekilen yoklama (60 s → 30 dk), panel açılınca veya credentials değişince hemen deneme.
- **Kalıcılık:** Plan "aynı değer 60 s içinde atlanır, 5 dk'da bir heartbeat" diyordu. Uygulanan: aynı kaynak + aynı pencere + aynı yüzde **5 dk** içinde atlanır. Değişen her değer anında yazılır (ADR-028).
- `eta_100_at` süre (`Duration`) değil mutlak zaman damgası. `PanelSnapshot`'a plana ek olarak `top_sessions` (penceredeki en pahalı 3 kök oturum) eklendi; snapshot tek birincil hesap taşır.
- `tokens_per_min`, `cache_read` tokenlarını saymaz.
- `CoreStatus` ve `CoreEvent` artık `Eq` türetmiyor (`f64` alanlar).
- Plandaki doğrulama komutları birebir çalışmıyor:
  - `cargo test -p tokenradar-core limits usage_api credentials`: cargo tek filtre kabul eder; üç ayrı komut çalıştırıldı.
  - `Select-String ... logs\*.log -Pattern sk-ant-`: log dosyası henüz yok, komut **boşa geçer**. Yerine stderr ve test çıktısı tarandı (yukarıda).

### Açık konular
- **Log dosyası ve `tracing` subscriber'ı yok.** Uygulama `tracing` uyarılarını hiçbir yere yazmıyor; plandaki log taraması bu yüzden şimdilik anlamsız. Log dosyası geldiğinde (doctor / log adımı) aynı tarama dosyalara karşı yeniden yapılmalı ve bir CI testi eklenmeli.
- **Her istek gözlemlenemiyor.** Debug günlüğü yalnızca görünür değişiklikte satır basar; değeri değişmeyen bir istek ne günlüğe ne DB'ye (heartbeat) düşer. `PollDiagnostics` sayaçları `CoreStatus.limits.api` içinde mevcut; `doctor` çıktısı (Adım 11) bunları göstermeli.
- Busy (2 dk) ve statusline-taze (15 dk) aralıkları gerçek ortamda gözlenmedi; yalnızca birim testleriyle doğrulandı. Offline/Backoff/TokenStale geçişleri de yalnızca testlerde görüldü.
- `aws-lc-rs` (rustls varsayılanı) ikili boyutunu artırır; ölçülmedi. Boyut bütçesi ölçümünde (Adım 13) bakılmalı, gerekirse `ring`'e geçiş değerlendirilmeli.
- `architecture.md` §10 tablosu ADR-028'deki çevrimdışı geri çekilmeyi henüz anmıyor.

### Sonraki adıma notlar (Adım 07)
- Statusline gözlemleri için `CoreHandle::ingest_limit(LimitObservation)` hazır. `source: Source::Statusline` verildiğinde füzyonda API'yi yener ve poller'ı 15 dk aralığa çeker (`PollInputs.statusline_at`).
- S1 `resets_at` epoch saniyedir; `Timestamp::from_millis(secs * 1000)` ile verin, pencere eşleşmesi toleranslıdır.
- `CoreHandle::panel_opened()` panel açılışında çağrılmalı (60 s'den eski veriyi tazeler). `set_api_consent(bool)` Adım 11'deki ayar ekranı için.
- UI `CoreStatus.limits` (Resync) ve `CoreEvent::LimitsUpdated`'i dinlemeli; `CoreEvent::Limit` bildirim kuralları (Adım 11) içindir.
