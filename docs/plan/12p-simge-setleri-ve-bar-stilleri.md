# Adım 12p — Statusline simge setleri ve Nerd bar stilleri

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1.x | 12o | 1–2 oturum | ⬜ |

## Amaç

Statusline öğelerinin metin etiketlerini (`git`, `ctx`, `5h`…) isteğe bağlı olarak fonttan çizilen simgelerle değiştirmek ve iki yeni bar stili eklemek: Nerd Font'un progress glifleriyle `capsule` ve fonttan bağımsız `solid`.

Bu adım, 12o ile bu makineye kurulan gerçek Nerd Font ile görsel olarak doğrulanır.

## Önce oku

- ADR-044, ADR-049, ADR-051, ADR-053, ADR-055
- Adım 12o'nun devir notu (font tespiti, kurulu sürüm, terminal türü)
- `crates/tokenradar-core/src/statusline/line/prefs.rs` (`SegmentKind`, `BarStyle::glyphs`, `clean`, TOML okuma/yazma)
- `crates/tokenradar-core/src/statusline/line/render.rs` (`label`, `default_label`, `bar`, `bar_sample`, `window`, `compact`, `fit`)
- `crates/tokenradar-core/src/statusline/line/tests/v12m.rs` (bar-glif genişlik testi, `unicode-width` `cjk`)
- `docs/data-sources.md` (glif genişlik gözlemleri)

## Kullanıcıyla kararlaştırılanlar (2026-11-08)

- **Kapsam bütün öğeler,** yalnızca `git` değil. Kullanıcının verdiği `git` örneği kapsamı anlatmak içindi.
- **`nerd` seti bugün etiketi olmayan öğelere de simge verir** (model, version, cwd, session_id, agent_name, lines_changed…). Simgenin asıl faydası çıplak değerleri ayırt etmek.
- **Progress barlar Nerd Font ile güzelleştirilir;** fonttan bağımsız bir stil de eklenir.

## Kapsam

### 1. Tercih

- `[statusline] icons = "text" | "unicode" | "nerd"`, varsayılan `text`. Bilinmeyen değer `text`'e düşer (ADR-042 esnek okuma).
- **Öncelik:** kullanıcının etiketi > setin simgesi > Fluent metin etiketi. `unicode` setinde simgesi olmayan öğe metin etiketine düşer. Etiketi yalnızca boşluk olan öğe (ADR-049) simge de göstermez.
- Simgeler dilden bağımsızdır, Fluent'e girmez: `SegmentKind::icon(IconSet) -> Option<&'static str>`.
- Düzenleyici: genel ayarlarda simge seti seçici. Powerline teması seçilince `nerd` önerilir. 12o'nun tespiti font yok diyorsa `nerd` seçeneğinin yanında uyarı gösterilir.

### 2. Dokunulmayanlar (dürüst sayılar)

- `≈` (tahmin) ve dim çizim hiçbir sette değişmez.
- `cost` ve `burn_rate`'teki `API` kelimesi kalır: bu tutar abonelik faturası değil, API fiyatıyla hesap. Simge bu anlamı taşıyamaz.
- `session_share`'deki "bu oturum" ifadesi kalır.

### 3. Öğe-simge tablosu

Genişlik: **N** = East Asian Width'te Ambiguous değil. PUA (Nerd) glifleri Unicode'da hep **A**'dır; Mono fontta tek hücre çizilir, bu adımda gerçek fontla doğrulanır.
Cascadia: glif Windows Terminal'in varsayılanı Cascadia Mono'da var mı? Yoksa sistem yedek fontu (Segoe UI Symbol) çizer. (2026-11-08 cmap taraması.)

| Öğe | Bugün (metin) | `unicode` | Genişlik / Cascadia | `nerd` (Octicons, Nerd Fonts ≥ 3.0) |
|---|---|---|---|---|
| `five_hour` | `5h` / `5s` | (metin kalır) | | `oct-clock` U+F43A |
| `seven_day` | `7d` / `7g` | (metin kalır) | | `oct-calendar` U+F455 |
| `scoped` | model adı (`Fable`) | (metin kalır) | | (metin kalır) |
| `context` | `ctx` | (metin kalır) | | `oct-stack` U+F51E |
| `tokens_in` | `in` / `girdi` | `⇡` U+21E1 | N / ✓ | `oct-arrow_up` U+F431 |
| `tokens_out` | `out` / `çıktı` | `⇣` U+21E3 | N / ✓ | `oct-arrow_down` U+F433 |
| `duration` | `time` / `süre` | `◷` U+25F7 | N / ✓ | `oct-stopwatch` U+F520 |
| `api_duration` | `api` | (metin kalır) | | `oct-cloud` U+F4AC |
| `cache_ratio` | `cache` | (metin kalır) | | `oct-cache` U+F49B |
| `git_branch` | `git` | `⎇` U+2387 | N / yedek font | `oct-git_branch` U+F418 |
| `project_dir` | (yok) | `⌂` U+2302 | N / ✓ (Consolas'ta da) | `oct-repo` U+F401 |
| `cwd` | (yok) | `›` U+203A | N / ✓ (Consolas'ta da) | `oct-file_directory` U+F413 |
| `session_id` | (yok) | `#` | ASCII | `oct-hash` U+F4DF |
| `model` | (yok) | — | | `oct-cpu` U+F4BC |
| `version` | (yok) | — | | `oct-tag` U+F412 |
| `agent_name` | (yok) | — | | `oct-hubot` U+F477 |
| `session_name` | (yok) | — | | `oct-note` U+F4F6 |
| `output_style` | (yok) | — | | `oct-paintbrush` U+F48F |
| `lines_changed` | (yok, `+156 -23`) | — | | `oct-diff` U+F440 |
| `daily_budget` | (yok, `%x/gün`) | — | | `oct-goal` U+F4DE |
| `burn_rate` | (yok, `API $x/sa`) | — | | `oct-flame` U+F490 (`API` kalır) |
| `compact` | değerde `⟲` | `⟲` kalır | N / yedek font | `⟲` yerine `oct-fold` U+F48C |
| `session_share`, `cost`, `vim_mode` | | — | | — (değer kendini anlatıyor) |
| Değer içi `⏱` (geçen zaman) | | `⏱` kalır | | `oct-hourglass` U+F4E3 |
| Değer içi `▲` (projeksiyon) | | kalır | | kalır |

