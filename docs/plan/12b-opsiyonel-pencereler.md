# Adım 12b — Opsiyonel limit pencereleri: çekirdek, tray ve panel

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1.x | 12 | 1 oturum | ✅ |

## Amaç

Anthropic, 30 Ekim 2026'da Max 20x planının adını **Ultra** olarak değiştirdi ve **doğrulanmış** Ultra hesaplarında 5 saatlik pencereyi kaldırdı. Pro, Max ve doğrulanmamış Ultra hesaplarında pencere devam ediyor. Kullanıcının kendi verisinde, 5 Kasım'dan beri statusline payload'unda `five_hour` hiç görünmüyor ve `limit_snapshots` tablosunda `session` kaydı yok.

TokenRadar şu ana kadar her hesapta 5 saatlik pencere olduğunu varsaydı. Bu adım, bir limit penceresinin hesaba göre **var olmayabileceğini** domain'e ekler. Ayrıca tahmin motorunun olmayan bir pencere için `≈%` üretmemesini sağlar ve tray ile paneli buna uyarlar (ADR-041).

## Önce oku

- `docs/decisions.md`: ADR-041, ADR-008
- `docs/architecture.md` §8 (limit motoru, füzyon kuralı)
- `docs/data-sources.md` S1, S2
- `docs/design-language.md`: tray ikonu, panel bileşenleri
- Adım 08 ve 10'un devir notları, Adım 12'nin devir notu

## Kapsam

