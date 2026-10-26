# Adım 14 — Atıf motoru ve pencere arşivi (backend)

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.2 | 13 | 1-2 oturum | ⬜ |

## Amaç

Projenin ayırt edici sorusuna cevap veren backend'i yazmak:

> **"Hangi oturum ve proje 5 saatlik pencerenin ve haftalık limitin ne kadarını tüketti?"**

Bunun için şunlar yapılır:
- Aralık bazlı atıf ve "Diğer" kovası.
- Kapanan ve tahmini (geçmiş) pencerelerin arşivi.
- Ana pencerenin ihtiyaç duyacağı tüm sorgu ve görünüm modelleri. Bu sorgular hızlı ve önceden hesaplanmış olmalı.

## Önce oku

- `docs/architecture.md` §6, §8, §9.3
- `docs/data-sources.md` S2 (`seven_day_breakdown`)
- Adım 13'ün devir notu ve backtest raporu

## Kapsam

- `attribution/`: aralık bazlı dağıtım, korunum garantisi, "Diğer" kovası, subagent'ları ana oturumda toplama, `attributions` tablosu, sürüm bazlı tembel yeniden hesaplama.
- `windows/`:
  - Kapanan pencereleri sonlandırma (tepe değer, limite takılma).
  - **Tahmini geçmiş pencereler:** snapshot kaydı başlamadan önceki ~30 günlük log verisinden, `estimated = 1` olarak.
  - Haftalık döngüler.
- Çapraz doğrulama: haftalık "Diğer" payı ile `seven_day_breakdown` karşılaştırması.
- Sorgu API'leri ve görünüm modelleri (specta):
  - `list_windows(range, kind, page)`, `get_window(id)`
  - `get_session_impact(session_id)`, `list_sessions(filter, sort, page)`, `get_session(id)`
  - `list_projects(filter, sort, page)`, `get_project(id)`
  - `get_timeseries(kind, range, resolution)` (LTTB ile seyreltilmiş % serisi + oturum işaretleri)
  - `get_heatmap(range, metric)` (haftanın günü × saat)
  - `get_overview(range)`
- Performans: 1 yıllık sentetik veri setinde her sorgu ≤ 50 ms.

## Kapsam dışı

- UI (Adım 15-17).

## Görevler

- [ ] **Atıf algoritması** (`architecture.md` §9.3, v0.2):
  - Her aralık için: `yerel = min(Δp, k·ΔC_yerel)`. Oturum payı `yerel × ΔC_s/ΔC_yerel`. `Diğer = Δp − yerel`.
  - Snapshot olmayan dönemlerde (örneğin uygulama kapalıyken ve spool da yokken) `k·ΔC` ile tahmin yapılır ve "tahmini" bayrağı set edilir.
  - Pencere toplamı: oturum payları + Diğer = pencerenin son (veya tepe) yüzdesi. **Korunum testi zorunlu** (tolerans 0,01 puan).
  - Subagent `ΔC` değeri `root_id` üzerinden ana oturuma eklenir. Ayrıntı görünümü için subagent kırılımı ayrıca saklanır (ya ayrı bir tablo ya da sorgu anında `usage_5m` üzerinden hesaplama; ölçüp karar ver).
  - Haftalık atıf aynı yöntemle `k_weekly` kullanılarak yapılır. Scoped limitlerde yalnızca ilgili model ailesi sayılır.
- [ ] **Tembel yeniden hesaplama:**
  - `attributions.calibration_version` güncel değilse ve pencere görüntüleniyorsa yeniden hesaplanır.
  - Açık (canlı) pencere 60 sn'de bir güncellenir.
  - Kapanan pencereler kalibrasyon sürümü değişene kadar sabit kalır.
- [ ] **Pencere sonlandırma:** `resets_at` geçince pencere kapatılır. `peak_percent` ve `hit_limit` kesinleşir, atıf son kez hesaplanır.
- [ ] **Tahmini geçmiş pencereler:**
  - Hizalama kuralı: Gerçek snapshot'lı pencereler loglarla karşılaştırılır. Örneğin: "Pencere başlangıcı, önceki pencere bittikten sonraki ilk olay mı, yoksa dakika veya saate yuvarlanmış hali mi?" Gözlenen `resets_at` değerleri (örneğin `22:29:59.94`) incelenerek kural belirlenir ve `architecture.md` §8 güncellenir.
  - Snapshot kaydı başlamadan önceki dönem için pencereler bu kuralla türetilir. Yüzdeleri `k·C` ile tahmin edilir; `hit_limit` belirsizdir (`NULL`).
