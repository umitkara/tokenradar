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

- [ ] **Parser** (`usage_api.rs`):
  - `limits[]` → `LimitObservation` listesi. `kind` değerleri `LimitKind` enum'una eşlenir; bilinmeyen `kind` için `Other(String)`. `scope.model.display_name` → `Scope::Model`.
  - `limits[]` yoksa veya boşsa: `five_hour` → `Session`, `seven_day` → `WeeklyAll`, `seven_day_opus/sonnet/...` (null değilse) → `WeeklyScoped(model)`.
  - `seven_day_breakdown` → `surface_breakdown` tablosu.
  - `extra_usage` ve `spend` → özet struct. UI'da yalnızca "extra usage etkin mi?" bilgisi gösterilecek.
  - Kod adlı ve bilinmeyen alanlar sessizce yok sayılır. **Fixture testleri:** gerçek yanıt, sentetik düz şema, boş `limits`, bilinmeyen `kind`, eksik `resets_at`.
- [ ] **İstemci:**
  - `reqwest` (rustls), zaman aşımı 10 s, `User-Agent: TokenRadar/<sürüm>`.
  - Başlıklar `data-sources.md` S2'deki gibi.
  - Token `Secret<String>` içinde tutulur ve istek biter bitmez drop edilir.
- [ ] **Zamanlayıcı:**
  - `architecture.md` §10'daki tabloya göre çalışır. Girdiler: S1 tazeliği, canlı oturum durumları, son başarılı istek, `resets_at` zamanları.
  - **Sahte saat** (`Clock` trait'i) ile test edilebilir olmalı.
  - Sert sınır: 60 s'de en fazla 1 istek. Jitter ±%10.
- [ ] **Hata durumları:**
  - 401 → `TokenStale`; bir sonraki `CredentialsChanged` olayına (Adım 05) kadar istek atılmaz.
  - 429/5xx → üstel geri çekilme (60 s → 30 dk), `Retry-After` başlığına uyulur.
  - Ağ hatası → çevrimdışı modu.
  - Hiçbir hata uygulamayı durdurmaz. Son hata ve hata sayacı `doctor` çıktısı için saklanır.
- [ ] **Füzyon ve durum** (`engine.rs`):
  - Her `(account, kind, scope)` için güncel gözlem tutulur. Öncelik kuralı: aynı 30 s içinde `Statusline > Api > Estimate`.
  - `Estimate` dışındaki gözlemler `limit_snapshots` tablosuna yazılır. Tekrar yazmayı önlemek için aynı değer ve `resets_at` ile 60 s içinde gelen gözlemler atlanır, ama zaman serisi için en az 5 dakikada bir "kalp atışı" snapshot'ı yazılır.
  - `limit_windows`: yeni bir `resets_at` → yeni pencere satırı. `peak_percent` güncellenir. `percent ≥ 100` → `hit_limit = 1`.
  - **Reset algılama:** `now > resets_at` ve yeni gözlem yoksa yerel değer %0'a çekilir (`Estimate`, neden: reset) ve `LimitEvent::Reset` olayı yayınlanır.
  - **Bayatlık seviyeleri:** taze (< 10 dk), eski (10-30 dk), bayat (> 30 dk).
- [ ] **Burn rate ve projeksiyon** (`rate.rs`):
  - Mevcut penceredeki son 30 dakikanın gözlemleri (≥ 3 nokta) üzerinden Theil–Sen eğimi (%/dk) hesaplanır.
  - Nokta sayısı yetersizse yedek yöntem kullanılır: `k0 × son 15 dk maliyet hızı`. Burada `k0 = p_now / C_pencere` (`architecture.md` §9.2 prior).
  - Çıktılar: `rate_per_min`, `eta_100: Option<Duration>`, `will_hit_before_reset: bool`, `projected_at_reset: f64`, `confidence`.
- [ ] **v0.1 payı** (`share.rs`): Mevcut 5 saatlik pencere için oturum başına (subagent'lar ana oturuma eklenerek):
  - `C_oturum`
  - `pay = p_now × C_oturum / C_pencere`
  - Son 5 dakikalık token hızı
  
  Snapshot hiç yoksa yüzde gösterilmez; yalnızca maliyet ve token verilir. **Sahte yüzde uydurulmaz** (vizyon ilke 1).
- [ ] **`PanelSnapshot`** (specta tipi):
  - `account` (etiket, plan, tier)
  - `limits: Vec<LimitView>`: kind, scope, percent, resets_at, severity, source, freshness, rate, projection
  - `live_sessions: Vec<SessionShareView>`: ad, proje, durum, `≈ %` pay, maliyet, token/dk
  - `index_state`
  - `api_state`: kapalı / aktif / token bayat / geri çekilmede / hata
  - `statusline_state`: hiç görülmedi / son görülme zamanı
  - `generated_at`
- [ ] `CoreEvent::LimitsUpdated` olayı yalnızca görünür bir değişiklik olduğunda yayınlanır: yuvarlanmış yüzde, durum, kaynak veya tazelik değişimi.
- [ ] Geliştirici kolaylığı: `TOKENRADAR_API_CONSENT=1` ortam değişkeni yalnızca debug derlemede onayı simüle eder.
- [ ] Testler:
  - Parser fixture'ları
  - Zamanlayıcı (sahte saat): tüm durum geçişleri, geri çekilme, 401 kurtarma
  - Füzyon öncelik kuralı
  - Pencere geçişi
  - Reset algılama
  - Theil–Sen: bilinen eğim + gürültü
  - Pay toplamının `p_now`'a eşit olması (korunum)
- [ ] Gerçek doğrulama (yerel): Onay açıkken uygulamayı 30 dk çalıştır. Snapshot'ların aralıkları beklenen polling politikasına uymalı, gösterilen yüzdeler `/usage` çıktısıyla ±1 puan içinde eşleşmeli.

## Teknik notlar ve tuzaklar

- **Kimlik bilgisi dosyasını okurken** `accessToken` ve `refreshToken` alanları `IgnoredAny` ile atlanan bir struct kullanılır. Böylece plan meta verisi okunurken token belleğe hiç girmez. Token gerektiğinde **ayrı bir struct** yalnızca `accessToken` alanını okur.
- `resets_at` değerleri S2'de ISO 8601, S1'de epoch saniye biçimindedir. İkisi de jiff `Timestamp` tipine normalize edilir.
- `limits[].percent` tam sayıdır, düz alanlardaki `utilization` ise float'tır (ama gözlemde tam sayı değerli). Eşitlik karşılaştırmalarında tolerans kullan.
- S2 yanıtında `ratelimit` başlıkları yok. Polling'in nezaketi tamamen bizim sorumluluğumuzda.
- Tek bir hesap için birden fazla `weekly_scoped` olabilir (her model için bir tane). UI listesi dinamik olmalı.

## Kabul kriterleri

- [ ] Onay kapalıyken **hiçbir ağ isteği atılmaz**. Bu, bir testle doğrulanır: istemci mock'u çağrı sayısını kontrol eder.
- [ ] Onay açıkken polling politikası tabloya uyar. 30 dakikalık gerçek çalıştırmada istek sayısı beklenen aralıktadır.
- [ ] Gerçek ortamda `/usage` ile ±1 puan uyum sağlanır.
- [ ] Token hiçbir log satırında, hata mesajında veya `PanelSnapshot` içinde görünmez. Log'larda `sk-ant-` deseni aranır ve bulunmamalıdır.
- [ ] Tüm testler geçer.

## Doğrulama

```powershell
cargo test -p tokenradar-core limits usage_api credentials
$env:TOKENRADAR_API_CONSENT="1"; pnpm dev
Select-String -Path "$env:LOCALAPPDATA\TokenRadar\logs\*.log" -Pattern "sk-ant-"   # sonuç boş olmalı
```

## Devir notu

_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
