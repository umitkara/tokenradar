# Adım 10 — Tray paneli

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1 | 07, 08, 09 | 1-2 oturum | ⬜ |

## Amaç

Tray ikonuna tıklayınca açılan kompakt paneli yapmak. Bu, v0.1'in ana ekranıdır. Panel şu bilgileri gösterir:
- İkili halka ve tüm limitler: 5 saatlik pencere, haftalık, scoped limitler.
- Projeksiyon ve reset geri sayımları.
- Veri kaynağı ve tazelik bilgisi.
- Canlı oturumlar ve her birinin mevcut penceredeki ≈ payı.

Panel sıcakken anında, soğukken ≤ 400 ms içinde açılmalı. Kapatıldıktan sonra belleği geri vermeli.

## Önce oku

- `docs/design-language.md` §4, §5, §6, §7, §9
- `docs/architecture.md` §2 (pencere yaşam döngüsü), §7
- `docs/performance-budget.md` (panel satırları)
- Adım 06'daki `PanelSnapshot` tanımı, Adım 02 spike raporu

## Kapsam

- **Pencere davranışı:** Çerçevesiz, şeffaf, Acrylic, `skipTaskbar`, `alwaysOnTop`, yeniden boyutlandırılamaz. Tray'e göre konumlanır. Odak kaybında (blur) ve `Esc` ile gizlenir. `warm_ttl` (60 s) sonunda yok edilir.
- **İçerik:**
  - Üst bilgi: başlık, canlı oturum sayısı, ayarlar butonu.
  - Halka ve limitler bölümü.
  - Projeksiyon satırı.
  - Oturumlar listesi.
  - Alt bilgi: kaynak rozeti, son güncelleme, "Yenile", "Bildirimleri sustur".
