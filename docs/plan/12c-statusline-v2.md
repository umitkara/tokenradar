# Adım 12c — Statusline v2: görünüm, ek bilgi, özelleştirme, tazelik

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1.x | 12b | 1-2 oturum | ✅ |

## Amaç

`tokenradar statusline` çıktısını yeniden tasarlamak. Kullanıcının dört talebi var:
1. **Görünüm:** mini progress bar ve eşiğe göre renk.
2. **Ek bilgi:** context doluluğu, scoped (ör. Fable) haftalık limit, oturum payı, oturum maliyeti, model.
3. **Özelleştirme:** hangi segmentlerin hangi sırayla görüneceği; tek satır / iki satır.
4. **Doğruluk ve tazelik:** her değerin kaynağı ve yaşı belli olmalı. Bayat değer soluk gösterilir, tahmin `≈` ile işaretlenir.

5 saatlik penceresi olmayan hesap (doğrulanmış Ultra) için onaylanan varsayılan satır:

```
7d ████░░░░ 41% · 3g4s │ Fable 22% │ ctx 38% │ ≈2.1% bu oturum · $4.12
```

5 saatlik penceresi olan hesapta, aynı düzende `5h` segmenti en başa eklenir.

## Önce oku

- `docs/architecture.md` §11 (`tokenradar statusline` akışı, p95 < 30 ms)
- `docs/data-sources.md` S1 (payload alanları)
- `docs/performance-budget.md` (CLI soğuk başlatma ve statusline bütçeleri)
- `docs/design-language.md` (renk token'ları, `≈` kuralı, EN/TR sözlük)
- `docs/decisions.md`: ADR-029, ADR-041
- Adım 07 ve 12b'nin devir notları

## Kapsam

- **Ingest'e dokunulmaz.** `respond()` içindeki "önce push et / spool'a yaz, sonra formatla" sırası ve `StatuslinePush` aynen kalır. Değişiklik yalnızca `Line` modeli ve `render` katmanındadır.
- `Line` modeli genişler. Her segment değeri, kaynağını (`statusline` / `api` / `estimate`) ve gözlem zamanını taşır.
  - Payload'dan: `context_window.used_percentage`, `cost.total_cost_usd`, `model.display_name`.
  - Uygulamadan (IPC `status.get`): scoped limitler, oturum payı, projeksiyon.
- Segmentler: `five_hour`, `seven_day`, `scoped`, `context`, `session_share`, `cost`, `model`. Her biri bağımsız render edilir.
- Bar: 8 hücre, Unicode blok karakterleri. `NO_COLOR` ve dar terminalde bar düşer, yüzde kalır.
- Renk: mevcut `Severity` eşikleri. Eşikler ayarlardan gelir (renk kodu sabitlenmez).
- Tazelik: uygulamadan gelen değer belirli bir yaştan (öneri: 10 dk) eskiyse soluk gösterilir. Uygulama yoksa yalnızca payload segmentleri gösterilir.
- Özelleştirme: `config.toml` içinde `[statusline]` tablosu:
  - `segments = ["seven_day", "scoped", "context", "session_share", "cost"]`
  - `layout = "single" | "double"`
  - `bar = true`
  - Bilinmeyen segment adları yok sayılır. Config okunamazsa varsayılan kullanılır, statusline asla hata basmaz.
- Genişliğe uyum: mevcut `Detail` kademeleri segment önceliğiyle genelleştirilir. Önce düşük öncelikli segmentler düşer.
- Ayarlar penceresi: "Statusline bridge" sekmesinde segment seçimi ve canlı önizleme. Önizleme metni backend'den gelir (UI'da hesap yapılmaz).
- Fluent: yeni metinler `locales/{en,tr}/cli.ftl` ve `settings.ftl` içinde.

## Kapsam dışı

- Claude Code mod'u (Adım 12d).
- Kurulu v0.1.0'ı güncellemek veya sürüm çıkarmak. Veri haftası bitene kadar dal üzerinde kalır. Sürüm kararı kullanıcınındır.
- Statusline'a yeni bir veri kaynağı eklemek.

## Görevler

- [x] `Line` → segment listesi modeli; kaynak ve yaş alanları.
- [x] Segment render fonksiyonları ve testleri (renk açık/kapalı, dar/geniş, tek/iki satır).
- [x] `[statusline]` config şeması, okuma ve varsayılanlar. Config testleri.
- [x] Tazelik ve `≈` kuralları için testler: payload taze / uygulama bayat / uygulama yok.
- [x] Ayarlar sekmesi: segment seçimi ve önizleme (tipli `tauri-specta` komutu).
- [x] Fluent metinleri (`en`, `tr`), TR yazımı kontrol edilir.
- [x] Performans: statusline p95 ölçümü, `docs/perf/` altına kayıt.

## Teknik notlar ve tuzaklar

