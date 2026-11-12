# Adım 12s — Limit yüzdelerinin yuvarlama farkını füzyonda gidermek

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1.x | 12r | 1 oturum | ✅ |

## Amaç

Panelin gösterdiği limit yüzdesi, Claude Code'un `/usage` ekranından 1 puan düşük olabiliyor. Bu adım farkın nedenini doğrular ve füzyonu, iki kaynağın tam sayı okumalarını birleştirerek gerçeğe en yakın değeri gösterecek hâle getirir.

**Bulgu (12r devir notu, 2026-11-12):**

- Statusline `rate_limits.*.used_percentage` ve OAuth usage API `utilization` değerleri şimdiye kadar hep tam sayı geldi.
- ±2 dakika içindeki API/statusline çiftlerinde `statusline − API` farkı yaklaşık yarı yarıya 0 ya da −1. +1 hiç görülmedi.
- Bu dağılım şu varsayıma uyuyor: **API en yakın tam sayıya yuvarlıyor, statusline aşağı yuvarlıyor (floor).** Bu, belgelenmiş bir davranış değil, veriden yapılan bir çıkarım.
- Füzyon kuralına göre (`limits/engine.rs`, `FUSION_WINDOW_MS`: 30 saniye içinde statusline kazanır, sonrasında en yeni okuma) panel çoğu zaman statusline değerini gösteriyor. API yoklamasından hemen sonra 1 puan yukarı çıkıp sonra geri inebiliyor.

## Önce oku

- `CLAUDE.md` "Honest numbers"; ADR-028 (heartbeat), ADR-041 (limit varlığı), ADR-059
- Adım 12r'nin devir notu ("Statusline ile API arasında 1 puanlık yuvarlama farkı")
- `docs/architecture.md` §7–§8 (limit motoru, füzyon)
- `docs/data-sources.md`: S1 (statusline `rate_limits`), OAuth usage API
- `crates/tokenradar-core/src/limits/{engine.rs, view.rs, rate.rs, history.rs, detail.rs}`, `limits/engine/tests.rs`
- `crates/tokenradar-core/src/providers/claude/usage_api/parse.rs`, `statusline/payload.rs`

## Kullanıcıyla kararlaştırılanlar (2026-11-12)

- Fark ayrı bir adımda ele alınır (bu adım). Füzyon kuralının değişmesi yeni bir ADR gerektirir.

## Öneriler (adım başında kullanıcıya sorulacak)

Aşağıdakiler ajanın önerileridir, kullanıcı onaylamadan karar sayılmaz.

1. **Aralık füzyonu.** Her okuma gerçek değer için bir aralık verir:
   - statusline `n` → `[n, n+1)`
   - API `m` → `[m−0,5, m+0,5)`

   Aynı pencereye ait (`same_window`) en yeni statusline ve en yeni API okumasının kesişimi alınır. Gösterilen değer kesişimden üretilir (ör. `[5,5, 6)` → 6). Kesişim boşsa (API yoklamaları arasında statusline yükseldiyse) en yeni okumanın aralığına geçilir.
2. **Kaynak etiketi.** Kesişimden çıkan değer bir ölçümdür, `≈` işareti almaz. Etiket ne olmalı: "Canlı", "API" ya da birleşik bir etiket ("Canlı + API")? Önerim: değeri belirleyen en yeni okumanın kaynağı görünür, ipucu iki kaynağı da sayar.
3. **Kesişimin geçerlilik süresi.** API okuması ne kadar eskiyse kesişim o kadar riskli olur, çünkü bu arada kullanım artmış olabilir. Önerim: API okuması statusline'dan en fazla bir yoklama aralığı (`usage_api` poll süresi) eskiyse kesişime girer. Daha eskiyse yalnızca statusline aralığı kullanılır.
4. **Saklanan değer değişmez.** `limit_snapshots` her kaynağın ham değerini saklamaya devam eder. Birleştirme yalnızca bellekteki `fused` değerinde yapılır. Böylece geçmiş veri yeniden yorumlanabilir.

