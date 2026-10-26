# Adım 04 — Claude JSONL indeksleyici

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1 | 02, 03 | 1-2 oturum | ⬜ |

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

- [ ] **Kayıt tipleri envanteri:** Gerçek veride hangi `type` değerleri var? (`assistant`, `user`, `attachment`, `api-request`, `api-request-blob`, `mode`, `summary`, `custom-title`…) Gözlem sırasında `api-request` ve `api-request-blob` tipleri görüldü:
  - Bu kayıtlar `usage` taşıyor mu?
  - Taşıyorsa assistant kayıtlarıyla çift sayıma yol açar mı?
  - Karar ver ve `data-sources.md`'ye işle.
- [ ] **Oturum başlığı kaynakları:** `summary`, `custom-title` veya benzeri kayıtlar ve `sessions/*.json` içindeki `name`. Öncelik sırasını belirle. Başlık için **içerik okunmaz**; yalnızca bu meta kayıtlar kullanılır.
- [ ] **Subagent kimliği:** Subagent dosyalarındaki `sessionId` ana oturumu mu gösteriyor? Dosya yolundaki üst dizin adıyla karşılaştır.
- [ ] **`iterations` alanı:** Birden fazla eleman içeren `iterations` var mı? Varsa üst düzey `usage` ile ilişkisi ne?
- [ ] **`speed` değerleri ve `fast` modu:** Hangi değerler görülüyor?
- [ ] Bulguları `docs/data-sources.md` S3 bölümüne ekle.

### Uygulama

