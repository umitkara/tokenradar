# Adım 08 — Tray: dinamik ikili halka ve menü

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1 | 06 | 1 oturum | ✅ |

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

- [x] `locales/en/tray.ftl`, `locales/tr/tray.ftl`: tooltip ve menü anahtarları.
- [x] `tokenradar-i18n`:
  - `Localizer::new(locale)`, `fmt(key, args)`
  - Özel fonksiyonlar `NUM`, `PCT`, `DUR`, `TR_SUFFIX` (Rust uygulaması)
  - `Locale::detect()` (`sys-locale`)
- [x] `testdata/i18n-vectors.json`: Ortak test vektörleri. Adım 09'da TS tarafı da bu dosyayı kullanacak. En az şu örnekler bulunmalı:
  - `PCT`: 0 / 5 / 72 / 100 / 23,5
  - `DUR`: 59 sn / 1 dk / 1 sa 48 dk / 2 g 3 sa
  - `NUM`: 1234,5
  - `TR_SUFFIX`: 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 1000 ve 1.000.000 için `loc`/`abl`/`dat`/`acc`/`gen` halleri, ayrıca yüzde biçimleri (`%72'si`, `%100'e`)
- [x] `TR_SUFFIX` kuralı: Sayının **okunuşunun** son kelimesi bulunur (son sıfır olmayan basamak veya on/yüz/bin/milyon büyüklüğü). Ünlü uyumu (e/a, i/ı/u/ü) ve ünsüz sertleşmesi (`-de/-te`, `-den/-ten`) bu kelimeye göre uygulanır. Kesme işareti kullanılır: `5'te`.

### Çizim

- [x] `render(state: &TrayVisual, px: u32, theme: TaskbarTheme) -> RgbaImage`:
  - Geometri `design-language.md` "Tray ikonu" bölümündeki gibi. Kalınlıklar piksel ızgarasına oturtulur, böylece 16 px'te bulanıklık oluşmaz.
  - Halkalar 12 yönünden başlar ve saat yönünde dolar. Track rengi temaya göre seçilir. Dolu yay durum rengindedir.
  - Kritik durumda iç halkanın ucuna çentik eklenir. Bayat veride ikon %50 opaklıkta çizilir. Hata durumunda köşeye nokta konur. `NoData` durumunda yalnızca track'ler ve ortada küçük bir nokta görünür.
  - Antialiasing açıktır; 16 px için bu ayar görsel olarak doğrulanmalı.
- [x] Boyut seçimi: Windows'ta `GetSystemMetricsForDpi(SM_CXSMICON, dpi)` kullanılır (16 px @%100, 20 @%125, 24 @%150, 32 @%200). DPI değişikliği olduğunda (monitör değişimi, ölçek değişimi) ikon yeniden üretilir.
- [x] Tema tespiti:
  - `HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize\SystemUsesLightTheme` değeri okunur.
  - Değişiklikler `RegNotifyChangeKeyValue` ile ayrı ve uyuyan bir thread'de beklenir; polling yapılmaz.
  - Yüksek kontrast modu `SystemParametersInfo(SPI_GETHIGHCONTRAST)` ile tespit edilir ve bu modda monokrom ikon çizilir.
- [x] Güncelleme kısıtlaması:
  - `TrayVisual`, görsel olarak anlamlı alanlara yuvarlanır: yüzdeler 1 puana, durumlar ve bayraklar olduğu gibi.
  - Önceki görselle aynıysa `set_icon` çağrılmaz.
  - Tooltip için ayrı bir kısıtlama uygulanır: geri sayım en fazla dakikada bir güncellenir.
- [x] `CoreEvent::LimitsUpdated` → `TrayVisual` dönüşümü ve uygulama.
- [x] Görev çubuğu yeniden başlatıldığında (Explorer çökmesi) ikon kaybolabilir. Tauri bunu `TaskbarCreated` mesajıyla ele alıyor mu? **Doğrula**. Ele almıyorsa ikonu yeniden ekle.

### Menü ve etkileşim

- [x] Sol tık (button up) → paneli aç/kapat. Sağ tık → menü. Çift tık için davranış tanımlanmaz.
- [x] "Bildirimleri 1 saat sustur": durum config'e değil `kv` tablosuna yazılır (`notify_snoozed_until`). Menü öğesinin metni duruma göre değişir.
- [x] Dil değişimi olayı geldiğinde menü ve tooltip yeniden kurulur.

### Test ve ölçüm