**Daha zayıf seçenekler (ADR'de alternatif olarak yazılır):**

- API'yi hep tercih etmek: statusline'ın tazeliği kaybolur.
- Statusline değerine +0,5 eklemek: ölçümü tahminle değiştirir.
- Hiçbir şey yapmamak ve ipucunda farkı açıklamak: gidip gelme sürer.

## Kapsam

### 1. Varsayımı doğrulamak

- Gerçek `limit_snapshots` (salt okunur) üzerinde API/statusline çiftleri tüm limit türleri ve kapsamlar (`weekly_all`, `weekly_scoped`, varsa `session`) için karşılaştırılır.
- Floor/round varsayımı tutmuyorsa (ör. +1 farkı görülürse) **dur ve kullanıcıya sor.** Plan buna göre değişir.
- Sonuç `docs/data-sources.md`'ye yazılır. Gerçek yüzdeler ve çift sayıları yazılmaz, yalnızca fark dağılımının biçimi yazılır ("yaklaşık yarı yarıya 0 / −1, +1 yok").
- Tam sayı olmayan bir değer gelirse aralık ±0 kabul edilir, yani okuma tam değer sayılır. Bu durum testle sabitlenir.

### 2. Füzyon

- `Track` her kaynağın son gerçek okumasını tutar (bugün yalnızca `fused` ve `written` var).
- Kesişim hesabı saf bir fonksiyondur (`limits/fusion.rs` ya da `engine.rs` içinde), birim testlidir.
- `fused.percent`'i kullanan her yer yeni değeri görür: panel, tray halkası, eşik olayları (`THRESHOLDS`), hız/öngörü (`rate.rs`), pencere zirvesi (`peak_percent`).
- `rate.rs` geçmişi (`history`) karışık kaynaklı tam sayılarla bugün de testere dişi üretiyor olabilir. Bu da kontrol edilir: öngörü, birleştirilmiş değerle mi beslenmeli?

### 3. Görünüm

- `LimitView` değişirse (ör. birleşik kaynak) bindings yeniden üretilir. Fluent (EN/TR): kaynak etiketi ve ipucu.
- Ayrıntı görünümündeki "Canlı"/"API" rozetinin anlamı karara göre güncellenir.

## Kapsam dışı

- Claude Code'un yuvarlama davranışını belgelemesini istemek ya da ona bağlı kalmak.
- Codex limitleri (Adım 20). Codex günlüğünün hassasiyeti orada ele alınır.
- Eski snapshot'ları yeniden yazmak.

## Görevler

- [x] Doğrulama sorgusu, sonuç `docs/data-sources.md`'de.
- [x] Kullanıcıyla öneriler 1–4 kararlaştırılır.
- [x] Core: kaynak başına son okuma, kesişim fonksiyonu; testler (eşit, −1, boş kesişim, eski API, tam sayı olmayan değer, pencere değişimi, reset tahmini ile etkileşim).
- [x] Core: eşik olaylarında gidip gelme yok (89 ↔ 90 sınırında tek olay); test.
- [x] Görünüm: kaynak etiketi ve ipucu, bindings, Fluent.
- [x] Yeni ADR ve devir notu.

## Teknik notlar ve tuzaklar

- **Gerçek veri git'e girmez:** Doğrulama sonuçlarında kullanım yüzdesi, toplam ya da çift sayısı yer almaz.
- **Kesişim yalnızca aynı pencerede:** `resets_at` farklı iki okuma birleştirilmez. API `resets_at` milisaniye kesirleri taşıyor (`…800746`), statusline saniyeye yuvarlıyor. Mevcut `same_window` toleransı korunur.
- **Reset tahmini:** `resets_at` geçtiğinde motor `Estimate` 0 % üretir. Kesişim yalnızca gerçek okumalar arasında yapılır.
- **Heartbeat (ADR-028):** Saklama kuralı değişmez. Birleştirilmiş değer saklanmaz.

## Kabul kriterleri

- Panel değeri API yoklamasından sonra gidip gelmiyor ve en geç bir API yoklamasında `/usage` ile eşitleniyor. (2026-11-12'de kullanıcıyla güncellendi; önceki hâli: "API ve statusline okumaları 1 puan farklıyken panel `/usage` ile aynı değeri gösteriyor ve API yoklamasından sonra gidip gelmiyor". Statusline bir tam sayıyı geçtiği anda `/usage`'ın `n` mi `n+1` mi gösterdiği tahmin yapmadan bilinemez, ADR-060.)
- Statusline yeni bir artış gösterdiğinde panel API'yi beklemeden yükseliyor.
- Hiçbir birleştirilmiş değer `≈` olmadan tahmin içermiyor. Her değerin kaynağı görünür.

## Doğrulama

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
- `pnpm -C app check`, `pnpm -C app lint`, `pnpm -C app test`
- Kurulu ya da portable build ile panel ve `/usage` yan yana karşılaştırılır (kullanıcıyla).

## Devir notu

**Tarih:** 2026-11-12 · **Karar:** ADR-060 · **Ölçüm:** `docs/perf/2026-11-12-limit-rounding-fusion.md`

**Kullanıcıyla kararlaştırılanlar (2026-11-12):** öneri 1'in yerine monoton alt sınır (ajanın plandaki öneri 3'e itirazı, kullanıcı onayladı); öneri 2 en yeni kaynak + ipucu; öneri 3'ün yerine hız geçmişi birleşik değerle; öneri 4 saklama ham kalır. Plan dışı: statusline satırı da birleşik değeri kullanır. Kabul kriteri "gidip gelmiyor, en geç bir API yoklamasında `/usage` ile eşitleniyor" olarak güncellendi.

