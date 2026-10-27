# Adım 02 — Performans spike'ı ve mimari kapı

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1 | 01 | 1 oturum | ✅ |

## Amaç

Mimarinin dayandığı iki kritik varsayımı **ölçerek** doğrulamak:

1. **WebView2 yaşam döngüsü (ADR-004 kapısı):** Tüm pencereler yok edildiğinde `msedgewebview2.exe` süreçleri sonlanıyor mu ve toplam RAM ≤ 30 MB'a dönüyor mu? Soğuk ve sıcak panel açılışı ne kadar sürüyor?
2. **JSONL parse hızı:** 11,7 GB'lık gerçek veri setinde hangi parse stratejisi en hızlı? İlk indeksleme bütçesi (≤ 60 s) gerçekçi mi?

Bu adım **ölçüm ve karar** adımıdır. Üretim kodu yazılmaz, ama ölçüm scriptleri kalıcıdır.

## Önce oku

- `docs/performance-budget.md` (tamamı)
- `docs/architecture.md` §2
- `docs/data-sources.md` S3
- `docs/decisions.md`: ADR-004, ADR-007

## Kapsam

- `scripts/measure-memory.ps1`: süreç ağacı bellek ölçümü (CSV + özet tablo).
- Uygulamada **yalnızca debug derlemede** etkin bir spike senaryosu: `TOKENRADAR_SPIKE=lifecycle` ortam değişkeni.
- WebView2 bellek ayarlarının denenmesi.
- `crates/tokenradar-core/benches/parse.rs` (criterion) + `examples/parse_real.rs` (gerçek veri, salt-okunur).
- Sonuç raporu ve ADR güncellemeleri.

## Kapsam dışı

- Gerçek indeksleyici (Adım 04). Burada yalnızca parse stratejileri karşılaştırılır.
- UI tasarımı.

## Görevler

### A. WebView2 yaşam döngüsü

- [x] `scripts/measure-memory.ps1 -ProcessName tokenradar-app -IntervalMs 500 -Out docs/perf/raw/<tarih>-lifecycle.csv`:
  - Kök süreci ve tüm alt süreçlerini bulur (`Get-CimInstance Win32_Process` ile `ParentProcessId` üzerinden özyinelemeli).
  - Her örnekte `PrivateMemorySize64`, `WorkingSet64` ve süreç sayısını yazar.
  - Bitişte min/max/son değer özetini basar.
- [x] Spike senaryosu (debug derleme, `TOKENRADAR_SPIKE=lifecycle`). Her adım zaman damgasıyla loglanır:
  1. 30 s boşta bekle (taban ölçümü).
  2. Paneli **soğuk** oluştur. Oluşturma çağrısından frontend'in `DOMContentLoaded` → Rust'a gönderdiği `ready` olayına kadar geçen süreyi ölç.
  3. 10 s bekle, sonra paneli gizle.
  4. 5 s sonra **sıcak** göster ve ölç. Ardından tekrar gizle.
  5. Pencereyi **yok et** (`close`/`destroy`), 60 s gözle: süreç sayısı ve RAM.
  6. İki pencere birlikte (panel + 220×44 boyutunda widget benzeri bir pencere): ikinci pencerenin **ek** maliyeti.
  7. Gizli ama sıcak pencerede `MemoryUsageTargetLevel = Low` (`ICoreWebView2_19`) dene. `with_webview` ile ham erişim gerekir.
- [x] Senaryoyu en az 3 kez çalıştır ve medyan değerleri raporla.
- [x] ~~Süreçler sonlanmıyorsa sırayla şu çareleri dene ve her birini ölç~~ Gerekmedi: süreçler kendiliğinden sonlanıyor (ADR-023). Aşağıdaki alt maddeler uygulanmadı:
  - Tüm webview'leri drop etmek.
  - Ortamı (environment) serbest bırakmak.
  - `additional_browser_args` (örneğin `--renderer-process-limit=1`, gereksiz `msEdge` özelliklerinin kapatılması).
