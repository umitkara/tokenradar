# Adım 12s — Limit yüzdelerinin yuvarlama farkını füzyonda gidermek

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1.x | 12r | 1 oturum | ⬜ |

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

- [ ] Doğrulama sorgusu, sonuç `docs/data-sources.md`'de.
- [ ] Kullanıcıyla öneriler 1–4 kararlaştırılır.
- [ ] Core: kaynak başına son okuma, kesişim fonksiyonu; testler (eşit, −1, boş kesişim, eski API, tam sayı olmayan değer, pencere değişimi, reset tahmini ile etkileşim).
- [ ] Core: eşik olaylarında gidip gelme yok (89 ↔ 90 sınırında tek olay); test.
- [ ] Görünüm: kaynak etiketi ve ipucu, bindings, Fluent.
- [ ] Yeni ADR ve devir notu.

## Teknik notlar ve tuzaklar

- **Gerçek veri git'e girmez:** Doğrulama sonuçlarında kullanım yüzdesi, toplam ya da çift sayısı yer almaz.
- **Kesişim yalnızca aynı pencerede:** `resets_at` farklı iki okuma birleştirilmez. API `resets_at` milisaniye kesirleri taşıyor (`…800746`), statusline saniyeye yuvarlıyor. Mevcut `same_window` toleransı korunur.
- **Reset tahmini:** `resets_at` geçtiğinde motor `Estimate` 0 % üretir. Kesişim yalnızca gerçek okumalar arasında yapılır.
- **Heartbeat (ADR-028):** Saklama kuralı değişmez. Birleştirilmiş değer saklanmaz.

## Kabul kriterleri

- API ve statusline okumaları 1 puan farklıyken panel `/usage` ile aynı değeri gösteriyor ve API yoklamasından sonra gidip gelmiyor.
- Statusline yeni bir artış gösterdiğinde panel API'yi beklemeden yükseliyor.
- Hiçbir birleştirilmiş değer `≈` olmadan tahmin içermiyor. Her değerin kaynağı görünür.

## Doğrulama

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
- `pnpm -C app check`, `pnpm -C app lint`, `pnpm -C app test`
- Kurulu ya da portable build ile panel ve `/usage` yan yana karşılaştırılır (kullanıcıyla).

## Devir notu

_(Adım bitince doldurulur.)_
