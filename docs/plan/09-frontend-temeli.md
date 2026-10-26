# Adım 09 — Frontend temeli: tasarım token'ları, tema, i18n, tipli köprü

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1 | 02, 03 | 1-2 oturum | ⬜ |

## Amaç

Tüm pencerelerin üzerine inşa edileceği frontend temelini kurmak:
- Kesinleştirilmiş tasarım token'ları.
- Açık ve koyu tema, Mica/Acrylic zemin.
- Fluent tabanlı i18n (Rust ile ortak kaynak).
- Rust'tan üretilmiş tipli komut ve olay bağlamaları.
- Temel bileşen kütüphanesi ve görsel kontrol için bir galeri sayfası.

## Önce oku

- `docs/design-language.md` (tamamı)
- `docs/architecture.md` §4, §12, §13 (CSP)
- `docs/performance-budget.md` (bundle bütçeleri)
- `docs/decisions.md`: ADR-010, ADR-019

## Kapsam

- **Tasarım kesinleştirme:** `frontend-design` ve/veya `impeccable` skill'i ile `design-language.md`'deki önerileri gerçek Mica zemin üzerinde dene, token değerlerini sabitle ve dokümanı güncelle.
- `app/src/lib/styles/`: `tokens.css` (renk, tipografi, boşluk, yarıçap, hareket), `themes.css` (açık/koyu + `forced-colors`), `base.css` (reset, odak halkaları).
- Font: Alt kümesi alınmış mono variable font (woff2) + lisans dosyası (OFL). Alt küme alma script'i `scripts/subset-font.*`.
- Pencere zemini: Tauri `windowEffects` (Mica/Acrylic) + şeffaflık. Windows 10 için opak yedek, işletim sistemi build numarası Rust'tan alınır.
- i18n (TS):
  - `@fluent/bundle` + `locales/*.ftl` dosyalarının Vite ile içe aktarılması.
  - `NUM`/`PCT`/`DUR`/`TR_SUFFIX` fonksiyonları (TS).
  - Svelte için `t()` yardımcısı ve dil store'u.
  - `<html lang>` yönetimi.
- Tipli köprü:
  - `tauri-specta` ile `app/src/lib/ipc/bindings.ts` üretimi.
  - Komut iskeleti: `get_panel_snapshot`, `get_settings`, `set_locale`, `set_theme`.
  - Olay aboneliği yardımcısı: pencere görünürken abone olur, gizlenince aboneliği bırakır.
