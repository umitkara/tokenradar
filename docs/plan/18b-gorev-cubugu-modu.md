# Adım 18b — Widget'ın görev çubuğu modu

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.3 (13'ten önce yapılabilir) | 18 | 1 oturum | ✅ (2026-11-11) |

## Amaç

Adım 18'in native şeridine ikinci bir mod eklemek: şerit, görev çubuğunun boş alanına kenetlenir ve görev çubuğunun bir parçası gibi görünür (TrafficMonitor tarzı). Mod isteğe bağlıdır. Yüzen mod her durumda yedek olarak kalır.

Windows 11'de görev çubuğuna öğe eklemek için resmi bir API yoktur; DeskBand kaldırıldı. Bu yüzden mod **en iyi çaba** (best effort) olarak sunulur: kenetlenme başarısız olursa widget yüzen moda düşer ve bunu kullanıcıya bir kez söyler (ADR-058).

## Önce oku

- `docs/plan/18-widget.md` ve **devir notu**
- `docs/decisions.md`: **ADR-058**, ADR-030 (`TaskbarCreated`, görev çubuğu DPI'ı), ADR-032 (panel konumu, `ABM_GETTASKBARPOS`)
- `docs/design-language.md` §4 (Widget, görev çubuğu modu)
- Kod: `app/src-tauri/src/widget/` (Adım 18), `panel/place.rs` + `panel/system.rs` (görev çubuğu kenarı, otomatik gizleme payı, DPI), `tray/system.rs` (`TaskbarCreated`, görev çubuğu teması)

## Kapsam

- **Yöntem spike'ı (ilk iş, ≤ 1 saat):** İki yol karşılaştırılır. Seçim ve gerekçe ADR-058'e eklenir.
  - (a) **Owned topmost popup:** Pencerenin sahibi `Shell_TrayWnd` olur; pencere görev çubuğunun üstünde konumlanır. Explorer'ın pencere ağacına girilmez. Görev çubuğuna tıklanınca z-sırası kaybolursa yeniden öne alınır (`EVENT_SYSTEM_FOREGROUND` / `SetWinEventHook`).
  - (b) **Çocuk pencere:** `SetParent(Shell_TrayWnd)`. Z-sırası sorunu yoktur, ama süreçler arası ebeveynlik giriş kuyruklarını birleştirir (`AttachThreadInput` etkisi). Explorer'ın donması widget'ı da dondurur, ters yönde de aynısı olur.
  - Ölçüt: Windows 11 24H2+'da görünürlük, tıklama, Explorer'ın kararlılığı, Explorer yeniden başlatıldığında davranış. Varsayılan tercih (a)'dır.
- **Konum:**
  - Görev çubuğu dikdörtgeni ve kenarı: `panel/system.rs`'teki `ABM_GETTASKBARPOS` okuması yeniden kullanılır.
  - Bildirim alanı: `Shell_TrayWnd` → `TrayNotifyWnd` dikdörtgeni. Varsayılan konum bildirim alanının hemen solu, 8 px boşlukla.
  - Kullanıcı ofseti: Simgeler ortalanmışsa veya çok uygulama açıksa çakışma olabilir. Ayarlarda yatay ofset ve "sola yasla" seçeneği bulunur.
  - Hesap `widget/place.rs` içinde saf bir fonksiyondur, fiziksel pikselde ve birim testlidir.
- **Yerleşim:**
  - Yükseklik görev çubuğu yüksekliğine uyar. Görev çubuğu ≥ 40 mantıksal px ise iki satır, değilse tek satır kullanılır. Tek satır biçimi: `7g %34 ▮▮▮▯▯ · 5s %72 ▮▮▮▮▯`. 5s yoksa yalnızca haftalık gösterilir.
  - Zemin şeffaftır (görev çubuğuyla kaynaşır). Hover'da görev çubuğu düğmeleri gibi ince bir vurgu çizilir. Renkler görev çubuğu temasından (`SystemUsesLightTheme`) gelir, uygulama temasından gelmez.
- **Dayanıklılık:**
  - `TaskbarCreated` (Explorer yeniden başladığında): Widget yeniden kenetlenir.
  - Görev çubuğu boyutu, kenarı veya DPI'ı değişince konum yeniden hesaplanır.
  - Otomatik gizlenen görev çubuğu: Widget görev çubuğuyla birlikte gizlenir ve görünür. Bu desteklenemezse yüzen moda düşülür. Spike'ta hangisinin olacağına karar verilir.
  - Görev çubuğu sol, sağ veya üst kenardaysa (Windows 11'de resmi olarak yalnızca alt kenar var; Windows 10 ve araçlarla diğerleri de mümkün): yüzen moda düşülür.
  - Tam ekran tespiti 18'deki gibi çalışır.
- **Yedek davranış:** Kenetlenme herhangi bir nedenle başarısız olursa widget yüzen moda geçer ve tek bir bildirim gösterilir ("Görev çubuğuna yerleşilemedi; widget yüzen modda"). Ayar `taskbar` olarak kalır; bir sonraki `TaskbarCreated` veya yeniden başlatmada tekrar denenir.
- **Etkileşim:** Çift tıklama paneli açar. Sağ tık menüsü 18'deki gibidir; "Mod: yüzen / görev çubuğu" öğesi eklenir. Görev çubuğu modunda sürükleme yalnızca yatay ofseti değiştirir.
- **Ayarlar:** `widget_mode = taskbar` seçilebilir hale gelir; yatay ofset ve hizalama ayarları eklenir. Fluent anahtarları `en` ve `tr` için eklenir.

## Kapsam dışı

- İkincil monitörlerin görev çubukları (`Shell_SecondaryTrayWnd`). Yalnızca birincil görev çubuğu desteklenir; talep gelirse ayrı bir adım olur.
- Windows 11 Widgets panosu (Adaptive Cards, MSIX gerektirir).
- Görev çubuğu düğmesinde ilerleme çubuğu (`ITaskbarList3`). Uygulamanın görev çubuğu düğmesi yoktur.
- Explorer'a DLL enjeksiyonu veya Explorer'ın içinde kod çalıştıran herhangi bir yöntem (Windhawk tarzı). Güvenlik ilkesine aykırıdır.

## Görevler

- [x] **Yöntem spike'ı** ve ADR-058'e ek.
- [x] **Görev çubuğu geometrisi:** `TrayNotifyWnd` okuması (`widget/system.rs`), saf konum hesabı (`widget/place.rs`) ve birim testleri. Test durumları: alt kenar, %100/%150, otomatik gizleme, ofset, sığmayan genişlik.
- [x] **Tek satır çizimi** ve görev çubuğu teması. Golden testler eklenir.
- [x] **Kenetlenme ve yeniden kenetlenme:** `TaskbarCreated`, `WM_DISPLAYCHANGE`, `WM_DPICHANGED`, `WM_SETTINGCHANGE` (görev çubuğu ayarları).
- [x] ~~**Z-sırası koruması**~~ gerekmedi: sahiplik z-sırasını koruyor; yalnızca yeniden kenetlenirken öne alınıyor (ADR-058).
- [x] **Yedek davranış** ve tek seferlik bildirim.
- [x] **Ayarlar** ve Fluent metinleri.
- [x] **Ölçüm:** Görev çubuğu modunda ek RAM ve 10 dk boşta CPU. Sonuçlar `docs/perf/` altına yazılır. WinEvent kancası CPU bütçesini etkilememeli.

## Teknik notlar ve tuzaklar

- **Windows 11 görev çubuğu XAML tabanlıdır.** `Shell_TrayWnd` içindeki eski çocuk pencere sınıfları (`ReBarWindow32`, `MSTaskSwWClass`) 24H2'de güvenilir değildir. Görev çubuğu düğmelerinin genişliğini okumaya çalışma; konum yalnızca `Shell_TrayWnd` ve `TrayNotifyWnd` dikdörtgenlerinden hesaplanır.
- **Görev çubuğunun kendisi topmost'tur.** Görev çubuğuna tıklanınca widget'ın arkada kalması en olası hatadır. Spike'ta bunu ilk iş dene.
- **Explorer yeniden başlatma testi** (`taskkill /f /im explorer.exe` + `start explorer`) kullanıcının masaüstünü kısa süre kapatır. **Yalnızca kullanıcının onayıyla** çalıştırılır.
- **Erişilebilirlik:** Ekran okuyucu için pencere başlığı ve `WM_GETOBJECT` ile basit bir ad verilir (ör. "TokenRadar: haftalık %34, Çarşamba yenilenir"). Metin Fluent'ten gelir.

## Kabul kriterleri

- [x] Görev çubuğu modunda widget bildirim alanının solunda görünür, görev çubuğuna tıklandığında kaybolmaz.
- [x] Explorer yeniden başlatıldığında birkaç saniye içinde yeniden kenetlenir.
- [x] Desteklenmeyen düzenlerde yüzen moda düşer ve bunu bir kez söyler. (Birim testli; canlı denenmedi.)
- [x] Açık ve koyu görev çubuğunda, %100 ve %150 ölçekte doğru çizilir. (Canlı yalnızca açık, %125; diğerleri golden.)
- [x] Ek RAM ve CPU 18'deki bütçe içinde kalır. Ölçüm `docs/perf/` altına yazılır.

## Doğrulama

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
pnpm -C app check
pnpm -C app test
pnpm dev   # Ayarlar → Widget → Mod: görev çubuğu
.\scripts\measure-memory.ps1 -ProcessName tokenradar-app -DurationSec 600 -Out docs/perf/raw/YYYY-MM-DD-widget-taskbar.csv
```

Görsel kontrol `CopyFromScreen` ile yapılır: görev çubuğu bölgesinin ekran görüntüsü. Kontrol listesi: açık ve koyu tema, ortalanmış ve sola yaslı simgeler, otomatik gizleme, Explorer yeniden başlatma (kullanıcı onayıyla).

## Devir notu

**Tarih:** 2026-11-11 · **Karar:** ADR-058 (Sonuçlar, Adım 18b) · **Ölçüm:** `docs/perf/2026-11-11-widget-taskbar.md` · **Ekran görüntüsü:** `img/18b-taskbar-en-light.png` (açık görev çubuğu, EN, iki satır)

### Yapılanlar
- **Yöntem spike'ı:** (a) sahibi `Shell_TrayWnd` olan topmost popup ile (b) `Shell_TrayWnd`'nin çocuğu olan pencere denendi. (b) görünmüyor (XAML içeriği örtüyor), (a) seçildi. Ayrıntılar ve ölçütler ADR-058'de.
- **Core `config/widget.rs`:** `TaskbarAlign` (`tray` varsayılan, `left`), `taskbar_align`, `taskbar_offset` (mantıksal px, 0–4000'e kırpılır). Eski config'ler varsayılanlarla okunur.
- **Uygulama `widget/`:**
  - `place.rs`: saf `Taskbar` (kenar, dikdörtgen, bildirim alanı, otomatik gizleme, ölçek), `Undockable` nedenleri, `dock` (bildirim alanının 8 px solu veya sol uç + ofset), `drag_docked` (sürüklemeyi boş aralıkta yataya kısıtlar), `dock_offset` (bırakılan yerden ofset). Test durumları: alt kenar, %100/%150, otomatik gizleme, yan kenar, ofset, sığmayan genişlik, sol hizalama.
  - `render.rs`: `Shape::Floating | Docked { height, compact, hover }`. Kenetli şerit görev çubuğu yüksekliğinde, dikeyde ortalı; zemin alfa 1 (görünmez ama fareyi alır), hover dolgusu, yüksek kontrastta plaka. Kompakt tek satır (`7g %34 ███░░ · 5s %72 ████░`). Zemin yokken zaman işaretinin halesi çubuğu keser.
  - `system.rs`: kenetlenme ve yeniden kenetlenme (`TaskbarCreated`, `WM_DISPLAYCHANGE`, `WM_DPICHANGED`, `WM_SETTINGCHANGE`, 5 sn'lik dikdörtgen yoklaması), sahiplik `GWLP_HWNDPARENT` ile, yeniden kenetlenince `HWND_TOPMOST`, `WM_MOVING` ile yatay sürükleme ve ofsetin kaydı, `WM_NCMOUSEMOVE`/`WM_NCMOUSELEAVE` ile hover, sağ tık menüsünde "Konum" alt menüsü, yüzen moda düşüş ve çalıştırma başına tek bildirim, pencere sahibiyle yok olursa yeniden kurulması. Pencere başlığı ekran okuyucu için Fluent'ten (`WidgetVisual::spoken`).
  - `fake.rs`/`mod.rs`: `:taskbar` son eki (`TOKENRADAR_FAKE_LIMITS=widget:both:taskbar`).
- **`notify.rs`:** başlık ve gövdeyi doğrudan alan `show_text`.
- **Ayarlar:** Genel → Masaüstü widget'ı: "Konum" segment kontrolü; görev çubuğundayken hizalama ve uzaklık seçimleri, opaklık yalnızca yüzen modda. Tam ekran anahtarı görev çubuğundayken işaretli ve kilitli. `TASKBAR_OFFSET_PRESETS`; sürüklemeyle gelen değer listeye eklenir.
- **Fluent (EN/TR):** `widget-menu-mode*`, `widget-undocked-*` (neden seçicili), `widget-a11y*`, `general-widget-mode*`, `general-widget-align*`, `general-widget-offset*`, `general-widget-fullscreen-docked-hint`.
- **Testler:** core ofset kırpma; yerleşim (8 yeni); render (42 yeni golden: `docked-<tema>-<durum>-<ölçek>.png`, 7 durum × 3 tema × 2 ölçek; yüzen golden'lar değişmedi) ve kenetli şerit için 3 davranış testi; ekran okuyucu adı.
- **Dokümanlar:** ADR-058 Sonuçlar (Adım 18b); design-language §4 görev çubuğu modu; perf raporu.

### Sapmalar
- **Z-sırası kancası yok.** Plan (a) için `SetWinEventHook` ile öne alma öngörüyordu. Spike'ta, kanca olmadan 90 sn ve 30 ön plan değişikliğinde widget hiç arkada kalmadı (kullanıcı gözle doğruladı). Tek istisna yeni bir sahip: yeniden kenetlenirken pencere bir kez öne alınıyor. Explorer yeniden başlatma testinde bu eksiklik görüldü ve düzeltildi.
- **Kanca yerine yoklama.** Görev çubuğu ve bildirim alanı dikdörtgenleri 5 sn'de bir okunuyor (tam ekran yoklamasıyla aynı zamanlayıcı). Bildirim alanı büyürse şerit en geç 5 sn sonra kayıyor.
- **Otomatik gizlemede yüzen moda düşülüyor** (plan iki yolu açık bırakmıştı).
- **Görev çubuğundayken tam ekran gizlemesi zorunlu**, opaklık seçimi gizli. Plan bunlardan söz etmiyordu.
- **Erişilebilirlik:** `WM_GETOBJECT` uygulanmadı; ad pencere başlığından geliyor (Windows'un varsayılan erişilebilirlik vekili başlığı okur). Ekran okuyucuyla denenmedi.
- **Kurulu uygulama** canlı denemeler ve ölçüm için kullanıcı onayıyla kapatıldı ve sonra yeniden başlatıldı. Explorer iki kez kullanıcı onayıyla yeniden başlatıldı.

### Doğrulama
- `cargo fmt --all -- --check` ve `cargo clippy --workspace --all-targets -- -D warnings` temiz. `cargo test --workspace`: 569 geçti, 4 ignored.
- `pnpm -C app check`, `lint` ve `test` temiz (133 test).
- Ölçüm (release, sahte mod, 10 dk): görev çubuğu modunda private WS +1,77 MB (bütçe ≤ 8 MB), boşta CPU +%0,023 (bütçe < %0,1). Aynı build ile yüzen mod karşılaştırması sistem belleği azaldığı için yarıda durdu; ayrıntı perf raporunda. Widget thread'inin bağlam değişimleri ölçülemedi.
- Canlı (`CopyFromScreen`, %125, açık görev çubuğu): bildirim alanının solunda iki satır, zemin görünmüyor. Explorer iki kez yeniden başlatıldı: görev çubuğu yokken yüzüyor, ≈ 5 sn içinde yeniden kenetleniyor; öne alma düzeltmesinden sonra saniyede bir alınan görüntülerde hep görünür. Görev çubuğuna tıklamalarda arkada kalmadığını kullanıcı spike'ta gözle doğruladı.

### Açık konular
- **Elle denenecekler:** hover vurgusu, görev çubuğu boyunca sürükleme ve ofsetin yeniden başlatmadan sonra geri gelmesi, sağ tık menüsünden mod değişimi, çift tıklama, ortalanmış simgelerle çakışma (hizalama ve ofsetle çözülebilmeli).
- **Canlı görülmeyenler:** koyu görev çubuğu, %100 ve %150 ölçek, yüksek kontrast ve kompakt tek satır yalnızca golden'larda görüldü. Görev çubuğu temasını veya ölçeği değiştirmek kullanıcının ayarlarını değiştirir. Otomatik gizleme ve yan kenar düşüşü birim testli, canlı denenmedi.
- **Bildirim alanının büyümesi** canlı gözlenemedi (geçici ikon taşma menüsüne gitti). Yoklama bunu kapsamalı.
- **Giriş kuyruğu bağı:** Süreçler arası sahiplik widget thread'ini Explorer'ın görev çubuğu thread'ine bağlayabilir; ölçülmedi. Explorer donarsa widget'ın da donup donmadığı denenmedi.
- **İkincil monitör görev çubukları** kapsam dışı kaldı (plan gereği).
- 18'den kalanlar sürüyor: uyanma tabanı, ikili boyutun 15 MB sınırı, kısayol çakışmasının ayarlarda gösterilmemesi, sanal masaüstleri.

### Sonraki adıma notlar
- Yeni bir görev çubuğu durumu (ör. ikincil monitör) `place::Taskbar`'a alan ekleyip `Undockable`'a neden ekleyerek gelir; konum hesabı saf ve testli kalmalı.
- Sahibin değiştiği her yerde `raise()` çağrılmalı; sahip değişimi z-sırasını kendiliğinden düzeltmiyor.
- Widget thread'i bloklanmamalı: görev çubuğu modunda Explorer'ın girdi işlemesini etkileyebilir.
