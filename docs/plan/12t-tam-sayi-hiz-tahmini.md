# Adım 12t — Tam sayı okumalarda hız tahmini ve güvene bağlı uyarı

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1.x | 12s | 1 oturum | ✅ |

## Amaç

Öngörü ("reset'ten önce dolacak"), tam sayı yüzde veren kaynaklarda dolma hızını gerçekte olduğundan birkaç kat yüksek hesaplayabiliyor. Ayrıca bu güvenilmez tahmin Claude Code mod'unun bandını açabiliyor. Bu adım:

1. Hızı tam sayı okumalardan doğru tahmin edilecek şekilde **adım kenarlarından** hesaplar.
2. Mod'un bandını ve uyarılarını tahminin **güvenine** bağlar.

**Bulgu (2026-11-13, kullanıcının mod ekran görüntüsü):**

- Mod bandı "Haftalık (Fable) limit ≈ 1 g 12 sa içinde dolar, sıfırlanmadan önce" gösterdi. Gerçek okumalara bakıldığında bu limit, sıfırlanmadan önce dolmayacaktı.
- **Neden:** `rate.rs` hızı son 30 dakikanın okumalarından (`RATE_LOOKBACK_MS`) Theil–Sen eğimiyle hesaplıyor. Kapsamlı haftalık limit yalnızca OAuth usage API'den geliyor. API tam sayı yüzde veriyor ve yaklaşık 15 dakikada bir yoklanıyor. Bu nedenle 30 dakikalık pencerede en fazla bir adım (1 puan) görülüyor.
  - Pencerede adım yoksa eğim 0 çıkıyor ve öngörü kayboluyor.
  - Adım varsa eğim "1 puan / adımdan önceki ve sonraki okumalar arasındaki süre" çıkıyor. Bu süre birkaç saatlik gerçek adım aralığından çok daha kısa.
- **Örnek (sentetik):** Her 15 dakikada bir okuma var ve gerçek hız saatte 1 puan. Adımdan hemen sonra 30 dakikalık pencerede `n, n, n+1, n+1` okumaları kalıyor. Theil–Sen medyanı yaklaşık 1 puan / 20–25 dk çıkıyor, yani gerçeğin 2,5–3 katı.
- **Sonuç:** Bant, bir adım pencereye girince açılıp pencereden çıkınca kapanarak gidip geliyor.
- **Uygulamanın bildirimleri** bundan büyük ölçüde korunuyor: `notifier::projection_rule` `Low` güveni ve 60 dakikadan uzak ETA'yı (`PROJECTION_HORIZON_MIN`) yok sayıyor. **Mod ise** `will_hit_before_reset` ve `eta_100_at` değerlerini güven ve ufuk filtresi olmadan kullanıyor (`integrations/claude-code/hooks/model.ts`, `alertsOf`).

## Önce oku

- `CLAUDE.md` "Honest numbers"
- ADR-043 (mod), ADR-060 (alt sınır füzyonu); Adım 12s'nin devir notu
- `docs/architecture.md` §7 (burn rate, projeksiyon)
- `crates/tokenradar-core/src/limits/{rate.rs, engine.rs, fusion.rs, view.rs}`, `limits/engine/tests.rs`
- `crates/tokenradar-core/src/notifier/mod.rs` (`projection_rule`), `notifier/prefs.rs`
- `crates/tokenradar-cli/src/report.rs` (`status --json` alanları)
- `integrations/claude-code/hooks/{model.ts, view.tsx, i18n.ts}`, `integrations/claude-code/tests/model.test.ts`

## Kullanıcıyla kararlaştırılanlar (2026-11-13)

- Bu iş ayrı bir adımda yapılır (bu adım).
- Kapsamda iki öneri var: **(1)** hızı adım kenarlarından hesaplamak, **(3)** mod bandını ve uyarılarını güvene bağlamak.
- **(2)** "geriye bakışı pencere uzunluğuna göre ölçeklemek" tek başına seçilmedi. (1)'in tasarımı içinde gerekirse kullanılabilir.
- Öngörü mantığı değiştiği için yeni bir ADR yazılır.

