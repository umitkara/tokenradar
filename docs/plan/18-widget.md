# Adım 18 — Mini widget (kompakt şerit)

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.3 | 12 | 1 oturum | ⬜ |

## Amaç

Masaüstünde her zaman üstte duran, sürüklenebilir, isteğe bağlı bir kompakt şerit yapmak:
- İki çubuk (5s ve haftalık), yüzde ve geri sayım.
- Tam ekran uygulamalarda otomatik gizlenme.
- İsteğe bağlı tıklama geçirgenliği (click-through).

Bellek bütçesi aşılırsa widget native çizime geçirilir (ADR-011).

## Önce oku

- `docs/design-language.md` §1, §4 (Widget)
- `docs/decisions.md`: ADR-003, ADR-011, ADR-019
- `docs/performance-budget.md` (widget satırı)
- Adım 02 spike raporu (ikinci pencerenin ek maliyeti)

## Kapsam

- `widget.html` + Tauri penceresi:
  - Şeffaf, çerçevesiz, `alwaysOnTop`, `skipTaskbar`, odak almayan.
  - Boyut 220×44.
  - Acrylic zemin; Windows 10'da opak.
- **Etkileşim:**
  - Sürükleme ile taşınır; ekran kenarlarına ve köşelere yapışır (snap).
  - Konum monitör bazında hatırlanır.
  - Çift tıklama paneli açar.
  - Sağ tık menüsü: Gizle, Tıklama geçirgenliği, Opaklık.
- **Tam ekran tespiti** ve otomatik gizleme.
- **Tıklama geçirgenliği:** `set_ignore_cursor_events`. Geri almak için tray menüsü ve global kısayol kullanılır.
- **Ayarlar:** Widget aç/kapat, opaklık, tıklama geçirgenliği, "tam ekranda gizle", içerik varyantı (yalnızca 5s / ikisi).
- **Ölçüm ve ADR-011 kararı.** Gerekirse native çizim yolu.

## Kapsam dışı

- Kare halka veya tek satır hap varyantları. Röportajda kompakt şerit seçildi; diğerleri talep gelirse eklenir.

## Görevler

- [ ] **Pencere:**
  - Tauri pencere yapılandırması.
  - Odak almama: Windows'ta `WS_EX_NOACTIVATE` + `WS_EX_TOOLWINDOW` (`with_webview`/`hwnd` ile). Böylece widget'a tıklamak, kullanıcının yazı yazdığı uygulamanın odağını çalmaz.
- [ ] **İçerik:**
  - `LimitBar` bileşeninin kompakt varyantı.
  - Satır 1: `5s ████████░░ %72 · 1:48`. Satır 2: `7g ███░░░░░░░ %34 · Çar`.
  - Durum renkleri, "≈" ve bayatlık göstergeleri paneldekiyle aynı.
  - Yalnızca `LimitsUpdated` olayına abone olunur.
  - Geri sayım dakika çözünürlüğündedir; widget'ta saniye gösterilmez, bu yüzden uyanma sayısı azdır.
- [ ] **Sürükleme ve yapışma:**
  - `data-tauri-drag-region` ile sürükleme.
  - Bırakınca en yakın kenara veya köşeye 16 px eşikle yapışır.
  - Konum `(monitör kimliği, göreli konum)` olarak saklanır. Monitör yoksa birincil monitörün sağ alt köşesi kullanılır.
- [ ] **Tam ekran tespiti:**
  - Widget görünürken 5 sn'de bir `SHQueryUserNotificationState` çağrılır. `QUNS_BUSY`, `QUNS_RUNNING_D3D_FULL_SCREEN` veya `QUNS_PRESENTATION_MODE` dönerse widget gizlenir, durum değişince geri gelir.
  - Polling yalnızca widget **etkinken** yapılır.
- [ ] **Tıklama geçirgenliği:**
  - Açıkken widget fare olaylarını almaz.
  - Kapatmak için: tray menüsü ("Widget'ı etkileşimli yap") veya global kısayol (varsayılan `Ctrl+Alt+Shift+R`; ayarlanabilir; `tauri-plugin-global-shortcut`).
  - Kısayol yalnızca widget etkinken kaydedilir.
- [ ] **Ölçüm** (`scripts/measure-memory.ps1 -Scenario widget`):
  - Widget açıkken ve tüm diğer pencereler kapalıyken toplam RAM.
  - Widget'ın **ek** maliyeti (tray-only değerinden farkı).
  - 10 dk boşta CPU.
- [ ] **ADR-011 kararı:**
  - **Ek maliyet ≤ 45 MB ise:** WebView widget kalır; ADR'ye "onaylandı" notu düşülür.
  - **Ek maliyet > 45 MB ise:**
    1. Widget native olarak çizilir. Yöntem: `tiny-skia` ile bitmap çizimi + `UpdateLayeredWindow` (per-pixel alpha) ile layered window. Metin `cosmic-text` veya `fontdue` ile rasterleştirilir; gömülü mono font kullanılır.
    2. Etkileşim (sürükleme, yapışma, menü) Win32 mesajlarıyla yeniden yazılır.
    3. Ölçüm tekrarlanır ve ADR güncellenir.
    4. Bu durumda adım **2 oturuma** uzayabilir; kullanıcıya bildirilir.
- [ ] **Testler:** Yapışma geometrisi (birim testi), konum geri yükleme (monitör kaldırıldığında yedek konum), tam ekran durum makinesi (sahte API).

## Teknik notlar ve tuzaklar

- **`alwaysOnTop` ve tam ekran oyunlar:** Exclusive fullscreen modunda çalışan oyunlarda topmost pencereler sorun çıkarabilir. Tam ekran tespiti bu yüzden zorunlu.
- **Şeffaf ve odak almayan WebView2:** WebView2 bazen `WS_EX_NOACTIVATE` stiline rağmen tıklamada odak alabiliyor. Test et; gerekirse `WM_MOUSEACTIVATE` → `MA_NOACTIVATE` alt sınıflaması (subclassing) yap.
- **Sanal masaüstleri:** Widget tüm sanal masaüstlerinde görünmeli mi? Varsayılan: evet (`IVirtualDesktopManager` ile pin yapılmaz; topmost tool window çoğu durumda yeterli). Davranışı test edip belgele.

## Kabul kriterleri

- [ ] Widget odak çalmadan çalışır, kenarlara yapışır, konumu hatırlanır.
- [ ] Tam ekran video ve sunum modlarında otomatik gizlenir.
- [ ] Tıklama geçirgenliği açılıp kapatılabilir; kısayol ve tray menüsü çalışır.
- [ ] Bellek ve CPU ölçümleri raporlanmış, ADR-011 kararı kayda geçirilmiştir.

## Doğrulama

```powershell
pnpm dev   # Ayarlar → Widget'ı aç
.\scripts\measure-memory.ps1 -ProcessName tokenradar-app -Scenario widget -DurationSec 600 -Cpu
```

## Devir notu

_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
