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

## Ön keşif (2026-11-20, gerçek DB'nin `.backup` kopyası, salt okunur; sayılar yazılmadı)

- **5 saatlik pencere yok** (ADR-041). `limit_windows` içinde yalnızca haftalık pencereler var: `weekly_all` ve Fable kapsamlı limit, ikişer kapanmış, birer açık. "En az 10 kapanmış pencere" kabul kriteri bu veriyle karşılanamaz.
- Kurulu sürüm Adım 13'ten eski olduğu için `calibrations` tablosu boş. Atıf denemesinde kalibrasyon bellekte hesaplandı (`examples/attr_probe.rs`).
- Ham olaylar 2026-09-26'dan, snapshot'lar 2026-11-05'ten beri var. `seven_day_breakdown`'da `claude_code` dışı pay hafta başına ≈ %0–4.
- **Aralık başına formül "Diğer"i şişiriyor.** Kenar aralığı başına `yerel = min(Δp, k·ΔC)` yapıldığında, oranın gürültüsünün (yuvarlama, MAD ≈ %4) pozitif tarafı "Diğer"e akıyor. Gerçek ≈ %0–1 olan bir haftada "Diğer" aralık başına %4,3, 6 sa dilimde %1,7, 24 sa dilimde %0,9, pencere bütününde %0,0. Fable kapsamlı limitte (API, güven 0,29) aralık başına %13–19. Diğer haftalarda da 24 sa dilim `seven_day_breakdown`'a en yakın sonuç.

## Kullanıcıyla kararlaştırılanlar (2026-11-20)

- **Bölme:** Adım 14, **14a** ve **14b** olarak iki oturuma bölünür.
  - 14a: atıf, pencere sonlandırma, tahmini geçmiş pencereler, çapraz doğrulama, gerçek veri incelemesi.
  - 14b: sorgu API'leri, görünüm modelleri, sentetik üreteç, ≤ 50 ms benchmark'lar.
- **"Diğer" 24 saatlik dilimde hesaplanır.** Dilimler pencere başlangıcına hizalıdır: `Diğer = max(0, ΣΔp − k·ΣαΔC)`. Dilimin yerel payı oturumlara α ağırlıklı maliyetle bölünür. 5 saatlik pencerede dilim pencerenin kendisidir.
- **Kalibrasyon orta güvenin altındaysa "Diğer" hesaplanmaz.** Yüzde oturumlara maliyet oranıyla dağıtılır (v0.1 gibi); değerler `≈` ve silik gösterilir, nedeni belirtilir.
- **Tahmini geçmiş pencereler yalnızca haftalık.** Gözlenen haftalık reset 7'şer gün geriye taşınır; yüzde `k·C` ile `≈` olarak tahmin edilir (`estimated = 1`, limite takılma bilinmez). 5 saatlik türetme, 5 saatlik snapshot'ı olan bir hesapta kural doğrulanana kadar yapılmaz ve kod yolu da hazırlanmaz.

## Kullanıcıyla kararlaştırılanlar: 14a tasarımı (2026-11-20)

Ajanın önerdiği 1–7 numaralı maddeleri kullanıcı yazıldığı gibi onayladı. 6. maddenin uyarı kuralı, bir yan ajanın notu üzerine kullanıcı kararıyla tek yönlü yapıldı.

1. **Parçalar (segment).** Bir pencerenin kaynağı, o pencerede keskin kenarı çok olan kaynaktır (12t ile aynı kural). Keskin kenarlar 24 saatlik dilimlere göre gruplanır. Her parça, önceki dilimin son kenarından bu dilimin son kenarına kadar uzanır. Kenarı olmayan dilim bir sonrakine katılır.
   - **Baş:** pencere başlangıcı (%0) → ilk grubun son kenarı. Kayıt pencere ortasında başladıysa da `Δp` bilinir.
   - **Kuyruk:** son kenar → `min(now, resets_at)`, `Δp = peak_percent − son kenarın puanı`. Böylece toplam, gösterilen birleşik değere (ADR-060) eşit olur.
   - Parçada yerel maliyet yoksa `Δp` tümüyle "Diğer"e gider.
   - **Korunum:** Σ oturum payı + Diğer = `peak_percent`, tolerans 0,01 puan.
