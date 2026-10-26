# Adım 08 — Tray: dinamik ikili halka ve menü

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1 | 06 | 1 oturum | ⬜ |

## Amaç

Uygulamanın her zaman görünen yüzü olan tray ikonunu yapmak. İkon **WebView olmadan, Rust ile çizilen** canlı bir ikili halkadır: dış halka haftalık limiti, iç halka 5 saatlik pencereyi gösterir. Yanında yerelleştirilmiş tooltip ve menü bulunur. 16 piksellik bir alanda bile okunabilir olmalı ve ihmal edilebilir kaynak tüketmelidir.

## Önce oku

- `docs/design-language.md` §1, §2, §4 (Tray ikonu)
- `docs/architecture.md` §2, §12
- `docs/decisions.md`: ADR-003, ADR-019

## Kapsam

- `app/src-tauri/src/tray/render.rs`: `tiny-skia` ile RGBA ikon üretimi.
- DPI farkındalıklı boyut seçimi; görev çubuğu teması (açık/koyu) ve yüksek kontrast tespiti.
- Durum makinesi: `NoData`, `Ok`, `Warn`, `Crit`, `Stale`, `Error`, `Paused` (bildirimler susturulmuş) ve kısmi veri (yalnızca 5s veya yalnızca haftalık).
- Yeniden çizim kısıtlaması: yalnızca görsel olarak farklı bir sonuç üretilecekse ikon güncellenir.
- Tooltip (yerelleştirilmiş, en fazla 127 karakter).
- Menü: Paneli aç · Bildirimleri 1 saat sustur / Susturmayı kaldır · Ayarlar · Hakkında · Çıkış. Dil değişince menü yeniden kurulur.
- `tokenradar-i18n` crate'inin ilk gerçek kullanımı: `.ftl` yükleme, `PCT`/`DUR` fonksiyonları ve `TR_SUFFIX` fonksiyonunun Rust uygulaması.

## Kapsam dışı

- Panel içeriği (Adım 10).
- Ayarlar penceresi (Adım 11). Menü öğesi şimdilik boş bir pencere açar.
- Windows 11 tray taşma menüsünde ikonu sabitleme yardımcısı. Burada yalnızca araştırılır, uygulama Adım 11'de.

## Görevler

### i18n çekirdeği (Rust)

- [ ] `locales/en/tray.ftl`, `locales/tr/tray.ftl`: tooltip ve menü anahtarları.
- [ ] `tokenradar-i18n`:
  - `Localizer::new(locale)`, `fmt(key, args)`
  - Özel fonksiyonlar `NUM`, `PCT`, `DUR`, `TR_SUFFIX` (Rust uygulaması)
  - `Locale::detect()` (`sys-locale`)
- [ ] `testdata/i18n-vectors.json`: Ortak test vektörleri. Adım 09'da TS tarafı da bu dosyayı kullanacak. En az şu örnekler bulunmalı:
  - `PCT`: 0 / 5 / 72 / 100 / 23,5
  - `DUR`: 59 sn / 1 dk / 1 sa 48 dk / 2 g 3 sa
  - `NUM`: 1234,5
  - `TR_SUFFIX`: 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 1000 ve 1.000.000 için `loc`/`abl`/`dat`/`acc`/`gen` halleri, ayrıca yüzde biçimleri (`%72'si`, `%100'e`)
- [ ] `TR_SUFFIX` kuralı: Sayının **okunuşunun** son kelimesi bulunur (son sıfır olmayan basamak veya on/yüz/bin/milyon büyüklüğü). Ünlü uyumu (e/a, i/ı/u/ü) ve ünsüz sertleşmesi (`-de/-te`, `-den/-ten`) bu kelimeye göre uygulanır. Kesme işareti kullanılır: `5'te`.

### Çizim

- [ ] `render(state: &TrayVisual, px: u32, theme: TaskbarTheme) -> RgbaImage`:
  - Geometri `design-language.md` "Tray ikonu" bölümündeki gibi. Kalınlıklar piksel ızgarasına oturtulur, böylece 16 px'te bulanıklık oluşmaz.
  - Halkalar 12 yönünden başlar ve saat yönünde dolar. Track rengi temaya göre seçilir. Dolu yay durum rengindedir.
  - Kritik durumda iç halkanın ucuna çentik eklenir. Bayat veride ikon %50 opaklıkta çizilir. Hata durumunda köşeye nokta konur. `NoData` durumunda yalnızca track'ler ve ortada küçük bir nokta görünür.
  - Antialiasing açıktır; 16 px için bu ayar görsel olarak doğrulanmalı.
