# Adım 03 — Çekirdek domain modeli ve SQLite deposu

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1 | 01 | 1 oturum | ✅ |

## Amaç

`tokenradar-core` içinde provider-agnostik domain tiplerini, SQLite şemasını (v1), migration altyapısını, tipli depo API'sini ve API karşılığı maliyet hesabını (`Pricing`) kurmak. Bu adımın sonunda üst katmanlar veriyi yazıp okuyabilir hale gelir.

## Önce oku

- `docs/architecture.md` §5, §6, §7, §9.1, §14, §16
- `docs/data-sources.md` S3 (tekilleştirme kuralı)
- `docs/decisions.md`: ADR-007, ADR-008, ADR-020

## Kapsam

- **Domain tipleri:** `AccountId`, `SessionId`, `ProjectId`, `ModelId`, `Account`, `Project`, `Session` (kind: main/subagent, parent/root), `Model` (+ family), `UsageEvent`, `LimitObservation`, `LimitKind`, `Scope`, `Source`, `LimitWindow`, `Timestamp` (UTC ms, jiff tabanlı), `MicroUsd`.
- `Secret<T>` tipi: `Debug`/`Display` çıktısında `***` basar, drop anında `zeroize` eder.
- **`Store`:**
  - Tek yazar thread (komut kanalı) ve okuyucu bağlantı havuzu.
  - PRAGMA'lar.
  - Migration'lar (`PRAGMA user_version` + `migrations/NNNN_*.sql`, `include_str!` ile gömülü).
- **Depo API'leri:**
  - Hesap, proje ve oturum için upsert.
  - **Olay UPSERT'i ("max output kazanır") + `usage_5m` fark uygulaması**, toplu (batch) transaction.
  - Snapshot ekleme ve sorgu.
  - `kv` tablosu.
  - Saklama işi: ham olayların silinmesi (`usage_5m` dokunulmaz).
- **`Pricing`:** Gömülü `pricing/claude.toml`, model ailesi tespiti, `cost_micro_usd(event)`.
- **Veri dizini çözümleme:** `directories` crate'i ve portable mod bayrağı.

## Kapsam dışı

- JSONL okuma (Adım 04), limit füzyon mantığı (Adım 06), kalibrasyon (Adım 13).
- `calibrations` ve `attributions` tabloları şemada **oluşturulur**, ama depo API'leri Adım 13-14'te yazılır.

## Görevler

