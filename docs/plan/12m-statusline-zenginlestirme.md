# Adım 12m — Statusline zenginleştirme: yeni öğeler ve biçimler

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1.x | 12l | 1–2 oturum | ✅ |

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

- [x] Prob: kurulum, gözlem, geri dönüş; sonucun `data-sources.md`'ye yazılması.
- [x] Core: `SegmentKind`, `Format`, `Item` alanları, esnek okuma, yazma; testler.
- [x] Core: `Line`/`Facts` türetimleri (`daily_budget`, `burn_rate`, gözlenen alanlar); testler.
- [x] Render: yeni biçimler, çubuk genişliği ve stili, `clock`, `fit()` sırası; iki tema için testler.
- [x] Şablonlar ve örnek veri.
- [x] Bindings (`tauri-specta`) ve düzenleyici: palet, öğe editörü, Fluent.
- [x] İlk çalışan hâlin ekran görüntüsü kullanıcıya gösterilir ve onaylanır, ardından cila.
- [x] ADR-053 ve devir notu.

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

**Tarih:** 2026-11-08 · **Karar:** ADR-053 · **Ölçüm:** `docs/perf/2026-11-08-statusline-enrichment.md` · **Ekran görüntüleri:** `img/12m-statusline-en.png` (açık tema, EN), `img/12m-statusline-tr-dark.png` (koyu tema, TR); ikisinde de yeni öğeler eklenmiş ve haftalık limit 12 hücre noktalı çubuk ve saat biçimiyle seçili

### Yapılanlar
- **Prob:** Kullanıcı onayıyla kuruldu (diff → onay → backup → atomic write), mevcut komutu zincirledi ve yalnızca anahtar yolu ile tipi kaydetti. `output_style.name` ve `context_window.current_usage.*` gözlendi; beş öğenin hepsi kaldı. `settings.json` backup'tan birebir geri döndü (SHA-256 aynı). Sonuç `docs/data-sources.md` S1'de.
- **`payload.rs`:** `output_style.name` ve `context_window.current_usage` (girdi, cache yazma, cache okuma) eklendi; ikisi de `skip_serializing`, push ve spool biçimi değişmedi.
- **`prefs.rs` (çekirdek):**
  - Yeni türler: `daily_budget` (Limit grubu), `burn_rate`, `session_name`, `cache_ratio`, `output_style`.
  - Yeni biçimler: `remaining` (limitler ve bağlam), `tokens_left` (bağlam).
  - `ResetFormat`, `BarStyle` ve `BAR_WIDTHS`; `Item` alanları `reset_format`, `bar_width`, `bar_style`. Esnek okuma, normalize etme ve yalnızca varsayılandan farklı değerleri yazma.
- **`line.rs`:** `Zone` (sistem ya da sabit ofset), `Clock`, `Budget`; `Line::daily_budget`, `burn_rate`, `tokens_left`, `reset_clock`. Windows'ta yerel saat `SystemTimeToTzSpecificLocalTime` ile (`windows-sys` `Win32_System_Time`). `Facts` oturum adı, output style, cache oranı ve kullanılan bağlamı taşıyor. Örnek veri yeni öğeleri dolduruyor.
- **`render.rs`:** Yeni arm'lar ve biçimler, çubuk genişliği ve stili (`Look`), saat biçimi (Fluent gün kısaltmaları), `fit()`'te 12 → 8 hücre adımı. `bar_sample(style)` editör örnekleri için.
- **Şablonlar:** `detailed` şablonuna `daily_budget` eklendi.
- **Testler:** `tests/v12m.rs` (12 test): günlük bütçe hesabı ve sınırları (pencere yok, kalan 0, uyarı, eski veri, tahmin), tüketim hızı (5 dakika eşiği), payload türetimleri, kalan biçimleri, saat biçimi (EN/TR, sabit ofset, dar terminal), sistem saat dilimi, çubuk genişlikleri ve stilleri, genişten dara çubuk, glif genişliği (`width_cjk`), esnek okuma ve gidiş-dönüş, şablon ve örnekler. Mevcut 335 çekirdek testi değişmeden geçti; toplam 347.
- **Uygulama:** `StatuslineKind` artık `reset_formats` ve `bar_styles` (stil + çekirdeğin çizdiği örnek) taşıyor; bindings yenilendi.
- **Düzenleyici:** Biçim `bar` iken çubuk genişliği ve stil seçicileri, sıfırlanma açıksa limit pencerelerinde sıfırlanma biçimi seçicisi. `Segmented` seçenekleri isteğe bağlı bir glif örneği alıyor (`--font-term`).
- **Fluent (EN/TR):** Öğe adları ve ipuçları, biçim adları, `left`/`kaldı`, `/day`/`/gün`, `/h`/`/sa`, `cache`, gün kısaltmaları, seçici etiketleri.
- **`scripts/measure-statusline.ps1`:** Sentetik payload yeni alanları taşıyor.
- **Dokümanlar:** ADR-053, data-sources S1 gözlemi, perf kaydı ve bütçe satırları, design-language (Segmented, düzenleyici, sözlük), CHANGELOG.