- [ ] Boyut seçimi: Windows'ta `GetSystemMetricsForDpi(SM_CXSMICON, dpi)` kullanılır (16 px @%100, 20 @%125, 24 @%150, 32 @%200). DPI değişikliği olduğunda (monitör değişimi, ölçek değişimi) ikon yeniden üretilir.
- [ ] Tema tespiti:
  - `HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize\SystemUsesLightTheme` değeri okunur.
  - Değişiklikler `RegNotifyChangeKeyValue` ile ayrı ve uyuyan bir thread'de beklenir; polling yapılmaz.
  - Yüksek kontrast modu `SystemParametersInfo(SPI_GETHIGHCONTRAST)` ile tespit edilir ve bu modda monokrom ikon çizilir.
- [ ] Güncelleme kısıtlaması:
  - `TrayVisual`, görsel olarak anlamlı alanlara yuvarlanır: yüzdeler 1 puana, durumlar ve bayraklar olduğu gibi.
  - Önceki görselle aynıysa `set_icon` çağrılmaz.
  - Tooltip için ayrı bir kısıtlama uygulanır: geri sayım en fazla dakikada bir güncellenir.
- [ ] `CoreEvent::LimitsUpdated` → `TrayVisual` dönüşümü ve uygulama.
- [ ] Görev çubuğu yeniden başlatıldığında (Explorer çökmesi) ikon kaybolabilir. Tauri bunu `TaskbarCreated` mesajıyla ele alıyor mu? **Doğrula**. Ele almıyorsa ikonu yeniden ekle.

### Menü ve etkileşim

- [ ] Sol tık (button up) → paneli aç/kapat. Sağ tık → menü. Çift tık için davranış tanımlanmaz.
- [ ] "Bildirimleri 1 saat sustur": durum config'e değil `kv` tablosuna yazılır (`notify_snoozed_until`). Menü öğesinin metni duruma göre değişir.
- [ ] Dil değişimi olayı geldiğinde menü ve tooltip yeniden kurulur.

### Test ve ölçüm

- [ ] Altın (golden) PNG testleri: 4 boyut × 2 tema × 7 durum. Tolerans kullanılır, çünkü antialiasing farkları olabilir. Golden dosyalar `testdata/tray/` altında tutulur ve `UPDATE_GOLDEN=1` ile güncellenir.
- [ ] i18n vektör testleri (Rust tarafı).
- [ ] Ölçüm: 1.000 kez render süresi (hedef < 0,2 ms/ikon) ve 10 dakika boyunca saniyede bir sahte `LimitsUpdated` olayı gönderilirken CPU kullanımı.
- [ ] Görsel kontrol: Ekran görüntüleri alınır (açık ve koyu görev çubuğu, %100 ve %150 ölçek) ve devir notuna eklenir. 16 px'te iki halkanın ayırt edilebilir olduğu insan gözüyle onaylanmalı. **Gerekirse kalınlıkları ayarla** ve `design-language.md`'yi güncelle.

## Teknik notlar ve tuzaklar

- Tauri'nin tray API'si `Image::new_owned(rgba, w, h)` ile çalışır. Windows HICON üretimini Tauri yapar. Her `set_icon` çağrısı yeni bir HICON oluşturur. Sızıntı olmadığını, 10.000 güncellemelik bir döngüde GDI nesne sayısını izleyerek (Görev Yöneticisi → GDI objects) doğrula.
- Tooltip Windows'ta en fazla 127 karakter olabilir. TR metinleri daha uzun olduğu için kısaltma kuralları gerekir.
- Bildirim susturma süresi uygulama yeniden başlasa bile korunur, bu yüzden `kv` tablosunda saklanır.

## Kabul kriterleri

- [ ] İkon 16-32 px aralığında, açık ve koyu temada okunur. Durum değişimleri (ok → warn → crit, bayat) görsel olarak ayırt edilebilir.
- [ ] Tooltip EN ve TR'de doğru biçimlendirilir (`%72`, `1 sa 48 dk`, `Çar 10:59'da`).
- [ ] Hiçbir değişiklik yokken `set_icon` çağrılmaz.
- [ ] GDI nesne sayısı 10.000 güncelleme sonunda sabit kalır.
- [ ] Rust i18n vektör testlerinin hepsi geçer.

## Doğrulama

```powershell
cargo test -p tokenradar-i18n
cargo test -p tokenradar-app tray
pnpm dev   # TOKENRADAR_FAKE_LIMITS=cycle ile durumlar arasında döngü (yalnızca debug)
```

## Devir notu

_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