Tablonun gerekçeleri:

- **Tek aile: Octicons.** FA, Material ve Codicons'u karıştırmak farklı çizgi kalınlıkları demek. Octicons bir geliştirici aracına uyuyor ve hepsi BMP'de. Material (`nf-md-*`, U+F0000+) UTF-16'da vekil çift ister, conhost'ta sorun çıkarabilir.
- **Hepsi Nerd Fonts 3.0.0'da var** (glyphnames.json ile doğrulandı). `cod-git_branch` ve `cod-robot` 3.0.0'da yok, bu yüzden seçilmedi.
- **`unicode` seti bilerek küçük.** Yalnızca anlamı güçlü ve Ambiguous olmayan glifler var. `↑↓` Ambiguous olduğu için `⇡⇣` seçildi; `●○◑▦` aynı nedenle yok. `5h`, `7d` ve `ctx` için iyi bir Unicode sembolü yok, metin daha net.
- **Ok yönü:** girdi `⇡` (gönderilen), çıktı `⇣` (alınan); ağdaki yükleme/indirme alışkanlığı.
- **`⏱` yalnızca `nerd`'de değişir.** `⏱` bazı fontlarda çift genişlikte çiziliyor (ADR-044 bedeli). `oct-stopwatch` `duration`'a ayrıldığı için geçen zaman `oct-hourglass` alır.
- **`⟲` ve `⎇` yedek fonttan gelir.** Cascadia ve Consolas'ta yoklar; Windows Terminal'de Segoe UI Symbol çiziyor. `⟲` bugün de böyle (12n).

### 4. Bar stilleri

Yeni iki `BarStyle`:

1. **`capsule` (Nerd Fonts ≥ 3.4):** U+EE00–EE05 progress glifleri (Fira Code kökenli): sol uç, orta ve sağ uç; her birinin boş ve dolu hali var. Kenarları yuvarlak, çerçeveli bir kapsül.
   - 3.0–3.3'te bu glifler yok. 12o'nun tespiti < 3.4 diyorsa düzenleyici uyarır.
   - **Zaman işareti (ADR-044):** bugün `bar()` işareti iki hücre arasına ek hücre olarak sokuyor. Kapsülde bu çerçeveyi böler. Öneri: işaret ek hücre olmaz, düştüğü hücre `Paint::Plain` (parlak) çizilir. Gerçek fontla görülüp karar verilir ve ADR'ye yazılır.
   - Renk kapalıyken bu stil `blocks`'a düşer: boş ve dolu hücre yalnızca şekilden ayırt edilir, bunun okunur olup olmadığı görülerek karar verilir.
