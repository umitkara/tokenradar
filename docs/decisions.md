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
