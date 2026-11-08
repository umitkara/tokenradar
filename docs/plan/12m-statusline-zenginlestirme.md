# Adım 12m — Statusline zenginleştirme: yeni öğeler ve biçimler

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1.x | 12l | 1–2 oturum | ⬜ |

## Amaç

Statusline'ı hem sayıca hem içerikçe zenginleştirmek. Yeni öğeler eklenir ve mevcut öğelere yeni biçimler gelir. Bütün değerler ya Claude Code payload'ından doğrudan okunur ya da payload'dan saf hesapla türetilir. Bu adımda yeni bir veri kaynağı ve pipe turu yoktur.

Doğrulanmış Ultra hesaplarında 5 saatlik pencere yok (ADR-041). Bu yüzden öncelik haftalık pencereye hizmet eden öğelerdedir.

## Önce oku

- ADR-041, ADR-042, ADR-044, ADR-049, ADR-051, ADR-052
- Adım 12i (prob akışı, öğe modeli) ve 12l'nin devir notları
- `docs/data-sources.md` (S1: statusline payload)
- `crates/tokenradar-core/src/statusline/{payload.rs, line.rs}`, `statusline/line/{prefs.rs, render.rs, presets.rs}`
- `app/src/pages/settings/statusline/{AddPalette.svelte, ItemEditor.svelte}`, `app/src/pages/settings/view.ts`

## Kullanıcıyla kararlaştırılanlar (2026-11-08)

