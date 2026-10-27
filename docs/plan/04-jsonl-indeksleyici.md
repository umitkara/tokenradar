# Adım 04 — Claude JSONL indeksleyici

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1 | 02, 03 | 1-2 oturum | ✅ |

## Amaç

Claude Code transcript'lerini (`<config>/projects/**/*.jsonl`) **artımlı**, **tekilleştirilmiş** ve **hızlı** biçimde SQLite'a aktaran indeksleyiciyi yazmak. İlk indeksleme en yeni dosyalardan başlar; mevcut pencere ve bu haftanın verisi saniyeler içinde hazır olur.

## Önce oku

- `docs/data-sources.md` S3 (**tüm tuzaklar**)
- `docs/architecture.md` §5, §6, §16
- `docs/perf/<tarih>-spike.md` (Adım 02 sonuçları ve seçilen parse stratejisi)
- `docs/decisions.md`: ADR-007, Adım 02'de eklenen parse ADR'si

## Kapsam

- `providers/claude/`:
  - `discover`: varsayılan `~/.claude`, `CLAUDE_CONFIG_DIR` ve ayarlardaki ek dizinler.
  - `classify_path`: ana oturum dosyası, subagent dosyası veya yok sayılacak dosya.
  - `parse_line`: seçici deserialize.
  - Oturum meta verisi: cwd, gitBranch, version, entrypoint, başlık.
- `indexer/`:
  - Dosya envanteri (stat) ve `source_files` ile karşılaştırma.
  - Önceliklendirme: en yeni önce.
  - Offset'ten okuma, satır bölme, yarım satırların korunması.
  - Worker havuzu → tek yazıcı.
  - Truncate ve silinme durumlarının işlenmesi.
  - İlerleme olayları.
  - Windows'ta arka plan önceliği.