2. **Maliyet kaynağı `usage_5m`.** Atıf, ham olaylar yerine 5 dakikalık rollup'ı kullanır. Rollup hiç silinmez, ham olaylar 90 günde silinir. Kenar zamanı 5 dakikalık kovaya yuvarlanır: kova başı < kenar ise kova o parçaya sayılır. 24 saatlik dilimde bu hata önemsiz kalır. Subagent kırılımı için tablo tutulmaz, sorgu anında `usage_5m`'den hesaplanır (14b ölçer).
3. **Şema v3 (`0003_attribution.sql`).**
   - Hiç yazılmamış `attributions` tablosu yeniden kurulur: `est_percent` NULL olabilir (yüzdesiz, yalnızca maliyet), `ON DELETE CASCADE`.
   - Yeni `attribution_runs` tablosu (pencere başına): `calibration_version` (0 = kalibrasyonsuz), `method` (`calibrated` / `proportional` / `estimated` / `cost_only`), güven, toplam ve Diğer yüzdesi, `breakdown_other`, `suspect`, `closed`, `computed_at`.
   - v3 veritabanını v2 build'ler açamaz (`SchemaTooNew`).
4. **Güncelleme zamanlaması.** Boşta uyanma eklenmez.
   - Açık pencere en fazla 60 saniyede bir yeniden hesaplanır, yalnızca o arada yeni snapshot ya da kullanım geldiyse.
   - Kapanan pencere, kapandıktan sonraki ilk turda bir kez sonlandırılır (`closed = 1`).
   - Kalibrasyon sürümü değişince kapanmış pencereler yalnızca görüntülenince (`ensure_attribution(window)`) yeniden hesaplanır.
   - Hiç atıfı olmayan pencereler (yeni ve tahmini) kalibrasyon turunda (15 dk) arka planda doldurulur.
5. **Tahmini pencereler yalnızca `weekly_all`.** Kapsamlı limitin snapshot'tan önce var olup olmadığı bilinmiyor. `hit_limit` sütunu `NOT NULL` olarak kalır (tablo yeniden kurulmaz); görünüm modeli tahmini pencerede limite takılmayı "bilinmiyor" (`None`) verir. Kalibrasyon orta güvenin altındaysa tahmini pencerenin yüzdesi yoktur (`peak_percent` NULL, `cost_only`).
6. **Çapraz doğrulama yalnızca `weekly_all`.** Penceredeki son `seven_day_breakdown` (başlangıç ±5 dk eşleşir) ile karşılaştırılır. **Kural tek yönlüdür (kullanıcı kararı, 2026-11-20; plandaki iki yönlü `|fark| > 10` kuralının yerine).**
   - Kırılım kullanımı makineye göre değil ürüne göre ayırır: başka bir bilgisayarda ya da WSL'de çalışan Claude Code orada `claude_code` sayılır. Bu yüzden `Σ claude_code dışı`, "Diğer"in alt sınırıdır.
   - `Diğer/toplam < Σ claude_code dışı − 10 puan` ise `suspect = 1` yazılır ve `CalibrationSuspect` sinyali üretilir. Bu, makine sayısından bağımsız kesin bir çelişkidir.
   - "Diğer" 10 puandan fazla üstteyse uyarı çıkmaz. Görünüm modeli bunu "başka makine ya da kalibrasyon" bilgisi olarak taşır (`other_excess`).
   - UI'da gösterimi 14b/15'e kalır.