- [x] `domain/` modülü: tipler + `serde` + `specta::Type` (UI'ya gidecek olanlar için). Id'ler newtype olsun (`struct SessionId(i64)`).
- [x] `secret.rs`: `Secret<T: Zeroize>` + birim testi (`format!("{:?}")` çıktısında sırrın görünmediğini doğrula).
- [x] `store/migrations/0001_init.sql`: `architecture.md` §6'daki şema. Gerekli indeksleri ve `FOREIGN KEY` ilişkilerini ekle. Şema farkı çıkarsa doküman da güncellenir.
- [x] `store/mod.rs`:
  - `Store::open(path)`
  - PRAGMA'lar: `journal_mode=WAL`, `synchronous=NORMAL`, `foreign_keys=ON`, `busy_timeout=5000`, `temp_store=MEMORY`, `cache_size=-8000` (8 MB)
  - Migration çalıştırıcısı
  - `Store::writer()` → komut kanalı; `Store::read(|conn| …)` → havuzdan bağlantı
- [x] `store/events.rs`: `upsert_events(batch: &[UsageEvent])`:
  - Tek transaction içinde hazırlanmış (prepared) statement'lar kullanılır.
  - Her olay için önce mevcut satır okunur (msg_key ile). Yeni olayın `output` değeri ≥ eskisininkiyse güncellenir ve `usage_5m` satırına **(yeni − eski)** farkı uygulanır. Kayıt yoksa eklenir ve `usage_5m` artırılır.
  - Kova: `bucket_ts = ts - ts % 300_000`.
  - Olayın kovası değişmiş olabilir (aynı mesajın farklı satırlarında `ts` farklıysa). Bu durumda eski kovadan çıkarıp yeni kovaya ekle.
- [x] `store/retention.rs`: `prune_raw_events(older_than)`. `usage_5m` silinmez. İşlem `VACUUM` gerektirmez; `incremental_vacuum` değerlendirilsin.
- [x] `pricing/`:
  - `claude.toml` (sürümlü). Her model ailesi için `input`, `output`, `cache_write_5m_mult = 1.25`, `cache_write_1h_mult = 2.0`, `cache_read_mult = 0.10`, `fast_mult`.
  - **Fiyatları resmî kaynaktan doğrula:** `claude-api` skill'ini kullan veya Anthropic'in fiyat sayfasını oku. Dosyaya kaynak URL'sini ve doğrulama tarihini yorum olarak yaz. Fiyatları hafızadan yazma.
  - `family_of(model_id)`: önek ve desen eşlemesi (opus/sonnet/haiku/fable…). Bilinmeyen model → `unknown` ailesi. Bu durumda en yakın fiyat kullanılır, bayrak set edilir ve bir kez uyarı loglanır.
  - `cost_micro_usd(&UsageEvent) -> i64` (bkz. `architecture.md` §9.1).
- [x] `paths.rs`: `data_dir()`, `config_path()`, `logs_dir()`, `spool_dir()`. Portable mod: exe'nin yanında `portable` dosyası varsa veriler `./data`'ya yazılır.
- [x] Testler:
  - Boş DB'den migration; ardından ikinci açılışta migration'ın tekrar çalışmadığının (idempotent) kontrolü.
  - Tekilleştirme senaryoları:
    - Aynı anahtar, artan `output`.
    - Aynı anahtar, azalan `output` (eski değer korunmalı).
    - Aynı anahtar, farklı kova.
  - **Özellik testi (proptest):** Rastgele sırayla gelen tekrar/kısmi olay dizileri sonrasında `usage_5m` toplamları ≡ "her anahtarın nihai olayı" üzerinden hesaplanan toplam.
  - Maliyet fonksiyonu için bilinen girdi/çıktı vektörleri (`cache_creation` kırılımı olan ve olmayan kayıtlar, fast mode).
  - Saklama işi: `usage_5m` değişmeden kalır.
- [x] Criterion bench: 100 bin olayın toplu UPSERT süresi (hedef ≥ 150 bin olay/s, WAL, NVMe).

## Teknik notlar ve tuzaklar

- **Tek yazar ilkesi:** SQLite'ta eşzamanlı yazma kilitlenmeye yol açar. Tüm yazmalar tek bir thread üzerinden geçer. İndeksleyicinin worker'ları parse eder, yazıcıya batch gönderir.
- `msg_key` 16 baytlık bir BLOB'dur. `message.id` yoksa `uuid` alanı, o da yoksa satır hash'i yedek olarak kullanılır. Yedek anahtar kullanılan olaylar sayılıp loglanır.
- Zaman damgaları JSONL'de ISO 8601 biçimindedir. jiff ile parse edilir; hatalı damga → satır `Unknown` olarak sayılır.
- `rusqlite` için `bundled` feature'ı kullanılır, sistem SQLite'ına bağımlılık olmaz. `modern_sqlite` gerekmez.
- Şema değişiklikleri her zaman **yeni bir migration** ile yapılır. `0001` dosyası bir kez yayınlandıktan sonra asla değiştirilmez. v0.1 yayınlanana kadar düzenlenmesine izin var; bu not devir notunda tutulur.

## Kabul kriterleri

- [x] Tüm testler geçer. Özellik testi en az 1.000 rastgele vaka ile çalışır.
- [x] Bench'te toplu UPSERT hızı ≥ 150 bin olay/s (değilse nedeni ve iyileştirme planı devir notuna yazılır).
- [x] `pricing/claude.toml` resmî kaynakla doğrulanmış, kaynak URL ve tarih yazılı.
- [x] Token veya içerik verisi taşıyan hiçbir tip `Debug` çıktısında sızdırmıyor.

## Doğrulama

```powershell
cargo test -p tokenradar-core
cargo bench -p tokenradar-core --bench store
cargo clippy -p tokenradar-core --all-targets -- -D warnings
```

## Devir notu

**Tarih:** 2026-10-27 · **Dal:** `step/03-domain-store` · **Rapor:** `docs/perf/2026-10-27-store.md` · **Karar:** ADR-025

### Yapılanlar
- **`domain/`:**
  - Newtype id'ler: `AccountId`, `SessionId`, `ProjectId`, `ModelId`.
  - `Timestamp` (UTC ms, jiff ile ISO parse, `bucket_5m`) ve `MicroUsd`.
  - `Provider`, `SessionKind`, `ModelFamily`, `Source` için saklama metinli enum'lar (`as_str`/`FromStr`). `LimitKind::Other(String)`; `Scope` `model:`/`surface:` biçiminde saklanır.
  - `*Spec` (yazılacak) ve satır tipleri: `Account`, `Project`, `Session`, `Model`.
  - `UsageEvent` = kimlikler + `Usage { tokens: TokenCounts, web_search, web_fetch, speed }` + `cost`.
  - `MsgKey::from_ids` (xxh3-128, alanlar arası ayraçlı) ve `from_fallback`. `Debug` çıktısı yalnızca ilk 4 baytı gösterir.
  - UI'ya gidecek tipler `serde` + `specta::Type` türetir.
- **`secret.rs`:** `Secret<T: Zeroize>`. `Debug`/`Display` çıktısı `***`; drop'ta zeroize edilir; `Clone` ve `Serialize` yok. Drop'ta zeroize çağrıldığı bir probe ile test edildi.
- **`store/`:**
  - Tek yazar thread'i (`Writer::call` + tipli yardımcılar) ve okuyucu havuzu (`Store::read`).
  - PRAGMA'lar plandaki gibi. Ek olarak `auto_vacuum=INCREMENTAL`.
  - `PRAGMA user_version` + gömülü migration'lar. Daha yeni şema sürümü → `SchemaTooNew`.
  - Hesap, proje ve oturum upsert'leri. Oturum birleştirme: `None` alanlar eski değeri korur; `first_ts` yalnızca geriye, `last_ts` yalnızca ileri gider.
  - Model upsert'ünde saklanan `unknown` aile, model tanınınca düzeltilir.
  - Olay UPSERT'ü ve `usage_5m` fark uygulaması: kova/oturum/model değişimi doğru taşınır, boşalan satır silinir.
  - Snapshot ekleme (`Estimate` reddedilir), `latest_snapshots`, `snapshots_between`, `kv`.
  - Saklama işi: 10 bin satırlık parçalar + `incremental_vacuum`.
- **`pricing/`:** `claude.toml` (sürüm `2026-10-27`; kaynak URL ve doğrulama tarihi dosyada), `Pricing::cost(model, &Usage) -> Priced { cost, model_known }`, `family_of`.
- **`paths.rs`:** `Paths::resolve()` (exe'nin yanında `portable` dosyası varsa `./data`), `data_dir`, `config_path`, `db_path`, `logs_dir`, `spool_dir`.
- **Testler:** 48 birim testi ve 1.000 vakalık proptest. Ayrıca fiyat tablosunun resmî sayfayla satır satır karşılaştırması (5m/1h yazma, okuma, çıktı, Haiku uzun kademe).
- **Bench:** Yeni olaylar 172 bin olay/s (GA 143–203 bin), mevcut olaylar 247 bin olay/s.

### Plandan sapmalar
- **ADR-025 (kullanıcı onaylı):** Fiyatlar aile değil model bazında tutuluyor. Cache okuma çarpanı modele göre değişiyor (0,025 / 0,05 / 0,10). Haiku 5.5 uzun istem kademesi ve web search ücreti eklendi. `architecture.md` §5 ve §9.1 güncellendi.
- **`cost_micro_usd(&UsageEvent)` yerine `Pricing::cost(model_name, &Usage)`:** `UsageEvent` model adını değil `ModelId` taşıyor. Maliyet olay oluşturulurken hesaplanıp `UsageEvent::cost` alanına yazılıyor.
- **Saklama filigranı (planda yoktu):** Hesap başına `kv` → `retention.pruned_before.<id>`. Filigrandan eski yeni anahtarlar UPSERT'te atlanıyor (`skipped_pruned`), aksi hâlde pruning sonrası yeniden indeksleme `usage_5m`'de çift sayım yapardı. `architecture.md` §6'ya yazıldı.
- **Bench batch boyutu 5.000:** 1.000'lik batch'te hız ~100 bin olay/s. Neden, rastgele `msg_key` indeksinde commit başına sayfa yeniden yazımı. Eğri perf raporunda.
- **Okuyucu bağlantıları:** `cache_size=-2000`, boşta en fazla 1 bağlantı. `Store::release_memory()` eklendi (`shrink_memory` + boştaki okuyucuları kapatma). Gerekçe tray bellek bütçesi; yazar bağlantısı planla aynı (8 MB).
- **Şema:** Tüm ilişkilere FK, `NOT NULL` kısıtları, ek indeksler, `kv WITHOUT ROWID`. Kanonik şema `0001_init.sql`; `architecture.md` §6 not düşüldü.
- **`ModelFamily::Synthetic`:** Claude Code'un yerel olarak yazdığı `<synthetic>` kayıtları sıfır maliyetli, bilinen bir model olarak eşleşiyor (her açılışta uyarı loglanmasın diye).
- **`deny.toml`:** `BSL-1.0` izin listesine eklendi. `xxhash-rust` artık normal bağımlılık.
- **Yeni bağımlılıklar:** `directories 6`, `jiff 0.2` (yalnızca `std`), `rusqlite 0.41` (`bundled`), `serde`, `specta =2.0.0-rc.26` (tauri-specta ile aynı sürüm), `thiserror 2`, `toml 1` (parse), `tracing`, `xxhash-rust`, `zeroize`. Dev: `proptest`, `tempfile`.

### Açık konular
- **`0001_init.sql` v0.1 yayınlanana kadar düzenlenebilir.** Sonrasında şema değişiklikleri yalnızca yeni migration ile yapılır.
- **`attributions` birincil anahtarı** `(window_id, root_session_id)`, ama `root_session_id` NULL olabiliyor ("Diğer" kovası). SQLite NULL'ları birbirinden farklı saydığı için aynı pencerede birden çok "Diğer" satırı oluşabilir. Adım 14 bunu düzeltmeli (ör. `0` gözcü değeri veya kısmi benzersiz indeks).
- **`inference_geo: "us"` 1,1× çarpanı** uygulanmadı (ADR-025). Gerekirse Adım 04'te alan okunup çarpan eklenebilir.
- **Specta ve 64-bit tamsayılar:** id, `Timestamp` ve `MicroUsd` `i64`. Adım 09'da tauri-specta dışa aktarımında `BigIntExportBehavior::Number` seçilmeli; değerler 2^53'ün altında.
- **Bellek:** Yazar önbelleği 8 MB + okuyucu 2 MB. Tray bütçesine (25 MB) etkisi ölçülmedi; Adım 04/05 bellek ölçümünde kontrol edilmeli.
- **Bench varyansı:** Güven aralığının alt ucu (143 bin) hedefin biraz altında. Kaynağı disk (WAL commit ve checkpoint).

### Sonraki adıma notlar (Adım 04)
- **Batch:** Yazıcıya en az **5.000 olaylık** batch'ler gönder. `Writer::upsert_events(Vec<UsageEvent>)` tek transaction'da çalışır ve `UpsertStats` döner (`inserted`/`updated`/`kept_existing`/`skipped_pruned`).
- **Eşitlik kuralı:** UPSERT eşit `output` değerinde son geleni seçer. Adım 02'nin "deterministik eşitlik kuralı" notu için batch içi ve batch'ler arası sıra deterministik olmalı (ör. dosya yolu + offset sırası).
- **Olay oluşturma sırası:**
  - `MsgKey::from_ids(message_id, request_id)`; kimlik yoksa `MsgKey::from_fallback(uuid veya satır)` kullan ve say.
  - `TokenCounts::split_cache_creation(total, breakdown)`.
  - `Pricing::claude().cost(model, &usage)`.
  - `model_known == false` ise modeli `family_of(model)` ile kaydet; aile sonradan tanınınca otomatik düzelir.
- **Zaman:** `Timestamp::parse_iso` hatalı damgada `Err` döner → satır `Unknown`.
- **Oturum ve proje:** `upsert_session` ve `upsert_project` id döner. İndeksleyici bunları önbelleğe almalı; her olay için writer'a gidip gelme pahalı.
- **İndeksleme bitince** `Store::release_memory()` çağrılmalı.