- Fixture anonimleştirici: `crates/tokenradar-core/examples/sanitize_jsonl.rs`.
- Saklama işinin günlük çalıştırılması (Adım 03'teki fonksiyonun zamanlanması).

## Kapsam dışı

- Dosya izleme ve olay güdümlü tetikleme (Adım 05). Bu adımda indeksleyici "tam tarama" ve "verilen dosya listesini artımlı işle" API'leriyle çağrılır.
- Limit snapshot'ları (Adım 06).

## Görevler

### Keşif (kodlamadan önce, gerçek veride)

- [x] **Kayıt tipleri envanteri:** Gerçek veride hangi `type` değerleri var? (`assistant`, `user`, `attachment`, `api-request`, `api-request-blob`, `mode`, `summary`, `custom-title`…) Gözlem sırasında `api-request` ve `api-request-blob` tipleri görüldü:
  - Bu kayıtlar `usage` taşıyor mu?
  - Taşıyorsa assistant kayıtlarıyla çift sayıma yol açar mı?
  - Karar ver ve `data-sources.md`'ye işle.
- [x] **Oturum başlığı kaynakları:** `summary`, `custom-title` veya benzeri kayıtlar ve `sessions/*.json` içindeki `name`. Öncelik sırasını belirle. Başlık için **içerik okunmaz**; yalnızca bu meta kayıtlar kullanılır.
- [x] **Subagent kimliği:** Subagent dosyalarındaki `sessionId` ana oturumu mu gösteriyor? Dosya yolundaki üst dizin adıyla karşılaştır.
- [x] **`iterations` alanı:** Birden fazla eleman içeren `iterations` var mı? Varsa üst düzey `usage` ile ilişkisi ne?
- [x] **`speed` değerleri ve `fast` modu:** Hangi değerler görülüyor?
- [x] Bulguları `docs/data-sources.md` S3 bölümüne ekle.

### Uygulama

- [x] `providers/claude/record.rs`: Seçilen strateji (Adım 02) ile ödünç alan struct'lar. `message.content` için `IgnoredAny`. Gerekli alanlar: `type`, `uuid`, `sessionId`, `isSidechain`, `timestamp`, `cwd`, `gitBranch`, `version`, `entrypoint`, `requestId`, `message.id`, `message.model`, `message.usage.*`.
- [x] `ParseOutcome`: `UsageEvent` | `SessionMeta` (cwd, branch, başlık…) | `Skip` | `Unknown(reason)`. Unknown sayaçları tutulur (şema sapması sinyali, `architecture.md` §15).
- [x] Proje çözümleme: `cwd` → `path_norm` dönüşümü:
  - Sürücü harfi büyük harfe çevrilir.
  - `\` ve `/` tek biçime getirilir.
  - Sondaki ayraç silinir.
  - Windows'ta karşılaştırma büyük-küçük harfe duyarsızdır.
  - `display_name` olarak klasör adı kullanılır.
  - Kayıtlarda `cwd` hiç yoksa dizin adından kayıplı çözümleme yapılır ve proje "çözümlenmemiş" olarak işaretlenir.
- [x] Subagent bağlama: `<proje>/<sessionUuid>/subagents/agent-<id>.jsonl` → `kind=subagent`, `uuid="agent-<id>"`, `parent_id` = ana oturum. Ana oturum henüz indekslenmediyse yer tutucu bir oturum oluşturulur ve ana dosya geldiğinde doldurulur.
- [x] `indexer/reader.rs`:
  - `offset`'ten itibaren 1 MB'lık parçalarla okur, `memchr` ile `\n` arar.
  - Son yarım satırı bir sonraki tura bırakır; offset yalnızca tam satır sonuna ilerletilir.
  - `size < offset` ise truncate kabul edilir ve dosya baştan işlenir (tekilleştirme sayesinde idempotent).
  - Silinen dosya → `source_files.deleted_at` set edilir. Veriler korunur.
- [x] `indexer/mod.rs`:
  - `full_scan()`: envanter çıkarılır, değişen dosyalar `mtime` değerine göre azalan sırada (en yeni önce) kuyruğa alınır.
  - `index_files(paths)`: aynı işlem hattını verilen dosya listesi için çalıştırır.
  - Worker sayısı `min(4, çekirdek/2)` (Adım 02 kararına göre). Worker'lar parse eder, yazıcı thread 5.000 olayda bir veya 250 ms'de bir batch yazar.
  - `source_files.offset` değeri, ilgili batch yazıldıktan **sonra**, aynı transaction içinde güncellenir. Çökme sonrası tutarlılık bu sırayla sağlanır.
- [x] Arka plan önceliği: Windows'ta worker thread'leri `SetThreadPriority(THREAD_MODE_BACKGROUND_BEGIN)` ile başlar (`windows-sys`, `unsafe` blok yerel `allow` ve gerekçe yorumuyla). Diğer platformlar için no-op stub bırakılır.
- [x] İlerleme: `IndexProgress { files_done, files_total, bytes_done, bytes_total, phase }`. Şimdilik bir callback/kanal; Adım 05'te EventBus'a bağlanacak.
- [x] "Mevcut pencere hazır" sinyali: Son 7 güne ait dosyalar bittiğinde `IndexMilestone::RecentReady` yayınlanır.
- [x] `examples/sanitize_jsonl.rs`:
  - `message.content`, `thinking`, `signature` ve araç girdi/çıktıları silinir.
  - `cwd` ve yollar `proj-<hash8>` biçimine çevrilir.
  - Kimlikler korunur, çünkü tekilleştirme testleri bunlara dayanıyor.
  - Çıktı `testdata/claude/` altına yazılır.
  - **Gerçek kullanıcı verisi commit'lenmez.** Fixture'lar sentetik olabilir veya anonimleştirilmiş kısa kesitlerden oluşabilir.
- [x] Fixture'lar:
  - Tekrar eden ve kısmi kayıtlar (`iterations: []`)
  - Yarım son satır
  - Truncate
  - Subagent ağacı
  - Bilinmeyen kayıt tipi
  - Hatalı JSON satırı
  - `cache_creation` kırılımı olmayan kayıt
- [x] Testler: fixture'lar üzerinde snapshot testleri (insta) ve uçtan uca "dosya ekle → indeksle → toplamları doğrula" testleri. Artımlı tur: dosyaya satır ekle → yalnızca yeni satırlar işlenir.
- [x] **Gerçek veri doğrulaması** (yerel, commit'lenmez):
  - Ölçülecekler: ilk indeksleme süresi, tepe RSS, DB boyutu.
  - Birkaç günlük token toplamlarını `npx ccusage@latest daily --json` ile karşılaştır. Fark ±%2'yi aşarsa nedenini açıkla. ccusage tekilleştirmeyi farklı yapıyor olabilir; bu durumda hangisinin doğru olduğunu ham veriden göster.
- [x] Ölçümü `docs/perf/<tarih>-indexer.md` olarak kaydet.

## Teknik notlar ve tuzaklar

- **Tekilleştirme kuralı:** "İlk görüleni tut" yaklaşımı **yanlıştır**. Kısmi kayıtlar önce gelebilir. Kural, Adım 03'teki UPSERT'te uygulanıyor; burada yalnızca doğru `msg_key` ve tam `usage` alanları üretilmeli.
- Satırların bir kısmı onlarca KB'tır. Ön filtre, `type` alanını bulmadan önce `"usage"` arar; `"usage"` geçmeyen satırlar parse edilmez. Ama `SessionMeta` için gereken kayıtları (başlık, cwd) atlamamaya dikkat et: bu kayıtlar için ayrı ve ucuz bir filtre kullan.
- Windows'ta Claude Code'un yazdığı bir dosyayı okumak için `FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE` paylaşım modu gerekir. Rust'ın `File::open` varsayılanı uygundur, ama dosyayı **asla kilitleme**.
- `mtime` çözünürlüğü ve saat kayması nedeniyle "değişti mi?" kontrolü `(size, mtime)` çiftiyle yapılır. Boyut aynı ama `mtime` farklıysa dosya baştan işlenmez; offset'ten devam edilir.
- 1.500 proje klasörü var. Envanter `read_dir` özyinelemesi ile çıkarılır. `walkdir` kullanılabilir ama `tool-results` gibi dizinler budanmalı (prune).

## Kabul kriterleri

- [x] Referans veri setinde ilk indeksleme süresi bütçe içinde (Adım 02'de revize edilen değer). `RecentReady` ≤ 5 s. _(Sıcak önbellekte ölçüldü; soğuk değer tahmin, yeniden başlatma sonrası ölçüm açık.)_
- [x] Yeniden başlatma sonrası değişmemiş veri seti için tarama ≤ 500 ms.
- [x] ccusage karşılaştırması ±%2 içinde veya fark gerekçelendirilmiş.
- [x] Bütün fixture testleri geçer. Hatalı satır indekslemeyi durdurmaz.
- [x] Keşif bulguları `data-sources.md`'ye işlenmiş.

## Doğrulama

```powershell
cargo test -p tokenradar-core indexer
cargo run --release -p tokenradar-core --example index_real -- "$env:USERPROFILE\.claude" --db "$env:TEMP\tr-test.db"
npx ccusage@latest daily --json --since 20261020   # karşılaştırma için
```

## Devir notu

**Tarih:** 2026-10-27 · **Dal:** `step/04-jsonl-indexer` · **Rapor:** `docs/perf/2026-10-27-indexer.md` · **Karar:** ADR-026

### Yapılanlar
- **`providers/claude/`:**
  - `discover` / `discover_from`: `~/.claude`, `CLAUDE_CONFIG_DIR` ve ek dizinler. Kanonik yolla (Windows'ta küçük harfli) tekilleştirir. Varsayılan dizinin etiketi `Claude`.
  - `classify_path` → `SourceKind::{Main, Subagent}`. `tool-results` ve diğer her şey `None`.
  - `record.rs`, `LineParser`: üç `memmem` bulucu, ödünç alan struct'lar. `ParseOutcome::{Usage, Meta(Title), Skip, Unknown}`. Advisor elemanları ayrı `AdvisorRecord`. İçerik ve `last-prompt` asla parse edilmez (test ile korunuyor).
  - `project.rs`, `ProjectKey::from_cwd`: Windows yollarında büyük-küçük harf katlama, POSIX yollarında koruma. `from_dir_name` → `unresolved:<dizin>`.
- **`indexer/`:**
  - `Indexer::full_scan` ve `index_files(paths)`. `plan_jobs`: değişmemiş dosyayı atlar, truncate ve silinme sonrası geri gelen dosyayı baştan okur, işleri mtime'a göre azalan sıralar.
  - `reader.rs`: 1 MiB parçalar, yarım satır korunur, 4 MiB üstü satırlar belleğe alınmadan atlanır.
  - Worker havuzu (`(çekirdek/2).clamp(1,4)`) → sınırlı kanal → tek yazıcı. Worker'lar parça içinde tekilleştirir. Yazıcı oturumları, olayları ve offset'leri tek transaction'da yazar (`Writer::commit_index_batch`).
  - Hibrit öncelik (`priority.rs`), `IndexProgress` ve `IndexMilestone::RecentReady`, `ParseStats` sayaçları, sapma uyarısı (`warn_on_drift`).
  - `retention.rs`: `RetentionScheduler` saatlik kontrol eder, `prune_if_due` en fazla 24 saatte bir çalışır (kv `retention.last_run`).
- **Store:** `sessions.title_source`, başlık öncelik `CASE`'i, sıra bağımsız eşitlik kuralı (`tie_prefers`), `store/index.rs` (`SourceFileState`, `IndexBatch`, `mark_deleted`). `MsgKey::from_ids_tagged`.
- **Araçlar:** `examples/index_real.rs` (ölçüm + kararlı parmak izi + `--daily-since`), `examples/sanitize_jsonl.rs` (allowlist anonimleştirici).
- **Fixture'lar:** `testdata/claude/projects/` altında sentetik Alpha/Beta/Gamma (`testdata/README.md`). Uçtan uca 10 test ve bir insta snapshot'ı.
- **Sonuçlar:**
  - Sıcak önbellekte ilk indeksleme 3,7-6,5 s, `RecentReady` 1,1-2,4 s.
  - Yeniden tarama 107-123 ms, tepe private 50-57 MiB.
  - ccusage ile 7 tamamlanmış günde birebir eşitlik.

### Sapmalar (ADR-026)
- Advisor iterasyonları ayrı olay (kullanıcı kararı).
- Eşitlik kuralı "en son görülen" yerine sıra bağımsız.
- Tamamen arka plan önceliği yerine hibrit öncelik (kullanıcı kararı). Tamamen arka planda `RecentReady` 7-9 s ölçüldü ve bütçeyi aştı.
- Başlık kaynağı için yeni `title_source` sütunu. Migration 0001 yerinde değiştirildi, çünkü henüz yayımlanmış veritabanı yok.
- `Provider` trait'i yazılmadı; Claude serbest fonksiyonlarla uygulandı (architecture.md §16 notu, Adım 20).
- `sessions/*.json` içindeki `name` alanı başlık kaynağı olarak okunmadı (S4, Adım 05).

### Açık konular
- **Soğuk önbellek ölçümü:** Yeniden başlatmadan hemen sonra bir `index_real` koşusu. Tahmin `RecentReady` ≈ 2-3 s.
- `cargo deny` bu oturumda ağ olmadığı için yerel advisory veritabanıyla (`--offline`) çalıştı.
- Advisor olayları DB'de ayrı işaret taşımıyor. UI'da etiket gerekirse şema değişir.
- Release `opt-level` kararı (Adım 02'den) hâlâ açık.

### Sonraki adıma notlar (Adım 05)
- `index_files` her çağrıda hesabın tüm `source_files` satırlarını yükler. Watcher'ın sık, küçük partileri için yol bazlı bir sorgu veya bellekte tutulan bir önbellek gerekebilir.
- `IndexEvent` (ilerleme + `RecentReady`) şimdilik callback. EventBus'a bağlanmalı.
- `RetentionScheduler` uygulamada başlatılmalı (`spawn(store, DEFAULT_KEEP_DAYS, CHECK_EVERY)`); drop edilince durur.
- Ana oturumlarda `root_id` NULL. Ağaç toplamlarında `COALESCE(root_id, id)` kullanın.
- Silinen dosyalar `deleted_at` ile işaretleniyor. Watcher'ın silme olayı `Writer::mark_files_deleted` ile işlenebilir.
- Canlı oturumların (`sessions/*.json`) `name` alanı başlık önceliğine eklenecekse `TitleSource`'a yeni bir değer ve sıra gerekir.