- [ ] `providers/claude/record.rs`: Seçilen strateji (Adım 02) ile ödünç alan struct'lar. `message.content` için `IgnoredAny`. Gerekli alanlar: `type`, `uuid`, `sessionId`, `isSidechain`, `timestamp`, `cwd`, `gitBranch`, `version`, `entrypoint`, `requestId`, `message.id`, `message.model`, `message.usage.*`.
- [ ] `ParseOutcome`: `UsageEvent` | `SessionMeta` (cwd, branch, başlık…) | `Skip` | `Unknown(reason)`. Unknown sayaçları tutulur (şema sapması sinyali, `architecture.md` §15).
- [ ] Proje çözümleme: `cwd` → `path_norm` dönüşümü:
  - Sürücü harfi büyük harfe çevrilir.
  - `\` ve `/` tek biçime getirilir.
  - Sondaki ayraç silinir.
  - Windows'ta karşılaştırma büyük-küçük harfe duyarsızdır.
  - `display_name` olarak klasör adı kullanılır.
  - Kayıtlarda `cwd` hiç yoksa dizin adından kayıplı çözümleme yapılır ve proje "çözümlenmemiş" olarak işaretlenir.
- [ ] Subagent bağlama: `<proje>/<sessionUuid>/subagents/agent-<id>.jsonl` → `kind=subagent`, `uuid="agent-<id>"`, `parent_id` = ana oturum. Ana oturum henüz indekslenmediyse yer tutucu bir oturum oluşturulur ve ana dosya geldiğinde doldurulur.
- [ ] `indexer/reader.rs`:
  - `offset`'ten itibaren 1 MB'lık parçalarla okur, `memchr` ile `\n` arar.
  - Son yarım satırı bir sonraki tura bırakır; offset yalnızca tam satır sonuna ilerletilir.
  - `size < offset` ise truncate kabul edilir ve dosya baştan işlenir (tekilleştirme sayesinde idempotent).
  - Silinen dosya → `source_files.deleted_at` set edilir. Veriler korunur.
- [ ] `indexer/mod.rs`:
  - `full_scan()`: envanter çıkarılır, değişen dosyalar `mtime` değerine göre azalan sırada (en yeni önce) kuyruğa alınır.
  - `index_files(paths)`: aynı işlem hattını verilen dosya listesi için çalıştırır.
  - Worker sayısı `min(4, çekirdek/2)` (Adım 02 kararına göre). Worker'lar parse eder, yazıcı thread 5.000 olayda bir veya 250 ms'de bir batch yazar.
  - `source_files.offset` değeri, ilgili batch yazıldıktan **sonra**, aynı transaction içinde güncellenir. Çökme sonrası tutarlılık bu sırayla sağlanır.
- [ ] Arka plan önceliği: Windows'ta worker thread'leri `SetThreadPriority(THREAD_MODE_BACKGROUND_BEGIN)` ile başlar (`windows-sys`, `unsafe` blok yerel `allow` ve gerekçe yorumuyla). Diğer platformlar için no-op stub bırakılır.
- [ ] İlerleme: `IndexProgress { files_done, files_total, bytes_done, bytes_total, phase }`. Şimdilik bir callback/kanal; Adım 05'te EventBus'a bağlanacak.
- [ ] "Mevcut pencere hazır" sinyali: Son 7 güne ait dosyalar bittiğinde `IndexMilestone::RecentReady` yayınlanır.
- [ ] `examples/sanitize_jsonl.rs`:
  - `message.content`, `thinking`, `signature` ve araç girdi/çıktıları silinir.
  - `cwd` ve yollar `proj-<hash8>` biçimine çevrilir.
  - Kimlikler korunur, çünkü tekilleştirme testleri bunlara dayanıyor.
  - Çıktı `testdata/claude/` altına yazılır.
  - **Gerçek kullanıcı verisi commit'lenmez.** Fixture'lar sentetik olabilir veya anonimleştirilmiş kısa kesitlerden oluşabilir.
- [ ] Fixture'lar:
  - Tekrar eden ve kısmi kayıtlar (`iterations: []`)
  - Yarım son satır
  - Truncate
  - Subagent ağacı
  - Bilinmeyen kayıt tipi
  - Hatalı JSON satırı
  - `cache_creation` kırılımı olmayan kayıt
- [ ] Testler: fixture'lar üzerinde snapshot testleri (insta) ve uçtan uca "dosya ekle → indeksle → toplamları doğrula" testleri. Artımlı tur: dosyaya satır ekle → yalnızca yeni satırlar işlenir.
- [ ] **Gerçek veri doğrulaması** (yerel, commit'lenmez):
  - Ölçülecekler: ilk indeksleme süresi, tepe RSS, DB boyutu.
  - Birkaç günlük token toplamlarını `npx ccusage@latest daily --json` ile karşılaştır. Fark ±%2'yi aşarsa nedenini açıkla. ccusage tekilleştirmeyi farklı yapıyor olabilir; bu durumda hangisinin doğru olduğunu ham veriden göster.
- [ ] Ölçümü `docs/perf/<tarih>-indexer.md` olarak kaydet.

## Teknik notlar ve tuzaklar

- **Tekilleştirme kuralı:** "İlk görüleni tut" yaklaşımı **yanlıştır**. Kısmi kayıtlar önce gelebilir. Kural, Adım 03'teki UPSERT'te uygulanıyor; burada yalnızca doğru `msg_key` ve tam `usage` alanları üretilmeli.
- Satırların bir kısmı onlarca KB'tır. Ön filtre, `type` alanını bulmadan önce `"usage"` arar; `"usage"` geçmeyen satırlar parse edilmez. Ama `SessionMeta` için gereken kayıtları (başlık, cwd) atlamamaya dikkat et: bu kayıtlar için ayrı ve ucuz bir filtre kullan.
- Windows'ta Claude Code'un yazdığı bir dosyayı okumak için `FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE` paylaşım modu gerekir. Rust'ın `File::open` varsayılanı uygundur, ama dosyayı **asla kilitleme**.
- `mtime` çözünürlüğü ve saat kayması nedeniyle "değişti mi?" kontrolü `(size, mtime)` çiftiyle yapılır. Boyut aynı ama `mtime` farklıysa dosya baştan işlenmez; offset'ten devam edilir.
- 1.500 proje klasörü var. Envanter `read_dir` özyinelemesi ile çıkarılır. `walkdir` kullanılabilir ama `tool-results` gibi dizinler budanmalı (prune).

## Kabul kriterleri

- [ ] Referans veri setinde ilk indeksleme süresi bütçe içinde (Adım 02'de revize edilen değer). `RecentReady` ≤ 5 s.
- [ ] Yeniden başlatma sonrası değişmemiş veri seti için tarama ≤ 500 ms.
- [ ] ccusage karşılaştırması ±%2 içinde veya fark gerekçelendirilmiş.
- [ ] Bütün fixture testleri geçer. Hatalı satır indekslemeyi durdurmaz.
- [ ] Keşif bulguları `data-sources.md`'ye işlenmiş.

## Doğrulama

```powershell
cargo test -p tokenradar-core indexer
cargo run --release -p tokenradar-core --example index_real -- "$env:USERPROFILE\.claude" --db "$env:TEMP\tr-test.db"
npx ccusage@latest daily --json --since 20261020   # karşılaştırma için
```

## Devir notu

_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