- Bileşenler (Svelte 5, runes):
  - `RingGauge` (SVG; dış/iç halka, tick'ler, hayalet projeksiyon yayı, değer geçişi, tek seferlik tarama)
  - `LimitBar`, `Countdown`, `SourceBadge`, `SessionRow`, `Card`, `Button`, `IconButton`, `Toggle`, `Tooltip`, `Skeleton`, `EmptyState`
- **Paylaşılan zamanlayıcı:** Tek bir 1 Hz ticker kullanılır. `document.visibilityState === "hidden"` olduğunda **durur**. Tüm geri sayımlar bu ticker'a abone olur; bileşen başına `setInterval` kullanılmaz.
- Galeri sayfası `gallery.html`: yalnızca geliştirme derlemesinde bulunur. Tüm bileşenler × açık/koyu × EN/TR × durumlar.
- Bundle bütçe kontrolü (`scripts/check-bundle-size.mjs`) ve bunun CI'a eklenmesi.

## Kapsam dışı

- Panel sayfasının kendisi (Adım 10), ayarlar ve onboarding sayfaları (Adım 11).
- uPlot ve grafikler (Adım 15).

## Görevler

- [ ] Tasarım skill'ini yükle ve `design-language.md` ile çalış. Mica üzerinde kontrastı ölç (açık ve koyu duvar kağıtlarıyla). Token'ları `tokens.css`'e yaz, dokümandaki değerleri güncelle.
- [ ] Tema mantığı:
  - `prefers-color-scheme` izlenir; kullanıcı ayarı (`system | light | dark`) önceliklidir.
  - Mica `system` modunda işletim sistemi temasını izler.
  - `data-theme` ve `data-os="win11|win10"` öznitelikleri `<html>` etiketine yazılır.
- [ ] `RingGauge`:
  - Prop'lar: `outer` (haftalık), `inner` (5s), `projection`, `state`, `estimated`.
  - Tahmini değerler kesikli çizgiyle gösterilir.
  - Kritik durumda çentik eklenir.
  - `aria-label` tam cümle olarak i18n'den gelir.
  - `prefers-reduced-motion` desteklenir.
  - Değer değiştiğinde 400 ms geçiş ve tek seferlik conic tarama efekti oynar. **Sürekli animasyon yoktur** (ADR-019).
- [ ] i18n:
  - `locales/{en,tr}/common.ftl` ve `panel.ftl` (Adım 10 için temel anahtarlar).
  - TS fonksiyonları `testdata/i18n-vectors.json` ile test edilir. Adım 08 henüz yapılmadıysa bu dosyayı burada oluştur; Adım 08 aynı dosyayı kullanır. **İki tarafın aynı vektörlerden geçmesi zorunludur.**
  - **Parite testi** (Vitest): EN ve TR `.ftl` dosyalarında anahtar kümeleri ve her mesajdaki değişken adları birebir aynı olmalı.
  - Biçimlendirmede TS tarafı `Intl` kullanır, ama çıktılar ortak vektörlerle karşılaştırılır. TR için `Intl.NumberFormat('tr-TR', {style:'percent'})` çıktısı `%72` olmalı; doğrula.
- [ ] Tipli köprü: `specta` tipleri (`PanelSnapshot`, `LimitView`, `SessionShareView`, `CoreEvent`) TS'e üretilir. Üretim debug derlemede otomatik yapılır, CI'da "üretilen dosya güncel mi?" kontrolü bulunur.
- [ ] Olay köprüsü (Rust): `CoreEvent` → yalnızca **açık** pencerelere `emit`. Pencere yokken serileştirme maliyeti bile ödenmez.
- [ ] Galeri sayfası ve ekran görüntüleri: açık/koyu × EN/TR için temel durumların görüntüleri devir notuna eklenir.
- [ ] Bundle bütçesi: panel girişi için ≤ 60 KB JS ve ≤ 15 KB CSS (gzip). Aşılırsa CI kırmızıya düşer.

## Teknik notlar ve tuzaklar

- **Şeffaf pencere ve Mica:** Tauri 2'de `transparent: true` + `windowEffects: { effects: ["mica"] }`. HTML ve body arka planı şeffaf olmalı; kartlar yarı saydam `--surface-1` kullanır. Mica yalnızca pencere **odaktayken** canlı görünür, odak kaybında düz renge döner. Tasarım bu durumu da kaldırmalı.
- **Acrylic ve Windows 10:** Windows 10'da Acrylic pencere sürüklenirken takılma yapabiliyor. Bu nedenle Windows 10'da varsayılan opak yedek kullanılır.
- **Svelte 5 runes:** Global durum için `$state` içeren `.svelte.ts` modülleri kullanılır. Store'lar sade tutulmalı; tüm hesaplama Rust'ta yapılır.
- **Türkçe büyük harf:** CSS `text-transform: uppercase` doğru sonuç vermesi için `lang="tr"` gerektirir. JS tarafında `toLocaleUpperCase(lang)` kullanılır. Testte "istatistik" → "İSTATİSTİK" dönüşümü doğrulanmalı.
- Font alt kümesi alırken `font-feature-settings: "tnum"` için gerekli OpenType tablolarının (`tnum`) korunduğundan emin ol.

## Kabul kriterleri

- [ ] Galeri sayfası tüm bileşenleri açık/koyu ve EN/TR olarak doğru gösterir. Mica zeminde metin kontrastı AA seviyesindedir.
- [ ] TS ve Rust i18n vektör testleri aynı dosyadan geçer. Parite testi yeşildir.
- [ ] Bağlamalar otomatik üretilir ve CI'da tazelik kontrolü vardır.
- [ ] Bundle bütçesi içinde kalınır.
- [ ] `design-language.md` kesinleşmiş token değerleriyle güncellenmiştir.

## Doğrulama

```powershell
pnpm -C app test
pnpm -C app check
pnpm -C app build; node scripts/check-bundle-size.mjs
pnpm dev   # galeri: tray menüsünde (debug) "Gallery" veya doğrudan gallery.html
```

## Devir notu

_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