- **Kapsam:** A grubu (payload'dan gelen yeni öğeler) ve B grubu (mevcut öğelere yeni biçimler).
- **Store ya da daemon'a bağlı öğeler bu adımda yok.** Bugünkü toplam maliyet, aktif oturum sayısı ve eğilim oku istenmedi. Compact sayısı ve auto-compact limiti Adım 12n'dedir.
- **Doğrulanmamış payload alanları önce gözlenir:** `context_window.current_usage` (cache oranı) ve `output_style`. Gözlem yapılmadan bu alanlar için kod yazılmaz.

## Kapsam

### 0. Önkoşul gözlemi: payload şeması

- **Kurulum:** 12i'deki prob akışı kullanılır: diff → kullanıcı onayı → backup → atomic write.
- **Probun işi:**
  - Mevcut TokenRadar komutunu zincirler, böylece ingest kesilmez.
  - Payload'ın **yalnızca anahtar yolunu ve JSON tipini** bir dosyaya yazar, örneğin `context_window.current_usage.cache_read_input_tokens: number`.
  - Değer yazmaz. Yol, token sayısı ve maliyet kaydedilmez.
- **Gözlem bitince** backup'tan geri dönülür.
- **Sonuç** `docs/data-sources.md` S1 bölümüne yazılır. Gözlenmeyen alanın öğesi bu adımdan çıkar ve devir notuna yazılır.

### 1. Yeni öğeler (`SegmentKind`)

| `kind` | Grup | Değer | Biçimler | Kaynak |
|---|---|---|---|---|
| `daily_budget` | Limit | Haftalık pencerede kalan payın kalan günlere bölümü: `%9/gün` | `percent` | Pencerenin kaynağı. Tahminse `≈` |
| `burn_rate` | Oturum | Oturum maliyetinin süreye bölümü: `$4.20/sa` | `two_decimals` | cost ÷ duration, türetilmiş ölçüm |
| `session_name` | Oturum | Claude Code oturum adı | — | payload `session_name` (zaten parse ediliyor) |
| `cache_ratio` | Oturum | Son istekte cache'ten okunan girdi oranı: `cache %82` | `percent` | payload `current_usage`. **Gözleme bağlı** |
| `output_style` | Oturum | Etkin output style adı | — | payload `output_style.name`. **Gözleme bağlı** |

- **Hesaplar render'da yapılmaz.** Değerler `Line` ya da `Facts` içine hesaplanmış olarak girer. `render.rs` yalnızca biçimlendirir.
- **`daily_budget` kuralları:**
  - `kalan_gün = max(1, ceil(resets_in / 1 gün))`.
  - Kalan pay 0 ise kritik boyayla `%0/gün` gösterilir.
  - Haftalık pencere yoksa öğe gizlenir.
  - Tempo eşikleri (ADR-044) renge uygulanır.
- **`burn_rate`:** Süre 5 dakikadan kısaysa gizlenir; ilk dakikaların oranı gürültüdür.
- **Dar terminal:** `daily_budget` limit grubundadır ve limitlerle birlikte en son düşer. Diğerleri oturum grubuyla birlikte düşer.

### 2. Yeni biçimler (`Format`) ve öğe alanları

- **Limitler (`five_hour`, `seven_day`, `scoped`):**
  - Yeni biçim `remaining`: kalan yüzde, `%58 kaldı`. Renk eşikleri kullanılan yüzdeye göre kalır.
  - Yeni alan `reset_format`: `countdown` (varsayılan, bugünkü `3g4s`) ya da `clock` (`Pzt 09:00`, yerel saat). Gün adları Fluent'tan gelir.
- **Çubuk (`bar` biçimi olan bütün öğeler):**
  - Yeni alan `bar_width`: 4, 8 (varsayılan) ya da 12 hücre. `space` öğesinin `width` alanı yeniden kullanılmaz.
  - Yeni alan `bar_style`: `blocks` (bugünkü), `thin`, `dots`. Glifler tek hücre genişliğinde olmalı (bkz. tuzaklar).
- **Bağlam (`context`):**
  - `remaining`: kalan yüzde.
  - `tokens_left`: kalan token, `185k kaldı`.
- **`fit()` sırası:** Yeni alanlar ayrıntı düşürme sırasına uyar. Dar terminalde çubuk önce 8'e, sonra yüzdeye iner; `clock` sıfırlanmayı gizleme adımında düşer.

### 3. Model, geriye uyum ve şablonlar

- `config.toml` biçimi `version = 2` olarak kalır. Yeni alanlar isteğe bağlıdır; yoksa bugünkü davranış korunur.
- **Esnek okuma (ADR-042):** Bilinmeyen `kind`, `format`, `reset_format`, `bar_style` atlanır ya da varsayılana döner. `bar_width` 4, 8 ve 12 dışındaysa 8'e döner.
- **Şablonlar:** Yeni öğeler varsayılan şablonlara girmez. Yalnızca `detailed` şablonuna `daily_budget` eklenir.
- **Örnek veri:** `Line::sample_in` yeni öğelerin değerlerini de üretir, önizleme dört durumda da dolu görünür.

### 4. Düzenleyici

- **Ekleme paleti:** Yeni öğeler kendi gruplarında görünür. Gözlemle düşen öğeler palete girmez.
- **Öğe editörü:**
  - Limitlerde "Sıfırlanma: Geri sayım / Saat" seçici.
  - Çubuklu öğelerde genişlik (4/8/12) ve stil seçicileri. Stil seçicinin her seçeneği glif örneğini gösterir.
  - Biçim seçicisi yeni biçimleri otomatik olarak listeler; seçenekler backend'den gelir.
- **Fluent (EN/TR):** Öğe adları, biçim adları, `/gün`, `/sa`, `kaldı` ekleri ve gün kısaltmaları. Mevcut "her anahtar EN dosyasında var" testi yenileri de kapsar.

## Kapsam dışı

- Store ya da daemon'a bağlı öğeler. Bugünkü maliyet, aktif oturum sayısı ve eğilim oku istenmedi.
- Compact sayısı ve auto-compact limiti: Adım 12n.
- Eğilim oku ve geçmiş gerektiren her şey.
- Sürekli animasyon (ADR-019).
- Kurulu sürümü güncellemek ya da sürüm çıkarmak.

## Görevler

- [ ] Prob: kurulum, gözlem, geri dönüş; sonucun `data-sources.md`'ye yazılması.
- [ ] Core: `SegmentKind`, `Format`, `Item` alanları, esnek okuma, yazma; testler.
- [ ] Core: `Line`/`Facts` türetimleri (`daily_budget`, `burn_rate`, gözlenen alanlar); testler.
- [ ] Render: yeni biçimler, çubuk genişliği ve stili, `clock`, `fit()` sırası; iki tema için testler.
- [ ] Şablonlar ve örnek veri.
- [ ] Bindings (`tauri-specta`) ve düzenleyici: palet, öğe editörü, Fluent.
- [ ] İlk çalışan hâlin ekran görüntüsü kullanıcıya gösterilir ve onaylanır, ardından cila.
- [ ] ADR-053 ve devir notu.

## Teknik notlar ve tuzaklar

- **Glif genişliği:** `●○` gibi gliflerin East Asian Width değeri "Ambiguous"tır. Bazı terminallerde iki hücre kaplar ve hizayı bozar. `bar_style` glifleri `unicode-width`'te 1 olan ve Ambiguous olmayanlardan seçilir. Bunu bir test denetler.
- **Ölçüm ile tahmin ayrımı:** `burn_rate` ölçülmüş iki değerden türetilir, `≈` almaz. `daily_budget` pencerenin kaynağını devralır.
- **Eski sürümler** bilinmeyen öğe türünü okurken atlar. Eski bir sürüm dosyayı yeniden yazarsa yeni öğeler kaybolur. Bu, 12i ve 12k'daki davranışla aynıdır; devir notunda hatırlatılır.
- **Soğuk başlatma bütçesi:** Öğe ve alan sayısı arttı. `docs/performance-budget.md`'deki statusline soğuk başlatması ölçülür ve `docs/perf/` altına yazılır.
- **Gerçek veriye dokunulmaz:** Testler portable kopya ve geçici dizinlerle yapılır. Prob çıktısı ve ölçüm commit'leri değer içermez.
- **UI'da hesap yapılmaz.** Glif örnekleri ve biçim önizlemeleri backend önizlemesinden gelir.

## Kabul kriterleri

- Payload gözlemi yapılmış, sonucu `data-sources.md`'de yazılı. Settings.json backup'tan birebir geri dönmüş.
- Yeni öğeler paletten eklenebiliyor ve önizlemede, `tokenradar statusline` çıktısında doğru görünüyor.
- `items` içinde yeni alan yokken bütün mevcut render testleri aynı çıktıyı veriyor.
- `daily_budget`, `burn_rate` ve yeni biçimlerin sınır durumları testli: pencere yok, kalan 0, kısa süre, dar terminal.
- EN ve TR ekran görüntüleri kayıtlı.

## Doğrulama

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
- `pnpm -C app check`, `pnpm -C app lint`, `pnpm -C app test`
- Statusline soğuk başlatma ölçümü (`docs/perf/`).
- Debug build ve portable mod ile CDP üzerinden EN ve TR ekran görüntüleri (`img/12m-*.png`).

## Devir notu

_Adım bitince doldurulur._