- [x] ~~Hâlâ ≤ 30 MB'a dönülemiyorsa **yedek plan için prototip** yap: UI'nın `tokenradar-app --ui` olarak ayrı bir süreçte açılması. Bu süreç kapatıldığında toplam RAM'i ölç.~~ Gerekmedi (ADR-023).

### B. JSONL parse stratejileri

- [x] Gerçek veriyi **repoya kopyalamadan**, yerinde ve salt-okunur kullan (`%USERPROFILE%\.claude\projects`).
- [x] Karşılaştırılacak stratejiler (aynı çıktı: `UsageEvent` alanları):
  - **A:** `serde_json::Value` ile tam parse (taban).
  - **B:** Tipli, ödünç alan (`&'a str`) struct + `content` için `IgnoredAny`.
  - **C:** `memchr::memmem` ile `"type":"assistant"` ve `"usage"` ön filtresi + B.
  - **D:** C + `simd-json` (tipli). Mutable buffer gerektirdiği için kopya maliyeti de ölçülmeli.
- [x] Metrikler:
  - MB/s ve satır/s (tek thread)
  - 4 thread ile toplam süre
  - Tepe RSS
  - Soğuk önbellek ölçümü: mümkünse yeniden başlatma sonrası, değilse RAMMap ile "Empty Standby List"
- [x] **Stat yürüyüşü:** 5.200 dosyanın `metadata()` taramasının süresi. Bu değer "yeniden başlatma sonrası artımlı tarama ≤ 500 ms" bütçesinin dayanağı.
- [x] Doğruluk kontrolü: Dört strateji de aynı tekilleştirilmiş olay sayısını ve token toplamlarını vermeli.

### C. Rapor ve kararlar

- [x] `docs/perf/<tarih>-spike.md`: ölçüm tabloları, yöntem, makine bilgisi ve yorum.
- [x] `docs/decisions.md`:
  - ADR-004 için sonuç: **Onaylandı** veya **Yerini aldı → ADR-022 (ayrık UI süreci)**.
  - Yeni ADR: seçilen parse stratejisi ve thread sayısı.
- [x] Gerekirse `docs/performance-budget.md` içindeki indeksleme ve bellek bütçelerini gerçek ölçümlere göre revize et (gerekçesiyle).
- [x] ~~Yedek plana geçildiyse **etkilenen adımların** (05, 07, 10, 18) görev listelerini güncelle ve devir notuna yaz.~~ Gerekmedi (ADR-023).

## Teknik notlar ve tuzaklar

- WebView2 süreç ağacı ana sürecin **altında değil**, `msedgewebview2.exe` olarak ayrı bir ağaçta görünebilir. Tarayıcı süreci uygulamanın alt sürecidir, renderer'lar ise tarayıcı sürecinin alt süreçleridir. Script ağacı özyinelemeli izlemeli; ölçümü yalnızca ada göre yapmak, sistemdeki **diğer** WebView2 uygulamalarını da sayar.
- Tauri/wry webview oluştururken bir `WebContext` (kullanıcı veri dizini) kullanır. Aynı veri dizinini paylaşan pencereler aynı tarayıcı sürecini paylaşır. Ölçümde bu durumu not et.
- `PrivateMemorySize64` ile Görev Yöneticisi'ndeki "Bellek" sütunu (private working set) farklıdır. İkisini de raporla, bütçede private working set esas alınır.
- Soğuk açılış süresi `WebView2 environment` oluşturmayı da içerir. Ölçüm, ilk açılış ile sonraki soğuk açılışlar arasında fark gösterebilir; ikisini ayrı raporla.

## Kabul kriterleri

- [x] Ölçümler 3'er tekrarla alınmış ve medyanlar rapordadır.
- [x] ADR-004 için net bir karar kaydedilmiştir (onay veya ADR-022).
- [x] Parse stratejisi seçilmiş ve gerekçesi ADR olarak yazılmıştır. Seçilen stratejinin gerçek veride ölçülen tahmini ilk indeksleme süresi raporda yer alır.
- [x] `scripts/measure-memory.ps1` başka adımlarda tekrar kullanılabilecek durumdadır (parametreli, yardım metni var).

## Doğrulama

