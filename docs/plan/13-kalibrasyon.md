# Adım 13 — Kalibrasyon motoru

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.2 | 12 + **en az 1 hafta (ideal: 2) gerçek snapshot verisi** | 1-2 oturum | ⬜ |

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

- **Dış tüketim oranı yukarı iter.** claude.ai'da bir sohbet `Δp`'yi artırır ama yerel `ΔC`'yi artırmaz. Medyan bu aykırı değerlere dayanıklıdır, ancak dış tüketim aralıkların çoğunda varsa `k` yukarı kayar. Bu durumda `seven_day_breakdown` içindeki `claude_code` payı kullanılarak haftalık `k` düzeltilebilir: `k_düz = k̂ × claude_code_payı`. Bu düzeltmenin yararı backtest ile ölçülür.
- **Eşzamanlı oturumlar** kalibrasyonu etkilemez, çünkü `ΔC` tüm yerel oturumların toplamıdır. Etki yalnızca atıfta (Adım 14) önemlidir.
- **S1'in ondalıklı değerleri** aslında yuvarlanmış başlık değerleri olabilir. Gerçek çözünürlüğü veriden gözlemle (örneğin 0,1 adımlar mı?) ve gürültü modelini buna göre ayarla.
- **Zaman hizası:** Snapshot zamanı (S1: yanıt sonrası), `ΔC` olaylarının zamanından birkaç saniye sonra gelir. Aralık sınırlarında olayları `observed_at` zamanına göre değil, `ts ≤ observed_at` koşuluna göre ata. Belirsiz kenar olaylarının etkisini ölç.

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
