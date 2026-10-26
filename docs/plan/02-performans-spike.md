# Adım 02 — Performans spike'ı ve mimari kapı

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1 | 01 | 1 oturum | ⬜ |

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

- [ ] `scripts/measure-memory.ps1 -ProcessName tokenradar-app -IntervalMs 500 -Out docs/perf/raw/<tarih>-lifecycle.csv`:
  - Kök süreci ve tüm alt süreçlerini bulur (`Get-CimInstance Win32_Process` ile `ParentProcessId` üzerinden özyinelemeli).
  - Her örnekte `PrivateMemorySize64`, `WorkingSet64` ve süreç sayısını yazar.
  - Bitişte min/max/son değer özetini basar.
- [ ] Spike senaryosu (debug derleme, `TOKENRADAR_SPIKE=lifecycle`). Her adım zaman damgasıyla loglanır:
  1. 30 s boşta bekle (taban ölçümü).
  2. Paneli **soğuk** oluştur. Oluşturma çağrısından frontend'in `DOMContentLoaded` → Rust'a gönderdiği `ready` olayına kadar geçen süreyi ölç.
  3. 10 s bekle, sonra paneli gizle.
  4. 5 s sonra **sıcak** göster ve ölç. Ardından tekrar gizle.
  5. Pencereyi **yok et** (`close`/`destroy`), 60 s gözle: süreç sayısı ve RAM.
  6. İki pencere birlikte (panel + 220×44 boyutunda widget benzeri bir pencere): ikinci pencerenin **ek** maliyeti.
  7. Gizli ama sıcak pencerede `MemoryUsageTargetLevel = Low` (`ICoreWebView2_19`) dene. `with_webview` ile ham erişim gerekir.
- [ ] Senaryoyu en az 3 kez çalıştır ve medyan değerleri raporla.
- [ ] Süreçler sonlanmıyorsa sırayla şu çareleri dene ve her birini ölç:
  - Tüm webview'leri drop etmek.
  - Ortamı (environment) serbest bırakmak.
  - `additional_browser_args` (örneğin `--renderer-process-limit=1`, gereksiz `msEdge` özelliklerinin kapatılması).
- [ ] Hâlâ ≤ 30 MB'a dönülemiyorsa **yedek plan için prototip** yap: UI'nın `tokenradar-app --ui` olarak ayrı bir süreçte açılması. Bu süreç kapatıldığında toplam RAM'i ölç.

### B. JSONL parse stratejileri

- [ ] Gerçek veriyi **repoya kopyalamadan**, yerinde ve salt-okunur kullan (`%USERPROFILE%\.claude\projects`).
- [ ] Karşılaştırılacak stratejiler (aynı çıktı: `UsageEvent` alanları):
  - **A:** `serde_json::Value` ile tam parse (taban).
  - **B:** Tipli, ödünç alan (`&'a str`) struct + `content` için `IgnoredAny`.
  - **C:** `memchr::memmem` ile `"type":"assistant"` ve `"usage"` ön filtresi + B.
  - **D:** C + `simd-json` (tipli). Mutable buffer gerektirdiği için kopya maliyeti de ölçülmeli.
- [ ] Metrikler:
  - MB/s ve satır/s (tek thread)
  - 4 thread ile toplam süre
  - Tepe RSS
  - Soğuk önbellek ölçümü: mümkünse yeniden başlatma sonrası, değilse RAMMap ile "Empty Standby List"
- [ ] **Stat yürüyüşü:** 5.200 dosyanın `metadata()` taramasının süresi. Bu değer "yeniden başlatma sonrası artımlı tarama ≤ 500 ms" bütçesinin dayanağı.
- [ ] Doğruluk kontrolü: Dört strateji de aynı tekilleştirilmiş olay sayısını ve token toplamlarını vermeli.

### C. Rapor ve kararlar

- [ ] `docs/perf/<tarih>-spike.md`: ölçüm tabloları, yöntem, makine bilgisi ve yorum.
- [ ] `docs/decisions.md`:
  - ADR-004 için sonuç: **Onaylandı** veya **Yerini aldı → ADR-022 (ayrık UI süreci)**.
  - Yeni ADR: seçilen parse stratejisi ve thread sayısı.
- [ ] Gerekirse `docs/performance-budget.md` içindeki indeksleme ve bellek bütçelerini gerçek ölçümlere göre revize et (gerekçesiyle).
- [ ] Yedek plana geçildiyse **etkilenen adımların** (05, 07, 10, 18) görev listelerini güncelle ve devir notuna yaz.

## Teknik notlar ve tuzaklar

- WebView2 süreç ağacı ana sürecin **altında değil**, `msedgewebview2.exe` olarak ayrı bir ağaçta görünebilir. Tarayıcı süreci uygulamanın alt sürecidir, renderer'lar ise tarayıcı sürecinin alt süreçleridir. Script ağacı özyinelemeli izlemeli; ölçümü yalnızca ada göre yapmak, sistemdeki **diğer** WebView2 uygulamalarını da sayar.
- Tauri/wry webview oluştururken bir `WebContext` (kullanıcı veri dizini) kullanır. Aynı veri dizinini paylaşan pencereler aynı tarayıcı sürecini paylaşır. Ölçümde bu durumu not et.
- `PrivateMemorySize64` ile Görev Yöneticisi'ndeki "Bellek" sütunu (private working set) farklıdır. İkisini de raporla, bütçede private working set esas alınır.
- Soğuk açılış süresi `WebView2 environment` oluşturmayı da içerir. Ölçüm, ilk açılış ile sonraki soğuk açılışlar arasında fark gösterebilir; ikisini ayrı raporla.

## Kabul kriterleri

- [ ] Ölçümler 3'er tekrarla alınmış ve medyanlar rapordadır.
- [ ] ADR-004 için net bir karar kaydedilmiştir (onay veya ADR-022).
- [ ] Parse stratejisi seçilmiş ve gerekçesi ADR olarak yazılmıştır. Seçilen stratejinin gerçek veride ölçülen tahmini ilk indeksleme süresi raporda yer alır.
- [ ] `scripts/measure-memory.ps1` başka adımlarda tekrar kullanılabilecek durumdadır (parametreli, yardım metni var).

## Doğrulama

```powershell
$env:TOKENRADAR_SPIKE="lifecycle"; pnpm tauri dev    # ayrı terminalde:
.\scripts\measure-memory.ps1 -ProcessName tokenradar-app -IntervalMs 500 -DurationSec 180
cargo bench -p tokenradar-core --bench parse
cargo run --release -p tokenradar-core --example parse_real -- "$env:USERPROFILE\.claude\projects"
```

## Devir notu

_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
