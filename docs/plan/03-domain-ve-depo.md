# Adım 03 — Çekirdek domain modeli ve SQLite deposu

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1 | 01 | 1 oturum | ⬜ |

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

- [ ] `domain/` modülü: tipler + `serde` + `specta::Type` (UI'ya gidecek olanlar için). Id'ler newtype olsun (`struct SessionId(i64)`).
- [ ] `secret.rs`: `Secret<T: Zeroize>` + birim testi (`format!("{:?}")` çıktısında sırrın görünmediğini doğrula).
- [ ] `store/migrations/0001_init.sql`: `architecture.md` §6'daki şema. Gerekli indeksleri ve `FOREIGN KEY` ilişkilerini ekle. Şema farkı çıkarsa doküman da güncellenir.
- [ ] `store/mod.rs`:
  - `Store::open(path)`
  - PRAGMA'lar: `journal_mode=WAL`, `synchronous=NORMAL`, `foreign_keys=ON`, `busy_timeout=5000`, `temp_store=MEMORY`, `cache_size=-8000` (8 MB)
  - Migration çalıştırıcısı
  - `Store::writer()` → komut kanalı; `Store::read(|conn| …)` → havuzdan bağlantı
- [ ] `store/events.rs`: `upsert_events(batch: &[UsageEvent])`:
  - Tek transaction içinde hazırlanmış (prepared) statement'lar kullanılır.
  - Her olay için önce mevcut satır okunur (msg_key ile). Yeni olayın `output` değeri ≥ eskisininkiyse güncellenir ve `usage_5m` satırına **(yeni − eski)** farkı uygulanır. Kayıt yoksa eklenir ve `usage_5m` artırılır.
  - Kova: `bucket_ts = ts - ts % 300_000`.
  - Olayın kovası değişmiş olabilir (aynı mesajın farklı satırlarında `ts` farklıysa). Bu durumda eski kovadan çıkarıp yeni kovaya ekle.
- [ ] `store/retention.rs`: `prune_raw_events(older_than)`. `usage_5m` silinmez. İşlem `VACUUM` gerektirmez; `incremental_vacuum` değerlendirilsin.
- [ ] `pricing/`:
  - `claude.toml` (sürümlü). Her model ailesi için `input`, `output`, `cache_write_5m_mult = 1.25`, `cache_write_1h_mult = 2.0`, `cache_read_mult = 0.10`, `fast_mult`.
  - **Fiyatları resmî kaynaktan doğrula:** `claude-api` skill'ini kullan veya Anthropic'in fiyat sayfasını oku. Dosyaya kaynak URL'sini ve doğrulama tarihini yorum olarak yaz. Fiyatları hafızadan yazma.
  - `family_of(model_id)`: önek ve desen eşlemesi (opus/sonnet/haiku/fable…). Bilinmeyen model → `unknown` ailesi. Bu durumda en yakın fiyat kullanılır, bayrak set edilir ve bir kez uyarı loglanır.
  - `cost_micro_usd(&UsageEvent) -> i64` (bkz. `architecture.md` §9.1).
- [ ] `paths.rs`: `data_dir()`, `config_path()`, `logs_dir()`, `spool_dir()`. Portable mod: exe'nin yanında `portable` dosyası varsa veriler `./data`'ya yazılır.
- [ ] Testler:
  - Boş DB'den migration; ardından ikinci açılışta migration'ın tekrar çalışmadığının (idempotent) kontrolü.
  - Tekilleştirme senaryoları:
    - Aynı anahtar, artan `output`.
    - Aynı anahtar, azalan `output` (eski değer korunmalı).
    - Aynı anahtar, farklı kova.
  - **Özellik testi (proptest):** Rastgele sırayla gelen tekrar/kısmi olay dizileri sonrasında `usage_5m` toplamları ≡ "her anahtarın nihai olayı" üzerinden hesaplanan toplam.
  - Maliyet fonksiyonu için bilinen girdi/çıktı vektörleri (`cache_creation` kırılımı olan ve olmayan kayıtlar, fast mode).
  - Saklama işi: `usage_5m` değişmeden kalır.
- [ ] Criterion bench: 100 bin olayın toplu UPSERT süresi (hedef ≥ 150 bin olay/s, WAL, NVMe).

## Teknik notlar ve tuzaklar

- **Tek yazar ilkesi:** SQLite'ta eşzamanlı yazma kilitlenmeye yol açar. Tüm yazmalar tek bir thread üzerinden geçer. İndeksleyicinin worker'ları parse eder, yazıcıya batch gönderir.
- `msg_key` 16 baytlık bir BLOB'dur. `message.id` yoksa `uuid` alanı, o da yoksa satır hash'i yedek olarak kullanılır. Yedek anahtar kullanılan olaylar sayılıp loglanır.
- Zaman damgaları JSONL'de ISO 8601 biçimindedir. jiff ile parse edilir; hatalı damga → satır `Unknown` olarak sayılır.
- `rusqlite` için `bundled` feature'ı kullanılır, sistem SQLite'ına bağımlılık olmaz. `modern_sqlite` gerekmez.
- Şema değişiklikleri her zaman **yeni bir migration** ile yapılır. `0001` dosyası bir kez yayınlandıktan sonra asla değiştirilmez. v0.1 yayınlanana kadar düzenlenmesine izin var; bu not devir notunda tutulur.

## Kabul kriterleri

- [ ] Tüm testler geçer. Özellik testi en az 1.000 rastgele vaka ile çalışır.
- [ ] Bench'te toplu UPSERT hızı ≥ 150 bin olay/s (değilse nedeni ve iyileştirme planı devir notuna yazılır).
- [ ] `pricing/claude.toml` resmî kaynakla doğrulanmış, kaynak URL ve tarih yazılı.
- [ ] Token veya içerik verisi taşıyan hiçbir tip `Debug` çıktısında sızdırmıyor.

## Doğrulama

```powershell
cargo test -p tokenradar-core
cargo bench -p tokenradar-core --bench store
cargo clippy -p tokenradar-core --all-targets -- -D warnings
```

## Devir notu

_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