- [ ] **Çapraz doğrulama:** Haftalık döngü için `Diğer / toplam` oranı, `seven_day_breakdown` içindeki `claude_code` dışı satırların toplamıyla karşılaştırılır. Fark > 10 puan ise `CalibrationSuspect` sinyali üretilir ve UI'da bilgi gösterilir.
- [ ] **Sorgular ve görünüm modelleri:**
  - Yalnızca **ilk görüntülemede bir kez** hesaplanması gerekenler için materyalize tablolar veya önbellek kullanılır. Örneğin saatlik ısı haritası `usage_5m` üzerinden hesaplanır; 1 yıllık veride bunun ≤ 50 ms sürdüğünü ölç, gerekirse saatlik bir rollup tablosu ekle (yeni migration).
  - **Sayfalama:** keyset pagination kullanılır (OFFSET kullanılmaz).
  - **Zaman serisi:** Ham snapshot'lar ve S1 kalp atışları LTTB ile en fazla 1.500 noktaya seyreltilir. Oturum başlangıç ve bitiş işaretleri eklenir.
  - **`get_overview(range)` içeriği:**
    - Güncel limitler
    - 7 günlük seri
    - Bu haftanın maliyeti ve token kırılımı (tür ve model)
    - En yüksek etkili 5 oturum ve 5 proje
    - "Diğer" payı
    - Kalibrasyon durumu
    - Plan değeri: API karşılığı maliyet ve plan ücreti. Plan ücreti `pricing/plans.toml` dosyasından gelir, resmî kaynaktan doğrulanır ve kullanıcı tarafından düzenlenebilir.
- [ ] **Sentetik büyük veri üreteci** (`examples/gen_synthetic.rs`): 1 yıl, 3.000 oturum, 150 bin olay/ay, snapshot'lar. Sorgu benchmark'ları bu veri üzerinde çalışır.
- [ ] **Testler:**
  - Korunum (özellik testi).
  - "Diğer" kovasının dış tüketimi yakalaması (sentetik).
  - Subagent toplamı.
  - Tahmini pencere türetme (fixture).
  - Sayfalama kararlılığı.
  - Sorgu performans eşikleri (criterion).

## Teknik notlar ve tuzaklar

- **Eşzamanlı oturumlar:** Aynı aralıkta birden fazla oturum varsa dağıtım `ΔC` oranına göre yapılır. Bu, "maliyet limit tüketimiyle orantılıdır" hipotezinin (H1) doğrudan sonucudur. Adım 13'te α çarpanları eklendiyse dağıtımda da **α ağırlıklı** maliyet kullanılmalı.
- **Kenar olayları:** Aralık sınırına denk gelen olaylar Adım 13'teki kuralla atanır. İki motor aynı aralık tanımını **paylaşmalı** (ortak modül).
- **Başlık ve ad:** Görünüm modellerinde oturum adı şu öncelikle seçilir: `session_name` (S1/S4) → Claude Code başlığı → proje adı + kısa uuid. İçerik **asla** kullanılmaz.
- **Saat dilimi:** Sorgular UTC'de tutulur; ısı haritası ve gün sınırları **yerel saat dilimine** göre hesaplanır (jiff `TimeZone::system()`). Yaz saati geçişlerinde saat kovaları test edilmeli. Türkiye'de yaz saati yok, ama diğer kullanıcılar için gerekli.

## Kabul kriterleri

- [ ] Korunum testleri geçer. Sentetik veride "Diğer" kovası dış tüketimi ±2 puan içinde yakalar.
- [ ] Gerçek veride en az 10 kapanmış pencerenin atıfları elle incelenip mantıklı bulunur (devir notunda özet).
- [ ] Tüm sorgular 1 yıllık sentetik veride ≤ 50 ms (p95).
- [ ] Tahmini pencere hizalama kuralı veriyle doğrulanmış ve dokümana işlenmiştir.

## Doğrulama

```powershell
cargo test -p tokenradar-core attribution windows queries
cargo run --release -p tokenradar-core --example gen_synthetic -- --out "$env:TEMP\tr-synth.db"
cargo bench -p tokenradar-core --bench queries
```

## Devir notu

_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
