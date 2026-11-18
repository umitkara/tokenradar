# Adım 13 — Kalibrasyon motoru

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.2 | 12 + **en az 1 hafta (ideal: 2) gerçek snapshot verisi** | 1-2 oturum | ⬜ |

> **Başlangıç: 18 Kasım 2026'dan sonra, tek adım olarak** (kullanıcı kararı, 2026-11-11). İlk tam haftalık pencere o gün kapanır. Başlamadan önce aşağıdaki "Ön keşif" bölümünü oku; oradaki sapmalar için ADR gerekir.

## Amaç

Limit yüzdesi ile API karşılığı maliyet arasındaki katsayıyı (`k`, %/USD) gerçek veriden **öğrenmek**. Bunun için bir backtest düzeneği kurulur ve tahmin hatası ölçülür. Sapma (drift) algılanır ve güven skoru üretilir.

Kalibrasyon şu özelliklerin temelidir:
- Atıf (Adım 14)
- Daha isabetli projeksiyonlar
- Snapshot olmayan geçmiş pencereler için yüzde tahmini
- "Diğer" (claude.ai/Desktop) tüketiminin ayrıştırılması

## Önce oku

- `docs/architecture.md` §9 (**tamamı**)
- `docs/data-sources.md` S1, S2 (kuantizasyon, `seven_day_breakdown`)
- `docs/decisions.md`: ADR-008
- Adım 06'nın devir notu (snapshot kayıt sıklığı, kalp atışı)

## Kapsam

- `calibration/intervals.rs`: snapshot çiftlerinden gözlem aralıkları ve her aralığın yerel `ΔC` değeri.
- `calibration/estimator.rs`: zaman sönümlü ağırlıklı medyan, güven skoru, CUSUM ile sapma algılama.
- Model ailesi çarpanları (`α_f`). **Yalnızca** backtest, sistematik bir sapma gösterirse eklenir.
- `calibrations` tablosu için depo API'si; sürümleme.
- Backtest düzeneği: walk-forward değerlendirme ve rapor.
- Sentetik veri üreteci: bilinen `k` + gürültü + dış tüketim + kuantizasyon.
- Entegrasyon: `LimitEngine` projeksiyonlarında `k0` yerine kalibre edilmiş `k` kullanılır (güven eşiğini geçerse).
- `CoreEvent::CalibrationUpdated` ve `get_calibration_status` komutu.

## Kapsam dışı

- Atıf hesapları ve UI (Adım 14-15). Bu adımda yalnızca katsayı ve kalitesi üretilir.

## Görevler

- [x] **Veri keşfi (önce):**
  - Kayıtlı snapshot sayısı, kaynak dağılımı (S1/S2) ve aralık uzunluklarının histogramı.
  - Kaç pencerede hem S1 hem S2 verisi var?
  - Bulguları devir notuna yaz. Veri yetersizse (< 30 kullanılabilir aralık) adımı **duraklat** ve kullanıcıya bildir.