```powershell
$env:TOKENRADAR_SPIKE="lifecycle"; pnpm tauri dev    # ayrı terminalde:
.\scripts\measure-memory.ps1 -ProcessName tokenradar-app -IntervalMs 500 -DurationSec 180
cargo bench -p tokenradar-core --bench parse
cargo run --release -p tokenradar-core --example parse_real -- "$env:USERPROFILE\.claude\projects"
```

## Devir notu

**Tarih:** 2026-10-27 · **Dal:** `step/02-perf-spike` · **Rapor:** `docs/perf/2026-10-27-spike.md`

### Yapılanlar
- **`scripts/measure-memory.ps1`:**
  - Kök süreci ada (`-ProcessName`) veya PID'e (`-RootPid`) göre bulur, alt ağacı ebeveyn bağlantılarıyla izler.
  - Private working set, private bytes, working set, CPU süresi, süreç sayısı ve WebView2 süreç sayısını CSV'ye yazar.
  - Yardım metni `Get-Help .\scripts\measure-memory.ps1 -Full` ile görülür.
- **`scripts/spike-lifecycle.ps1`:** Senaryoyu N kez çalıştırır, spike işaretçilerini bellek örnekleriyle birleştirir ve medyan tablosu üretir.
- **`app/src-tauri/src/spike.rs`:**
  - `TOKENRADAR_SPIKE=lifecycle` ile çalışan senaryo.
  - Ham WebView2 erişimi (`with_webview`) ile `SetIsVisible` ve `MemoryUsageTargetLevel` deneyleri.
  - `panel::builder()` üretim panelinden ayrıldı; spike aynı yapılandırmaya yalnızca bir init script ekliyor.
- **Parse karşılaştırması:**
  - Ortak modül: `crates/tokenradar-core/spike/parse_strategies.rs`.
  - Sentetik bench: `benches/parse.rs` (criterion).
  - Gerçek veri: `examples/parse_real.rs`. Modları: matris, `--single`, `--stat`, `--profile`; seçenekleri: `--uncached`, `--background`, `--limit-mb`, `--manifest`.
- **Kararlar:**
  - ADR-023: WebView2 kapısı geçildi, tek süreç modeli korunur. ADR-004'ün durum satırı güncellendi.
  - ADR-024: Parse stratejisi C, `min(4, çekirdek/2)` worker. `simd-json` reddedildi.
  - `performance-budget.md`: Bütçe değerleri değişmedi, ölçüm notları ve yöntem açıklaması eklendi.

### Plandan sapmalar
- **Spike yalnızca debug'da değil, `spike` cargo özelliğiyle release'te de derlenebiliyor.**
  - Debug derleme frontend'i Vite dev sunucusundan yüklediği için soğuk açılış temsilî değil: debug'da 736 ms, release'te 345 ms.
  - Özellik varsayılan olarak kapalı, normal release derlemeye spike kodu girmiyor.
- **ADR numaraları:** Plan "ADR-022 (ayrık UI süreci)" diyordu, ama ADR-022 Adım 01'de kullanılmıştı. Yeni kararlar ADR-023 ve ADR-024 oldu.
- **Soğuk önbellek:** RAMMap veya yeniden başlatma yerine `FILE_FLAG_NO_BUFFERING` kullanıldı. Her okuma diske gider ve yönetici yetkisi gerekmez.
- **Gizlilik (kullanıcı kararı):**
  - Ham parse çıktıları token toplamlarını değil, toplamların xxh3 parmak izini içerir.
  - `--single` modu ham toplamları basar; yalnızca matrisin iç iletişimi içindir, commit edilmez.
- **Ek ölçümler (planda yoktu):**
  - `--disable-gpu` deneyi (n=1).
  - `opt-level = "s"` karşılaştırması (Adım 01 devir notunun isteği).
  - Gizli pencerede `SetIsVisible(false)` kademesi.
