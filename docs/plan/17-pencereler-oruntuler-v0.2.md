# Adım 17 — Pencereler, Örüntüler, periyodik özet ve v0.2 sürümü

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.2 | 16 | 2 oturum | ⬜ |

> Gerekirse **17a (Pencereler + Örüntüler + dışa aktarma)** ve **17b (periyodik özet + v0.2 sürümü)** olarak bölün.

## Amaç

v0.2'yi tamamlamak:
- **Pencereler** ekranı: 5 saatlik pencere arşivi ve haftalık döngüler. Hangi pencere limite takıldı, kim doldurdu?
- **Örüntüler** ekranı: haftanın günü × saat ısı haritası, gün içi dağılım, saatlere göre limit riski, model karışımının zaman içindeki değişimi.
- Dışa aktarma (CSV/JSON).
- Periyodik özet bildirimi.
- v0.2.0 sürümü.

## Önce oku

- `docs/design-language.md` §2, §4, §8
- `dataviz` skill'i (ısı haritası renk ölçeği, sıralı palet)
- Adım 14'teki sorgular: `list_windows`, `get_window`, `get_heatmap`
- Adım 11'deki `Notifier` (özet kuralı buraya eklenecek)
- Adım 12'deki sürüm prosedürü

## 17a öncesi kararlar (2026-11-21)

Gerçek veride (salt okunur sayım) hiç 5 saatlik pencere yok (ADR-041): yalnızca `weekly_all` ve `weekly_scoped` (Fable) pencereleri var, kapanmış 2 ve açık 1. Sentetik üretici de 5 saatlik pencere üretmiyor.

**Kullanıcıyla kararlaştırılanlar:**
- **Bölme:** 17a Pencereler, Örüntüler ve dışa aktarma; 17b periyodik özet ve v0.2 sürümü (ayrı oturum).
- **Pencereler haftalık öncelikli:** Ana görünüm haftalık döngülerdir. Her hafta bir satırdır: 7 günlük şeritte gün başına puan, tepe değer, limite takılınan gün, ilk 3 proje ve kapsamlı limit sütunları. Liste ve detay çekmecesi her tür pencerede çalışır. 5 saatlik zaman şeridi de yazılır, ama yalnızca hesapta 5 saatlik pencere varsa görünür. Sentetik üretici bunu sınamak için 5 saatlik pencere üretir.
- **Limit riski yalnızca 5 saatlik pencerelerde:** Genel olarak yazılır ve sentetik veriyle sınanır. 5 saatlik penceresi olmayan hesapta açıklayıcı bir boş durum gösterilir; haftalık için bir karşılık uydurulmaz.

**Adım sonunda onaylananlar (ADR-070, 2026-11-21):** Devir notundaki sekiz öneri kullanıcı tarafından onaylandı.

## 17b öncesi kararlar (2026-11-21)

**Kullanıcıyla kararlaştırılanlar:**
- **Özet haftası:** Son yenilenen limit haftası özetlenir. Varsayılan gönderim haftalık reset'ten 1 saat sonradır; ayarlardan gün ve saat seçilebilir, o durumda da son yenilenen limit haftası özetlenir.
- **Kaçan özet:** Uygulama gönderim anında kapalıysa, açıldığında bir kez gönderilir; vadesinin üzerinden 48 saat geçtiyse atlanır. Sessiz saatlerde bekletilir.
- **İçerik:** 5 saatlik pencere varsa "N pencere limite takıldı"; yoksa satır haftalık limitin durumunu söyler ("takılmadı" / "Çar 14:20'de takıldı"), kapsamlı limitler (Fable) aynı satırdadır. Diğer satırlar: tepe %, API karşılığı maliyet, en çok etkileyen proje, önceki haftaya göre fark.
- **Sürüm kapsamı:** Bu oturumda hazırlık ve yerel kurulum: sürüm 0.2.0'a yükselir, CHANGELOG, README ve performans raporu yazılır, yerel build kurulur. Tag, push ve release komutları kullanıcıya verilir; ne zaman çalıştırılacağına kullanıcı karar verir.

