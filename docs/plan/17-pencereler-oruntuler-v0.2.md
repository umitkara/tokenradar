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

- [ ] `Notifier` kuralı: Haftalık özet, haftalık reset anından 1 saat sonra veya kullanıcının seçtiği gün ve saatte gönderilir. İçerik:
  - Geçen haftanın tepe değeri
  - Limite takılan pencere sayısı
  - API karşılığı maliyet
  - En yüksek etkili proje
  - Önceki haftayla karşılaştırma
- [ ] Günlük özet opsiyoneldir (varsayılan kapalı).
- [ ] Ayarlara "Özet" bölümü eklenir.
- [ ] Testler (sahte saat): Zamanlama, yeniden başlatma sonrası tekrar göndermeme, sessiz saatlere uyma.

### v0.2.0 sürümü

- [ ] Performans: Ana pencerenin tüm ekranları için bellek ve gecikme ölçülür. 1 yıllık sentetik veriyle sorgu ve çizim süreleri raporlanır.
- [ ] README: v0.2 özellikleri (ekran görüntüleriyle), atıf ve kalibrasyonun nasıl çalıştığına dair kısa ve dürüst bir açıklama ("estimates are labeled").
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

_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