7. **Kabul kriteri (değişiklik):** "En az 10 kapanmış pencere" yerine mevcut bütün kapanmış pencereler (bugün 4) ve açık pencere elle incelenir. Her haftalık pencerenin "Diğer" payı `seven_day_breakdown` ile karşılaştırılır. Özet devir notuna yalnızca yüzde olarak yazılır. Doğrulama yeni pencereler kapandıkça 14b'de genişletilir.

## Kullanıcıyla kararlaştırılanlar: 14b (2026-11-20)

- **ADR-064'ün "Uygulama ayrıntıları" onaylandı.** Kuyruk son parçaya katılır. Baş parçasından 0,5 puan düşülür; bu ofset deneysel bir sabit olarak kalır ve `attr_inspect --trace` ile izlenir. ADR-060 ile çelişkisi birkaç hafta veri birikince ayrı bir adımda çözülür. Turda en fazla 64 pencere hesaplanır. `CalibrationSuspect` olayı kalır ve bildirim üretmez.
- **Eskimiş atıflar arka planda yakalanır.** Kalibrasyon turu (15 dk), kalibrasyon sürümü eskimiş kapanmış pencereleri de yeniden hesaplar: en yeniden başlar, turda en fazla 64 pencere. Liste sorguları saklanan değeri okur ve satırda `stale` bayrağı taşır. Bu, 14a'nın "yalnızca görüntülenince" kuralının yerini alır; `ensure` pencere detayında kalır.
- **"Bu hafta", açık `weekly_all` penceresidir.** "Geçen hafta" bir önceki penceredir. Haftalık penceresi olmayan hesapta son 7 gün kullanılır.
- **Plan ücreti gömülü tablodan gelir, kullanıcı değeri önce gelir.** `pricing/plans.toml`, resmî sayfadan doğrulanmış aylık ücretleri `subscriptionType` ve `rateLimitTier` ile eşler. Config'teki kullanıcı değeri tablonun önüne geçer. Tabloda olmayan planda ücret yoktur.
- **Bütün sorgular 14b'de Tauri komutu olarak kaydedilir** ve bindings yenilenir (planın "kullanan adımda" önerisi yerine kullanıcı kararı).

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

### 14a (2026-11-20)

**Yapılanlar**
- `attribution` modülü (`crates/tokenradar-core/src/attribution/`):
  - Saf algoritma: `segments`, `attribute`, `cross_check` ve dört yöntem (kalibre, orantılı, tahmini, yalnızca maliyet).
  - `refresh.rs`: `compute`, `refresh` (en yeni pencereden başlar, turda en fazla 64 pencere), `ensure`, `estimated_weekly_windows`, `fill_estimated`, `window_edges`.
- Şema v3 (`0003_attribution.sql`): `attributions` yeniden kuruldu, yeni tablo `attribution_runs`. Store okuma ve yazma işlevleri `store/attribution.rs`'te, `ReadOnlyStore`'a da eklendi.
- Runtime:
  - `limits_loop` kalibrasyon turunda tahmini pencereleri ekler ve atıfları günceller.
  - Yeni veri gelince en fazla 60 sn'de bir yeniden günceller; boşta uyanmaz.
  - Kalan pencere varsa 60 sn sonra devam eder.
  - `CoreEvent::CalibrationSuspect { account, window_id }` eklendi, bindings yenilendi.
- `Calibration::confidence_for_families` eklendi, `scope_family` artık `pub(crate)`.
- Örnekler: `attr_inspect` (gerçek veri incelemesi, `--names` ve `--trace`) ve `attr_probe` (toplama düzeyi karşılaştırması).
- 16 yeni test:
  - Korunum özellik testi.
  - Dış kullanım simülatörü (±2 puan).
  - Ağırlıklı bölüşüm, güven ve kapsam filtresi.
  - Çapraz doğrulamanın yönü.
  - Store yaşam döngüsü (doldurma, sonlandırma, 60 sn kapısı, `ensure` ile sürüm değişimi).
  - Subagent'ın ana oturuma sayılması.