## Kapsam

- **Pencereler ekranı:**
  - Gün bazlı zaman şeridi (her gün bir satır, 5 saatlik pencereler bloklar halinde).
  - Liste görünümü.
  - Pencere detay çekmecesi (drawer).
  - Haftalık döngüler sekmesi.
- **Örüntüler ekranı:**
  - Isı haritası (metrik seçimi: ≈ % / maliyet / token).
  - Saat dağılımı.
  - "Limite takılma riski en yüksek saatler".
  - Model karışımı trendi.
  - Aralık seçici.
- **Dışa aktarma:** Oturumlar, pencereler ve günlük özetler; CSV ve JSON; kaydetme diyaloğu.
- **Periyodik özet:** Haftalık (varsayılan) ve günlük (opsiyonel) yerelleştirilmiş bildirim. Tıklanınca Genel Bakış açılır.
- **v0.2.0:** Performans doğrulaması, README güncellemesi, CHANGELOG, sürüm.

## Kapsam dışı

- Widget, çoklu hesap (Adım 18-19).

## Görevler

### Pencereler

- [ ] **Zaman şeridi:**
  - Y ekseni: günler (en yeni üstte). X ekseni: 00-24 saat.
  - Her 5 saatlik pencere bir blok olarak çizilir. Bloğun doluluğu tepe yüzdeye göre ayarlanır; limite takılan pencereler kırmızı kenarlıkla, tahmini pencereler kesikli kenarlıkla gösterilir.
  - Hover → tepe %, süre ve ilk 3 oturum.
  - Klik → detay çekmecesi.
  - Uzun aralıklarda sanal kaydırma.
  - Çizim SVG ile yapılır; 90 gün × ~3 pencere/gün = 270 blok olduğu için SVG yeterli.