- [x] Altın (golden) PNG testleri: 4 boyut × 2 tema × 7 durum. Tolerans kullanılır, çünkü antialiasing farkları olabilir. Golden dosyalar `testdata/tray/` altında tutulur ve `UPDATE_GOLDEN=1` ile güncellenir.
- [x] i18n vektör testleri (Rust tarafı).
- [x] Ölçüm: 1.000 kez render süresi (hedef < 0,2 ms/ikon) ve 10 dakika boyunca saniyede bir sahte `LimitsUpdated` olayı gönderilirken CPU kullanımı.
- [ ] Görsel kontrol: Ekran görüntüleri alınır (açık ve koyu görev çubuğu, %100 ve %150 ölçek) ve devir notuna eklenir. 16 px'te iki halkanın ayırt edilebilir olduğu insan gözüyle onaylanmalı. **Gerekirse kalınlıkları ayarla** ve `design-language.md`'yi güncelle.

## Teknik notlar ve tuzaklar

- Tauri'nin tray API'si `Image::new_owned(rgba, w, h)` ile çalışır. Windows HICON üretimini Tauri yapar. Her `set_icon` çağrısı yeni bir HICON oluşturur. Sızıntı olmadığını, 10.000 güncellemelik bir döngüde GDI nesne sayısını izleyerek (Görev Yöneticisi → GDI objects) doğrula.
- Tooltip Windows'ta en fazla 127 karakter olabilir. TR metinleri daha uzun olduğu için kısaltma kuralları gerekir.
- Bildirim susturma süresi uygulama yeniden başlasa bile korunur, bu yüzden `kv` tablosunda saklanır.

## Kabul kriterleri

- [x] İkon 16-32 px aralığında, açık ve koyu temada okunur. Durum değişimleri (ok → warn → crit, bayat) görsel olarak ayırt edilebilir.
- [x] Tooltip EN ve TR'de doğru biçimlendirilir (`%72`, `1 sa 48 dk`, `Çar 10:59'da`).
- [x] Hiçbir değişiklik yokken `set_icon` çağrılmaz.
- [x] GDI nesne sayısı 10.000 güncelleme sonunda sabit kalır.
- [x] Rust i18n vektör testlerinin hepsi geçer.

## Doğrulama

```powershell
cargo test -p tokenradar-i18n
cargo test -p tokenradar-app tray
pnpm dev   # TOKENRADAR_FAKE_LIMITS=cycle ile durumlar arasında döngü (yalnızca debug)
```

## Devir notu

**Tarih:** 2026-10-27 · **Dal:** `step/08-tray` · **Karar:** ADR-030 · **Ölçüm:** `docs/perf/2026-10-27-tray.md`

### Yapılanlar
- **`tokenradar-i18n`:**
  - `suffix.rs`: `tr_suffix(text, Case)`. Haller `loc`/`abl`/`dat`/`acc`/`gen`/`poss`. Son okunan kelime bulunur (birler, onlar, yüz, bin, milyon, milyar, trilyon), ardından ünlü uyumu ve sertleşme uygulanır. Girdi biçimlenmiş metin de olabilir (`%23,5'te`, `10:59'da`, `Çar 10:00'da`).
  - `TR_SUFFIX` Fluent fonksiyonu: sayı alırsa önce `NUM` ile biçimler. EN'de girdiyi olduğu gibi döndürür.
  - `format.rs`: `Weekday`, `HourCycle`, `clock`, `weekday_clock` (TR her zaman 24 saat; EN 12/24 sistemden).
  - `Domain::Tray` ve `locales/{en,tr}/tray.ftl` eklendi.
  - `testdata/i18n-vectors.json` eklendi (NUM/PCT/DUR/clock/TR_SUFFIX). `tests/vectors.rs` bu dosyayı çalıştırır.
- **Tray (`app/src-tauri/src/tray/`):**
  - `visual.rs`: `TrayModel` → `TrayVisual` (tam puanlı yüzdeler, halka başına önem, stale/error/paused). Kapsamlı haftalık limitler `scoped_alerts()` ile ayrılır.
  - `render.rs`: tiny-skia ile annular sector yolları, piksel ızgarasına oturan kalınlıklar, koyu/açık/HC paletleri.
  - `tooltip.rs`: Full → Untitled → Short → satır sınırında kesme; `next_change` dakika sınırında veya susturma bitiminde uyanır.
  - `system.rs`: ikon boyutu (`GetDpiForWindow(Shell_TrayWnd)` + `GetSystemMetricsForDpi`), tema, HC mürekkebi, yerel saat, gizli pencereli `watch`.
  - `mod.rs`: `TrayMsg`, `Presenter` (ikon, tooltip ve menü ayrı ayrı kısıtlanır), `Driver` (kendi thread'i ve kendi `Localizer`'ı), menü, susturmanın `kv`'ye yazılması ve okunması.
  - `fake.rs`: `TOKENRADAR_FAKE_LIMITS=cycle|sweep|stress` (debug veya `spike`).