- Belgeler: ADR-064, `architecture.md` §6 (şema v3), §8 (tahmini pencereler) ve §9.3, `docs/perf/2026-11-20-attribution.md`.

**Gerçek veri incelemesi** (kopya, yalnızca yüzde):
- Kapanmış 4 ve açık 2 pencerenin hepsinde korunum tam (< 10⁻⁴ puan).
- `weekly_all` "Diğer/toplam" %6,1 / %1,3 / %2,5, kırılım %1,0 / %1,0 / %0,0. İşaret yok.
- Fable kapsamlı limitte güven 0,28, bu yüzden orantılı dağıtım ve `≈` geçerli.
- 6 tahmini hafta eklendi.

**Sapmalar** (ADR-064):
- Aralık başına formül yerine 24 sa dilim.
- Çapraz doğrulama tek yönlü (`other_excess` yalnızca bilgi).
- Tahmini pencereler yalnızca `weekly_all`.
- Düşük güvende "Diğer" yok.
- Doğrulama listesindeki `queries` testleri, `gen_synthetic` ve bench 14b'ye kaldı.

**Kullanıcı onayı bekleyenler** (ADR-064 "Uygulama ayrıntıları"):
- Kuyruğun son parçaya katılması.
- Baş parçasından 0,5 puanlık yuvarlama ofseti.
- Turda 64 pencere sınırı.
- `CalibrationSuspect` olayı.

**Açık konular**
- **Yuvarlama ve ADR-060:** İlk kenarlar statusline'da da birikimli ≈ n − 0,5'te geliyor, yani en yakına yuvarlama gibi davranıyor. ADR-060 "statusline aşağı yuvarlar" diyor. İki gözlem ancak pencere başında ≈ 0,5 puanlık bir ofsetle uzlaşıyor. Füzyon kuralının yeniden incelenmesi ayrı bir adım olmalı.
- Yalnızca 2 kapanmış haftalık pencere var. Birkaç hafta sonra `attr_inspect` yeniden çalıştırılmalı; "Diğer" ile kırılım farkı ve ofset izlenmeli.
- `α_sonnet` hâlâ öğrenilmedi. Sonnet ağırlıklı bir haftada güven düşer ve dağıtım orantılıya iner.
- Kalibrasyon sürümü değişince kapanmış pencereler yalnızca `ensure` ile yeniden hesaplanır. 14b'deki liste sorguları bunu hesaba katmalı: listede eski sürüm görünebilir ya da liste de `ensure` çağırmalı.

**14b'ye notlar**
- Sorgu API'leri `attribution::ensure` üzerinden okumalı.
- Görünüm modelleri:
  - `method != calibrated` ise `≈` ve silik gösterim.
  - Tahmini pencerede `hit_limit` "bilinmiyor".
  - `suspect` için bir ipucu.
  - `other_excess` için tooltip bilgisi.
- `CalibrationSuspect` olayı `core_bridge.rs`'te yalnızca "other" olarak loglanıyor, UI'da gösterimi yok.
- Sentetik üretici atıf tablolarını doldurmak için `fill_estimated` ve `refresh`'i doğrudan çağırabilir.

### 14b (2026-11-20)