- [x] **Aralık oluşturma:**
  - Aynı `(account, kind, scope, resets_at)` içindeki ardışık snapshot'lardan aralıklar oluşturulur.
  - S1 noktaları önceliklidir. Yalnızca S2 noktaları varsa aralıklar `Δp ≥ 3` olana kadar birleştirilir.
  - `ΔC`, aralık içindeki yerel olayların maliyet toplamıdır. Scoped limitlerde yalnızca ilgili model ailesinin olayları sayılır.
  - Atılan aralıklar:
    - `ΔC < gürültü tabanı` (başlangıç önerisi: medyan `ΔC` değerinin %5'i, veriden ayarlanır)
    - `Δp < 0`
    - İçinde reset bulunan aralıklar
- [x] **Tahminci:**
  - `r_i = Δp_i / ΔC_i` oranları hesaplanır.
  - Ağırlık: `w_i = ΔC_i × exp(−yaş_i / τ)`, burada `τ` = 3 gün yarı ömrünün karşılığıdır.
  - `k̂` = ağırlıklı medyan.
  - Dağılım ölçüsü olarak ağırlıklı MAD kullanılır.
  - Ayrı katsayılar: `k_session`, `k_weekly` ve her `weekly_scoped` için bir `k`.
- [x] **Güven skoru:** `n_obs`, `MAD/k̂` ve tazelik ile [0,1] aralığında hesaplanır. Formül belgelenir ve birim testi yazılır. Eşikler: < 0,4 düşük, 0,4-0,7 orta, > 0,7 yüksek.
- [x] **Sapma algılama:** Normalize edilmiş artıklar (`r_i/k̂ − 1`) üzerinde CUSUM uygulanır. Eşik aşılırsa `CalibrationDrift` olayı yayınlanır, `τ` geçici olarak kısaltılır (hızlı yeniden öğrenme) ve UI'da "Anthropic limit hesaplamasını değiştirmiş olabilir" bilgisi gösterilir.
- [x] **Model ailesi çarpanları (koşullu):**
  - Backtest artıkları model karışımına göre ayrıştırılır.
  - Opus ağırlıklı ve Sonnet ağırlıklı aralıklar arasında sistematik bir fark (> %15) varsa `Δp ≈ k·Σ α_f ΔC_f` modeli kısıtlı en küçük kareler veya ağırlıklı medyan regresyonuyla çözülür; `α` normalize edilir.
  - Fark yoksa çarpan **eklenmez**: H1 doğrulanmış sayılır ve sonuç ADR olarak kaydedilir.
- [x] **Kalıcılık:**
  - `calibrations` tablosuna yalnızca anlamlı bir değişim olduğunda (≥ %2) veya günde bir kez yeni sürüm yazılır.
  - `version` monoton artar.
  - Yeniden hesaplama yeni snapshot'lar geldiğinde 15 dk debounce ile yapılır; maliyeti düşüktür.
- [x] **Backtest düzeneği** (`examples/backtest.rs` veya `tokenradar-app --backtest`):
  - Walk-forward değerlendirme: her pencere için yalnızca **önceki** verilerle öğrenilen `k` kullanılır. Pencere sonundaki %, `k × C_pencere + (varsa) Diğer` ile tahmin edilir.
  - Metrikler: pencere başına MAE (puan), sapma (bias), model karışımına göre MAE, güven skoruna göre MAE (kalibrasyon eğrisi).
  - Çıktı `docs/perf/<tarih>-calibration-backtest.md` dosyasına yazılır. Gerçek değerler yerine yüzdeler ve hata metrikleri yazılır; mutlak $ değerleri raporda yer alabilir ama kişisel proje adları yer almaz.
- [x] **Sentetik testler:**
  - Bilinen `k`, Gauss gürültüsü, %10 aralıkta pozitif dış tüketim ve tam sayı kuantizasyonu ile üretilen veride tahminci `k`'yı %5 içinde bulmalı.
  - Ani `k` değişimi senaryosunda CUSUM değişimi 24 saat içinde yakalamalı.
- [x] **Entegrasyon:** `rate.rs` içindeki yedek projeksiyon yöntemi, güven ≥ orta olduğunda kalibre `k` kullanır; aksi halde `k0`. `PanelSnapshot` içine `calibration: { confidence, n_obs }` eklenir.

## Teknik notlar ve tuzaklar

- **5 saatlik pencere her hesapta yok (ADR-041).** Doğrulanmış Ultra hesaplarında (kullanıcının hesabı dahil) `session` snapshot'ı gelmiyor. Bu hesaplarda `k_session` üretilmez; kalibrasyon `k_weekly` ve scoped `k`'lara dayanır. Veri keşfi bu dağılımı raporlamalı. "≥ 30 kullanılabilir aralık" eşiğine yalnızca haftalık aralıklarla ulaşmak daha uzun sürebilir.
- **Dış tüketim oranı yukarı iter.** claude.ai'da bir sohbet `Δp`'yi artırır ama yerel `ΔC`'yi artırmaz. Medyan bu aykırı değerlere dayanıklıdır, ancak dış tüketim aralıkların çoğunda varsa `k` yukarı kayar. Bu durumda `seven_day_breakdown` içindeki `claude_code` payı kullanılarak haftalık `k` düzeltilebilir: `k_düz = k̂ × claude_code_payı`. Bu düzeltmenin yararı backtest ile ölçülür.
- **Eşzamanlı oturumlar** kalibrasyonu etkilemez, çünkü `ΔC` tüm yerel oturumların toplamıdır. Etki yalnızca atıfta (Adım 14) önemlidir.
- **S1'in ondalıklı değerleri** aslında yuvarlanmış başlık değerleri olabilir. Gerçek çözünürlüğü veriden gözlemle (örneğin 0,1 adımlar mı?) ve gürültü modelini buna göre ayarla.
- **Zaman hizası:** Snapshot zamanı (S1: yanıt sonrası), `ΔC` olaylarının zamanından birkaç saniye sonra gelir. Aralık sınırlarında olayları `observed_at` zamanına göre değil, `ts ≤ observed_at` koşuluna göre ata. Belirsiz kenar olaylarının etkisini ölç.

## Ön keşif (2026-11-11)

Adım başlamadan, gerçek veritabanı salt okunur incelendi. Kişisel kullanım sayıları buraya yazılmadı.

**Veri durumu**
- Snapshot kaydı 6 Kasım 2026'da başladı. Hesapta yalnızca `weekly_all` ve `weekly_scoped` (`model:Fable`) var; `session` yok (ADR-041).
- Haftalık pencere çarşamba 11:00'de (yerel) sıfırlanıyor. 11 Kasım'da henüz tamamlanmış bir pencere yoktu.
- `seven_day_breakdown` tüketimin neredeyse tamamını `claude_code` olarak gösteriyor; dış tüketim oranı bozmuyor.

**Planla uyuşmayan noktalar (karar ve ADR gerekir)**
- **S1 ondalıklı değil.** `seven_day` yüzdesi statusline'da da tam sayı. Doğru gözlem birimi, yüzdenin bir puan arttığı ilk anlar arasındaki aralıktır (`Δp = 1`). Rastgele snapshot çiftleri ±1 puan kuantizasyon hatası taşır.
- **Pencere düzeyinde walk-forward bu hesapta işlemez.** Haftada tek pencere var. Backtest pencere içi ileri tahmine çevrilmeli (ör. önceki aralıklardan öğrenilen `k` ile sonraki +1 puanın maliyeti).
- **Kabul kriteri uygulanamıyor.** "MAE ≤ 3 puan (5 saatlik pencere)" ölçülemez; haftalık pencere için yeni bir ölçüt belirlenmeli.
- **API `resets_at` değeri ±1 s oynuyor.** Birebir eşitlikle yüzlerce "pencere" çıkar. Aralık oluşturucu, `limit_windows` ile aynı toleransı (`WINDOW_MATCH_TOLERANCE_MS`) kullanmalı.
- **API, statusline'dan yaklaşık 1 puan yüksek okuyor** (yuvarlama farkı). İki kaynak aynı aralıkta karıştırılmamalı.
- **Boşluktan sonraki ilk okuma bayat.** Uzun bir aranın ardından bir puanın maliyeti önce yüksek, hemen sonraki puanınki düşük görünüyor; ikisi birlikte normal. Komşu aralıklar birleştirilmeli.
- **İlk aralık kırpık.** Kaydın başladığı andaki yüzde bir eşik geçişi değildir; o aralık atılmalı.

**H1 için ilk işaret**
- Opus ve Sonnet ağırlıklı aralıklarda `Δp/ΔC` dar bir bantta kalıyor (çeyrekler medyandan en fazla %7 uzakta).
- Fable ağırlıklı az sayıdaki aralıkta oran yaklaşık 2 kat. Tek `k` büyük olasılıkla yetmeyecek; aile çarpanı (`α_fable`) ya da cache okuma ağırlığı sınanmalı. 11 Kasım'da bunu ayırt edecek kadar Fable aralığı yoktu.

**Maliyet sütunu**
- Saklı maliyetler artık tablo değişince yeniden hesaplanıyor (ADR-057). 9-11 Kasım arasındaki `claude-fable-5-2` olayları Fable 5.1 fiyatıyla kaydedilmişti; yeni derleme ilk açılışta düzeltir.
- `IndexState.unpriced_models` listesindeki modellerin ağırlıklı olduğu aralıklar kalibrasyona alınmamalı.

## Kullanıcıyla kararlaştırılanlar (2026-11-18)

Adım başında veri keşfi yapıldı (salt okunur; sayılar git'e yazılmadı). Bulgular devir notunda. Kullanıcı şunları seçti:

- **Gözlem birimi: kenar aralıkları.**
  - Aralık, pencere içindeki birikimli maksimumda yüzdenin arttığı iki ardışık "kenar" arasıdır. Çoğunlukla `Δp = 1` olur.
  - Kenarın zamanı, kenardan önceki ve sonraki okumanın orta noktasıdır.
  - `weekly_all` statusline serisinden, kapsamlı limit API serisinden öğrenilir. Bir aralıkta iki kaynak karışmaz.
  - Plandaki "rastgele snapshot çifti + `Δp ≥ 3` birleştirme" kuralının yerini alır.
- **Aile çarpanları veriden öğrenilir.** `α_f` için Opus = 1 kabul edilir. Yeterli verisi olmayan aile 1'de kalır.
- **Kabul ölçütü:** pencere içi walk-forward. Her an yalnızca önceki veriden öğrenilen parametrelerle (`k` ve `α`) 24 saat sonraki yüzde tahmin edilir. Ölçüt: MAE ≤ 1,5 puan ve |sapma| ≤ 0,5. "5 saatlik pencerede MAE ≤ 3" kriterinin yerini alır.
- **Kalibre öngörünün güveni `Low` kalır.** Bildirim ve mod davranışı değişmez; yalnızca hız sayısı iyileşir.
- **Kapsamlı limite maliyet tabanlı yedek öngörü eklenmez.** Bu konu Adım 12t'ye bırakılır.
- **Görünüm:** Tray Ayrıntı'da öngörü ipucu güveni ve aralık sayısını gösterir. Sapma olursa Ayrıntı'da tek satırlık bilgi çıkar. `tokenradar doctor` kalibrasyonu raporlar. Ana kart değişmez.

## Kabul kriterleri

- [x] Sentetik testlerde `k` %5 içinde bulunur ve sapma 24 saat içinde algılanır.
- [x] ~~Gerçek veride walk-forward backtest MAE ≤ 3 puan (5 saatlik pencere).~~ Kullanıcı kararıyla yerine: 24 sa ufukta pencere içi walk-forward MAE ≤ 1,5 ve |sapma| ≤ 0,5. `weekly_all` 0,75 / −0,46; Fable kapsamlı 0,75 / −0,02.
- [x] H1 (tek k) hipotezinin sonucu ADR olarak kaydedilir (ADR-061: reddedildi).
- [x] Panel projeksiyonları kalibre `k` ile çalışır ve güven göstergesi görünür (yedek öngörüde, kullanım anı güveni ≥ orta iken; Ayrıntı ipucu).

## Doğrulama

```powershell
cargo test -p tokenradar-core calibration
cargo run --release -p tokenradar-core --example backtest -- --db "$env:LOCALAPPDATA\TokenRadar\tokenradar.db"
```

## Devir notu

**Tarih:** 2026-11-19 · **Karar:** ADR-061 (Kabul edildi, 2026-11-19) · **Ölçüm:** `docs/perf/2026-11-19-calibration-backtest.md`

**Kullanıcıyla kararlaştırılanlar:** yukarıdaki "Kullanıcıyla kararlaştırılanlar (2026-11-18)" bölümü. Ek olarak adım sırası: 12t'den önce 13.

**Veri keşfi (2026-11-18, salt okunur; sayılar yazılmadı):**
- Veri yeterli: `weekly_all` statusline serisinde ve Fable kapsamlı API serisinde kullanılabilir aralık eşiği (≥ 30) aşılıyor. `session` yok (ADR-041).
- Statusline'daki geri düşüşlerin tamamı bayat payload taşıyan başka oturumlardan; birikimli maksimum şart.
- Kenar zamanı belirsizliği medyanda statusline ≈ 2,5 dk, API ≈ 15 dk.
- H1 reddedildi: Fable ağırlıklı aralıklarda oran ≈ 2 kat; `α_fable` ≈ 2,2 dağılımı ≈ %12'den ≈ %5'e indiriyor. Cache okuma ağırlığını değiştirmek farkı açıklamıyor.
- Fable kapsamlı limitin `k`'sı, `weekly_all` içindeki Fable `k`'sının ≈ 1,9 katı (kapsamlı payın ≈ %50 olmasıyla tutarlı).
- `seven_day_breakdown` ≈ %99 `claude_code`; `claude_code` payı düzeltmesi gereksiz bulundu.

**Yapılanlar:**
- **`calibration/intervals.rs`:** okumalardan kenarlar ve kenar aralıkları (birikimli maksimum, orta nokta zamanı, 45 dk belirsizlik sınırı, pencere eşleme toleransı).
- **`calibration/estimator.rs`:** ağırlıklı medyan `k` (ham maliyet × sönüm), log ızgarada `α` araması, güven formülü, CUSUM sapma ve sapma sonrası hızlı yeniden öğrenme. `Estimate.fitted` hangi ailelerin öğrenildiğini taşır.
- **`calibration/mod.rs`:** `CalibrationData` (Store ve ReadOnlyStore), seri seçimi (statusline ≥ 10 aralık, yoksa API), aile maliyetleri, fiyatsız pay filtresi, `calibrate`, `should_store`, `Calibration::confidence_for` (kullanım anı güveni), `CalibrationView`, `CalibrationStatus`.
- **Store:** `store/calibrations.rs` (`insert` sürümlü, `latest` limit başına son satır, `model_costs_between`). Şema değişmedi; ek alanlar `alpha_json` içinde.
- **Motor:** `RateMethod::Calibrated`, `rate::Fallback` (Prior / Calibrated), `calibrated_rate`. `LimitEngine::set_calibrations`. Yedek öngörü yalnızca `session` / `weekly_all` için ve kullanım anı güveni ≥ orta iken kalibre `k`'yı kullanır; güven `Low` kalır. `LimitView.calibration` ve `VisibleKey`'e düzey + sapma.
- **Çalışma zamanı:** açılışta saklı kalibrasyonlar yüklenir; ilk hesap 30 s sonra (`Timings::calibration_delay`), sonra 15 dakikada bir ve yalnızca yeni snapshot geldiyse. `CoreEvent::CalibrationUpdated`, `CoreStatus.calibrations`, Tauri `get_calibration_status`.
- **Doctor:** paketteki `database.calibrations` ve terminalde limit başına bir satır (`doctor-calibration`, EN/TR).
- **Ayrıntı:** öngörü ipucu "Öğrenilen oran: … güven, N aralıktan." (`detail-calibration`); sapma varsa tek satır (`detail-calibration-drift`, yönlü). Ana kart değişmedi.
- **Backtest:** `examples/backtest.rs` (`--db`, `--days`, `--trace`); 6 sa ve 24 sa walk-forward, model ve H1, karışım ve güven alt kümeleri, "kullanılan (≥ orta)" alt kümesi, bir `calibrate` çalışmasının süresi. Sayı, `k` ya da maliyet basmaz; tahmin sayısı sütunu rapora alınmadı.
- **Testler:** çekirdek 522 (kalibrasyon 20, store 2, motor 2, rate 1 yeni), CLI 28 (+1 doctor satırı), uygulama 101, frontend 136 (+2 Ayrıntı).

**Sapmalar:**
- Gözlem birimi, kabul ölçütü ve aile çarpanları kullanıcı kararıyla plandan farklı (ADR-061).
- **`α` ağırlık hatası bulundu ve düzeltildi:** ilk sürümde medyan ağırlığı `α`'ya bağlıydı; walk-forward'da `α_fable` 0,25'e çöküyordu.
- **Kullanım anı güveni plana ek:** öğrenilmemiş bir ailenin yoğun kullanımında öğrenilen güven yanıltıcı yüksekti (backtest'te hata −3 puana kadar).
- **CUSUM eşiği 5 → 6:** gerçek veride iki kısa yanlış alarm vardı, 6'da yok; sentetik algılama 5–12 sa.
- **Ayrı `CalibrationDrift` olayı yok;** sapma `CalibrationUpdated` içinde. UI metni "Anthropic hesaplamayı değiştirmiş olabilir" demez, yalnızca kullanımın limite eskisinden fazla/az sayıldığını söyler (nedeni bilinmiyor).
- Plandaki `PanelSnapshot.calibration` yerine limit başına `LimitView.calibration`.
- Olaylar kenarlara `observed_at` yerine kenarın orta nokta zamanıyla atanır (kenar belirsizliği ölçüldü, yukarıda).

**Açık konular:**
- **ADR-061 onaylandı (2026-11-19):** güven formülü, CUSUM parametreleri, saklama kuralları ve kullanım anı güveni kullanıcı onayıyla kabul edildi.
- **`weekly_all` sapması sınıra yakın (−0,46).** Kaynağı öğrenilmemiş aile dönemleri (güven düşük alt kümesinde −1,32); uygulamanın kullandığı alt kümede −0,35.
- **`α_sonnet` 1,41** az veriyle öğrenildi; yanlı bir dönem onu oynatabilir. İzlenmeli.
- Ayrıntı ipucu öğrenilen güveni gösterir, kullanım anı güvenini değil; ikisi öğrenilmemiş aile kullanılırken ayrışır.
- Gerçek uygulamada görsel kontrol (CDP ekran görüntüsü) yapılmadı; ipucu ve sapma satırı render testleriyle doğrulandı.
- Hesapta 5 saatlik pencere olmadığı için `session` kalibrasyonu gerçek veride sınanmadı.

**Sonraki adıma notlar:**
- **Adım 12t:** kalibre yedek yalnızca gözlenen eğim yokken devreye giriyor. 12t'nin adım-kenarı hızı bu boşluğu küçültür; ikisinin önceliği orada belirlenmeli. Kapsamlı limitin maliyet yedeği 12t'de: Fable kapsamlı kalibrasyonu (`k`, güven) hazır, `Calibration::family()` ile yalnızca ailenin maliyeti sayılır.
- **Adım 14 (atıf):** `calibrations.version` atıf satırlarına yazılmalı; `Calibration::weighted_cost` aile ağırlıklı maliyeti verir. Atıfta kullanılacak güven `confidence_for` ile hesaplanmalı.
- `LimitView` kuran her yer `calibration` alanını doldurmalı (sahte sahneler `None`).
- Backtest yeniden: `cargo run --release -p tokenradar-core --example backtest -- --db <kopya> [--trace]`. Gerçek DB'ye değil, `.backup` kopyasına karşı çalıştırın.
