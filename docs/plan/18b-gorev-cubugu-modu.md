# Adım 18b — Widget'ın görev çubuğu modu

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.3 (13'ten önce yapılabilir) | 18 | 1 oturum | ⬜ |

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

- [ ] **Yöntem spike'ı** ve ADR-058'e ek.
- [ ] **Görev çubuğu geometrisi:** `TrayNotifyWnd` okuması (`widget/system.rs`), saf konum hesabı (`widget/place.rs`) ve birim testleri. Test durumları: alt kenar, %100/%150, otomatik gizleme, ofset, sığmayan genişlik.
- [ ] **Tek satır çizimi** ve görev çubuğu teması. Golden testler eklenir.
- [ ] **Kenetlenme ve yeniden kenetlenme:** `TaskbarCreated`, `WM_DISPLAYCHANGE`, `WM_DPICHANGED`, `WM_SETTINGCHANGE` (görev çubuğu ayarları).
- [ ] **Z-sırası koruması** (yöntem (a) seçilirse).
- [ ] **Yedek davranış** ve tek seferlik bildirim.
- [ ] **Ayarlar** ve Fluent metinleri.
- [ ] **Ölçüm:** Görev çubuğu modunda ek RAM ve 10 dk boşta CPU. Sonuçlar `docs/perf/` altına yazılır. WinEvent kancası CPU bütçesini etkilememeli.

## Teknik notlar ve tuzaklar

- **Windows 11 görev çubuğu XAML tabanlıdır.** `Shell_TrayWnd` içindeki eski çocuk pencere sınıfları (`ReBarWindow32`, `MSTaskSwWClass`) 24H2'de güvenilir değildir. Görev çubuğu düğmelerinin genişliğini okumaya çalışma; konum yalnızca `Shell_TrayWnd` ve `TrayNotifyWnd` dikdörtgenlerinden hesaplanır.
- **Görev çubuğunun kendisi topmost'tur.** Görev çubuğuna tıklanınca widget'ın arkada kalması en olası hatadır. Spike'ta bunu ilk iş dene.
- **Explorer yeniden başlatma testi** (`taskkill /f /im explorer.exe` + `start explorer`) kullanıcının masaüstünü kısa süre kapatır. **Yalnızca kullanıcının onayıyla** çalıştırılır.
- **Erişilebilirlik:** Ekran okuyucu için pencere başlığı ve `WM_GETOBJECT` ile basit bir ad verilir (ör. "TokenRadar: haftalık %34, Çarşamba yenilenir"). Metin Fluent'ten gelir.

## Kabul kriterleri

- [ ] Görev çubuğu modunda widget bildirim alanının solunda görünür, görev çubuğuna tıklandığında kaybolmaz.
- [ ] Explorer yeniden başlatıldığında birkaç saniye içinde yeniden kenetlenir.
- [ ] Desteklenmeyen düzenlerde yüzen moda düşer ve bunu bir kez söyler.
- [ ] Açık ve koyu görev çubuğunda, %100 ve %150 ölçekte doğru çizilir.
- [ ] Ek RAM ve CPU 18'deki bütçe içinde kalır. Ölçüm `docs/perf/` altına yazılır.

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

_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