**Yapılanlar**
- `queries` modülü (`crates/tokenradar-core/src/queries/`), ADR-065:
  - `list_windows`, `get_window`, `list_sessions`, `get_session`, `get_session_impact`, `list_projects`, `get_project`, `get_timeseries`, `get_heatmap`, `get_overview`. Görünüm modelleri specta tipli.
  - Keyset sayfalama (imleç `"{anahtar}:{id}"`, OFFSET yok).
  - `Basis` her yüzdede yöntem, güven, `≈`, silik ve `stale` taşır. Tahmini pencerede `hit_limit` "bilinmiyor".
  - `lttb.rs` (en fazla 1.500 nokta), `zone.rs` (`LocalZone`: Windows'ta sistem saat dilimi, yaz saati ve yarım saatlik dilim testli), `plans.rs` + `pricing/plans.toml`.
- Atıf: `refresh(…, catch_up)`. Kalibrasyon turu eskimiş kapanmış pencereleri de yakalar (kullanıcı kararı).
- Config: `[[accounts]] plan_fee_usd` (yalnızca pozitif sayı kabul edilir, tablonun önüne geçer).
- Uygulama: `history.rs` sorgu bağlamını çalışan çekirdekten kurar. On komut `ipc.rs`'e kaydedildi, bindings yenilendi.
- Sentetik üretici: `examples/gen_synthetic.rs` + `examples/synthetic/mod.rs` (deterministik, atıflar dahil).
- Benchmark: `benches/queries.rs`. 15 durum; p95 geçişi 50 ms'yi aşarsa başarısız olur, ardından criterion grubu çalışır.
- 13 yeni sorgu testi (sayfalama kararlılığı, filtreler, "Diğer" payı, arka planda yakalama ve `stale`, proje ağacı, ısı haritası ve yaz saati, işaretler, Genel Bakış ve 7 gün yedeği, plan ücreti, `Basis`, imleç) ve bir config testi.
- Belgeler: ADR-064 kabul edildi, ADR-065 (Önerildi), `architecture.md` §5 ve §9.3, `docs/perf/2026-11-20-queries.md`.

**Ölçüm** (`docs/perf/2026-11-20-queries.md`):
- Sentetik yılda her sorgunun p95'i ≤ 39 ms. En yavaşlar `list_sessions` ve `list_projects` (tüm yıl, 34–39 ms).
- Gerçek verinin kopyasında her sorgu ≤ 18,4 ms.

**Sapmalar ve notlar**
- **Doğrulama komutu geçersiz:** `cargo test -p tokenradar-core attribution windows queries` hata veriyor (cargo tek filtre alır). Doğrusu `cargo test -p tokenradar-core -- attribution windows queries` (41 test geçti).
- Plandaki jiff `TimeZone::system()` yerine `LocalZone` kullanıldı (Windows'ta gömülü tzdb gerekmesin diye). macOS ve Linux'ta Adım 21'e kadar saat kovaları UTC.
- Subagent'ın kendi `project_id`'si yok. Proje maliyeti ana oturumun projesi üzerinden toplanır; ilk sürüm subagent maliyetini kaçırıyordu, test bunu yakalıyor.
- `usage_5m`'de geniş aralıkta SQLite zaman indeksini seçip satır başına PK'ye bakıyordu (oturum listesi 2,1 s). `+u.bucket_ts` kuralı ve iki aşamalı toplama ADR-065'te.
- Bench, `TR_SYNTH_DB` yolunda dosya yoksa sentetik veritabanını orada üretir. Gerçek veriyle ölçerken yolun doğru yazıldığından emin olun.
- Oturum işaretleri atıflardan gelir; atıfı henüz yazılmamış pencerede işaret yoktur.

**Kullanıcı onayı bekleyenler** (ADR-065, kullanıcı kararı olmayan maddeler):
- `LocalZone`, imleç biçimi, `Basis` kuralları, ısı haritası yüzde metriği, 200 işaret sınırı, indeks kuralı, `plan_fee_usd` doğrulaması.

**Kapsam dışı / sonraki adımlar**
- UI (Adım 15–17): `CalibrationSuspect` ve `other_excess` gösterimi, `stale` ve `≈` görselleri.
- Oturum × gün rollup tablosu: bugün gerekmiyor. `usage_5m` büyüyüp tüm yıl sorguları bütçeye yaklaşırsa yeni migration ve ADR ile.
- Yalnızca 2 kapanmış haftalık pencere var. Birkaç hafta sonra `attr_inspect --trace` ile "Diğer" ve 0,5 puanlık ofset yeniden incelenmeli (14a açık konusu). "En az 10 kapanmış pencere" kabul kriteri hâlâ veri bekliyor.
- macOS ve Linux yerel saat dilimi (Adım 21).