- Claude Code, yeni güncelleme geldiğinde çalışan statusline komutunu iptal eder. Config okuma soğuk başlatma bütçesine sığmalı. Okumanın maliyeti ölçülür; gerekirse yalnızca `[statusline]` tablosu ayrıştırılır.
- Payload'daki değer her zaman uygulamanın değerinden tazedir. Aynı segment için iki kaynak varsa payload kazanır (v1 kuralı korunur).
- `cost.total_cost_usd` API karşılığı maliyettir, abonelik faturası değildir. Etiket bunu yanlış anlatmamalı (design-language sözlüğüne bak).
- Gerçek spool ve pipe'a dokunmamak için testler portable kopya ve geçici dizinlerle yapılır.

## Kabul kriterleri

- Varsayılan satır, 5 saatlik penceresi olmayan hesapta yukarıdaki örnekle aynı düzende çıkar.
- `segments` ve `layout` ayarları satırı beklenen şekilde değiştirir.
- Hiçbir tahmin `≈` olmadan gösterilmez. Bayat değerler soluk gösterilir.
- Statusline p95 < 30 ms (bütçe), ölçüm kayıtlı.
- Ingest davranışı değişmez: mevcut statusline push ve spool testleri değişmeden geçer.

## Doğrulama

- `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`
- `pnpm -C app check`, `pnpm -C app test`
- Portable kopyayla sentetik payload'lar üzerinden `tokenradar statusline` çıktısının görsel kontrolü.
- `scripts/` altındaki statusline ölçüm betiği ile p95.

## Devir notu

**Tarih:** 2026-11-07 · **Dal:** `step/12c-statusline-v2` · **Karar:** ADR-042

#### Kullanıcı kararları
- **Oturum payı:** Değer yoksa segment gizlenir. Haftalık pencereye göre pay Adım 14'te gelecek; segment o zaman kendiliğinden görünür.
- **Maliyet etiketi:** `API $4.12` (TR: `API $4,12`). `API` işareti değerin fatura olmadığını söyler.
- **İki satır düzeni:** 1. satırda limitler, 2. satırda oturum bilgisi. Her satırda kullanıcının sırası korunur.
- **Renk eşikleri (plandan sapma):** Plan "eşikler ayarlardan gelir" diyordu, ama uygulamada renk eşiği ayarı yok. Statusline, tray ve panelle aynı sabit eşikleri kullanır: `WARN_AT` = 75, `CRIT_AT` = 90. Kullanıcı bunu onayladı (ADR-042).
- **Ölçüm ve ekran görüntüsü için kurulu v0.1.0 iki kez kısa süre kapatıldı** (kullanıcı onayıyla, toplam ~15 dk) ve yeniden başlatıldı. Kapalıyken gerçek statusline okumaları spool'a düştü ve açılışta içeri alındı.

#### Yapılanlar
- **Core (`statusline/line.rs`, `line/render.rs`):**
  - `Line` modeli: `five_hour`, `seven_day`, `scoped[]`, `context`, `session_share`, `cost`, `model`. Her değer `source` ve `observed_at` taşır; tazelik `Line::is_stale` ile, `FRESH_MS` (10 dk) eşiğine göre belirlenir.
  - `Line::from_sources`: payload kazanır. Uygulamanın tahmini gösterilmez, "yok" sayılan pencere gösterilmez, payload'daki geçmiş reset `≈ 0 %` olur. Scoped etiketi `model:Fable` → `Fable`.
  - Render: boyalı span'ler (`Paint`) → `to_ansi`. 8 hücrelik bar (`█`/`░`, yukarı yuvarlanır, yani her kullanım en az bir hücre doldurur). Bar `NO_COLOR` altında gösterilmez. Genişliğe uyum: önce oturum segmentleri, sonra bar, projeksiyon ve reset sayaçları, en son limitler düşer.
  - `StatuslinePrefs` (`[statusline]`): esnek okunur, normalize edilir (tekrarlar atılır, boş liste varsayılana döner). `config::save_statusline_prefs` bilinmeyen anahtarları korur.
  - `ModelInfo.display_name` okunur ama `skip_serializing` ile işaretli: push ve spool biçimi değişmedi (testli).
- **CLI:** `render.rs` yalnızca `style_from_env()` içeriyor. `Deps.prefs` alanı `ctx.config.statusline`'dan gelir. `respond()` içindeki "önce push et veya spool'a yaz, sonra biçimlendir" sırası değişmedi.
- **Uygulama:**
  - Yeni komutlar: `set_statusline_prefs` ve `preview_statusline`. Önizleme, örnek değerlerle çizilen span'leri ve hesapta olmayan segmentleri (`absent`) döndürür.
  - `PrefsView.statusline` eklendi; binding'ler yeniden üretildi.
- **Ayarlar:** Statusline bridge kartında "Görünüm" bölümü var:
  - Canlı önizleme, terminal fontuyla (Cascadia Mono / Consolas).
  - Segment anahtarları ve yukarı/aşağı sıralama. Son açık segment kapatılamaz.
  - Hesapta olmayan pencereye "bu hesapta yok" notu.
  - Düzen seçimi ve bar anahtarı.
  - Saf mantık `view.ts` içinde: `segmentRows`, `toggleSegment`, `moveSegment`, `segmentLabelKey`.
