# TokenRadar — Mimari

> Bu doküman sistemin **hedef** mimarisidir. Uygulama sırasında bir karar değişirse önce [decisions.md](decisions.md)
> güncellenir, ardından bu doküman. Veri formatlarının ayrıntısı için [data-sources.md](data-sources.md),
> görsel dil için [design-language.md](design-language.md), sayısal hedefler için [performance-budget.md](performance-budget.md).

## 1. Genel bakış

```
                         ┌──────────────────────── tokenradar-app.exe (tek süreç) ─────────────────────────┐
 Claude Code             │                                                                                  │
 ┌──────────────┐ stdin  │  ┌───────────────┐   IPC (named pipe)   ┌──────────────────────────────────────┐ │
 │ statusLine   │──JSON─▶│  │ tokenradar.exe│ ───────────────────▶ │  IpcServer                           │ │
 │  command     │◀─line──│  │ (CLI, ~1-2MB) │ ◀─────────────────── │                                      │ │
 └──────────────┘        │  └───────────────┘                      │            tokenradar-core           │ │
                         │        (ayrı binary, aynı paket)        │  ┌──────────┐ ┌──────────┐ ┌───────┐ │ │
 ~/.claude/projects/**  ─┼──── notify (debounce) ─────────────────▶│  │ Indexer  │ │ LimitEng.│ │Calib. │ │ │
 ~/.claude/sessions/*   ─┼──── notify ─────────────────────────────▶│  │ Watcher  │ │ Sources  │ │Attrib.│ │ │
 ~/.claude/.credentials ─┼──── (onaylıysa, salt-okunur) ───────────▶│  │ LiveSess.│ │ S1/S2/Est│ │Notify │ │ │
 api.anthropic.com      ◀┼──── UsageApiPoller (onaylı, adaptif) ────│  └────┬─────┘ └────┬─────┘ └───┬───┘ │ │
                         │                                          │       └──── EventBus ─────────┘     │ │
                         │                                          │                 │  Store (SQLite WAL)│ │
                         │                                          └─────────────────┼────────────────────┘ │
                         │  ┌──────────────┐   ┌──────────────────────────────────────▼───────────────────┐ │
                         │  │ Tray (Rust)  │   │ Tauri katmanı: komutlar (tipli), olay köprüsü, pencereler │ │
                         │  │ tiny-skia    │   └──────┬──────────────┬───────────────┬──────────────┬─────┘ │
                         │  │ ikili halka  │          │ istek üzerine │               │              │       │
                         │  └──────────────┘   ┌──────▼─────┐ ┌──────▼─────┐ ┌───────▼─────┐ ┌──────▼─────┐ │
                         │                     │ panel      │ │ main (v0.2)│ │ settings /  │ │ widget     │ │
                         │                     │ (WebView2) │ │ (WebView2) │ │ onboarding  │ │ (v0.3)     │ │
                         │                     └────────────┘ └────────────┘ └─────────────┘ └────────────┘ │
                         └──────────────────────────────────────────────────────────────────────────────────┘
```

**Temel fikir:** Her zaman çalışan şey, küçük bir Rust çekirdeği ve Rust ile çizilen bir tray ikonudur. WebView pencereleri **istek üzerine** oluşturulur ve kapatıldıktan sonra **yok edilir**. Bellek maliyeti yalnızca kullanıcı pencereye bakarken ödenir.

## 2. Süreç modeli ve pencere yaşam döngüsü

| Binary | Alt sistem | Görev |
|---|---|---|
| `tokenradar-app.exe` | Windows GUI | Tray, çekirdek, IPC sunucusu ve pencereleri barındırır. Ürün adı ve kısayollar "TokenRadar" olarak görünür. |
| `tokenradar.exe` | Console | CLI: `statusline`, `status`, `limits`, `sessions`, `doctor`, `statusline install/uninstall`. ~1-2 MB, soğuk başlatma < 15 ms. |

