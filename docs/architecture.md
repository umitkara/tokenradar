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
                         │                     │ (WebView2) │ │ (WebView2) │ │ onboarding  │ │ (native)   │ │
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

**Pencereler:** `panel` (tray popover), `main` (analitik, v0.2), `settings`, `onboarding`. Vite **çok sayfalı** derlenir; her pencere yalnızca kendi bundle'ını yükler. `widget` bir WebView penceresi değildir: kendi thread'inde çalışan, `tiny-skia` ve DirectWrite ile çizilen native bir layered window'dur (`app/src-tauri/src/widget/`, ADR-058). Yalnızca kullanıcı açınca vardır.

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
│  └─ src/                       # Svelte 5 frontend (çok sayfalı: panel, main, settings, onboarding)
│     ├─ lib/ (components, stores, i18n, ipc-bindings)
│     └─ pages/ (panel/, main/, settings/, onboarding/)
├─ locales/{en,tr}/*.ftl         # TEK çeviri kaynağı (Rust ve TS ortak kullanır)
├─ integrations/claude-code/     # Claude Code mod'u (tamamlayıcı; kurulum paketine girer, ADR-043, ADR-047)
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
| `terminal_font` | Nerd Font yardımcısı (ADR-055): font klasörlerini tarar (cmap kademesi, mono), sabit sürümü indirir ve SHA-256 ile doğrular, kullanıcı başına kurar ve kaldırır; Windows Terminal ve VS Code ayarlarını `jsonc` ile cerrahi düzenler, yedekten geri alır. Tauri'siz; uygulama yalnızca görünüm modeli ve onay token'ı ekler. |
| `Pricing` | Uygulamaya gömülü, sürümlü fiyat tablosu: model → $/MTok, modele özgü cache okuma çarpanı, uzun istem kademesi, fast mode çarpanı, web search ücreti. Bilinmeyen model aile yedeğiyle fiyatlanır (ADR-025). Aylık plan ücretleri `pricing/plans.toml`'dadır (`subscriptionType` + `rateLimitTier`); config'teki `plan_fee_usd` önce gelir (ADR-065). |
| `Queries` (v0.2) | Geçmiş sayfalarının görünüm modelleri (ADR-065): pencere, oturum ve proje listeleri (keyset sayfalama, imleç `"{anahtar}:{id}"`), detaylar, LTTB'li zaman serisi ve oturum işaretleri, haftanın günü × saat ısı haritası, Genel Bakış. Saklanan atıfı okur, eski sürüm satırı `stale` taşır. Gün ve saat sınırları `LocalZone` ile yerel saattedir. Her değer `Basis` (yöntem, güven, `≈`, silik, eski) taşır. |

**UI'da hesaplama yapılmaz.** Tauri komutları önceden agregasyonu yapılmış görünüm modelleri döner (`get_panel_snapshot`, `get_panel_detail`, `get_overview(range)`, `list_sessions(filter, page)` …). Frontend yalnızca çizer.

## 6. Veri modeli (SQLite şeması v3)

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
>
> Adım 12n'de eklenenler (şema v2, `0002_compact_events.sql`, ADR-054):
> - `compact_events` tablosu ve `ix_sessions_uuid` indeksi.
> - Migration bütün canlı dosyaların okuma konumunu sıfırlar; eski compact'lar bir sonraki taramada tam yeniden indekslemeyle gelir. Tekilleştirme ve saklama filigranı mevcut olayları korur.
> - v2 veritabanını v0.1.0 açamaz (`SchemaTooNew`); migration geri alınmaz.

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

CREATE TABLE attributions (                                 -- v3 (0003_attribution.sql, ADR-064)
  window_id INTEGER NOT NULL REFERENCES limit_windows (id) ON DELETE CASCADE,
  root_session_id INTEGER REFERENCES sessions (id),         -- NULL = "Diğer"
  est_percent REAL,                                         -- NULL = yalnızca maliyet
  cost_micro_usd INTEGER NOT NULL);
CREATE UNIQUE INDEX ux_attributions ON attributions (window_id, IFNULL(root_session_id, 0));

CREATE TABLE attribution_runs (                             -- v3: pencere başına son atıf
  window_id INTEGER PRIMARY KEY REFERENCES limit_windows (id) ON DELETE CASCADE,
  calibration_version INTEGER NOT NULL,                     -- 0 = kalibrasyon yok
  method TEXT NOT NULL,                                     -- calibrated | proportional | estimated | cost_only
  confidence REAL, total_percent REAL, other_percent REAL,
  breakdown_other REAL,                                     -- seven_day_breakdown, claude_code dışı
  suspect INTEGER NOT NULL DEFAULT 0, other_excess INTEGER NOT NULL DEFAULT 0,
  closed INTEGER NOT NULL DEFAULT 0, computed_at INTEGER NOT NULL);

CREATE TABLE notification_log (id INTEGER PRIMARY KEY, rule TEXT, dedupe_key TEXT UNIQUE, fired_at INTEGER);
CREATE TABLE kv (key TEXT PRIMARY KEY, value TEXT NOT NULL);

CREATE TABLE compact_events (                               -- v2: compact sınırları (saklama işi silmez)
  uuid TEXT PRIMARY KEY,                                    -- kaydın uuid'si; --resume kopyaları tekilleşir
  session_uuid TEXT NOT NULL,                               -- kaydın kendi sessionId'si
  at INTEGER NOT NULL, trigger TEXT NOT NULL,               -- 'auto' | 'manual' | 'other'
  pre_tokens INTEGER) WITHOUT ROWID;
CREATE INDEX ix_compact_session ON compact_events (session_uuid);
CREATE INDEX ix_sessions_uuid ON sessions (uuid);
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

- **Füzyon kuralı:** Her `(account, kind, scope)` için öndeki okuma en yeni gözlemdir. 30 saniye içindeki çakışmalarda öncelik `Statusline > Api > Estimate` sırasıyla belirlenir. Öndeki okuma kaynağı, pencereyi ve zamanı verir. Tüm gözlemler (tahminler hariç) `limit_snapshots` tablosuna yazılır. Tahminler yalnızca bellekte tutulur.
- **Tam sayıların birleşimi (ADR-060):** Statusline aşağı, API en yakın tam sayıya yuvarlıyor. Gösterilen yüzde, aynı penceredeki gerçek okumaların alt sınırlarının en büyüğüdür (`limits/fusion.rs`): statusline `n` → `[n, n+1)`, API `m` → `[m−0,5, m+0,5)`, ondalıklı değer → tam. Kullanım pencere içinde azalmadığı için alt sınır pencere boyunca geçerli kalır. Değer API'nin yuvarlamasıyla gösterilir; pratikte `max(statusline, API)` olur. Bilinen en yeni okumadan daha yeni bir okumanın üst sınırı alt sınırın altında kalırsa sınır o okumadan yeniden başlar. Eşikler, ondalıklı kaynakların hız geçmişi, Ayrıntı grafiği ve `history --json` birleşik değeri kullanır. Tam sayı kaynakların hızı birleşik değerden değil, her kaynağın kendi okumalarından hesaplanır (aşağıda, ADR-062). `LimitView.combined_with` değeri yükselten ikinci kaynağı söyler. Birleşik değer saklanmaz; açılışta pencerenin snapshot'larından yeniden kurulur.
- **Bayatlık:** Gözlem yaşı > 10 dk ise UI "son güncelleme X dk önce" gösterir. Yaş > 30 dk ise ve tahmin mümkünse tahmin katmanı devreye girer (kesikli çizgi ve "tahmini" etiketi ile).
- **Reset algılama:** `resets_at` değeri geçtiyse ve yeni gözlem yoksa, limit yerel olarak %0'a çekilir (kaynak: `Estimate`, neden: `reset`) ve reset bildirimi tetiklenir.
- **Opsiyonel pencereler (ADR-041):** Her `(account, kind, scope)` için pencere durumu `WindowPresence` ile üç değerlidir: `Present`, `Unknown`, `Absent` (`limits/presence.rs`).
  - **S2:** Boş olmayan bir yanıt tam listedir. `limits[]` şemasında beklenen limitlerden (`session`, `weekly_all`) ve izlenen limitlerden yanıtta olmayan her biri hemen **yok** sayılır. Düz alan şemasında yalnızca beklenen limitler değerlendirilir.
  - **S1:** Claude Code, reset geçince pencereyi payload'dan düşürür ve statusline'ı yeni API yanıtı olmadan da çağırır. Bu yüzden eksik pencere, yalnızca başka bir limitin okuması **değişmiş** olan payload'da sayılır. Değişen okuma taze bir API yanıtını kanıtlar. Art arda 3 böyle payload pencereyi yok yapar.
  - Herhangi bir kaynaktan gelen ilk gerçek gözlem kararı geri alır.
  - Yok sayılan limitin track'i silinir: tahmin, reset, projeksiyon ve bildirim üretilmez. Geçmiş `limit_snapshots` tablosunda kalır.
  - Karar `kv` tablosunda `limits.absent.<account_id>` anahtarıyla JSON dizisi olarak saklanır. Yeniden başlatma ve uygulama kapalıyken CLI aynı kararı görür. Kaçırma sayaçları saklanmaz.
  - `PanelSnapshot.absent` ve IPC yanıtlarındaki `absent` alanı yok sayılan limitleri taşır.
- **Burn rate (`limits/rate.rs`, ADR-062):** Sırayla ilk uygun olan kullanılır:
  1. **Ondalıklı okumalar:** son 30 dakikanın birleşik değerlerinden Theil–Sen eğimi (en az 3 nokta; 5+ nokta `High`, yoksa `Medium`).
  2. **Tam sayı okumalar, adım kenarları (`Steps`):** Kaynak başına birikimli maksimumun yükseldiği anlar kenardır. Kenar zamanı, eski değeri gösteren son okuma ile yeni değeri gösteren ilk okumanın orta noktasıdır. Belirsizliği 45 dakikadan büyük kenar tutulmaz. Pencere başına kaynak başına en fazla 4 kenar saklanır. Hız = (son − ilk kenarın puanı) / aradaki süre. **Durma tavanı:** son kenardan bu yana aynı değer okunuyorsa `hız ≤ 1 puan / (son okuma − son kenar)`. **Güven:** 1 aralık `Low`, 2 aralık `Medium`, 3 aralık `High`; ilk ve son kenarın ortalama belirsizliği aralığın %25'ini aşarsa bir düzey düşer. Statusline ile API'nin ikisi de kenar verdiğinde kenarı çok olan, eşitlikte son kenarı yeni olan kaynak seçilir. Bu yöntem `≥ Medium` iken kullanılır.
  3. **Kalibre maliyet yedeği** (Adım 13, ADR-061): `session` ve `weekly_all` için, `Low`.
  4. **`Low` güvenli kenar hızı.**
  5. **`k0` önseli:** `p_now / C_pencere × son 15 dk maliyet hızı`, `Low`.

  **Projeksiyon:** `eta_100 = (100 − p) / rate`. `eta_100 < resets_at − now` ise "reset'ten önce dolacak" denir. Uygulama bildirimi ve Claude Code mod'unun bandı/toast'u aynı kuralla uyarır (`notifier::projection_horizon`): 5 saatlik pencere ve diğerlerinde güven en az `Medium` ve ETA 60 dakika (`PROJECTION_HORIZON_MIN`) içinde; haftalık limitlerde güven `High` ve ETA 24 saat (`WEEKLY_PROJECTION_HORIZON_MIN`, ADR-063) içinde. Düşük güvenli öngörü gösterilir ama silik ve "kaba" işaretlidir, uyarı üretmez.

## 8. Pencere modeli

- **5 saatlik pencere:** Kimlik `(account, session, resets_at)` olarak tanımlanır. `starts_at = resets_at − 5h`. Farklı bir `resets_at` değeri taşıyan snapshot yeni bir pencere demektir. `peak_percent` gözlemlenen en yüksek değerdir. `hit_limit` değeri, `percent ≥ 100` gözlemlendiğinde veya API severity bunu gösterdiğinde set edilir.
- **Haftalık döngü:** `weekly_all` ve `weekly_scoped` için `starts_at = resets_at − 7d` (gözlemde reset zamanı hesap bazında sabit, örneğin Çarşamba 07:59:59 UTC).
- **Tahmini pencereler (Adım 14a, ADR-064):** Yalnızca `weekly_all` için türetilir. Bilinen haftalık pencerelerin reset zamanı 7'şer gün geriye, kullanımın başladığı haftaya kadar taşınır. Bilinen pencereler arasındaki boşluklar da doldurulur. Bu pencereler `estimated = 1` olarak işaretlenir. `peak_percent` değerleri kalibrasyondan gelen `k·Σα·C` tahminidir ve görünümde `≈` ile gösterilir. `hit_limit` bilinmez. Kapsamlı limit ve 5 saatlik pencere türetilmez: 5 saatlik pencerenin hizalama kuralı veriyle doğrulanamadı (hesapta 5 saatlik pencere yok, ADR-041).

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

**Tablo değişince (ADR-057):**
- Maliyet indeksleme anında saklanır. Depo, olayları fiyatlayan tablonun parmak izini (`version` + tablo metninin özeti) `kv.pricing.table` anahtarında tutar.
- Çekirdek başlarken parmak izi farklıysa bütün ham olaylar yeniden fiyatlanır ve fark `usage_5m`'e uygulanır (`store/reprice.rs`). Saklama süresini aşmış olayların `usage_5m` payı eski maliyetle kalır.
- Tabloda satırı olmayan model aile yedeğiyle fiyatlanır (ADR-025). Son 7 günde kullanılan böyle modeller `IndexState.unpriced_models` ile panele ve `doctor` çıktısına gider; maliyetleri tahmin sayılır.

**Hipotez H1:** Limit tüketimi C ile orantılıdır (`Δ% ≈ k·ΔC`) ve tüm modeller için aynı k geçerlidir. H1 backtest ile sınanır. Modele bağlı sistematik bir sapma görülürse aile bazlı çarpanlar (`α_f`, normalize edilmiş) öğrenilir: `Δ% ≈ k·Σ_f α_f·ΔC_f`.

### 9.2 Kalibrasyon (v0.2, ADR-061)

- **Gözlemler: kenar aralıkları.**
  - Statusline ve API tam sayı verir (ADR-060). Pencerenin birikimli maksimumunun her adımı bir "kenar"dır; zamanı adımdan önceki son ve sonraki ilk okumanın orta noktasıdır.
  - Gözlem, ardışık iki kenar arasındaki `Δp` (çoğunlukla 1) ile aynı aralıktaki (`from < ts ≤ to`) aile başına yerel maliyet `ΔC_f`'dir.
  - Belirsizliği 45 dakikayı aşan kenarlar birleştirilerek atlanır. Pencerenin ilk okuması kenar değildir. Pencereler `WINDOW_MATCH_TOLERANCE_MS` ile eşlenir.
  - Kaynaklar karışmaz: en az 10 statusline aralığı olan limit statusline serisinden, diğerleri API serisinden öğrenilir.
  - Atılanlar: maliyeti medyanın %5'inin altında kalan aralıklar ve fiyatsız modellerin payı %10'u aşan aralıklar.
- **Model:** `Δp ≈ k · Σ α_f · ΔC_f`, `α_opus = 1`. Tek `k` (H1) reddedildi: Fable'ın ağırlığı ≈ 2 kat. Kapsamlı limit yalnızca kendi ailesini sayar ve `α` öğrenmez.
- **Tahminci:**
  - `k`: `Δp / Σ α ΔC` oranlarının ağırlıklı medyanı. Ağırlık ham maliyet × üstel sönüm (yarı ömür 3 gün), `α`'dan bağımsız.
  - `α_f`: log ızgarada koordinat araması, ağırlıklı göreli MAD'i en aza indirir. Aile ancak yeterince aralığı taşıyorsa ve yeterince aralıkta yoksa kendi ağırlığını alır; 1'e %15'ten yakın sonuç 1'dir.
  - Ayrı katsayılar: her `(kind, scope)` için bir kalibrasyon (`session` yalnızca 5 saatlik penceresi olan hesaplarda).
- **Güven skoru:** sayı (`min(1, n/30)`) × kesinlik (`1 − se/0,15`) × tazelik (bir günden sonra 3 günde yarıya) × öğrenilmiş aile payı, [0,1]. Eşikler: < 0,4 düşük, 0,4–0,7 orta, > 0,7 yüksek. Uygulanırken güven, son maliyette ağırlığı bilinmeyen ailelerin payı kadar ayrıca düşer (`confidence_for`).
- **Sapma (drift):** `ln(r/k)` artıkları üzerinde iki yönlü CUSUM (eşik 6). Alarmda `k` değişimin başından 1 günlük yarı ömürle yeniden öğrenilir; sapma 3 gün görünür. Ayrıntı'da tek satır bilgi çıkar.
- **Kalıcılık ve döngü:** limit thread'i 15 dakikada bir, yalnızca yeni snapshot geldiyse 21 günlük veriden yeniden hesaplar (≈ 20 ms). `calibrations` tablosuna `k`/`α` ≥ %2 oynadığında, düzey ya da sapma değiştiğinde veya günde bir yeni sürüm yazılır.
- **Başlangıç (prior):** Kalibrasyon yokken ya da kullanım anı güveni düşükken, mevcut pencerenin anlık oranı kullanılır: `k0 = p_now / C_window`. Kalibrasyon en az orta güvendeyse yedek öngörü `k · Σ α · c_f` (son 15 dk) kullanır (`RateMethod::Calibrated`); öngörünün güveni yine `Low`'dur.

### 9.3 Atıf

- **v0.1 (basit, kalibrasyonsuz):** Bir oturumun mevcut penceredeki payı = `p_now × C_oturum / C_pencere_toplam`. Bu, tüm tüketimin yerel olduğu varsayımına dayanan "orantılı dağıtım"dır ve UI'da "≈" işaretiyle gösterilir.
- **v0.2 (24 saatlik dilimler, Adım 14a, ADR-064):** Kalibrasyonun penceredeki güveni en az orta ise kullanılır, değilse v0.1 geçerlidir ve "Diğer" hesaplanmaz.
  - Kenarlar, pencerede keskin kenarı çok olan kaynaktan gelir (≤ 45 dk belirsizlik). Pencere başlangıcına hizalı 24 sa dilimlere gruplanırlar. Her parça bir dilimin son kenarında biter. Son kenardan sonraki bölüm son parçaya katılır.
  - Her parçada `yerel = min(Δp, k·Σα·ΔC_yerel)` hesaplanır. Oturum s'nin payı `yerel × α·ΔC_s / Σα·ΔC_yerel` olur, kalan `Δp − yerel` **"Diğer"** kovasına gider. Aralık başına hesap, yuvarlama gürültüsünün pozitif yarısını "Diğer"e topladığı için kullanılmaz.
  - Baş parçasından 0,5 puan düşülür (yuvarlama ofseti) ve oturum paylarına orantılı eklenir.
  - Σ paylar + Diğer = `peak_percent`. Maliyet `usage_5m`'den gelir. Subagent maliyeti `COALESCE(root_id, id)` üzerinden ana oturuma eklenir.
- **Çapraz doğrulama (tek yönlü):** Yalnızca `weekly_all` ve kalibre yöntemde yapılır. `Diğer/toplam`, pencerenin son `seven_day_breakdown` değerindeki `claude_code` dışı satırların toplamıyla karşılaştırılır. Fark 10 puandan fazla aşağıdaysa `suspect` set edilir ve `CoreEvent::CalibrationSuspect` yayınlanır. 10 puandan fazla yukarıdaysa yalnızca `other_excess` set edilir: başka bir bilgisayardaki Claude Code da `claude_code` sayılır.
- **Zamanlama:** Limit thread'i kalibrasyon turunda tahmini pencereleri ekler ve eksik ya da kapanmış atıfları yazar. Yeni veri geldiğinde açık pencereyi en fazla 60 sn'de bir günceller; boşta uyanmaz. Atıflar `attribution_runs.calibration_version` ile saklanır. Kalibrasyon sürümü değişince kalibrasyon turu kapanmış pencereleri de arka planda yeniden hesaplar (en yeniden başlar, turda en fazla 64; ADR-065). Arada listeler saklanan değeri `stale` olarak gösterir, pencere detayı `attribution::ensure` ile hemen hesaplar.
- **Sorgu indeks kuralı (ADR-065):** `usage_5m` üzerinde bir oturum ağacını adlandıran ya da 60 günden geniş aralığı okuyan sorgular `+u.bucket_ts` yazar. Böylece SQLite `ix_5m_acct_bucket` yerine birincil anahtarı kullanır; zaman indeksi her satır için birincil anahtara baktığı için geniş aralıkta tablo taramasından pahalıdır. Toplama önce oturum başına (`MATERIALIZED` CTE), sonra `COALESCE(root_id, id)` başına yapılır.

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
- **Alan eklemeleri:** Yeni alanlar `#[serde(default)]` ile eklenir; `PROTOCOL_VERSION` ve `--json` şema numarası değişmez. Eski CLI bilinmeyen alanı yok sayar, yeni CLI eski uygulamadan boş değer okur. Örnek: 0.1.1'de `StatusReply` ve `LimitsReply`'a eklenen `absent` (ADR-041). Adım 12o: `StatusQuery.terminal` (`windows_terminal` / `vscode` / `other`); uygulama görülen türleri ve sonuncusunu bellekte tutar, saklamaz (ADR-055). Adım 12r: `StatusQuery.cwd` ve `StatusQuery.git_untracked` (varsayılan `true`, `true` iken yazılmaz), `StatusReply.git` (`GitStatus`, yalnızca sayılar; ADR-059).
- **`tokenradar statusline` akışı (p95 < 30 ms):**
  1. stdin okunur (en fazla 1 MB) ve minimal alanlar parse edilir.
  2. Daemon'a 50 ms zaman aşımıyla bağlanılır ve `statusline.push` gönderilir.
  3. Satır stdin verisiyle biçimlendirilir; varsa daemon'dan gelen ekstra bilgiyle (oturumun pencere payı, projeksiyon) zenginleştirilir.
  4. Daemon yoksa veri `spool/statusline-YYYYMMDD.jsonl` dosyasına eklenir (günlük en fazla 5 MB). Uygulama açıldığında spool dosyaları içe aktarılır. Böylece **uygulama kapalıyken bile % geçmişi kaybolmaz**.
  5. Zincirleme modu (`--chain`): Kullanıcının önceki statusline komutu aynı stdin ile çalıştırılır ve çıktısı bizim segmentimizle birleştirilir. Zaman aşımı 300 ms'dir.
  6. Yüzdeler ve reset süreleri stdin'deki payload'dan, projeksiyon ve oturum payı uygulamadan gelir (ADR-029).
  7. **Satır (v2, ADR-042):** Model ve render `tokenradar_core::statusline::line` içindedir; ayarlar penceresinin önizlemesi de aynı kodu kullanır. Segmentler `five_hour`, `seven_day`, `scoped`, `context`, `session_share`, `cost` ve `model`'dir. Hangilerinin, hangi sırayla ve tek mi iki satırda görüneceği `config.toml`'daki `[statusline]` tablosundan gelir. Her değer kaynağını ve gözlem zamanını taşır. Uygulamadan gelen ve 10 dakikadan eski değer soluk gösterilir. Uygulamanın tahminleri statusline'da gösterilmez.
  8. **Zaman işareti (ADR-044):** Bar, pencerenin geçen zamanının düştüğü hücre sınırında bir `│` taşır ve yüzdeden sonra `⏱ 43%` gelir (boşluk ADR-056'dan; yedek fonttan gelen glif ona taşar). Geçen zaman, reset anı ve pencere uzunluğundan (5 saat, 7 gün) hesaplanan bir ölçümdür. `[statusline] pace` ile kapatılır.
  9. **Öğe modeli (v3, ADR-049):** `[statusline]` artık bir öğe listesi (`[[statusline.items]]`). Her öğenin türü, biçimi, etiketi ve rengi vardır. 22 tür dört gruptadır: limitler, oturum (context, maliyet, model, token, süre, satır değişimi, vim, sürüm, ajan, oturum kimliği), proje (cwd, proje dizini, git dalı) ve düzen (ayraç, satır sonu, özel metin). Temalar: `plain`, `powerline`, `powerline_round`. `color` (`auto` / `always` / `never`) renkleri belirler. Öğe listesi yoksa satır, 12c anahtarlarından türetilir; bu anahtarlar eski CLI'lar için yazılmaya devam eder. Dar terminalde her satır ayrı ölçülür. Önce oturum ve proje öğeleri sondan düşer, sonra limit ayrıntıları; ilk limit her zaman kalır.
  10. **Düzen (ADR-051):** `version = 2` ile ayraçlar da öğedir; listede ne varsa o çizilir. `version`'sız bir tablo okunurken yan yana iki değerin arasına global `separator` glifiyle bir ayraç öğesi eklenir (glif boşsa `space`), böylece satır aynı görünür. Göç bir kez yapılır. Öğeler arasına tek bir boşluk konur. Bölge başındaki ve sonundaki ayraçlar ile değersiz öğenin bıraktığı ikinci ayraç düşer. `space` 1–8 sütunluk sabit boşluktur. `flex` satırı bölgelere ayırır: bir `flex` sonrasını sağa yaslar, iki `flex` ortadaki bölgeyi ortalar, üç ve daha fazlası boş genişliği eşit dağıtır. Kullanılabilir genişlik `COLUMNS − 4 − margin`'dir: Claude Code satırın son 4 sütununu `…` ile keser. `margin` 0–8, varsayılan 0. `COLUMNS` yoksa her `flex` tek boşluktur. Genişlik `unicode-width` ile ölçülür. Satır boşlukla başlıyorsa önüne `ESC[0m` yazılır, çünkü Claude Code baştaki boşlukları kırpar. Powerline'da `flex` ve `space` blok zincirini kapatır; son bölge sola bakan gliflerle çizilir.
  11. **Simgeler ve bar stilleri (ADR-056):** `icons` (`text` / `unicode` / `nerd`) öğe etiketinin yerine bir simge koyar. Öncelik: kullanıcının etiketi > setin simgesi > Fluent kelimeleri; yalnızca boşluktan oluşan etiket hiçbir şey göstermez. Nerd seti Octicons (BMP PUA) kullanır ve zaman ile compact işaretlerini de değiştirir; `≈`, soluk çizim, `API` ve "bu oturum" hiçbir sette değişmez. Bar stilleri `blocks`, `thin`, `dots`, `capsule` (Nerd Fonts ≥ 3.4, U+EE00–EE05) ve `solid` (arka plan hücreleri). Kapsül ve dolgu zaman işaretini ek hücre olarak değil, zamanın düştüğü hücrede çizer. Çubuklar yalnızca renkle ve düz temada çizilir; kapsül, şekille okunduğu için Powerline bloklarında da çizilir. `unicode-width` (cjk kapalı) PUA'yı 1 sayar.
  12. **Payload'ın yeni alanları (ADR-059, Adım 12q):** Efor, düşünme, hızlı mod, prompt cache, PR, worktree, depo ve harcama limiti yalnızca satır için okunur (`skip_serializing`). Yeni alanlar esnek okunur: beklenmeyen tipte bir alan yalnızca kendisini boş bırakır. Harcama limiti bir limit gibi çizilir (eşik renkleri, sıfırlanma) ama zaman işareti ve projeksiyonu yoktur, limit motoruna ve tray'e girmez. Cache durumu `caching_observed`, `warm` ve `expires_at`'ten birlikte türetilir; saat biçimi yenileme gerektirmez.
  13. **Canlı yenileme ve git durumu (ADR-059, Adım 12r):** İsteğe bağlı `statusLine.refreshInterval` (5/10/30 sn) Claude Code'un satırı periyodik çizmesini sağlar; `cache_timer`'ın `countdown` biçimi ancak onunla akar. Aynı içerikteki push tekrarı (`content_key`, serileştirilmiş payload'ın hash'i) daemon'da en fazla dakikada bir snapshot kurar; spool son satırı (16 KiB kuyruk, kilitsiz) heartbeat içinde aynıysa yazmaz. `git_status` öğesi için CLI çalışma klasörünü gönderir; uygulamanın `GitStatusCache`'i repo köküne göre (en fazla 8, LRU) `git --no-optional-locks status --porcelain=v2 --branch -z` sonucunu arka plan thread'inde tutar: 5 sn TTL, `HEAD`/`index` mtime değişince geçersiz, en fazla 1 dk bayat sonuç yeniden hesap sürerken sunulur, 2 sn zaman aşımı, iki zaman aşımından sonra `-uno`. Pipe yanıtı git'i asla beklemez; ilk sorguda öğe gizlidir.
- **Claude Code mod'u (ADR-043):** `integrations/claude-code/` altında durur ve tamamlayıcıdır. Claude Code'un kendi limit değerlerini `session.measure` ile alır; projeksiyon, scoped limit ve oturum payı için en fazla 30 saniyede bir `tokenradar status --json` çalıştırır. Tek satırlık kenarlıklı bant (mini bar, "Ayrıntı", "Gizle"), toast, reset satırı ve `/radar` sunar. `/radar` bir panel açar ve kısa bir özet döndürür. Panelde limit kartları (kaynak rozetleriyle), haftalık zaman şeridi, kullanım grafiği (masaüstü, VS Code ve mobilde `Svg`, terminalde `Raster`), canlı oturumlar ve bağlam barı bulunur. Grafiğin verisi `tokenradar history --json`'dan gelir ve yalnızca panel açıkken, en fazla 5 dakikada bir alınır (ADR-046). Statusline köprüsü kurulu değilse mod, `$.ui.status` ile kısa bir durum girdisi gösterir. Uygulama açıkken toast göstermez, bildirimi uygulama gönderir. Pencerenin ilk yedide biri geçtikten sonra kullanım zaman çizgisinin 10 puan önündeyse bant tempo uyarısı gösterir (toast yok, ADR-045); `/radar` tempo satırını ve haftalık zaman çizgisini yazar (ADR-044). Mod kurulum paketinde `<kurulum>\claude-code` altında gelir ve `settings.json`'daki `env.CLAUDE_CODE_PLUGIN_DIRS` listesine eklenerek kurulur; CLI seçeneği varsayılandaysa yanındaki `..\bin\tokenradar.exe`'yi kullanır (ADR-047).
- **Diğer komutlar:**
  - `tokenradar status [--json]`, `limits`, `sessions`: Uygulama çalışmıyorsa SQLite'tan salt-okunur okunur.
  - `status --json` çıktısında, IPC yanıtına ek olarak CLI'ın çözdüğü `locale` alanı (`en`, `tr`) bulunur. Bu alan Claude Code mod'unun dilini belirler (ADR-043).
  - `history [--json]`: Her limitin geçerli penceresindeki saklı okumalarını JSON olarak verir; pencere başına en fazla 168 nokta. Uygulama çalışırken de veritabanından salt-okunur okur ve IPC'ye dokunmaz (ADR-046).
  - `doctor`: Anonim bir tanılama paketi üretir.
  - `statusline install|uninstall`: `settings.json` dosyasını yedekler, değişikliği diff olarak gösterir, atomik olarak yazar ve önceki komutu korur. Yalnızca `statusLine` üyesinin baytları değişir; `install → uninstall` dosyayı bayt düzeyinde geri getirir (ADR-029).
  - `mod install|uninstall`: Claude Code mod'unun klasörünü `env.CLAUDE_CODE_PLUGIN_DIRS` listesine ekler ya da listeden çıkarır. Aynı diff, onay, yedek ve atomik yazma kuralları geçerlidir; yalnızca `env` üyesinin baytları değişir. `uninstall --all` kaldırıcı içindir (ADR-047).

## 12. i18n mimarisi

- **Tek kaynak:** `locales/{en,tr}/*.ftl`. Rust tarafı (`tokenradar-i18n`: bildirimler, tray tooltip'i, CLI) ve TS tarafı (UI) aynı dosyaları kullanır. CI testi iki dilde **anahtar paritesini** ve değişken tutarlılığını doğrular.
- **Özel Fluent fonksiyonları (Rust ve TS'de birebir aynı davranış):**
  - `NUM(n, decimals)`: yerel ayara göre gruplama ve ondalık ayırıcı. TR: `1.234,5`, EN: `1,234.5`.
  - `PCT(n)`: TR `%72`, EN `72%`.
  - `DUR(ms, style)`: TR `1 sa 48 dk`, EN `1h 48m`. `"long"` stili ekran okuyucular içindir: `1 saat 48 dakika`, `1 hour 48 minutes` (ADR-031).
  - `TR_SUFFIX(n, case)`: Türkçe sayı ekleri. Örnekler: `5'te`, `6'da`, `%72'si`, `10'dan`, `3'e`. Ek, sayının okunuşunun son hecesine göre seçilir (bir, iki, üç … on, yirmi … yüz, bin, milyon). Girdi sayı veya biçimlenmiş metin olabilir (`%23,5`, `10:59`, `Çar 10:00`); saatte `:00` saate, diğer dakikalar dakikaya göre çekimlenir. Haller: `loc`, `abl`, `dat`, `acc`, `gen`, `poss` (ADR-030).
  - Parite garantisi: `testdata/i18n-vectors.json` dosyasındaki ortak test vektörleri hem Rust hem TS testlerinde çalıştırılır.
- **Tarih ve saat:**
  - Rust ve TS tarafında EN/TR için küçük, el yazımı biçimlendiriciler bulunur (TS: `app/src/lib/i18n/format.ts`, Rust'ın portu). `Intl` kullanılmaz: yuvarlama ve `-0` davranışı Rust'tan farklı ve ICU sürümüne bağlı (ADR-031). İki taraf da ortak test vektörleriyle doğrulanır. ICU verisi gömülmez, çünkü binary boyutunu artırır.
  - TR her zaman 24 saat kullanır. EN, işletim sistemi bölge ayarını izler.
- **İstisna, Claude Code mod'u:** Mod `locales/` dosyalarına erişemez. Bu yüzden `integrations/claude-code/hooks/i18n.ts` içinde kendi küçük EN/TR tablosunu ve `NUM`/`PCT`/`DUR` portunu taşır (ADR-043).
- **Dil seçimi:** İşletim sistemi dili (`sys-locale`) `tr` ile başlıyorsa TR, aksi halde EN seçilir. Kullanıcı ayarlardan değiştirebilir. Değişiklik yeniden başlatmadan uygulanır.
- **Büyük-küçük harf:** `<html lang>` her zaman ayarlanır. JS'de `toUpperCase()` yerine `toLocaleUpperCase(lang)` kullanılır. Böylece `i → İ` ve `ı → I` dönüşümleri doğru çalışır.

## 13. Güvenlik ve mahremiyet

- **Telemetri yok.** Ağ trafiği yalnızca şu iki hedefe gider:
  - `api.anthropic.com` (S2 onayı varsa)
  - GitHub Releases (güncelleme kontrolü; ayarlardan kapatılabilir)
  - Nerd Fonts'un GitHub release varlığı ve `release-assets.githubusercontent.com` (yalnızca kullanıcı font indirmeyi onaylarsa; sabit sürüm ve SHA-256, ADR-055)
- **Token:** Bkz. data-sources.md S2/S5. Token yalnızca istek anında okunur, `zeroize` edilir, loglanmaz ve IPC'den geçmez. `refreshToken` asla okunmaz.
- **İçerik:** Konuşma içeriği (`message.content`) **asla** okunmaz, saklanmaz ve gösterilmez. Yalnızca meta veri ve kullanım bilgisi işlenir. Oturum başlığı için yalnızca Claude Code'un ürettiği başlık ve ad alanları kullanılır.
- **Tauri:**
  - Pencere başına minimal izin (capabilities) tanımlanır.
  - CSP `default-src 'self'` olarak ayarlanır. Uzak içerik yüklenmez, `eval` kullanılmaz.
  - `opener` yalnızca beyaz listedeki URL'leri açar.
- **Dosya değişikliği:** Uygulamanın dışarıda değiştirdiği dosyalar Claude Code'un `settings.json`'ı (statusline ve mod kurulumu) ile Windows Terminal ve VS Code'un `settings.json`'larıdır (terminal fontu, ADR-055). Bu işlem her zaman kullanıcı tıklamasıyla yapılır; öncesinde diff gösterilir, yedek alınır ve dosya atomik olarak yazılır. Nerd Font kullanıcının font klasörüne ve HKCU font anahtarına kurulur. Tek tıkla geri dönüş tek bir özet onayıyla ayarları ve fontu birlikte geri alır.
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