- **Fluent:** `cli.ftl` dosyasına `sl-percent`, `sl-context` ve `sl-cost` eklendi, `sl-window` kaldırıldı. `settings.ftl` dosyasına `statusline-look*` ve `statusline-segment-*` anahtarları eklendi. EN ve TR eşit.
- **Dokümanlar:**
  - ADR-042.
  - architecture §11 (madde 7).
  - design-language sözlüğüne "ctx" ve "API $" girişleri.
  - performance-budget, README (satır örneği ve `[statusline]`), CHANGELOG (Unreleased).
- **Ölçüm betiği:**
  - Payload'a model, maliyet ve bağlam alanları eklendi.
  - A/B karşılaştırması için `-Cli` parametresi eklendi.
  - `[CmdletBinding()]` eklendi. Önceden bilinmeyen bir parametre sessizce `$args`'a düşüyordu; bu oturumdaki ilk A/B koşusu bu yüzden iki kez yeni binary'yi ölçmüştü.

#### Doğrulama
- `cargo fmt --check` ve `cargo clippy --workspace --all-targets -- -D warnings`: temiz.
- `cargo test --workspace`: core 282 (yeni: line 15, config 1), app 61 + 2 ignored, CLI 21 + 2 (6 render testi core'a taşındı), i18n 16 + 6.
- `pnpm -C app check` (0 hata, 0 uyarı), `pnpm -C app lint` temiz, `pnpm -C app test` 87 test geçti.
- **Görsel kontrol (CLI):** `release-cli` derlemesinin portable kopyasıyla, sentetik payload'larla yapıldı:
  - EN ve TR, yalnız haftalık pencere ve 5 saatlik + haftalık.
  - `NO_COLOR`, `COLUMNS=50`.
  - Özel segmentlerle iki satır.
  - Bozuk `[statusline]` (varsayılanlara döndü) ve bozuk stdin.
  - Bulunup düzeltilen hata: dolu barda boş bir ANSI çifti yazılıyordu.
- **p95:** açık modda 12,0 / 13,4 ms, kapalı modda 25,0 / 26,9 ms. `main` ile aynı oturumda A/B yapıldı, fark gürültü içinde (`docs/perf/2026-11-07-statusline-v2.md`).
- **Ekran görüntüleri:** Debug build, portable modda ve `panel:no-five-hour` sahte sahnesiyle çalıştırıldı; görüntüler CDP ile alındı.
  - `img/12c-settings-statusline-en.png`
  - `img/12c-settings-statusline-double-tr.png`

#### Plandan sapmalar
- **Renk eşikleri** sabit kaldı (yukarıda, ADR-042).
- **Mevcut CLI statusline testlerinde yalnızca çıktı metni assert'leri değişti**, çünkü pasta glifi kalktı (`◑ 5h 42%` → `5h 42%`). Push sayısı ve spool satırı assert'leri aynen geçiyor.
- **Bar yalnızca 5 saatlik ve haftalık pencerede var.** Scoped segment kompakt kalır (`Fable 22%`, bar ve reset yok), onaylanan örnekteki gibi.
- **Önizleme fontu** Geist Mono değil, terminal fontu. Geist alt kümesinde blok karakterleri yok.
- **Sıfır kullanımda bar:** Plan bir yuvarlama kuralı belirtmiyordu. Onaylanan örnek (%41 → 4 hücre) yukarı yuvarlamayı gösteriyor; bu yüzden %0,1 bile bir hücre doldurur.

#### Açık konular ve sonraki adımlar için notlar
- **`NO_COLOR` ve Claude Code:** Claude Code bu oturumun araç kabuğuna `NO_COLOR=1` veriyor. Statusline komutuna da veriyorsa gerçek satırda bar ve renk görünmez. Bu, gerçek Claude Code statusline'ında henüz gözlenmedi. Dal kurulu sürüme gitmeyeceği için gözlem, 0.1.1 kurulduğunda yapılmalı. Gerekirse ayrı bir ayar düşünülebilir (ör. `color = "always"`).
- **Oturum payı (Adım 14):** Segment hazır; değer `SessionInfo.share_percent`'ten gelir. Haftalık pay farklı bir alan olarak gelirse `Line::from_sources` ona bağlanmalı. Etiket şu an "bu oturum" diyor; hangi pencereye ait olduğu belirsiz kalmamalı.
- **12d (mod):** Mod, aynı span modelini kullanabilir: `line::spans` + `Paint`. `preview_statusline` örnek değerlerle çalışıyor; mod gerçek değer isterse `Line::from_sources` kullanılmalı.
- **Önizlemede sarma:** Dar ayarlar penceresinde satır segmentin ortasından sarabiliyor (`API` / `$4.12`). Kozmetik.
- **CHANGELOG:** 12b değişiklikleri (tek halka, "yok" kuralı) henüz Unreleased'a yazılmadı. 0.1.1 sürümü hazırlanırken eklenmeli.
- **Kurulu sürüm:** Bu adım da kurulu v0.1.0'a gitmedi. 0.1.1 sürümünü çıkarma kararı kullanıcının.