> **Neden iki ayrı ad?** Windows dosya sistemi büyük-küçük harfe duyarsız. `TokenRadar.exe` ile `tokenradar.exe` aynı dizinde
> bulunamaz. Kullanıcının terminale yazacağı kısa ad (`tokenradar`) CLI'a verildi. Installer yalnızca CLI'ı içeren
> `bin\` dizinini kullanıcı PATH'ine ekler.

**Tek örnek (single instance):** `tauri-plugin-single-instance` kullanılır. İkinci bir başlatma yeni süreç açmak yerine paneli açar.

**Başlangıç sırası:**
1. Tray ikonu, son bilinen durumla 300 ms içinde görünür (SQLite'tan okunur).
2. IPC sunucusu dinlemeye başlar.
3. Watcher başlar.
4. **2 saniye sonra** indeksleyici, düşük öncelikli bir arka plan thread'inde çalışmaya başlar. Windows'ta `THREAD_MODE_BACKGROUND_BEGIN` hem CPU hem **I/O önceliğini** düşürür, böylece 11,7 GB indeksleme kullanıcının diğer işlerini yavaşlatmaz.
5. Polling zamanlayıcısı başlar.

**Pencere yaşam döngüsü ("lazy destroy"):**
- Pencereler gizlendikten sonra `warm_ttl` süresi boyunca sıcak tutulur. Süre panel için 60 s, ana pencere için 30 s, ayarlar için 0 s. Süre dolunca webview **yok edilir**.
- Sık açılıp kapanan panel bu sayede anında açılır. Belleğin geri verilmesi ise birkaç saniye gecikir.
- Son webview yok edildiğinde WebView2 tarayıcı süreçlerinin de sonlanması beklenir. **Bu varsayım Adım 02'de ölçülerek doğrulanır.** Doğrulanamazsa ADR-004'te tanımlanan yedek plana (UI'yı ayrı bir sürece bölmek) geçilir.

**Pencereler:** `panel` (tray popover), `main` (analitik, v0.2), `settings`, `onboarding`, `widget` (v0.3). Vite **çok sayfalı** derlenir; her pencere yalnızca kendi bundle'ını yükler.

## 3. Repo düzeni

```
TokenRadar/
├─ Cargo.toml                    # workspace
├─ package.json / pnpm-workspace.yaml
├─ crates/
│  ├─ tokenradar-core/           # domain, store, indexer, providers, limits, calibration, attribution, ipc protokolü
│  ├─ tokenradar-i18n/           # Fluent yükleyici + EN/TR biçimlendiriciler (Rust tarafı)
│  └─ tokenradar-cli/            # bin: tokenradar
├─ app/
│  ├─ src-tauri/                 # bin: tokenradar-app (Tauri kabuğu, tray, pencereler, IPC sunucusu)
│  └─ src/                       # Svelte 5 frontend (çok sayfalı: panel, main, settings, onboarding, widget)
│     ├─ lib/ (components, stores, i18n, ipc-bindings)
│     └─ pages/ (panel/, main/, settings/, onboarding/, widget/)
├─ locales/{en,tr}/*.ftl         # TEK çeviri kaynağı (Rust ve TS ortak kullanır)
├─ testdata/                     # anonimleştirilmiş JSONL / API / statusline fixture'ları
├─ scripts/                      # fixture anonimleştirici, sürüm yardımcıları, ölçüm scriptleri
├─ docs/                         # vizyon, mimari, kararlar, plan
└─ .github/workflows/
```

**Bağımlılık yönü:** `cli → core`, `app → core + i18n`, `core → i18n` (yalnızca bildirim metinleri için). `core`, Tauri'ye **bağımlı değildir**. İleride headless bir daemon veya uzak toplayıcı aynı crate ile yazılabilir.

## 4. Teknoloji yığını

| Alan | Seçim | Not |
|---|---|---|
| Dil | Rust stable (edition 2024), TypeScript | |
| Masaüstü kabuğu | Tauri 2.x (güncel stable) | Plugin'ler: single-instance, autostart, notification, updater, positioner, opener |
| UI | Svelte 5 (runes) + Vite, SvelteKit **yok** | Çok sayfalı SPA'lar |
| Grafik | uPlot (zaman serileri), özel SVG/Canvas (halka, ısı haritası) | Ağır grafik kütüphanesi kullanılmaz |
| Stil | Saf CSS + custom properties (design token'lar) | Tailwind/UI kiti yok, bkz. design-language.md |
| Tipli köprü | `specta` + `tauri-specta` | Rust tiplerinden TS bağlamaları üretilir |
| DB | SQLite (`rusqlite`, bundled), WAL | Migration: `PRAGMA user_version` + sıralı SQL dosyaları |
| JSON | `serde_json` (seçici deserialize), gerekirse `simd-json` | Seçim Adım 02 ölçümüne göre yapılır |
| Ön filtre | `memchr` | |
| Hash | `xxhash-rust` (xxh3-128) | Tekilleştirme anahtarı için |
| Zaman | `jiff` | Saat dilimi farkındalıklı |
| Dosya izleme | `notify` + `notify-debouncer-full` | |
| HTTP | `reqwest` (rustls, minimum feature) | Tauri updater ile paylaşılır, kopya bağımlılık olmaz |
| IPC | `tokio` named pipe (sunucu), `std::fs` + `windows-sys` (CLI istemcisi) | ADR-029; Unix soketi Adım 21 |
| Tray ikonu | Tauri tray API + `tiny-skia` ile bitmap çizimi | |
| i18n | Fluent (`fluent-bundle`, `@fluent/bundle`) | Özel fonksiyonlar (§12) |
| Log | `tracing` + dönen dosya appender | |
| Güvenlik | `zeroize` | Token bellekten silinir |
| Paket yöneticisi / görevler | pnpm + kök `package.json` scriptleri + cargo alias'ları | |
| Kalite | rustfmt, clippy `-D warnings`, cargo-deny, Biome, svelte-check, Vitest, insta (snapshot) | |

Sürümler uygulama anında güncel stable olarak seçilir ve lock dosyalarıyla sabitlenir.

## 5. Çekirdek bileşenler (`tokenradar-core`)

| Bileşen | Sorumluluk |
|---|---|
| `Store` | SQLite bağlantı havuzu: tek yazar thread + okuyucular. Migration'lar, tipli sorgular, saklama ve sıkıştırma işleri. |
| `Discovery` | Hesapları ve kaynak köklerini bulur: `~/.claude`, `CLAUDE_CONFIG_DIR`, kullanıcı tanımlı dizinler, çalışan WSL dağıtımları (v0.3). |
| `Indexer` | Artımlı JSONL okuma (offset), ön filtre, seçici parse, tekilleştirme (UPSERT, "max output kazanır"; eşitlikte sıra bağımsız kural, ADR-026), 5 dakikalık rollup, oturum ve proje çözümleme. Worker'lar parse eder ve parça içinde tekilleştirir; tek yazıcı oturumları, olayları ve `source_files` offset'lerini aynı transaction'da yazar. Son 7 günün dosyaları normal, eskileri arka plan önceliğinde işlenir. |
| `Watcher` | `notify` olaylarını sabit bir pencereyle debounce eder (750 ms) ve indeksleyiciye "kirli dosya" kuyruğu olarak verir. Windows'ta `notify` buffer taşmasını bildirmediği için yakın zamanda değişen dosyalar 30 s'de bir `stat` edilir; ayrıca 10 dakikada bir tam tarama yapılır (ADR-027). |
| `LiveSessions` | `sessions/*.json` dosyalarını okur, pid canlılığını doğrular, `busy`/`idle` durumunu izler. |
| `LimitEngine` | Kaynakları (S1/S2/tahmin) birleştirir. Snapshot'ları ve pencereleri yönetir, burn rate ve projeksiyon hesaplar. |
| `UsageApiPoller` | S2 istemcisi: onay kapısı, token'ı salt-okunur kullanma, adaptif zamanlama, geri çekilme. |
| `StatuslineIngest` | IPC'den ve spool dosyalarından gelen S1 verisini işler. `core-ipc` thread'i push'ları `core-limits`'e iletir; `core-limits` spool'u açılışta ve 5 dakikada bir içe aktarır (ADR-029). |
| `Calibrator` (v0.2) | %/maliyet katsayısı k'yı tahmin eder, güven skorunu ve sapmayı (drift) izler. |
| `Attributor` (v0.2) | Δ%'yi oturumlara dağıtır, "Diğer" kovasını yönetir. |
| `Notifier` | Eşik, projeksiyon ve reset kurallarını uygular. Histerezis ve tekilleştirme yapar, sessiz saatlere uyar. |
| `IpcServer` | CLI ve statusline protokolünü sunar (§11). |
| `EventBus` | `tokio::sync::broadcast<CoreEvent>` (kapasite 256, runtime gerektirmez). Geride kalan alıcı `Resync` alır ve tam durumu `CoreHandle::status()` ile yeniden yükler. Tauri katmanı olayları yalnızca açık pencerelere iletir. |
| `CoreRuntime` | Bileşenleri adlandırılmış `std::thread`'lerde başlatır (`core-watch`, `core-index-<hesap>`, `retention`, `store-writer`) ve `shutdown(timeout)` ile durdurur (ADR-027). |
| `Pricing` | Uygulamaya gömülü, sürümlü fiyat tablosu: model → $/MTok, modele özgü cache okuma çarpanı, uzun istem kademesi, fast mode çarpanı, web search ücreti. Bilinmeyen model aile yedeğiyle fiyatlanır (ADR-025). |

**UI'da hesaplama yapılmaz.** Tauri komutları önceden agregasyonu yapılmış görünüm modelleri döner (`get_panel_snapshot`, `get_overview(range)`, `list_sessions(filter, page)` …). Frontend yalnızca çizer.

## 6. Veri modeli (SQLite şeması v1)

Tüm zamanlar **UTC epoch milisaniye** (`INTEGER`) olarak saklanır. Para birimi **mikro-USD** (`INTEGER`) olarak tutulur.

> **Kanonik şema:** `crates/tokenradar-core/src/store/migrations/0001_init.sql`. Aşağıdaki özet okumayı kolaylaştırır. Adım 03'te taslağa şunlar eklendi:
> - Tüm ilişkilere `FOREIGN KEY`.
> - `usage_5m` ve `notification_log` sütunlarına `NOT NULL`.
> - İndeksler: `ix_sessions_root`, `ix_sessions_project`, `ix_sessions_last`, `ix_5m_acct_bucket`, `ix_calib`.
> - `kv` tablosu `WITHOUT ROWID` oldu.
> - Yeni veritabanında `auto_vacuum = INCREMENTAL`.
>
> Adım 04'te eklenenler (ADR-026):
> - `sessions.title_source` (`ai` | `agent` | `custom`). Başlık yalnızca eşit veya daha yüksek öncelikli bir kaynaktan gelirse değişir.
> - Ana oturumlarda `root_id` NULL'dur; yalnızca subagent'larda ana oturumu gösterir. Ağaç toplamlarında `COALESCE(root_id, id)` kullanılır.
> - `cwd`'si bilinmeyen projelerin `path_norm` değeri `unresolved:<dizin adı>` biçimindedir.
> - Advisor tüketimi ayrı `usage_events` satırlarıdır (anahtar `from_ids_tagged(…, "advisor#<i>")`).

```sql
CREATE TABLE accounts (
  id INTEGER PRIMARY KEY, provider TEXT NOT NULL,            -- 'claude' | 'codex'
  label TEXT NOT NULL, config_dir TEXT NOT NULL,             -- normalize edilmiş yol (WSL için UNC)
  plan TEXT, tier TEXT, environment TEXT NOT NULL DEFAULT 'native',  -- native | wsl:<distro>
  created_at INTEGER NOT NULL, UNIQUE (provider, config_dir));

CREATE TABLE source_files (
  id INTEGER PRIMARY KEY, account_id INTEGER NOT NULL REFERENCES accounts,
  path TEXT NOT NULL UNIQUE, kind TEXT NOT NULL,              -- 'main' | 'subagent'
  size INTEGER NOT NULL, mtime INTEGER NOT NULL, offset INTEGER NOT NULL,
  session_id INTEGER REFERENCES sessions, deleted_at INTEGER);

CREATE TABLE projects (
  id INTEGER PRIMARY KEY, account_id INTEGER NOT NULL, path_norm TEXT NOT NULL,
  display_name TEXT NOT NULL, UNIQUE (account_id, path_norm));

CREATE TABLE sessions (
  id INTEGER PRIMARY KEY, account_id INTEGER NOT NULL, uuid TEXT NOT NULL,  -- subagent: 'agent-<id>'
  kind TEXT NOT NULL,                                       -- 'main' | 'subagent'
  parent_id INTEGER REFERENCES sessions, root_id INTEGER,   -- subagent → ana oturum
  project_id INTEGER REFERENCES projects, title TEXT, title_source TEXT, git_branch TEXT,
  entrypoint TEXT, cc_version TEXT, first_ts INTEGER, last_ts INTEGER,
  UNIQUE (account_id, uuid));

CREATE TABLE models (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE, family TEXT NOT NULL);

CREATE TABLE usage_events (                                 -- ham olaylar (varsayılan 90 gün saklanır)
  id INTEGER PRIMARY KEY, account_id INTEGER NOT NULL, session_id INTEGER NOT NULL,
  model_id INTEGER NOT NULL, ts INTEGER NOT NULL,
  msg_key BLOB NOT NULL,                                    -- xxh3-128(message.id || requestId)
  input INTEGER NOT NULL, output INTEGER NOT NULL, cache_read INTEGER NOT NULL,
  cache_w5m INTEGER NOT NULL, cache_w1h INTEGER NOT NULL, thinking INTEGER NOT NULL,
  web_search INTEGER NOT NULL, web_fetch INTEGER NOT NULL, speed INTEGER NOT NULL,
  cost_micro_usd INTEGER NOT NULL,                          -- API karşılığı maliyet (§9)
  UNIQUE (account_id, msg_key));
CREATE INDEX ix_events_acct_ts ON usage_events (account_id, ts);
CREATE INDEX ix_events_session ON usage_events (session_id, ts);

CREATE TABLE usage_5m (                                     -- kalıcı rollup (hiç silinmez)
  account_id INTEGER, session_id INTEGER, model_id INTEGER, bucket_ts INTEGER,
  input INTEGER, output INTEGER, cache_read INTEGER, cache_w5m INTEGER, cache_w1h INTEGER,
  cost_micro_usd INTEGER, events INTEGER,
  PRIMARY KEY (account_id, session_id, model_id, bucket_ts)) WITHOUT ROWID;

CREATE TABLE limit_snapshots (
  id INTEGER PRIMARY KEY, account_id INTEGER NOT NULL,
  limit_kind TEXT NOT NULL,                                 -- session | weekly_all | weekly_scoped | spend | …
  scope TEXT,                                               -- örn. 'model:Fable'
  percent REAL NOT NULL, resets_at INTEGER, severity TEXT,
  source TEXT NOT NULL,                                     -- 'statusline' | 'api' | 'estimate' | 'codex_log'
  session_uuid TEXT, observed_at INTEGER NOT NULL);
CREATE INDEX ix_snap ON limit_snapshots (account_id, limit_kind, scope, observed_at);

CREATE TABLE limit_windows (
  id INTEGER PRIMARY KEY, account_id INTEGER NOT NULL, limit_kind TEXT NOT NULL, scope TEXT,
  starts_at INTEGER NOT NULL, resets_at INTEGER NOT NULL,
  peak_percent REAL, hit_limit INTEGER NOT NULL DEFAULT 0,
  estimated INTEGER NOT NULL DEFAULT 0,                     -- snapshot yoksa loglardan türetilmiştir
  UNIQUE (account_id, limit_kind, scope, resets_at));

CREATE TABLE surface_breakdown (account_id INTEGER, as_of INTEGER, window_started_at INTEGER,
  key TEXT, percent REAL, PRIMARY KEY (account_id, as_of, key));

CREATE TABLE calibrations (id INTEGER PRIMARY KEY, account_id INTEGER, limit_kind TEXT, scope TEXT,
  k REAL NOT NULL, alpha_json TEXT, confidence REAL NOT NULL, n_obs INTEGER NOT NULL,
  version INTEGER NOT NULL, computed_at INTEGER NOT NULL);

CREATE TABLE attributions (window_id INTEGER, root_session_id INTEGER,   -- NULL = "Diğer"
  est_percent REAL NOT NULL, cost_micro_usd INTEGER NOT NULL, calibration_version INTEGER NOT NULL,
  PRIMARY KEY (window_id, root_session_id));

CREATE TABLE notification_log (id INTEGER PRIMARY KEY, rule TEXT, dedupe_key TEXT UNIQUE, fired_at INTEGER);
CREATE TABLE kv (key TEXT PRIMARY KEY, value TEXT NOT NULL);
```

**Tekilleştirme UPSERT kuralı:**
```sql
INSERT INTO usage_events (...) VALUES (...)
ON CONFLICT (account_id, msg_key) DO UPDATE SET ... WHERE excluded.output >= usage_events.output;
```
Bir olay güncellendiğinde `usage_5m` satırına **fark** uygulanır: eski değer çıkarılır, yeni değer eklenir. Bu, aynı transaction içinde yapılır.

- Olayın kovası, oturumu veya modeli değişmişse eski değer eski satırdan çıkarılır, yeni değer yeni satıra eklenir. Boşalan satır (`events = 0`) silinir.
- Eşit `output` değerinde son görülen satır kazanır.
- **Saklama filigranı:** Saklama işi, hesap başına `kv` tablosuna `retention.pruned_before.<account_id>` kaydını yazar. Bu zamandan eski bir anahtar yeniden gelirse (ör. offset sıfırlanınca eski dosya yeniden okunursa) eklenmez, çünkü `usage_5m` onu zaten sayıyor. Filigrandan sonra eklenen hesaplar geçmişlerini normal biçimde indeksler.

**Hacim tahmini:** Tekilleştirme sonrası ayda ~150 bin olay, yani ~25 MB/ay (indeksler dahil). Ham olaylar 90 gün saklanır (ayarlanabilir), `usage_5m` süresiz tutulur. Ayrıntılar [performance-budget.md](performance-budget.md) içinde.

## 7. Limit modeli ve kaynak füzyonu

```rust
struct LimitObservation {
    account: AccountId,
    kind: LimitKind,          // Session | WeeklyAll | WeeklyScoped | Spend | Other(String)
    scope: Option<Scope>,     // Model("Fable") | Surface(..)
    percent: f64,             // 0..=100+ (aşım mümkün)
    resets_at: Option<Timestamp>,
    severity: Option<String>,
    source: Source,           // Statusline | Api | Estimate | CodexLog
    session_uuid: Option<String>,
    observed_at: Timestamp,
}
```

- **Füzyon kuralı:** Her `(account, kind, scope)` için güncel değer, en yeni gözlemdir. 30 saniye içindeki çakışmalarda öncelik `Statusline > Api > Estimate` sırasıyla belirlenir. Tüm gözlemler (tahminler hariç) `limit_snapshots` tablosuna yazılır. Tahminler yalnızca bellekte tutulur.
- **Bayatlık:** Gözlem yaşı > 10 dk ise UI "son güncelleme X dk önce" gösterir. Yaş > 30 dk ise ve tahmin mümkünse tahmin katmanı devreye girer (kesikli çizgi ve "tahmini" etiketi ile).
- **Reset algılama:** `resets_at` değeri geçtiyse ve yeni gözlem yoksa, limit yerel olarak %0'a çekilir (kaynak: `Estimate`, neden: `reset`) ve reset bildirimi tetiklenir.
- **Burn rate:** Son 15-30 dakikanın gözlemlerinden robust bir eğim hesaplanır (Theil–Sen). Gözlem azsa `k × (son 15 dk maliyet hızı)` kullanılır. **Projeksiyon:** `eta_100 = (100 − p) / rate`. `eta_100 < resets_at − now` ise "reset'ten önce dolacak" uyarısı verilir.

## 8. Pencere modeli

- **5 saatlik pencere:** Kimlik `(account, session, resets_at)` olarak tanımlanır. `starts_at = resets_at − 5h`. Farklı bir `resets_at` değeri taşıyan snapshot yeni bir pencere demektir. `peak_percent` gözlemlenen en yüksek değerdir. `hit_limit` değeri, `percent ≥ 100` gözlemlendiğinde veya API severity bunu gösterdiğinde set edilir.
- **Haftalık döngü:** `weekly_all` ve `weekly_scoped` için `starts_at = resets_at − 7d` (gözlemde reset zamanı hesap bazında sabit, örneğin Çarşamba 07:59:59 UTC).
- **Tahmini pencereler** (snapshot kaydının başlamasından önceki geçmiş için): Loglardan türetilir. Pencere, bir önceki pencere bittikten sonraki ilk olayla başlar ve 5 saat sürer. Başlangıç zamanının tam hizalama kuralı (dakikaya yuvarlama vb.) Adım 14'te gerçek snapshot'larla karşılaştırılarak belirlenir. Bu pencereler `estimated = 1` olarak işaretlenir.

## 9. Ağırlıklı token, kalibrasyon ve atıf

### 9.1 API karşılığı maliyet (ağırlık)

Her olay için şu değer hesaplanır:

```
C = speed_mult(m) × [ p_in(m) × (input + 1.25·cache_w5m + 2.0·cache_w1h + r(m)·cache_read)
                    + p_out(m) × output ]
  + web_search × $0,01                         (p: $/token, gömülü fiyat tablosundan)
```

- `r(m)` cache okuma çarpanıdır: varsayılan 0,10; Fable 5.1 ve Mythos 5.1'de 0,025; Opus 5.5 ve Sonnet 5.5'te 0,05.
- `speed_mult` fast mode'da 2,0'dır. Cache çarpanları bunun üzerine eklenir.
- **Uzun istem kademesi:** Haiku 5.5'te istem (`input + cache_read + cache_w5m + cache_w1h`) 100.000 token'ı aşarsa `p_in` ve `p_out` daha yüksek kademeden alınır.
- `thinking` token'ları `output` içinde sayılır, ayrıca fiyatlanmaz.
- Fiyatlar model bazındadır (ADR-025). `$1/MTok` = 1 mikro-USD/token olduğu için sonuç doğrudan `MicroUsd` olarak yuvarlanır.

`cache_creation` kırılımı yoksa `cache_creation_input_tokens` değeri 5 dakikalık cache yazımı olarak kabul edilir. Çarpanlar ve fiyatlar `Pricing` tablosunda sürümlenir.

**Hipotez H1:** Limit tüketimi C ile orantılıdır (`Δ% ≈ k·ΔC`) ve tüm modeller için aynı k geçerlidir. H1 backtest ile sınanır. Modele bağlı sistematik bir sapma görülürse aile bazlı çarpanlar (`α_f`, normalize edilmiş) öğrenilir: `Δ% ≈ k·Σ_f α_f·ΔC_f`.

### 9.2 Kalibrasyon (v0.2)

- **Gözlemler:** Aynı pencere içindeki ardışık snapshot çiftleri `(t0,p0)→(t1,p1)` ile o aralıktaki yerel `ΔC` değeri.
- **Kuantizasyon:**
  - S1 ondalıklı değer verdiği için tercih edilir.
  - Yalnızca S2 (tam sayı) varsa aralıklar `Δp ≥ 3` olana kadar birleştirilir.
  - `ΔC` değeri, eşiğin (gürültü tabanının) altında kalan aralıklar atılır.
- **Model:** `Δp = k·ΔC + u + ε`. Burada `u ≥ 0` yerel olmayan tüketimdir (claude.ai, Desktop, başka cihaz).
- **Tahminci:** `Δp/ΔC` oranlarının ağırlıklı medyanı kullanılır. Ağırlık `ΔC` ile orantılıdır ve üstel zaman sönümü uygulanır (yarı ömür 3 gün). `u > 0` aralıkları oranı yukarı iter, medyan bu aykırı değerlere karşı dayanıklıdır. Sapma (drift) takibi için CUSUM uygulanır; katsayı ani bir sıçrama yaparsa bu, Anthropic'in limit formülünü değiştirdiğine işaret edebilir ve UI'da bildirilir.
- `k_session`, `k_weekly` ve her `weekly_scoped` için ayrı `k` tutulur.
- **Güven skoru:** gözlem sayısı, MAD/medyan oranı ve tazelik birleştirilerek [0,1] aralığında hesaplanır. Skor < 0,4 ise atıflar "düşük güven" olarak gösterilir.
- **Başlangıç (prior):** Kalibrasyon verisi yokken, mevcut pencerenin anlık oranı kullanılır: `k0 = p_now / C_window`. Bu yaklaşım pencerenin tamamı yerel tüketimden oluşuyorsa iyi bir tahmindir.

### 9.3 Atıf

- **v0.1 (basit, kalibrasyonsuz):** Bir oturumun mevcut penceredeki payı = `p_now × C_oturum / C_pencere_toplam`. Bu, tüm tüketimin yerel olduğu varsayımına dayanan "orantılı dağıtım"dır ve UI'da "≈" işaretiyle gösterilir.
- **v0.2 (aralık bazlı):** `Δp > 0` olan her aralık için:
  - `yerel = min(Δp, k·ΔC_yerel)`
  - Oturum s'nin payı: `yerel × ΔC_s / ΔC_yerel`
  - Kalan `Δp − yerel` değeri **"Diğer"** kovasına gider.
  - Pencere ve hafta toplamları bu paylardan oluşur. Subagent maliyeti `root_id` üzerinden ana oturuma eklenir.
- **Çapraz doğrulama:** Haftalık "Diğer" payı, S2'deki `seven_day_breakdown` içindeki `claude_code` dışı satırlarla karşılaştırılır. Fark > 10 puan ise kalibrasyon şüpheli kabul edilir.
- Atıflar `calibration_version` ile saklanır. Yeni kalibrasyon sürümü geldiğinde yeniden hesaplama tembel (lazy) yapılır: yalnızca görüntülenen pencereler hesaplanır.

## 10. Polling politikası

S2 yalnızca **açık onay** varsa kullanılır.

| Durum | Aralık |
|---|---|
| Taze S1 verisi var (< 3 dk) | 15 dk (yalnızca scoped limitler ve kırılım için) |
| En az bir canlı oturum `busy` ve S1 yok | 2 dk |
| Canlı oturumlar `idle` | 5 dk |
| Canlı oturum yok | 15 dk |

- **Ek tetikleyiciler:** Panel açıldığında, son S2 verisi 60 sn'den eskiyse istek atılır. Ayrıca bir `resets_at` zamanı geçtikten 30 sn sonra ve uygulama başlangıcında tetiklenir.
- **Kısıtlar:** İki istek arasında en az 60 sn olmalı. Her aralığa ±%10 jitter eklenir.
- **Geri çekilme:** 429 ve 5xx yanıtlarında aralık 60 s'den başlayıp 30 dk'ya kadar katlanarak artar (`Retry-After` varsa ona uyulur).
- **401:** Token bayat olarak işaretlenir ve istek atılmaz. `.credentials.json` mtime'ı değişince tekrar denenir.
- **Ağ yok:** Çevrimdışı durumda polling durur. İşletim sisteminden ağ değişikliği olayı gelince devam eder.

## 11. IPC protokolü ve CLI

- **Taşıma (ADR-029):** Sunucu tarafında `tokio` named pipe, CLI tarafında runtime'sız senkron istemci kullanılır. Windows'ta `\\.\pipe\tokenradar-<kullanıcı-SID-hash>`, Unix'te `$XDG_RUNTIME_DIR/tokenradar.sock` (yoksa `~/.local/state/tokenradar/…`).
- **Güvenlik:**
  - Named pipe için **açık bir DACL** tanımlanır: yalnızca mevcut kullanıcı SID'i erişebilir. Windows'un varsayılan DACL'i *Everyone* grubuna okuma izni verdiği için bu adım şart.
  - `PIPE_REJECT_REMOTE_CLIENTS` bayrağı set edilir.
- **Çerçeveleme:** Satır sonuyla ayrılmış JSON. İstek `{"v":1,"id":7,"op":"…","data":{…}}`, yanıt `{"v":1,"id":7,"ok":true,"data":{…}}` biçimindedir.
- **İşlemler (v1):** `hello` (sürüm müzakeresi), `statusline.push` (yanıt beklenmez), `status.get`, `limits.get`, `sessions.live`.
- **`tokenradar statusline` akışı (p95 < 30 ms):**
  1. stdin okunur (en fazla 1 MB) ve minimal alanlar parse edilir.
  2. Daemon'a 50 ms zaman aşımıyla bağlanılır ve `statusline.push` gönderilir.
  3. Satır stdin verisiyle biçimlendirilir; varsa daemon'dan gelen ekstra bilgiyle (oturumun pencere payı, projeksiyon) zenginleştirilir.
  4. Daemon yoksa veri `spool/statusline-YYYYMMDD.jsonl` dosyasına eklenir (günlük en fazla 5 MB). Uygulama açıldığında spool dosyaları içe aktarılır. Böylece **uygulama kapalıyken bile % geçmişi kaybolmaz**.
  5. Zincirleme modu (`--chain`): Kullanıcının önceki statusline komutu aynı stdin ile çalıştırılır ve çıktısı bizim segmentimizle birleştirilir. Zaman aşımı 300 ms'dir.
  6. Yüzdeler ve reset süreleri stdin'deki payload'dan, projeksiyon ve oturum payı uygulamadan gelir (ADR-029).
- **Diğer komutlar:**
  - `tokenradar status [--json]`, `limits`, `sessions`: Uygulama çalışmıyorsa SQLite'tan salt-okunur okunur.
  - `doctor`: Anonim bir tanılama paketi üretir.
  - `statusline install|uninstall`: `settings.json` dosyasını yedekler, değişikliği diff olarak gösterir, atomik olarak yazar ve önceki komutu korur. Yalnızca `statusLine` üyesinin baytları değişir; `install → uninstall` dosyayı bayt düzeyinde geri getirir (ADR-029).

## 12. i18n mimarisi

- **Tek kaynak:** `locales/{en,tr}/*.ftl`. Rust tarafı (`tokenradar-i18n`: bildirimler, tray tooltip'i, CLI) ve TS tarafı (UI) aynı dosyaları kullanır. CI testi iki dilde **anahtar paritesini** ve değişken tutarlılığını doğrular.
- **Özel Fluent fonksiyonları (Rust ve TS'de birebir aynı davranış):**
  - `NUM(n, decimals)`: yerel ayara göre gruplama ve ondalık ayırıcı. TR: `1.234,5`, EN: `1,234.5`.
  - `PCT(n)`: TR `%72`, EN `72%`.
  - `DUR(ms, style)`: TR `1 sa 48 dk`, EN `1h 48m`.
  - `TR_SUFFIX(n, case)`: Türkçe sayı ekleri. Örnekler: `5'te`, `6'da`, `%72'si`, `10'dan`, `3'e`. Ek, sayının okunuşunun son hecesine göre seçilir (bir, iki, üç … on, yirmi … yüz, bin, milyon).
  - Parite garantisi: `testdata/i18n-vectors.json` dosyasındaki ortak test vektörleri hem Rust hem TS testlerinde çalıştırılır.
- **Tarih ve saat:**
  - TS tarafında `Intl.DateTimeFormat` ve `Intl.RelativeTimeFormat` kullanılır.
  - Rust tarafında EN/TR için küçük, el yazımı biçimlendiriciler bulunur. Bunlar da ortak test vektörleriyle doğrulanır. ICU verisi gömülmez, çünkü binary boyutunu artırır.
  - TR her zaman 24 saat kullanır. EN, işletim sistemi bölge ayarını izler.
- **Dil seçimi:** İşletim sistemi dili (`sys-locale`) `tr` ile başlıyorsa TR, aksi halde EN seçilir. Kullanıcı ayarlardan değiştirebilir. Değişiklik yeniden başlatmadan uygulanır.
- **Büyük-küçük harf:** `<html lang>` her zaman ayarlanır. JS'de `toUpperCase()` yerine `toLocaleUpperCase(lang)` kullanılır. Böylece `i → İ` ve `ı → I` dönüşümleri doğru çalışır.

## 13. Güvenlik ve mahremiyet

- **Telemetri yok.** Ağ trafiği yalnızca şu iki hedefe gider:
  - `api.anthropic.com` (S2 onayı varsa)
  - GitHub Releases (güncelleme kontrolü; ayarlardan kapatılabilir)
- **Token:** Bkz. data-sources.md S2/S5. Token yalnızca istek anında okunur, `zeroize` edilir, loglanmaz ve IPC'den geçmez. `refreshToken` asla okunmaz.
- **İçerik:** Konuşma içeriği (`message.content`) **asla** okunmaz, saklanmaz ve gösterilmez. Yalnızca meta veri ve kullanım bilgisi işlenir. Oturum başlığı için yalnızca Claude Code'un ürettiği başlık ve ad alanları kullanılır.
- **Tauri:**
  - Pencere başına minimal izin (capabilities) tanımlanır.
  - CSP `default-src 'self'` olarak ayarlanır. Uzak içerik yüklenmez, `eval` kullanılmaz.
  - `opener` yalnızca beyaz listedeki URL'leri açar.
- **Dosya değişikliği:** Uygulamanın dışarıda değiştirdiği tek dosya `settings.json`'dır (statusline kurulumu). Bu işlem her zaman kullanıcı tıklamasıyla yapılır; öncesinde yedek alınır, diff gösterilir ve dosya atomik olarak yazılır.
- **Güncelleme:** Tauri updater minisign imzasıyla güncellemeleri doğrular. Özel anahtar yalnızca GitHub Secrets'ta tutulur.
- **Tedarik zinciri:** Lock dosyaları commit'lenir. `cargo-deny` (lisans ve advisory kontrolü) ve `pnpm audit` CI'da çalışır. Bağımlılık eklemek bir gerekçe gerektirir.

## 14. Konfigürasyon ve dosya konumları

| Öğe | Windows | macOS | Linux |
|---|---|---|---|
| Veri (DB, spool, cache) | `%LOCALAPPDATA%\TokenRadar\` | `~/Library/Application Support/TokenRadar/` | `$XDG_DATA_HOME/tokenradar/` |
| Ayarlar | `%LOCALAPPDATA%\TokenRadar\config.toml` | aynı dizin | `$XDG_CONFIG_HOME/tokenradar/config.toml` |
| Loglar | `…\TokenRadar\logs\` (günlük dönen, 7 gün, toplam ≤ 20 MB) | | |

- **Portable mod:** Exe'nin yanında `portable` dosyası varsa tüm veriler exe'nin yanındaki `data\` dizinine yazılır. Not: Windows toast bildirimleri AppUserModelID gerektirir; portable modda bildirimler sınırlı olabilir.
- `config.toml` dosyası `schema_version` alanı taşır ve geriye dönük uyumlu şekilde migrate edilir. Bilinmeyen alanlar korunur.

## 15. Gözlemlenebilirlik ve hata yönetimi

- `tracing` ile yapılandırılmış loglama yapılır. Varsayılan seviye `info`; ayarlardan `debug` seçilebilir. Token ve içerik redaksiyonu yalnızca disiplinle değil, **tipler üzerinden** zorunlu kılınır: `Secret<T>` tipi `Debug`/`Display` çıktısında `***` basar.
- **Parser şema sapması:** Tanınmayan kayıt biçimleri sayılır. Eşik aşılırsa UI'da "Claude Code formatı değişmiş olabilir" uyarısı gösterilir ve tanılama verisi toplanır.
- **Hata politikası:** Tek bir dosyanın, satırın veya API isteğinin başarısız olması uygulamayı asla durdurmaz. Hatalar sayılır ve `doctor` çıktısında raporlanır.
- **`tokenradar doctor`:** Sürümler, yollar, dosya ve olay sayıları, son hatalar, ayarlar (sırlar hariç) ve performans ölçümleri tek bir zip dosyasında toplanır.

## 16. Provider soyutlaması

> **Adım 04 notu:** Claude provider'ı şimdilik serbest fonksiyonlar olarak yazıldı (`providers::claude::{discover, classify_path, LineParser}`). Aşağıdaki trait, ikinci provider'ın (Codex) ihtiyaçları belli olunca Adım 20'de çıkarılır. Tek uygulamadan trait tasarlamak arayüzü yanlış yerde dondurur.

```rust
pub trait Provider: Send + Sync {
    fn id(&self) -> ProviderId;                                        // "claude" | "codex"
    fn discover(&self, env: &Env) -> Vec<AccountDescriptor>;           // hesaplar + kaynak kökleri
    fn classify_path(&self, path: &Path) -> Option<SourceKind>;        // main | subagent | ignore
    fn parse_line(&self, line: &[u8], ctx: &mut ParseCtx) -> ParseOutcome;
        // → UsageEvent | LimitObservation | SessionMeta | Skip | Unknown
    fn live_sessions(&self, acct: &Account) -> Result<Vec<LiveSession>>;
    fn limit_pollers(&self, acct: &Account) -> Vec<Box<dyn LimitPoller>>; // Claude: S2; Codex: yok
    fn pricing(&self) -> &PricingTable;
}
```

Codex'in `parse_line` metodu `LimitObservation` da üretebilir (`source = CodexLog`). Çekirdek hiçbir yerde provider'a özgü alanlara dokunmaz. Claude'a özgü her şey `providers/claude/` altında kalır.

## 17. Test stratejisi

- **Fixture'lar:** `scripts/sanitize-jsonl` gerçek bir dosyadan içeriği siler (`message.content`, imzalar, yollar hash'lenir) ve yalnızca kullanım ile meta veriyi bırakır. Gerçek kullanıcı verisi **asla** commit'lenmez. Aşağıdaki durumlar için ayrı fixture'lar bulunur:
  - Tekrar eden ve kısmi kayıtlar
  - Yarım satır
  - Truncate
  - Subagent'lar
  - Eski ve yeni API şeması
  - Statusline'da eksik pencere
- **Birim testleri:** Parser, tekilleştirme, rollup farkları, pencere türetme, füzyon, polling zamanlayıcısı (sahte saat), kalibrasyon (sentetik veri + gürültü), atıf korunumu (pay toplamı ≡ Δp), i18n vektörleri.
- **Snapshot testleri (insta):** Parse çıktıları ve görünüm modelleri.
- **Benchmark'lar (criterion):** İndeksleme throughput'u, artımlı güncelleme gecikmesi. Sonuçlar CI'da eşiklerle karşılaştırılır.
- **Ölçüm scriptleri:** `scripts/measure-*.ps1` ile RSS, WebView2 süreç sayısı ve açılış gecikmesi ölçülür; bütçe tablosuyla karşılaştırılır.
- **UI:** Vitest (mantık ve store'lar) ve svelte-check. Uçtan uca test (tauri-driver) v0.2 sonrasında değerlendirilecek.
