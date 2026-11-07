# Adım 12e — Zaman temposu: statusline işareti ve mod'da tempo

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1.x | 12d | 1 oturum (12d ile aynı oturumda, kullanıcı kararıyla) | ✅ |

## Amaç

Kullanıcının "Kota Zaman Çizelgesi" fikri: pencerenin zaman olarak yüzde kaçının geçtiğini göstermek ve kullanımı bu çizgiyle karşılaştırmak. TokenRadar pencerenin reset zamanını bildiği için başlangıç elle girilmez: başlangıç = reset − pencere uzunluğu.

## Önce oku

- ADR-041, ADR-042, ADR-043
- Adım 12c ve 12d devir notları
- `crates/tokenradar-core/src/statusline/line.rs`, `line/render.rs`
- `integrations/claude-code/hooks/model.ts`

## Kapsam

- **Statusline (kullanıcı kararı: bar üstünde zaman işareti):**
  - 5 saatlik ve haftalık pencerede bara, geçen zamanın denk geldiği hücre sınırına `│` işareti eklenir. İşaret hücrenin yerine geçmez, iki hücre arasına girer.
  - Barın ardından `⏱43%` metni gelir. Metin renksiz terminalde de görünür.
  - `[statusline]` tablosuna `pace` anahtarı eklenir (varsayılan `true`). Ayarlar penceresinde bir anahtar ve önizleme bulunur.
  - Dar terminalde küçülme sırası: önce bar ve işaret, sonra `⏱` metni, sonra projeksiyon.
- **Mod:**
  - `/radar`'a tempo satırı eklenir: geçen zaman, kullanım ve fark (puan ve saat).
  - `/radar`'a gün gün çizelge eklenir: kalan her gün sonunda zaman çizgisinin değeri.
  - Bant: Kullanım zaman çizgisini 10 puan veya daha fazla aşarsa uyarı seviyesinde bir satır gösterilir. Bu satır toast üretmez (projeksiyon uyarısıyla çakışmasın diye yalnızca bantta).
- ADR-044.

## Kapsam dışı

- Mod'un görsel paneli ve görsel bant → Adım 12f.
- Tray ve panelde tempo gösterimi.

## Kabul kriterleri

- Statusline'da işaret doğru hücre sınırında. `pace = false` iken işaret ve metin yok.
- Geçen zaman bir ölçümdür, tahmin olarak işaretlenmez. Zaman çizgisi bir referanstır, projeksiyon olarak sunulmaz.
- `cargo test`, clippy, `pnpm -C app check/test`, mod'un üç komutu temiz.

## Doğrulama

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
- `pnpm -C app check`, `pnpm -C app lint`, `pnpm -C app test`
- `claude plugin validate|test integrations/claude-code`, `tsc -p integrations/claude-code`

## Devir notu

**Tarih:** 2026-11-07 · **Durum:** ✅ Tamamlandı

**Protokol sapması:** Adım, 12d ile aynı oturumda yapıldı. Kullanıcı "hemen bu oturumda uygula" dedi. Plan dosyası (bu dosya ve 12f) oturum içinde yazıldı.

**Yapılanlar:**
- **Core:**
  - `statusline::line::elapsed(resets_in, length)` eklendi. `Window.elapsed`, payload'dan da uygulamadan da (`from_app`) doldurulur. Pencere uzunluğu `SESSION_WINDOW_MS` ya da `WEEK_MS`'dir.
  - `StatuslinePrefs.pace` (varsayılan `true`) eklendi.
  - Render tarafında bar, `mark_at()` ile seçilen hücre sınırına `│` koyar. Yüzdeden sonra `sl-elapsed` (`⏱43%` / `⏱%43`) gelir.
  - `fit()` düşürme sırası: oturum segmentleri → bar (işaretle birlikte) → `⏱` metni → projeksiyon → reset → limitler.
- **Uygulama:** Ayarlar → Statusline köprüsü bölümüne "Zaman işareti" anahtarı eklendi. Önizleme aynı core kodunu kullanır. Bindings yeniden üretildi.
- **Mod:**
  - `Row.elapsed` eklendi; `windowLength()` ve `elapsedOf()` ile hesaplanır.
  - Tempo uyarısı eklendi: `PACE_AHEAD = 10`, `reason: 'pace'`, uyarı seviyesi 1. Yalnızca bantta görünür. Tahmini satırlar karşılaştırılmaz.
  - `toastLevel()` tempo uyarısını toast dışında tutar. `stepAlerts(prev, bandLevel, toastable)` imzası değişti. Tempo uyarısı sürdükçe "Gizle" sıfırlanmaz.
  - `/radar`'a `paceLines()` eklendi: tempo satırı ve haftalık "Zaman çizgisi". Saatler `clock(lang, at, tzOffsetMin)` ile yazılır. `register.tsx`, yerel saat farkını `getTimezoneOffset()` ile verir.
- **Metin ve testler:**
  - EN/TR metinler `hooks/i18n.ts` içinde.
  - Testler: mod tarafında 21 test (5'i yeni); core tarafında 4 yeni statusline testi.
  - CLI testi güncellendi: `pace` varsayılan açık olduğu için satırda artık `⏱` var.
- **Belgeler:** ADR-044 yazıldı. Architecture §11 (madde 8 ve mod), README (örnek satır, `pace = true`, mod maddeleri), CHANGELOG ve design-language sözlüğü güncellendi.

**Doğrulama:**
- `cargo fmt --check`, clippy (`-D warnings`) ve `cargo test --workspace`: temiz.
- `pnpm -C app check`, `pnpm -C app lint` ve `pnpm -C app test`: temiz.
- `claude plugin validate`: geçti. `tsc -p integrations/claude-code`: temiz. `claude plugin test`: 21/21.

**Ölçülmeyenler ve riskler:**
- `⏱` bazı terminal ve fontlarda iki hücre genişliğinde çizilebilir; `fit()` bunu bir hücre sayar (ADR-044, Bedel). Kullanıcının terminalinde denenmedi.
- Statusline'ın gerçek Claude Code içindeki görünümü ve modun yeni bant satırı elle denenmedi. Denemek için 12d'deki geçici CLI yolunu (`/config` → TokenRadar CLI) yeni build'e yöneltmek gerekir.
- Statusline sıcak yolu yalnızca birkaç biçimlendirme adımı kazandı. Bütçe ölçümü yapılmadı; değişiklik, render maliyetini anlamlı ölçüde değiştirmiyor.

**Kapsam dışı kalanlar:**
- Tray ve panelde tempo gösterimi. İstenirse ayrı bir adım olur.
- Mod'un görsel yüzeyleri (Pane, Svg grafik, görsel bant) → Adım 12f.

**Sonraki adım:** 12f (görsel mod). Plan dosyası hazır, yeni oturumda başlanır.
