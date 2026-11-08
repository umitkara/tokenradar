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

- [ ] Prefs: `IconSet`, `icons` anahtarı (okuma/yazma, eski build uyumu), `SegmentKind::icon`; testler.
- [ ] Render: `label()` önceliği, `compact()` ve `window()` içindeki `⟲`/`⏱` değişimi; testler.
- [ ] Bar: `BarStyle::{Capsule, Solid}`, `bar()` ve `bar_sample`; işaret davranışı; testler.
- [ ] Düzenleyici: simge seti seçici, bar stilleri, uyarılar; Fluent (EN/TR); bindings; örnek veri.
- [ ] Gerçek görsel kontrol (aşağıda) ve kararların ADR-056'ya yazılması.
- [ ] Ölçüm: CLI boyutu ve statusline gecikmesi (`docs/perf/`).
- [ ] Ekran görüntüsü onayı ve devir notu.

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

_Adım bitince doldurulur._
