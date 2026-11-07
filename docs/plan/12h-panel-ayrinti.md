# Adım 12h — Tray panelinde ikinci görünüm: Ayrıntı

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1.x | 12f | 1 oturum | ✅ |

## Amaç

Claude Code mod'unun `/radar` paneli (Adım 12f, ADR-046) uygulamada olmayan ayrıntılar gösteriyor: zaman işareti ve tempo, haftalık zaman şeridi, kullanım grafiği, limit başına kaynak rozeti, oturum maliyeti ve pay barı. Bunlar tray panelinde ikinci bir görünüme gelir.

## Önce oku

- ADR-019, ADR-032, ADR-044, ADR-045, ADR-046
- `docs/design-language.md` §4–§5 (panel, limit çubuğu, kaynak rozeti)
- `crates/tokenradar-core/src/limits/history.rs`, `statusline/line.rs` (`elapsed`)
- `app/src/pages/panel/Panel.svelte`, `state.svelte.ts`, `integrations/claude-code/hooks/` (mod'un panel çizimi)

## Kullanıcıyla kararlaştırılanlar (2026-11-07)

- Ayrıntılar tray panelinde ama **ikinci bir görünümde**. Mevcut görünüm ("Özet") olduğu gibi kalır, ikisi arasında geçiş yapılır.
- Gelecek öğeler: zaman işareti ve tempo, haftalık zaman şeridi, kullanım grafiği, rozetler ve oturum maliyeti.
- Bağlam doluluğu barı gelmez; mod bu değeri yalnızca kendi oturumundan bilebiliyor.

## Kapsam

- Core: `limits::pace` (`elapsed`, `warns`, `PACE_AHEAD`, `PACE_QUIET`; statusline ile ortak), `limits::detail` (`PanelDetail`, `PaceView`, `FocusView`, `PeriodMark`, `ChartPoint`), `history::window_readings`, `LimitView.window_start`, `GeneralConfig.panel_view`.
- Uygulama: `get_panel_detail`, `get_panel_view`, `set_panel_view` komutları; sahte sahnede geçmiş verisi.
- Frontend: `Özet | Ayrıntı` segment kontrolü, tembel yüklenen `Detail.svelte` (limit kartları, şerit, grafik, oturumlar), `LimitBar` zaman işareti, `SessionRow` maliyet ve pay barı, galeri bölümü, EN/TR metinler.

## Kapsam dışı

- Bağlam doluluğu barı.
- Odak limit dışındaki limitlerin (5 saatlik pencere, Fable) grafiği.
- Ana penceredeki grafikler (uPlot, Adım 15).

## Doğrulama

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
- `pnpm -C app check`, `pnpm -C app lint`, `pnpm -C app test`
- `node scripts/check-bundle-size.mjs`, `scripts/measure-panel.ps1` (bütçeli yol: panel)
- Gerçek panelde (portable kopya, sahte sahne) iki görünümün açık/koyu ve EN/TR ekran görüntüleri

## Devir notu

**Tarih:** 2026-11-07 · **Durum:** ✅ Tamamlandı

**Kullanıcı kararları:** İkinci görünüm (Özet/Ayrıntı) ve dört öğe; bağlam barı yok (ADR-048). Gerçek panel denemesi ve ölçüm için kurulu uygulamanın kısa süre kapatılmasına onay verildi; sonra yeniden başlatıldı.

**Yapılanlar:**
- **Core:**
  - `limits::pace`: `elapsed` statusline'dan buraya taşındı (`statusline::line` yeniden dışa aktarıyor). Uyarı kuralı `warns` olarak tek yerde.
  - `limits::detail::panel_detail`: her ölçülmüş limitin temposu, odak penceresi (haftalık, yoksa 5 saatlik), 24 saatlik işaretler ve pencerenin ölçümleri.
  - `history::window_readings`, `window_series` ile ortak.
  - `LimitView.window_start` (görünür diff'e girmez). `[general] panel_view`.
- **Uygulama:** `get_panel_detail` (salt okunur depodan bir haftalık okuma), `get_panel_view`/`set_panel_view`; sahte sahnede `fake::history`.
- **Frontend:**
  - `Özet | Ayrıntı` sekmeleri: ←/→, `Ctrl+Tab`; seçim `config.toml`'da hatırlanır.
  - `Detail.svelte` tembel yüklenir. Görünür kaldığı sürece dakikada bir yenilenir, `limits_updated` ile de yenilenir.
  - `DetailLimits`, `TimeStrip`, `UsageChart` (satır içi SVG). Yerleşim yardımcıları `detail.ts` içinde.
  - Özet'in çubuklarında zaman işareti.
  - Oturum satırlarında API eşdeğeri maliyet ve pay barı (≈pay kesikli).
- **Gerçek panel denemesinde bulunan ve düzeltilenler:**
  - `#app`'in yüksekliği yoktu; `main` hiç kaymıyor, uzun içerik kesiliyordu. Bu hata Adım 10'dan beri gizliydi. Özet'te de çok oturumla görülebilirdi.
  - Kaydırma çubuğu ince.
  - "▲ şimdi" ile 24 saatlik değer çakışıyordu; kelimenin uzandığı tarafta boşluk %20'ye çıkarıldı.
  - TR açıklamada "Sıfırlanma" yerine sözlükteki `Sal 21:52'de yenilenir` kalıbı (`resets-at`) kullanılıyor.
- **Belgeler:** ADR-048; design-language (zaman işareti, görünüm geçişi, şerit ve grafik, oturum satırı, sözlük); architecture §5; README; CHANGELOG (Added + Fixed); perf kaydı.

**Sapma:** Plan, yerel gece yarılarını core'da `jiff` ve sistem saat dilimiyle hesaplamayı öngörüyordu. Bunun yerine frontend'de `Date` ile hesaplanıyor (yaz saatine duyarlı). `jiff`'in saat dilimi veritabanı CLI'ın 3 MB bütçesini zorlardı; gece yarısı bir alan hesabı değil, takvim yerleşimi. ADR-048'de kayıtlı.

**Doğrulama:**
- `cargo fmt --check`, clippy (`-D warnings`), `cargo test --workspace`: temiz (core 300, app 61, cli 22+2, i18n 16+6).
- `pnpm -C app check`, `lint`, `test` (103): temiz.
- Bundle: panel ilk yükü 42,8 KB JS / 4,6 KB CSS (main 39,2 / 4,3), Ayrıntı parçası 3,6 + 1,2 KB.
- Panel ölçümü (Ayrıntı açık): soğuk 379,6 ms, sıcak 47,2 ms, açık 82,2 MB, TTL sonrası 8–11 MB. Hepsi bütçe içinde ([perf kaydı](../perf/2026-11-07-panel-detail.md)).
- Gerçek panelde ekran görüntüleri: Ayrıntı EN açık (üst ve kaydırılmış), TR koyu (üst ve kaydırılmış), Özet TR koyu (Mica ile). Ayrıntı içeriği 866 px, panel 600 px; içerik kayıyor.

**Ölçülmeyenler ve riskler:**
- Ayrıntı yalnızca sahte sahneyle denendi. Gerçek veride haftanın ilk günlerinde grafik az nokta gösterecek; geçmiş yoksa boş durum metni çıkıyor (testli).
- Başlık ve sekmeler içerikle birlikte kayıyor. Aşağıdayken Özet'e dönmek için yukarı kaydırmak ya da `Ctrl+Tab` gerekiyor. Yapışkan başlık bir seçenek; kullanıcı geri bildirimine bırakıldı.
- TR'de tempo satırı ve 24 saat açıklaması iki satıra kırılıyor; okunur ama yer kaplıyor.
- %150/%200 ölçekte ve yüksek kontrast temasında gerçek panel denenmedi (forced-colors kuralları yazıldı).
- Kurulu uygulama bu adımdan önceki sürüm; yeni bir kurulum paketi üretilip kurulana kadar Ayrıntı görünmez.

**Kapsam dışı kalanlar:** Bağlam barı; odak dışı limitlerin grafiği; yapışkan başlık.

**Sonraki adım:** Durum tablosundaki sıradaki adım.