2. **`solid` (font gerektirmez):** Hücreler boşluk karakteri; doluluk arka plan rengiyle (dolu: durum rengi, boş: koyu gri). Glif yok, genişlik riski yok. Renk kapalıyken ve Powerline bloklarının içinde `blocks`'a düşer.

- `bar_sample` her iki stili de çizer; düzenleyicinin stil seçicisi onları gösterir.
- 12m'deki "Ambiguous olmayan glif" kuralı `capsule` için istisna alır: PUA her zaman A'dır, bunu `theme = powerline` zaten kabul etmişti. ADR-056 yazar.

## Kapsam dışı

- Font kurulumu ve terminal ayarı (Adım 12o).
- Kullanıcının kendi simge tablosunu tanımlaması (öğe başına etiket zaten var).
- Tray, panel ve mod'da simge.
- Sürekli animasyon, ör. U+EE06–EE0B spinner glifleri (ADR-019).
- Kurulu sürümü güncellemek ya da sürüm çıkarmak.

## Görevler

- [x] Prefs: `IconSet`, `icons` anahtarı (okuma/yazma, eski build uyumu), `SegmentKind::icon`; testler.
- [x] Render: `label()` önceliği, `compact()` ve `window()` içindeki `⟲`/`⏱` değişimi; testler.
- [x] Bar: `BarStyle::{Capsule, Solid}`, `bar()` ve `bar_sample`; işaret davranışı; testler.
- [x] Düzenleyici: simge seti seçici, bar stilleri, uyarılar; Fluent (EN/TR); bindings; örnek veri.
- [x] Gerçek görsel kontrol (aşağıda) ve kararların ADR-056'ya yazılması.
- [x] Ölçüm: CLI boyutu ve statusline gecikmesi (`docs/perf/`).
- [x] Ekran görüntüsü onayı ve devir notu.

## Teknik notlar ve tuzaklar

- **Genişlik:** `unicode-width` (cjk kapalı) PUA'yı 1 sayar. Mono olmayan Nerd Font varyantlarında simgeler komşu hücreye taşabilir. Etiketten sonra gelen boşluk bu taşmayı emer, ama `fit()` ve sağa yaslı bölgeler (ADR-051) gerçek fontla denenmeli.
- **Eski build uyumu:** `icons` anahtarını bilmeyen bir build onu atlar ve metin etiketi çizer. Bu kabul edilebilir.
- **Etiket uzunluğu:** Simge tek karakter; `MAX_LABEL` değişmez.
- **Powerline blokları:** Simge bloğun metin renginde çizilir; `capsule` blok içinde çizilmez (bugün bar da blokta çizilmiyor).

## Kabul kriterleri

- `icons = "nerd"` ile tablodaki her öğe simgesini, `unicode` ile tanımlı olanlar simgesini, diğerleri metin etiketini çiziyor; kullanıcı etiketi her zaman önde.
- `≈`, dim, `API` ve "bu oturum" hiçbir sette değişmiyor.
- `capsule` ve `solid` doğru doluluğu ve zaman işaretini gösteriyor; renk kapalıyken `blocks`'a düşüyor.
- Gerçek Nerd Font ile Windows Terminal ve VS Code'da hiza bozulmuyor.

## Doğrulama

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
- `pnpm -C app check`, `pnpm -C app lint`, `pnpm -C app test`
- Testler:
  - Her `unicode` glifi `unicode-width` ile (cjk açık ve kapalı) 1 hücre ve Ambiguous değil (12m bar-glif testinin genişletilmesi).
  - Her `nerd` glifi U+E000–F8FF'de (BMP PUA).
  - Simge önceliği ve boşluk etiket kuralı.
- Gerçek görsel kontrol: 12o'da kurulan fontla, Windows Terminal ve VS Code'da, Mono ve Mono olmayan varyant. plain, `powerline`, `capsule`, `solid`; `COLUMNS` 120/80/50; simgenin komşu hücreye taşmadığı.
- Debug build ve portable mod ile CDP üzerinden düzenleyicinin EN ve TR ekran görüntüleri (`img/12p-*.png`).

## Devir notu

**Tarih:** 2026-11-09 · **Karar:** ADR-056 · **Ölçüm:** `docs/perf/2026-11-09-icon-sets.md` · **Ekran görüntüleri:** `img/12p-statusline-en.png` (açık tema, EN; düz tema, Nerd simgeleri, haftalık kapsül, bağlam dolgu), `img/12p-statusline-tr-dark.png` (koyu tema, TR; Powerline, haftalık dolgu seçili ve "yalnızca kapsül" notu)