- Domain: hesap başına pencere durumu üç değerli olur: **var** (ölçüm var), **bilinmiyor** (henüz ölçüm yok), **yok** (bu hesapta bu limit yok).
- "Yok" kararının kuralı (ADR-041):
  - S2 yanıtında `session` kaydı yoksa veya
  - payload'da `seven_day` varken `five_hour` art arda N kez (öneri: 3) yoksa
  - pencere **yok** sayılır. İlk `five_hour` gözleminde karar geri alınır (hesap doğrulanmamış Ultra'ya dönebilir).
- `LimitEngine`: **yok** durumundaki pencere için tahmin üretmez ve bildirim eşiği tetiklemez.
- IPC `StatusReply` / `LimitInfo`: pencerenin durumu taşınır. CLI `status`/`limits` çıktısı "bu hesapta yok" gösterir.
- Tray: 5 saatlik pencere yoksa ikon tek halkaya (haftalık) iner. Menüde 5 saatlik satır gizlenir.
- Panel: 5 saatlik kart gizlenir. Haftalık ve scoped limitler öne çıkar. Bilgilendirme metni Fluent'ten gelir.
- `render.rs` (statusline v1): payload'da `five_hour` yoksa artık uygulamanın tahmini 5 saatlik değerine düşmez. Pencere **yok** ise segment hiç gösterilmez. Satırın geri kalanı Adım 12c'de yeniden tasarlanır.

## Kapsam dışı

- Statusline v2'nin görsel tasarımı ve özelleştirilmesi (Adım 12c).
- Claude Code mod'u (Adım 12d).
- Kalibrasyon. Adım 13'ün `k_session` katsayısı, pencere olmayan hesaplarda üretilmez. Bu adım 13'te ele alınır.
- Plan adının (Ultra / Max 20x) UI'da gösterilmesi.

## Görevler

- [x] ADR-041 kabul edildi (2026-11-07). N değeri ve kalıcılık kararı netleşince ADR-041'e "Uygulama notu" olarak ekle.
- [x] Domain: `WindowPresence` (ad öneri) ve hesap başına saklanması. Kalıcılık gerekip gerekmediğine karar ver (yeniden başlatmada "bilinmiyor"dan başlamak kabul edilebilir mi?).
- [x] Füzyon ve limit motoru: kural, testler (S1-only, S2-only, ikisi, geri dönüş).
- [x] IPC sürümü: alan eklemesi geriye uyumlu mu? Değilse `PROTOCOL_VERSION` artışı ve eski CLI ile davranış.
- [x] Tray: tek halka ikonu (design-language'e uygun), menü.
- [x] Panel: kart düzeni, Fluent metinleri (`en`, `tr`).
- [x] `render.rs`: tahmini 5 saatlik değere düşmeyi kaldır, testleri güncelle.
- [x] Sentetik fixture: `five_hour` içermeyen statusline payload'u ve `session` içermeyen API yanıtı (`testdata/README.md` kurallarına uygun).

## Teknik notlar ve tuzaklar

- **Veri haftası sürüyor.** Kurulu v0.1.0'a ve `%LOCALAPPDATA%\TokenRadar` altındaki gerçek veriye dokunma. Bütün denemeler portable kopyayla yapılır (bkz. bellek: CLI çalıştırmaları gerçek veriye dokunur).
- `rate_limits` yalnızca oturumdaki ilk API yanıtından sonra gelir. Bu yüzden `seven_day` olmadan `five_hour`'un yokluğu hiçbir şey kanıtlamaz.
- `spend_limit` (gateway) için de aynı üç değerli model geçerli olmalı. Kod bunu genel tutmalı, `five_hour`'a özel olmamalı.

## Kabul kriterleri

- `five_hour` içermeyen payload akışında tray, panel, CLI ve statusline hiçbir yerde 5 saatlik `%` veya `≈%` göstermez.
- `five_hour` tekrar geldiğinde pencere bir sonraki güncellemede geri gelir.
- Mevcut testler ve yeni testler geçer.

## Doğrulama

- `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`
- `pnpm -C app check`, `pnpm -C app test`
- Portable kopyayla tray ve panelin ekran görüntüsü (CDP), iki senaryo: pencere var / yok.

## Devir notu

**Tarih:** 2026-11-07 · **Dal:** `step/12b-optional-windows` · **Karar:** ADR-041 (uygulama notu)

#### Kullanıcı kararları
- **"Yok" kuralı:** N = 3, ama yalnızca **taze** statusline payload'ları sayılır, yani başka bir limitin okumasının değiştiği payload'lar. Sebep: Claude Code reset'ten sonra `five_hour`'u düşürüyor ve statusline'ı yeni API yanıtı olmadan da çağırıyor; ADR'deki düz kural boştaki Pro/Max hesaplarında yanlış alarm verirdi.
- **Kalıcılık:** Karar `kv` tablosunda saklanır (`limits.absent.<account_id>`).
- **Tek halka:** Kalın tek halka. Kalınlık dış halka + boşluk + iç halka kadar (16 px'te 6 px). Kullanıcıya 16 px'te ikonun pasta grafiğine benzediği gösterildi; kullanıcı bu geometride kalınmasını istedi.
- **Oturum payı:** 5 saatlik penceresi olmayan hesaplarda pay boş kalır. Haftalık paya göre atıf Adım 14'e bırakıldı (aşağıda).

#### Yapılanlar
- **Domain:** `WindowPresence { Present, Unknown, Absent }` ve kalıcı biçimi olan `LimitRef` (`kind` veya `kind|scope`).
- **Çekirdek (`limits/presence.rs`):**
  - S1 sayımı, S2 listesi, ilk gözlemde geri alma ve kv kodlaması.
  - Kural geneldir: beklenen limitler (`session`, `weekly_all`) ve S2'de izlenen her `(kind, scope)` için çalışır, `five_hour`'a özel değildir.
- **`LimitEngine`:**
  - `note_statusline()`: payload bazında sayım yapar.
  - `apply_poll()`: boş olmayan yanıtı tam liste kabul eder.
  - Yok sayılan limitin track'i silinir. Böylece tahmin, reset olayı, projeksiyon, `poll_inputs` reset'leri, uyanma zamanı ve bildirim üretilmez. Notifier snapshot'ı değerlendirdiği için ek koda gerek kalmadı.
  - Açılışta kv yüklenir; eski 5 saatlik snapshot geri yüklenmez.
  - `PanelSnapshot.absent` ve `PanelSnapshot::presence()` eklendi; `VisibleKey` `absent`'ı da içeriyor.
- **Runtime:**
  - IPC push artık tek tek gözlem değil, payload başına bir grup gönderiyor (`LimitsMsg::Statusline`).
  - Spool içe aktarması da payload bazında. Okumalarının hepsi daha önce saklanmış bir payload yeniden sayılmaz.
- **IPC:**
  - `AbsentLimit` ve `StatusReply.absent` / `LimitsReply.absent` alanları `#[serde(default)]` ile eklendi.
  - `PROTOCOL_VERSION` ve `--json` şeması 1'de kaldı. Eski CLI alanı yok sayar; yeni CLI eski uygulamadan boş liste okur (testli).
- **CLI:**
  - `status` ve `limits` çıktısında "5 saatlik: bu hesapta yok." satırı var.
  - Uygulama kapalıyken karar kv'den okunur ve eski snapshot gizlenir.
- **`render.rs`:**
  - Payload'da olmayan pencere yalnızca uygulamanın **ölçümüne** düşer, tahminine (`estimated`) asla düşmez.
  - **Yok** durumundaki pencerenin segmenti hiç gösterilmez.
  - Payload'ın kendi okuması "yok" kararından önce gelir.
- **Tray:**
  - `TrayVisual.single` ile tek halka çizilir.
  - Golden dosyalar eklendi: `single`, `single-crit` × 3 tema × 4 boyut.
  - Fake `cycle`'a `no-five-hour` durumu eklendi.
- **Panel:**
  - `RingGauge` için `single` modu: yarıçap 70, kalınlık 20, merkezde haftalık değer.
  - 5 saatlik satır gizlenir ve Fluent'ten bir bilgi notu (`panel-no-five-hour`, EN/TR) gösterilir.
  - Projeksiyon satırı haftalık değere düşer.
  - Fake sahne `panel:no-five-hour` eklendi.
- **Fixture'lar:** `testdata/statusline/no-five-hour.json` ve `testdata/api/claude-usage-no-session.json`, ikisi de sentetik (`testdata/README.md`).
- **Dokümanlar:** ADR-041 uygulama notu, architecture §7 ve §11, design-language (ring gauge, tray ikonu).

#### Doğrulama
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`: temiz.
- `cargo test --workspace`: core 266, app 61 + 2 ignored, CLI 27 + 2, i18n 16 + 6. Hepsi geçti.
- `pnpm -C app check` (0 hata, 0 uyarı), `pnpm -C app lint` temiz, `pnpm -C app test`: 84 test geçti.
- **Ekran görüntüleri:** Debug build, `portable` işaretiyle ve sahte sahneyle çalıştırıldı (gerçek veriye dokunulmadı). Görüntüler CDP `Page.captureScreenshot` ile alındı:
  - `img/12b-panel-ok-light-en.png`: pencere var.
  - `img/12b-panel-no-five-hour-light-en.png`: pencere yok.
- Kurulu v0.1.0 bunun için kullanıcı onayıyla birkaç dakika kapatıldı ve sonra yeniden başlatıldı.
- **Tray:** `tray_contact_sheet` ile göz kontrolü yapıldı.

#### Plandan sapmalar
- **Tray menüsünde limit satırı yok.** "Menüde 5 saatlik satır gizlenir" maddesi uygulanamazdı. 5 saatlik metin yalnızca tooltip'te geçiyor ve tooltip, pencere snapshot'ta yoksa o satırı zaten atlıyor.
- **"Bilinmiyor" ayrı bir görünüm almadı.** Okuma yokken eskisi gibi boş iç halka ve gizli satır gösterilir. "Yok" ise tek halka ve bilgi notu alır.
- **Ekran görüntüleri** açık tema ve İngilizce. Debug build kullanıcının gerçek tercihlerini değil, portable kopyanın varsayılanlarını kullandı.

#### Açık konular ve sonraki adımlar için notlar
- **Oturum payı (Adım 14):** Pay 5 saatlik pencereye göre hesaplanıyor (`share::shares(p_now, …)`). Pencere yoksa `p_now` `None` olur; panelde ve statusline'da "≈%… bu oturum" görünmez. Bu, kullanıcının doğrulanmış Ultra hesabında "her oturum her pencereyi nasıl etkiledi" hedefinin bugün çalışmadığı anlamına geliyor. Haftalık pencereye göre atıf Adım 14'ün atıf motoruyla yapılmalı. Statusline v2'nin (12c) bu payın yerini tasarlarken bunu hesaba katması gerekir.
- **12c:** `render.rs` artık yok sayılan pencereyi göstermiyor; satırın geri kalanı 12c'de yeniden tasarlanacak. `StatusReply::is_absent()` hazır.
- **12d / mod:** `absent` alanı IPC'de mevcut.
- **Adım 13:** Kalibrasyon `k_session`'ı yok sayılan hesaplarda üretmemeli. Kontrol için `kv.limits.absent.<id>` veya `PanelSnapshot::presence()` kullanılabilir.
- **`spend_limit`:** S1 payload'ı `spend_limit`'i henüz parse etmiyor. S2'den gelen her `(kind, scope)` için kural zaten çalışıyor. S1'e eklendiğinde `presence::EXPECTED`'a değil, `note_statusline`'ın `covered` listesine eklenmeli.
- **Kurulu sürüm:** Bu adım kurulu v0.1.0'a gitmedi. Sürüm çıkarma (0.1.1) ayrı bir karar.
