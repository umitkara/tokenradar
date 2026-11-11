# Adım 12q — Statusline payload alanları: efor, prompt cache, harcama limiti, PR ve depo

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1.x | 12p | 1–2 oturum | ⬜ |

## Amaç

Claude Code'un statusline stdin JSON'una son sürümlerde eklenen ama bizim okumadığımız alanları satıra taşımak. Bunlar efor seviyesi, düşünme ve hızlı mod, prompt cache istatistikleri, gateway harcama limiti, açık PR ve depo kimliği.

Bütün değerler payload'dan doğrudan okunur ya da payload'dan saf hesapla türetilir. Bu adımda ağ isteği, git süreci, store veya daemon bağımlılığı yoktur.

Rakip araçlarla karşılaştırma (2026-11-11):
- [claude-code-tool-manager](https://github.com/tylergraydev/claude-code-tool-manager): 22 segmentin tamamı bizde var, yalnızca `git_status` eksik. O da Adım 12r'de.
- [ccstatusline](https://github.com/sirmalloc/ccstatusline): Cache Timer, Cache Misses, Thinking Effort ve Spend Limit widget'ları bu adımın ilham kaynağıdır.

## Önce oku

- ADR-013, ADR-042, ADR-049, ADR-053, ADR-056, ADR-059
- Adım 12m'nin devir notu (prob akışı, `Zone`/`Clock`, açık konu: `prompt_cache.hit_ratio`) ve 12p'nin devir notu (simge setleri)
- `docs/data-sources.md` S1 (12m gözlemi: hangi yollar görüldü)
- Resmi şema: <https://code.claude.com/docs/en/statusline> ("Available data", "Prompt cache fields", "Spend limit fields")
- `crates/tokenradar-core/src/statusline/{payload.rs, line.rs}`, `statusline/line/{prefs.rs, render.rs, presets.rs}`
- `app/src/pages/settings/statusline/{AddPalette.svelte, ItemEditor.svelte}`, `app/src/pages/settings/view.ts`

## Kullanıcıyla kararlaştırılanlar (2026-11-11)

- **Kapsam:** Araştırmada önerilen öğelerin hepsi ("hepsini içeren bir plan hazırla"). Saf stdin öğeleri bu adımda; canlı yenileme, geri sayım ve git durumu Adım 12r'de.
- **Cache süresi** kullanıcının fikri. Bu adımda yenileme gerektirmeyen **saat** biçimiyle gelir (`→14:32`); geri sayım 12r'de.
- **Gözlenemeyen alanlar belgeye dayanır:** `spend_limit` ve gözlenemezse `pr.*`, resmi şemadan sentetik fixture ile uygulanır. Bu, 12m'deki "gözlemsiz kod yok" kuralından bilinçli bir sapmadır (ADR-059).
- **Ağ gerektiren widget'lar yok** (ADR-013'ün gereği, kullanıcıya bildirildi): Claude servis durumu ve GitHub CI durumu.

## Kapsam

### 0. Önkoşul gözlemi: gözlenmemiş yollar

12m'deki prob şunları zaten gördü: `effort.level`, `thinking.enabled`, `fast_mode`, `prompt_cache.*` (`last_miss_cause` null idi), `workspace.repo.*`, `workspace.added_dirs`. Şu yollar henüz gözlenmedi:

| Yol | Görünme koşulu | Gözlem planı |
|---|---|---|
| `prompt_cache.last_miss_cause.causes` | Oturumda en az bir cache kaçırma | Uzun bir oturumda prob |
| `pr.{number, url, review_state, kind}` | Dalın açık bir PR'ı var | Kullanıcı onaylarsa, açık PR'ı olan bir dalda prob |
| `worktree.*`, `workspace.git_worktree` | Worktree oturumu ya da bağlı worktree | Geçici bir `git worktree add` dizininde prob |
| `rate_limits.spend_limit.*` | Claude apps gateway arkasında | **Gözlenemez** (bu hesap gateway arkasında değil) |

- **Prob kuralları 12m ile aynı:** diff → kullanıcı onayı → backup → atomic write. Mevcut komut zincirlenir. Değer değil, yalnızca anahtar yolu ve JSON tipi yazılır. Bitince backup'tan geri dönülür ve SHA-256 karşılaştırılır.
- **Gözlenemeyen alanlar belgeye dayanır (ADR-059).** Resmi şemadaki örnek ve açıklamadan sentetik fixture üretilir. Esnek okuma zorunludur, `data-sources.md`'de "belgeye dayalı, gözlenmedi" diye işaretlenir. 12m'deki "gözlemsiz kod yok" kuralından bilinçli bir sapmadır.

### 1. Payload (`payload.rs`)

- Yeni alanlar:
  - `effort.level`, `thinking.enabled`, `fast_mode`
  - `prompt_cache.{warm, caching_observed, ttl, expires_at, misses, hit_ratio, last_miss_cause.causes, recache_tokens_if_cold}`
  - `rate_limits.spend_limit.{used_percentage, resets_at, used_usd, limit_usd, period}`
  - `pr.{number, review_state, kind}`
  - `worktree.name`, `workspace.git_worktree`, `workspace.repo.{host, owner, name}`, `workspace.added_dirs` (yalnızca uzunluk)
- **Hepsi `skip_serializing`.** Push ve spool satırının biçimi değişmez, uygulama bu alanları görmez. Harcama limitini limit motoruna taşımak kapsam dışıdır.
- **`pr.url` okunmaz.** Bağlantı (OSC 8) kapsam dışı olduğu için gerek yok.
- `added_dirs` dizisinin içeriği değil yalnızca uzunluğu tutulur (selective serde, `IgnoredAny` sayacı).
- Esnek okuma (ADR-042): Bilinmeyen `effort.level`, `review_state` ya da `ttl` değeri öğeyi gizlemez, ham metin olarak gösterilir. Sayısal alanda tip uyuşmazlığı alanı `None` yapar, payload'ın tamamını düşürmez.

### 2. Yeni öğeler (`SegmentKind`)

| `kind` | Grup | Örnek | Biçimler | Kaynak |
|---|---|---|---|---|
| `effort` | Oturum | `high`, `max ⚡` | `level` (varsayılan), `flags` | `effort.level`, `fast_mode`, `thinking.enabled` |
| `cache_timer` | Oturum | `cache →14:32 1h`, `cache soğuk` | `clock` (varsayılan); `countdown` 12r'de | `prompt_cache.{warm, ttl, expires_at}` |
| `cache_misses` | Oturum | `2 kaçırma · araçlar değişti` | `count`, `cause` (varsayılan) | `prompt_cache.{misses, last_miss_cause}` |
| `recache_cost` | Oturum | `soğursa 45k` | `short` | `prompt_cache.recache_tokens_if_cold` |
| `spend_limit` | Limit | `$ %42`, `$21/$50` | `bar`, `percent`, `remaining`, `amount` | `rate_limits.spend_limit.*` |
| `pr` | Proje | `#123 ✓`, `!45 taslak` | `short` (varsayılan), `full` | `pr.*` |
| `worktree` | Proje | `⎇ feature-x` | — | `worktree.name`, yoksa `workspace.git_worktree` |
| `repo` | Proje | `TokenRadar`, `owner/TokenRadar` | `name`, `full` (varsayılan), `host` | `workspace.repo.*` |

**Mevcut öğelere yeni biçimler:**

| Öğe | Yeni biçim | Örnek |
|---|---|---|
| `model` | `with_effort` | `Opus 5.5 · high` |
| `cache_ratio` | `session` (12m açık konusu) | `cache %91 (oturum)`, `prompt_cache.hit_ratio` |
| `project_dir`, `cwd` | `with_added` | `TokenRadar +2` |

**Öğe kuralları:**
- **`effort`:**
  - Alan yoksa öğe gizlenir; model efor desteklemiyor demektir, "bilinmiyor" değildir.
  - Renk: `low` soluk, `medium` normal, `high`/`xhigh` vurgu, `max` uyarı rengi.
  - `flags` biçimi `fast_mode` açıksa `⚡`, düşünme kapalıysa `düşünme yok` ekler. Simgeler simge setine (ADR-056) göre değişir: text, unicode, nerd.
- **`cache_timer`:**
  - `caching_observed = false` ise gizlenir.
  - `warm` ve `expires_at` doluysa ve `expires_at > now` ise sıcak: yerel saat (12m `Clock`) ve TTL etiketi. `expires_at - now ≤ 60 s` ise uyarı rengi.
  - Aksi hâlde soğuk: soluk `soğuk`.
  - **Saat biçimi yenileme gerektirmez:** satır boşta donsa bile saat doğru kalır. Saat 12m'deki gibi yalnızca Windows'ta hesaplanır; diğer platformlarda `countdown` yoksa `TTL` etiketine düşer.
- **`cache_misses`:**
  - `misses = 0` iken gizlenir (`count` biçimi dahil).
  - Neden adları Fluent'tan gelir: `tools_changed`, `system_prompt_changed`, `ttl_expired_5m`, `likely_server_side`. Bilinmeyen neden ham adla, birden fazla neden ilk ad ve `+N` ile gösterilir.
- **`recache_cost`:**
  - `null` iken (compact'tan hemen sonra) gizlenir.
  - `≈` almaz: değer Claude Code'un hesapladığı bir sayıdır, metin koşulu söyler ("soğursa").
  - Cache soğumuşsa da gösterilir, çünkü bir sonraki isteğin bedelidir.
- **`spend_limit`:**
  - Limit öğelerinin renk eşiklerini ve sıfırlanma biçimlerini (`reset_format`) paylaşır. `amount` biçimi `used_usd`/`limit_usd` eksikse `percent` biçimine düşer.
  - Dar terminalde limit grubuyla birlikte en son düşer.
- **`pr`:**
  - `kind = mr` ise numara `!` ile, değilse `#` ile yazılır.
  - İnceleme durumu: `approved` ✓ iyi rengi, `changes_requested` ✗ kritik, `pending` • uyarı, `draft` soluk `taslak`. Durum yoksa yalnızca numara.
- **`worktree`:** Ana çalışma ağacında gizlenir.
- **`repo`:** `origin` yoksa gizlenir. Kullanıcı adı terminalde görünür ama hiçbir yere yazılmaz.
- **Hesaplar render'da yapılmaz.** `now` ile kıyas, neden seçimi ve TTL etiketi `Line`/`Facts` içinde hazırlanır, `render.rs` yalnızca biçimlendirir.

### 3. Model, geriye uyum ve şablonlar

- `config.toml` `version = 2` olarak kalır. Yeni öğeler ve biçimler isteğe bağlıdır.
- Eski sürümler bilinmeyen `kind` ve `format` değerlerini atlar (12i, 12k ve 12m ile aynı).
- **Şablonlar:**
  - `detailed` şablonuna `effort` ve `cache_timer` eklenir.
  - `minimal` dahil diğer şablonlar değişmez. Model biçimi varsayılanda `with_effort` olmaz, mevcut satırlar aynı kalır.
- **Örnek veri:** `Line::sample_in` bütün yeni öğeleri doldurur; önizleme dört durumda da dolu görünür. Örnekte bir PR ve bir kaçırma nedeni vardır.

### 4. Düzenleyici

- **Ekleme paleti:** Yeni öğeler gruplarında görünür. `spend_limit` öğesinin ipucu "yalnızca gateway arkasında" der.
- **Öğe editörü:** Biçim seçicisi yeni biçimleri backend'den listeler. `cache_timer`'da `countdown` seçeneği 12r'ye kadar yoktur.
- **Fluent (EN/TR):** Öğe adları ve ipuçları, efor seviyeleri, kaçırma nedenleri, `soğuk`/`cold`, `soğursa`/`if cold`, `taslak`/`draft`, `düşünme yok`/`no thinking`, `(oturum)`/`(session)`.
- `docs/design-language.md` sözlüğüne efor, cache süresi ve kaçırma terimlerinin EN/TR karşılıkları eklenir.

## Kapsam dışı

- `cache_timer` için `countdown` biçimi, `refreshInterval` ve `git_status`: Adım 12r.
- Harcama limitini tray, panel ve limit motoruna taşımak. Bu iş limit füzyonuna dokunur, ayrı adım gerekir.
- PR bağlantısı (OSC 8 hyperlink) ve PR/CI durumunu `gh` ile sorgulamak (ADR-013).
- Claude servis durumu widget'ı (`status.anthropic.com`, ADR-013).
- Alt ajan statusline'ı (`subagentStatusLine`) ve alt ajan eforu.
- Prompt cache kaçırmalarını store'a yazıp oturum ekranında göstermek. Atıf motoruna (Adım 14) not olarak düşülür.
- Kurulu sürümü güncellemek ya da sürüm çıkarmak.

## Görevler

- [ ] Prob: gözlenmemiş yollar için kurulum, gözlem ve geri dönüş; sonucu `data-sources.md` S1'e yaz, gözlenemeyenleri işaretle.
- [ ] Core: `payload.rs` alanları, esnek okuma, `added_dirs` sayacı; resmi şema örneğinden sentetik fixture.
- [ ] Core: `SegmentKind`, `Format`, block renkleri, `default_label`, simge setleri, esnek okuma ve yazma; testler.
- [ ] Core: `Line`/`Facts` türetimleri (cache durumu, neden seçimi, harcama limiti, PR durumu); testler.
- [ ] Render: yeni öğeler ve biçimler, `fit()` sırası, iki tema ve üç simge seti; testler.
- [ ] Şablonlar ve örnek veri.
- [ ] Bindings (`tauri-specta`) ve düzenleyici: palet, öğe editörü, Fluent, `view.test.ts` `KINDS` listesi.
- [ ] İlk çalışan hâlin ekran görüntüsü kullanıcıya gösterilir ve onaylanır, ardından cila.
- [ ] Statusline ölçümü, ADR-059 sonuçları ve devir notu.

## Teknik notlar ve tuzaklar

- **`warm` ile `expires_at` aynı şey değil.** Son yanıtta cache token'ı yoksa `warm = false` ve `expires_at = null` gelir; `caching_observed` ise `true` kalabilir. Üç alan birlikte değerlendirilir.
- **`expires_at` epoch saniyesidir,** `resets_at` ile aynı birimde. 12m'deki `Clock` ve `Zone` yeniden kullanılır.
- **`effort` yokluğu anlamlıdır:** model efor parametresini desteklemiyor. Boş yer tutucu gösterilmez.
- **`ultrathink`, skill ya da alt ajan eforu** `effort.level` değerinde görünmez (resmi şema). İpucu metni bunu söyler: "oturum eforu".
- **`spend_limit` alanları farklı zamanlarda gelir** (resmi şema, "Spend limit fields"). `used_usd` ve `limit_usd` yokken `percent` ya da `bar` biçimi çalışır.
- **Kişisel veri:** Repo sahibi, PR numarası ve worktree adı yalnızca terminale yazılır. Push, spool ve log'a girmez. Fixture'larda sahte adlar kullanılır (`testdata/README.md`).
- **Glif genişliği:** `✓ ✗ • ⚡ ⎇` için `unicode-width` 1 ve Ambiguous olmama testi (12m tuzağı). Uygun değilse text setine düşülür. `⚡` emoji sunumlu olabilir, nerd ve unicode setlerinde test zorunludur.
- **Soğuk başlatma bütçesi:** Payload struct'ı büyüyor. Statusline ölçümü `docs/perf/` altına yazılır (p95 ≤ 30 ms daemon açık, ≤ 40 ms kapalı).
- **Gerçek veriye dokunulmaz:** Testler portable kopya ve geçici dizinlerle yapılır. Prob çıktısı değer içermez.

## Kabul kriterleri

- Prob yapılmış, gözlenen ve gözlenemeyen yollar `data-sources.md`'de yazılı, `settings.json` birebir geri dönmüş.
- Yeni öğeler paletten eklenebiliyor, önizlemede ve `tokenradar statusline` çıktısında doğru görünüyor.
- Yeni alanlar yokken bütün mevcut render testleri aynı çıktıyı veriyor.
- Sınır durumları testli: efor yok, cache hiç gözlenmedi, sıcak, son dakika, soğuk, kaçırma yok, bilinmeyen neden, `recache` null, harcama limitinde eksik dolar alanları, PR durumu yok, GitLab MR, worktree yok, dar terminal.
- EN ve TR ekran görüntüleri kayıtlı.

## Doğrulama

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
- `pnpm -C app check`, `pnpm -C app lint`, `pnpm -C app test`
- Statusline soğuk başlatma ölçümü (`scripts/measure-statusline.ps1`, sentetik payload yeni alanları taşır) → `docs/perf/`.
- Debug build ve portable mod ile CDP üzerinden EN ve TR ekran görüntüleri (`img/12q-*.png`).

## Devir notu

_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
