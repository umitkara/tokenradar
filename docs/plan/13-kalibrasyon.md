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

- [ ] **Veri keşfi (önce):**
  - Kayıtlı snapshot sayısı, kaynak dağılımı (S1/S2) ve aralık uzunluklarının histogramı.
  - Kaç pencerede hem S1 hem S2 verisi var?
  - Bulguları devir notuna yaz. Veri yetersizse (< 30 kullanılabilir aralık) adımı **duraklat** ve kullanıcıya bildir.
- [ ] **Aralık oluşturma:**
  - Aynı `(account, kind, scope, resets_at)` içindeki ardışık snapshot'lardan aralıklar oluşturulur.
  - S1 noktaları önceliklidir. Yalnızca S2 noktaları varsa aralıklar `Δp ≥ 3` olana kadar birleştirilir.
  - `ΔC`, aralık içindeki yerel olayların maliyet toplamıdır. Scoped limitlerde yalnızca ilgili model ailesinin olayları sayılır.
  - Atılan aralıklar:
    - `ΔC < gürültü tabanı` (başlangıç önerisi: medyan `ΔC` değerinin %5'i, veriden ayarlanır)
    - `Δp < 0`
    - İçinde reset bulunan aralıklar
- [ ] **Tahminci:**
  - `r_i = Δp_i / ΔC_i` oranları hesaplanır.
  - Ağırlık: `w_i = ΔC_i × exp(−yaş_i / τ)`, burada `τ` = 3 gün yarı ömrünün karşılığıdır.
  - `k̂` = ağırlıklı medyan.
  - Dağılım ölçüsü olarak ağırlıklı MAD kullanılır.
  - Ayrı katsayılar: `k_session`, `k_weekly` ve her `weekly_scoped` için bir `k`.
- [ ] **Güven skoru:** `n_obs`, `MAD/k̂` ve tazelik ile [0,1] aralığında hesaplanır. Formül belgelenir ve birim testi yazılır. Eşikler: < 0,4 düşük, 0,4-0,7 orta, > 0,7 yüksek.
- [ ] **Sapma algılama:** Normalize edilmiş artıklar (`r_i/k̂ − 1`) üzerinde CUSUM uygulanır. Eşik aşılırsa `CalibrationDrift` olayı yayınlanır, `τ` geçici olarak kısaltılır (hızlı yeniden öğrenme) ve UI'da "Anthropic limit hesaplamasını değiştirmiş olabilir" bilgisi gösterilir.
- [ ] **Model ailesi çarpanları (koşullu):**
  - Backtest artıkları model karışımına göre ayrıştırılır.
  - Opus ağırlıklı ve Sonnet ağırlıklı aralıklar arasında sistematik bir fark (> %15) varsa `Δp ≈ k·Σ α_f ΔC_f` modeli kısıtlı en küçük kareler veya ağırlıklı medyan regresyonuyla çözülür; `α` normalize edilir.
  - Fark yoksa çarpan **eklenmez**: H1 doğrulanmış sayılır ve sonuç ADR olarak kaydedilir.
- [ ] **Kalıcılık:**
  - `calibrations` tablosuna yalnızca anlamlı bir değişim olduğunda (≥ %2) veya günde bir kez yeni sürüm yazılır.
  - `version` monoton artar.
  - Yeniden hesaplama yeni snapshot'lar geldiğinde 15 dk debounce ile yapılır; maliyeti düşüktür.
- [ ] **Backtest düzeneği** (`examples/backtest.rs` veya `tokenradar-app --backtest`):
  - Walk-forward değerlendirme: her pencere için yalnızca **önceki** verilerle öğrenilen `k` kullanılır. Pencere sonundaki %, `k × C_pencere + (varsa) Diğer` ile tahmin edilir.
  - Metrikler: pencere başına MAE (puan), sapma (bias), model karışımına göre MAE, güven skoruna göre MAE (kalibrasyon eğrisi).
  - Çıktı `docs/perf/<tarih>-calibration-backtest.md` dosyasına yazılır. Gerçek değerler yerine yüzdeler ve hata metrikleri yazılır; mutlak $ değerleri raporda yer alabilir ama kişisel proje adları yer almaz.
- [ ] **Sentetik testler:**
  - Bilinen `k`, Gauss gürültüsü, %10 aralıkta pozitif dış tüketim ve tam sayı kuantizasyonu ile üretilen veride tahminci `k`'yı %5 içinde bulmalı.
  - Ani `k` değişimi senaryosunda CUSUM değişimi 24 saat içinde yakalamalı.
- [ ] **Entegrasyon:** `rate.rs` içindeki yedek projeksiyon yöntemi, güven ≥ orta olduğunda kalibre `k` kullanır; aksi halde `k0`. `PanelSnapshot` içine `calibration: { confidence, n_obs }` eklenir.

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

## Kabul kriterleri

- [ ] Sentetik testlerde `k` %5 içinde bulunur ve sapma 24 saat içinde algılanır.
- [ ] Gerçek veride walk-forward backtest MAE ≤ 3 puan (5 saatlik pencere). Değilse nedenleri ve iyileştirme seçenekleri raporlanır ve kullanıcıyla görüşülür (ADR-008 başarı ölçütü).
- [ ] H1 (tek k) hipotezinin sonucu ADR olarak kaydedilir.
- [ ] Panel projeksiyonları kalibre `k` ile çalışır ve güven göstergesi görünür.

## Doğrulama

```powershell
cargo test -p tokenradar-core calibration
cargo run --release -p tokenradar-core --example backtest -- --db "$env:LOCALAPPDATA\TokenRadar\tokenradar.db"
```

## Devir notu

_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