### Yapılanlar
- **Core `prefs.rs`:** `IconSet` (`text` / `unicode` / `nerd`), `icons` anahtarı (hoşgörülü okuma, varsayılan `text` yazılmaz), `SegmentKind::icon`, `elapsed_mark` ve `compact_mark`. `BarStyle::{Capsule, Solid}` ve `CAPSULE` glif tablosu.
- **Core `render.rs`:**
  - Etiket önceliği: kullanıcı etiketi > set simgesi > Fluent kelimeleri; yalnızca boşluk olan etiket hiçbir şey göstermez.
  - Zaman ve compact işaretleri sete göre. Metin ve Unicode setlerinde `⏱ ` ve `⟲ ` boşlukla gelir (aşağıda, Sapmalar).
  - Kapsül ve dolgu; zaman işareti hücre içinde. Kapsülde dolu glif: düz temada parlak, blokta soluk. Dolguda `|`.
  - Kapsül Powerline bloklarında da çiziliyor. Kullanıcının rengi dolgunun doluluğuna uyarı eşiğinin altında uygulanıyor.
- **Şablonlar** kullanıcının simge setini koruyor.
- **Uygulama:**
  - Simge seti seçici. Powerline + kurulu Nerd Font'ta "Nerd Font simgelerini kullan" önerisi. Font uyarıları (`fontNoteKey`: font yok, Mono değil, kapsül için 3.4 öncesi).
  - Bar stili seçicisi beş seçenekle; dolgu örneği renkli (`glyphRuns`). Powerline'da kapsül dışı stiller için not.
  - Katalog etiketleri ve örnekleri seçili sette. Terminal fontu sayfadaki bütün örneklere ulaşıyor.
