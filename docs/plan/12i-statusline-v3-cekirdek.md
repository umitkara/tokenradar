# Adım 12i — Statusline v3 çekirdeği: öğe modeli, biçimler, renkler, Powerline, yeni segmentler

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1.x | 12h | 1-2 oturum | 🟨 |

## Amaç

Kullanıcı, statusline'ın `tylergraydev/claude-code-tool-manager` içindeki "Status Line Builder" kadar ayrıntılı özelleştirilebilmesini istiyor. Bu adım, bunun için gereken çekirdeği kurar: config modeli, render, yeni segmentler, şablonlar, dışa ve içe aktarma, IPC. Düzenleyici arayüzü Adım 12j'dedir.

Referans proje segment listesinden bir Python script üretiyor. Biz üretmeyiz: render zaten core'da (ADR-042). CLI yeni config'i okur. Böylece Python bağımlılığı olmaz, p95 bütçesi korunur ve settings.json'a ek yazma gerekmez.

## Önce oku

- ADR-013, ADR-019, ADR-029, ADR-041, ADR-042, ADR-044
- `docs/data-sources.md` S1 (payload alanları)
- `docs/architecture.md` §11 (statusline akışı, p95 < 30 ms)
- `docs/performance-budget.md` (CLI soğuk başlatma ve statusline bütçeleri)
- `docs/design-language.md` (renk token'ları, `≈` kuralı, EN/TR sözlük)
- Adım 12c'nin devir notu (özellikle `NO_COLOR` açık konusu)
- `crates/tokenradar-core/src/statusline/{payload.rs, line.rs, line/render.rs}`, `config.rs`, `crates/tokenradar-cli/src/render.rs`

## Kullanıcıyla kararlaştırılanlar (2026-11-07)

- **Dört özellik grubunun tamamı istendi:**
  - Segment başına biçim, etiket ve renk.
  - Serbest düzen: satır sonu, ayraç, özel metin.
  - Yeni segmentler.
  - Powerline teması.
- **Limit renkleri:** Kullanıcı yalnızca "ok" rengini seçer. Uyarı (75) ve kritik (90) renkleri her zaman kazanır (ADR-042 eşikleri).
- **Git:** Yalnızca dal adı gösterilir; `.git/HEAD` doğrudan okunur. Süreç çalıştırılmaz, `git status` yok.
- **Galeri yok (ADR-013).** Yerine yerleşik şablonlar ve TOML metniyle dışa ve içe aktarma gelir.

## Kapsam

### 0. Önkoşul gözlemi: `NO_COLOR`

Claude Code'un statusline komutuna `NO_COLOR` verip vermediği kod yazılmadan önce gözlenir.

- **Kurulum:** Kullanıcı onayıyla, settings.json akışı üzerinden (diff → onay → backup → atomic write) geçici bir prob komutu kurulur.
- **Probun işi:**
  - Mevcut TokenRadar komutunu zincirler, ingest kesilmez.
  - Ortamdaki `NO_COLOR`, `TERM` ve `COLORTERM` değerlerini bir dosyaya yazar.
  - Renkli bir test satırı basar.
- **Gözlem bitince** backup'tan geri dönülür.
- **Sonuca göre varsayılan:**
  - `NO_COLOR` gelmiyor → `color = "auto"` varsayılan.
  - Geliyor ama ANSI çiziliyor → statusline'da varsayılan `"always"`, ayrıca `"never"` seçeneği. Kullanıcıya sorulur.
  - ANSI çizilmiyor → kapsam kullanıcıyla yeniden konuşulur.

### 1. Config: `[statusline]` öğe modeli

```toml
[statusline]
theme = "plain"            # plain | powerline | powerline_round
separator = "│"            # yalnızca plain temada
color = "auto"             # gözleme göre: auto | always (| never)
segments = [...]           # eski sürümler için türetilip yazılır

[[statusline.items]]
kind = "seven_day"
format = "bar"             # bar | percent
reset = true
pace = true
color = "cyan"             # limitlerde yalnızca ok rengi
label = "7d"               # boşsa Fluent varsayılanı
```

- **Geriye uyum:** `items` yoksa öğeler `segments`, `layout`, `bar` ve `pace` alanlarından türetilir (`double` → limit ve oturum grupları arasına `line_break`). Bugünkü satır birebir aynı kalır.
- **Esnek okuma (ADR-042):** Bilinmeyen `kind`, `format` veya `color` atlanır ya da varsayılana döner. Okuma hiçbir zaman başarısız olmaz.
- **Yazma:** `save_statusline_prefs` bilinmeyen anahtarları korur ve `segments`'i türetip yazar.
- **Palet:** 16 ANSI rengi + `default`.
- **Temizlik:** `label`, `text` ve `separator` içindeki kontrol karakterleri (C0, ESC, C1) atılır, uzunluk sınırlanır.

### 2. Segmentler

| Grup | Öğe | Biçimler |
|---|---|---|
| Limit | `five_hour`, `seven_day` | `bar` / `percent`, `reset`, `pace` |
| Limit | `scoped` | kompakt |
| Oturum | `context` | `percent` / `bar` / `fraction` |
| Oturum | `session_share` | (değer yoksa gizli) |
| Oturum | `cost` | 2 / 4 ondalık; **`API` işareti kaldırılamaz** |
| Oturum | `model` | `short` / `full` |
| Oturum | `tokens_in`, `tokens_out` | `compact` / `full` |
| Oturum | `duration`, `api_duration` | `short` / `hms` |
| Oturum | `lines_changed` | `both` / `net` |
| Oturum | `vim_mode`, `version`, `agent_name` | — |
| Oturum | `session_id` | `short` / `full` |
| Proje | `cwd` | `basename` / `short` (`~`) / `full` |
| Proje | `project_dir` | `basename` / `full` |
| Proje | `git_branch` | — |
| Düzen | `separator`, `line_break`, `custom_text` | — |

**`git_branch` kuralları:**
- Payload'da `worktree.branch` varsa o kullanılır.
- Yoksa cwd'den yukarı doğru `.git` aranır. Derinlik sınırlıdır.
- `.git` bir dosyaysa `gitdir:` izlenir.
- Detached HEAD'de 7 karakterlik SHA gösterilir.
- Hata olursa segment gizlenir.

### 3. Payload

Aşağıdaki alanlar resmî belgede var (code.claude.com/docs/en/statusline, 2026-11-07):
- `workspace.project_dir`
- `cost.total_duration_ms`, `total_api_duration_ms`, `total_lines_added`, `total_lines_removed`
- `context_window.total_input_tokens`, `total_output_tokens`, `context_window_size`
- `vim.mode`, `agent.name`, `worktree.branch`

Kurallar:
- Hepsi `skip_serializing` ile eklenir. Push ve spool biçimi değişmez.
- İçerik alanı okunmaz.
- `docs/data-sources.md` S1 bu alanlarla güncellenir.

### 4. Render

- **`Paint`:** Semantik boyalar korunur; isteğe bağlı `fg` ve `bg` eklenir. Limitlerde `Ok` yerine kullanıcı rengi kullanılır, `Warn` ve `Crit` kazanır. `≈` ve soluk kuralları aynı kalır.
- **Plain tema:** Ayraç yapılandırılabilir.
- **Powerline ve `powerline_round`:**
  - Arka plan bloklarıyla çizilir, geçişlerde U+E0B0 / U+E0B4 kullanılır.
  - Limit bloğunun arka planı severity'ye göre değişir.
  - Renk kapalıyken plain temaya düşer.
- **`fit()`:**
  - Öğe listesine ve çok satıra genelleşir. Her satır ayrı ölçülür.
  - Düşme sırası: önce oturum ve proje öğeleri sondan düşer, sonra limit detayları, en son limitler; ilk limit her zaman kalır.
  - Ayraç ve `custom_text`, komşusuyla birlikte düşer.
- **`to_ansi`:** 30–37, 90–97, 40–47 ve 100–107 SGR kodları.

### 5. Şablonlar, dışa ve içe aktarma

- **`statusline::presets`:** `minimal`, `default`, `detailed`, `powerline`, `two_line`.
- **`export_toml`:** Yalnızca `[statusline]` tablosunu üretir.
- **`import_toml`:** Esnek ayrıştırıcıyı kullanır, yok sayılan anahtarları uyarı listesi olarak döndürür.

### 6. IPC

- `set_statusline_prefs` yeni modelle çalışır.
- `preview_statusline(prefs, sample, width)`:
  - `sample` değerleri: `normal` / `warn` / `crit` / `stale`.
  - Span'ler fg/bg ve `item` indeksi taşır.
- Yeni komutlar: `list_statusline_presets`, `export_statusline`, `import_statusline` (kaydetmez).
- Binding'ler yeniden üretilir. `StatuslineLook.svelte` yeni modelle derlenecek kadar uyarlanır; asıl arayüz 12j'dedir.

### 7. Fluent ve dokümanlar

- `cli.ftl` dosyasına yeni `sl-*` anahtarları, `settings.ftl` dosyasına öğe, biçim, renk, tema ve şablon adları eklenir. EN ve TR eşit olur.
- ADR-049 yazılır.
- Güncellenecek yerler: architecture §11, design-language sözlüğü, README `[statusline]` örneği, CHANGELOG (Unreleased).

## Kapsam dışı

- Düzenleyici arayüzü ve ayarlar sayfası (Adım 12j).
- `git status` ve git süreci çalıştırmak.
- Ağdan galeri.
- 256 renk veya hex renk.
- Ayarlanabilir renk eşikleri (ADR-042: ileride üç yüzey için birlikte).
- Kurulu sürümü güncellemek veya sürüm çıkarmak.

## Görevler

- [x] `NO_COLOR` gözlemi (kullanıcıyla, 2026-11-07). Sonuçlar:
  - Gerçek Claude Code oturumunda (Windows Terminal, komut PowerShell 5.1 üzerinden) statusline komutunun ortamı: `NO_COLOR` yok, `FORCE_COLOR` yok, `TERM=xterm-256color`, `COLORTERM` yok, `COLUMNS` terminal genişliği.
  - ANSI ön plan, arka plan, parlak renk ve soluk (2) kodlarının hepsi çizildi.
  - Karar: `color = "auto"` varsayılan kalır.
  - Yan gözlem: PowerShell 5.1 probu stdin'i UTF-8 BOM ile aktarınca kurulu v0.1.0 "henüz limit verisi yok" dedi. Prob yaklaşık 1 dakika kurulu kaldı; bu sürede okumalar büyük olasılıkla kayboldu.
  - BOM'a dayanıklılık bu adımın kapsamında değil. Devir notuna aday olarak yazılacak.
- [ ] Payload alanları ve S1 dokümanı; push/spool biçim testleri değişmeden geçer.
- [ ] `StatuslinePrefs` öğe modeli, geriye uyum türetmesi, esnek okuma, temizlik, yazma.
- [ ] Segment render'ları ve biçimleri; `git_branch` okuyucu.
- [ ] Renkler, Powerline temaları, genelleştirilmiş `fit()`, `color` modu.
- [ ] Şablonlar, `export_toml` ve `import_toml`.
- [ ] IPC komutları, `Line::sample` varyantları, binding'ler.
- [ ] Fluent (EN/TR), ADR-049, dokümanlar.
- [ ] Performans ölçümü (`main`'e karşı A/B), `docs/perf/` kaydı.

## Teknik notlar ve tuzaklar

- **Config okuma soğuk başlatma bütçesine sığmalı.** Öğe dizisi büyüdükçe ayrıştırma maliyeti ölçülür.
- **Geriye uyum testi:** 12c'nin bütün render testleri, `items` yokken birebir aynı çıktıyı vermeli.
- **Powerline glifleri** Nerd Font ister. Font yoksa kutucuk görünür; bu, 12j önizlemesinde not olarak söylenir.
- **Dar terminal:** Powerline blokları daha fazla yer kaplar, `fit()` testi iki tema için de yazılır.
- **Gerçek spool ve pipe'a dokunulmaz:** Testler portable kopya ve geçici dizinlerle yapılır.
- **Gerçek veriyle ölçüm:** Ölçüm commit'lerine token toplamı, sayı veya yol yazılmaz.

## Kabul kriterleri

- `items` olmayan bir config ile satır, 12c ile birebir aynıdır.
- Her öğe ve biçim beklenen çıktıyı verir. Uyarı ve kritik renkleri kullanıcı rengini ezer.
- Powerline renk kapalıyken plain temaya düşer. Kontrol karakterleri çıktıya geçmez.
- Dışa aktar → içe aktar gidiş-dönüşü aynı prefs'i verir. Bozuk TOML hata basmaz.
- Statusline p95 < 30 ms, ölçüm kayıtlı.
- Ingest davranışı değişmez.

## Doğrulama

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
- `pnpm -C app check`, `pnpm -C app lint`, `pnpm -C app test`
- `release-cli` portable kopyasıyla sentetik payload'lar üzerinde görsel kontrol: EN/TR, plain/powerline, `NO_COLOR`, `COLUMNS=50`, çok satır, her şablon.
- `scripts/` altındaki statusline ölçüm betiği ile p95 ve CLI boyut farkı.

## Devir notu

_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