- **Durumlar:** İlk indeksleme sürüyor; limit verisi yok (CTA'lar); bayat veri; token bayat; çevrimdışı; şema sapması.
- **Olaylar:** Panel görünürken `LimitsUpdated` ve `LiveSessionsChanged` olaylarına abone olunur. Panel gizlenince abonelik bırakılır.

## Kapsam dışı

- "Detaylar" (ana pencere) butonu v0.2'de etkinleşir; şimdilik gizli.
- Ayarlar sayfası ve onboarding (Adım 11). CTA'lar şimdilik ayarlar penceresinin ilgili sekmesini açan komutları çağırır; sekmeler Adım 11'de dolacak.

## Görevler

### Pencere

- [ ] Konumlama:
  - `tauri-plugin-positioner` (`TrayBottomCenter` vb.) **veya** özel hesaplama kullanılır.
  - Görev çubuğunun kenarı (alt/üst/sol/sağ) `SHAppBarMessage(ABM_GETTASKBARPOS)` ile bulunur.
  - Panel monitörün çalışma alanı (work area) içine sıkıştırılır.
  - Çoklu monitör ve DPI desteği: panel, tray ikonunun bulunduğu monitörde açılır.
- [ ] Gösterme ve gizleme:
  - Sol tık panel açıksa gizler, kapalıysa gösterir.
  - **Blur ile gizleme:** Tray'e tıklama da bir blur olayı üretir. Gizle → hemen tekrar göster döngüsünü önlemek için 200 ms'lik bir "yeni gizlendi" penceresi tutulur.
- [ ] Yaşam döngüsü:
  - Gizlenince `warm_ttl` zamanlayıcısı başlar, süre dolunca pencere `destroy()` edilir.
  - Gizli ama sıcak durumdayken `MemoryUsageTargetLevel = Low` uygulanır (Adım 02'de faydalı bulunduysa).
- [ ] İlk boyama titremesini önleme: Pencere `visible: false` olarak oluşturulur. Frontend ilk snapshot'ı alıp boyadıktan sonra `ready` olayı gönderir, ardından Rust pencereyi gösterir.
- [ ] Önceden ısıtma (opsiyonel, ölçüme bağlı): Kullanıcı tray ikonunun üzerine geldiğinde (hover olayı) webview arka planda oluşturulmaya başlanır. Bu, soğuk açılışı gizleyebilir. Fayda ölçülür, karar devir notuna yazılır.

### İçerik

- [ ] **Üst bilgi:** "TokenRadar", hesap etiketi ve plan (örneğin "Max 20x"), `● 2 aktif`, ⚙ butonu.
- [ ] **Halka bölümü:** `RingGauge` (dış: haftalık, iç: 5s) ve sağında limit satırları:
  - 5 saatlik pencere: `%72 · 1 sa 48 dk sonra yenilenir`
  - Haftalık: `%34 · Çar 10:59'da yenilenir`
  - Scoped limitler (her biri): `Fable · haftalık %0 · …` (girintili)
  - Extra usage etkinse küçük bir bilgi satırı.
- [ ] **Projeksiyon satırı:** Durumlardan biri gösterilir:
  - `▲ Bu hızla 38 dk içinde dolacak — reset 1 sa 52 dk sonra` (crit)
  - `Bu hızla reset'e kadar ≈ %86` (ok/warn)
  - "Tüketim yok" (rate ≈ 0)
  
  Güven düşükse metin "≈" ile ve soluk renkte gösterilir.
- [ ] **Oturumlar:**
  - Canlı oturumlar mevcut penceredeki ≈ paya göre sıralanır, en fazla 5 gösterilir (fazlası "+3 daha" olarak).
  - Satır içeriği: durum noktası, ad (`session_name` → başlık → proje adı), proje, `≈ %9 bu pencere`, `12,4k tok/dk`.
  - Canlı oturum yoksa: "Şu an aktif oturum yok" + bu pencerede en çok tüketen 3 oturum (canlı olmayanlar soluk gösterilir).
- [ ] **Alt bilgi:** Kaynak rozeti (`canlı` / `API` / `tahmini`) + "2 dk önce güncellendi", "Yenile" butonu (S2 onaylıysa ve son istekten 60 s geçtiyse aktif), "Sustur" butonu.
- [ ] **Boş ve hata durumları:** `design-language.md` §9'daki tablo uygulanır. "Limit verisi yok" durumunda iki CTA bulunur: **Statusline köprüsünü kur (önerilen)** ve **API erişimine izin ver**. İkisi de ayarlar penceresini ilgili sekmede açar.
- [ ] **Klavye:** `Esc` paneli kapatır, `Tab` sırası mantıklıdır, `Enter`/`Space` butonları etkinleştirir. Panel açılınca odak ilk etkileşimli öğeye gelir.
- [ ] Yükseklik içeriğe göre ayarlanır (en fazla 600 px). İçerik değişince pencere `ResizeObserver` → Rust `set_size` ile yeniden boyutlanır. Konumlama yeni boyuta göre tekrar hesaplanır.

### Ölçüm ve test

- [ ] Ölçümler (`scripts/measure-memory.ps1` + zaman damgalı log):
  - Soğuk açılış (≤ 400 ms), sıcak açılış (≤ 50 ms)
  - Panel açıkken bellek (≤ 130 MB)
  - Kapattıktan `warm_ttl + 10 s` sonra bellek (≤ 30 MB)
- [ ] Manuel test matrisi:
  - Görev çubuğu alt / üst / sol / sağ
  - %100 / %150 / %200 ölçek
  - İki monitör
  - Açık / koyu tema
  - EN / TR
  
  Ekran görüntüleri devir notuna eklenir.
- [ ] Vitest: snapshot → görünüm modeli dönüşüm yardımcıları (sıralama, "+N daha", projeksiyon metni seçimi).

## Teknik notlar ve tuzaklar

- **Odak çalma:** Windows'ta arka plandaki bir süreç pencereyi öne getiremeyebilir (foreground lock). Tray tıklaması kullanıcı girdisi olduğu için genelde sorun çıkmaz. Sorun çıkarsa `AllowSetForegroundWindow` veya `SetForegroundWindow` geçici çözümünü belgele.
- **Acrylic ve gölge:** Çerçevesiz pencerede Windows 11'in yuvarlak köşe ve gölgesi için `DwmSetWindowAttribute(DWMWA_WINDOW_CORNER_PREFERENCE)` gerekebilir.
- **Geri sayımlar:** Paylaşılan ticker kullanılır (Adım 09). Panel gizlenince ticker durur.
- Panel her açılışta `get_panel_snapshot` ile tam durumu çeker, ardından olaylarla güncellenir. Olay kaçırılırsa (lagged) tam durum tekrar çekilir.

## Kabul kriterleri

- [ ] Gecikme ve bellek bütçeleri ölçülmüş ve sağlanmıştır (rapor `docs/perf/`).
- [ ] Test matrisinde konumlama doğrudur, panel ekran dışına taşmaz.
- [ ] Tüm boş ve hata durumları gözle doğrulanmıştır. "Veri yok" durumunda sahte yüzde gösterilmez.
- [ ] Klavyeyle tam kullanım mümkündür. Ekran okuyucu (Narrator) halka bilgisini tam cümleyle okur.

## Doğrulama

```powershell
pnpm dev
.\scripts\measure-memory.ps1 -ProcessName tokenradar-app -DurationSec 120   # panel aç/kapat senaryosu
pnpm -C app test
```

## Devir notu

_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