### Sapmalar
- **Saat biçimi yalnızca Windows'ta.** Windows dışında `clock` geri sayıma düşüyor; v0.1 yalnızca Windows'u destekliyor. Saat dilimi veritabanı gömülmedi (ADR-053).
- **İnce ve noktalı stilde zaman işareti `|`.** `│` Ambiguous genişlikte; `blocks` bugünkü çıktıyı korumak için `│`'yu tutuyor.
- **`daily_budget` yuvarlama:** 10 ve üstü tam sayı, altı tek ondalık (`9.4%/day`). Plan biçimi `%9/gün` diye örneklemişti.
- **`StatuslineKind.reset_formats` bir liste,** plandaki gibi bir bayrak değil: dördüncü `bool` alanı clippy'nin `struct_excessive_bools` kuralına takıldı. Liste `formats` ve `bar_styles` ile aynı biçimde; seçenekler backend'den geliyor.
- **Stil seçicinin glif örneği için `Segmented`'e `glyphs` seçeneği eklendi.** Örnekler UI fontunda çizilince ince stil kare gibi görünüyordu.
- **Ölçümde kurulu uygulama kapatıldı** (kullanıcı onayıyla), ardından yeniden başlatıldı.
- **Prob kurulmadan önceki kuru denemede** sentetik bir payload yanlışlıkla kurulu `tokenradar.exe`'ye gitti (kullanıcıya bildirildi). Store'da limit içermeyen bir oturum gözlemi kalmış olabilir; zararsız.

### Doğrulama
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` temiz; `cargo test --workspace` 455 test (çekirdek 347).
- `pnpm -C app check`, `lint` ve `test` temiz (120 test).
- Statusline ölçümü: açık p95 12,3–13,0 ms, kapalı p95 21,3–30,7 ms (bir koşuda tekrarlanmayan 62,2 ms), CLI 2.459.136 B (`main`'den +18.432 B). Hepsi bütçede.
- Debug build, portable mod ve `TOKENRADAR_FAKE_LIMITS=panel:no-five-hour` ile CDP üzerinden (kurulu uygulama kapalıydı) yeni öğeler paletten eklendi; haftalık limitte 12 hücre, noktalı stil ve saat biçimi, bağlamda ince stil seçildi. Önizleme EN ve TR'de doğru çizdi. Ekran görüntüleri kullanıcıya gösterildi ve onaylandı.

### Açık konular
- **Gerçek satır yeni sürüme kadar değişmez.** Kurulu v0.1.0 CLI yeni öğeleri ve alanları atlıyor. Eski bir build dosyayı yeniden yazarsa bunlar kaybolur (12i ve 12k ile aynı).
- **Ekran görüntüsünde görülen iki ayrıntı** (kullanıcı olduğu gibi bıraktı): seçili öğenin metinsiz boşluk span'i tonlu görünüyor (12l davranışı); noktalı stilin boş hücreleri (`⣀`) soluk duruyor.
- **`prompt_cache.hit_ratio`** oturum geneli bir cache oranı sunuyor. Belgesiz; gerekirse ayrı bir öğe ya da `cache_ratio` biçimi olabilir.
- **Varsayılan output style'ın adı doğrulanmadı** (prob değer kaydetmiyor); satır adı olduğu gibi gösteriyor.
- **Bağlamın `fraction` biçimi `total_input_tokens` kullanıyor;** bu alan oturum boyunca birikimli olabilir. `current_usage` girdi toplamı daha doğru olabilir; gerçek bir oturumda kıyaslanmalı.
- **Çubuk genişlikleri (`BAR_WIDTHS`) frontend'de tekrarlanıyor** (12l'deki sabitler gibi); çekirdek değeri yine kendisi sınırlıyor.
- **12i ve 12l'den kalanlar sürüyor:** Powerline'da `▲` rengi, stdin'de UTF-8 BOM, gerçek Nerd Font ile görsel kontrol, sürüklerken otomatik kaydırma.

### Sonraki adıma notlar
- 12n (compact sayısı ve auto-compact limiti) aynı öğe modelini kullanır. Yeni öğe eklerken `SegmentKind::ALL`, `formats`, block rengi, `default_label`, Fluent anahtarları ve `view.test.ts`'deki `KINDS` listesi birlikte güncellenmeli.
- 12n'nin store/daemon'a bağlı değerleri için `Line`'a yeni yöntemler eklenir; `render.rs` hesap yapmamalı.