**Yapılanlar:**
- **Doğrulama** (salt okunur, mevcut DB ve 9 Kasım yedeği): `weekly_all` çiftlerinde (±30 s, ±2 dk) `statusline − API` yalnızca 0 / −1, kabaca yarı yarıya; +1 yok; tam sayı olmayan değer yok. `session` bu hesapta yok, `weekly_scoped` statusline'da gelmiyor; ikisi doğrulanamadı. Sonuç `data-sources.md` S1 "Yuvarlama"da.
- **`limits/fusion.rs`** (saf): `interval` (statusline `[n, n+1)`, API `[m−0,5, m+0,5)` ve 0'da kırpılır, diğerleri tam), `Bound` (alt sınır, kaynağı, `since`; `fold` çelişkide yalnızca en yeni okumayla yeniden başlar, eşitlikte tam değer kazanır), `series`, `fuse_latest`.
- **Motor:** `Track` artık `bound`, `lead_percent`, `last_at` tutar. Aynı penceredeki her gerçek okuma sınırı katlar, önde olsun olmasın; pencere değişimi ya da tahminden sonraki ilk okuma track'i yeniden başlatır. Eşikler birleşik değerde (89 ↔ 90 sınırında tek olay). Hız geçmişi birleşik değer tutar ve yalnızca en yeni okumalar girer. Açılışta `restore_track` bir haftalık snapshot'tan pencereyi yeniden katlar; heartbeat kaynak başına son yazımdan sürer.
- **Görünüm:** `LimitView.combined_with` (+ `VisibleKey`), bindings. `SourceBadge` `combinedWith` alır; ipucu "Güncellendi … . Canlı ve API birlikte: Canlı aşağı yuvarlar, API en yakın tam sayıya" (Fluent `source-combined`, EN/TR). Panel alt bilgisi ve Ayrıntı limitleri bağlı; galeriye örnek rozet eklendi.
- **Grafikler:** `history::window_readings` artık birleşik seriyi döndürür (`WindowReading`), kovalara bölmeden önce hesaplanır. Ayrıntı odak grafiği ve mod'un `history --json`'u testere dişi çizmez; her nokta okumanın kaynağını taşır.
- **CLI (uygulama kapalı):** yeni `latest_snapshots_by_source` + `fuse_latest`.
- **Statusline satırı:** `line::lifted`, payload'ın tam sayı `n`'ini uygulamanın aynı penceredeki birleşik değeri `(n, n+1]` ise ona yükseltir; tahmin, başka pencere, uyumsuz değer yükseltmez. Kaynak `Statusline` kalır.
- **Testler:** çekirdek 500 (yeni: `fusion/tests.rs` 13, `engine/tests/fusion.rs` 11, `line/tests/v12s.rs` 3, history 1, store testine kaynak bazlı doğrulama), CLI 27, uygulama 101, frontend 134 (+1 `SourceBadge`).

**Sapmalar:**
- Kesişim + geçerlilik süresi yerine monoton alt sınır (ADR-060, kullanıcı kararı).
- Statusline satırı plan kapsamında değildi; kullanıcı onayıyla eklendi.
- `architecture.md` §9.2'deki "S1 ondalıklı olduğu için tercih edilir" yanlış çıktı; not düşüldü, düzeltmesi Adım 13'e bırakıldı.

**Açık konular:**
- **Kabul kriteri güncellendi (kullanıcı kararı, 2026-11-12):** "panel `/usage` ile aynı değeri gösteriyor" ancak bir API okumasından sonra garanti edilebiliyordu. Statusline bir tam sayıyı geçtiği anda panel bir sonraki yoklamaya kadar (≤ 15 dk) yaklaşık yarı yarıya ihtimalle 1 puan geride kalabilir; gidip gelme yok. Kriter "gidip gelmiyor, en geç bir API yoklamasında eşitleniyor" oldu.
- **Yan yana karşılaştırma kullanıcıyla yapılmadı:** kurulu sürüm 12s'yi içermiyor. Sürüm güncellemesinden sonra panel, statusline satırı ve `/usage` karşılaştırılmalı.
- **Claude Code mod'u** (`integrations/claude-code/`) limit değerini Claude Code'un `session.measure`'ından alır; aynı aşağı yuvarlamayı gösteriyor olabilir. Bu adımda dokunulmadı.
- `session` ve `weekly_scoped` için floor/round varsayımı doğrulanamadı; 5 saatlik penceresi olan bir hesapta bakılmalı.
- CLI çevrimdışı `status` yolunun DB'li testi yok (`fuse_latest` ve store sorgusu ayrı test edildi).

**Sonraki adıma notlar:**
- **Adım 13 (kalibrasyon):** S1 tam sayı ve aşağı yuvarlanmış; "S1 ondalıklı" varsayımı geçersiz. Δp için ham snapshot'lar yerine `fusion::series` ya da kaynak başına aralıklar kullanılabilir.
- Yeni bir limit kaynağı eklenirse `fusion::interval`'a yuvarlama biçimi yazılmalı; bilinmeyen kaynak tam değer sayılır (Codex şimdilik böyle).
- `LimitView` kuran her yer `combined_with` alanını doldurmalı (fake'ler `None`).