- **Araç çubuğu Türkçede de tek satır** (12o düzeltmesinin 12p'de bozulması, ekran görüntüsünde görüldü):
  - "Şablonlar…" başlık satırına taşındı.
  - Dışa ve içe aktar, ipucu ve erişilebilir adı tam cümle olan `IconButton` oldu (kullanıcı kararı).
  - `Segmented` dar sütunda kırılıyor (`flex-wrap`). Glif örneği boşlukları koruyor (`white-space: pre`).
- **Öğe listesi:** Düz temada dolgu çubuğunun hücreleri renkli (`itemRuns`). Önce bütün Powerline satırlarını da boyuyordu (benim yan etkim); yalnızca düz temayla sınırlandı.
- **Fluent (EN/TR):** `statusline-icons-*`, kapsül ve dolgu stil adları, `statusline-capsule-*` uyarıları. Dışa ve içe aktar metinleri ipucu cümlesi oldu. CLI'da `sl-compact-limit-only` işareti `$mark` argümanıyla.
- **Testler:** Çekirdek `tests/v12p.rs` (setler, öncelik, genişlik: Unicode glifleri cjk açık ve kapalı 1 hücre ve Ambiguous değil, Nerd glifleri BMP PUA'da, kapsül ve dolgu hücreleri, işaret, bloklar, şablonlar). Eski testler boşluklu işarete güncellendi. Ön yüz: `fontNoteKey`, `glyphRuns`, `itemRuns`.
- **Dokümanlar:** ADR-056; data-sources (glif gözlemi düzeltmesi); architecture §11 (madde 8 ve yeni madde 11); design-language (sözlük, bileşen notu, araç çubuğu); performance-budget (CLI satırı); perf kaydı; CHANGELOG ve README (`⏱ 43%`).

### Sapmalar
- **Kapsül Powerline bloklarında çiziliyor** (plan: çizilmez). Şekille okunuyor; WT'de görülerek kullanıcıyla karar verildi.
- **`⏱` ve `⟲`'den sonra boşluk** (metin ve Unicode setleri; kullanıcı kararı). Gerçek WT'de iki glif de yedek fonttan geniş çiziliyor ve sonraki rakamın üstüne biniyordu; `⏱` U+FE0E ile bile emoji. Bu, 12k'nin "`⏱` bir hücre kaplıyor" gözlemini düzeltiyor: sayım 1, çizim değil. Nerd seti boşluk almıyor. Dar terminal eşikleri bir sütun kaydı (test `cols(19)` / `cols(18)`).
- **"Renk kapalıyken `blocks`'a düşer" kabul kriterinin etkisi yok:** Çubuklar zaten yalnızca renkle çiziliyor; renk kapalıyken hiçbir stil çizilmiyor.
- **Araç çubuğu düzeni değişti** (Şablonlar başlık satırında, dışa ve içe aktar simge düğmeleri). Plan bunu öngörmüyordu; 12o'nun tek satır kuralı korunsun diye.
- **Gerçek görsel kontrol yalnızca Windows Terminal'de** (kullanıcı, `show.ps1` ile icons, bars ve fit). VS Code terminali ve Mono olmayan varyant görülmedi; kabul kriterinin VS Code kısmı açık (aşağıda).
- **Kurulu uygulama ölçüm ve ekran görüntüleri için kapatıldı,** sonra yeniden başlatıldı (kullanıcı onayıyla). İlk ölçüm denemesi kurulu uygulama açıkken yapıldı: portable CLI, SID'den türeyen pipe adıyla kurulu uygulamaya bağlandı. Çıktı geçersiz sayıldı ve kaydedilmedi, çünkü gerçek bir limit değeri içeriyordu. Okumalar bilinmeyen config dizini yüzünden düştü; gerçek veriye yazılmadı.

### Doğrulama
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` temiz. `cargo test --workspace`: 516 test (çekirdek 394 → 406, CLI 25, uygulama 61); 4 ignored.
- `pnpm -C app check`, `lint` ve `test` temiz (126 → 129 test).
- Boyut: CLI 2.488.320 B (+5.120 B). Statusline p95 açık 13,7–14,1 ms, kapalı 26,5–27,4 ms; `main` ile aynı. Hepsi bütçede.
- Gerçek WT (CaskaydiaMono Nerd Font Mono): üç set × üç tema, beş stil × dört durum × iki tema, `COLUMNS` 120/80/50. Boşluk sonrası `⏱` ve `⟲` rakamın üstüne binmiyor. Nerd glifleri ve kapsül hücresine sığıyor. Satırlar ölçüye sığıyor; kullanıcı onayladı.
- Debug build, portable mod ve CDP ile düzenleyici. Araç çubuğu EN ve TR'de 30 px (tek satır). Ekran görüntüleri `PrintWindow` ile alındı (odak değiştirmeden, Mica dahil); kullanıcıya gösterildi.

### Açık konular
- **VS Code terminali görülmedi.** Kabul kriterinin bu kısmı açık. Aynı kontrol, geçici `show.ps1` ve `.ans` örnekleriyle (silinmişse yeniden üretilmeli) VS Code terminalinde yapılmalı. VS Code'un yedek fontu `⏱`'yi farklı genişlikte çizebilir.
- **Mono olmayan Nerd Font varyantı denenmedi** (bu makinede yok). Düzenleyici uyarıyor.
- **Öğe listesi tek renkli:** Kapsülün işareti orada doluluk renginde, bir boşlukla ayrılmış görünüyor ve kullanım sanılabilir. Önizleme ve terminal renkli. Listede kapsülü renkli çizmek küçük bir iş; karar bekliyor.
- **Mod'un kendi `⏱`'si** (`TokenRadar · Weekly 42% ⏱43%`) boşluksuz; mod 12p kapsamı dışında. Aynı taşma orada da olabilir.
- **Dil ve tema aynı anda değişirse (12p dışı):** CDP'den ikisi aynı milisaniyede değiştirilince `config.toml` "light" yazdı, ama uygulamanın görünümü "dark" kaldı. Tek tek değiştirince doğru. İnsan eliyle olası değil; incelenmedi.
- **Renk karoları radiogroup'unun erişilebilir adı yok** (`aria-label` null). 12p öncesinden mi geldiği kontrol edilmedi.
- **WT'nin yeni kurulan fontu yeniden başlatmadan çizip çizmediği** hâlâ gözlenmedi (WT süreci font kurulumundan sonra başladı).

### Sonraki adıma notlar
- Simge tabloları `SegmentKind::icon` içinde; yeni bir öğe türü eklenirse iki sete de bakın. Unicode setine yalnızca fontta olan, Ambiguous olmayan glif girmeli: WT'de yedek fonttan gelen glif hücresinden taşıyor.
- Ekran görüntüleri için `PrintWindow(PW_RENDERFULLCONTENT)` kullanın: odak gerektirmiyor ve Mica arka planını çiziyor. CDP `Page.captureScreenshot` şeffaf alanları gri bırakıyor.
- Ölçüm ve CLI denemelerinden önce kurulu uygulamanın kapalı olduğunu yolundan kontrol edin; pipe adı kullanıcı başına olduğundan portable CLI de ona bağlanır.
