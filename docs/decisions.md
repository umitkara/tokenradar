# Karar Kayıtları (ADR)

> Her karar şu bölümlerden oluşur: bağlam, karar, bedel ve değerlendirilen alternatifler.
> Bir kararı değiştirmek için eski kaydı silmeyin. Durumunu "Yerini aldı: ADR-0xx" olarak işaretleyip yeni bir kayıt ekleyin.
> İlk 21 karar, 2026-10-26/27 tarihli planlama oturumunda (röportaj + keşif) alınmıştır.

---

### ADR-001 — Açık kaynak, MIT lisansı
**Durum:** Kabul
- **Bağlam:** Windows'ta kaliteli bir limit takip aracı yok. Topluluk benimsemesi ve katkı alımı hedefleniyor.
- **Karar:** Proje GitHub'da açık kaynak olarak yayınlanır, lisansı MIT'dir.
- **Bedel:** Kapalı kaynak fork'lar mümkün. Bu bilinçli olarak kabul edildi.
- **Alternatifler:** MIT OR Apache-2.0 (daha resmî, patent maddesi var); GPL-3.0 (fork'ları açık kalmaya zorlar ama katkıcıları uzaklaştırabilir).

### ADR-002 — Rust çekirdek + Tauri 2 + Svelte 5
**Durum:** Kabul
- **Bağlam:** Hedefler hem yüksek performans ve küçük ayak izi hem de göz alıcı bir arayüz. Bunlar birbiriyle gerilim içinde.
- **Karar:** İş mantığının tamamı Rust ile yazılır (`tokenradar-core`). Arayüz Tauri 2 (sistem WebView'i) ve Svelte 5 ile yapılır. Svelte derlendiği için runtime yükü çok düşük.
- **Bedel:** Pencere açıkken WebView2 süreçleri ~90-130 MB tüketir. Bu bedel ADR-003/004 ile yalnızca pencere açıkken ödenir.
- **Alternatifler:** Slint (en düşük RAM, ama grafik ve estetik cila maliyetli); egui/iced (boşta CPU ve "geliştirici aracı" görünümü); Avalonia/.NET (runtime belleği yüksek).

### ADR-003 — Tray-first yapı + panel + istek üzerine pencereler + isteğe bağlı widget
**Durum:** Kabul
- **Karar:**
  - Tray'de canlı bir ikili halka bulunur (Rust ile bitmap olarak çizilir, WebView gerektirmez).
  - Tıklayınca kompakt bir panel açılır. "Detaylar" ile ana pencereye geçilir.
  - Kompakt şerit biçimindeki mini widget isteğe bağlıdır ve varsayılan olarak kapalıdır.
- **Bedel:** Widget açıkken WebView süreci ayakta kalır (bkz. ADR-011).
- **Alternatif:** Klasik pencere uygulaması. Reddedildi, çünkü arka planda çalışmadan % geçmişi kaydı ve bildirim mümkün değil.

### ADR-004 — Tek uygulama süreci + ayrı CLI binary'si
**Durum:** Kabul. Adım 02 kapısı geçildi (2026-10-27): bkz. ADR-023.
- **Karar:** `tokenradar-app.exe` tray, çekirdek ve pencereleri tek süreçte barındırır. `tokenradar.exe` ise ayrı ve hafif bir konsol CLI'ıdır. Çekirdek Tauri'den bağımsız bir crate olduğu için ileride headless bir daemon'a bölünebilir.
- **Kapı:** Adım 02'de ölçülür: tüm pencereler yok edildikten sonra WebView2 süreçleri sonlanıyor mu ve RAM ≤ 30 MB'a dönüyor mu? **Dönmüyorsa yedek plan** devreye girer: UI, istek üzerine başlatılan ayrı bir süreç olur (`tokenradar-app --ui`) ve çekirdekle IPC üzerinden konuşur.
- **Alternatif:** Baştan ayrık daemon + UI süreci. Daha güçlü bir bellek garantisi verir ama ~%20-30 ek karmaşıklık getirir. Kanıt olmadan bu bedeli ödemiyoruz.

### ADR-005 — Veri kaynağı hiyerarşisi: statusline birincil, API tamamlayıcı
**Durum:** Kabul
- **Bağlam:** Keşif sırasında Claude Code'un statusline JSON'unda `rate_limits.five_hour/seven_day` alanlarının **belgelenmiş ve ondalıklı** olarak gönderildiği görüldü. OAuth usage API daha zengin (scoped limitler, yüzey kırılımı) ama belgelenmemiş ve kimlik bilgisi gerektiriyor.
- **Karar:** S1 (statusline köprüsü) birincil limit kaynağıdır. S2 (OAuth API) açık onayla kullanılan tamamlayıcı kaynaktır. S3 (JSONL) token, oturum ve proje verisinin kaynağıdır. İkisi de yoksa loglardan tahmin yapılır ve açıkça "tahmini" olarak etiketlenir.
- **Bedel:** S1 için kullanıcının statusline'ı kurması gerekir (tek tıklık kurulum, ADR-009). S1 veri gönderdiği sürece S2 nadiren çağrılır.

### ADR-006 — OAuth token salt-okunur kullanılır, asla refresh edilmez
**Durum:** Kabul
- **Karar:**
  - `accessToken` yalnızca onay varsa ve yalnızca istek anında okunur. Diske yazılmaz, loglanmaz, `zeroize` edilir.
  - `refreshToken` hiçbir koşulda okunmaz.
  - 401 alındığında `.credentials.json` mtime'ı değişene kadar beklenir.
- **Gerekçe:** Refresh token'lar rotating. Biz refresh edersek Claude Code'un oturumunu bozabiliriz. Ayrıca açık kaynak bir uygulamada güven her şeydir.

### ADR-007 — Artımlı indeksleme + SQLite + "max output kazanır" tekilleştirmesi
**Durum:** Kabul
- **Bağlam:** Örnek kullanıcıda 30 günde 11,7 GB JSONL birikmiş (günde ~400 MB). Satırların %55-63'ü tekrar ve bazı tekrarlar kısmi `usage` içeriyor.
- **Karar:**
  - Dosya başına byte offset tutulur.
  - `memchr` ile ön filtre uygulanır, ardından seçici deserialize yapılır.
  - Tekilleştirme anahtarı `(message.id, requestId)` çiftinin xxh3-128 hash'idir. UPSERT sırasında `output` değeri büyük olan kayıt kazanır.
  - İlk indeksleme en yeni dosyalardan başlar.
- **Bedel:** Şema ve migration disiplini gerektirir.

### ADR-008 — API karşılığı maliyet ağırlığı + kalibrasyon + etiketli tahminler
**Durum:** Kabul
- **Karar:** Limit tüketimi, API karşılığı maliyetle (C) orantılı modellenir: `Δ% ≈ k·ΔC`. k robust tahmincilerle öğrenilir. Atıf, aralık bazlı orantılı dağıtımla yapılır. Açıklanamayan kısım "Diğer" kovasına gider. Her tahmin "≈" işareti ve güven göstergesiyle sunulur.
- **Doğrulama:** Backtest (pencere başına MAE ≤ 3 puan) ve `seven_day_breakdown` ile çapraz kontrol.
- **Alternatif:** Plan bazlı sabit katsayılar. Reddedildi: Anthropic katsayıları yayınlamıyor ve değiştirebiliyor.

### ADR-009 — Statusline köprüsü zincirleme modunda çalışır; settings.json yalnızca kullanıcı onayıyla değiştirilir
**Durum:** Kabul
- **Karar:**
  - `tokenradar statusline install` komutu `settings.json` dosyasını yedekler ve değişikliği diff olarak gösterir.
  - Kullanıcının önceki `statusLine.command` değeri **korunur** ve `--chain` ile çalıştırılır.
  - `uninstall` önceki durumu geri yükler.
  - Uygulama kapalıyken CLI veriyi spool dosyasına yazar, böylece veri kaybolmaz.

### ADR-010 — Fluent tek kaynaklı i18n (Rust + TS)
**Durum:** Kabul
- **Karar:** `locales/{en,tr}/*.ftl` dosyaları her iki taraf için de tek kaynaktır. `NUM`, `PCT`, `DUR` ve `TR_SUFFIX` özel fonksiyonları iki dilde ortak test vektörleriyle doğrulanır.
- **Gerekçe:** Bildirimler ve tray metni, WebView yokken Rust tarafından üretiliyor. Türkçe ünlü uyumlu ekler, basit bir şablon motoruyla doğru üretilemiyor.
- **Alternatif:** i18next yalnızca TS tarafında çalışır. Rust tarafı için ikinci bir sistem gerekirdi.

### ADR-011 — Widget WebView ile başlar; bütçe aşılırsa native çizime geçilir
**Durum:** Kabul
- **Karar:** v0.3'te widget bir Tauri penceresi (WebView) olarak yapılır ve bellek etkisi ölçülür. Ek maliyet > 45 MB ise widget `tiny-skia` + layered window ile native çizilir (hedef ~3-5 MB).

### ADR-012 — Dağıtım: imzasız başlangıç, ardından SignPath
**Durum:** Kabul
- **Karar:**
  - GitHub Releases üzerinden NSIS (kullanıcı bazlı, admin gerektirmez) installer ve portable zip dağıtılır.
  - winget ve Scoop manifestleri hazırlanır.
  - Otomatik güncelleme Tauri updater ile yapılır (minisign imzalı).
  - Proje olgunlaşınca SignPath Foundation'a (açık kaynak projelere ücretsiz Authenticode) başvurulur.
- **Bedel:** Tarayıcıdan indirilen dosyalarda SmartScreen uyarısı çıkar. winget ve Scoop kurulumlarında Mark-of-the-Web olmadığı için genelde çıkmaz.
- **Alternatifler:** Azure Trusted Signing (ücretli, bölge kısıtlaması var); Microsoft Store (sandbox `~/.claude` ve `\\wsl$` erişimini zorlaştırır).

### ADR-013 — Telemetri yok, ağ trafiği allowlist ile sınırlı
**Durum:** Kabul
- **Karar:** Ağ trafiği yalnızca `api.anthropic.com` (onaylıysa) ve GitHub Releases (güncelleme) hedeflerine gider. Hata raporlama, kullanıcının kendi oluşturup paylaştığı `doctor` paketiyle yapılır.

### ADR-014 — OpenTelemetry alıcısı ertelendi
**Durum:** Ertelendi
- **Bağlam:** Claude Code OTel metrik ve olayları yayınlayabiliyor. Ancak bu, kullanıcının ortam değişkeni ayarlamasını gerektiriyor ve JSONL + statusline zaten ihtiyacı karşılıyor.
- **Yeniden değerlendirme:** Uzak makine ve ekip senaryoları gündeme gelirse.

### ADR-015 — Dil politikası
**Durum:** Kabul
- **Karar:** Kod, kod yorumları, commit mesajları, README ve CONTRIBUTING İngilizce yazılır. Plan dosyaları ve `docs/` altındaki tasarım/karar dokümanları Türkçe yazılır. Public sürüm öncesinde (Adım 12) `architecture.md` dosyasının İngilizce özeti `docs/en/` altına eklenir.

### ADR-016 — İnce MVP ve sürüm sırası
**Durum:** Kabul
- **Karar:** Sürüm sırası şöyle: v0.1 (canlı takip, veri toplama, bildirimler, CLI) → v0.2 (analitik, kalibrasyon, atıf) → v0.3 (widget, çoklu hesap, WSL) → v0.4 (Codex) → v0.5 (macOS/Linux).
- **Gerekçe:** Atıf, birikmiş % geçmişine ihtiyaç duyuyor. Veri toplamaya ne kadar erken başlanırsa atıf o kadar isabetli olur.

### ADR-017 — Binary adlandırma
**Durum:** Kabul
- **Karar:** GUI binary'si `tokenradar-app.exe`, CLI binary'si `tokenradar.exe` olarak adlandırılır. Windows dosya sistemi büyük-küçük harfe duyarsız olduğu için `TokenRadar.exe` ile `tokenradar.exe` çakışırdı. Kullanıcının yazacağı kısa ad CLI'a verildi.

### ADR-018 — WSL'de yalnızca çalışan dağıtımlar taranır
**Durum:** Kabul (v0.3)
- **Karar:** `\\wsl.localhost\…` yoluna erişmek kapalı bir dağıtımı uyandırır. Bu yüzden önce `wsl.exe -l --running -q` ile çalışan dağıtımlar listelenir (çıktı UTF-16), yalnızca onlar taranır. Kapalı dağıtımlar için son bilinen veri gösterilir.

### ADR-019 — Sürekli animasyon yok
**Durum:** Kabul
- **Karar:** Animasyonlar yalnızca veri değiştiğinde ve sınırlı süreyle oynar (≤ 600 ms). Sürekli dönen radar, nabız veya shimmer efektleri kullanılmaz.
- **Gerekçe:** Boşta CPU bütçesi < %0,1. Açık bir pencerede bile sürekli animasyon GPU ve CPU uyandırır.

### ADR-020 — Ham olaylar 90 gün saklanır, 5 dakikalık rollup kalıcıdır
**Durum:** Kabul
- **Karar:** `usage_events` tablosundaki kayıtlar varsayılan olarak 90 gün sonra silinir (ayarlanabilir). `usage_5m` tablosu süresiz tutulur. Analitik ve atıf için 5 dakikalık çözünürlük yeterli. Claude Code'un kendi sildiği dosyaların geçmişi bizde kalır.

### ADR-021 — Platform tabanı
**Durum:** Kabul
- **Karar:** Birincil hedef Windows 11'dir. Windows 10 (1809+, WebView2 mevcut) desteklenir: Mica yerine opak/Acrylic yedek kullanılır. macOS ve Linux v0.5'te eklenir. Tüm kod, platforma özgü kısımlar `cfg` ile ayrılmış şekilde yazılır.

### ADR-022 — Adım 01 araç zinciri sürümleri
**Durum:** Kabul (2026-10-27, Adım 01)
- **Bağlam:** Plan yazıldıktan sonra ekosistemde üç değişiklik çıktı: Vite 8 paketleyici olarak Rolldown'a geçti, TypeScript 7.0 (Go portu) yayımlandı ve rustfmt'nin `imports_granularity` seçeneği hâlâ yalnızca nightly'de çalışıyor. Ayrıca `clippy -D warnings` kullandığımız için her yeni Rust sürümü CI'ı habersizce kırabilir.
- **Karar:**
  - **Rust toolchain sabitlenir:** `rust-toolchain.toml` → `1.99.1` (rustfmt + clippy). Yerel ortam ve CI aynı lint kümesini görür. Yükseltme bilinçli bir commit ile yapılır: `rust-toolchain.toml` ve `rust-version` birlikte güncellenir, yeni clippy uyarıları aynı commit'te giderilir.
  - **Vite 8:** Çok sayfalı girişler `build.rolldownOptions.input` ile tanımlanır. Plandaki `build.rollupOptions` adı yalnızca uyumluluk takma adı olarak yaşıyor.
  - **TypeScript 6.x'e sabitlenir:** `svelte-check` 4.x yalnızca `typescript ^5 || ^6` destekliyor. svelte-check TS 7 desteği yayımladığında yükseltilir.
  - **rustfmt yalnızca stable ayarlarla çalışır:** `rustfmt.toml` içinde yalnızca `max_width = 100` bulunur. Crate bazlı import gruplama bir konvansiyondur, araçla zorlanmaz. `imports_granularity` stable'a geçtiğinde geri eklenir.
- **Bedel:** Rust sürüm yükseltmeleri elle yapılır. TS 7'nin hız kazancından şimdilik yararlanılmıyor. Import biçimi kod incelemesine kalıyor.
- **Alternatifler:** `channel = "stable"` (bakımı daha az, ama CI sürprizleri olabilir); biçimlendirme için tarihli nightly toolchain (ayar zorlanır ama ikinci bir toolchain gerekir). İkisi de kullanıcıyla değerlendirildi ve reddedildi.

### ADR-023 — WebView2 yaşam döngüsü kapısı geçildi: tek süreç modeli korunur
**Durum:** Kabul (2026-10-27, Adım 02)
- **Bağlam:** ADR-004, tek süreç modelini bir ölçüm koşuluna bağlamıştı. Tüm pencereler yok edildiğinde WebView2 süreçleri sonlanmalı ve toplam bellek ≤ 30 MB'a dönmeliydi. Adım 02'de `TOKENRADAR_SPIKE=lifecycle` senaryosu release derlemede 3 kez çalıştırıldı (`docs/perf/2026-10-27-spike.md`).
- **Ölçüm (medyan, private working set, tüm süreç ağacı):**
  - Yalnızca tray: 1,6 MB.
  - Panel açık: 90,8 MB, 7 süreç.
  - Panel yok edildikten 60 s sonra: 3,6 MB, 1 süreç.
  - Tüm `msedgewebview2` süreçleri `destroy()` çağrısından ~0,4 s sonra çıkıyor (9 ölçümün tamamında ≤ 0,5 s).
  - Soğuk açılış: ilk frame'e kadar 345 ms. Sıcak açılış: 17 ms. İkinci pencerenin ek maliyeti: +18 MB.
- **Karar:**
  - ADR-004'teki tek süreç modeli **onaylandı**. Ayrık UI süreci (`tokenradar-app --ui`) yedek planı uygulanmaz ve prototipi yazılmadı.
  - Pencere yaşam döngüsü architecture.md §2'deki gibi kalır: `warm_ttl` süresi dolunca webview `destroy()` edilir. Ek bir çare (ortamı serbest bırakma, `--renderer-process-limit`) gerekmez.
  - Gizli ama sıcak pencerede `ICoreWebView2_19::SetMemoryUsageTargetLevel(Low)` working set'i ~14 MB düşürüyor, private bytes'ı ise değiştirmiyor. `controller.SetIsVisible(false)` tek başına ölçülebilir bir fark yaratmıyor. Kısa `warm_ttl` (60 s) asıl kazancı zaten sağladığı için bu ayarlar zorunlu tutulmaz. Adım 10 isterse Low seviyesini gizlemede kullanabilir.
- **Bedel:** Sıcak tutulan panel `warm_ttl` boyunca ~90 MB kullanır. Bu, tray bütçesinin değil "panel açık" bütçesinin kapsamındadır.
- **Alternatif:** Ayrık UI süreci. Kanıt gereksiz olduğunu gösterdi; ~%20-30 ek karmaşıklıktan kaçınıldı.

### ADR-024 — JSONL parse stratejisi: `memmem` ön filtresi + tipli, ödünç alan `serde_json`
**Durum:** Kabul (2026-10-27, Adım 02)
- **Bağlam:** Adım 02'de dört strateji gerçek veri setinde (5.190 dosya, 11,4 GiB, 1,27 milyon satır) ve sentetik bir criterion bench'inde karşılaştırıldı (`docs/perf/2026-10-27-spike.md`). Dördü de birebir aynı tekilleştirilmiş olay sayısını ve token toplamlarını verdi.
  - A, `serde_json::Value`: 4 thread'de soğuk diskte 6,8 s.
  - B, tipli ve ödünç alan: 4,1 s.
  - C, B + ön filtre: 3,8 s.
  - D, C + `simd-json`: 3,8 s.
  - Önbelleğe sığan 3 GiB alt kümede tek thread hızları: A 661, B 1.786, C 2.537, D 2.713 MiB/s.
- **Karar:**
  - **Strateji C** kullanılır. Satır `memchr::memmem` ile `"type":"assistant"` ve `"usage":` için ön filtrelenir. Geçen satırlar ödünç alan tipli struct'lara `serde_json` ile deserialize edilir. `content` gibi bilinmeyen alanlar serde tarafından materyalize edilmeden atlanır.
  - String alanları `Cow<'a, str>` olarak ödünç alınır. `Option<Cow<'a, str>>` serde'de her zaman allocate ettiği için `Option` içinde ödünç alan bir newtype kullanılır. Windows'ta `cwd` kaçışlı ters bölü içerdiği için `&'a str` kullanılamaz.
  - **Worker sayısı `min(4, çekirdek/2)`** olarak kalır. Arka plan önceliğinde (`THREAD_MODE_BACKGROUND_BEGIN`) disk okuması darboğazdır: 1 thread 16,1 s, 4 thread 12,6 s. Daha fazla thread kazanç getirmez.
  - `simd-json` **reddedildi.** Gerçek veride C ile farkı gürültü düzeyinde. Buna karşılık ek bir bağımlılık, satır başına kopya ve thread başına +8-20 MiB tepe bellek getiriyor.
- **Bedel:** C'nin ön filtresi kompakt JSON biçimine (`"type":"assistant"`, boşluksuz) dayanır. Claude Code biçimi değiştirirse filtre her satırı reddeder. Adım 04'te bunu yakalayan bir test (fixture + "hiç olay yok" uyarısı) yazılmalı.
- **Alternatifler:** B (ön filtresiz, ~%6 daha yavaş ama biçimden bağımsız), D (yukarıda). Ölçülen ilk indeksleme süresi yalnızca parse içindir. SQLite yazma maliyeti Adım 04'te ölçülür.

### ADR-025 — Fiyat tablosu model bazında; cache okuma çarpanı modele göre değişir
**Durum:** Kabul (2026-10-27, Adım 03)
- **Bağlam:** Plan (architecture.md §5, §9.1) fiyatları aile bazında (opus/sonnet/haiku…) tutmayı ve cache okuma çarpanını sabit `0.10` almayı öngörüyordu. Resmî fiyat sayfası (https://platform.claude.com/docs/en/about-claude/pricing, 2026-10-27 tarihinde okundu) farklı bir tablo gösteriyor:
  - **Aynı ailede farklı fiyatlar var.** Opus 5.5 $4/$20, Opus 5 ve 4.x $5/$25, Opus 4/4.1 $15/$75 per MTok. Sonnet 5.x $2/$10, Sonnet 4.x $3/$15.
  - **Cache okuma çarpanı sabit değil.** Fable 5.1 ve Mythos 5.1'de 0,025; Opus 5.5 ve Sonnet 5.5'te 0,05; diğerlerinde 0,1.
  - **Haiku 5.5 kademeli fiyatlanıyor.** İstem 100.000 token'ı aşarsa (girdi + cache okuma + cache yazma) taban fiyatlar $0,10/$0,50'den $0,50/$2,50'ye çıkıyor.
  - Fast mode, sunulduğu üç modelde (Opus 5.5, Opus 5, Opus 4.8) 2× fiyatla geliyor ve cache çarpanları bunun üzerine ekleniyor. Web search arama başına $0,01, web fetch ücretsiz.
- **Karar (kullanıcıyla kararlaştırıldı):**
  - `pricing/claude.toml` **model başına** bir satır içerir: `input`, `output`, isteğe bağlı `cache_read_mult`, `fast_mult` ve `long_context` (eşik + taban fiyatlar). Ortak çarpanlar `[defaults]` bölümündedir.
  - **Model eşleme:** Ad, `id` ile birebir aynıysa veya `id`'nin sonunda tarih (`-20251001`), sürüm (`-v1:0`), `@` ya da `[` eki varsa eşleşir. `us.anthropic.` gibi platform önekleri atılır. Düz önek eşleşmesi kabul edilmez: `claude-opus-5`, `claude-opus-5-5` ile eşleşmez.
  - **Bilinmeyen model:** Ailesi desenle bulunur (`family_of`). Aile biliniyorsa o ailenin tablodaki en yeni modelinin fiyatı, bilinmiyorsa `fallback` modelinin (`claude-opus-5-5`) fiyatı kullanılır. `Priced::model_known = false` döner ve model başına bir kez uyarı loglanır.
  - **Aile yalnızca gruplama ve kalibrasyondaki aile katsayısı (`α_f`) için tutulur.** Fiyat hesabında kullanılmaz.
  - **Web search maliyete dahildir.** §9.1 formülüne `+ web_search × $0,01` terimi eklendi.
  - `inference_geo: "us"` 1,1× çarpanı **uygulanmaz** (bkz. "Bedel").
- **Bedel:**
  - Yeni bir model çıktığında tablo satırı elle eklenir. O zamana kadar model aile yedeğiyle fiyatlanır ve `model_known = false` olarak işaretlenir.
  - `inference_geo` alanı JSONL'de bulunuyor, ama varsayılan değer `global`. US-only kullanan bir kullanıcıda maliyet %10 düşük çıkar. Kalibrasyon (k) oransal olduğu için limit tahminini etkilemez, yalnızca "API karşılığı $" gösterimini etkiler. Gerekirse Adım 04'te alan okunup çarpan eklenebilir.
- **Alternatifler:** Plandaki aile bazlı tablo. Reddedildi: aynı ailede %25-200 fiyat farkı var.

### ADR-026 — İndeksleyici kuralları: advisor olayları, sıra bağımsız eşitlik, başlık önceliği ve hibrit öncelik
**Durum:** Kabul (2026-10-27, Adım 04)
- **Bağlam:** Adım 04'ün gerçek veri keşfi, plandaki varsayımlardan dört noktada ayrıldı (`data-sources.md` S3, tuzak 2, 9, 11, 12):
  - `iterations` dizisinde `advisor_message` elemanları var. Bunlar başka bir modelin tüketimini taşıyor ve üst düzey `usage` toplamına dahil değil.
  - Aynı `msg_key` birden fazla oturum dosyasında, çoğunlukla eşit `output_tokens` ile görülüyor (resume, fork). "Eşitlikte en son görülen kazanır" kuralı, çok worker'lı indekslemede olayın hangi oturuma yazılacağını işlem sırasına bağlı hale getiriyor.
  - Başlık için üç ayrı meta kayıt var: `ai-title`, `agent-name`, `custom-title`.
  - Tamamen arka plan önceliğinde `RecentReady` sıcak önbellekte bile 7-9 s sürdü (bütçe 5 s).
- **Karar (advisor ve öncelik kullanıcıyla kararlaştırıldı):**
  - **Advisor elemanları ayrı olaydır.** Her `advisor_message` elemanı kendi modeliyle bir `UsageEvent` olur. Anahtar `MsgKey::from_ids_tagged(message.id, requestId, "advisor#<i>")`. Üst düzey `usage` olduğu gibi kalır.
  - **Eşitlik kuralı sıra bağımsızdır.** `output_tokens` eşitse:
    - aynı oturumda son satır kazanır (streaming tekrarı);
    - farklı oturumlarda ana oturum subagent'ı yener, sonra küçük `sessions.uuid` kazanır.
    Bu tam bir sıralama olduğu için sonuç worker sayısından ve dosya sırasından bağımsızdır (1/2/4 worker aynı parmak izini veriyor).
  - **Başlık önceliği:** `custom` (3) > `agent` (2) > `ai` (1). Aynı kaynakta son gelen kazanır. Kaynak `sessions.title_source` sütununda tutulur, böylece artımlı turlarda düşük öncelikli bir başlık yüksek öncelikliyi ezmez.
  - **Hibrit öncelik:** Worker'lar mtime'ı son 7 gün içindeki dosyaları normal öncelikte işler. İlk eski dosyaya geçtiklerinde `THREAD_MODE_BACKGROUND_BEGIN` moduna girerler. İşler en yeniden en eskiye sıralı olduğu için geri dönüş gerekmez.
  - **Uzun satırlar:** 4 MiB'ı aşan satırlar belleğe alınmadan atlanır ve sayılır (`oversized_lines`). Bu, Adım 02'de görülen ~165 MiB tepe bellek riskini kapatır.
  - **Worker tarafında tekilleştirme:** Worker'lar her parça (2.000 olay) içinde aynı anahtarı birleştirir. Yazıcıya giden güncelleme sayısı gerçek veride ~%99,8 azaldı.
- **Bedel:**
  - Advisor olayları DB'de ayrı bir işaret taşımaz; yalnızca anahtar türetmesinden ayırt edilir. UI'da "advisor" etiketi gerekirse şema değişir.
  - Hibrit öncelik, ilk birkaç saniye boyunca indeksleyicinin kullanıcının işiyle normal öncelikte yarışması demektir. Ölçülen CPU süresi değişmedi (~7-9 s); yalnızca zamanlama değişti.
  - Farklı oturumlar arasındaki eşitlikte kazanan sahip "en mantıklı" değil, yalnızca deterministik olandır.
  - 4 MiB'tan uzun bir `assistant` satırı çıkarsa tüketimi kaybolur. Bu durumda `oversized_suspect` sayacı ve uyarı devreye girer.
- **Alternatifler:**
  - Advisor tüketimini yok saymak (eksik sayım) veya üst düzey toplama eklemek (model yanlış atanır).
  - Tamamen arka plan önceliği (bütçeyi aşıyor) veya tamamen normal öncelik (eski veri için kullanıcıyla gereksiz yarış).
  - Eşitlikte "ilk yazılan kalır" (yine sıraya bağımlı).

### ADR-027 — Çekirdek runtime'ı: std thread'ler + `tokio::sync`; taşmaya karşı sıcak dosya taraması
**Durum:** Kabul (2026-10-27, Adım 05)
- **Bağlam:**
  - Plan, `CoreRuntime` için bir Tokio runtime'ı (gerekirse Tauri'ninki) öngörüyordu. Oysa Adım 03-04'te yazılan store yazıcısı, indeksleyici worker'ları ve retention zamanlayıcısı tamamen bloklayan, `std::thread` tabanlı kod.
  - Plan, `ReadDirectoryChangesW` buffer'ı taşınca `notify`'ın bir "rescan" olayı üreteceğini varsayıyordu. `notify` 8.2.1'in Windows backend'i (`windows.rs`, `handle_event`) ise tamamlanan isteğin bayt sayısına hiç bakmıyor. Taşmada Windows "başarılı, 0 bayt" döner; `notify` sıfırlı buffer'ı `Action = 0` olarak okur ve sessizce atar. Yani **Windows'ta taşma bildirilmez.** Buffer 16 KiB'tır; tipik bir transcript yoluyla bu ~65 olay eder.
- **Karar (kullanıcıyla kararlaştırıldı):**
  - **Runtime:** Çekirdek bir async runtime'a bağlı değildir. Her görev adlandırılmış bir `std::thread`'dir: `core-watch`, hesap başına `core-index-<id>`, `retention` ve `store-writer`. `EventBus` yalnızca `tokio`'nun `sync` özelliğini kullanır (`broadcast`, runtime gerektirmez). Yayınlama senkron yapılır; Tauri tarafı `recv().await` ile, düz thread'ler `blocking_recv()` ile alır. `CoreHandle::shutdown(timeout)` her thread'e durma sinyali verir ve iptal bayrağını kaldırır: indeksleyici o anki dosyayı bitirip durur, bu sırada transaction tutarlılığı korunur. Ardından thread'leri join eder ve zamanında bitmeyenleri raporlar.
  - **Taşma:** 10 dakikalık güvenlik taramasına ek olarak bir **sıcak dosya kümesi** tutulur. Son 15 dakikada değişen transcript'ler, küme boş değilken 30 s'de bir `stat` edilir. Boyutu veya mtime'ı olay gelmeden değişen dosya indeksleyiciye gönderilir. Küme boşken zamanlayıcı yoktur; boşta CPU bütçesi etkilenmez. Diğer platformlarda `notify`'ın `need_rescan()` işareti ve hata olayları tam taramayı tetikler.
  - **Debounce penceresi sabittir:** İlk olay 750 ms'lik bir pencere açar ve olay geldikçe uzamaz. Claude Code akış sırasında saniyede birkaç kez yazdığı için kayan bir pencere hiç kapanmazdı; sabit pencerede gecikme ≤ 750 ms + indeksleme süresiyle sınırlı kalır.
- **Bedel:**
  - Kaçırılan bir olay, aktif dosyalar için en fazla ~30 s gecikir; 15 dakikadan uzun süre sessiz kalmış bir dosya için en fazla 10 dakika.
  - Thread sayısı runtime task'larından fazladır: hesap başına bir thread artı sabit dört thread. Hepsi zamanlayıcı veya kanal üzerinde bekler, boşta uyanmaz.
  - `notify`'ın taşma davranışı ileride düzeltilirse sıcak dosya taraması gereksiz hale gelir; o zaman kaldırılabilir.
- **Alternatifler:**
  - Tokio runtime + `spawn_blocking` (bloklayan kodu sarmalamak gerekir, kapanışta blocking pool'u izlemek zorlaşır).
  - Yalnızca 10 dakikalık güvenlik ağı (aktif bir dosyanın son yazması 10 dakika gecikebilir).
  - Kendi `ReadDirectoryChangesW` sarmalayıcımızı yazmak (taşmayı doğru yakalar, ama ~300 satır unsafe FFI ve bakım yükü getirir).

### ADR-028 — Limit motoru ve kullanım API'si: yerel runtime'lı `reqwest`, geri çekilen yoklama, heartbeat'li kalıcılık
**Durum:** Kabul (2026-10-27, Adım 06)
- **Bağlam:**
  - ADR-027 çekirdeği düz thread'lere bağladı. OAuth kullanım API'si (S2) ise bir HTTP istemcisi gerektiriyor ve kapanış bütçesi (2 s), 10 s'lik bir istek zaman aşımıyla çakışıyor.
  - Plan, ağ değişimini işletim sisteminin bağlantı olaylarıyla (NLM) izlemeyi öneriyordu. Bu, Windows'a özgü COM kodu ve bir thread daha demek.
  - Plandaki "60 s / 5 dk" kalıcılık ifadesi iki biçimde okunabiliyordu: her gözlemi 60 s'de bir mi, değişmeyen değeri 5 dakikada bir mi yazmak.
  - API'nin `resets_at` değeri her yanıtta mikrosaniye düzeyinde kayıyor; statusline ise saniye çözünürlüklü epoch veriyor. Aynı pencere iki farklı zaman damgasıyla geliyor.
- **Karar (HTTP ve çevrimdışı davranış kullanıcıyla kararlaştırıldı):**
  - **HTTP:** `reqwest` 0.13 (`rustls`, varsayılan özellikler kapalı). Hesap başına `core-api-<id>` thread'i, ilk istekte kendi `current_thread` Tokio runtime'ını kurar ve isteği `block_on` ile bekler. Gizli bir runtime thread'i yoktur; onay yoksa ne runtime ne istemci oluşur. Uçuştaki istek 100 ms dilimlerle iptal bayrağını yoklar, bu yüzden kapanış 10 s beklemez. Aynı yığın ileride güncelleyici için de kullanılır.
  - **Çevrimdışı:** İşletim sistemi olayı dinlenmez. Ağ hatası `Offline` durumuna geçer ve 60 s'den 30 dakikaya kadar katlanarak (±%10 jitter) yeniden denenir. Panelin açılması veya `.credentials.json`'un değişmesi beklemeyi hemen bitirir. 429/5xx'te `Retry-After` (en fazla 6 saat) uygulanır. 401/403'te istek atılmaz; mtime değişene kadar `TokenStale` durumunda kalınır (ADR-006).
  - **Kalıcılık (heartbeat):** Tahmin dışındaki her gözlem `limit_snapshots`'a yazılır. Tek istisna: aynı kaynaktan, aynı pencerede ve aynı yüzde (±0,005) ise 5 dakika içinde tekrar yazılmaz. Böylece saniyede bir gelen statusline çağrıları tabloyu şişirmez, ama zaman serisi kopmaz. Tahminler (`Estimate`) hiç yazılmaz.
  - **Pencere eşleşmesi:** `limit_windows` satırı `(hesap, tür, kapsam)` ve ±5 dakika toleranslı `resets_at` ile bulunur. SQLite `UNIQUE` içinde NULL kapsamları farklı saydığı için `ON CONFLICT` yerine önce arama yapılır.
  - **Görünüm modeli:** `PanelSnapshot` tek bir birincil hesabı taşır; plandakine ek olarak penceredeki en pahalı üç oturum `top_sessions` olarak gelir. `eta_100_at` süre değil mutlak zaman damgasıdır (UI saati yerelleştirir, geri sayımı kendisi yapmaz). `tokens_per_min` girdi, çıktı ve cache yazmasını sayar; `cache_read` sayılmaz, çünkü kotaya etkisi çok küçüktür ve oranı anlamsızca şişirir. `LimitsUpdated` yalnızca kullanıcının görebileceği bir alan (yuvarlanmış yüzde, önem, tazelik, kaynak…) değişince yayınlanır.
  - **Plan bilgisi:** `subscriptionType` ve `rateLimitTier` onaysız okunur. Bunu okuyan yapı token alanlarını hiç tanımlamaz, dolayısıyla token belleğe deserialize edilmez.
- **Bedel:**
  - Her API thread'i istek sırasında küçük bir runtime taşır (~birkaç yüz KiB). Onay yoksa sıfırdır.
  - Ağ geri geldiğinde veri en fazla 30 dakika gecikebilir (panel açılırsa hemen yenilenir).
  - `rustls` varsayılan olarak `aws-lc-rs` kullanır: derleme süresi ve ikili boyutu artar. Ölçüm Adım 13'teki boyut bütçesinde yapılır.
  - `reqwest`'in `HeaderValue`'su içindeki token baytları sıfırlanamaz; değer `set_sensitive(true)` ile işaretlenir ve istekle birlikte serbest bırakılır.
- **Alternatifler:**
  - `ureq` gibi senkron bir istemci (ikinci bir TLS/HTTP yığını getirir, güncelleyiciyle paylaşılamaz).
  - Tauri'nin async runtime'ı (çekirdeği Tauri'ye bağlar; CLI'da yok).
  - NLM / `NetworkInformation` olayları (platforma özgü, test edilmesi zor).
  - Her gözlemi yazmak (statusline ile günde ~86.000 satır) veya 60 s'de bir örneklemek (eşik geçişini geç kaydeder).

### ADR-029 — IPC ve statusline köprüsü: tokio named pipe, cerrahi `settings.json` düzenlemesi, Windows kabuk tırnaklaması
**Durum:** Kabul (2026-10-27, Adım 07)
- **Bağlam:**
  - Plan IPC için `interprocess` kütüphanesini öngörüyordu. Pipe'ın açık bir DACL ile ve `PIPE_REJECT_REMOTE_CLIENTS`, `FILE_FLAG_FIRST_PIPE_INSTANCE` bayraklarıyla oluşturulması gerekiyor. `tokio` zaten bağımlılık (ADR-027/028) ve `ServerOptions::create_with_security_attributes_raw` bu üçünü doğrudan destekliyor.
  - Plan, `settings.json`'u `serde_json` `preserve_order` ile yeniden yazmayı öngörüyordu. Bu yöntem anahtar sırasını korur ama girintiyi, satır sonlarını, sayı biçimlerini ve kaçışları normalize eder. Bu yüzden `install → uninstall` döngüsünün bayt düzeyinde aynı sonucu vermesi garanti edilemez.
  - Fluent çekirdeği (Localizer, `NUM`/`PCT`/`DUR`) Adım 08'e bırakılmıştı. Ancak CLI'ın kendi metinleri bu adımda yazılmak zorunda.
  - Claude Code'un Windows'ta statusline komutunu hangi kabukla çalıştırdığı belirsizdi. Resmî belgelere göre Git Bash kuruluysa onu, değilse PowerShell'i kullanıyor. Git Bash tırnaksız ters bölüleri siliyor (`python C:\Users\…\statusline.py` → `python C:Users…statusline.py`).
- **Karar (IPC, settings ve i18n kapsamı kullanıcıyla kararlaştırıldı):**
  - **IPC sunucusu:** `tokio::net::windows::named_pipe`, kendi thread'inde (`core-ipc`) `current_thread` runtime üzerinde çalışır. Her pipe örneği SDDL `D:P(A;;GA;;;<kullanıcı SID>)` ile oluşturulur, uzak istemciler reddedilir, ilk örnek `first_pipe_instance` ile açılır. Böylece adı önceden kapmış bir süreç sunucuyu taklit edemez; sunucu başlamaz ve uyarı loglar. Sınırlar: en fazla 16 eşzamanlı istemci, 2 s okuma zaman aşımı, 1 MB mesaj. Bozuk satır yalnızca kendi bağlantısını kapatır. Bilinmeyen `op` ve yeni `v` değeri hata yanıtı alır, bağlantı açık kalır.
  - **IPC istemcisi (CLI):** Runtime kullanmaz. Pipe `std::fs::OpenOptions` ile ve `SECURITY_IDENTIFICATION` QoS ile açılır; böylece sahte bir sunucu CLI kullanıcısını taklit edemez. Meşgulse `WaitNamedPipeW` ile beklenir. Zaman aşımları yardımcı bir thread ile uygulanır. İstekler tek yazmada gönderilir; push her zaman ilk iştir.
  - **Statusline satırı:** Yüzdeler ve reset süreleri **stdin'deki payload'dan** gelir, çünkü o an alınmış en taze ölçümdür. Uygulamanın snapshot'ı push'u henüz işlememiş olabilir. Uygulama yalnızca projeksiyonu (`▲`) ve oturum payını (`≈`) ekler. Payload'da olmayan pencere uygulamanın değerine düşer. Reseti geçmiş pencere, motorla aynı kuralla tahmini %0 olarak gösterilir.
  - **Spool:** `spool/statusline-YYYYMMDD.jsonl` dosyasının günü **UTC**'dir. Her satır `{observed_at, payload}` taşır ve yalnızca seçili alanlar yazılır, ham JSON yazılmaz. Yazma dosya kilidi altında tek `write` çağrısıyla yapılır; günlük üst sınır 5 MiB, saklama süresi 14 gündür. İçe aktarma, dosyayı `.importing` adıyla sahiplenir ve her gözlemi `snapshot_exists` ile kontrol eder; aynı okuma iki kez saklanmaz. Dosya ancak bütün satırlar işlenince silinir. İçe aktarılan gözlemler de motorun heartbeat kuralından (ADR-028) geçer.
  - **`settings.json` (cerrahi düzenleme):** Belge önce `serde_json` ile doğrulanır. Ardından yalnızca üst düzey `statusLine` üyesinin bayt aralığı değiştirilir; geri kalan baytlara dokunulmaz. Yeni üye eklenirken girinti ve satır sonu (LF/CRLF) dosyanın ilk üyesinden alınır. Önceki ham değer `config.toml`'a **olduğu gibi** kaydedilir; `uninstall` onu aynen geri yazar. Değişiklik unified diff olarak gösterilir. Onay alınır (`--yes` yoksa ve terminal değilse iptal edilir). Yedek `settings.json.tokenradar-<UTC ts>.bak` adıyla alınır. Yazma atomiktir: geçici dosya + rename. Diff gösterildikten sonra dosya değişmişse hiçbir şey yazılmaz.
  - **Komut biçimi:** Exe yolu düz bölülerle ve tırnaksız yazılır (`D:/…/tokenradar.exe statusline`). Hem Git Bash hem PowerShell bunu aynen çalıştırır. Yolda boşluk varsa 8.3 kısa yol (`GetShortPathNameW`) denenir. O da yoksa kabuğa göre tırnaklanır: Git Bash için `"…"`, PowerShell için `& '…'`. Kurulum, zincirlenecek önceki komutun Git Bash'te bozulacağını fark ederse uyarır, ama komutu kendiliğinden değiştirmez.
  - **Zincirleme:** Önceki bir komut varsa varsayılan olarak zincirlenir (`--no-chain` ile kapatılır). Önceki komut, algılanan kabukla (`bash -c` veya `powershell -NoProfile -NonInteractive -Command`) aynı stdin ile 300 ms içinde çalıştırılır. Süre aşılırsa süreç öldürülür ve yalnızca bizim satırımız basılır. Varsayılan konum `before`: onun çıktısı üstte, bizim satırımız altta.
  - **i18n:** `tokenradar-i18n` bu adımda yazıldı. `Localizer` gelen değerleri izole etmez (`set_use_isolating(false)`); `NUM`/`PCT`/`DUR` fonksiyonları kayıtlıdır; dil `config.toml`'daki `general.locale`'den, yoksa sistemden gelir. Metinler `locales/{en,tr}/cli.ftl` dosyalarındadır ve iki dilin anahtar ve değişken eşliği testle denetlenir. TR sonek kuralları, `i18n-vectors.json` ve tray metinleri Adım 08'de kalır.
- **Bedel:**
  - Unix soketi yok; IPC şimdilik yalnızca Windows'ta (Adım 21). Diğer platformlarda CLI her zaman spool'a yazar.
  - Cerrahi düzenleyici yalnızca üst düzey bir anahtarı bilir. Başka bir ayarı değiştirmek gerekirse aynı kod genişletilmelidir.
  - CLI, çekirdeğin tamamına bağlı (SQLite dahil, salt-okunur rapor için). Release boyutu **2,95 MiB** ve 3 MB bütçesine yalnızca ~48 KiB pay kalıyor. Aynı derleme `opt-level = "s"` ile 2,05 MiB'a iniyor. Bu, profil kararı olduğu için açık bırakıldı (bkz. Adım 07 devir notu).
  - Uygulamanın IPC'si başka bir süreç tarafından tutuluyorsa, uygulama köprüsüz çalışır ve CLI spool'a yazar. Durum yalnızca log'da görünür.
- **Alternatifler:**
  - `interprocess` (DACL ve ilk-örnek bayrağı için yine ham `windows-sys` gerekirdi; ikinci bir soyutlama katmanı ekler).
  - `serde_json` `preserve_order` ile yeniden yazmak (biçimi normalize eder, bayt eşitliği sağlanmaz).
  - Kabuk bağımsızlığı için her zaman `cmd /c` veya bir `.cmd` sarmalayıcı (Claude Code komutu kendi kabuğunda çalıştırır; sarmalayıcı ek süreç ve gecikme getirir).
  - Satırı uygulamanın snapshot'ından basmak (bir önceki yanıtın değerini gösterebilir).

### ADR-030 — Tray: gizli pencereyle ortam izleme, halka başına önem rengi, çok satırlı tooltip
**Durum:** Kabul (2026-10-27, Adım 08)
- **Bağlam:**
  - Plan tema değişikliğini `RegNotifyChangeKeyValue` ile ayrı bir thread'de beklemeyi öngörüyordu. Ancak ikonun yeniden çizilmesini gerektiren beş olay daha var: DPI/ölçek değişimi, monitör değişimi, yüksek kontrast açılıp kapanması, sistem renk değişimi ve görev çubuğunun yeniden oluşması. Registry beklemesi bunların yalnızca birini görür.
  - Plan "Paused" durumunu sayıyor ama nasıl görüneceğini tanımlamıyordu. Kritik çentik yalnızca iç halka için tanımlıydı; haftalık limit kritikken dış halka diğer durumlardan ayırt edilemiyordu.
  - `design-language.md`'deki tooltip örneği tek satırdı. Sonekli tam TR cümleleri (`Çar 10:59'da yenilenir`) 127 UTF-16 birimine her durumda sığmıyor.
  - Model kapsamlı haftalık limitler (`WeeklyScoped`, ör. "Fable · haftalık") ikonda yer almıyordu; kullanıcı bunlara yaklaştığında tray hiçbir şey söylemiyordu.
  - Track token'ı koyu temada `/0.5` alfa taşıyor. 16 px'te bu track görev çubuğunun üzerinde neredeyse kayboluyor ve kritik çentiği gizliyordu.
- **Karar (kullanıcıyla kararlaştırıldı):**
  - **Ortam izleme:** Kendi thread'inde (`tray-env`) `GetMessageW` ile uyuyan tek bir gizli üst düzey pencere (`WS_POPUP`, `WS_EX_TOOLWINDOW`). `WM_SETTINGCHANGE`, `WM_DISPLAYCHANGE`, `WM_DPICHANGED`, `WM_THEMECHANGED`, `WM_SYSCOLORCHANGE` ve `TaskbarCreated` mesajlarında ortam (ikon boyutu + tema) yeniden okunur. Değer aynıysa hiçbir şey çizilmez. Polling yok.
  - **Boyut:** Görev çubuğu penceresinin DPI'ı (`GetDpiForWindow(Shell_TrayWnd)`, yoksa `GetDpiForSystem`) ile `GetSystemMetricsForDpi(SM_CXSMICON)`; 16–64 aralığına sıkıştırılır.
  - **Görev çubuğunun yeniden oluşması:** `tray-icon` 0.25 `TaskbarCreated`'ı zaten ele alıyor (kayıtlı ikon ve tooltip'i yeniden ekliyor); ek kod gerekmedi. Eski HICON `DestroyIcon` ile bırakılıyor.
  - **Önem rengi:** Her halka kendi önemine göre renklenir (`Severity::of`, 75/90). Çentik, kritik olan **her** halkanın ucuna konur; halka doluysa (≥ %100 − çentik) 12 yönüne.
  - **Susturulmuş:** Sağ altta iki dikey çubuktan oluşan duraklat işareti, saydam bir halo ile halkalardan ayrılır. Hata noktası sağ üstte, kritik renkte. Bayat veride tüm ikon %50 opaklık; işaretler opaklığı düşürüldükten sonra çizilir, tam opak kalır.
  - **Track:** Tray'de opaktır (koyu `rgb(68 72 77)`, açık `rgb(201 206 212)`). Yüksek kontrastta mürekkep `GetSysColor(COLOR_WINDOWTEXT)`'tir, track 1 px ve tam piksele hizalıdır.
  - **Tooltip:** Çok satırlı, tam cümle: başlık, 5s satırı (dakikaya yukarı yuvarlanmış geri sayım), haftalık satırı (yerel gün + saat, `TR_SUFFIX`), gerekirse kapsamlı limit satırları, bayat/hata/susturma satırı. Sığmazsa sırayla başlıksız biçime, sonra kısa biçime düşer; o da sığmazsa satır sınırında kesilir. Yüzdeler tam puana yuvarlanır, böylece tooltip ikonla aynı sıklıkta değişir. Tahminler `≈` taşır.
  - **Kapsamlı haftalık limitler:** İkona girmez. `WARN_AT` (%75) ve üstündeyse tooltip'e kendi satırı eklenir (yüksekten düşüğe).
  - **`TR_SUFFIX`:** Sayının yanında biçimlenmiş metni de kabul eder (`%23,5`, `10:59`, `Çar 10:59`): son okunan kelimeye göre çekimlenir; `:00` saate, diğer dakikalar dakikaya bakar. Planlanan beş hale `poss` (iyelik, `%72'si`) eklendi.
  - **Saat biçimi:** 12/24 saat tercihi `GetLocaleInfoEx(LOCALE_STIMEFORMAT)`'tan okunur; TR her zaman 24 saattir.
  - **Susturma:** `kv` anahtarı `notify_snoozed_until`, değeri Unix milisaniye metni. Süre dolunca menü etiketi ve ikon kendiliğinden geri döner; tray thread'i bir sonraki dakika sınırında veya susturma bitiminde uyanır.
  - **Sahte mod:** `TOKENRADAR_FAKE_LIMITS=cycle|sweep|stress` (yalnızca debug veya `spike` özelliği) çekirdeği **hiç başlatmaz**: veritabanı, indeksleme ve API yoktur. `TOKENRADAR_FAKE_LOG` release ölçümlerinde log dosyası verir.
- **Bedel:**
  - Bir thread ve bir gizli pencere daha (uyurken CPU sıfır, bellek birkaç KiB).
  - Tray track'i panel track'inden (yarı saydam) farklıdır; tasarım dili bunu ayrıca belirtir.
  - Yüksek kontrast mürekkebinin görev çubuğunun gerçek rengiyle uyumu gerçek bir HC temasında denenmedi.
  - Kapsamlı limitler yalnızca uyarı eşiğinden sonra görünür; altında tray onlar hakkında bir şey söylemez.
- **Alternatifler:**
  - `RegNotifyChangeKeyValue` (yalnızca tema; DPI ve HC için ikinci bir mekanizma gerekirdi).
  - Tauri pencere olaylarını dinlemek (WebView penceresi her zaman açık değil; ADR-003).
  - Duraklatılmışken ikonu soluklaştırmak (bayat durumla karışır) veya halkaları griye çevirmek (değerler okunmaz olur).
  - Yalnızca iç halkada çentik (haftalık kritik durum görünmez kalır).
  - Tek satırlı kısa tooltip (Windows çok satırı destekliyor; tam cümle ekran okuyucuda daha iyi okunuyor).

### ADR-031 — Frontend temeli: TS biçimleme portu, `light-dark()` token'ları, açılış betiği, tek komut yönlendiricisi
**Durum:** Kabul (2026-10-27, Adım 09)
- **Bağlam:**
  - Plan TS tarafında biçimlemenin `Intl` ile yapılmasını, çıktıların ortak vektörlerle karşılaştırılmasını öngörüyordu. Node 22.14 (ICU 76) üzerinde ölçüldü: `Intl.NumberFormat('tr-TR', {style: 'percent'})` beklendiği gibi `%72` veriyor. Ancak `-0.04` tek ondalıkla `-0` oluyor (vektör `0,0` bekliyor); Rust `{:.N}` tam yarımları çifte yuvarlarken (`0.125` → `0.12`) `Intl`/`toFixed` yukarı yuvarlıyor. ICU sürümü WebView2 ile Node arasında da farklı olabilir. (Soru sırasında `en-US` saatlerinde U+202F dar boşluk olduğunu söylemiştim; Node 22/ICU 76'da bu **doğrulanmadı**, normal boşluk çıktı.)
  - Ekran okuyucu cümleleri (`aria-*`) `1 sa 48 dk` kısaltmasını iyi okumuyor.
  - Sayfa ilk çizildiğinde tema ve dil yanlışsa bir kare yanlış görünür; config.toml okunması ise ana thread'i bekletmemeli.
  - tauri-specta, Tauri'nin yalnızca **son** `invoke_handler`'ı tuttuğunu bilmiyor: debug derlemede her zaman kayıtlı olan Adım 02 `spike_ready` komutu tipli komutları sessizce eziyordu (`Command get_settings not found`).
- **Karar (kullanıcıyla kararlaştırıldı):**
  - **Biçimleme:** `NUM`/`PCT`/`DUR`/`TR_SUFFIX` Rust'ın elle yazılmış TS portudur (`app/src/lib/i18n/format.ts`, `suffix.ts`); `Intl` kullanılmaz. `NUM` tam yarımları çifte yuvarlar (`toFixed(100)` ile tam açılım), `PCT` sıfırdan uzağa yuvarlar. İki taraf da `testdata/i18n-vectors.json`'dan geçer; eşitlik vakaları vektörlere eklendi.
  - **`DUR(ms, "long")`:** Ekran okuyucular için kelimeli biçim (`1 saat 48 dakika`, `1 hour 48 minutes`); Rust ve TS'te aynı, vektörlerle test edilir. Halka merkezi için kısa `resets-in-short` mesajı eklendi.
  - **Token'lar:** Her renk CSS `light-dark()` çiftidir; `data-theme` → `color-scheme`. Kontrast `tokens.test.ts`'te OKLCH → sRGB ve zemin aralığı üzerine bindirme ile test edilir; Win10 opak renkleri Rust `chrome.rs` ile eşitliği de test edilir. Yeni token `--on-accent`.
  - **Zemin:** Windows 11 (build ≥ 22000) şeffaf + Mica; panel yalnızca build ≥ 22523'te (sistem Acrylic'i) Acrylic, aksi halde Mica. Windows 10 opak `background_color`. Build numarası registry'den (`CurrentBuildNumber`).
  - **Açılış betiği:** Her pencere `window.__TOKENRADAR_BOOT__ = {SettingsView}` ile açılır (initialization script); ilk kare doğru dil/temadadır. Ayarlar config.toml'dan ayrı bir thread'de okunur, değişirse `settings-changed` olayı gider.
  - **`set_locale`/`set_theme`:** config.toml `[general]` altına `toml_edit` ile yazılır (yorumlar ve diğer anahtarlar korunur), sonra tüm pencerelere ve tray'e yayılır.
  - **Bağlamalar:** `disable_serde_phases()` (tek tip, `_Serialize`/`_Deserialize` ikizi yok) ve `dangerously_cast_bigints_to_number()` (id, zaman damgası ve para 2^53'ün çok altında). `f64` alanları `number | null` olarak üretilir (specta-typescript'te sabit); UI `null`'u "veri yok" sayar. `specta-typescript` 0.0.13'e sabitlendi (0.0.14 specta rc.27 istiyor). Tazelik `bindings_are_fresh` testiyle CI'da kontrol edilir; dışa aktarma yalnızca debug'dadır.
  - **Komut yönlendirici:** Spike komutu ve tipli komutlar tek bir `invoke_handler` içinde komut adına göre yönlendirilir (`spike::register`).
  - **Olaylar:** `CoreEvent` yalnızca görünür pencerelere gider; görünür pencere yoksa serileştirme yapılmaz. Sayfa gizlenince dinleyiciyi bırakır, görünür olunca tam snapshot çeker.
  - **Galeri:** `gallery.html` yalnızca `vite build --mode development` ve `pnpm dev`'de vardır; `check-bundle-size.mjs` üretim derlemesinde bulunursa başarısız olur. Tray menüsünde (yalnızca debug) "Bileşen galerisi" öğesi; `TOKENRADAR_GALLERY=1` galeriyi açılışta açar. Galeri tema/dil değişikliklerini kaydetmez; yalnızca kendi pencere temasını değiştirir (`core:window:allow-set-theme`, yalnızca galeri capability'sinde).
- **Bedel:**
  - Biçimleme kodu iki dilde bakım ister; bir taraf değişip vektör eklenmezse sapma fark edilmez.
  - Yalnızca EN ve TR biçimleri vardır; yeni dil, iki tarafta da kod ister.
  - `f64` için `number | null` tipi her kullanımda `null` kontrolü gerektirir.
  - Galeri capability dosyası release'te de bulunur (pencere hiç oluşmadığı için etkisizdir).
- **Alternatifler:**
  - `Intl` + vektörlerde ICU'ya özgü istisnalar (eşitlik ve `-0` farkları istisnasız giderilemiyor; WebView2 güncellemesiyle davranış değişebilir).
  - Tüm metni Rust'ta biçimleyip hazır göndermek (her saniyelik geri sayım için IPC gerekir).
  - `prefers-color-scheme`'e bırakıp açılış betiği kullanmamak (kullanıcı teması sistemden farklıysa ilk kare yanlış).
  - specta-typescript 0.0.14 ve specta rc.27 (tauri-specta rc.26 ile uyumsuz).

### ADR-032 — Tray paneli: ikona ortalı konum, GPU'suz WebView, hover ön ısıtması, panel görünüm kuralları
**Durum:** Kabul (2026-10-27, Adım 10)
- **Bağlam:**
  - Plan, konumlama için `tauri-plugin-positioner` ya da özel hesaplama arasında seçim bırakıyordu. Eklentinin `TrayBottomCenter` vb. konumları görev çubuğu kenarını ve çalışma alanını bilmiyor; sol veya sağ görev çubuğunda ve DPI'ı farklı ikinci monitörde panel ekran dışına taşabiliyor.
  - Adım 02, `--disable-gpu` ile panel belleğinin ~%33 düştüğünü gördü (n=1); kararı gerçek panele bıraktı.
  - Gerçek panelde (release, sahte veri, 3 koşu × 3 tur) GPU açıkken soğuk açılış medyanı 432 ms (391–983), panel açık 118–122 MB, tepe 131,6 MB oldu: iki bütçe birden sınırda ya da aşılmış. Faz dökümü, gizli pencere ilk kez görünür olduğunda GPU birleştirmesiyle ilk karenin ~470 ms geciktiğini gösterdi (yazılım birleştirmesinde 17 ms). Pencere örtülme hesabını kapatmak (`CalculateNativeWinOcclusion`) etkisizdi.
  - GPU kapalıyken bile soğuk açılış medyanı 407 ms (386–492; bütçe 400): sürenin ~%70'i WebView2 tarayıcı sürecinin başlaması (`build()` 267–316 ms). `warm_ttl` sonunda tüm WebView2 süreçleri çıktığı için (ADR-023) her soğuk açılış bunu yeniden öder.
  - Tauri `hide()` yalnızca pencereyi gizler; WebView2 controller görünür kaldığı için sayfa `hidden` olmaz, saati ve olay dinleyicisi çalışmaya devam eder, bellek seviyesi düşürülse de sayfa uyumaz.
  - `PanelSnapshot` ilk indekslemenin ilerlemesini ve şema sapmasını taşımıyordu (yalnızca olay olarak geliyordu); panel kapalıyken gelen olaylar kayboluyordu.
  - Plandaki tek satırlı oturum satırı (nokta, ad, proje, pay, hız) 360 px'te çevirilerle sığmıyordu.
- **Karar (kullanıcıyla kararlaştırıldı):**
  - **Konum:** Özel hesaplama, eklenti yok. Panel tray ikonuna ortalanır, görev çubuğu kenarına 12 mantıksal px boşlukla yaslanır, ikonun monitörünün çalışma alanına sıkıştırılır. Kenar `SHAppBarMessage(ABM_GETTASKBARPOS)`, çalışma alanı `MonitorFromPoint` + `GetMonitorInfoW`, ölçek `GetDpiForMonitor`'dan gelir; otomatik gizlenen görev çubuğunun da payı bırakılır. Hesap fiziksel pikselde ve saf bir fonksiyondur (`panel/place.rs`, birim testli). Ölçeği farklı bir monitöre geçerken sıra konum → boyut → konumdur. Köşeler `DWMWA_WINDOW_CORNER_PREFERENCE = DWMWCP_ROUND`.
  - **GPU:** Tüm pencereler `--disable-gpu` ile açılır (`chrome::BROWSER_ARGS`; wry'nin varsayılan argümanları korunur, aynı WebView2 ortamını paylaşan pencerelerin argümanları aynı olmalıdır). Sonuç: soğuk açılış 407 ms medyan, sıcak 47 ms, panel açık 74–77 MB (tepe 80,6), `warm_ttl + 10 s` 4,5 MB; soğuk açılış + 10 s açık CPU'su 1,48–1,62 s'den 1,14–1,38 s'ye indi. %125 ölçekte görsel fark yok. Bir `gpu-process` yine çalışır (yazılım yolu).
  - **Soğuk açılış bütçesi:** Değişmez (≤ 400 ms). Ölçülen 407 ms "sınırda" diye kaydedilir; kalan iyileştirme açık iştir.
  - **Hover ön ısıtması:** `TrayIconEvent::Enter` panel yoksa pencereyi arka planda oluşturur. Sayfa hazır olunca WebView gizlenir ve düşük bellek seviyesine alınır (sıcak gizli panelle aynı durum: gösterilince sayfa görünür olur ve snapshot'ı yeniden çeker); kullanılmazsa `warm_ttl` sonunda yok edilir. Sayfa hâlâ yüklenirken gelen açma isteği hazır olunca gösterilir. Durum makinesi `Page { Absent, Loading { reveal }, Ready }`.
  - **Gizleme:** `window.hide()` + `Webview::hide()` (controller `IsVisible = false`, sayfa `hidden` olur) + `MemoryUsageTargetLevel = Low`; gösterirken tersi. Bellek seviyesi çağrısı ana thread'i beklemez (blur işleyicisinden çağrılır).
  - **`PanelShown` olayı:** Her gösterişte sayfaya gider; soğuk açılışta `visibilitychange` olmadığı için girişi ve odağı bu olay tetikler. Odak ilk butona değil panel kabına gider (ilk buton her açılışta ipucunu açıyordu); `Tab` ayarlar butonundan başlar. Plandaki "odak ilk etkileşimli öğeye" maddesinden sapmadır.
  - **Tray tıklaması:** Basış panelin açık olduğunu (ya da son 200 ms içinde blur ile kapandığını) hatırlar, bırakış buna göre ya kapatır ya açar.
  - **`IndexState` genişletildi:** `progress`, `recent_ready`, `drift` (`SchemaDriftView`). `CoreStatus` bunları tutar, `panel_snapshot()` güncel indeks durumunu snapshot'a bindirir.
  - **"Yenile":** Yeni bir API isteği yolu açılmaz; `CoreHandle::panel_opened()` çağrılır (60 s'den eski veriyi yeniler, backoff'a uyar). Buton API kapalıysa gizli, son istekten 60 s geçmediyse (yerel tıklama dahil) veya istemci engelliyse pasiftir.
  - **Sustur:** Durumun sahibi tray thread'idir; panel `MuteChanged` olayıyla güncellenir.
  - **Projeksiyon satırı:** Önce 5 saatlik, sonra haftalık pencerede "dolacak" uyarısı aranır (kritik renk). Yoksa 5 saatlik pencerenin reset anındaki tahmini, `eta_100_at` yoksa "Tüketim yok". Düşük güvende "≈" ve soluk renk.
  - **Çevrimdışı uyarısı:** Yalnızca birincil limitin kaynağı statusline değilse gösterilir (statusline taze veri getiriyorsa API'nin çevrimdışı olması kullanıcıyı ilgilendirmez).
  - **Görünüm:** `SessionRow` iki satırdır (ad · proje · pay / durum · hız). Ad sırası: ad → başlık → proje → "Adsız oturum"; proje ad olarak kullanıldıysa tekrar yazılmaz. `LimitBar` panelde `stacked` düzende (geri sayım çubuğun altında).
  - **Test kancaları (yalnızca debug veya `spike`):** `TOKENRADAR_FAKE_LIMITS=panel:<sahne>` (14 sahne) ve `fake_panel_scene` komutu; `TOKENRADAR_PANEL_SCENARIO=lifecycle|prewarm|open`; `TOKENRADAR_PANEL_LOG` (`painted`, `phase` satırları). Ölçüm `scripts/measure-panel.ps1`.
- **Bedel:**
  - Soğuk açılış bütçeyi ~7 ms aşıyor; klavye, menü veya ikinci başlatmayla açılış ön ısıtmadan yararlanmaz.
  - Ön ısıtma, imleç ikonun üzerinden geçerken tıklanmasa da bir pencereyi 60 s yaşatır. Hover işleyicisi `build()` süresince (~270 ms) ana thread'i meşgul eder; bu sürede gelen tıklama sıraya girer.
  - GPU'suz çizim, gelecekte ağır animasyon veya büyük grafikler (ana pencere, v0.2) için CPU maliyeti doğurabilir; o adımda yeniden ölçülmeli. ADR-019 sürekli animasyonu zaten yasaklıyor.
  - Konumlama Windows'a özgü; macOS ve Linux Adım 21'de ayrı ele alınır.
  - `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` ortam değişkeni koddaki argümanları ezer (CDP ile geliştirirken `--disable-gpu` elle eklenmeli).
- **Alternatifler:**
  - `tauri-plugin-positioner` (kenar ve çalışma alanı farkındalığı yok).
  - GPU açık + hover ön ısıtması (bellek bütçesi tepe değerde aşılıyor, ilk kare gecikmesi ön ısıtmayla gizlenmiyor: gecikme gösterildikten sonra oluşuyor).
  - Pencereyi ekran dışında görünür tutarak GPU yüzeyini önceden kurmak (odak, Alt+Tab ve örtülme hesabıyla karmaşık; bellek kazancı yok).
  - WebView2 tarayıcı sürecini `warm_ttl` sonrası canlı tutmak (soğuk açılış kalkar ama boşta bellek bütçesi ≤ 30 MB aşılır; ADR-023).
  - Soğuk açılış bütçesini 450 ms'ye çekmek (kullanıcı bütçeyi korumayı seçti).

### ADR-033 — Bildirimler: duruma dayalı notifier, config şeması v2, hesap başına API onayı, OS'ta autostart, log dosyası
**Durum:** Kabul (2026-10-27, Adım 11a)
- **Bağlam:**
  - Plan notifier'ı `LimitEngine` olaylarıyla beslemeyi öngörüyordu. Ancak motorun `ThresholdCrossed` olayı sabit `[75, 90, 100]` eşikleriyle çalışıyor; kullanıcının eşik listesi farklı olabilir. Ayrıca olaylar yalnızca geçiş anında geliyor: yeniden başlatma, eşik listesinin değişmesi ya da tek okumada iki eşiğin birden aşılması olayla doğru ifade edilemiyor.
  - Usage API'nin `resets_at` değeri mikrosaniye düzeyinde kayıyor, statusline'ınki saniye hassasiyetinde (ADR-028). Tekilleştirme anahtarı (`rule:kind:scope:resets_at:threshold`) birebir eşleşmeyle aynı pencereyi iki kez sayabilir.
  - `tauri-plugin-notification` 2.7, Windows'ta tıklamayı yalnızca toast ekrandayken bildiriyor. Bildirim Merkezi'ndeki bir toast'a tıklama, COM activator kaydı olmayan bir uygulamaya hiç ulaşmıyor.
  - Toast'un gönderen adı AppUserModelID'den gelir. Eklenti exe `target\debug` veya `target\release` içindeyse PowerShell'in kimliğini kullanır; başka her yerde uygulama kimliğini (`identifier`) kullanır. Bu kimliği yalnızca installer'ın Başlat menüsü kısayolu kaydeder (Adım 12).
  - Plan şeması `[[accounts]]` altında hesap başına `api_consent` istiyordu; v1 dosyalarında onay en üstte `[api_consent]` tablosundaydı.
  - Planın `[general].autostart` alanı, kullanıcı autostart'ı Görev Yöneticisi'nden kapattığında gerçeği yansıtmaz.
  - Uygulamanın log dosyası yoktu (`tracing` çıktısı hiçbir yere yazılmıyordu); doctor (11b) ve log seviyesi ayarı buna dayanıyor.
  - Fluent terimleri (`-limit`) yalnızca sabit argüman alabiliyor; `-limit(limit: $limit)` biçimi çözümlenmiyor.
- **Karar (kullanıcıyla kararlaştırıldı):**
  - **Bölme:** Adım 11, 11a (notifier, toast gönderimi, config v2 + migration, log dosyası, autostart, ayar komutları) ve 11b (ayarlar UI, onboarding, tray sabitleme yardımcısı, doctor, bir günlük gürültü değerlendirmesi) olarak iki oturuma bölündü.
  - **Duruma dayalı notifier:** `core-limits`, oluşturduğu her `PanelSnapshot` için `Notifier::evaluate` çağırır. Notifier saftır: ne gösterileceğini ve ne kaydedileceğini döndürür; runtime kayıtları `notification_log`'a yazar ve `CoreEvent::Notify(Notice)` yayınlar. Reset bir kenar olduğu için motorun `LimitEvent::Reset` olayından gelir (`on_reset`). Açılışta son 8 günün kayıtları okunur (`HISTORY_MS`); kayıtlar 30 gün tutulur.
  - **Pencere eşleşmesi:** İki `resets_at` değeri ±5 dk içindeyse aynı pencere sayılır (`SAME_WINDOW_MS`). Anahtar biçimi değişmedi: `rule:kind:scope|-:resets_ms:threshold|-`.
  - **Birden çok eşik:** Açılışta veya bir sıçramada birden çok eşik aynı anda yeni geçilmişse yalnızca en yüksek eşik gösterilir, düşükler sessizce kaydedilir. Susturma bittiğinde de aynı kural geçerlidir. Öncelik sırası: dolu (%100) > son uyarı > projeksiyon > diğer eşikler, sonra eşik değeri.
  - **Projeksiyon:** `projection` açık, yüzde < 100, `will_hit_before_reset`, güven `Low` değil ve ETA ≤ 60 dk. ETA `final_warning_min` (varsayılan 15) altına inerse bir kez "son uyarı" verilir; son uyarı tetiklendiğinde projeksiyon sessizce kaydedilir. Tahmin kaynaklı (`Source::Estimate`) limitler bildirim üretmez.
  - **Susturma ve sessiz saatler:** Bu sürelerde bildirimler kaydedilir ama bekletilir. İkisi de bitince yalnızca hâlâ geçerli olanlar, güncel sayılarla yeniden yazılarak gösterilir: tek bildirim olduğu gibi, birden çok bildirim tek bir özet halinde. `limits_loop` uyanma zamanına susturmanın veya sessiz saatlerin bitişi eklenir. Susturmanın tek kaynağı `kv.notify_snoozed_until`'dır (`notifier::KV_MUTED_UNTIL`). Tray yazar ve `CoreHandle::set_muted_until` ile bildirir; runtime açılışta okur.
  - **Saat:** `LocalClock` trait'i core'a taşındı (`clock.rs`). Uygulama Windows saatini (`SystemClock`) enjekte eder; core'a saat dilimi veritabanı girmez. Testler `UtcClock` kullanır.
  - **Metinler:** `locales/{en,tr}/notify.ftl`. Limit adı ayrı bir `notify-limit` mesajı olarak Rust'ta biçimlenir ve `$what` argümanıyla geçirilir (Fluent terimlerine değişken geçirilemez). 5 saatlik pencerede "N dk sonra yenilenir", haftalık pencerede "Cum 10:59'da yenilenir" yazılır.
  - **Toast tıklaması:** Sınır kabul edildi. Toast ekrandayken tıklanırsa panel açılır; Bildirim Merkezi'nden tıklama hiçbir şey yapmaz. COM activator Adım 12'nin installer'ına bırakıldı.
  - **AppUserModelID:** `target\debug` ve `target\release` içinden çalışan build'lerin toast'ları "Windows PowerShell" adıyla görünür; bu davranış doğrulandı. Portable kopya ve kurulmamış başka bir konum uygulama kimliğini kullanır. Bu kimlik kayıtlı olmadığı için Windows toast'u **hata vermeden göstermeyebilir**. Bu nedenle gerçek kullanım testleri `target\release` içinden yapılır ve görünürlük önce "Test bildirimi gönder" ile doğrulanır.
  - **Config şeması v2:** `schema_version = 2`; yeni bölümler `[notifications]`, `[data]` (`raw_retention_days`, `update_check`) ve `[advanced]` (`log_level`); `[general].panel_warm_ttl` (0–600 s). Bölümler hoşgörülü okunur: bozuk bir bölüm varsayılana döner ve diğer bölümler (özellikle onay) kaybolmaz. Yazımlar `toml_edit` ile bilinmeyen anahtarları korur ve değerleri normalize eder (eşikler 1–100 arası, sıralı, tekil, en fazla 5; saat biçimi `HH:MM`).
  - **Hesap başına API onayı ve migration:** `[[accounts]]` altında `api_consent { granted, granted_at, consent_version }`. v1 → v2 geçişi en üstteki `[api_consent]` tablosunu birincil hesaba (ilk keşfedilen) taşır; birincil hesap yoksa tablo yerinde kalır ve okumada hesabı olmayan her hesaba uygulanır. Dosya yoksa oluşturulmaz, daha yeni şemalara dokunulmaz. Çoklu hesap UI'ı gelene kadar (Adım 19) tüm poller'lar birincil hesabın onayını kullanır.
  - **Autostart:** `tauri-plugin-autostart` (HKCU `Run`, değer adı `TokenRadar`). Gerçeğin kaynağı işletim sistemidir; `config.toml`'de `autostart` alanı yoktur. `get_autostart` / `set_autostart` komutları OS'un bildirdiği durumu döndürür. Kayıt yalnızca kullanıcı açınca yazılır.
  - **Log dosyası:** `<data>/logs/tokenradar.YYYY-MM-DD.log`, günlük döner, son 7 dosya kalır. Açılışta toplam 20 MB'ı aşan en eski dosyalar silinir. Seviye `advanced.log_level`'dan gelir (varsayılan `info`) ve yeniden başlatmadan değişir (`reload` katmanı). `env-filter` özelliği regex bağımlılığı getirdiği için kullanılmadı.
  - **Ayar köprüsü:** `SettingsView` yalnızca her sayfanın ilk boyamadan önce ihtiyaç duyduklarını taşır (dil, tema, OS, saat biçimi). Diğer ayarlar ayrı `PrefsView` / `get_prefs` / `PrefsChanged` ile taşınır; böylece panelin açılış betiği büyümez. Setter komutları: `set_notification_prefs`, `set_data_prefs`, `set_log_level`, `set_panel_warm_ttl`, `send_test_notification`. Değişiklikler core'a canlı iletilir; saklama süresi ise bir sonraki günlük çalışmada uygulanır.
- **Bedel:**
  - Notifier her snapshot'ta (en sık 2 s'de bir) tüm limitleri değerlendirir; maliyeti birkaç limitlik bir döngüdür.
  - ±5 dk tolerans, 5 dk'dan kısa aralıkla art arda gelen iki pencereyi (pratikte yok) tek pencere sayar.
  - Bildirim Merkezi'ndeki toast'a tıklama Adım 12'ye kadar işe yaramaz.
  - Portable modda toast görünmeyebilir.
  - Autostart ayarı kayıt defterinde tutulur; `auto-launch` ayrıca `StartupApproved\Run` altına bir değer yazar ve kapatınca silmez (zararsız, Görev Yöneticisi'nin durum bilgisi).
  - Log boyut sınırı yalnızca açılışta uygulanır; çok gürültülü (`trace`) uzun bir oturum sınırı geçici olarak aşabilir.
- **Alternatifler:**
  - Olay tabanlı notifier, motorun eşiklerini kullanıcının listesiyle yapılandırarak (yeniden başlatma ve liste değişimi yine ayrı ele alınmak zorunda).
  - `resets_at`'i dakikaya yuvarlayarak anahtar üretmek (dakika sınırında bölünüyor).
  - Toast için kendi COM activator'ımızı şimdi kaydetmek (kurulumsuz kayıt, portable'da kalıntı bırakır; installer işi).
  - `config.toml`'de `autostart` alanı tutmak (OS ile ayrışabilir).
  - Ayarların tamamını `SettingsView`'a koymak (her pencerenin açılış betiği büyür).

### ADR-034 — Ayarlar penceresi ve doctor: 11b/11c bölmesi, `release-cli` profili, birincil Claude dizini, veri silme, Rust'tan dialog/opener
**Durum:** Kabul (2026-10-27, Adım 11b)
- **Bağlam:**
  - 11a'nın devir notu 11b'ye ayarlar UI'ını, onboarding'i, tray sabitleme yardımcısını, doctor'ı, bir günlük gürültü değerlendirmesini ve bellek ölçümünü bırakmıştı. Bunların hepsi tek oturuma sığmıyor.
  - Doctor alt komutu eklenince `tokenradar.exe` (`--release`, `opt-level = 3`) 3.289.600 B'a (3,14 MiB) çıktı. Bu, ≤ 3 MB bütçesini aşıyor. Adım 07'de `opt-level = "s"` denenmiş, ama uygulanmamıştı.
  - Plan "Claude dizini: … Değiştir… butonu" diyor; ancak birden çok dizin keşfediliyor (`~/.claude`, `CLAUDE_CONFIG_DIR`). Seçilen dizinin diğerlerini devre dışı bırakması, kullanıcının fark etmeden veri kaybetmesine yol açardı.
  - "Tüm verileri sil" çalışan çekirdeğin açık SQLite bağlantılarıyla çakışıyor: Windows açık bir dosyayı silmeye izin vermiyor. Tray de kendi `Store` kopyasını tutuyor.
  - `uninstall` statusline'ı geri alırken config'teki önceki komut kaydına ihtiyaç duyuyor (ADR-029).
  - Klasör seçici ve "Explorer'da göster" / bağlantı açma için platform API'leri gerekiyor.
  - Plan, kullanıcının mevcut statusline komutu usage API'sini çağırıyorsa bir bilgi notu istiyordu. Bunu bilmenin tek yolu kullanıcının betiğini okumak olurdu.
- **Karar (kullanıcıyla kararlaştırıldı):**
  - **Bölme:** 11b: ayarlar penceresi (5 sekme), statusline diff diyaloğu, API onayı, veri silme, doctor. 11c: onboarding, tray sabitleme yardımcısı, bir günlük gürültü değerlendirmesi, bellek ölçümü.
  - **CLI profili:** Kök `Cargo.toml`'e `[profile.release-cli] inherits = "release", opt-level = "s"` eklendi. CLI bununla 2.273.280 B (2,17 MiB) oluyor. Statusline p95 yeniden ölçüldü ve bütçe içinde (`docs/perf/2026-10-27-cli-profile.md`). Uygulama `opt-level = 3` ile kalır. `measure-statusline.ps1` varsayılan olarak bu profili ölçer. Adım 12'de paketleme (`tauri.conf` `beforeBuildCommand`) bu profile geçirilir.
  - **Birincil Claude dizini:** `[general].claude_config_dir` seçilen dizini tutar. `discover_configured` seçilen dizini listenin başına koyar, böylece birincil hesap (panelin gösterdiği) o olur. `~/.claude` ve `CLAUDE_CONFIG_DIR` **izlenmeye devam eder**. Bu ayarlar ekranında yazılıdır. Seçilen dizin yoksa uyarı loglanır, algılanan dizinler kullanılır ve ekranda gösterilir. Seçim yalnızca içinde `projects\` veya `settings.json` olan bir klasörü kabul eder. Seçim ve sıfırlama çekirdeği süreç içinde yeniden başlatır.
  - **Çekirdeği süreç içinde yeniden başlatma:** `core_bridge::stop` çekirdeği kapatır ve tray'e `TrayMsg::CoreStopped(ack)` gönderir. Tray `Store`'unu bırakıp onaylar (en fazla 2 s beklenir). `restart` ardından `start` çağırır.
  - **Tüm verileri sil:**
    - Sırayla: çekirdek durdurulur; veritabanı, `-wal`, `-shm` ve spool silinir. Log dosyalarından günün dosyası boşaltılır (yazıcı onu append modunda açık tutar), diğerleri silinir.
    - `config.toml` yalnızca `statusline.installed = true` olan hesapların `provider`, `config_dir` ve `statusline` alanlarıyla yeniden yazılır; böyle hesap yoksa silinir.
    - Ardından ayarlar yeniden okunur (`prefs::reload`, değişiklik olaylarıyla) ve çekirdek yeniden başlar.
    - Claude Code dosyalarına, WebView2 verisine ve autostart kaydına dokunulmaz.
    - Silinemeyen dosyalar listelenir ve ekranda genel bir uyarı gösterilir.
    - Onboarding'in yeniden görünmesi 11c'nin işidir.
  - **Doctor:**
    - Biçim: sıkıştırılmamış (stored) zip ve elle yazılmış CRC32; sıkıştırıcı bağımlılığı eklenmez.
    - İçerik: `README.txt`, `report.json` (`schema = 1`), temizlenmiş `config.toml` ve redakte edilmiş son 200 log satırı.
    - Yollar `path#<sha256'nın ilk 8 hex'i>` olarak gösterilir (küçük harf, `/` normalleştirilmiş). `label` gizlenir. `previous` / `previous_command` yalnızca uzunluklarıyla yazılır.
    - Log satırları: `accessToken`, `refreshToken` veya `access_token` geçen satırlar atılır. `sk-ant-…` ve `Bearer …` maskelenir. Sürücü, UNC ve `/home/` yolları etiketlenir.
    - Canlı bölüm (uygulamada snapshot'tan, CLI'da IPC `status.get`'ten) ad, başlık ve proje taşımaz.
    - Performans ölçümleri pakete girmez (plan istiyordu). Uygulama çalışırken bunları tutmuyor; `docs/perf/` kayıtları geliştirici ölçümleridir.
    - Uygulamadaki buton bir kaydetme diyaloğu açar ve dosyayı Explorer'da gösterir. Hiçbir şey gönderilmez.
  - **Eklentiler:** `tauri-plugin-dialog` (klasör seçici, kaydetme diyaloğu) ve `tauri-plugin-opener` (Explorer'da göster, sabit GitHub sayfaları). İkisi de yalnızca Rust'tan çağrılır; JS komutlarına capability verilmez. `open_link` yalnızca `Link` enum'undaki dört sabit GitHub adresini açar (ADR-013).
  - **Statusline diyaloğu:** `plan_statusline` diff'i sınıflandırılmış satırlar olarak ve bir token ile döner. `apply_statusline` yalnızca o token'lı planı uygular. `Plan::apply` dosya arada değiştiyse yine reddeder; o zaman diyalog yeni planı gösterir. Diyalog kapanınca plan atılır. CLI önce `<exe>\bin\tokenradar.exe`, sonra `<exe>\tokenradar.exe` yolunda aranır; bulunamazsa kurulum düğmesi kapalıdır.
  - **Usage API notu:** Kullanıcının betiği okunmaz. Zincirleme kurulumda genel bir not gösterilir: önceki komut, kendi ağ istekleri dahil, bugün ne yapıyorsa yapmaya devam eder.
  - **API onayı:** Açıklama (ne olur, ne okunur, ne asla okunmaz, nereye gider) her zaman görünür. İzin açık bir düğmeyle verilir ve `granted_at` ile `consent_version` (`API_CONSENT_VERSION = 1`) kaydedilir. Sürüm eskiyse onay sayılmaz ve ekranda söylenir.
  - **Lisanslar:** Üçüncü parti lisans listesi Adım 12'nin build'inde üretilir. "Hakkında" sekmesi şimdilik bunu yazar ve MIT lisansına bağlantı verir.
  - **Loglama:** Uygulamadaki hata `eprintln!` çağrıları `tracing`'e taşındı; böylece log dosyasına ve doctor'a ulaşıyorlar.
- **Bedel:**
  - CLI `opt-level = "s"` ile p95'te ~1 ms yavaşlayabilir (ölçüm gürültüsü içinde).
  - İki eklenti uygulama ikilisini büyütür; ölçülen boyut 12.623.360 B (12,04 MiB) ve ≤ 15 MB bütçesinin içinde.
  - Seçilen dizinden başka dizinler de izlendiği için, bir dizini izlemekten tamamen çıkarmanın yolu yok (Adım 19'un çoklu hesap UI'ı).
  - Silme sonrası, çekirdek durduktan sonra yazılan birkaç log satırı kaybolabilir.
  - Doctor'da performans verisi yok.
- **Alternatifler:**
  - Seçilen dizinin diğerlerinin yerine geçmesi (sessiz veri kaybı) veya listeye "eklenmesi" (birincil belirsiz kalır).
  - Silme için uygulamayı yeniden başlatmak (tray simgesi kaybolup gelir; autostart'sız portable'da kendini başlatamayabilir).
  - JS'ten dialog/opener (capability yüzeyi büyür).
  - Deflate'li zip için `zip` crate'i (CLI bütçesine yük).
  - Kullanıcının statusline betiğini okuyup usage API çağrısı aramak (içerik okuma; tahmin).
  - CLI'yı `--release` ile bırakıp bütçeyi gevşetmek.

### ADR-035 — Onboarding ve tray sabitleme: `kv` bayrağı, kapatma = tamamlandı, `IsPromoted` + geri okuma, sayfa başına katalog
**Durum:** Kabul (2026-10-27, Adım 11c)
- **Bağlam:**
  - Plan, ilk çalıştırmada 4 adımlı bir onboarding istiyor ve "indeksleme 2. adımda arka planda başlar" diyor. Ama çekirdek Adım 05'ten beri uygulama açılır açılmaz indekslemeye başlıyor.
  - Kullanıcı onboarding'i bitirmeden pencereyi kapatabilir. Pencere her açılışta geri gelirse tray-first bir uygulamada can sıkar.
  - Windows 11'de bildirim alanı simgeleri varsayılan olarak taşma menüsüne düşer. Bunu programla değiştirmenin belgelenmiş bir API'si yok. Tek yol `HKCU\Control Panel\NotifyIconSettings\<id>` altındaki yarı-belgelenmiş `IsPromoted` değeri.
  - Bu anahtarlarda `ExecutablePath` bazen known-folder GUID önekiyle saklanıyor (`{6D809377-…}\…` Program Files, `{1AC14E77-…}\…` System32). Aynı exe farklı konumlardan çalıştırıldıysa birden çok anahtar olabiliyor.
  - Tek bir `catalogs.ts` modülü, Rolldown'un paylaşılan chunk'ı yüzünden panelin tüm sayfaların FTL metinlerini yüklemesine yol açtı (panel JS 45,4 → 47,4 KB gzip).
- **Karar (kullanıcıyla kararlaştırıldı):**
  - **İlk çalıştırma:** `kv.onboarding_done` yoksa çekirdek her başladığında (`core_bridge::start`) onboarding açılır. Bu yüzden "Tüm verileri sil" sonrası da yeniden açılır. `TOKENRADAR_SKIP_ONBOARDING` ortam değişkeni ölçüm ve otomasyon için açılışı atlar.
  - **Kapatma = tamamlandı:** Bayrak pencere yok edilince (`WindowEvent::Destroyed`) yazılır; X ile kapatmak da "Bitir" gibi sayılır. Yalnızca açıkça tıklanan eylemler uygulanır. Tray menüsündeki "Başlangıç rehberi…" pencereyi istendiğinde yeniden açar.
  - **İndeksleme:** Uygulama açılışında başlamaya devam eder (plandan sapma). 2. adım yalnızca canlı ilerlemeyi (`index_progress`, `index_milestone`) gösterir. Böylece onboarding'i atlayan kullanıcıda da veri hazır olur.
  - **Limit adımı:** Statusline kartı ayarlardaki `StatuslineDialog`'u, API kartı ayarlardan çıkarılan ortak `ApiConsent` bileşenini kullanır. Böylece onay metni iki yerde birebir aynıdır. Hiçbir kaynak yoksa "İleri" düğmesi "Atla" olur ve sonucu açıkça yazılır. Onboarding metinleri `onboarding.ftl`'de; ortak metinler `settings.ftl`'den gelir.
  - **Windows ile başlat:** Varsayılan açık. Yalnızca "Bitir"de ve mevcut değerden farklıysa yazılır. X ile kapatmak autostart'ı değiştirmez.
  - **Tray sabitleme:**
    - Yalnızca kullanıcı "Simgeyi her zaman göster"e basınca yazılır.
    - `NotifyIconSettings` altındaki tüm alt anahtarlar gezilir. `ExecutablePath` known-folder GUID'i `SHGetKnownFolderPath` ile açılarak, büyük/küçük harf ve `/`↔`\` farkı gözetmeden çalışan exe ile karşılaştırılır. Eşleşen **tüm** anahtarlara `IsPromoted = 1` (DWORD) yazılır.
    - Yazımdan sonra değer geri okunur. Durum: `shown` (tüm eşleşmeler promoted), `hidden`, `unknown` (anahtar henüz yok; Explorer simge bir kez görünene kadar oluşturmaz), `unsupported` (Windows 10 veya kök anahtar yok).
    - Explorer değişikliği yeniden başlatmadan uygular (değer eklenip silinerek doğrulandı).
    - Başarısız olursa veya durum `hidden` kalırsa manuel talimat ve `ms-settings:taskbar`'ı açan bir düğme gösterilir. Windows 10'da yalnızca talimat gösterilir.
    - Değer geri alınmaz ve başka hiçbir anahtara dokunulmaz.
  - **Taşma menüsü görseli:** Ekran görüntüsü değil, çizilmiş bir SVG şema. Temaya ve yüksek kontrasta uyar, dil taşımaz.
  - **Katalog:** Her sayfanın kendi `catalog.ts` dosyası var (panel, settings, onboarding, gallery). Panel yalnızca common + panel metinlerini yükler (JS 38,5 KB gzip).
- **Bedel:**
  - `IsPromoted` belgelenmemiş bir mekanizma. Bir Windows güncellemesi bunu bozabilir; o durumda manuel talimat yolu kalır.
  - Exe konumu değişirse (güncelleme, portable kopya) Explorer yeni bir anahtar oluşturur ve simge tekrar taşma menüsüne düşebilir.
  - Onboarding'i yanlışlıkla kapatan kullanıcı onu tray menüsünden bulmak zorunda.
  - Onboarding penceresi açıkken bellek ~80 MB'a çıkar (WebView2). Pencerenin `warm_ttl` değeri 0'dır.
- **Alternatifler:**
  - Bayrağı yalnızca "Bitir"de yazmak (her açılışta tekrar gelir).
  - Sabitleme için Explorer'ı yeniden başlatmak veya `TrayNotify` ikili blob'unu düzenlemek (kırılgan, yan etkili).
  - Yalnızca talimat göstermek (kayıt yazmadan).
  - Taşma menüsünün ekran görüntüsü (dile ve temaya bağlı, bakımı zor).
  - İndekslemeyi onboarding'in 2. adımına kadar bekletmek.

### ADR-036 — Paketleme ve güncelleme: özel NSIS şablonu, `Programs` kurulum dizini, PATH'i CLI düzenler, el ile tetiklenen updater, release doğrulaması
**Durum:** Kabul (2026-10-28, Adım 12)
- **Bağlam:**
  - Tauri'nin `currentUser` NSIS şablonu uygulamayı `%LOCALAPPDATA%\${PRODUCTNAME}` dizinine kuruyor. Bizim veri dizinimiz de `%LOCALAPPDATA%\TokenRadar`. Exe'ler ile veritabanı aynı yerde olurdu ve kaldırıcının `RmDir` adımı veriyi de silerdi.
  - Şablonun yeniden kurulum akışı, eski kaldırıcıyı yalnızca kullanıcı "güncelle" seçerse `/UPDATE` ile çalıştırıyor. Aksi hâlde bizim kaldırma hook'larımız çalışır; statusline köprüsü, autostart ve PATH silinip yeni kurulum bunları geri getirmez.
  - Tauri'nin NSIS'i `NSIS_MAX_STRLEN=1024` ile derlenmiş. Bu makinedeki kullanıcı PATH'i 1254 karakter. NSIS'te okunan PATH kırpılır, geri yazılınca bozulur.
  - `statusline uninstall` varsayılan Claude dizinini hedefliyor. Köprü başka bir dizine kurulmuş olabilir ya da varsayılan dizin hiç olmayabilir.
  - Plan, imzalı updater, günlük denetim ve "sessiz yükleme yok" kuralını istiyor. Portable kopyada updater kapalı olmalı.
  - Updater imzası ile uygulamadaki açık anahtar uyuşmazsa yayınlanan güncellemeyi hiçbir kurulu kopya kabul etmez. Bu hata ancak bir sonraki sürümde fark edilir.
- **Karar:**
  - **Şablon:** `app/src-tauri/windows/installer.nsi`, tauri-cli 2.12.4 şablonunun birebir kopyası. Yalnızca `TokenRadar:` ile işaretli iki değişiklik var:
    1. Kurulum dizini `%LOCALAPPDATA%\Programs\TokenRadar`.
    2. Yeniden kurulumda eski kaldırıcı her zaman `/UPDATE` ile çalışır.
  - `scripts/check-nsis-template.mjs` işaretli değişiklikleri geri alır ve sonucu kurulu `@tauri-apps/cli` paketinin gömülü şablonuyla karşılaştırır. Tauri yükseltmesinde şablon değiştiyse CI başarısız olur ve kopya elle güncellenir.
  - **Hook'lar (`windows/hooks.nsh`):**
    - Kurulum sonrası `bin\tokenradar.exe path add` çalıştırılır.
    - Kaldırmadan önce (güncelleme değilse) sırasıyla şunlar yapılır: uygulama kapatılır, `statusline uninstall --all --yes` çalıştırılır, `path remove` çalıştırılır, `Run` ve `StartupApproved\Run` altındaki `TokenRadar` değerleri silinir.
    - Veri dizini yalnızca Tauri'nin "Uygulama verilerini sil" kutusu işaretliyse silinir. Varsayılan: veri korunur.
  - **PATH'i CLI düzenler (`tokenradar path add|remove`):**
    - Kayıt defteri Rust'tan okunup yazılır; değer tipi (`REG_EXPAND_SZ`/`REG_SZ`) korunur.
    - Yineleme kontrolü büyük/küçük harfe, tırnağa, sondaki `\` işaretine ve `%VAR%` genişletmesine duyarsızdır.
    - Sonuç 2047 karakteri aşacaksa PATH'e yazılmaz, çıkış kodu 3 olur. Installer bu durumda dizini elle eklemeyi anlatan bir mesaj gösterir.
    - Değişiklikten sonra `WM_SETTINGCHANGE` yayınlanır.
  - **`statusline uninstall --all`:** `config.toml`'de köprüsü kurulu görünen her hesabın dizinini geri alır. Bir dizin başarısız olsa da diğerleri denenir. Başarısızlıkta installer, yedeklerin `settings.json.tokenradar-….bak` adıyla durduğunu söyler.
  - **CLI pakete `bundle.resources` ile girer** (`bin/tokenradar.exe`). `externalBin` ada target triple eklediği için kullanılmadı. CLI `beforeBuildCommand` içinde `release-cli` profiliyle derlenir.
  - **Portable kuralı:** `bin\` altındaki CLI, `portable` işaretini bir üst dizinde de arar. Böylece zip'teki uygulama ve CLI aynı `data\` dizinini kullanır. Scoop aynı zip'i kurar ve `data`'yı `persist` eder.
  - **Updater:**
    - Rust'ta `tauri-plugin-updater` (`rustls-tls`) kullanılır. Frontend'e izin verilmez; yalnızca `get_update`, `check_for_update` ve `install_update` komutları açıktır.
    - Günlük denetim debug build'de çalışmaz. Release build'de açılıştan 120 s sonra başlar ve en fazla 24 saatte bir tekrarlanır. Son denetim zamanı `kv.update_checked_at` içinde tutulur. Ayar kapatılırsa denetim durur; açılınca zamanlayıcı uyanır.
    - Bulunan sürüm tray menüsünün başına ve Hakkında'ya eklenir. Kurulum yalnızca "Kur ve yeniden başlat" ile başlar. Installer `/UPDATE /P` (passive) ile çalışır; uygulama çıkmadan önce çekirdeği düzgünce kapatır.
    - Uygulama yalnızca yanında `uninstall.exe` varsa ve portable değilse kurulum yapabilir. Portable kopya sürümü gösterir ve sürümler sayfasını önerir.
  - **İmza yalnızca release'de:** `createUpdaterArtifacts` ana config'de kapalıdır, `tauri.release.conf.json` overlay'i açar. Yerel `pnpm tauri build` ve CI anahtar gerektirmez.
  - **Release doğrulaması:** `scripts/release-assets.mjs` installer imzasını `tauri.conf.json`'daki açık anahtarla gerçekten doğrular (Ed25519, BLAKE2b ön özeti, güvenilir yorum imzası). Key id uyuşmazsa veya imza tutmazsa taslak release oluşmaz. Betik `latest.json`, `SHA256SUMS.txt`, sürüm notları (CHANGELOG bölümü) ile Scoop ve winget manifestlerini üretir.
  - **Lisanslar:** `scripts/third-party-notices.mjs`, Windows hedefinde gönderilen Rust bağımlılık grafiğini (`cargo metadata`, normal bağımlılıklar), Vite'ın bundle'a koyduğu npm paketlerini ve gömülü fontu `THIRD-PARTY-NOTICES.txt` dosyasına yazar. Hakkında bu dosyayı açar.
  - **`paste` advisory'si (RUSTSEC-2024-0436) yok sayılır.** Paket bakımsız ama güvenlik açığı yok. Tauri ve specta'nın derleme zamanı makrosu; gönderilen binary'de kodu yok.
- **Bedel:**
  - Şablon kopyası her tauri-cli yükseltmesinde elle eşitlenmeli; CI bunu hatırlatır.
  - PATH 2047 karakteri aşıyorsa CLI kendiliğinden PATH'e girmez.
  - Installer imzasız (ADR-012). İlk indirmelerde SmartScreen uyarı verir.
  - Portable ve Scoop kopyalarında Başlat menüsü kısayolunda AUMID olmadığı için toast görünmeyebilir (ADR-033).
  - Bildirim Merkezi'nden toast'a tıklamak uygulamayı açmaz; COM activator kayıtlı değil.
  - Uygulama 12,04 → 13,02 MiB büyüdü (updater, rustls, `reqwest` paylaşımı). 15 MiB bütçesinin içinde.
- **Alternatifler:**
  - Veri dizinini taşımak (mevcut kullanıcıların verisi için migration gerekirdi).
  - PATH için NSIS `EnVar` eklentisi (ek ikili bağımlılık; uzun PATH'te davranışı belirsiz) ya da ham NSIS registry yazımı (1024 sınırı).
  - `externalBin` ile CLI'ı ekleyip adını hook'ta değiştirmek.
  - Updater'ı frontend'den sürmek (geniş izin, iş mantığı UI'da olurdu).
  - Release'de imzayı doğrulamamak (anahtar uyuşmazlığı ilk güncellemede ortaya çıkardı).

### ADR-037 — Panel soğuk açılış bütçesi 400 ms'den 500 ms'ye
**Durum:** Kabul (2026-10-28, Adım 12; ADR-032'deki "bütçe değişmez" maddesinin yerini alır)
- **Bağlam:**
  - Adım 10'da soğuk açılış medyanı 407 ms ölçülmüş, bütçeyi 7 ms aştığı hâlde "sınırda" kaydedilmişti (ADR-032).
  - v0.1.0 doğrulamasında iki ayrı ölçüm yapıldı (her biri 3 koşu × 3 tur). Medyanlar 443 ve 440 ms çıktı.
  - Aynı gün ölçülen Adım 11c build'i 468 ms verdi. Yani aşımı Adım 12 getirmedi. Değer makinenin o anki durumuna göre 400–470 ms arasında oynuyor; WebView2 sürümü değişmedi.
  - Sürenin ~%70'i WebView2 tarayıcı sürecinin başlaması (`build()` 267–316 ms). Bu süreç ADR-023 gereği `warm_ttl` sonunda kapatılıyor; böylece "kapandıktan sonra ≤ 30 MB" bütçesi tutuyor. İki bütçe birbirini çekiştiriyor.
  - Gerçek tray tıklamasında hover ön ısıtması bu sürenin çoğunu gizliyor (ADR-032). Klavye, tray menüsü ve ikinci başlatma ise soğuk süreyi tam öder.
- **Karar (kullanıcıyla kararlaştırıldı):**
  - Panel soğuk açılış bütçesi **≤ 500 ms**. Sıcak açılış bütçesi (≤ 50 ms) ve bellek bütçeleri değişmedi.
  - Sayfa fazındaki iyileştirme açık iş olarak kalır: snapshot'ı açılış betiğiyle göndermek tahminen 10–40 ms kazandırır.
- **Bedel:**
  - Hover olmadan açılan panel (klavye, menü, ikinci başlatma) yarım saniyeye yakın gecikebilir.
  - Bütçe ölçülen değere yaklaştırıldı. 500 ms'yi aşan bir gerileme yine sürüm kapısıdır.
- **Alternatifler:**
  - WebView2 tarayıcı sürecini canlı tutmak: soğuk açılışı ~150 ms kısaltır ama boşta ~50 MB ekler. Bu, "görünmez ayak izi" vaadini ve ≤ 30 MB bütçesini bozar.
  - Bütçeyi koruyup sürümü ertelemek: ulaşılabilir kazanç 400 ms'nin altına inmeyi garanti etmiyor.
  - "Bilinen aşım" olarak kaydedip bütçeyi değiştirmemek: belgelenmiş bir kapının sürekli aşık durması onu anlamsızlaştırır.