- **`core_bridge.rs`:** `CoreReady(Store)`, ilk snapshot, `LimitsUpdated` ve Resync → `TrayMsg::Snapshot`; hata durumunda `CoreFailed`. Adım 01'den kalan debug tooltip'i kaldırıldı.
- **`settings.rs`:** "Ayarlar" ve "Hakkında" boş bir ayarlar penceresi açar.
- Kullanılmayan yer tutucular `icons/tray.png` ve `icons-src/tray-icon.svg` silindi.
- **Testler:** tray 34 (+2 ignored: kontak sayfası, render hızı); 108 golden PNG (3 tema × 9 durum × 4 boyut); i18n 16 birim + 6 vektör. core 201 ve CLI 17 değişmedi.

### Gerçek doğrulama (2026-10-27)
- **GDI:** 10.000 `set_icon` sonunda GDI 7 / USER 13; başlangıçla aynı.
- **CPU:** 10 dakika, saniyede bir güncellemeyle %0,055 (tek çekirdek). Render 19-32 µs/ikon.
- **Görsel:** Kullanıcı `pnpm dev` (`TOKENRADAR_FAKE_LIMITS=cycle`) ile tray'i canlı kontrol etti. Halkalar ayırt ediliyor, tooltip ve menü doğru, tema ve ölçek değişince ikon yeniden çiziliyor. Ekran görüntüsü eklenmedi; kontak sayfaları `cargo test -p tokenradar-app tray_contact_sheet -- --ignored` ile `target/tray-{dark,light,hc}.png` olarak üretilir.

### Sapmalar (ADR-030, kullanıcı kararları)
- `RegNotifyChangeKeyValue` yerine tek bir gizli pencere: tema, DPI, ekran, HC ve `TaskbarCreated` olaylarını birlikte dinler.
- Her halka kendi önemiyle renklenir; çentik kritik olan her halkada (plan: yalnızca iç halka).
- Susturulmuş durum: sağ altta duraklat işareti. Hata noktası sağ üstte.
- Tray track'i opak (token'daki `/0.5` 16 px'te kayboluyordu).
- Tooltip çok satırlı, tam cümle; sığmazsa önce başlıksız, sonra kısa biçim.
- Kapsamlı haftalık limitler ikona girmez; ≥ %75 ise tooltip satırı.
- `TR_SUFFIX` biçimlenmiş metin de kabul eder; `poss` hali eklendi.
- Golden testler 2 değil 3 tema (HC dahil) ve 7 değil 9 durum (kısmi veri iki ayrı durum) kapsar.
- Sahte mod çekirdeği hiç başlatmaz (gerçek veri, DB ve API'ye dokunmaz).

### Açık konular
- **Explorer yeniden başlatma:** `tray-icon` 0.25 `TaskbarCreated`'ı ele alıyor (kaynakta doğrulandı), ancak gerçek bir Explorer yeniden başlatmasıyla denenmedi.
- **Yüksek kontrast:** Mürekkep `COLOR_WINDOWTEXT`; gerçek bir HC temasında görev çubuğu rengiyle uyumu denenmedi.
- **Dil değişimi:** `TrayMsg::Locale` hazır, ama göndereni Adım 11'deki ayarlar penceresi olacak. Şimdilik yalnızca sahte `cycle` modu gönderiyor.
- **Hakkında:** Şimdilik ayarlar penceresini açıyor (Adım 11).
- **Windows 11 taşma menüsünde sabitleme:** Bu adımda araştırılmadı. Olası yol `HKCU\Control Panel\NotifyIconSettings\<id>` altındaki `IsPromoted` değeridir (doğrulanmadı); araştırma ve uygulama Adım 11'e.
- **Bildirim tarafı:** Bildirimler (sonraki adımlar) susturma durumunu `kv` anahtarı `notify_snoozed_until`'dan (Unix ms metni) okumalı.
- **Gerçek veriyle boşta CPU** hâlâ ölçülmedi (Adım 05'ten kalan iş).

### Sonraki adıma notlar
- **Adım 09 (TS i18n):** `testdata/i18n-vectors.json` ortak dosyadır; TS tarafı aynı bölümleri (`num`, `pct`, `dur`, `clock`, `tr_suffix`) çalıştırmalı. `tr_suffix` girdisi sayıysa önce TR `NUM` ile 0 ondalıkla biçimlenir.
- **Adım 10 (panel):** Sol tık `panel::toggle`, menüdeki "Paneli aç" `panel::show` çağırır. Panel, tray ile aynı `Severity::of` eşiklerini ve aynı halka grameri kullanmalı.
- **Adım 11 (ayarlar):** Dil değişince `tray::send(app, TrayMsg::Locale(..))` gönderilmeli. Susturma menüden `TrayMsg::ToggleMute` ile yapılır.
