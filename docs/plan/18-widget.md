# Adım 18 — Native kompakt şerit (yüzen mod)

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.3 (13'ten önce yapılabilir) | 12 | 1 oturum | ⬜ |

> **Yeniden planlandı (2026-11-11, ADR-058).** Kullanıcı, Adım 13'ü (18 Kasım) beklerken Windows'a özel bir görev çubuğu widget'ı istedi. Widget artık WebView ile değil, baştan native çiziliyor. Görev çubuğu modu ayrı bir adım: [18b](18b-gorev-cubugu-modu.md). Bu adım, 18b'nin üstüne kurulacağı ortak pencere ve çizim temelini yapıyor.

## Amaç

Masaüstünde her zaman üstte duran, sürüklenebilir, isteğe bağlı bir kompakt şerit yapmak:
- Haftalık çubuk; 5 saatlik pencere varsa ikinci çubuk. Yüzde ve geri sayım.
- Native çizim: `tiny-skia` bitmap + per-pixel alpha layered window. WebView yok.
- Tam ekran uygulamalarda otomatik gizlenme ve isteğe bağlı tıklama geçirgenliği.
- Varsayılan olarak kapalı (ADR-003).

## Önce oku

- `docs/decisions.md`: ADR-003, ADR-011 (yerini aldı), **ADR-058**, ADR-019, ADR-030 (tray çizimi ve ortam izleme), ADR-041 (5s penceresi yok), ADR-044 (zaman temposu)
- `docs/design-language.md` §1, §4 (Widget, Tray ikonu)
- `docs/performance-budget.md` (widget satırı)
- Kod: `app/src-tauri/src/tray/render.rs` (`render`, `Palette`, `Theme`, `to_rgba`), `tray/system.rs` (`watch`, `tray-env` thread'i, `environment`), `tray/visual.rs` (`TrayModel`, `TrayVisual`), `panel/place.rs` + `panel/system.rs` (çalışma alanı, DPI), `prefs.rs` (`Prefs`, `PrefsView`)

## Kapsam

- **Widget modülü** (`app/src-tauri/src/widget/`), yapısı tray ve panel modülleriyle aynı:
  - `model.rs`: saf görünüm modeli. `TrayModel` ile aynı kaynaktan beslenir; UI tarafında hesap yapılmaz.
  - `render.rs`: saf `tiny-skia` çizimi. Renkler ve eşikler `tray/render.rs` ile paylaşılır.
  - `place.rs`: yapışma ve konum geri yükleme geometrisi. Saf, birim testli.
  - `system.rs`: Win32 penceresi, mesaj döngüsü, ortam olayları.
  - `fake.rs`: galeri ve testler için sahte sahneler (`panel/fake.rs` deseni).
- **Pencere:** Kendi thread'inde (`widget`) `GetMessageW` ile uyuyan tek bir üst düzey pencere.
  - Stiller: `WS_POPUP`, `WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST`.
  - Çizim `UpdateLayeredWindow` (premultiplied BGRA) ile yapılır. `WM_PAINT` kullanılmaz.
  - Tauri penceresi değildir. Widget kapalıyken thread yoktur; kapatınca thread ve pencere yok edilir.
- **Metin:** Adımın ilk işi küçük bir spike ile iki yol arasında seçim yapmaktır (aşağıya bakın). Seçim ADR-058'e eklenir.
- **İçerik:**
  - Satır 1: `7g ███░░░░░░░ %34 · Çar`. Satır 2 (yalnızca 5s penceresi varsa): `5s ████████░░ %72 · 1:48`.
  - Eşleme her yüzeyde aynıdır: üst çubuk haftalık, alt çubuk 5 saatlik (design-language §1).
  - 5s penceresi yoksa tek satır çizilir (ADR-041). Boşluk bir tahminle doldurulmaz.
  - Durum renkleri (75/90), `≈` ve bayatlık göstergeleri paneldekiyle aynıdır. Tempo çentiği (ADR-044) isteğe bağlıdır.
  - Geri sayım dakika çözünürlüğündedir. Yeniden çizim yalnızca model değişince veya dakika dönünce yapılır.
- **Etkileşim:**
  - Sürükleme: `WM_NCHITTEST` → `HTCAPTION`.
  - Bırakınca en yakın kenara veya köşeye 16 px eşikle yapışır (`WM_EXITSIZEMOVE`).
  - Konum `(monitör kimliği, göreli konum)` olarak saklanır. Monitör yoksa birincil monitörün sağ alt köşesi kullanılır.
  - Çift tıklama paneli açar.
  - Sağ tık menüsü (`TrackPopupMenu`, Fluent metinleriyle): Gizle, Tıklama geçirgenliği, Opaklık, Ayarlar.
- **Odak:** Widget'a tıklamak, kullanıcının yazı yazdığı uygulamanın odağını çalmaz (`WS_EX_NOACTIVATE`, `WM_MOUSEACTIVATE` → `MA_NOACTIVATE`).
- **Tam ekran tespiti:** Widget görünürken 5 sn'de bir `SHQueryUserNotificationState` çağrılır. `QUNS_BUSY`, `QUNS_RUNNING_D3D_FULL_SCREEN` veya `QUNS_PRESENTATION_MODE` dönerse widget gizlenir, durum değişince geri gelir. Zamanlayıcı yalnızca widget **etkinken** çalışır.
- **Tıklama geçirgenliği:** `WS_EX_TRANSPARENT`. Geri almak için tray menüsü ("Widget'ı etkileşimli yap") ve global kısayol (varsayılan `Ctrl+Alt+Shift+R`, ayarlanabilir; `RegisterHotKey`). Kısayol yalnızca widget etkinken kaydedilir.
- **Ortam:** Tema (`SystemUsesLightTheme`), DPI (`WM_DPICHANGED`), yüksek kontrast ve monitör değişiminde yeniden çizim yapılır. Değer aynıysa çizim yapılmaz. Polling yoktur (ADR-030 deseni).
- **Zemin:** Yarı saydam, köşe yarıçapı 8 px, tema rengi. Bu zemin `tiny-skia` ile pikselde çizilir. DWM Acrylic, layered pencerede kullanılamadığı için kapsam dışıdır (ADR-058).
- **Ayarlar:** Widget aç/kapat, opaklık, tıklama geçirgenliği, "tam ekranda gizle", tempo çentiği. Ayarlar `Prefs`'e eklenir; ayarlar sayfasında (Svelte) bir bölüm olarak görünür. Metinler `locales/{en,tr}` altındadır.
- **Mod alanı:** `widget_mode = floating | taskbar` ayarı bu adımda eklenir; `taskbar` değeri 18b'ye kadar seçilemez.
- **Ölçüm** ve ADR-058'e sonuç.

## Kapsam dışı

- Görev çubuğu modu → [18b](18b-gorev-cubugu-modu.md).
- WebView widget. ADR-058 ile kaldırıldı.
- Kare halka veya tek satır hap varyantları. Röportajda kompakt şerit seçildi; diğerleri talep gelirse eklenir.
- macOS ve Linux (Adım 21). Modül `cfg(windows)` arkasında kalır.
- Birden fazla hesap (Adım 19).

## Görevler

- [ ] **Metin spike'ı (ilk iş, ≤ 1 saat):**
  - (a) DirectWrite: sistem fontu (`Segoe UI Variable`, yoksa `Segoe UI`), grayscale antialias, bir DIB'e çizip `tiny-skia` bitmap'iyle birleştirme. Gömülü font yoktur, ikili boyut artmaz, Windows ile tutarlı görünür.
  - (b) `fontdue` + gömülü, alt kümesi çıkarılmış mono font (`scripts/subset-font.py` var). Tamamen saf Rust'tır, testlerde golden görüntü verir.
  - Karşılaştır: %100/%150'de netlik, Türkçe karakterler (`Çar`, `İ`), ikili boyut farkı, ek RAM. Seçimi ve gerekçesini ADR-058'e yaz.
- [ ] **Model ve çizim:** `widget/model.rs` + `widget/render.rs`. `Palette` ve eşikler `tray/render.rs` ile paylaşılır (gerekirse ortak bir modüle taşınır, davranış değişmez). Golden testler: koyu/açık/yüksek kontrast × %100/%150 × (tek satır, iki satır, bayat, tahmin).
- [ ] **Pencere ve mesaj döngüsü:** `widget/system.rs`. Açma ve kapatma `AppHandle`'dan bir kanal ile yapılır. Model güncellemeleri tray'in güncellendiği yerden aynı modelle gönderilir.
- [ ] **Sürükleme ve yapışma:** `widget/place.rs` saf geometri (16 px eşik, kenar/köşe, çalışma alanına sıkıştırma). Konum `Prefs`'te saklanır.
- [ ] **Menü, çift tıklama, odak çalmama.**
- [ ] **Tam ekran durum makinesi:** API bir trait arkasındadır; sahte uygulamayla test edilir.
- [ ] **Tıklama geçirgenliği + global kısayol + tray menüsü öğesi.**
- [ ] **Ayarlar:** `Prefs` alanları, `PrefsView`, ayarlar sayfası bölümü, `en` ve `tr` Fluent anahtarları, `tauri-specta` bağlamaları.
- [ ] **Galeri:** Sahte sahneler (`widget:weekly`, `widget:both`, `widget:stale`, `widget:estimate`) ile ekran görüntüsü.
- [ ] **Ölçüm:**
  - `scripts/measure-memory.ps1 -ProcessName tokenradar-app` ile önce tray-only, sonra widget açık durumda ölçüm alınır.
  - Sonuçlar: ek private working set, 10 dk boşta CPU, saniyedeki uyanma sayısı.
  - Rapor `docs/perf/YYYY-MM-DD-widget.md` olarak kaydedilir. Kişisel kullanım sayıları yazılmaz.

## Teknik notlar ve tuzaklar

- **Premultiplied alpha:** `UpdateLayeredWindow` premultiplied BGRA bekler. `tiny-skia` premultiplied RGBA üretir, bu yüzden yalnızca kanal sırası değişir. `to_rgba` ise demultiply yapar, burada kullanılmamalı.
- **DPI:** Pencere per-monitor v2 DPI farkındalığıyla oluşturulur (süreç zaten öyle). Monitör değişince sıra konum → boyut → konumdur (ADR-032 deseni).
- **`alwaysOnTop` ve tam ekran oyunlar:** Exclusive fullscreen modunda topmost pencereler sorun çıkarabilir. Tam ekran tespiti bu yüzden zorunlu.
- **Sanal masaüstleri:** Topmost tool window çoğu durumda tüm masaüstlerinde görünür. Davranışı test edip devir notuna yaz.
- **Explorer yeniden başlatma** yüzen modu etkilemez; 18b için önemlidir.
- **Uyanma sayısı:** Dakika zamanlayıcısı `SetTimer` ile bir sonraki dakika sınırına kurulur; saniyede bir uyanılmaz.

## Kabul kriterleri

- [ ] Widget açılıp kapatılabilir. Kapalıyken thread ve pencere yoktur.
- [ ] Odak çalmadan çalışır, kenarlara yapışır, konumu monitör bazında hatırlanır.
- [ ] 5s penceresi yoksa tek satır gösterir; tahmin `≈` ile işaretlidir.
- [ ] Tam ekran video ve sunum modlarında otomatik gizlenir.
- [ ] Tıklama geçirgenliği açılıp kapatılabilir; kısayol ve tray menüsü çalışır.
- [ ] Tema, DPI ve yüksek kontrast değişince doğru çizilir.
- [ ] Ek RAM ≤ 8 MB, boşta CPU < %0,1. Ölçüm `docs/perf/` altında, sonuç ADR-058'de.

## Doğrulama

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
pnpm -C app check
pnpm -C app test
pnpm dev   # Ayarlar → Widget'ı aç
.\scripts\measure-memory.ps1 -ProcessName tokenradar-app -DurationSec 600 -Out docs/perf/raw/YYYY-MM-DD-widget.csv
```

Görsel kontrol: Widget native bir penceredir, CDP ile görülemez. Ekran görüntüsü `CopyFromScreen` ile alınır. Kontrol listesi: açık ve koyu tema, %100 ve %150 ölçek, tek ve iki satır, yüksek kontrast.

## Devir notu

_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