- [ ] **Liste görünümü:** Başlangıç · Bitiş · Tepe % · Limite takıldı mı? · Oturum sayısı · Maliyet · Diğer payı · "tahmini" rozeti.
- [ ] **Detay çekmecesi:**
  - Pencerenin % eğrisi (snapshot'lar) ve oturum katkılarının yığılmış alan grafiği (zaman içinde).
  - Pay tablosu (oturumlar + Diğer).
  - Projeksiyon geçmişi: uyarı ne zaman verildi?
  - Bağlantılar: oturum detayları.
- [ ] **Haftalık döngüler:** Her hafta bir satır: tepe %, limite takıldığı gün, en yüksek etkili 3 proje, `seven_day_breakdown` yüzey payları. Scoped limitler (örneğin Fable) ayrı sütunlarda gösterilir.

### Örüntüler

- [ ] **Isı haritası (7 × 24):**
  - Yerel saat dilimi; haftanın ilk günü yerel ayara göre (TR'de Pazartesi).
  - Sıralı palet `dataviz` skill'iyle seçilir; renk körü dostu olmalı.
  - Hücre tooltip'i: ortalama ve toplam değer, aktif gün sayısı.
  - Metrik seçimi: ≈ % (5s) / maliyet / token.
- [ ] **Gün içi dağılım:** 24 sütunluk çubuk grafik. Hafta içi ve hafta sonu değerleri ayrı katmanlarda gösterilir.
- [ ] **Limit riski:** Saat başına "pencere limite takılma oranı" ve "≥ %90 olma oranı". Örnek içgörü cümlesi (i18n şablonu): *"Pencereleriniz en çok 14:00-17:00 arasında başlayınca limite takılıyor."* Cümle yalnızca istatistiksel olarak anlamlı yeterlilikte veri varsa (≥ 10 pencere) gösterilir.
- [ ] **Model karışımı trendi:** Haftalık yığılmış alan grafiği (maliyet payı).
- [ ] **Aralık seçici:** 4 hafta / 12 hafta / tümü / özel.
- [ ] Backend: Isı haritası ve risk sorguları Adım 14'te yazılmadıysa bu adımda eklenir. Performans eşiği ≤ 50 ms.

### Dışa aktarma

- [ ] `export(kind, range, format)` → kaydetme diyaloğu (`tauri-plugin-dialog`). Seçenekler: oturumlar (CSV/JSON), pencereler (CSV/JSON), günlük özet (CSV).
- [ ] CSV biçimi:
  - UTF-8 BOM eklenir (Excel uyumu için).
  - Ayraç yerel ayara göre seçilir: TR'de `;` (çünkü ondalık ayırıcı virgül), EN'de `,`.
  - Başlıklar seçili dilde olur.
  - JSON çıktısı makine dostu: İngilizce anahtarlar, ISO zaman damgaları.
- [ ] İçerik ve gizlilik: Dışa aktarılan dosyalarda yalnızca meta veri ve kullanım bulunur. Proje yolları dahil edilir (kullanıcının kendi verisi), ancak "yolları gizle" seçeneği de sunulur.

### Periyodik özet

- [x] `Notifier` kuralı: Haftalık özet, haftalık reset anından 1 saat sonra veya kullanıcının seçtiği gün ve saatte gönderilir. İçerik:
  - Geçen haftanın tepe değeri
  - Limite takılan pencere sayısı
  - API karşılığı maliyet
  - En yüksek etkili proje
  - Önceki haftayla karşılaştırma
- [x] Günlük özet opsiyoneldir (varsayılan kapalı).
- [x] Ayarlara "Özet" bölümü eklenir.
- [x] Testler (sahte saat): Zamanlama, yeniden başlatma sonrası tekrar göndermeme, sessiz saatlere uyma.

### v0.2.0 sürümü

- [x] Performans: Ana pencerenin tüm ekranları için bellek ve gecikme ölçülür. 1 yıllık sentetik veriyle sorgu ve çizim süreleri raporlanır.
- [x] README: v0.2 özellikleri (ekran görüntüleriyle), atıf ve kalibrasyonun nasıl çalıştığına dair kısa ve dürüst bir açıklama ("estimates are labeled").
- [ ] CHANGELOG ve sürüm numaraları güncellenir. Tag ve release **kullanıcı onayıyla** yapılır (Adım 12 prosedürü).

## Teknik notlar ve tuzaklar

- **Haftanın ilk günü ve saat dilimi:** `Intl.Locale.prototype.getWeekInfo` (destekleniyorsa) veya yerel ayar tablosu kullanılır. Gün sınırları backend'de yerel saat diliminde hesaplanır; frontend yeniden hesaplamaz.
- **İstatistiksel dürüstlük:** Az veriyle (örneğin 2 hafta) üretilen "en riskli saatler" cümlesi yanıltıcı olabilir. Minimum örneklem eşikleri uygulanır ve yetersiz veri durumunda "henüz yeterli veri yok" gösterilir.
- **Excel ve Türkçe:** BOM'suz UTF-8 dosyalarda Türkçe karakterler bozulur. Ayraç olarak virgül kullanılırsa sütunlar karışır.

## Kabul kriterleri

- [ ] Pencereler ve Örüntüler ekranları gerçek veriyle anlamlı ve doğru sonuçlar verir (devir notunda ekran görüntüleri ve 3 elle doğrulanmış örnek).
- [ ] CSV dosyası Excel'de (TR yerel ayarı) doğru açılır.
- [ ] Haftalık özet doğru zamanda bir kez gelir.
- [ ] v0.2 performans raporu bütçe içindedir. `v0.2.0` yayınlanmıştır.

## Doğrulama

```powershell
cargo test -p tokenradar-core notifier queries
pnpm -C app test; pnpm -C app check
pnpm tauri build
.\scripts\measure-memory.ps1 -ProcessName tokenradar-app -Scenario main-window
```

## Devir notu

### 17a (2026-11-21)

**Yapılanlar:**
- Çekirdek:
  - `list_weeks`: 24 saatlik gün dilimleri, `hit_at`, ilk üç proje, kapsamlı tepeler, kırılım.
  - `window_strip`: 5 saatlik pencereler gece yarısında kesilir.
  - `get_window` iki alan kazandı: `contrib` ve `alerts`.
  - `limit_risk` ve `model_mix_trend` eklendi; `get_heatmap` saat başına hafta içi ve hafta sonu ortalamalarını da döner.
  - `export` modülü: CSV ve JSON.
- i18n: `Domain::Export` ve `locales/*/export.ftl`.
- Sentetik üretici 5 saatlik pencere ve 5 saatlik kalibrasyon üretiyor.
- Uygulama:
  - `list_weeks`, `window_strip`, `limit_risk`, `model_mix_trend` ve `export_data` komutları. `export_data` kaydetme penceresini açar.
  - Pencereler sayfası: haftalık döngüler, 5 saatlik şerit, liste, çekmece.
  - Örüntüler sayfası: ısı haritası, gün içi dağılım, limit riski, model karışımı.
  - "Dışa aktar…" düğmesi Oturumlar, Pencereler ve Örüntüler sayfalarında.
  - Galeride sentetik örnekler. `Soon.svelte` ve `main-soon-*` anahtarları kaldırıldı.
- Testler:
  - Çekirdek: `queries/tests/windows_patterns.rs` (7 test) ve `export.rs` birim testleri.
  - Ön yüz: `windows/filter.test.ts` ve `windows/view.test.ts`.
- Belgeler: ADR-070 (Önerildi), `architecture.md` §6, `design-language.md` (bileşenler ve sözlük), `docs/perf/2026-11-21-windows-patterns.md`.

**Doğrulama:**
- `cargo fmt --check` ve `cargo clippy --workspace --all-targets -D warnings` temiz.
- `cargo test -p tokenradar-core -- queries export notifier` geçti.
- `pnpm -C app check / test / lint / build` temiz; `main.html` JS 124,8 KB (≤ 180 KB).
- Yeni sorguların hepsi p95 ≤ 50 ms (perf notu).
- Görsel inceleme: galeri headless Edge'de açık/EN ve koyu/TR çekildi. Düzeltilen kusurlar:
  - Haftalık tablo esnek düzende sıfır yüksekliğe çöküyordu.
  - Risk lejantının renk kutuları çubuk sınıflarıyla çakışıp kayboluyordu; "%90" rengi oklch karışımında morumsu çıkıyordu.
  - Katkı grafiğinde ölçek yoktu (sol üste tepe değeri eklendi).
  - Proje sütunu çok genişti.

**Sapmalar:**
- Kullanıcı kararlarıyla haftalık öncelikli sayfa ve yalnızca 5 saatlik limit riski (bkz. "17a öncesi kararlar").
- Katkı grafiği beş yerine dört oturum adlandırıyor: doğrulanmış kategorik renk dört tane (`CONTRIB_SESSIONS = 4`).
- Günlük toplamlar yalnızca CSV (plandaki gibi), oturum ve pencere CSV/JSON.

**Onaylanan kararlar (ADR-070, kullanıcı 2026-11-21'de onayladı):**
1. Günler sıfırlanmadan başlayan 24 saatlik dilimler; değerleri `≈` yerel tahmin, "Diğer" yok, toplamları tepenin altında kalabilir (not ile).
2. Limite takıldığı an ölçümdür (%100 olan ilk okuma); tahmini pencerede "Bilinmiyor".
3. Katkılar: saklanan pay, birikimli maliyetle orantılı zamana yayılır; dört oturum ve "Diğer oturumlar".
4. Riskli aralık kuralı: en az 10 pencere; 3 saatlik aralık, en az 5 pencere ve 2 takılma, oran genelin en az 1,5 katı.
5. Haftanın ilk günü ön yüzden (`getWeekInfo`, yoksa tr → Pazartesi, en → Pazar).
6. Dışa aktarma biçimi: BOM, CRLF, TR `;` ve ondalık virgül, formül koruması (`'`), JSON İngilizce anahtar ve ISO UTC.
7. Dışa aktarma düğmesi her sayfanın filtre satırında, sayfanın aralığı ve filtreleriyle.
8. Varsayılanlar: 12 hafta, haftalık sekmesi, ısı haritasında maliyet; model karışımı pay olarak.

**Gerçek veri koşusu (2026-11-21):**
- Kullanıcı kurulu uygulamayı kapattı. Release derlemesi, `portable` işaretli bir `%TEMP%` klasöründe gerçek verinin kopyasıyla açıldı; gerçek veri dizinine bir şey yazılmadı. Ekran görüntüleri `%TEMP%`'te kaldı.
- **3 elle doğrulanmış örnek:** Son üç haftalık pencerenin 21 günü, ayrı bir Python betiğiyle salt okunur hesaplandı: `usage_5m` maliyeti, son haftalık kalibrasyonun `k · α` ile çarpıldı. Uygulamanın `list_weeks` IPC çıktısıyla maliyet ve `≈` değerleri birebir eşleşti. Gün toplamları tepeyle karşılaştırılmadı (yalnızca yerel kullanım, "Diğer" yok).
- Gerçek veride hiç takılma yok. "Limite takıldı" yalnızca sentetik testlerle sınandı.
- Bulunan ve düzeltilen kusurlar:
  - Isı haritası rampası `oklch` karışımında lavanttan mora ve turkuaza kayıyordu: gri neredeyse hiç kroma taşımadığı için tonu belirsiz. Karışım `oklab`'a alındı, artık tek ton. Galeride gözden kaçmıştı.
  - Liste görünümünde "Fable · haftalık" yanındaki "Sürüyor" etiketi kesiliyordu. İlk sütun en az 13em oldu.
- Koşu ≈ 5 dakika sürdü. Bu sürede statusline ve API'den gelen 4 limit okuması kopyaya gitti ve gerçek veritabanına girmedi. Kopya `%TEMP%	r17aun` altında duruyor.

**Açık konular:**
- TR CSV'nin Excel'de açılması (kullanıcı).
- Bellek ölçümü (`measure-memory.ps1 -Scenario main-window`) bu koşuda yapılmadı; 17b'nin v0.2 raporuna kaldı.
- Pencere çekmecesi gerçek veride çekilmedi, yalnızca galeride.
- Tahmini pencerelerin tepesi ondalıklı (`≈%95,1`), ölçülenler tam sayı (`%53`). Tutarsız görünüyor; tahminler tam sayıya yuvarlanabilir.

**17b'ye notlar:**
- Değişmeyen `list_sessions/*` ve `list_projects/*` sorguları bu makinede gürültülü ölçüldü (p95 35–93 ms, eski veritabanında da). v0.2 raporu bunları sessiz bir makinede yeniden ölçmeli.
- `list_weeks/year` ≈ 47 ms, sınıra yakın; gerekirse pencere başına sorgular toplu hale getirilir.
- Bildirim geçmişi 30 günle sınırlı; dönemsel özet aynı `notification_log`'u kullanacaksa saklama süresi düşünülmeli.
- Tarihler mevcut sayfalardaki gibi `toLocaleDateString` ile yazılıyor; ADR-031'in "Intl kullanılmaz" kuralı yalnızca sayı, süre ve saat biçimlendiricileri için uygulanıyor.

### 17b (2026-11-21)

**Yapılanlar:**
- Çekirdek:
  - `notifier/digest.rs`: `Rule::Digest`, haftalık ve günlük dönem (`Slot`), durum (`DigestState`), metin. `NoticeTarget` (panel ya da Genel Bakış).
  - `notifier/prefs.rs`: `[notifications.digest]` (haftalık açık, reset+1 sa ya da seçilen gün ve saat; günlük kapalı, 09:00).
  - `queries/digest.rs`: `week_bounds`, `weekly_digest` (tepe, takılma anı, kapsamlı limitler, 5 saatlik pencereler, en çok etkileyen proje, önceki hafta) ve `daily_digest`.
  - `queries/zone.rs`: `at_local_minute`, `next_local_day_start`, `local_weekday`.
  - `runtime.rs`: Limit iş parçacığı bir sonraki özet vadesinde uyanır (`check_digests`). Atıf yoksa 15 dakikada bir, en çok 2 saat bekler.
- Uygulama:
  - Toast tıklaması `tokenradar://main/overview` ile Genel Bakış'ı açar.
  - `preview_weekly_digest` komutu.
  - Bildirimler sekmesinde "Haftalık özet" ve "Günlük özet" kartları. Gün seçicide tam gün adları (`weekdayLongName`).
- Testler:
  - Çekirdek: `notifier/digest_tests.rs` (12), `queries/tests/digest.rs` (3), runtime'da yeniden başlatmaya dayanıklılık testi.
  - Uygulama: özet toast'unun Genel Bakış'ı açması.
  - Ön yüz: tam gün adları.
- Sürüm:
  - 0.2.0 sürüm numarası (`Cargo.toml`, `app/package.json`).
  - CHANGELOG `[0.2.0]`.
  - README: ana pencere, özetler, "How the estimates work" ve Pencereler sayfasının sentetik görüntüsü (`docs/img/main-windows-*.png`).
  - Release profili boyuta göre (ADR-072).
- Sentetik üretici `--end-now` ve `--config-dir` aldı: uygulama sentetik veritabanını kendi hesabı gibi açabiliyor (README görüntüleri). Benchmark'a iki özet sorgusu eklendi.
- Belgeler:
  - ADR-071 ve ADR-072 (Önerildi).
  - `architecture.md` §5.
  - `performance-budget.md`.
  - `docs/perf/2026-11-21-v0.2.0.md`.

**Doğrulama:**
- `cargo fmt --check` ve `cargo clippy --workspace --all-targets -- -D warnings` temiz. `cargo test --workspace` geçti: core 623, app 105, CLI 29, i18n 16.
- `pnpm -C app check / lint / test` temiz (242 test). `main.html` JS 124,8 KB, `panel.html` JS 47,6 KB.
- `pnpm tauri build`: `TokenRadar_0.2.0_x64-setup.exe` 5,65 MiB. `tokenradar-app.exe` 12,91 MiB, CLI 2,60 MiB.
- Sorgular 1 yıllık sentetik veride p95 ≤ 36,7 ms.
- Bellek ve gecikme: perf raporu. Planın `-Scenario main-window` parametresi betikte yok; senaryo CDP ile sürüldü.
- Gerçek veri koşusu: Kullanıcı kurulu uygulamayı kapattı. Release derlemesinin portable kopyası gerçek verinin kopyasıyla açıldı.
  - Ana pencerenin beş sayfası ölçüldü.
  - Ayarlardaki özet kartları TR/açık ve EN/koyu çekildi.
  - "Geçen haftanın özetini şimdi göster" toast'u TR ve EN'de doğru metinle geldi.
  - Görüntüler incelendikten sonra silindi.
- 0.2.0 kurulumu kurulu uygulamanın üzerine yapıldı (kullanıcı onayıyla, `/S`). Uygulama şema v2'den v5'e geçti. Kapalıyken biriken 40 statusline okuması içe aktarıldı.

**Sapmalar:**
- Kullanıcı kararlarıyla: limit haftası, reset + 1 sa, kaçan özet 48 saat içinde, haftalık limite takılma satırı (bkz. "17b öncesi kararlar").
- Plandaki "limite takılan pencere sayısı", 5 saatlik penceresi olan hesaplarda gösterilir. Diğerlerinde haftalık limitin takılma durumu gösterilir.
- **İkili boyutu aşıldı ve düzeltildi:** `opt-level = 3` ile 16,05 MiB çıktı. Kullanıcı kararıyla kabuk `s`, çekirdek `3` (ADR-072): 12,91 MiB. Sorgu süreleri değişmedi.
- **v0.2.0 bilinen bir bütçe aşımıyla hazırlandı (kullanıcı kararı):** Panelin sıcak açılışı 56,2 ms (≤ 50 ms). Eski profille de 53,8 ms, yani neden profil değil.

**Önerilen kararlar (ADR-071 ve ADR-072, kullanıcı onayı bekliyor):**
1. Özetin anahtarı `digest:<weekly|daily>:-:<dönem sonu>:-`. Gönderilen özetler bellekte 10 gün tutulur.
2. Günlük özet bir sonrakinin vadesine kadar (24 saat) geç gönderilebilir.
3. Sessiz saatler ve susturma özeti bekletir. Sonunda gecikme sınırı geçmişse özet atlanır.
4. Atıf beklemesi: 15 dakikada bir, vadeden en çok 2 saat sonrasına kadar.
5. Boş dönem kaydedilir ama gösterilmez.
6. Tıklama Genel Bakış'ı açar. Windows dışında eklenti tıklaması şimdilik paneli açar (Adım 21).
7. Ayarlar kartları ve "şimdi göster" düğmesi. Düğme hiçbir şey kaydetmez.
8. Birincil hesap kullanılır. Açık pencere yoksa 6 saatte bir yeniden bakılır.
9. ADR-072: Sıcak yol bağımlılıkları da `3`'te kalır (`rusqlite`, `libsqlite3-sys`, `serde_json`, `memchr`). CLI profili bunları `s`'ye geri çeker.

**Açık konular:**
- **Tag ve release kullanıcıda.** Komutlar aşağıdadır. Release yayınlandıktan sonra:
  - `bucket/tokenradar.json` güncellenmeli.
  - winget PR'ı açılmalı (CONTRIBUTING, Releasing 3–4).
- **Panelin sıcak açılışı gerilemesi:** 47,2 ms'ten (7 Kasım) 54–56 ms'e çıktı. 12'den bu yana gelen commit'ler panel ölçümüyle ikiye bölünerek aranmalı.
- Gerçek bir özetin zamanında bir kez gelmesi henüz gözlenmedi. İlk fırsat Çar 2026-11-25'teki haftalık reset'ten bir saat sonrası. Kabul kriteri buna bağlı.
- Özet toast'una tıklamanın Genel Bakış'ı açtığı yalnızca birim testiyle sınandı. Portable kopya protokol kaydı olmadığı için tıklamayı yönlendiremez. Kurulu 0.2.0'da elle denenmeli.
- Ana pencere belleği hızlı bir ilk turda bir kez 183,0 MB'a çıktı (≈ 6 s, bütçe 180). İlk kullanımda tepe 140 MB. Kaynak WebView2'nin renderer/GPU süreçleri; v0.3'te izlenmeli.
- Oturumlar ve Projeler sayfalarının veriyle boyanması sınıra yakın (en kötü 94,8 ve 91,4 ms).
- Sentetik görüntülerde görülenler (gerçek hesapta 5 saatlik pencere yok):
  - Genel Bakış grafiğinde 5 saatlik çizgi, pencereler arasındaki boşlukta 0'a düşmek yerine saatlerce eğimli iniyor (15b'den beri).
  - CDP `click()` ile rota değişince sayfa başlığında kalın odak halkası görünüyor. Gerçek fare tıklamasında görünüp görünmediği denenmeli.
- Ayarlar Bildirimler sekmesinde, 5 saatlik penceresi olmayan hesapta da "5 saatlik pencere" eşik satırı görünüyor (ADR-041 öncesinden kalma).
- 17a'dan: TR CSV'nin Excel'de açılması (kullanıcı).

**Sürüm komutları (kullanıcı çalıştırır):**

```powershell
git push origin main
git tag v0.2.0
git push origin v0.2.0
# release.yml taslak release oluşturur; kontrol edip yayınla:
gh release view v0.2.0 --web
```