## Kullanıcıyla kararlaştırılanlar (2026-11-20)

- **Kenar hızı:** Kenarlar her kaynağın kendi ham okumalarından çıkarılır (birikimli maksimum, Adım 13'teki `intervals` mantığı). Belirsizliği 45 dakikadan büyük kenar atılır. Pencere içindeki son en fazla 4 keskin kenar kullanılır: hız = (son − ilk kenarın puanı) / aradaki süre. Kaynak başına hesaplanır, keskin kenarı çok olan kaynak seçilir. Durma tavanı: hız ≤ 1 puan / (aynı değeri gösteren son okuma − son kenar).
- **Güven:** 1 aralık (2 kenar) `Low`, 2 aralık `Medium`, 3 ve daha fazlası `High`. İlk ve son kenarın ortalama zaman belirsizliği aralığın %25'inden büyükse bir düzey düşer. Tek kenardan hız çıkmaz.
- **Öncelik:** kenar hızı (≥ `Medium`) > kalibre yedek (`Low`) > kenar hızı (`Low`) > `k0` önseli. Kapsamlı limitin maliyet yedeği bu adıma girmez (devir notuna).
- **Mod ufku:** Uygulamayla aynı kural: güven `Low` değilse ve ETA 60 dakika (`PROJECTION_HORIZON_MIN`) içindeyse. `projection_confidence` yoksa bant açılmaz. Mod panelinde düşük güvenli öngörü silik.

## Öneriler (adım başında kullanıcıya sorulacak)

Aşağıdakiler ajanın önerileridir, kullanıcı onaylamadan karar sayılmaz.

1. **Adım kenarı hızı.** Tam sayı kaynakta değerin değiştiği okumalar "kenar" sayılır. Hız, ilk ve son kenar arasındaki puan farkının bu kenarlar arasındaki süreye bölünmesiyle hesaplanır.
   - Bir kenarın gerçek zamanı, kenardan önceki ve sonraki okumanın arasında bir yerdedir. Önerim bu aralığın ortasını almak.
   - Son kenardan bu yana geçen süre, son aralıktan uzunsa hız aşağı çekilir: `rate ≤ 1 puan / (now − son kenar)`. Böylece kullanım durunca öngörü kendiliğinden söner.
2. **Ne zaman kenar yöntemi?** Bir kaynak tam sayıysa (`fusion::interval` genişliği > 0) kenar yöntemi kullanılır. Bilinmeyen ya da ondalıklı kaynakta bugünkü Theil–Sen kalır.
3. **Geçmişin uzunluğu.** Kenar yöntemi 30 dakikadan uzun geçmiş ister. Önerim: geçmiş en fazla pencere başlangıcına ya da kaynağa göre birkaç saate kadar tutulur. Bellek sınırlı kalır, çünkü yalnızca kenarlar saklanır.
4. **Güven.** En az iki kenar (`n → n+1 → n+2`) yoksa güven `Low` olur. İki kenar `Medium`, üç ve daha fazlası `High` olur. Tek kenardan hız ancak `Low` güvenle çıkar ya da hiç çıkmaz.
5. **Mod uyarısı (öneri 3).** Mod, öngörüden band açmak için uygulamanın bildirim kuralını kullanır: `projection_confidence` `low` değilse ve ETA bir ufuk içindeyse. Ufuk için iki seçenek var:
   - (a) Uygulamanınkiyle aynı: `PROJECTION_HORIZON_MIN` (60 dk). `status --json` bunu taşır ya da mod sabit kullanır.
   - (b) Kalan pencere süresinin bir oranı.

   Önerim (a). Kullanıcı tek bir kural görür ve iki yüzey aynı anda uyarır.
6. **Ayrıntıda güven görünür.** Güveni düşük öngörü, mod panelinde ve tray Ayrıntı'da silik ve `≈` ile gösterilir, bant açmaz. `status --json`'da `projection_confidence` zaten var.

**Daha zayıf seçenekler (ADR'de alternatif olarak yazılır):**

- Yalnızca `RATE_LOOKBACK_MS`'yi uzatmak (öneri 2): Adım sayısı artar ama kenar zamanı belirsizliği sürer. Pencere içinde hız değişirse geç tepki verir.
- Theil–Sen'i API okumalarının aralık orta noktasıyla beslemek: Yuvarlama ofsetini düzeltir, adım/pencere oranı sorununu çözmez.
- Mod'da yalnızca metni yumuşatmak: Gidip gelme sürer.

## Kapsam

### 1. Çekirdek: hız

- `rate.rs`'e saf bir kenar fonksiyonu eklenir (`step_rate` ya da benzeri), birim testlidir.
- `Track` geçmişi kenar yöntemi için gereken uzunlukta tutulur. 12s'deki "yalnızca en yeni okuma geçmişe girer" kuralı korunur.
- **Füzyonla etkileşim (ADR-060):** Geçmiş birleşik alt sınır değerini tutuyor. Yalnızca API'den beslenen bir limitte bu değerin adımları API adımlarıyla aynı mı, yoksa statusline ile karışık limitte (`weekly_all`) yarım puanlık ara adımlar mı üretiyor, kontrol edilir. Kenar tanımı buna göre yapılır.
- Cost-rate fallback (`cost_rate`) ve `Low` güveni değişmez.

### 2. Bildirim ve mod

- `notifier::projection_rule` yeni güven kurallarıyla tekrar gözden geçirilir. Davranış değişirse testle sabitlenir.
- Mod: `parseStatus` `projection_confidence` değerini okur, `alertsOf` öngörü uyarısını güvene ve ufka bağlar. `model.test.ts` testleri yazılır.
- Mod'un Fluent/i18n metinleri yalnızca gerekirse değişir (silik ya da düşük güven ipucu).

### 3. Görünüm

- `LimitView`/`Projection` değişirse bindings yeniden üretilir.
- Tray Ayrıntı'daki öngörü satırı güveni yansıtır. Bugün nasıl gösterildiği adım başında kontrol edilir.

## Kapsam dışı

- Günlük kullanım örüntüsünü (uyku, mola) öngörüye katmak. Haftalık pencerede 7/24 kullanım varsayımı bu adımda kalır, devir notuna yazılır. Adım 13/17'nin örüntü verisi bunun için daha uygun.
- Kalibrasyon (Adım 13).
- Codex limitleri (Adım 20).
- `/usage` ile değer eşitliği (Adım 12s).

## Görevler

- [x] Öneriler 1–6 kullanıcıyla kararlaştırılır.
- [x] Core: kenar hızı fonksiyonu; testler (sabit hız, tek kenar, hiç kenar yok, durmuş kullanım, hız değişimi, pencere değişimi, karışık kaynak, tam sayı olmayan kaynakta Theil–Sen).
- [x] Core: `Track` geçmişinin uzunluğu ve bellek üst sınırı; test.
- [x] Bulgudaki senaryo (adım arası 15 dakikalık yoklamalar) için regresyon testi: öngörü gerçek hızın yakınında, `will_hit_before_reset` gidip gelmiyor.
- [x] Notifier: güven kuralı gözden geçirilir; gerekirse test.
- [x] Mod: `projection_confidence` okunur; bant ve toast güvene ve ufka bağlanır; testler.
- [x] Görünüm: Ayrıntı ve mod panelinde düşük güven silik; bindings ve Fluent gerekirse güncellenir.
- [x] `docs/architecture.md` §7 güncellenir; yeni ADR; devir notu.

## Teknik notlar ve tuzaklar

- **Gerçek veri git'e girmez:** Bulgu ve test senaryoları sentetiktir. Gerçek yüzde, okuma sayısı ya da zaman damgası yazılmaz.
- **Kenar zamanı belirsiz:** Kenarın gerçek zamanı iki okuma arasında herhangi bir yerde olabilir. Yoklama aralığı uzadıkça (ör. 401 bekleyişi, uygulama kapalı) belirsizlik artar ve güven buna göre düşmelidir.
- **API yoklama aralığı sabit değil:** Gerçek okumalar 5–19 dakika arayla geliyor. Kenar yöntemi sabit aralık varsaymamalı.
- **Uygulama kapalıyken:** Mod `origin: database` ile CLI'nin çevrimdışı yolunu okur. Orada öngörü üretilip üretilmediği kontrol edilir. Üretiliyorsa aynı kural geçerli olur.
- **Mod uygulamayla birlikte güncellenmez:** Mod eski CLI ile çalışabilir. `projection_confidence` yoksa mod öngörüden bant açmamalı.
- **Performans:** Öngörü her snapshot'ta tüm track'ler için hesaplanır. Geçmiş uzayınca maliyet artmamalı. Kenar listesi küçük kalır, Theil–Sen'in O(n²) çifti uzun geçmişte kullanılmaz.

## Kabul kriterleri

- Okumaları 15 dakikada bir gelen tam sayı bir kaynakta, sabit hızlı kullanımın öngörüsü gerçek hızın yakınında ve adım pencereye girip çıktıkça gidip gelmiyor.
- Kullanım durduğunda öngörü kendiliğinden sönüyor.
- Mod bandı, güveni `Low` olan ya da ufkun dışındaki bir öngörüden açılmıyor. Uygulama bildirimi ile mod bandı aynı kurala uyuyor.
- Her öngörü `≈` ile ve kaynağıyla gösteriliyor. Düşük güven silik görünüyor.

## Doğrulama

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
- `pnpm -C app check`, `pnpm -C app lint`, `pnpm -C app test`
- Mod testleri (`integrations/claude-code`, mevcut test komutu)
- Gerçek veriyle (salt okunur) yeni hız ile eski hız karşılaştırılır. Sonuç yalnızca biçim olarak yazılır ("eski yöntem gerçeğin birkaç katı, yeni yöntem yakın").

## Devir notu

**Tarih:** 2026-11-20 · **Karar:** ADR-062 (Önerildi; kullanıcı kararları işaretli, uygulama ayrıntıları onay bekliyor) · **Ölçüm:** `docs/perf/2026-11-20-step-edge-rate.md`

**Kullanıcıyla kararlaştırılanlar:** yukarıdaki "Kullanıcıyla kararlaştırılanlar (2026-11-20)" bölümü (öneriler 1–5'in hepsi önerilen biçimiyle).

**Yapılanlar:**
- **`rate.rs`:** `Steps` (kaynak başına birikimli maksimum, en fazla 4 keskin kenar, 45 dk belirsizlik sınırı; `calibration::intervals::Edge` yeniden kullanıldı), `Steps::rate` (ilk–son kenar, durma tavanı, güven ve bulanıklık düşürmesi), `StepRate`, `best_step_rate`, `Confidence::lower`, `RateMethod::Steps`. `projection` artık `steps` alır ve önceliği uygular: Theil–Sen (ondalıklı) > kenar (≥ orta) > kalibre > kenar (düşük) > `k0`.
- **Motor:** `Track.steps` (kaynak başına). Tam sayı okumalar (`fusion::interval(...).exact == false`) yalnızca kenarlara, ondalıklı okumalar 30 dakikalık Theil–Sen geçmişine gider. Aynı pencerede track değişince geçmiş, kenarlar ve `will_hit` birlikte taşınır (`Track::carry_rate`); reset kenarları temizler. Açılışta pencerenin saklı okumaları kenarları yeniden kurar (heartbeat 5 dk, kenarlar keskin kalır).
- **Notifier:** değişmedi; `projection_rule` `Low` güveni ve 60 dakikadan uzak ETA'yı zaten yok sayıyor ve bu testle sabit.
- **Mod:** `parseStatus` `projection_confidence`'ı okur (`RadarLimit.confidence`). `mergeRows` her satır için `soon` hesaplar (`warnsSoon`: `willHit`, güven orta/yüksek, `0 < ETA − now ≤ PROJECTION_HORIZON_MS` = 60 dk). `rowLevel` ve `alertsOf` `willHit` yerine `soon`'a bakar. Kart ve `/radar` raporu düşük güvenli öngörüye "(kaba)" / "(rough)" ekler (`fullIn`).
- **Görünüm:** bindings yeniden üretildi (`"steps"`). Uygulamanın Ayrıntı ve ana kartı düşük güveni zaten silik ve `≈` ile gösteriyordu; değişmedi.
- **Statusline dolma süresi (ek düzeltme, kullanıcı kararı):** `Window.eta` artık `Eta { ms, rough }`. Metin her zaman `≈` taşır (`sl-eta = ▲≈…`, EN/TR); güven `Low` ya da yoksa silik, değilse uyarı rengi. Ufuk uygulanmaz. Dal: `fix/12t-statusline-eta`.
- **Örnek:** `examples/rate_compare.rs` (salt okunur; yalnızca oran ve pay basar).
- **Testler:** çekirdek 539 (+17: rate 12, motor 4, statusline 2; bir motor testi ve statusline beklenen dizeleri güncellendi), CLI 28, uygulama 101, frontend 136, mod 42 (+1).

**Sapmalar:**
- Planda `history`'nin birleşik değeri kenar için kullanılabileceği düşünülmüştü. Birleşik değer `weekly_all`'da kaynağa göre farklı eşiklerde adım ürettiği için kenarlar ham okumalardan, kaynak başına çıkarıldı (kullanıcıya adım başında bildirildi).
- Theil–Sen artık tam sayı okumalarla hiç beslenmiyor (plan öneri 2'nin doğrudan sonucu). Mevcut motor testinde (dakikada bir okuma) güven `High`'dan `Medium`'a indi: 1 dk kenar belirsizliği 3 dk aralığın %33'ü.
- Mod'da `rowLevel`'e `now` vermek yerine karar `mergeRows`'ta `Row.soon` olarak verildi.

**Açık konular:**
- **ADR-062'nin uygulama ayrıntıları** (`RateMethod::Steps`, `Row.soon`, "(kaba)" eki) kullanıcı onayı bekliyor.
- **Gerçek veride yeni hız gerçeğin altında (≈ 0,4–0,6×, kullanım olan anlarda).** Kenar hızı geçmişe bakıyor ve durma tavanı moladan sonra hızı düşürüyor; uyarı geç gelebilir. Mutlak hata ve gidip gelme ise belirgin azaldı. İzlenmeli; gerekirse tavan ya da kenar sayısı ayarlanır.
- Yeni davranış gerçek Claude Code mod'unda ve kurulu uygulamada gözlenmedi (yeni sürüm gerekli).
- Kapsamlı limitin kalibre maliyet yedeği yapılmadı (kullanıcı kararı); seyrek API adımlarında kapsamlı limitte öngörü eskisine göre daha az zaman var.

**Sonraki adıma notlar:**
- **Kapsam dışı kalan:** günlük kullanım örüntüsü (uyku, mola) öngörüye katılmadı; haftalık pencerede 7/24 varsayımı sürüyor. Adım 17'nin örüntü verisi bunun için uygun.
- **Adım 14 (atıf):** atıf hızı değil maliyeti kullanıyor; bu adımın değişikliği atıfı etkilemez.
- `rate::projection` çağıran her yer `steps` argümanını vermeli; kenar takibi `Track` içinde, dışarıdan yalnızca `Steps` ve `best_step_rate` kullanılabilir.
- Karşılaştırmayı yeniden çalıştırmak için: `cargo run --release -p tokenradar-core --example rate_compare -- --db <kopya>`. Gerçek DB'ye değil, `.backup` kopyasına karşı.