- **`clippy.toml`:** `doc-valid-idents` listesine `WebView2` eklendi.
- **Yeni bağımlılıklar:**
  - `tokenradar-core`, yalnızca dev: serde, serde_json, memchr, simd-json, criterion, xxhash-rust, windows-sys.
  - `tokenradar-app`, Windows'ta: `webview2-com 0.39` ve `windows-core 0.62`. Sürümler wry ile aynı, böylece COM tipleri uyuşuyor.
  - `cargo deny check` temiz.

### Açık konular
- **`--disable-gpu`** panel belleğini 90,8 → 60,5 MB düşürdü (n=1). CPU maliyeti, görsel kalite ve yüksek DPI davranışı ölçülmedi. **Adım 10'da** gerçek panelle ölçülüp karar verilmeli.
- **İlk soğuk açılış:** Derleme sonrası ilk başlatmada (disk önbelleği soğuk) 582 ms ölçüldü (n=1). Resmi turlarda 345 ms. Bilgisayar açıldıktan sonraki ilk açılış 400 ms bütçesini aşabilir. Adım 10'da gerçek panel ölçülürken dikkate alınmalı.
- **`opt-level = "s"`** binary'yi %26 küçültüyor (4,11 → 3,04 MiB), I/O'ya bağlı parse'ı pratikte etkilemiyor. SQLite'ın C kodu da bu seviyeyle derleneceği için karar Adım 04 (yazma hızı) veya Adım 12 ölçümüyle verilmeli. `Cargo.toml` profili değiştirilmedi.
- **Soğuk stat taraması** (yeniden başlatma sonrası) ölçülemedi. Sıcak değer 177 ms.

### Sonraki adımlara notlar
- **Adım 04:**
  - Strateji C'yi kullan. Referans kod: `spike/parse_strategies.rs`.
    - `Option<Cow<'a, str>>` serde'de her zaman allocate eder. Oradaki `Str<'a>` newtype'ı ödünç alır.
    - Windows `cwd` değerleri kaçışlı olduğu için `&'a str` çalışmaz.
  - **Uzun satırlar:** En büyük satır 22 MiB (`api-request-blob`), 610 satır 1 MiB'tan uzun. Bunları thread başına bir tamponda biriktirmek 4 worker'da tepe belleği ~165 MiB'a çıkarıyor (bütçe 120 MB). Assistant satırları en fazla 350 KiB. Eşiği aşan satır biriktirilmeden satır sonuna (`\n`) kadar atlanmalı ve tampon büyük satırdan sonra küçültülmeli.
  - **Tekilleştirme eşitliği:** Aynı `(message.id, requestId)` anahtarı birden fazla dosyada eşit `output_tokens` ama farklı `isSidechain` ve metadata ile görünüyor. Token toplamları etkilenmiyor, ama atıf birleştirme sırasına bağlı. Deterministik bir eşitlik kuralı gerekli.
  - **Ön filtre biçime bağlı:** Filtre kompakt `"type":"assistant"` biçimine dayanıyor. Biçim değişirse sessizce sıfır olay üretir. Bunu yakalayan bir test ve uyarı ekle.
  - **Canlı veri:** Claude Code çalışırken dosyalar büyüyor. Ölçümlerde dosya boyutlarını dondur (`--manifest` örneği).
  - Arka plan önceliğinde darboğaz disk: 4 worker ile 12,6 s. Daha fazla worker kazanç getirmez.
- **Adım 05/10:** `warm_ttl` sonrası `destroy()` yeterli, ek temizlik gerekmez. Tauri'nin `hide()` metodu WebView2'ye görünürlük bildirmiyor. Gizli panelde `requestAnimationFrame` çalışmaya devam ediyor; CPU etkisini Adım 10'da `SetIsVisible(false)` ile birlikte değerlendir.
- **Adım 18 (widget):** İkinci pencerenin ek maliyeti +18 MB (bütçe 45 MB). WebView yaklaşımı (ADR-011) bütçe içinde.
- **Ölçüm araçları:**
  - Bellek bütçesi olan her adım `measure-memory.ps1` kullanabilir.
  - Pencere senaryoları için `spike.rs` + `spike-lifecycle.ps1` örnek alınabilir. Gereksiz hâle gelirse Adım 10'da kaldırılabilir.
