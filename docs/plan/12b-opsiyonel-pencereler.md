# Adım 12b — Opsiyonel limit pencereleri: çekirdek, tray ve panel

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1.x | 12 | 1 oturum | ⬜ |

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

- [ ] ADR-041 kabul edildi (2026-11-07). N değeri ve kalıcılık kararı netleşince ADR-041'e "Uygulama notu" olarak ekle.
- [ ] Domain: `WindowPresence` (ad öneri) ve hesap başına saklanması. Kalıcılık gerekip gerekmediğine karar ver (yeniden başlatmada "bilinmiyor"dan başlamak kabul edilebilir mi?).
- [ ] Füzyon ve limit motoru: kural, testler (S1-only, S2-only, ikisi, geri dönüş).
- [ ] IPC sürümü: alan eklemesi geriye uyumlu mu? Değilse `PROTOCOL_VERSION` artışı ve eski CLI ile davranış.
- [ ] Tray: tek halka ikonu (design-language'e uygun), menü.
- [ ] Panel: kart düzeni, Fluent metinleri (`en`, `tr`).
- [ ] `render.rs`: tahmini 5 saatlik değere düşmeyi kaldır, testleri güncelle.
- [ ] Sentetik fixture: `five_hour` içermeyen statusline payload'u ve `session` içermeyen API yanıtı (`testdata/README.md` kurallarına uygun).

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
_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
