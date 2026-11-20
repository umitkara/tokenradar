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
**Durum:** Yerini aldı: ADR-058
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

### ADR-038 — `notify` Windows watcher'ı yamalı kopyayla; boşta CPU döngü ve uyanma sayısıyla ölçülür
**Durum:** Kabul (2026-10-28, Adım 12)
- **Bağlam:**
  - Kurulu v0.1.0 ile gerçek boşta ölçüm (Claude Code kapalı, 10 dk): bellek 11,6 MB (bütçe ≤ 25 MB), `TotalProcessorTime` farkı %0,169 (bütçe < %0,1).
  - `scripts/measure-threads.ps1` ile thread başına bakıldı: `notify-rs windows loop` thread'i boşta saniyede tam 10 kez uyanıyor. Diğer thread'ler dakikada birkaç kez uyanıyor.
  - Sebep `notify` 8.2.1'deki `WaitForSingleObjectEx(wakeup_sem, 100, TRUE)` çağrısı (`src/windows.rs`). Her komuttan sonra `wakeup_server()` semaforu zaten serbest bırakıyor, `ReadDirectoryChangesW` tamamlama rutinleri de alertable beklemede APC olarak çalışıyor. Zaman aşımının tek etkisi boşta uyanma. 9.0.0-rc.7'de de aynı.
  - Bu, bütçedeki "saniyede gereksiz uyanma yok" kuralını bozuyor.
  - `TotalProcessorTime` 15,6 ms'lik saat tikiyle örneklenir; kısa ve sık uyanmaları doğru ölçmez. `QueryThreadCycleTime` ile aynı uygulama boşta (Claude Code açık ama sessiz, 60 s) bir çekirdeğin ~%0,02'sini kullandı.
- **Karar (kullanıcıyla kararlaştırıldı):**
  - `notify` 8.2.1, `vendor/notify` altına kopyalanır ve `[patch.crates-io]` ile kullanılır. Tek değişiklik bekleme süresinin `INFINITE` olması; satır işaretli. Kopya workspace'ten `exclude` ile çıkarılır, bu yüzden lint ve fmt kapsamında değildir; lisans listesinde üçüncü parti olarak kalır.
  - Değişiklik upstream'e önerilir (kullanıcı onayıyla). Kabul edilen sürüme geçilince kopya ve patch kaldırılır.
  - Boşta CPU artık `scripts/measure-threads.ps1` ile ölçülür: thread başına context switch (uyanma) ve CPU döngüsü. `measure-memory.ps1`'in CPU sütunu yalnızca kaba bir gösterge.
- **Bedel:**
  - `notify` güncellemeleri elle taşınır. Dependabot, patch edilen crate'i güncelleyemez.
  - Kurulu v0.1.0 kopyaları yeni koddan geride kalır; aynı sürüm numarası olduğu için updater bunu göstermez. Gerçek boşta ölçüm ve bir günlük bildirim değerlendirmesi yeni build ile yeniden yapılır.
- **Alternatifler:**
  - `ReadDirectoryChangesW` üzerine kendi watcher'ımız: bağımlılık kalkar ama iş büyür ve v0.5'te macOS/Linux için yine `notify` gerekir.
  - v0.1.1'e bırakmak: sürüm, bilinen bir bütçe ihlaliyle çıkardı.
  - `PollWatcher`: periyodik tarama, sorunu büyütür.

### ADR-039 — `notify` yamasının kapsamı: semafor ömrü ve `configure`
**Durum:** Kabul (2026-10-28, Adım 12; ADR-038'i genişletir)
- **Bağlam:** Upstream PR'ı hazırlanırken, zaman aşımına dayanan iki yol daha bulundu:
  - `Watcher::configure` komutu gönderip yanıtı bekliyor ama sunucuyu uyandırmıyor. crates.io 8.2.1'de çağrı 104 ms sürüyor (yalnızca zaman aşımı sayesinde dönüyor). Zaman aşımı olmayınca kilitlenir. TokenRadar `configure` çağırmıyor.
  - `Drop` önce `Stop` gönderip sonra semaforu bırakıyor. Sunucu `Stop`'u erkenden görürse semaforu kapatıyor ve `Drop` kapanmış bir handle'ı bırakıyor (8.2.1'de zaten var). Upstream #1051 sırayı ters çevirmiş; o sıra da zaman aşımı olmadan `Stop`'un kaybolmasına yol açar.
- **Karar:**
  - Semafor, watcher ve sunucunun ortak sahip olduğu bir `Arc<WakeupSem>` içinde. Son sahip bırakınca kapanıyor. `Drop` önce `Stop` gönderir, sonra uyandırır.
  - `configure` gönderdikten sonra sunucuyu uyandırır.
  - Upstream'e aynı değişiklik 9.0 dalı için önerilir.
- **Doğrulama:** notify'ın test paketi (37 + 5) ve core'un 253 testi geçti. Deneme programı: `configure` 20 µs'de dönüyor; 3 s boşta kaldıktan sonra olay anında geliyor; 200 watcher oluşturup bırakınca handle sayısı değişmiyor ve sunucu thread'leri çıkıyor.
- **Bedel:** Kurulu ve ölçülen build (`11a5748`) ilk yamayı içeriyor. Fark yalnızca `configure` (kullanılmıyor) ve kapanıştaki handle ömrü; boşta ölçüm sonucu geçerli. Temiz makine testi ve tag son build ile yapılır.

### ADR-040 — Toast tıklaması protocol activation ile (`tokenradar://panel`)
**Durum:** Kabul (2026-10-28, Adım 12; ADR-033 ve ADR-036'daki "Bildirim Merkezi'nden tıklama uygulamayı açmaz" bedelinin yerini alır)
- **Bağlam:**
  - Windows, Bildirim Merkezi'ndeki bir toast'a tıklamayı yalnızca COM activator kaydetmiş bir uygulamaya iletir. Ekrandaki toast'a tıklama ise `tauri-plugin-notification` üzerinden çalışıyordu (ADR-033).
  - Eklenti (2.7 → `notify-rust` → `tauri-winrt-notification` 0.8) toast XML'ine `launch` veya `activationType` yazmaya izin vermiyor.
  - Kullanıcı bu kısıtla sürüm çıkarmak yerine tag'den önce düzeltmeyi seçti.
- **Karar:**
  - Windows'ta toast `notify.rs` içinde `windows` 0.62 crate'iyle (`UI_Notifications`, `Data_Xml_Dom`) kurulur. Eklentinin ürettiği toast'la aynıdır (sessiz, `short`, `ToastGeneric`). Tek fark `activationType="protocol"` ve `launch="tokenradar://panel"`. Metin DOM üzerinden yazılır, bu yüzden işaretlemeyi değiştiremez. Gönderen kimliği kuralı eklentininkiyle aynı (cargo build'leri PowerShell AUMID'iyle).
  - Şema `tauri.conf.json > plugins > deep-link > desktop > schemes` ile tanımlanır. Tauri'nin NSIS şablonu (kopyamız dahil) şemayı kurulumda HKCU `Software\Classes\tokenradar` altına yazar ve kaldırmada, komut bu kuruluma aitse siler. `tauri-plugin-deep-link` eklenmez.
  - Tıklama `tokenradar-app.exe tokenradar://panel` başlatır. Uygulama çalışıyorsa single-instance eklentisi bu başlatmayı ona iletir ve panel açılır (her ikinci başlatmada olduğu gibi). Çalışmıyorsa yeni süreç argümanı görür ve paneli açar; argüman yalnızca tam eşleşmeyle kabul edilir.
  - macOS ve Linux'ta eklenti kullanılmaya devam eder.
- **Güvenlik:** Herhangi bir web sayfası veya program `tokenradar://` açabilir. Tek etkisi panelin açılması; başka bir argüman hiçbir şey tetiklemez (soğuk başlatmada yok sayılır, sıcak durumda zaten her ikinci başlatma paneli açıyordu).
- **Bedel:**
  - Portable ve Scoop kopyaları şemayı kaydetmez; orada toast tıklaması hiçbir şey yapmaz (toast zaten görünmeyebilir).
  - Toast kodu Windows'ta eklentiden ayrıldı; eklenti güncellemeleri (ör. ikon, eylem düğmeleri) kendiliğinden gelmez.
- **Alternatifler:**
  - COM activator (`INotificationActivationCallback` + CLSID kaydı + kısayolda `ToastActivatorCLSID`): Tauri şablonunda kısayol özelliği yok; daha fazla registry ve COM sunucusu kodu.
  - `tauri-plugin-deep-link`: yalnızca argüman ayrıştırması için bağımlılık; single-instance zaten yeterli.
  - Kısıtı v0.1'de bırakmak: kullanıcı reddetti.

### ADR-041 — Limit pencereleri hesaba göre opsiyonel; olmayan pencere tahmin edilmez
**Durum:** Kabul (2026-11-07, Adım 12b planlaması)
- **Bağlam:**
  - Anthropic, 30 Ekim 2026'da Max 20x planının adını Ultra olarak değiştirdi ve doğrulanmış Ultra hesaplarında 5 saatlik pencereyi kaldırdı. Pro, Max ve doğrulanmamış Ultra hesaplarında pencere sürüyor. Doğrulama otomatik yapılıyor ve hesap durumu zamanla değişebiliyor.
  - Kullanıcının hesabında 5 Kasım'dan beri statusline payload'unda `five_hour` yok ve `limit_snapshots` tablosunda `session` kaydı oluşmuyor.
  - Plan ve kod her hesapta 5 saatlik pencere varsayıyordu: tray'de çift halka, panelde 5 saatlik kart, statusline'da `5h` segmenti. `render.rs`, payload'da pencere yoksa uygulamanın değerine düşüyor. Limit motoru tahmin üretirse ortaya var olmayan bir limit için `≈%` çıkar. Bu "dürüst sayılar" ilkesine aykırı.
- **Karar:**
  - Her `(hesap, limit türü)` için pencere durumu üç değerlidir: **var**, **bilinmiyor**, **yok**.
  - **Yok** kararı gözlemden çıkar: S2 yanıtında kayıt bulunmaması veya `seven_day` taşıyan art arda N payload'da (başlangıç önerisi 3) `five_hour` bulunmaması. İlk gözlemde karar geri alınır.
  - **Yok** durumundaki pencere için tahmin üretilmez, bildirim tetiklenmez. Tray, panel, CLI ve statusline o pencereyi göstermez.
  - Kural `five_hour`'a özel değildir; `spend_limit` ve ileride gelecek pencereler için de geçerlidir.
- **Bedel:**
  - Durum değişikliğinden sonra ilk birkaç güncellemede pencere "bilinmiyor" görünebilir.
  - Adım 13'te bu hesaplar için `k_session` üretilmez; kalibrasyon haftalık aralıklara dayanır ve yeterli veriye ulaşmak daha uzun sürebilir.
- **Alternatifler:**
  - Plan türünden (`subscriptionType`, `rateLimitTier`) çıkarım: doğrulama durumu bu alanlarda görünmüyor, ayrıca aynı plan içinde iki durum var.
  - 5 saatlik pencereyi `0 %` göstermek: yanlış. Limit dolmamış değil, limit yok.
  - Kullanıcı ayarı ("bu hesapta 5 saat yok"): elle bakım gerektirir ve hesap durumu değiştiğinde bayatlar. Gözlem kuralı yetmezse ek olarak düşünülebilir.
- **Uygulama notu (2026-11-07, Adım 12b, kullanıcı kararlarıyla):**
  - **N = 3, ama yalnızca taze payload'lar sayılır.** Claude Code `resets_at` geçince pencereyi payload'dan düşürüyor ve statusline'ı yeni API yanıtı olmadan da çağırıyor (cache süresi, izin modu, `refreshInterval`). "Art arda 3 payload" kuralı, boşta duran bir Pro/Max hesabını reset'ten sonra yanlışlıkla "yok" yapardı. Bu yüzden bir payload, ancak başka bir limitin okuması (yüzde veya `resets_at`) bir öncekinden farklıysa sayılır; bu, taze bir API yanıtını kanıtlar. Uygulama başladıktan sonraki ilk payload'ın karşılaştıracağı bir şey olmadığı için o sayılmaz.
  - **S2 tam listedir.** `limits[]` şemasında beklenen (`session`, `weekly_all`) ve izlenen limitlerden yanıtta olmayanlar hemen yok sayılır. Düz alan şemasında yalnızca beklenen limitler değerlendirilir. Boş yanıt hiçbir şey kanıtlamaz.
  - **Kalıcılık:** Karar `kv.limits.absent.<account_id>` anahtarında saklanır; yeniden başlatma ve uygulama kapalıyken CLI aynı kararı görür. Kaçırma sayaçları saklanmaz.
  - **Yok olan limitin track'i silinir.** Böylece tahmin, reset, projeksiyon ve bildirim tek noktadan kapanır; geçmiş `limit_snapshots` tablosunda kalır.
  - **IPC:** `StatusReply`/`LimitsReply`'a `#[serde(default)]` ile `absent` eklendi. `PROTOCOL_VERSION` ve `--json` şeması 1'de kaldı (geriye ve ileriye uyumlu ekleme).
  - **Görünüm:** Tray ve panelde haftalık limit iki halkanın bandını dolduran tek, kalın bir halka olur. Boş iç halka "bilinmiyor" demektir, "yok" değil.

### ADR-042 — Statusline v2: segment modeli core'da, `[statusline]` tablosu, sabit renk eşikleri
**Durum:** Kabul (2026-11-07, Adım 12c, kullanıcı kararlarıyla). `[statusline]` tablosu, iki satır düzeni ve küçülme sırası maddeleri ADR-049 ile genişledi.
- **Bağlam:**
  - Statusline satırı v1'de CLI'nin `render.rs` dosyasında sabit bir düzendi: pasta glifi, 5 saatlik ve haftalık pencere, oturum payı.
  - Adım 12c dört şey istedi: bar ve renk, ek bilgi (context, scoped limit, maliyet, model), segment seçimi ve tazelik kuralları.
  - Ayarlar penceresi canlı önizleme göstermeli, ama UI'da hesap yapılmamalı.
- **Karar:**
  - **Satır modeli ve render core'a taşındı** (`tokenradar_core::statusline::line`). CLI satırı Claude Code için, ayarlar penceresi örnek değerlerle önizleme için aynı kodla çizer. Render, boyalı span listesi üretir (`Paint`: `plain`, `dim`, `ok`, `warn`, `crit`). CLI bunu ANSI'ye, UI tema renklerine çevirir.
  - **Her değer kaynağını ve gözlem zamanını taşır.** Uygulamadan gelen değer `FRESH_MS`'den (10 dk) eskiyse soluk gösterilir. Payload değeri her zaman kazanır (v1 kuralı). Uygulamanın tahminleri statusline'da gösterilmez; payload'daki geçmiş reset `≈ 0 %` olur.
  - **`[statusline]` tablosu:** `segments`, `layout` (`single` / `double`) ve `bar` alanlarından oluşur. Okuma hiçbir zaman başarısız olmaz: bilinmeyen segment adı atlanır, boş liste varsayılana döner, bozuk tablo varsayılanları kullanır. Bu tablo, kurulum durumunu tutan `[accounts.statusline]`'dan ayrıdır.
  - **Varsayılan segmentler:** `five_hour`, `seven_day`, `scoped`, `context`, `session_share`, `cost`. Hesapta olmayan pencere (ADR-041) satıra girmez.
  - **İki satır düzeni:** 1. satırda limitler, 2. satırda oturum bilgisi yer alır. Her satırda kullanıcının sırası korunur.
  - **Dar terminalde küçülme sırası:** önce oturum segmentleri (sondan), sonra bar, projeksiyon ve reset sayaçları, en son limit segmentleri (sondan; ilki her zaman kalır). `NO_COLOR` altında bar gösterilmez.
  - **Maliyet etiketi `API $4.12` biçimindedir.** `cost.total_cost_usd` fatura değil, API karşılığı maliyettir.
  - **Oturum payı değeri yoksa gizlenir.** Pay bugün yalnızca 5 saatlik pencereye göre hesaplanıyor. Haftalık pencereye göre pay Adım 14'ün atıf motoruyla gelecek; segment o zaman kendiliğinden görünür.
  - **`model.display_name` yalnızca satır için okunur** ve `skip_serializing` ile işaretlenir. Push ve spool satırının biçimi değişmez.
- **Plandan sapma — renk eşikleri:**
  - Plan "eşikler ayarlardan gelir" diyordu. Uygulamada renk eşiği diye bir ayar yok; ayarlardaki eşikler bildirim eşikleri.
  - Kullanıcı kararıyla statusline, tray ve panelle aynı sabit eşikleri kullanır: `WARN_AT` = 75, `CRIT_AT` = 90.
  - Ayarlanabilir renk eşikleri gerekirse ileride üç yüzey için birlikte ve ayrı bir adımda eklenir.
- **Bedel:**
  - Core, CLI'ye özgü bir görünüm kodu taşıyor. Fluent anahtarları yine `cli.ftl` içinde; uygulama önizleme için `Domain::Cli`'yi yüklüyor.
  - CLI 18 KiB büyüdü. Gecikme değişmedi (`docs/perf/2026-11-07-statusline-v2.md`).
- **Alternatifler:**
  - Önizlemeyi UI'da TypeScript ile çizmek: kuralı iki yerde tutmak ve "UI'da hesap yok" kuralını bozmak demekti.
  - Önizleme için CLI'yi alt süreç olarak çalıştırmak: daha yavaş olur, gerçek pipe'a push gönderme riski taşır.
  - Yalnızca `[statusline]` tablosunu ayrı ayrıştırmak: gereksiz, çünkü CLI config'i zaten okuyor.

### ADR-043 — Claude Code mod'u: tamamlayıcı rol, iki kaynaklı veri yolu, kendi EN/TR tablosu
**Durum:** Kabul (2026-11-07, Adım 12d, kullanıcı kararlarıyla)
- **Bağlam:**
  - Claude Code'un mod (function hooks) API'si, prompt üstüne bant, toast ve slash komutu eklemeye izin veriyor. API erken erişimde ve sürümler arasında değişebilir.
  - Kullanıcı, modun **tamamlayıcı** olmasını seçti: statusline köprüsü birincil veri kaynağı olarak kalır, mod yalnızca görselleştirir ve uyarır.
  - Mod, Claude Code'un yalıtılmış ortamında çalışır: DOM, Node ve `locales/` dosyaları yoktur. Erişim yalnızca `$` üzerinden olur.
- **Karar:**
  - **Konum:** Mod `integrations/claude-code/` altında durur. Rust ve uygulama build'ine bağlı değildir; API değişirse TokenRadar etkilenmez. Geliştirme sırasında `claude --plugin-dir integrations/claude-code` ile yüklenir.
  - **Veri yolu, iki kaynak:**
    - `session.measure` olayı Claude Code'un kendi `rateLimits`, context ve maliyet değerlerini ücretsiz olarak push eder. Bu, statusline payload'ıyla aynı kaynaktır ve bir pencere için her zaman kazanır (`Claude Code` etiketiyle).
    - Projeksiyon, scoped limit, "bu hesapta yok" bilgisi ve oturum payı için `tokenradar status --json` çalıştırılır (`$.process.run`). Mod, kendi oturumunu `live_sessions` içinde `session_uuid` ile bulur; bunun için CLI'ya bayrak gerekmedi.
    - CLI en fazla 30 saniyede bir çağrılır (`session.start`, `session.measure`; `/radar` her seferinde). CLI yoksa, hata verirse veya şemayı tanımıyorsa mod sessizce yalnızca Claude Code değerleriyle çalışır ve `/radar` bunu açıkça söyler.
    - CLI yolu `userConfig.cli` ile ayarlanabilir (varsayılan PATH'teki `tokenradar`). Portable kurulum ve geliştirme sırasında yeni build'i denemek için gereklidir.
  - **Eşikler:** Tray ve statusline ile aynı sabit değerler kullanılır: `WARN_AT` = 75, `CRIT_AT` = 90 (ADR-042). Reset'ten önce %100'e ulaşma projeksiyonu uyarı seviyesi sayılır. Reset'i geçmiş pencere `≈ 0 %` olur ve uyarı üretmez.
  - **Bant:** Bant, yalnızca bir eşik aşıldığında en acil üç satırı gösterir. "Gizle" düğmesi bandı mevcut seviyeye kadar gizler; seviye yükselince bant geri gelir. Bütün pencereler eşiğin altına inince (reset) gizleme sıfırlanır.
  - **Toast ve çift bildirim kuralı:** Toast, seviye her yükseldiğinde bir kez çıkar. CLI `origin: "app"` dönüyorsa masaüstü uygulaması kendi bildirimini gönderdiği için mod toast göstermez. Uygulama kapalıysa veya CLI yoksa toast mod'dan gelir.
  - **`/radar`:** Her limit için kaynak, reset ve projeksiyon gösterilir. Hesapta olmayan pencereler, oturum payı, context ve maliyet de dökümde yer alır. Son satırda veri kaynağı (uygulama, veritabanı veya yalnızca Claude Code) ve gerekiyorsa `≈` notu bulunur. Komut çıktısı Claude Code'da modelin bağlamına da girer; içinde yalnızca limit sayıları vardır, konuşma içeriği yoktur.
  - **Metinler:** Mod, kendi küçük EN/TR tablosunu taşır (`hooks/i18n.ts`). İfadeler `cli.ftl`'den, sayı ve süre biçimleri `tokenradar-i18n/format.rs`'den kopyalanır. Dil, CLI'ın `status --json` çıktısına eklenen `locale` alanından (`en`, `tr`) gelir. Bu alan yalnızca CLI çıktısındadır; IPC yanıtı değişmedi. CLI yoksa veya eski sürümse dil İngilizce olur.
  - **İlkeler:** `prompt.submit`, `prompt.compose` ve tool olaylarına hook eklenmez. Mod ağ çağrısı yapmaz ve kimlik bilgisine dokunmaz. `$.state` sözleşmesi `types/index.d.ts` içindedir ve her değer bir `shape` etiketi taşır.
  - **Pane yapılmadı.** Statusline zaten sürekli görünür, `/radar` da ayrıntıyı verir. Pane ise 144 sütunun altında kendiliğinden açılmaz.
  - **Dağıtım:** Bu adımda yalnızca repo içinde, `--plugin-dir` ile. Marketplace yayını (`marketplace.json`) dışa dönük bir adımdır ve kullanıcının onayını bekler.
- **Bedel:**
  - EN/TR metinler ve biçimlendiriciler üçüncü bir yerde (Rust ve TS'den sonra) duruyor. Mod testleri ortak test vektörlerini (`testdata/i18n-vectors.json`) okuyamıyor, çünkü test ortamında dosya erişimi yok. Aynı örnekler testlere elle yazıldı.
  - Kurulu v0.1.0 CLI'ında `locale` ve `absent` yok. 0.1.1 kurulana kadar mod İngilizce konuşur ve "bu hesapta yok" satırını göstermez.
- **Alternatifler:**
  - Tüm metni CLI'dan almak (`tokenradar status` insan çıktısı): bant ve toast TokenRadar yokken de çalışmalı, bu yüzden tablo yine gerekirdi.
  - Mod'a Fluent çalıştırıcısı gömmek: mod büyür ve `locales/` dosyaları modla birlikte dağıtılmak zorunda kalır.
  - CLI'a `--session <uuid>` eklemek: gereksiz, çünkü oturum zaten `live_sessions` içinde.
  - Statusline köprüsünü modla değiştirmek: kullanıcı tamamlayıcı rolü seçti. Ayrıca mod yalnızca Claude Code açıkken çalışır ve spool'a yazamaz.

### ADR-044 — Zaman temposu: statusline'da zaman işareti, mod'da tempo uyarısı ve zaman çizgisi
**Durum:** Kabul (2026-11-07, Adım 12e, kullanıcı kararlarıyla). Tempo uyarısının ne zaman çıktığı ADR-045 ile değişti.
- **Bağlam:**
  - Kullanıcı kendi "Kota Zaman Çizelgesi" sayfasını örnek gösterdi. Sayfa, haftalık pencerenin ne kadarının geçtiğini yüzde olarak gösteriyor ve kullanımı bununla karşılaştırıyor.
  - "Kullanım zamanın önünde mi?" sorusunun cevabı, projeksiyondan (ADR-042, bu hızla dolar ya da dolmaz) farklı bir bilgi. Projeksiyon bir tahmin. Geçen zaman ise saatten okunan bir ölçüm.
- **Karar:**
  - **Geçen zaman bir ölçümdür:** Değer `(1 − kalan / pencere uzunluğu) × 100` ile hesaplanır. Pencere uzunluğu 5 saat (`session`) ya da 7 gündür (`weekly_all`, `weekly_scoped`). Kalan süre pencere uzunluğunu aşıyorsa ya da reset geçmişse değer hesaplanmaz. `≈` işareti almaz.
  - **Statusline:** Bar içine, geçen zamanın düştüğü hücre sınırına bir `│` işareti konur ve yüzdeden sonra `⏱43%` metni gelir. Örnek: `7d ███│█░░░░ 41% ⏱43% · 3d4h`. İşaret renksizdir (Plain). Pencere sönükse metin de sönük olur. Renk kapalıyken metin kalır. Dar terminalde, bar düşürüldükten sonra ilk düşen ayrıntı budur.
  - **Ayar:** `[statusline] pace = true` (varsayılan açık). Ayarlar penceresinde "Zaman işareti" anahtarı vardır.
  - **Mod, tempo uyarısı:** Kullanım zaman çizgisinin en az 10 puan önündeyse (`PACE_AHEAD`) ve hiçbir eşik aşılmamışsa, bant uyarı seviyesinde bir satır gösterir. Bu uyarı **toast üretmez**: bir eşik değil, bir eğilimdir. Tahmini (`≈`) değerler zamanla karşılaştırılmaz. "Gizle" bu uyarıyı da gizler. Tempo uyarısı sürdüğü sürece gizleme sıfırlanmaz. `stepAlerts` bant seviyesini ve toast seviyesini ayrı ayrı alır.
  - **Mod, `/radar`:** Haftalık pencere için (yoksa 5 saatlik pencere için) bir tempo satırı eklenir: geçen zaman, kullanım ve farkın puan ve süre olarak karşılığı. Haftalık pencerede bunu "Zaman çizgisi" izler: pencerenin kalan her gününün sonunda zaman çizgisinin kaçta olduğu, yerel saatle (`Cmt 17:00 %71,4 · … · Pzt 17:00 sıfırlanma`). Yerel saat farkı mod'un çalıştığı makineden alınır.
- **Bedel:**
  - `⏱` bazı terminallerde iki hücre genişliğinde çizilebilir. `fit()` bunu bir hücre sayar. Bu durumda satır birkaç sütun taşabilir.
  - Statusline'ın varsayılan çıktısı değişti. Eski görünümü isteyen `pace = false` yazar.
- **Alternatifler:**
  - Tempo için ayrı bir statusline segmenti açmak: satır uzar. Bar üstündeki işaret, aynı bilgiyi yer harcamadan verir.
  - Tempo uyarısında toast göstermek: hafta başında küçük sapmalar sık bildirim üretirdi. Bant yeterli.
  - Zaman çizgisini grafik olarak çizmek: mod'un görsel yüzeyleri Adım 12f'de ele alınır.

### ADR-045 — Tempo uyarısı pencerenin ilk yedide birinde sessiz
**Durum:** Kabul (2026-11-07, Adım 12e düzeltmesi, kullanıcı kararıyla). ADR-044'ün tempo uyarısı maddesini değiştirir.
- **Bağlam:** Reset'ten hemen sonra zaman çizgisi %0'a yakındır. Örneğin reset'ten 2 saat sonra zaman %1,2'dedir. O sırada sıradan bir oturum %12 kullanım getirirse fark 10 puanı aşar ve bant açılır. Bu yüzden uyarılar her hafta başında yığılıyordu.
- **Karar:** Pencerenin ilk yedide biri geçene kadar tempo uyarısı çıkmaz (`PACE_QUIET` = 100/7 ≈ %14,3). Bu süre haftalık pencerede bir gün, 5 saatlik pencerede yaklaşık 43 dakikadır. Bu sürenin ardından ADR-044'teki kural (en az 10 puan önde, tahmin değil, toast yok) aynen geçerlidir. `/radar` tempo satırı ve statusline'daki `⏱` her zaman görünür, çünkü bunlar uyarı değil, bilgidir.
- **Bedel:** İlk gün gerçekten hızlı tüketen biri tempo uyarısı görmez. Bu durumu 75/90 eşikleri ve reset'ten önce dolma projeksiyonu yine yakalar.
- **Alternatifler:**
  - Zamanla daralan eşik, örneğin `max(10, 30 − geçen%)`: daha yumuşak, ama kuralı anlatmak zor.
  - Uyarıyı yalnızca projeksiyon da varsa çıkarmak: projeksiyon uyarısı zaten bantta çıktığı için tempo uyarısı gereksizleşir.
  - Tempo bandını kaldırmak.

### ADR-046 — Görsel mod: panel, tek satırlık bant, geçmiş için `tokenradar history`
**Durum:** Kabul (2026-11-07, Adım 12f, kullanıcı kararlarıyla). ADR-043'ün bant ve `/radar` maddelerini değiştirir.
- **Bağlam:**
  - Adım 12d'nin modu yalnızca metin kullanıyordu. Bant renkli satırlardan oluşuyordu. `/radar` uzun bir metin döndürüyordu ve bu metin modelin bağlamına da giriyordu.
  - Mod API'sinin görsel yüzeyleri var: `Pane`, kenarlıklı ve renkli `Box`/`Text`, terminalde `Raster` (renkli hücre ızgarası), masaüstü, VS Code ve mobilde `Svg`, ayrıca `$.ui.status`.
  - Kullanım eğrisi için geçmiş veri gerekiyor. Bu veri yalnızca TokenRadar'ın veritabanında (`limit_snapshots`) var.
- **Karar:**
  - **`/radar` bir panel açar.** Panelde sırasıyla şunlar bulunur:
    - **Limit kartları:** Her kartta renkli bar, zaman işareti (`│`, ADR-044), değer, kaynak rozeti, reset süresi, projeksiyon ve tazelik bulunur. Eski ya da tahmini değerler soluk görünür; tahmin barı `▒` ile çizilir. Geçen zamanın yüzdesi (`⏱`) kartlarda yazılmaz: bazı fontlarda çift genişlikte çiziliyor; değer bir kez, zaman şeridinin başlığında yazılır.
    - **Haftalık zaman şeridi:** İki görünüm aynı şeritte. Üstte takvim günleri: gün adları ve gece yarısındaki gün sınırları (`┊`). Pencere reset saatinde başlayıp bittiği için ilk ve son gün kısmidir; ilk gün, yer varsa reset saatini de yazar. Ortada kullanım dolgusu. Reset'ten itibaren her 24 saatte şeritte düz bir sınır (`│`); şeridin hemen altındaki tek satırda zaman çizgisinin oradaki değeri (%14, %29 …) ve "▲ şimdi" işareti. İşarete değen değerler yer açar. Grafiğin altında açıklama: reset'in günü ve saati, işaretlerin anlamı ve grafiğin anahtarı. Gün sınırları yerel saat dilimine göre çizilir (sabit ofset; pencere içinde yaz saati değişirse sonraki sınırlar bir saat kayar). Grafiğin gün çizgileri de gece yarısındadır.
    - **Kullanım grafiği:** Masaüstü, VS Code ve mobilde `Svg`, terminalde `Raster` ile çizilir. Grafikte kullanım eğrisi, kesikli zaman çizgisi (terminalde barlar gibi: şimdiye kadar geçen zaman gri sekizlik bloklarla, kullanım önünde renkli), 75/90 eşikleri, gün çizgileri ve "şimdi" çizgisi bulunur.
    - **Canlı oturumlar:** Her oturum için küçük bir bar ve API maliyeti. Bar yalnızca karşılaştıracak bir şey varken çizilir: ≈pay biliniyorsa ya da en az iki oturumun maliyeti varsa. Bar, TokenRadar ≈payı biliyorsa o payı (tahmin, `▒`), bilmiyorsa oturumun canlı oturumların toplam maliyetindeki payını gösterir. Bu oturum ilk sırada yer alır.
    - **Bağlam doluluğu barı** ve oturum maliyeti.
    - **Veri kaynağı satırı.**
  - Aynı Box/Text ağacı dört yüzeyde de çizilir. Yalnızca grafik yüzeye göre değişir.
  - **`/radar` metni kısa:** Yanıt "panel açıldı" ve limit başına tek bir özet satırıdır. Panel yerleştirilemezse (`isPlaced: false`) eski tam döküm döner.
  - **Bant tek satırdır:**
    - Kenarlıklı bir kutu, uyarının renginde.
    - İçinde mini bar (10 hücre, zaman işaretiyle) ve bant metni. Birden fazla uyarı varsa `+n` eklenir.
    - "Ayrıntı" düğmesi paneli açar, "Gizle" düğmesi ADR-043'teki gibi çalışır.
  - **Reset satırı:** Mod, bir pencerenin reset anını önceden görmüşse ve o an geçmişse, bantta bir kez "✓ Haftalık limit sıfırlandı" satırı gösterir. Satır "Tamam" ile ya da reset'ten bir saat sonra kapanır. Reset'ten sonra ilk kez görülen bir pencere için satır çıkmaz. Bu satır toast üretmez.
  - **`$.ui.status`:**
    - Mod, Claude Code'un `settings.json` dosyasını (`CLAUDE_CONFIG_DIR`, yoksa `~/.claude`) salt-okunur okur ve yalnızca `statusLine.command` alanına bakar.
    - Komutta `tokenradar` geçiyorsa durum girdisi gösterilmez, böylece aynı bilgi iki kez görünmez.
    - Geçmiyorsa kısa bir girdi gösterilir: `TokenRadar · Haftalık %42 ⏱%43`.
    - Dosyanın geri kalanı kullanılmaz ve saklanmaz.
  - **Geçmiş verisi, `tokenradar history --json`:**
    - CLI'a yeni bir salt-okunur komut eklendi. Komut, veritabanını uygulama çalışırken de WAL sayesinde salt-okunur açar. IPC'ye dokunulmaz.
    - Her limit için, en son okuması geleceğe dönük bir reset taşıyorsa, o pencerenin okumalarını verir. Okumalar aynı pencereye aittir (`resets_at` farkı 5 dakika toleransın içindedir), pencerenin başlangıcından bugüne kadardır ve pencerenin 168 eşit dilimi başına sonuncusu alınır. Haftalık pencerede bu saatte bir nokta, 5 saatlik pencerede yaklaşık 107 saniyede bir nokta demektir.
    - `absent` pencereler dışarıda kalır. Tahminler zaten saklanmadığı için çıktıda yoktur, her nokta bir ölçümdür ve kaynağını taşır. Şema `{schema: 1, generated_at, series: [{kind, scope, window_start, resets_at, points: [{at, percent, source}]}]}` biçimindedir.
    - Mod bu komutu yalnızca panel açıkken ve en fazla 5 dakikada bir çalıştırır. Eski bir CLI'da komut yoktur; o durumda grafik çizilmez, panelin geri kalanı çalışır.
  - **Renkler:**
    - Metinde uyarı ve kritik durumlar tema anahtarlarıyla (`warning`, `error`) gösterilir, iz rengi `subtle`'dır. Normal durum terminalde statusline'ın cyan rengini, uzak yüzeylerde tasarımın orta açıklıktaki camgöbeği tonunu (`#00afaf`) kullanır.
    - `Svg`, design-language'in açık ve koyu token'larını `prefers-color-scheme` ile seçer.
    - `Raster`'da tema bilgisi yoktur, bu yüzden her iki zeminde de okunan orta açıklık tonları kullanılır: ok `#00afaf`, warn `#dd9314`, crit `#e54c4a`, iz `#7c8186`.
  - **Gizlilik:** Canlı oturumlar adlarıyla, yoksa proje klasörünün adıyla gösterilir. Oturumun `title` alanı okunmaz, çünkü Claude Code bu alanı konuşmadan üretebilir.
  - **Animasyon yok (ADR-019):** Panel yalnızca veri değiştiğinde yeniden çizilir. Zamanlayıcı ve SMIL kullanılmaz.
- **Bedel:**
  - Mod artık Claude Code'un ayar dosyasını okuyor. Dosyada başka araçların `env` değerleri de bulunabilir. Mod dosyanın tamamını belleğe ayrıştırır, ama yalnızca tek bir alanı kullanır ve hiçbir şey saklamaz.
  - `Svg` uzak yüzeylerde resim olarak çizilir. Görüntüleyicinin renk şemasını izleyip izlemediği yüzeye bağlıdır ve bu adımda elle denenmedi.
  - Bant kenarlıklı olduğu için terminalde üç satır yer kaplar; içerik tek satırdır.
  - Panelin terminal dışındaki yüzeylerde görünümü yalnızca testlerde, dört yüzeyle doğrulandı.
- **Alternatifler:**
  - Mod'un kendi geçmişini `$.store`'da tutması: CLI değişmezdi, ama yalnızca Claude Code açıkken toplanan noktalar olurdu ve eğri boşluklu çıkardı.
  - Geçmişi IPC'ye `history.get` olarak eklemek: uygulama kapalıyken çalışmaz ve protokol büyür. Veritabanından salt-okunur okumak her iki durumda da çalışır.
  - Terminalde grafiği yalnızca metinle çizmek: `Raster`, sekizlik bloklarla 6 satırda 48 düzey çözünürlük veriyor.
  - `$.ui.status`'ı her zaman ya da ayarla göstermek: kullanıcı, köprü varsa susan girdiyi seçti.

### ADR-047 — Claude Code mod'u TokenRadar'dan kurulur: `env.CLAUDE_CODE_PLUGIN_DIRS`
**Durum:** Kabul (2026-11-07, Adım 12g, kullanıcı kararlarıyla). ADR-043'ün "mod repodan `--plugin-dir` ile yüklenir" varsayımını değiştirir.
- **Bağlam:**
  - Mod yalnızca bir repo klonundan, `claude --plugin-dir integrations/claude-code` ile yükleniyordu. Kullanıcı, statusline köprüsü gibi modun da ilk kullanımda ve ayarlarda kurulup kaldırılabilmesini istedi.
  - Claude Code, `settings.json` dosyasının `env` bloğundaki `CLAUDE_CODE_PLUGIN_DIRS` değişkenini okur ve listedeki her klasörü `--plugin-dir` gibi yükler (Windows'ta `;`, diğer sistemlerde `:` ile ayrılır). Seçenekleri `pluginConfigs."<ad>@inline"` altında durur.
- **Karar:**
  - **Mod kurulum paketine girer:** `<kurulum>\claude-code` altında `plugin.json`, `hooks/` ve `types/index.d.ts`. Testler girmez. Uygulama güncellenince mod da güncellenir.
  - **Kurulum `settings.json`'a tek bir yol ekler:** Birincil hesabın `settings.json` dosyasında `env.CLAUDE_CODE_PLUGIN_DIRS` listesine mod klasörü eklenir. Listede başka yollar varsa korunur; `env` yoksa oluşturulur. Değer string değilse dosyaya dokunulmaz (`invalid_settings`).
  - **Kurallar statusline'ınkiyle aynıdır (ADR-009, ADR-029):** diff → onay → yedek → atomik yazma. Yalnızca `env` üyesinin baytları değişir; `install → uninstall` dosyayı bayt düzeyinde geri getirir. Plan gösterildikten sonra dosya değişmişse yazılmaz.
  - **Kaldırma durumsuzdur:** `config.toml`'da kayıt tutulmaz. Kaldırma listeden yalnızca bu kurulumun mod klasörünü siler. Liste boşalırsa anahtar, `env` boşalırsa `env` de silinir.
  - **Yüzeyler:**
    - CLI: `tokenradar mod install|uninstall [--yes] [--config-dir <DİZİN>]`; `uninstall --all` TokenRadar'ın bildiği her Claude Code dizinine bakar.
    - Uygulama: İlk kullanımın limit adımında statusline kartının altında ayrı bir kart; varsayılan olarak hiçbir şey kurulmaz, kullanıcı düğmeye basar. Ayarlar → Veri kaynakları'nda `mod` bölümü. İkisi de statusline'ın diff diyaloğunu kullanır.
    - Kaldırıcı: Kaldırırken (güncellemede değil) `mod uninstall --all --yes` çalışır ve `claude-code` klasörünü siler; Claude Code bu klasöre üretilmiş tipler yazdığı için dosya listesi onu tamamen silemezdi.
  - **Mod CLI'ı kendisi bulur:** `TokenRadar CLI` seçeneği varsayılanda (`tokenradar`) kalırsa mod, kendi klasörünün yanındaki `..\bin\tokenradar.exe`'yi kullanır; yoksa PATH'teki `tokenradar`'ı. Kullanıcının elle girdiği yol her zaman önce gelir.
- **Bedel:**
  - Önceden yalnızca `env: {}` taşıyan bir dosyada, kaldırma sonrasında boş `env` de silinir; bayt düzeyinde geri dönüş yalnızca TokenRadar'ın kendi eklediği `env` için geçerlidir.
  - Mod aynı anda hem listeden hem `--plugin-dir` ile yüklenirse iki kez yüklenebilir. TokenRadar bunu algılayamaz; diyalog ve CLI çıktısı uyarır.
  - `pluginConfigs` içindeki eski bir CLI yolu otomatik bulmayı geçersiz kılar. TokenRadar kullanıcının `pluginConfigs` ayarlarına dokunmaz.
  - Mod, kurulum klasörünün yolunu `settings.json`'a yazar. Kurulum klasörü taşınırsa mod yüklenmez; yeniden kurmak gerekir.
- **Alternatifler:**
  - Yerel bir marketplace ve `claude plugin install`: Claude Code'un kendi önbelleğine kopyalar, güncellemede eski sürüm kalır ve kaldırma için Claude Code'un komutu gerekir.
  - Mod'u `~/.claude/plugins` altına kopyalamak: Claude Code'un iç düzenine bağlanır ve güncellemede kopyalar senkron tutulmalıdır.
  - Kurulumda modu varsayılan olarak açmak: Kullanıcı, deneysel modun açıkça seçilmesini istedi.

### ADR-048 — Tray panelinde ikinci görünüm: "Ayrıntı"
**Durum:** Kabul (2026-11-07, Adım 12h, kullanıcı kararlarıyla).
- **Bağlam:**
  - Mod'un `/radar` paneli (ADR-046) uygulamada olmayan ayrıntılar gösteriyor: zaman işareti ve tempo, haftalık zaman şeridi, kullanım grafiği, limit başına kaynak rozeti, oturum maliyeti ve pay barı. Kullanıcı bunları uygulamada da görmek istedi.
  - Tray paneli 360 px genişliğinde ve kısa. Hepsini mevcut görünüme eklemek paneli uzatırdı.
- **Karar:**
  - **İki görünüm:** Panel başlığının altında `Özet | Ayrıntı` geçişi var (`role="tablist"`, ←/→, `Ctrl+Tab`). Özet aynı kalır; tek eki limit barlarındaki zaman işaretidir. Son seçilen görünüm `[general] panel_view` olarak saklanır ve panel o görünümle açılır. Limit yokken geçiş görünmez.
  - **Ayrıntı görünümü,** mod'un panelinin sırasıyla:
    - Limit kartları: bar ve zaman işareti, limitin kendi kaynak rozeti, reset, projeksiyon ve tempo satırı ("Geçen zaman %54 · zaman çizgisinin 10 puan önünde"). Tempo, ADR-044/045 kuralıyla uyarı rengi alır; tahminlerde tempo yoktur.
    - Haftalık zaman şeridi: takvim günleri, yerel gece yarısında noktalı sınır, reset'ten itibaren her 24 saatte düz sınır ve altında zaman çizgisinin değeri, "▲ şimdi". İlk günün reset saati gün genişliğine sığmadığı için açıklamada yazılır.
    - Kullanım grafiği: satır içi SVG, tema token'larıyla. Kullanım eğrisi ve dolgusu, kesikli zaman çizgisi, 75/90 eşikleri, gün çizgileri (5 saatlik pencerede saat çizgileri) ve "şimdi" çizgisi. Kayıtlı okuma yoksa kısa bir not gösterilir.
    - Oturumlar: maliyet ("API $2,57", fatura değil) ve küçük bar. Bar kuralı mod'dakiyle aynıdır: ≈pay biliniyorsa kesikli çizilen pay, bilinmiyorsa en az iki oturumun maliyeti varsa maliyet payı.
    - Bağlam doluluğu barı gelmez. Mod bu değeri yalnızca içinde çalıştığı oturumdan bilir; uygulamada tek bir "o anki oturum" yok.
  - **Veri:**
    - `LimitView.window_start`: Zaman işareti, geri sayım gibi saatten konumlanır: `(şimdi − başlangıç) / (reset − başlangıç)`. Bu sayede her dakika `LimitsUpdated` gönderilmez; alan `VisibleKey`'e girmez.
    - `get_panel_detail() -> PanelDetail`: limit başına tempo (`limits::pace`, statusline ile ortak) ve odak limitin (haftalık, yoksa 5 saatlik pencere) 24 saatlik işaretleri ile okumaları. Okumalar `tokenradar history` gibi veritabanından salt-okunur alınır (`history::window_readings` ortak). Son nokta panelin güncel değeridir; tahmin hiçbir zaman çizilmez.
    - Ayrıntı görünümü açıkken veri dakikada bir ve her `limits_updated` olayında (en fazla 5 saniyede bir) yeniden alınır. Panel gizliyken hiçbir istek gitmez.
  - **Yerel gece yarıları frontend'de hesaplanır:** Gün sınırları, yanlarındaki gün adları gibi takvim sunumudur. `Date`, sistem saat dilimini ve pencere içindeki yaz saati değişimini doğru uygular; mod'un sabit ofset sınırlaması burada yoktur. Plan bunu backend'de `jiff` ile öngörüyordu; core'a saat dilimi veritabanı eklemek CLI'ın 3 MB'lık boyut bütçesini zorlardı (2,21 MiB).
  - **Bundle:** Ayrıntı dinamik `import()` ile ayrı bir parçadır. Grafik kütüphanesi kullanılmaz; uPlot ana pencereye (Adım 15) kalır.
  - **Animasyon yok (ADR-019):** Geçiş anlıktır; zaman işareti 1 Hz saatle yer değiştirir, geçiş efekti yoktur.
- **Bedel:**
  - Panel sayfasının ilk yükü 39,2 KB'tan 42,8 KB'a çıktı (gzip JS, `scripts/check-bundle-size.mjs`); artışın çoğu yeni metinler. Ayrıntı ayrı bir parça: 3,6 KB JS + 1,2 KB CSS.
  - Ayrıntı görünümü gerçek panelde 866 px (üç limit, üç canlı oturum); panel en fazla 600 px (design-language §5) olduğu için içerik panelin içinde kayar. Başlık ve görünüm geçişi de kayar; geri dönmek için yukarı kaydırmak ya da `Ctrl+Tab` gerekir. Kaydırma çubuğu ince (`scrollbar-width: thin`).
  - Gerçek panel denemesi, Adım 10'dan beri gizli bir hatayı ortaya çıkardı: `#app`'in yüksekliği olmadığı için `main` hiç kaymıyor, uzun içerik kesiliyordu. Düzeltildi.
  - Odak limit dışındaki limitlerin (5 saatlik pencere, Fable) grafiği yok; kartlarında tempo satırı var.
- **Alternatifler:**
  - Mevcut panele açılır bir bölüm: kullanıcı ikinci bir görünümü seçti.
  - Ayrı bir rapor penceresi: ikinci bir WebView'in soğuk açılışını öder ve Adım 15'in ana penceresiyle çakışırdı.
  - Görünüm tercihini WebView'in `localStorage`'ında tutmak: ayarlar `config.toml`'da toplanıyor; WebView verisi silinince tercih de kaybolurdu.

### ADR-049 — Statusline v3: öğe modeli, biçim ve renk, Powerline, şablonlar
**Durum:** Kabul (2026-11-08, Adım 12i, kullanıcı kararlarıyla). ADR-042'yi genişletir. Ayraç öğeleri, öğe sınırı ve düzen ADR-051 ile genişledi.
- **Bağlam:**
  - Kullanıcı, statusline'ın `tylergraydev/claude-code-tool-manager` içindeki "Status Line Builder" kadar ayrıntılı özelleştirilebilmesini istedi: segment başına biçim, etiket ve renk; serbest düzen; daha fazla segment; Powerline teması.
  - Referans proje segment listesinden bir Python script üretip `~/.claude/statusline.py`'ye yazıyor ve bir topluluk galerisi sunuyor.
  - Renk özelleştirmesinin anlamlı olup olmadığı, Claude Code'un komuta `NO_COLOR` verip vermediğine bağlıydı (12c'nin açık konusu).
- **Karar:**
  - **Öğe modeli:** `[statusline]` bir öğe listesi taşır (`[[statusline.items]]`: `kind`, `enabled`, `format`, `label`, `color`, `reset`, `pace`, `text`). 22 tür vardır; düzen öğeleri (`separator`, `line_break`, `custom_text`) birden çok kez kullanılabilir, değer öğeleri bir kez. Liste en fazla 48 öğedir.
  - **Geriye uyum:** `items` yoksa öğeler 12c'nin `segments`, `layout`, `bar` ve `pace` alanlarından türetilir; `double` düzen, limitlerden sonra bir `line_break` olur. 12c'nin render testleri değişmeden geçer. Kaydederken bu dört anahtar öğelerden türetilip yazılmaya devam eder; böylece eski bir CLI da anlamlı bir satır çizer.
  - **Esnek okuma (ADR-042 kuralı):** Bilinmeyen tür, biçim ya da renk atlanır veya varsayılana döner; yanlış tipteki değer yok sayılır. Okuma hiçbir zaman başarısız olmaz.
  - **Temizlik:** `label` (16), `text` (32) ve `separator` (3 karakter) içindeki kontrol karakterleri (C0, ESC, C1) atılır. Kullanıcı metni ANSI kodu enjekte edemez.
  - **Renk kuralı (kullanıcı kararı):** Limit ve context öğelerinde kullanıcı yalnızca "ok" rengini seçer. Uyarı (75) ve kritik (90) renkleri, soluk çizilen tahmin ve bayat değerler her zaman kazanır. Palet 16 ANSI rengidir; 256 renk ya da hex yoktur.
  - **Powerline:** `powerline` (U+E0B0) ve `powerline_round` (U+E0B4) temaları arka plan bloklarıyla çizer. Limit bloğu uyarıda sarıya, kritikte kırmızıya döner. Aynı renkteki iki blok ince glifle birleşir. Blok içinde bar çizilmez, çünkü dolu ve boş hücreler aynı yazı rengini alır. Soluk yazı, SGR 2 siyah yazıda görünmediği için gri çizilir. Renk kapalıyken plain temaya düşülür.
  - **Renk modu:** `color = "auto"` varsayılandır ve `NO_COLOR`'a uyar; `always` ve `never` de vardır. Gözleme göre Claude Code komuta `NO_COLOR` vermiyor ve ANSI kodlarını çiziyor (`docs/data-sources.md` S1).
  - **Git dalı (kullanıcı kararı):** Yalnızca dal adı gösterilir. Önce payload'daki `worktree.branch` kullanılır; yoksa `.git/HEAD` doğrudan okunur (worktree'de `gitdir:` izlenir, detached HEAD'de 7 karakterlik SHA). Git süreci çalıştırılmaz. Dosya yalnızca öğe açıksa okunur.
  - **Galeri yok (ADR-013, kullanıcı kararı):** Yerleşik beş şablon (`default`, `minimal`, `detailed`, `two_line`, `powerline`) ve `[statusline]` TOML metniyle dışa ve içe aktarma. İçe aktarma kaydetmez; yok sayılan anahtarları ve türleri listeler.
  - **Script üretilmez:** Render core'da kalır (ADR-042). Python bağımlılığı yoktur ve `settings.json`'a ek yazma gerekmez.
  - **`API` işareti kaldırılamaz:** Maliyetin etiketi değiştirilebilir, ama `API` işareti kalır (ADR-042: değer fatura değil).
  - **IPC:** `preview_statusline(prefs, state, width)` örnek değerleri dört durumda (`normal`, `warn`, `crit`, `stale`) ve istenen genişlikte çizer. Span'ler fg/bg ve kaynak öğe indeksini taşır. `statusline_catalog`, `list_statusline_presets`, `export_statusline` ve `import_statusline` yeni komutlardır.
- **Bedel:**
  - CLI 63,5 KiB büyüdü (2,24 → 2,30 MiB, bütçe 3 MB). Gecikme ölçülebilir biçimde değişmedi; Detailed şablonu ve `.git/HEAD` okuması dahil (`docs/perf/2026-11-08-statusline-v3.md`).
  - Powerline glifleri Nerd/Powerline fontu ister; Consolas gibi fontlarda kutu görünür.
  - Powerline bloğunda projeksiyon işareti `▲` sarı rengini kaybeder; blok rengi yalnızca 75/90 eşiklerine göre değişir. İşaret görünür kalır.
  - `config.toml`'daki `[statusline]` tablosu uzadı; öğeler yalnızca varsayılandan farklı alanlarıyla yazılır.
- **Alternatifler:**
  - Referans gibi Python script üretmek: Python bağımlılığı, ikinci bir render kodu ve her değişiklikte dosya yazma demekti.
  - Topluluk galerisi: ağ allowlist'ini (ADR-013) genişletirdi.
  - `git status` ile kirli/temiz bilgisi: her çağrıda süreç başlatmak p95 bütçesini zorlardı.
  - 256 renk veya hex: terminal temasından bağımsız renkler, açık ve koyu temada okunabilirliği garanti etmez.

### ADR-050 — Statusline düzenleyicisi: ayrı sayfa, taslak ve otomatik kayıt, terminal paleti
**Durum:** Kabul (2026-11-08, Adım 12j, kullanıcı kararlarıyla)
- **Bağlam:**
  - 12c'nin gömülü görünüm alanı, 12i'nin öğe modelini (biçim, etiket, renk, düzen öğeleri, tema) taşıyamıyordu.
  - Kullanıcı ayrı bir sayfa, otomatik kayıt, geri alma ve varsayılana dönme istedi (2026-11-07).
- **Karar:**
  - **Ayrı "Statusline" sekmesi.** Kurulum durumu ve diff akışı Veri kaynakları'nda kalır. Orada yalnızca "Görünümü düzenle →" düğmesi bulunur.
  - **Taslak düzenleyicinin doğruluk kaynağıdır.** Her değişiklik önce taslağa girer ve çekirdek onu hemen çizer (`preview_statusline`). Kayıt 400 ms debounce ile yapılır ve aynı anda tek bir kayıt çalışır (`createSaver`). Başka yerden gelen değişiklik (`prefs-changed`) yalnızca bekleyen kayıt yokken taslağın yerine geçer; böylece kaydın yankısı yazılan etiketi ezmez.
  - **Geri alma yığını pencere ömrü boyunca tutulur**, diske yazılmaz ve en fazla 100 adımdır. Aynı alanın 1 sn içindeki düzenlemeleri tek adımda birleşir.
  - **Şablon, varsayılana dönme ve içe aktarma** uygulanmadan önce çekirdeğin çizdiği önizlemeyle onay ister.
  - **Terminal paleti uygulama temasından bağımsızdır.** Windows Terminal'in varsayılan 16 ANSI rengi kullanılır. Böylece önizleme, kullanıcının çoğunun gördüğü renklere yakın olur; koyu zeminde okunmayan mavi de burada okunmaz.
  - **Ayarlar penceresinin varsayılan boyutu 800×600'den 960×680'e çıktı** (en küçük boyut 640×480 kaldı). Kullanıcı ilk düzenin ekran görüntüsünde onayladı. 800 px'te araç çubuğu dağılıyor, satırlarda etiketler simgelerin üstüne taşıyordu. Dar pencerede sütunlar alt alta geçer.
  - **↑/↓ düğmeleri seçili öğenin başlığındadır**, satırlarda değil. Satırlarda tutamaç, gizle ve kaldır bulunur. Klavyede satır üzerinde Alt+↑/↓ taşır, ↑/↓ satırlar arasında gezer, `Delete` kaldırır.
- **Bedel:** Çekirdeğin `normalized` kuralları (son görünür değer öğesi, tekil değer öğeleri, 48 öğe sınırı) UI'da da tekrarlanıyor. Bu tekrar yalnızca çekirdeğin atacağı seçenekleri sunmamak içindir; kaydedilen değeri yine çekirdek normalize eder.
- **Alternatifler:**
  - Açık "Kaydet" düğmesi: kullanıcı otomatik kaydı seçti.
  - HTML5 sürükle-bırak: WebView2'de sürükleme görüntüsü hareketli ve kontrol dışı. Pointer olayları ve statik bir çizgi kullanıldı.

### ADR-051 — Statusline düzeni: ayraç öğeleri, boşluk ve esnek boşluk, sağ pay
**Durum:** Kabul (2026-11-08, Adım 12k, kullanıcı kararlarıyla). ADR-049'u genişletir.
- **Bağlam:**
  - 12i'de `│` ayraçlarını render kendisi koyuyordu (`value_sep`). Ayraçlar listede görünmüyor ve tek tek kaldırılamıyordu.
  - Kullanıcı sağa yaslama ve ortalama istedi. Bunun için satırın gerçek genişliğini ve Claude Code'un kenarlarda ne yaptığını bilmek gerekiyordu.
  - **Gözlem** (2026-11-08, gerçek Claude Code, Windows Terminal, prob Git Bash üzerinden; `docs/data-sources.md` S1):
    - `COLUMNS` her çalıştırmada güncel terminal genişliğini taşıyor.
    - `COLUMNS-4` genişliğindeki satır tam sığıyor, `COLUMNS-3` ve daha genişi `…` ile kesiliyor.
    - Satır başındaki boşluklar (NBSP dahil) kırpılıyor. Önlerine `ESC[0m` konunca korunuyorlar. Satır içindeki boşluk dizileri korunuyor.
    - `⏱`, `▲` ve `≈` birer hücre kaplıyor.
    - Pencere daralınca komut hemen yeniden çalışmıyor. Bir sonraki güncellemeye kadar eski satır `…` ile kesik görünüyor.
- **Karar:**
  - **Ayraçların hepsi öğedir (kullanıcı kararı).** Render yalnızca listedeki ayraçları çizer. Bir ayraç, aynı bölgede çizilen iki öğenin arasındaysa çizilir. Bölgenin başındaki ve sonundaki ayraçlar düşer, art arda gelenlerden yalnızca ilki kalır. Böylece değeri olmayan bir öğe (bu hesapta olmayan `five_hour` gibi) çift ayraç bırakmaz.
  - **Göç:** `[statusline]` tablosu `version = 2` taşır. `version` yoksa (12c ve 12i biçimi) her satırda yan yana duran iki değer öğesinin arasına global `separator` glifiyle bir ayraç öğesi eklenir. Glif boşsa ya da yalnızca boşluksa, 12i'nin çizdiği aralık kadar bir `space` eklenir. Göç bir kez yapılır: kaydedilen dosyada `version = 2` bulunur. Şablonlar da aynı kuraldan geçer. `custom_text` göçte değer sayılmaz, çünkü 12i metnin yanına ayraç değil boşluk koyuyordu; görünen metin aynı kalır.
  - **Global `separator` kalır.** Artık yeni eklenen ayraçların glifidir. Eski build'ler için yazılmaya devam eder; 12c anahtarlarının türetilmesi değişmez.
  - **Birleştirme:** Yan yana iki öğe arasına tek boşluk konur. `space` (1–8 sütun, `width`) ve `flex` kendi genişliğini taşır, yanlarına ayrıca boşluk eklenmez.
  - **Hizalama `flex` öğesiyle yapılır (kullanıcı kararı); öğe başına hiza ayarı yoktur.** `flex` satırı bölgelere ayırır:
    - 1 `flex`: ondan sonrası sağa yaslanır.
    - 2 `flex`: orta bölge satırın ortasına (`(W − w) / 2`) konur ve komşularından en az bir sütun uzakta tutulur.
    - Daha fazlası: artan sütunlar eşit dağıtılır, kalan sondaki `flex`'e verilir.
    - Genişlik bilinmiyorsa (`COLUMNS` yok) `flex` tek boşluktur.
  - **Sığdırma:** Ölçümde her `flex` en az 1 sütun sayılır. Düşme sırası aynı kalır. Bir değer düşerken önündeki ayraç, metin ve `space` öğeleri de düşer. `flex` hiçbir zaman düşmez.
  - **Sağ pay (kullanıcı kararı):** Satır en fazla `COLUMNS − 4 − margin` genişliğinde olur. 4 sütun sabittir (`CLAUDE_CODE_RESERVED`). `margin` 0–8 aralığında bir ek paydır ve varsayılanı 0'dır. Kullanıcı, daralma sonrası kesilmeyi gördükten sonra da varsayılanı 0'da bıraktı; bu davranış README'de anlatılır. Bu kural, `flex` içermeyen satırlarda da `COLUMNS-3` ile `COLUMNS` arasında kalan genişliklerdeki `…` kesilmesini düzeltir.
  - **Baştaki boşluk (kullanıcı kararı):** Boşlukla başlayan bir satırın önüne `ESC[0m` konur. Bu, `color = "never"` modunda da yapılır: kod bir renk değil, zararsız bir sıfırlamadır.
  - **Görüntü genişliği:** `chars().count()` yerine `unicode-width` (0.2, `cjk` özelliği kapalı) kullanılır. Gözlemde farklı genişlik çıkan glif olmadığı için istisna tablosu yoktur.
  - **Powerline:** `flex`, blok zincirini kapatır: önce bitiş glifi, ardından arka plansız boşluk, sonra yeni zincir. Son `flex`'ten sonraki bölge sola bakan gliflerle çizilir (U+E0B2 ve ince U+E0B3; yuvarlak temada U+E0B6 ve U+E0B7). `space` zinciri kapatır ve arka plansız çizilir. `separator` atlanır. Renk kapalıyken plain temaya düşülür.
  - **Span'ler:** `space` ve `flex` span'leri öğe indeksini taşır.
  - **Sınırlar:** `MAX_ITEMS` 48'den 64'e çıktı, çünkü ayraçlar öğe sayısını yaklaşık ikiye katlıyor.
  - **Yeni şablon:** `split`. Solda limitler, ardından `flex`, sağda model, cwd ve git dalı.
- **Bedel:**
  - CLI boyutu ve gecikme: `docs/perf/2026-11-08-statusline-layout.md`.
  - Sağa yaslı bir satır terminalin genişliğini doldurur. Pencere daralınca, satır yeniden çizilene kadar sağdaki öğeler `…` arkasında kalır. Bu, Claude Code'un davranışıdır; `margin` küçük daralmalara pay bırakır.
  - Eski CLI'lar (v0.1.0) `space` ve `flex`'i bilmez, kendi otomatik ayracıyla 12c anahtarlarını çizer.
  - Araç çubuğundaki ayraç alanı artık mevcut ayraçları değiştirmiyor, yalnızca yeni eklenenlerin glifini belirliyor. Etiketi "Yeni ayraçlar" oldu. Ayraç başına düzenleme öğe editöründe.
- **Alternatifler:**
  - Öğe başına hiza (`align = "right"`): liste sırası ile hiza ayrı ayrı doğruluk kaynağı olurdu.
  - Otomatik ayracı korumak ve yalnızca bastırma seçeneği eklemek: ayraçlar yine listede görünmez ve tek tek seçilemezdi.
  - `flex`'i yalnızca dolgu olarak kullanmak (satır başında düşmesi): gözlem `ESC[0m`'ın baştaki boşluğu koruduğunu gösterdi.
  - `margin`'i toplam pay (varsayılan 4) yapmak: kullanıcının Claude Code'un iç payını bilmesini gerektirirdi.

### ADR-052 — Statusline satır düzenleyicisi: kartlar düz listenin görünümüdür
**Durum:** Kabul (2026-11-08, Adım 12l, kullanıcı kararlarıyla). ADR-050'yi genişletir.
- **Bağlam:**
  - 12k'dan sonra ayraçlar öğe oldu, `space` ve `flex` geldi. Düz listede satır sonu yalnızca bir çizgiydi; satır ve öğe sayısı arttıkça liste takip edilemez hâle geliyordu.
  - Kullanıcı satır kartları, otomatik ayraç ve `flex` ile hizalama istedi (2026-11-08).
- **Karar:**
  - **Veri düz liste kalır.** Kartlar, listenin *etkin* `line_break` öğelerine göre bölünmüş bir görünümüdür (`lines`). Kural çekirdeğin `render.rs` gruplamasıyla aynıdır, böylece kart ile önizleme satırı ayrışmaz. Gizli bir satır sonu bölmez; kartın içinde sıradan bir satır olarak görünür ve oradan gösterilebilir. Geri alma, kayıt ve TOML biçimi değişmez.
  - **Taşıma:** Alt+↑/↓ ve tutamaç okları komşuyla takas eder. Komşu bir satır sonuysa öğe kendiliğinden komşu karta geçer. Sürükle-bırak hedefi (kart, aralık) olarak hesaplanır (`dropInto`). Duyuru satır numarasını da söyler.
  - **Satır silme** onay istemez ve geri alınabilir. Son görünür değer öğesini götürecekse düğme pasiftir.
  - **Otomatik ayraç (kullanıcı kararı):** Yeni bir değer, satırdaki önceki öğe de değerse önüne, sonraki öğe değerse arkasına bir ayraç alır. Plan yalnızca öne koymayı öngörüyordu; iki değerin arasına eklemede "A │ yeni B" oluşuyordu. Gizli bir değer de komşu sayılır: çekirdek, çizilmeyen komşunun ayracını zaten düşürür. Liste doluysa ayraç eklenmez, değer eklenir.
  - **Yer:** "Sağ pay" araç çubuğunda değil, önizleme çubuğunda terminal genişliğinin yanındadır (kullanıcı kararı). Araç çubuğuna sığmıyor, Dışa/İçe aktar düğmelerini alt satıra itiyordu. Pay, satırın doldurabileceği genişliği daraltır; genişlik seçicisinin yanında anlamı daha açıktır.
  - **Global ayraç alanı araç çubuğundan kalktı.** Ayraç editöründeki "Tüm ayraçlara uygula" hem mevcut ayraçları hem `separator` alanını (yeni ayraçların glifi) günceller.
  - **Düzen satırları incedir (kullanıcı kararı):** Ayraç, boşluk ve esnek boşluk satırları 30 px'tir. Ayraçlar öğe sayısını yaklaşık ikiye katlıyor.
  - **`line_break` paletten çıktı;** yerine kartların altındaki "+ Satır ekle" geldi. Her kartın kendi "+ Öğe ekle" düğmesi vardır: seçim o karttaysa ondan sonra, değilse kartın sonuna ekler.
- **Bedel:**
  - Çekirdeğin `CLAUDE_CODE_RESERVED` (4) ve `MAX_SPACE`/`MAX_MARGIN` sabitleri frontend'de tekrarlanıyor (önizlemedeki taralı bölge ve seçiciler). Binding değişmesin diye kabul edildi; çekirdek değeri yine kendisi sınırlar.
  - Kartlar, öğeleri satır sonlarına göre yeniden gruplamak için her çizimde listeyi tarar. 64 öğede ihmal edilebilir.
- **Alternatifler:**
  - Satırları veride ayrı listeler olarak tutmak (`lines: Item[][]`): TOML biçimi, göç ve geri alma değişirdi. Görünüm yeterli.
  - Satır silmede onay: geri alma zaten var.

### ADR-053 — Statusline zenginleştirme: yeni öğeler, kalan biçimleri, saat olarak sıfırlanma, çubuk genişliği ve stili
**Durum:** Kabul (2026-11-08, Adım 12m, kullanıcı kararlarıyla). ADR-049 ve ADR-051'i genişletir.
- **Bağlam:**
  - Doğrulanmış Ultra hesaplarında 5 saatlik pencere yok (ADR-041). Satırda haftalık pencereye hizmet eden bilgi azdı: günde ne kadar harcanabileceği, oturumun ne hızla tükettiği.
  - Kullanıcı payload'dan gelen yeni öğeleri ve mevcut öğelere yeni biçimleri istedi; store ya da daemon'a bağlı öğeleri istemedi (2026-11-08).
  - `output_style` ve `context_window.current_usage` belgede ayrıntılı değildi. Kod yazılmadan önce bir probla gözlendi (`docs/data-sources.md` S1, 2026-11-08). İkisi de vardı.
- **Karar:**
  - **Yeni öğeler:**
    - `daily_budget` (Limit grubu): haftalık pencerede kalan payın kalan günlere bölümü, `20%/day` / `%20/gün`. `kalan_gün = max(1, ceil(resets_in / 1 gün))`. 10 ve üstünde tam sayı, altında tek ondalık. Pencerenin kaynağını ve yaşını devralır: tahminse `≈` alır ve soluk çizilir, eskiyse soluk çizilir. Renk: pay bittiyse kritik; değilse kullanılan yüzdenin eşiği, kullanım zaman çizgisinin önündeyse (ADR-044, `pace::warns`) en az uyarı. Pencere yoksa ya da sıfırlanma zamanı bilinmiyorsa gizlenir. Limitlerle birlikte en son düşer.
    - `burn_rate`: `cost.total_cost_usd ÷ cost.total_duration_ms`, `API $12.01/h` / `API $12.01/sa`. İki ölçümden türetilir, `≈` almaz. Maliyet API fiyatıyla hesaplandığı için ADR-042'deki `API` işaretini taşır. İlk 5 dakikada gizlenir; o aralıktaki oran gürültüdür.
    - `session_name`, `output_style`: payload'daki ad olduğu gibi gösterilir.
    - `cache_ratio`: son istekte cache'ten okunan girdinin oranı, `cache 82%`. `cache_read / (input + cache_creation + cache_read)`; payda 0 ise gizlenir.
  - **Türetilmiş değerler `Line` yöntemleridir** (`daily_budget`, `burn_rate`, `tokens_left`, `reset_clock`). `Facts` yalnızca payload'dan okunanları taşır (`cache_ratio` dahil). `render.rs` biçimlendirir, hesap yapmaz. Önizleme ve CLI aynı yöntemleri kullanır.
  - **Yeni biçimler:**
    - `remaining` (limitler ve bağlam): kalan yüzde, `58% left` / `%58 kaldı`. Renk eşikleri kullanılan yüzdeye göre kalır.
    - `tokens_left` (bağlam): `context_window_size − kullanılan`; kullanılan, `current_usage`'ın girdi toplamıdır. Yoksa `used_percentage`'tan türetilir.
  - **Yeni öğe alanları** (isteğe bağlı, yalnızca varsayılandan farklıysa yazılır):
    - `reset_format = "countdown" | "clock"` (yalnızca `five_hour` ve `seven_day`). `clock` yerel gün ve saati gösterir: `Tue 04:20` / `Sal 04:20`. Gün kısaltmaları Fluent'tan gelir.
    - `bar_width = 4 | 8 | 12`. Varsayılan 8.
    - `bar_style = "blocks" | "thin" | "dots"`: `█░│`, `▬▭|`, `⣿⣀|`.
    - Esnek okuma (ADR-042): bilinmeyen değer varsayılana döner; `bar_width` 4/8/12 dışındaysa 8 olur. Çubuğu olmayan öğede genişlik ve stil, pencere olmayan öğede `reset_format` varsayılana normalize edilir.
  - **Yerel saat:** Windows'ta `SystemTimeToTzSpecificLocalTime` ile çevrilir (yaz saati dahil, saat dilimi veritabanı gerekmez). Testler sabit bir ofsetle (`Zone::Fixed`) çalışır. Windows dışında `clock` geri sayıma düşer; v0.1 yalnızca Windows'u destekliyor.
  - **Glifler:** `▬▭`, `⣿⣀` ve `|` East Asian Width'te Ambiguous değildir; `unicode-width`'in `cjk` genişliği de 1'dir. Bunu bir test denetler (`unicode-width` `cjk` özelliği yalnızca dev-dependency'de açık). İşaret ince ve noktalı stilde `|`'dir, çünkü kutu çizgisi `│` Ambiguous genişliktedir. `blocks` stili bugünkü çıktı değişmesin diye `█░│`'yu korur.
  - **Sığdırma:** Düşme sırasına "12 hücrelik çubuk 8'e iner" adımı, çubuğun düşmesinden önce girer. `clock` sıfırlanmalarla birlikte düşer.
  - **Şablonlar:** Yeni öğeler yalnızca `detailed` şablonuna girer (`daily_budget`).
  - **Düzenleyici:** Stil seçicinin seçenekleri backend'in çizdiği örneği (`bar_sample`) terminal fontuyla gösterir. Sıfırlanma biçimi seçicisi, öğe sıfırlanmayı gösteriyorsa görünür.
- **Bedel:**
  - CLI 18.432 B büyüdü; gecikmede ölçülebilir fark yok (`docs/perf/2026-11-08-statusline-enrichment.md`).
  - `windows-sys`'e `Win32_System_Time` özelliği ve bir `unsafe` çağrı eklendi.
  - Eski build'ler yeni öğeleri ve alanları okurken atlar. Eski bir build dosyayı yeniden yazarsa bunlar kaybolur (12i ve 12k ile aynı).
  - `prompt_cache.hit_ratio` (oturum geneli) kullanılmadı; belgesiz bir alan.
- **Alternatifler:**
  - `daily_budget`'i bugünün kullanımıyla kıyaslamak: geçmiş ve store gerektirirdi, kapsam dışıydı.
  - `jiff`'in saat dilimi veritabanını gömmek: CLI boyutu büyürdü; Windows API'si aynı sonucu verir.
  - `●○` gibi daire glifleri: Ambiguous genişlikte, bazı terminallerde iki hücre kaplar.
  - `space` öğesinin `width` alanını çubuk genişliği için yeniden kullanmak: anlamları farklı, esnek okuma karışırdı.

### ADR-054 — Statusline compact öğesi: compact sayısı, şema v2 ve auto-compact ayarının okunması
**Durum:** Kabul (2026-11-08, Adım 12n, kullanıcı kararlarıyla). ADR-042 ve ADR-053'ü genişletir.
- **Bağlam:**
  - Kullanıcı, oturumdaki compact sayısını (oto ve elle ayrı) ve tanımlıysa auto-compact penceresini statusline'da görmek istedi. "Compact'a kalan" değeri önerildi, reddedildi (2026-11-08).
  - Compact sınırları transcript'te `type: "system"`, `subtype: "compact_boundary"` kayıtlarıdır (`docs/data-sources.md` S3). İndeksleyici bunları şimdiye kadar atlıyordu; mevcut okuma konumları eski kayıtların ötesindeydi.
  - Auto-compact ayarı transcript'te ve payload'da yok; Claude Code'un ayar dosyalarında ve ortamında.
- **Karar:**
  - **Veri:**
    - Ön filtre `"compact_boundary"` arar. Seçici struct yalnızca `uuid`, `sessionId`, `timestamp`, `isSidechain`, `compactMetadata.trigger` ve `compactMetadata.preTokens` alanlarını okur. Özet mesajı ayrıştırılmaz.
    - Kayıt yalnızca üst düzeyde `type == "system"` ve `subtype == "compact_boundary"` ise sayılır. Kelimenin başka bir yerde geçmesi yetmez.
    - Sidechain kayıtları ve subagent dosyaları sayılmaz.
    - `trigger` `auto` | `manual` dışında bir değerse `other` olarak saklanır. Toplama girer, `split` biçiminde ayrı gösterilir (`1 other` / `1 diğer`).
  - **Store, şema v2** (`0002_compact_events.sql`):
    - `compact_events (uuid PK, session_uuid, at, trigger, pre_tokens)` ve `ix_sessions_uuid`.
    - **Sahiplik kaydın kendi `sessionId`'sidir**, kopyalandığı dosyanın oturumu değil. `--resume` kopyaları aynı `uuid` ile gelir ve `ON CONFLICT DO NOTHING` ile düşer. Dosya sahipliği seçilseydi sonuç, worker'ların hangi dosyayı önce bitirdiğine bağlı olurdu. `sessionId` yoksa dosya adı kullanılır; `uuid` yoksa satırın xxh3-128 hash'i.
    - Sayım yalnızca oturum `kind = 'main'` olarak biliniyorsa döner. Bilinmiyorsa sayı yoktur (gösterilmez); biliniyor ama kayıt yoksa 0'dır.
    - Saklama işi `compact_events`'i silmez (satırlar küçük, sayı oturum geneli).
  - **Geriye dönük doldurma: migration bütün canlı dosyaların okuma konumunu sıfırlar** (kullanıcı kararı). Ayrı bir compact taraması tam yeniden indekslemeden hızlı değildi: 16,7–17,3 s'ye karşı 14,5 s, çünkü ikisinde de disk okuması baskın, indeksleme ise paralel (`docs/perf/2026-11-08-statusline-compact.md`). Tekilleştirme ve saklama filigranı mevcut olayları korur; yükseltilmiş DB'nin parmak izi tam indekslemeyle aynı.
  - **IPC:** `SessionInfo.compacts: Option<CompactCount { auto, manual, other }>`, `#[serde(default)]`. Yeni pipe turu yok; değer `StatusGet` cevabında döner. Oturum cevabın listesinde yoksa yalnızca sayıyı taşıyan bir girdi eklenir. Eski uygulamadan gelen cevapta alan yoksa sayı bilinmiyor demektir.
  - **Auto-compact ayarı (CLI, salt okunur):**
    - Kaynaklar, belgelere göre (2026-11-08), en yüksek öncelikten: managed (`managed-settings.d/*.json` ters alfabetik, sonra `managed-settings.json`), `<proje>/.claude/settings.local.json`, `<proje>/.claude/settings.json`, `<config>/settings.json`. Proje kökü payload'ın `workspace.project_dir`'i, yoksa `cwd`. Config dizini transcript yolundan türetilir.
    - Her dosyadan yalnızca `autoCompactEnabled`, `autoCompactWindow` ve `modelSettings` okunur. Dosya içinde modele özgü pencere üst düzey değeri ezer. 1 MiB'tan büyük ya da okunamayan dosya yok sayılır.
    - Ortam: `DISABLE_AUTO_COMPACT` ya da `DISABLE_COMPACT` → kapalı; `CLAUDE_CODE_AUTO_COMPACT_WINDOW` her pencere ayarını ezer. Statusline Claude Code'un alt süreci olduğu için ayar dosyalarının `env` blokları zaten süreç ortamındadır; `env` blokları ayrıca okunmaz.
    - Pencere 100k–1M dışındaysa yok sayılır, bağlam penceresiyle sınırlanır. `"auto"` değeri pencere yok demektir.
    - Dosyalar yalnızca satırda `compact` öğesi varsa okunur.
    - Gösterilen değer config değeridir; `≈` almaz. **Tetiklenme noktası (gözlenen ≈%90) gösterilmez**, varsayılan pencere tahmin edilmez.
  - **Öğe `compact`** (Oturum grubu): `total` `⟲3 · 300k`, `split` `⟲2 oto 1 elle · 300k`. Kapalıysa `· oto kapalı`. Sayı 0 ise `⟲0`. Sayı bilinmiyorsa (uygulama kapalı) yalnızca ayar: `⟲ 300k`, `⟲ oto kapalı`. İkisi de yoksa öğe çizilmez. Dar terminalde önce pencere kısmı, sonra öğe düşer.
- **Bedel:**
  - **Şema v2 geri dönüşsüzdür.** Yeni sürüm DB'yi bir kez açınca v0.1.0 o DB'yi açamaz (`SchemaTooNew`). Eski sürüme dönmek için DB silinip yeniden indekslenmelidir. Sürüm notunda yazılmalı. Dev build'ler gerçek DB'ye karşı değil, yalnızca portable modda ya da geçici DB ile çalıştırılır.
  - Yükseltmeden sonraki ilk açılış bir tam indeksleme kadar sürer (referans veri setinde ≈15 s, arka planda; RecentReady etkilenmez).
  - Göremediğimiz kaynaklar: kayıt defteri ve sunucu tarafı managed policy'ler, Claude Code'un `--autocompact` ve `--settings` bayrakları, Bedrock/Vertex gibi sağlayıcılara özgü model id'leriyle `modelSettings` eşleşmesi. Bu durumlarda gösterilen değer gerçek davranıştan farklı olabilir.
  - CLI 22.528 B büyüdü; indeksleyici CPU'su ≈%7 arttı, süre değişmedi.
- **Alternatifler:**
  - Yalnızca compact için hafif tarama: daha hızlı değildi, ikinci bir kod yolu ve durum gerektirirdi.
  - Sahipliği dosyaya vermek: `--resume` kopyalarında sonuç belirsizdi.
  - Ayar dosyalarının `env` bloklarını okumak: süreç ortamıyla aynı bilgiyi verir, `env` içinde başka gizli değerler bulunabilir.
  - Pencerenin ≈%90'ını tetiklenme noktası diye göstermek: belgelenmemiş bir iç ayrıntı; kullanıcı istemedi.

### ADR-055 — Nerd Font yardımcısı: indirme, kullanıcı başına kurulum, terminal ayarına yazma ve tek tıkla geri dönüş
**Durum:** Kabul (2026-11-09, Adım 12o, kullanıcı kararlarıyla). ADR-013'ün ağ allowlist'ini ve ADR-009'un "kullanıcı ayarı kutsaldır" akışını genişletir.
- **Bağlam:**
  - Statusline'ı terminal kendi fontuyla çizer. TokenRadar da Claude Code da fontu terminale enjekte edemez; Claude Code ayarlarında font yok, CLI terminalin fontunu sorgulayamaz.
  - Bu makinede Nerd Font yoktu. Powerline temaları 12i'den beri hiç gerçek fontla görülmemişti. Simge setleri ve Nerd bar stilleri (12p) kör yapılmasın diye font önce geldi.
  - Kullanıcı kararları (2026-11-08): font uygulama içinden indirilir; Windows Terminal ve VS Code ayarına diff → onay → yedek → atomik yazma ile yazılır; diğer terminaller için yönerge gösterilir. 2026-11-09: CaskaydiaMono tar.xz'ten alınır; tek tıkla eski haline dönülebilir (ayarlar ve font birlikte, tek bir özet onayıyla).
- **Karar:**
  - **Font: CaskaydiaMono Nerd Font Mono, Nerd Fonts v3.5.1.**
    - Görünüşü Windows Terminal'in varsayılanı Cascadia Mono ile aynı, artı simgeler. Mono varyantında simgeler tek hücredir; bu, `unicode-width` sayımıyla uyumludur.
    - CaskaydiaCove (Cascadia Code) ligatürlüdür; terminalde `->` ve `!=` gibi dizileri birleştirir, statusline'da istenmez. Plandaki CaskaydiaCove bu yüzden değişti.
    - "Cascadia" OFL-1.1'de Reserved Font Name'dir; değiştirilmiş yapı bu adı taşıyamaz, Nerd Fonts adı "Caskaydia" yapar. Font dağıtılmaz, kaynağından indirilir.
    - Microsoft'un "Cascadia Code NF"si U+F418 içerir, U+EE00 içermez (Nerd Fonts < 3.4 kapsaması). İkinci seçenek olmadı.
  - **Kaynak ve doğrulama (ağ allowlist'ine ek):**
    - `https://github.com/ryanoasis/nerd-fonts/releases/download/v3.5.1/CascadiaMono.tar.xz` (3.577.856 B) ve GitHub'ın yönlendirdiği `release-assets.githubusercontent.com`. Başka yönlendirme izlenmez; en çok 3 yönlendirme, yalnızca HTTPS.
    - İndirme yalnızca kullanıcının açık onayıyla başlar. Onaydan önce aile, sürüm, boyut, lisans ve tam URL gösterilir. Telemetri ya da başka bir istek yoktur.
    - Arşivin SHA-256'sı ve alınan dört dosyanın (Regular, Bold, Italic, BoldItalic) SHA-256'ları koda sabitlenir. Gövde bellekte tutulur, sabit boyutla sınırlanır; hash tutmazsa hiçbir dosya yazılmaz. Kısmi indirme diske gitmez.
    - Çıkarma: xz (`lzma-rust2`, bellek sınırı 72 MiB) ve en küçük bir ustar okuyucu. Yalnızca adı koddaki listede olan normal dosyalar alınır; hedef adlar arşivden değil koddan gelir.
    - Sürüm yalnızca uygulama güncellemesiyle değişir; kendiliğinden güncelleme yoktur.
  - **Kurulum (Windows, yönetici izni yok):** Önce dört dosyanın da geçerli font olduğu (name ID 4) doğrulanır. Sonra her dosya `%LOCALAPPDATA%\Microsoft\Windows\Fonts` altına atomik yazılır (aynı baytlar varsa dokunulmaz), `HKCU\Software\Microsoft\Windows NT\CurrentVersion\Fonts` altına `"<tam ad> (TrueType)"` değeri yazılır, `AddFontResourceW` çağrılır ve `WM_FONTCHANGE` yayınlanır. macOS ve Linux Adım 21'e kalır.
  - **Tespit:** Sistem ve kullanıcı font klasörlerindeki dosyaların yalnızca `name`, `cmap`, `hhea` ve `hmtx` tabloları okunur. Kademe: yok / eski (U+F418 var, U+EE00 yok) / tam (ikisi de var). Mono: U+F418'in genişliği 'A'nınkinden büyük değil. Font kayıt defteri taranmaz; başka klasörlerden kurulmuş fontlar görünmeyebilir.
  - **Terminal türü:** CLI her statusline çağrısında yalnızca bir enum gönderir: `TERM_PROGRAM=vscode` → `vscode`, boş olmayan `WT_SESSION` → `windows_terminal`, aksi halde `other`. Değerler, yollar ve oturum kimliği gönderilmez ve loglanmaz. VS Code, kendisini başlatan WT'den `WT_SESSION`'ı devraldığı için VS Code işareti önce bakılır. IPC alanı `serde(default, skip_serializing_if)`; eski ve yeni CLI/uygulama birbiriyle çalışır.
  - **Terminal ayarına yazma:**
    - Windows Terminal (Store, Preview, paketsiz): `profiles.defaults.font.face`. Eski biçim (`profiles` bir dizi) yazılmaz, yönerge gösterilir. Kendi fontunu belirleyen profiller sayılır ve korunur.
    - VS Code ve Insiders: `terminal.integrated.fontFamily`. Font mevcut listenin başına eklenir; liste yoksa `editor.fontFamily`'nin, o da yoksa VS Code'un Windows varsayılanının başına. Yedek fontlar korunur.
    - JSONC cerrahi düzenleme (`jsonc.rs`): yorumlar ve sondaki virgüller aynı uzunlukta boşluğa çevrilmiş bir kopya üzerinde yürünür; değişiklik özgün metne eklenir. Dosya hiçbir zaman yeniden serileştirilmez; yorumlar, sıra ve girinti korunur.
    - Akış: plan → diff → kullanıcı onayı (token ile korunan tek plan) → dosya yeniden okunur ve değişmemiş olmalı → `settings.json.tokenradar-<UTC>.bak` yedeği → atomik yazma.
  - **Geri alma (terminal başına):** Kayıtlı durum gerekmez; dosyanın yanındaki en yeni TokenRadar yedeği önceki değeri tutar. Dosya bizim yazmamızdan beri değişmemişse yedek bayt bayt geri gelir. Değişmişse yalnızca anahtar geri alınır; TokenRadar'ın oluşturduğu nesneler boş kaldıysa onlar da silinir. Geri almanın kendi yedeği `.undo.bak` uzantısını alır ve sonraki geri almanın kaynağı olmaz.
  - **Tek tıkla geri dönüş (kullanıcı kararı, 2026-11-09):**
    - Tek düğme bir özet açar: her terminal için "önce → sonra" fontu ve kaldırılacak font dosyaları. Tek bir onay hepsini uygular. Dosya başına diff gösterilmez; bu, diff → onay kuralından bilinçli bir sapmadır. Kuralın amacı korunur: değişiklik önceden gösterilir, onaysız hiçbir şey yazılmaz, her dosyanın yedeği alınır.
    - Yalnızca TokenRadar'ın değişikliği hâlâ yerindeyse bir terminal geri alınır: anahtar bir Nerd Font ile (ya da kurulu olmasa bile TokenRadar'ın ailesiyle) başlamalı ve bir yedek farklı bir değer tutmalı. Kullanıcının sonradan seçtiği font onundur ve kalır.
    - Önce ayarlar, sonra font: hiçbir terminal kaldırılmış bir fonta işaret ederek kalmaz. Bir ayar yazılamazsa işlem orada durur.
    - Font kaldırma yalnızca baytları sabit SHA-256 ile eşleşen dosyalara uygulanır; aynı adlı başka bir dosya elle kurulmuştur ve dokunulmaz. Sıra: `RemoveFontResourceW` → dosyayı sil → kayıt defteri değerini sil → `WM_FONTCHANGE`. Dosya kullanımdaysa font oturuma geri eklenir ve hata döner; yarım kalmış bir durum oluşmaz.
  - **Önizleme:** Kurulu Nerd Font `--font-term` listesinin başına geçer. Pencere fontu gerçekten çizemiyorsa (aşağıya bakın) açıklama satırı "yeniden başlatınca" der; önizleme fontu çiziyormuş gibi yapmaz.
- **Bedel:**
  - **Geçici bellek:** xz sözlüğü 64 MiB'tır. Release derlemede doğrulama ve çıkarma süreci tepe private 102,2 MiB'a çıkarıyor, 0,32 s sürüyor; ardından bellek eski düzeyine iniyor (`docs/perf/2026-11-09-nerd-font.md`). Yalnızca kullanıcının başlattığı tek seferlik bir işlemde olur.
  - Uygulama 321.536 B büyüdü (13,81 MiB, bütçe ≤ 15 MB): `lzma-rust2`, `ttf-parser`, yeni ekran. CLI 1.536 B büyüdü. `lzma-rust2`, `sha2`'nin ikinci bir sürümünü (0.11) getiriyor.
  - **Yeniden başlatma:** Chromium tabanlı programlar (TokenRadar'ın WebView'u, VS Code) yalnızca süreç başlarken kurulu olan fontları görüyor; ölçüldü. Windows Terminal için gözlem açık. İpucu metni yeniden başlatmayı kesin bir adım olarak yazıyor.
  - Terminal türü yalnızca yeni CLI'dan gelir; kurulu v0.1.0 CLI'ı göndermez, bu yüzden kart "çağrı görülmedi" der.
  - Font kayıt defteri taranmadığı için başka bir yere kurulmuş Nerd Font'lar "yok" görünebilir.
- **Alternatifler:**
  - Fontu kurulum paketine gömmek: installer büyürdü ve kullanıcı istemeden font dağıtılırdı.
  - Kullanıcıyı tarayıcıya yönlendirmek: kurulum ve terminal ayarı kullanıcıya kalırdı; kullanıcı reddetti.
  - `CascadiaCode.zip` ya da tek tek TTF indirmek: zip daha büyük; tek tek TTF'ler release varlığı değil, depo dosyası. Kullanıcı tar.xz'i seçti.
  - Geri dönüşte dosya başına diff göstermek: tek tık isteğine uymuyordu. Hiç onaysız uygulamak: kural tamamen bırakılırdı; kullanıcı özet onayını seçti.
  - Kurulum durumunu bir dosyada tutmak: yedekler ve sabit hash'ler aynı bilgiyi veriyor, eskiyebilecek ikinci bir durum gerekmiyor.

### ADR-056 — Statusline simge setleri, kapsül ve dolgu çubukları, yedek fontlu işaretlerden sonra boşluk
**Durum:** Kabul (2026-11-09, Adım 12p, kullanıcı kararlarıyla). ADR-044'ün zaman işaretini, ADR-049'un etiket ve renk kurallarını genişletir.
- **Bağlam:**
  - 12o'dan beri bu makinede gerçek bir Nerd Font (CaskaydiaMono Nerd Font Mono 3.5.1) var. Etiketler hâlâ kelimeydi (`git main`, `ctx 38%`), çubuklar üç stilde (blok, ince, nokta) yalnızca düz temada çiziliyordu.
  - Kullanıcı kararları (2026-11-08): üç simge seti; Nerd Fonts 3.4'ün ilerleme glifleriyle bir çubuk ve arka plan hücreleriyle bir çubuk; kullanıcı etiketi her zaman önde. 2026-11-09: kapsül Powerline bloklarında denenip görülerek karara bağlanacaktı; zaman işareti dolu bir hücre olacak; yedek fonttan gelen işaretlerden sonra boşluk konacak.
  - **Gerçek Windows Terminal gözlemi (2026-11-09):** Cascadia Mono ve Caskaydia'da `⏱` (U+23F1), `⟲` (U+27F2) ve `⎇` (U+2387) yok. Üçü de yedek fonttan, bir hücreden geniş çiziliyor ve sonraki hücrenin üstüne biniyor. `⏱`, U+FE0E ile bile renkli emoji olarak çiziliyor. `◷ ⇡ ⇣ ⌂ ›` fontun içinde ve hücresine sığıyor. Nerd glifleri hücresine sığıyor. `⌚` (U+231A) East Asian Wide, yani iki hücre.
- **Karar:**
  - **Tercih:** `[statusline] icons = "text" | "unicode" | "nerd"`, varsayılan `text`. Bilinmeyen değer `text` okunur (ADR-042'nin hoşgörülü okuması). Anahtarı bilmeyen eski bir build onu atlar ve kelime çizer.
  - **Etiket önceliği:** Kullanıcının etiketi > setin simgesi > Fluent kelimeleri. Yalnızca boşluktan oluşan bir etiket hiçbir şey göstermez, simge de.
  - **Unicode seti** yalnızca güçlü ve Ambiguous olmayan simgeler kullanır: `⇡` gönderilen ve `⇣` alınan token, `◷` süre, `⎇` git dalı, `⌂` proje dizini, `›` çalışma dizini, `#` oturum kimliği. Diğer türler kelimelerini korur.
  - **Nerd seti** Octicons kullanır (hepsi BMP PUA, U+F400–F533): 5 saat, 7 gün, bağlam, token, süre, API süresi, önbellek, git, proje, cwd, oturum kimliği, model, sürüm, ajan, oturum adı, çıktı stili, satır değişimi, günlük bütçe ve yanma hızı. Kelime etiketi olmayan değerler de simge alır.
  - **Zaman ve compact işaretleri:** Yalnızca Nerd seti bunları değiştirir: `⏱` yerine kum saati (U+F4E3, çünkü `oct-stopwatch` süreye ait), `⟲` yerine U+F48C.
  - **Değişmeyenler:** `≈`, soluk çizim, `API` işareti ve "bu oturum" hiçbir sette değişmez. Dürüst sayılar kuralı simgeye bırakılmaz.
  - **Yedek fontlu işaretlerden sonra boşluk (kullanıcı kararı):** Metin ve Unicode setlerinde `⏱` ve `⟲`'den sonra bir boşluk gelir (`⏱ 43%`, `⟲ 3`). Geniş çizilen glif bu boşluğa taşar, rakamın üstüne binmez; WT'de doğrulandı. Nerd glifleri hücresine sığdığı için boşluk almaz; kum saati rakama bitişik çizilir. `unicode-width` bu glifleri yine 1 sayar; boşluk sayıma dahildir.
  - **`⎇` boşluk almaz:** Ardından zaten etiketin boşluğu gelir; glif o boşluğa taşıyor, kabul edildi.
  - **Bar stilleri:**
    - `capsule`: Nerd Fonts ≥ 3.4 ilerleme glifleri U+EE00–EE05 (boş ve dolu; sol uç, orta, sağ uç).
    - `solid`: arka planı boyanmış boşluk hücreleri; font istemez. Doluluk paint'in rengini (ok cyan, warn sarı, crit kırmızı), boş hücreler griyi alır. Kullanıcının rengi doluluğa yalnızca uyarı eşiğinin altında uygulanır (ADR-049).
  - **Zaman işareti hücre içinde:** İki stil de ADR-044'ün `│`'ünü ek bir hücre olarak eklemez; zamanın düştüğü hücreyi işaretler, böylece çerçeve ve doluluk bütün kalır. Dolgu bir `|` çizer. Kapsül, **dolu bir glif** çizer (kullanıcı kararı B): düz temada parlak, Powerline bloğunda soluk. İşareti doluluktan ayıran yalnızca renk; bu güvenli, çünkü çubuklar yalnızca renkle çizilir.
  - **Powerline blokları:** Kapsül bloklarda da çizilir. Hücreleri şekille ayrıldığı için bloğun tek renginde de okunuyor; WT'de görülerek karar verildi (kullanıcı: "Dene, görüp karar ver"). Plan "capsule blok içinde çizilmez" diyordu. Dolgu, blok, ince ve nokta bloklarda çizilmez. Düzenleyici bu durumda "Powerline blokları yalnızca kapsülü çizer" notunu gösterir.
  - **Renk kapalıyken:** Çubuklar zaten yalnızca renkle çizilir; renk kapalıyken hiçbir stil çizilmez. Plandaki "renk kapalıyken `blocks`'a düşer" kuralının bu yüzden bir etkisi yok.
  - **Şablonlar** kullanıcının simge setini korur.
  - **Düzenleyici:**
    - Araç çubuğunda simge seti seçici. Powerline teması seçilip bir Nerd Font kuruluysa Nerd simgeleri önerilir. Font yoksa ya da Mono değilse bir uyarı çıkar; kapsül 3.4 öncesi fontta uyarır. Uyarılar 12o'nun font kademesini kullanır ve font durumu bilinmeden konuşmaz.
    - Araç çubuğu Türkçede tek satıra sığsın diye "Şablonlar…" başlık satırına, "Varsayılana dön…"ün yanına taşındı. "Dışa aktar" ve "İçe aktar", ipucu ve erişilebilir adı tam cümle olan simge düğmeleri oldu (kullanıcı kararı).
    - Bar stili seçicisi beş seçeneği dar sütunda iki satıra kırar. Dolgunun örneği renkli hücrelerle çizilir.
    - Öğe listesi, düz temada dolgu çubuğunun hücrelerini renkli çizer; Powerline'da liste metni tek renkte kalır.
- **Bedel:**
  - CLI 5.120 B büyüdü (2,37 MiB, bütçe ≤ 3 MB). Statusline p95'te ölçülebilir fark yok (`docs/perf/2026-11-09-icon-sets.md`).
  - Metin ve Unicode setlerinde zaman ve compact işaretleri bir sütun daha uzun; dar terminalde öğeler bir sütun önce düşebilir.
  - **Öğe listesi tek renkli:** Kapsülün işareti orada doluluğun renginde, araya bir boşluk girmiş gibi görünür ve kullanım sanılabilir. Önizleme ve terminal renkli; liste bir sınır olarak bırakıldı.
  - Mono olmayan Nerd Font varyantları denenmedi (bu makinede yalnızca Mono kurulu); simgeler komşu hücreye taşabilir. Düzenleyici bunun için uyarır.
  - **VS Code terminali görülmedi.** Gerçek kontrol yalnızca Windows Terminal'de yapıldı. VS Code'un yedek fontu farklıysa boşluğun yetip yetmediği orada açık.
- **Alternatifler:**
  - Kapsül işaretini çerçeveli (boş) bir glifle çizmek: kullanıcı dolu işareti seçti.
  - `⏱` yerine fontta olan `◷`: süreyle aynı simge olurdu ve "geçen zaman" anlamı kaybolurdu.
  - `⏱` için U+FE0E (metin sunumu): WT emoji çizmeye devam etti.
  - `unicode-width`'i `⏱`'yi 2 sayacak biçimde değiştirmek: genişlik terminalden terminale değişiyor; boşluk iki durumda da doğru.
  - Araç çubuğunu iki satıra bırakmak: 12o'nun tek satır düzeltmesini geri alırdı. Düğme metinlerini kısaltmak yerine kullanıcı simge düğmelerini seçti.

### ADR-057 — Fiyat tablosu değişince saklı maliyetler yeniden hesaplanır; fiyatı bilinmeyen model görünür olur
**Durum:** Kabul (2026-11-11, Adım 13 öncesi veri keşfi, kullanıcı kararlarıyla). ADR-025'in "bilinmeyen model" kuralını genişletir.
- **Bağlam:**
  - `usage_events.cost_micro_usd` indeksleme anında hesaplanıp saklanır. Tabloda satırı olmayan model, ailesinin en yeni satırıyla fiyatlanır (ADR-025) ve bu yalnızca log dosyasına yazılır.
  - Kurulu derleme `claude-fable-5-2` satırından önce derlenmişti. Model iki gün boyunca Fable 5.1 fiyatıyla ($10/$50; doğrusu $8/$40) kaydedildi, yani %25 fazla. Tablo aynı akşam güncellendi, ama saklı maliyetler değişmedi ve yeni derleme kurulana kadar yeni olaylar da yanlış yazıldı. Fark, Adım 13 öncesi veri keşfinde ortaya çıktı.
  - Kalibrasyon (`k = Δp/ΔC`) ve atıf doğrudan bu sütuna dayanır. Yanlış ağırlık, yanlış katsayı demektir.
- **Karar:**
  - **Yeniden fiyatlama.** Depo, olayları fiyatlayan tablonun parmak izini `kv` tablosunda `pricing.table` anahtarıyla tutar. Parmak izi `version` ile tablo metninin xxh3 özetidir; sürümü artırmak unutulsa bile tablodaki her değişiklik sayılır.
    - Çekirdek başlarken gömülü tablonun parmak izi farklıysa (ya da anahtar yoksa) bütün ham olaylar yeniden fiyatlanır. Maliyeti değişen her olayın farkı `usage_5m` satırına uygulanır.
    - Geçiş 10.000 olaylık transaction'larla ve limit motoru kurulmadan önce çalışır. Yarıda kesilirse parmak izi yazılmaz, bir sonraki açılışta baştan çalışır ve doğru fiyatlı olaylara dokunmaz.
  - **Görünürlük.** Son 7 günde kullanılmış olup tabloda satırı bulunmayan modeller `IndexState.unpriced_models` alanıyla panele gider.
    - Liste açılışta `usage_5m`'den kurulur, sonra indeksleyici raporlarıyla büyür. Yalnızca yeniden başlatmada küçülür; düzeltme zaten yeni bir derleme gerektirir.
    - Panel sakin bir bildirim gösterir (`panel-unpriced`, model adlarıyla). `tokenradar doctor` aynı listeyi yazdırır ve `report.json` içine `database.pricing` bölümünü ekler. Model kimlikleri ürün adıdır, kişisel veri değildir.
- **Bedel:**
  - Saklama süresini aşmış olaylar yeniden fiyatlanamaz: `usage_5m` olay başına hız modunu, web aramasını ve istem büyüklüğünü taşımaz. O kovalar eski maliyetle kalır.
  - Tabloda mevcut bir modelin sayısı değişirse geçmiş de yeni sayıyla fiyatlanır. Anthropic bir modelin fiyatını değiştirirse tarihli fiyat satırları gerekir; bu, o gün ayrı bir ADR ile ele alınır.
  - Daha eski bir derlemeye dönülürse olaylar o derlemenin tablosuyla yeniden fiyatlanır.
  - Yayınlanmış sürümü kullananlar, güncelleme gelene kadar yedek fiyatı görmeye devam eder. Bildirim bunu söyler ama düzeltmez.
  - Ölçüm: gerçek bir veritabanının kopyasında tam geçiş release derlemede 0,1 saniyenin altında sürdü. Parmak izi eşleşiyorsa maliyet tek bir `kv` okumasıdır.
- **Alternatifler:**
  - Fiyat tablosunu uygulama sürümünden bağımsız olarak GitHub'dan güncellemek: yayınlanmış sürümleri de düzeltir, ama ağ allowlist'ini (ADR-013) genişletir ve bütünlük doğrulaması ister. Ertelendi; gerekirse ayrı bir adım olarak planlanır.
  - Yalnızca yedekle fiyatlanmış olayları işaretleyip onları yeniden fiyatlamak: şema değişikliği gerektirir ve eski derlemelerin işaretsiz kayıtlarını yakalayamaz.
  - Maliyeti saklamayıp sorgu anında hesaplamak: `usage_5m` toplamları ve panel sorguları için pahalı.
  - Kalibrasyonda `ΔC` değerini token'lardan hesaplamak: yalnızca kalibrasyonu düzeltir, paneldeki maliyetler yanlış kalır.
- **Adım 13'e not:** `unpriced_models` listesindeki modellerin ağırlıklı olduğu aralıklar kalibrasyona alınmamalı; maliyetleri tahmindir.

### ADR-058 — Widget baştan native çizilir; görev çubuğu modu eklenir
**Durum:** Kabul (2026-11-11, Adım 18 yeniden planı, kullanıcı kararlarıyla). ADR-011'in yerini alır.
- **Bağlam:**
  - Adım 13, ilk tam haftalık pencere kapanana kadar (18 Kasım 2026) başlayamıyor. Kullanıcı bu arada Windows'a özel, isteğe bağlı ve estetik bir görev çubuğu widget'ı istedi.
  - ADR-011 widget'ı önce WebView ile yapıp ölçmeyi, ek maliyet 45 MB'ı aşarsa native çizime geçmeyi öngörüyordu. Plan yalnızca masaüstünde yüzen bir şerit içeriyordu.
  - Windows 11'de görev çubuğuna öğe eklemenin resmi bir API'si yok (DeskBand kaldırıldı). Görev çubuğuna kenetlenen bir pencere Win32 yöntemleriyle yapılabiliyor. Bu pencerede bir WebView2 süreç ağacı hem kırılgan hem de pahalı olur.
  - Tray ikonu zaten `tiny-skia` ile çiziliyor ve ortam değişikliklerini gizli bir pencereyle izliyor (ADR-030). Native bir widget bu altyapıyı yeniden kullanabilir.
- **Karar:**
  - **Native çizim.** Widget bir Tauri penceresi değildir. Kendi thread'inde çalışan bir Win32 layered window'dur (`UpdateLayeredWindow`, per-pixel alpha); bitmap `tiny-skia` ile çizilir. Renkler ve eşikler tray ile paylaşılır. Widget kapalıyken thread ve pencere yoktur.
  - **İki mod.** `floating` (masaüstünde, topmost, sürüklenebilir, kenarlara yapışır) ve `taskbar` (birincil görev çubuğunun boş alanına, bildirim alanının soluna kenetli). İkisi aynı pencere ve çizim temelini kullanır.
  - **Görev çubuğu modu en iyi çabadır.** Kenetlenme başarısız olursa veya düzen desteklenmiyorsa (yan veya üst kenardaki görev çubuğu) widget yüzen moda düşer ve bunu bir kez söyler. Explorer'a kod enjekte edilmez.
  - **İçerik.** Haftalık çubuk her zaman, 5 saatlik çubuk yalnızca pencere varsa gösterilir (ADR-041). `≈`, bayatlık ve önem renkleri diğer yüzeylerle aynıdır.
  - **Bütçe.** Widget açıkken ek bellek tray-only değerinin üstüne ≤ 8 MB'tır (hedef 3-5 MB). Kesin değer ölçümden sonra bu kayda eklenir.
  - **Bölünme.** Adım 18 yüzen modu ve ortak temeli yapar, Adım 18b görev çubuğu modunu ekler. İkisi de Adım 13'ü beklemeden yapılabilir.
- **Açık kararlar (adımlarda spike ile kapanır, sonuç bu kayda eklenir):**
  - Metin: DirectWrite ve sistem fontu ile `fontdue` ve gömülü mono font arasında seçim (Adım 18).
  - Görev çubuğu yöntemi: `Shell_TrayWnd`'nin sahip olduğu topmost popup ile `SetParent` ile çocuk pencere arasında seçim (Adım 18b). Varsayılan tercih popup'tır.
- **Bedel:**
  - Sürükleme, menü, DPI ve erişilebilirlik Win32 ile elle yazılır; WebView'ın hazır sunduğu şeyler yeniden yapılır.
  - Widget'ın görünümü Svelte bileşenleriyle paylaşılmaz. Tasarım değişiklikleri iki yerde yapılır.
  - DWM Acrylic layered pencerede kullanılamaz; yüzen modun zemini yarı saydam düz renktir.
  - Görev çubuğu modu Windows'un gelecekteki görev çubuğu değişikliklerinde bozulabilir. Yüzen mod yedek olarak kalır.
  - Yalnızca birincil görev çubuğu desteklenir.
- **Alternatifler:**
  - ADR-011'in yolu (önce WebView, sonra ölç): yüzen modda işe yarardı, ama görev çubuğu modunda başka bir sürecin penceresine WebView2 bağlamak gerekirdi.
  - Windows 11 Widgets panosu (Adaptive Cards): resmi bir yol, ama MSIX paketleme ve COM sağlayıcı istiyor ve görev çubuğunda değil, ayrı bir panoda görünüyor.
  - Yalnızca görev çubuğu modu: yüzen yedek olmadan, görev çubuğu bozulduğunda widget tamamen kaybolurdu.
  - Explorer içinde çalışan bir mod (Windhawk tarzı DLL enjeksiyonu): güvenlik ve kararlılık açısından kabul edilemez.
- **Sonuçlar (Adım 18, 2026-11-11):**
  - **Metin: DirectWrite (kullanıcı kararı).** Spike: DirectWrite + `Segoe UI Variable` (Direct2D yazılım WIC hedefi, grayscale AA) %100'de net, ikili boyuta etkisi yok, ilk çizimden sonra ≈ +2,7 MB private bellek, ilk çizim 25–80 ms. `fontdue` + gömülü Geist Mono %100'de daha yumuşak ve geniş, +100 KB ikili, ≈ +0,5 MB bellek, deterministik. Golden testler kutu glifli sahte bir metin motoruyla çizer (`widget/render/tests.rs`); DirectWrite Windows'ta duman testiyle ve ekran görüntüsüyle denetlenir.
  - **Tema moda göre (kullanıcı kararı, plandan sapma).** Plan "Sistem" temada `SystemUsesLightTheme` diyordu. Yüzen widget uygulama pencerelerinin ve masaüstünün üstünde durduğu için uygulama temasına uyar: açık/koyu seçimi geçerlidir, "Sistem" `AppsUseLightTheme` demektir (panelle aynı). Görev çubuğu modu (18b) görev çubuğunun parçasıdır: uygulama ayarına bakmadan `SystemUsesLightTheme`'e uyar (tray ikonuyla aynı). Kural saf `widget::is_light` fonksiyonundadır.
  - **Opaklık yalnızca zemini etkiler.** Metin ve çubuklar her opaklıkta opaktır; %40'ta bile okunur kalır. Bedel: açık tema düşük opaklıkta koyu bir pencerenin üstünde griye döner ve ikincil metnin kontrastı düşer.
  - **Sağ tık menüsü odak alır.** `TrackPopupMenu`'nün dışarı tıklayınca kapanması için menüden hemen önce `SetForegroundWindow` çağrılır (Win32'nin belgelenmiş gereği). Widget'ın odak almadığı tek an budur; sol tık, sürükleme ve çift tıklama odak almaz.
  - **Mod seçicisi arayüzde yok.** `mode` alanı config'te ve bağlamalarda var; 18b'ye kadar ayarlar sayfası onu göstermez.
  - **Ölçüm** (`docs/perf/2026-11-11-widget.md`, sahte mod, 10 dk): Widget açıkken ek private working set **+1,73 MB** (bütçe ≤ 8 MB, hedef 3–5 MB). Boşta CPU +%0,005 (bütçe < %0,1). Widget thread'i ≈ 2,2 bağlam değişimi/s gösteriyor. Bunun ≈ 1,35/s'si 5 sn'lik `SHQueryUserNotificationState` çağrısından (Explorer'a RPC) geliyor; pencere mesajı dakikada ≈ 14. `tokenradar-app.exe` 14,22 MiB; 15 MB bütçesine yakın.
- **Sonuçlar (Adım 18b, 2026-11-11):**
  - **Yöntem: (a) sahibi `Shell_TrayWnd` olan topmost layered popup.** Spike makinesi: Windows 11 build 26300, %125 ölçek, alt kenarda 48 mantıksal px görev çubuğu.
    - (a) görünür, görev çubuğuyla kaynaşıyor; `WindowFromPoint` her zaman widget'ı buluyor. Görev çubuğu `SetWindowPos(HWND_TOPMOST)` ile zorla öne alındığında da widget üstte kaldı: sahip öne gelince sahip olunan pencere de onunla gelir.
    - (b) `Shell_TrayWnd`'nin çocuğu olan pencere hiç görünmedi: görev çubuğunun XAML/DirectComposition içeriği onu örtüyor, tıklamalar görev çubuğuna gidiyor. Reddedildi.
    - Sahip çalışırken `SetWindowLongPtrW(GWLP_HWNDPARENT)` ile değişebiliyor; yüzen ↔ görev çubuğu geçişi pencereyi yeniden kurmadan yapılıyor.
  - **Z-sırası kancası yok (plandan sapma).** 90 sn'lik elle denemede, kanca olmadan 30 ön plan değişikliği ve görev çubuğuna tıklamalar oldu; widget bir kez bile arkada kalmadı (kullanıcı gözle doğruladı). `SetWinEventHook` eklenmedi. İstisna: sahip değiştiğinde z-sırası kendiliğinden düzelmez. Explorer yeniden başladıktan sonra widget, log'a göre kenetli olduğu hâlde ekranda görünmedi; yeni görev çubuğu üstüne çıkmış olmalı. Yeniden kenetlenirken pencere bir kez `HWND_TOPMOST` ile öne alınır.
  - **Kanca yerine yoklama.** `EVENT_OBJECT_LOCATIONCHANGE` görev çubuğu thread'inde 40 sn boşta 0 olay üretti, ama bildirim alanının büyümesi gözlenemedi. Görev çubuğu modunda tam ekran yoklamasıyla aynı 5 sn'lik zamanlayıcı `Shell_TrayWnd` ve `TrayNotifyWnd` dikdörtgenlerini okur (`GetWindowRect`, Explorer'a çağrı yok). Değişirse yeniden kenetlenir. Kenar ve otomatik gizleme (`SHAppBarMessage`) yalnızca değişiklikte sorulur. Bedel: bildirim alanı büyüyünce şerit en geç 5 sn sonra kayar.
  - **Otomatik gizlenen görev çubuğunda yüzen moda düşülür.** Görev çubuğuyla birlikte gizlenip görünmek, gizleme animasyonunu izleyen bir kanca isterdi. Sol, sağ veya üst kenar ve yer kalmaması da yüzen moda düşürür. Bildirim çalıştırma başına bir kez gösterilir; Explorer yeniden başlarken (görev çubuğu yokken) gösterilmez.
  - **Görev çubuğundayken tam ekran gizlemesi her zaman açıktır.** Görev çubuğunun bir parçası tam ekran videonun üstünde duramaz. Ayarlar sayfası anahtarı işaretli ve kilitli gösterir; opaklık seçimi yalnızca yüzen modda görünür.
  - **Zemin.** Kenetli şeridin zemini alfa 1'dir: gözle görünmez, ama fare olaylarını alır. Hover vurgusu WinUI'nin ince dolgusudur (açıkta siyah α 10, koyuda beyaz α 15, köşe yarıçapı 4, üstten ve alttan 4 mantıksal px içeride). Yüksek kontrastta kenetli şerit de opak plakasını korur. Görev çubuğu 40 mantıksal px'ten alçaksa tek satırlık kompakt biçim çizilir.
  - **Explorer yeniden başlatma** (kullanıcı onayıyla, iki kez): Görev çubuğu yokken widget yüzer. Yeni görev çubuğu ≈ 5 sn içinde bulunur (yoklama veya `TaskbarCreated`) ve widget yeniden kenetlenir. Pencere sahibiyle birlikte yok olmadı; yok olursa thread pencereyi yeniden kurar.
  - **Bilinen bedel.** Süreçler arası sahiplik, widget thread'inin giriş kuyruğunu Explorer'ın görev çubuğu thread'iyle bağlayabilir (ölçülmedi). Widget thread'i bu yüzden hiçbir zaman bloklanmaz; uzun iş yapmaz.
  - **Ölçüm** (`docs/perf/2026-11-11-widget-taskbar.md`, sahte mod, 10 dk): Görev çubuğu modunda ek private working set **+1,77 MB** (yüzen modla aynı düzey), boşta CPU +%0,023 (bütçe < %0,1). CPU yüzen modun ≈ 2 katı; koşu bellek baskısı altında yapıldı ve aynı build ile yüzen karşılaştırması yarıda kaldı.

### ADR-059 — Statusline payload'ın yeni alanlarını okur; belgeye dayalı alanlar, canlı yenileme ve daemon'da git durumu
**Durum:** Kabul (2026-11-11, Adım 12q ve 12r planı, kullanıcı kararlarıyla: kapsam, cache süresinin biçimi, belgeye dayalı alanlar, git durumunun yeri ve yenilemenin varsayılanı).
- **Bağlam:**
  - Claude Code'un statusline stdin JSON'una yeni alanlar eklendi ve 12m'deki prob bunların çoğunu gördü: `effort.level`, `thinking.enabled`, `fast_mode`, `prompt_cache.*`, `workspace.repo.*`, `workspace.added_dirs`. Satır bunları okumuyor.
  - Rakip araçlar (ccstatusline, claude-code-tool-manager) efor, cache zamanlayıcısı, cache kaçırmaları, harcama limiti ve git durumu gösteriyor. Kullanıcı efor seviyesini ve cache süresini istedi, ardından araştırmadaki öğelerin hepsini.
  - Bazı alanlar bu hesapta gözlenemiyor. `rate_limits.spend_limit` yalnızca Claude apps gateway arkasında, `pr.*` yalnızca açık bir PR varken geliyor.
  - Claude Code satırı yalnızca olaylarda yeniden çizer. Geri sayımlar boştayken donar; `refreshInterval` bunu çözer ama satırı periyodik olarak çalıştırır.
  - `git status` Windows'ta bir süreç başlatmak demektir (≈ 20–80 ms) ve statusline p95 ≤ 30 ms bütçesini tek başına aşabilir.
- **Karar:**
  - **Yeni alanlar yalnızca satır için okunur** (`skip_serializing`, 12i deseni). Push ve spool biçimi değişmez. Harcama limiti limit motoruna girmez.
  - **Belgeye dayalı alanlar.** Resmi şemada tanımlı ama gözlenemeyen bir alan, resmi örnekten üretilen sentetik fixture'larla ve esnek okumayla uygulanabilir. `data-sources.md`'de "belgeye dayalı, gözlenmedi" diye işaretlenir. Bu, 12m'deki "gözlemsiz kod yok" kuralının bu alanlar için gevşetilmesidir.
  - **Cache süresi önce saat biçimiyle gelir.** `prompt_cache.expires_at` yerel saat olarak gösterilir; yenileme gerektirmez ve boştayken de doğrudur. Geri sayım biçimi 12r'de gelir, yenileme kapalıysa editör onun donacağını söyler.
  - **Canlı yenileme isteğe bağlıdır ve varsayılan olarak kapalıdır.** Seçenekler 5, 10 ve 30 saniye. `settings.json` değişikliği diff → onay → backup → atomic write ile yapılır. Kullanıcının `statusLine` nesnesindeki diğer anahtarlar korunur. Aynı payload'ın tekrarı store'a ve spool'a yazılmaz.
  - **Git durumu daemon'da hesaplanır.** CLI sorguyla `cwd` gönderir, daemon önbellekli (HEAD/index mtime + 5 s TTL) ve arka planda `git --no-optional-locks status --porcelain=v2` çalıştırır. Daemon kapalıyken öğe gizlenir. Dosya adları saklanmaz, yalnızca sayılar tutulur.
  - **Ağ gerektiren öğeler yok** (ADR-013): servis durumu ve `gh` ile PR/CI sorgusu.
- **Bedel:**
  - Belgeye dayalı alanlar şema değişirse sessizce boş kalabilir. Esnek okuma çökmeyi önler ama yanlışlığı göstermez.
  - Yenileme açıkken her aralıkta bir CLI süreci ve bir pipe turu çalışır. Zincirlenmiş önceki komut da çalışır.
  - Git durumu ilk çizimde görünmez (önbellek soğuk), daemon kapalıyken hiç görünmez.
  - Payload struct'ı ve öğe sayısı büyür; soğuk başlatma her adımda yeniden ölçülür.
- **Alternatifler:**
  - Yalnızca gözlenen alanlar: harcama limiti ve PR öğeleri, gözlenene kadar ertelenirdi. Kullanıcı hepsini istedi.
  - Geri sayımı yenilemesiz sunmak: boştayken donmuş bir sayı gerçeği yansıtmaz ("dürüst sayılar").
  - Git durumunu CLI'da senkron hesaplamak: bütçeyi aşar.
  - CLI'da dosya önbellekli, ayrıntılı bir alt süreç ile git durumu: daemon kapalıyken de çalışırdı, ama her satır çağrısına dosya kilidi ve süreç yönetimi ekler; daemon zaten uzun ömürlü ve tek yazıcı.
  - `.git/index`'i doğrudan ayrıştırmak: izlenmeyen dosyalar ve upstream farkı için yetersiz, sürüm uyumluluğu kırılgan.
- **Sonuçlar (Adım 12q, 2026-11-11):**
  - **Prob:** Yeni gözlenen yollar `worktree.{name, path, branch, original_cwd}` ve `workspace.git_worktree`. `pr.*`, `prompt_cache.last_miss_cause.causes` ve `rate_limits.spend_limit.*` gözlenemedi; belgeye dayalı olarak uygulandı ve `data-sources.md` S1'de işaretli. PR, oturum ortasında açık PR'ı olan bir worktree'ye geçince gelmedi; Claude Code aramayı büyük olasılıkla oturum başında yapıyor.
  - **Esnek okuma** iki yardımcıyla: `lax` (yanlış tipte alan → `None`, payload'ın geri kalanı okunur) ve `seq_len` (`added_dirs` dizisini tamponlamadan sayar). Bilinmeyen efor düzeyi, kaçırma nedeni ve inceleme durumu geldiği gibi gösterilir.
  - **Sekiz öğe, on bir biçim.** Öğeler: `effort`, `cache_timer`, `cache_misses`, `recache_cost`, `spend_limit`, `pr`, `worktree`, `repo`. Mevcut öğelerin yeni biçimleri: `model` `with_effort`, `cwd`/`project_dir` `with_added`, `cache_ratio` `session`. Eski öğelerin varsayılan biçimi değişmedi; yeni alanlar yokken eski satırlar birebir aynı.
  - **Harcama limiti limit gibi, pencere gibi değil.** Eşik renkleri ve sıfırlanma biçimi (geri sayım / saat) var. Zaman işareti ve projeksiyon yok, çünkü dönemin uzunluğu her zaman bilinmiyor. Editör sıfırlanma bloğunu `reset_formats` boş değilse, tempo anahtarını yalnızca pencerelerde gösterir. Sıfırlanma geçmişse tahmini %0 ve dolar tutarı düşer.
  - **Glifler (plandan sapma):** `✗` ve `⚡` Cascadia'da yok, `•` Ambiguous. Yerlerine `⨯` (U+2A2F) ve `∙` (U+2219) kullanıldı; hızlı mod metin ve Unicode setlerinde sözcükle, Nerd setinde roketle (U+F427) gösterilir. `✓`, `⨯`, `∙` cjk açık ve kapalı 1 hücre (test).
  - **`cache_timer` bu adımda biçim listesi almadı** (plan: `clock`). Tek seçenekli bir seçici göstermemek için; geri sayım 12r'de gelince liste `clock` ve `countdown` olur.
  - **Ölçüm** (`docs/perf/2026-11-11-statusline-payload-fields.md`): CLI 2,51 MiB (+52 KiB). Statusline p95 açık 13,6–18,5 ms, kapalı 26,7–31,5 ms; `main` ile aynı düzey, bütçede.
- **Sonuçlar (Adım 12r, 2026-11-11):**
  - **Heartbeat kalır (kullanıcı kararı).** Limit motorunun 5 dakikalık heartbeat yazımı değişmedi. Aynı içerikteki push store'a yazılmaz; tekrar, serileştirilmiş payload'ın hash'iyle (`content_key`) anlaşılır, çünkü boştayken yalnızca serileştirilmeyen süre alanları ilerler. Plandan sapma: daemon tekrarda snapshot'ı tamamen atlamak yerine **en fazla dakikada bir** yeniden kurar; panel ve tazelik bilgisi böylece eskimez.
  - **Spool tekrarı** yalnızca son satıra bakar (16 KiB kuyruk, kilitsiz) ve yalnızca heartbeat süresi içinde atlanır; daha eski bir aynı satır yeniden yazılır, böylece uygulama kapalıyken de geçmişte 5 dakikadan büyük boşluk oluşmaz.
  - **Kurulum kaydına yeni alan gerekmedi.** Kurulum `statusLine`'ın önceki değerini birebir sakladığı için kaldırma, kullanıcının kendi `refreshInterval`'ını zaten geri getirir. Mevcut bir `statusLine` nesnesine kurulum yalnızca `type`, `command` ve bizim `refreshInterval`'ımızı değiştirir.
  - **Yalnızca uygulama ayarı değişiyorsa** (köprü `settings.json`'da değilse ya da dosya zaten aynı değeri taşıyorsa) diff gösterilmez, ayar doğrudan kaydedilir. Ayar ekranı dosyadaki değer seçimden farklıysa bunu söyler; Claude Code dosyaya uyar.
  - **Glifler (plandan sapma):** `↑`, `↓` ve `●` Doğu Asya genişliğinde "Ambiguous". Yerlerine `⇡` (U+21E1), `⇣` (U+21E3) ve `*` (Nerd setinde oct-dot_fill U+F444) kullanıldı. Çakışma `!N`, temiz ağaç `✓`. Taranmayan izlenmeyen dosyalar plandaki `?…` yerine `??` ile gösterilir, çünkü `…` de Ambiguous. Hepsi cjk açık ve kapalı 1 hücre (test); Cascadia Mono ve CaskaydiaMono NF'de var.
  - **Bayat sonuç sunulur (stale-while-revalidate, plana ek):** TTL dolunca yeni hesap arka planda sürerken en fazla 1 dakikalık eski sonuç gösterilir; `HEAD` ya da `index` değiştiyse gösterilmez. Aksi hâlde öğe her 5 saniyede bir çizimlik kaybolurdu.
  - **`cache_timer` geri sayımı** TTL'yi göstermez (`cache 4m`); saat biçimi varsayılan kalır.
  - **Doctor** her Claude dizini için seçilen ve `settings.json`'daki aralığı, `git` bulunup bulunmadığını ve çalışma klasöründe izlenmeyen dosya taramasının kapanıp kapanmadığını yazar (yol ve sayı yazmaz).
  - **Ölçüm** (`docs/perf/2026-11-11-live-refresh-git-status.md`): CLI 2,54 MiB (+30,5 KiB). Statusline p95 açık 20,3–21,6 ms (`git_status` ekliyken de), kapalı medyan ≈ 33,0 ms (11 koşu) (`main` ≈ 30,7 ms, 10 koşu); bütçede. 12r'nin kapalı koşularından 3'ü (11'de) tümüyle yavaşladı (p50 de yükseldi, p95 59–65 ms); `main`'de görülmedi, nedeni açıklanamadı. Git komutunun kendisi bu depoda p95 ≈ 48 ms; CLI onu beklemez. Sentetik boştaki oturum (10 dk): 5 sn yenilemede çekirdek %0,010 CPU, 64,9 uyanma/dk ve store'a yalnızca heartbeat; `git_status` ile %0,013, ama 5 sn TTL ve 5 sn yenileme birleşince git neredeyse her çizimde çalışır (≈ 126 süreç / 10 dk).

### ADR-060 — Statusline ve API'nin tam sayı okumaları alt sınır olarak birleştirilir

**Durum:** Kabul edildi (2026-11-12, Adım 12s)

- **Bağlam:**
  - Panel haftalık limiti Claude Code'un `/usage` ekranından 1 puan düşük gösterebiliyordu. API yoklamasından hemen sonra 1 puan yükselip sonra geri iniyordu (12r devir notu).
  - Gerçek `limit_snapshots`'ta statusline ve API değerleri hep tam sayı. Aynı penceredeki çiftlerde `statusline − API` yalnızca 0 ya da −1 (kabaca yarı yarıya), +1 hiç yok. Bu, statusline'ın aşağı, API'nin en yakın tam sayıya yuvarladığına uyuyor. Belgelenmiş değil; yalnızca `weekly_all` için doğrulanabildi (`data-sources.md` S1 "Yuvarlama").
  - Eski kural (30 s içinde statusline, sonra en yeni okuma) iki yuvarlamayı sırayla gösteriyordu.
- **Karar:**
  - **Her okuma gerçek değer için bir aralıktır:** statusline `n` → `[n, n+1)`, API `m` → `[m−0,5, m+0,5)`. Tam sayı olmayan değerler ve Codex okumaları (Adım 20'ye kadar) tam değer sayılır.
  - **Monoton alt sınır (kullanıcı kararı).** Kullanım pencere içinde azalmaz. Bu yüzden bir okumanın alt sınırı pencere boyunca geçerli kalır, yalnızca üst sınırı eskir. Gösterilen değer, penceredeki alt sınırların en büyüğünün API gibi yuvarlanmış hâlidir; pratikte `max(statusline, API)`. Plandaki "API okumasının geçerlilik süresi" bu yüzden gereksiz.
  - **Çelişki:** Bilinen en yeni okumadan daha yeni bir okumanın üst sınırı alt sınıra eşit ya da altındaysa (gerçek bir düşüş ya da yuvarlama değişikliği) sınır o okumadan yeniden başlar. Daha eski okumalar yalnızca sınırı yükseltebilir; son yeniden başlamadan önceki okumalar sayılmaz.
  - **Kaynak etiketi (kullanıcı kararı):** Rozet öndeki okumanın kaynağıdır (en yeni; 30 s içinde daha iyi kaynak). `LimitView.combined_with`, değeri öndeki okumanın tek başına söyleyeceğinden yukarı taşıyan ikinci kaynağı verir; ipucu "Canlı ve API birlikte" der. Değer bir ölçümdür, `≈` almaz.
  - **Hız ve öngörü birleşik değerle beslenir (kullanıcı kararı).** Geçmiş, her yeni okumadan sonraki birleşik değeri tutar; daha eski okumalar geçmişe girmez.
  - **Saklama değişmez (kullanıcı kararı).** `limit_snapshots` her kaynağın ham değerini tutar, heartbeat (ADR-028) aynıdır. Motor açılışta pencerenin snapshot'larını (bir haftalık tek okuma) yeniden katlar.
  - **Aynı kural her yüzeyde:** eşik olayları, tray, panel, Ayrıntı grafiği, `history --json`, uygulama kapalıyken CLI `status` (kaynak başına son okuma, `fuse_latest`). Statusline satırı payload'ın `n` değerini, uygulamanın aynı penceredeki birleşik değeri `(n, n+1]` aralığındaysa ona yükseltir (kullanıcı kararı).
- **Bedel:**
  - Statusline bir tam sayıyı geçtiği anda (`[n, n+1)`) `/usage` `n` ya da `n+1` gösterebilir. Hangisi olduğu tahmin yapmadan bilinemez. Panel bir sonraki API yoklamasına kadar (statusline tazeyken 15 dk) `n`'de kalır. Fark artık gidip gelmez; yalnızca bu aralıkta 1 puan geride kalabilir.
  - Pencere içindeki 1 puanlık gerçek bir düşüş, bir sonraki API okumasına kadar görünmez (statusline `m−1`, API `m` ile tutarlı sayılır).
  - Varsayım belgelenmemiş bir davranışa dayanıyor. Claude Code yuvarlamayı değiştirirse çelişki kuralı sınırı yeniden başlatır, ama değer bir süre 1 puan yüksek kalabilir.
  - Motor açılışı bir haftalık snapshot okur (gerçek DB kopyasında medyan 1,97 → 3,39 ms; `docs/perf/2026-11-12-limit-rounding-fusion.md`).
- **Alternatifler:**
  - Plandaki kesişim ve API geçerlilik süresi: API eskiyince panel yine 1 puan aşağı iner.
  - API'yi hep tercih etmek: statusline'ın tazeliği kaybolur, artışlar 15 dakikaya kadar gecikir.
  - Statusline değerine +0,5 eklemek: ölçümü tahminle değiştirir.
  - Hiçbir şey yapmamak ve ipucunda açıklamak: gidip gelme sürer.
  - Birleşik değeri saklamak: şema değişikliği getirir, ham veriyi yeniden yorumlama imkânını kaldırır.

### ADR-061 — Kalibrasyon kenar aralıklarından öğrenilir; aile çarpanları veriden gelir, kalibre öngörü düşük güvende kalır

**Durum:** Kabul edildi (2026-11-19, Adım 13). "(kullanıcı kararı)" işaretli maddeler 2026-11-18'de kullanıcıyla kararlaştırıldı; kalan parametreleri (güven formülü, CUSUM, saklama kuralları, kullanım anı güveni) kullanıcı 2026-11-19'da onayladı.

- **Bağlam:**
  - Plan (`architecture.md` §9.2) ondalıklı statusline okumaları, rastgele snapshot çiftleri ve `Δp ≥ 3` birleştirmesi varsayıyordu. Statusline ve API tam sayı veriyor (ADR-060); rastgele çift ±1 puan kuantizasyon hatası taşır.
  - Ön keşif ve 2026-11-18 veri keşfi (salt okunur): statusline'daki "düşüşlerin" tamamı bayat payload taşıyan başka oturumlardan geliyor. Fable ağırlıklı aralıklarda `Δp/ΔC` yaklaşık 2 kat; cache okuma ağırlığı farkı açıklamıyor. Tek `k` ile dağılım ≈ %12, `α_fable` ile ≈ %5. Dış tüketim payı ihmal edilebilir (`seven_day_breakdown` ≈ tümüyle `claude_code`).
  - Hesapta haftada tek pencere var, 5 saatlik pencere yok (ADR-041); plandaki "pencere sonu MAE ≤ 3 (5 sa)" ölçülemez.
- **Karar:**
  - **Gözlem birimi: kenar aralıkları (kullanıcı kararı).** Pencerenin birikimli maksimumunun her adımı bir kenardır; zamanı adımdan önceki son ve sonraki ilk okumanın orta noktasıdır. Belirsizliği 45 dakikayı aşan kenarlar birleştirilerek atlanır; pencerenin ilk okuması kenar değildir. Pencereler `WINDOW_MATCH_TOLERANCE_MS` ile eşlenir (bayat oturumlar önceki pencereyi bildirebilir). `ΔC` iki kenar arasındaki (`from < ts ≤ to`) yerel olayların aile başına maliyetidir.
  - **Kaynaklar karışmaz (kullanıcı kararı).** Bir limitin en az 10 statusline aralığı varsa statusline serisi, yoksa API serisi kullanılır. Pratikte `weekly_all` statusline'dan, `weekly_scoped` API'den.
  - **Model `Δp ≈ k · Σ α_f · ΔC_f`, `α_opus = 1`; `α` veriden öğrenilir (kullanıcı kararı). H1 (tek `k`) reddedildi.**
    - `k`: oranların ağırlıklı medyanı. Ağırlık ham maliyet × üstel sönüm (yarı ömür 3 gün). Ağırlık `α`'ya bağlı değildir; bağlı olunca küçük bir `α` ailenin aralıklarını medyandan dışarı itip uyumu "iyileştiriyordu" (walk-forward'da `α_fable` 0,25'e çöküyordu).
    - `α_f`: 0,25–8 arası log ızgarada (2^(i/10)) iki tur koordinat araması, ağırlıklı göreli MAD'i en aza indirir. Bir aile ancak en az 5 aralıkta maliyetin ≥ %30'unu taşıyorsa ve en az 5 aralıkta yoksa kendi ağırlığını alır. 1'e %15'ten yakın sonuç 1'e yuvarlanır. Synthetic ve Unknown aileleri her zaman 1'dir.
    - Kapsamlı limit (`model:<aile>`) yalnızca kendi ailesinin maliyetini sayar ve `α` öğrenmez.
    - Fiyatsız modellerin maliyet payı %10'u aşan aralıklar ve maliyeti medyanın %5'inin altında kalan aralıklar atılır.
  - **Güven** = sayı × kesinlik × tazelik × öğrenilmiş pay:
    - sayı `min(1, n/30)`;
    - kesinlik `1 − se/0,15`, `se = 1,4826 · MAD_göreli / √n_etkin`;
    - tazelik, en yeni aralık bir günden eskiyse her 3 günde yarıya iner;
    - öğrenilmiş pay, kendi ağırlığı gereken ama öğrenilmemiş ailelerin maliyet payının tümleyenidir.
    - Eşikler: < 0,4 düşük, 0,4–0,7 orta, > 0,7 yüksek.
  - **Kullanım anı güveni.** Kalibrasyonu son 15 dakikanın maliyetine uygularken güven, o maliyette ağırlığı bilinmeyen ailelerin payı kadar düşürülür (`Calibration::confidence_for`). Backtest'te Fable'ın henüz görülmediği günlerde öğrenilen güven yüksekken 24 sa hatası −3 puana iniyordu.
  - **Sapma: `ln(r / k_önce) / σ` üzerinde CUSUM.**
    - `σ = max(1,4826 · MAD(ln r), 0,15)`; z ±3'te kırpılır; gevşeklik 0,5; alarm eşiği 6; ilk 10 gözlem ısınmadır, en az 3 temel gözlem gerekir.
    - Alarmdan sonra `k` değişimin başından itibaren 1 günlük yarı ömürle yeniden öğrenilir; `α` tam veriyle öğrenilmiş hâlinde kalır.
    - Sapma 3 gün görünür kalır.
    - Eşik 5 iken gerçek veride iki kısa yanlış alarm çıktı; 6'da hiç çıkmadı, sentetik algılama 5–12 saat sürdü.
  - **Kalıcılık.** Mevcut `calibrations` tablosu kullanılır, şema değişmez. `version` hesap başına `MAX + 1`. `alpha_json` `{alpha, fitted, source, mad_rel, drift}` taşır. Yeni satır şu durumlarda yazılır: `k` ya da bir `α` ≥ %2 oynadığında, güven düzeyi ya da sapma değiştiğinde, ya da son satır bir günden eskiyse.
  - **Yeniden hesaplama.** Limit thread'inde, açılıştan 30 s sonra ve sonra 15 dakikada bir, yalnızca hesabın en yeni snapshot'ı değiştiyse. Geriye bakış 21 gündür. Açılışta saklı son satırlar yüklenir.
  - **Öngörüde kullanım (kullanıcı kararı).** Yalnızca `session` ve `weekly_all` için, gözlenen eğim yokken kullanılan maliyet yedeğinde kullanılır: kullanım anı güveni en az orta ise `k0` yerine `k · Σ α · c_f` (son 15 dakika), yöntem `RateMethod::Calibrated`. Öngörünün güveni yine `Low` kalır; bildirim ve mod davranışı değişmez.
  - **Kapsamlı limite maliyet yedeği eklenmez (kullanıcı kararı);** Adım 12t'ye bırakıldı.
  - **Görünüm (kullanıcı kararı).** `LimitView.calibration` (`confidence`, `level`, `n_obs`, `drift`). Ayrıntı'da öngörü ipucu güveni ve aralık sayısını gösterir; sapma varsa tek satırlık bilgi çıkar. `tokenradar doctor` kalibrasyonu paketine ve terminale yazar. Ana kart değişmez. `get_calibration_status` komutu ve `CoreEvent::CalibrationUpdated` olayı eklendi.
  - **Kabul ölçütü (kullanıcı kararı):** pencere içi walk-forward, 24 sa ufuk, `k` ve `α` yalnızca geçmişten. Ölçüt MAE ≤ 1,5 puan ve |sapma| ≤ 0,5.
- **Sonuç** (`docs/perf/2026-11-19-calibration-backtest.md`):
  - `weekly_all`: MAE 0,75, sapma −0,46. H1: MAE 1,58.
  - Fable kapsamlı: MAE 0,75, sapma −0,02.
  - Sentetik veri (40 tohum): `k` %4,7, `α` %20 içinde; yanlış sapma yok.
  - Bir hesaplama gerçek veride ≈ 20 ms.
- **Bedel:**
  - `weekly_all` sapması sınıra yakın (−0,46). Öğrenilmemiş ailenin yoğun kullanıldığı dönemlerde tahmin düşük kalır; uygulama bu dönemlerde `k0`'a döner, ama Ayrıntı ipucu öğrenilen güveni gösterir.
  - `α_sonnet` gibi az verili aileler, eşikleri geçtiği anda kendi ağırlığını alır; tek bir yanlı dönem onu oynatabilir.
  - Kenar zamanı orta nokta varsayımına dayanır. API serisinde kenar belirsizliği statusline'ınkinden ≈ 6 kat büyüktür, bu yüzden kapsamlı limitin güveni düşük kalır.
  - Model formülünü Anthropic değiştirirse ilk yarım gün eski `k` kullanılır (sapma algılama 5–12 sa).
- **Alternatifler:**
  - Plandaki snapshot çiftleri ve `Δp ≥ 3`: kuantizasyon hatası, iki kaynağın yuvarlama farkı.
  - H1 (tek `k`): 24 sa MAE 1,58, ölçütü geçmez.
  - `α`'yı sabitlemek (ör. tüm veriden 2,2): sızıntılı; walk-forward'da iyimser sonuç verir.
  - Kalibre öngörüye orta güven vermek: bildirim ve mod davranışını değiştirirdi; kullanıcı istemedi.
  - Cache okuma ağırlığını yeniden ölçeklemek: Fable farkını açıklamadı.

### ADR-062 — Tam sayı kaynakların hızı adım kenarlarından hesaplanır; öngörü uyarısı güvene ve ufka bağlıdır

**Durum:** Kabul edildi (2026-11-20, Adım 12t). "(kullanıcı kararı)" işaretli maddeler 2026-11-13 ve 2026-11-20'de kullanıcıyla kararlaştırıldı; kalan uygulama ayrıntılarını kullanıcı 2026-11-20'de onayladı.

- **Bağlam:**
  - `rate.rs` hızı son 30 dakikanın birleşik değerlerinden Theil–Sen eğimiyle hesaplıyordu. Statusline ve API tam sayı veriyor (ADR-060), API ≈ 15 dakikada bir yoklanıyor. 30 dakikada en fazla bir adım görülüyor: adım yoksa öngörü kayboluyor, varsa hız "1 puan / iki okuma arası" çıkıp gerçeğin birkaç katı oluyor.
  - Claude Code mod'u `will_hit_before_reset`'i güven ve ufuk filtresi olmadan kullanıyordu; bant, adım pencereye girip çıktıkça açılıp kapanıyordu (2026-11-13 bulgusu). Uygulama bildirimi `Low` güveni ve 60 dakikadan uzak ETA'yı zaten yok sayıyordu.
  - Birleşik değer API'nin yuvarlamasıyla gösteriliyor: `weekly_all`'da statusline `n` eşiğini, API `m − 0,5` eşiğini geçince adım üretiyor. Birleşik serinin kenarları farklı eşiklere karşılık geliyor.
- **Karar:**
  - **Kenarlar kaynak başına, ham okumalardan (kullanıcı kararı).** Birikimli maksimumun yükseldiği an kenardır; zamanı eski değeri gösteren son okuma ile yeni değeri gösteren ilk okumanın orta noktasıdır. Daha düşük okuma (boştaki oturumun bayat payload'u) yok sayılır, eşit okuma "son görülme"yi ilerletir. Belirsizliği 45 dakikayı (`MAX_EDGE_UNCERTAINTY_MS`, ADR-061 ile aynı) aşan kenar tutulmaz; komşuları onu daha büyük puan farkıyla kapsar. Pencerenin ilk okuması kenar değildir.
  - **Hız = (son − ilk kenarın puanı) / aradaki süre, pencerenin son en fazla 4 keskin kenarından (kullanıcı kararı).** Bellek kaynak başına 4 kenar; Theil–Sen'in O(n²) çifti uzun geçmişte kullanılmaz.
  - **Durma tavanı (kullanıcı kararı):** son kenardan bu yana aynı değer okunuyorsa `hız ≤ 1 puan / (son okuma − son kenar)`. Kullanım durunca öngörü kendiliğinden söner. Tavan `now`'a değil son okumaya bağlıdır: okuma gelmiyorsa (401 bekleyişi) hız uydurulmaz, bayatlık zaten gösterilir.
  - **Güven (kullanıcı kararı):** 1 aralık `Low`, 2 aralık `Medium`, 3 aralık `High`. İlk ve son kenarın ortalama zaman belirsizliği aralığın %25'ini aşarsa bir düzey düşer. Tek kenardan hız çıkmaz.
  - **Kaynak seçimi (kullanıcı kararı):** statusline ve API'nin ikisi de kenar verirse kenarı çok olan, eşitlikte son kenarı yeni olan.
  - **Öncelik (kullanıcı kararı):** ondalıklı okumaların Theil–Sen eğimi > kenar hızı (≥ `Medium`) > kalibre maliyet yedeği (`Low`, ADR-061) > kenar hızı (`Low`) > `k0` önseli. Theil–Sen artık yalnızca ondalıklı okumalarla beslenir (`fusion::interval(...).exact`); tam sayı okumalar yalnızca kenarlara gider.
  - **Mod uyarısı uygulamanın kuralıyla (kullanıcı kararı):** öngörü bant ve toast'u yalnızca güven `Low` değilse ve ETA 60 dakika (`PROJECTION_HORIZON_MIN`) içindeyse açar. `projection_confidence` yoksa (eski CLI) açmaz. Mod'un kendi sabiti `PROJECTION_HORIZON_MS` uygulamanınkiyle aynı değerdedir.
  - **Kapsamlı limitin maliyet yedeği bu adımda eklenmez (kullanıcı kararı).**
  - **Statusline'ın dolma süresi öğesi gösterge olarak kalır (kullanıcı kararı, 2026-11-20).** Ufuk uygulanmaz, uzak ETA da görünür. Her zaman `≈` ile yazılır (`▲≈38m`; önceden işaretsizdi). Güven `Low` ya da bilinmiyorsa uyarı rengi yerine silik çizilir (`statusline::line::Eta.rough`).
  - **Uygulama ayrıntıları (kullanıcı 2026-11-20'de onayladı):**
    - `RateMethod::Steps` eklendi (bindings'te `"steps"`); arayüz `method`'u göstermediği için görünür bir değişiklik yok.
    - Mod, kararı `mergeRows` içinde bir kez verip satıra `soon` olarak yazar; `rowLevel` ve `alertsOf` buna bakar.
    - Mod panelinde ve `/radar` raporunda düşük güvenli öngörü "(kaba)" / "(rough)" ekiyle gösterilir; ayrıntı satırı zaten silik. Uygulamanın Ayrıntı ve ana kartı düşük güveni zaten silik ve `≈` ile gösteriyordu, değişmedi.
    - `calibration::intervals::Edge` yeniden kullanıldı; `Confidence::lower` eklendi.
- **Sonuç** (`docs/perf/2026-11-20-step-edge-rate.md`, gerçek verinin salt okunur kopyası, biçim olarak):
  - Eski yöntem, kullanım olan anlarda sonraki 3 saatin gerçek hızının birkaç katını (≈ 3–6×) okuyordu. Yeni yöntem gerçeğin altında ama aynı mertebede (≈ 0,4–0,6×).
  - Sonraki 3 saatin mutlak hatası (kullanım olmayan anlar dahil) yeni yöntemde eskisinden düşük ya da eşit.
  - `will_hit_before_reset` gidip gelmeleri eskisinin ≈ %2–10'una indi.
  - Sentetik regresyon (5–19 dk aralıklı tam sayı API okumaları): hız gerçeğin %20'si içinde, öngörü bir kez açılıyor ve gidip gelmiyor.
- **Bedel:**
  - Kenar hızı geçmişe bakar: bir moladan sonraki ani yükselişi ancak iki-üç kenar sonra görür. Gerçek veride aktif anlarda hızı düşük okuma eğilimi bunun ve durma tavanının sonucudur; uyarı daha geç gelebilir.
  - Yavaş kullanılan kapsamlı limitte (API, seyrek adım) kenar hızı daha az zaman var; eskiden görünen (ama birkaç kat yüksek) öngörü artık çoğu zaman yok.
  - Kenar zamanı orta nokta varsayımına dayanır (ADR-061 ile aynı).
- **Alternatifler:**
  - Yalnızca `RATE_LOOKBACK_MS`'yi uzatmak: adım sayısı artar ama kenar belirsizliği sürer; hız değişimine geç tepki verir.
  - Theil–Sen'i API okumalarının aralık orta noktasıyla beslemek: yuvarlama ofsetini düzeltir, adım/pencere oranı sorununu çözmez.
  - Kenar noktalarına Theil–Sen (kullanıcı ilk–son kenarı seçti): az kenarda aynı sonuç, daha çok kod.
  - Sabit süreli geriye bakış (ör. 3 sa): yavaş kullanımda kenar bulamaz, hızlı kullanımda eski veri taşır.
  - Güveni yalnızca kenar sayısıyla vermek: 15 dk belirsizlikli tek aralık 30 dk sürse ±%50 hatayla `Medium` görünürdü.
  - Mod'da yalnızca metni yumuşatmak, ya da ufku kalan sürenin oranı yapmak: gidip gelme sürer / uygulama bildirimiyle ayrışır.

### ADR-063 — Haftalık limitlerde öngörü uyarısı bir gün ileriye bakar, yalnızca yüksek güvenle

**Durum:** Kabul (2026-11-20, Adım 12t ek işi; kullanıcı kararı). ADR-062'deki "mod uyarısı uygulamanın kuralıyla: 60 dakika, güven `Low` değil" maddesinin haftalık limitler için yerini alır; 5 saatlik pencere ve diğer limitler için o madde geçerli kalır.

- **Bağlam:**
  - Uygulama bildiriminin 60 dakikalık ufku (`PROJECTION_HORIZON_MIN`, Adım 11) 5 saatlik pencere için tasarlanmıştı. Haftalık kullanım saatte birkaç puan arttığı için "60 dakikada dolar" ancak %97–99 civarında söylenebiliyor; bildirim %75 ve %90 eşik uyarılarından sonra geliyor ve yeni bir şey söylemiyor.
  - 12t'den önce mod bandı haftalık pencerede erken açılıyordu, ama güven ve ufuk filtresi olmadığı için yanlış alarm veriyor ve gidip geliyordu (ADR-062 bulgusu). Uygulama bildirimi haftalık pencerede hiçbir zaman erken uyarmadı.
  - Hesapta 5 saatlik pencere yok (ADR-041); haftalık pencere tek pencere.
- **Karar (kullanıcı kararı):**
  - `weekly_all` ve `weekly_scoped` için öngörü uyarısı ETA **24 saat** (`WEEKLY_PROJECTION_HORIZON_MIN`) içindeyse ve güven **`High`** ise çıkar. Diğer limitlerde 60 dakika ve en az `Medium` kalır.
  - Kural tek yerde: `notifier::projection_horizon(kind) → (ufuk, en düşük güven)`. Claude Code mod'u aynı kuralı `warnsSoon` içinde uygular (`WEEKLY_PROJECTION_HORIZON_MS`; haftalık = pencere uzunluğu 7 gün olan satır).
  - Son uyarı (`final_warning_min`), eşik uyarıları ve "pencere başına bir kez" kuralı değişmez. Statusline göstergesi etkilenmez (ADR-062).
  - `Confidence` sıralı hâle getirildi (`Low < Medium < High`).
  - Ayarlardaki ipucu metni haftalık kuralı söyler (EN/TR).
- **Bedel:**
  - **24 saat gerçek veriyle seçilmedi.** Kullanıcı değeri hemen istedi; `rate_compare` ile bir ufuk taraması yapılmadı.
  - **7/24 varsayımı:** 24 saatlik öngörü o anki hızın gece de süreceğini varsayar (örüntü verisi Adım 17'de). Gün içindeki yoğun bir çalışma limiti olduğundan çabuk dolacakmış gibi gösterebilir. Yoğun çalışma kısa sürede çok sayıda keskin kenar ürettiği için `High` şartı bunu tamamen engellemez.
  - **Tek hak erken harcanabilir:** "dolacak" bildirimi pencere başına bir kez gider. Erken bir yoğunlukta giderse, hafta sonuna doğru gerçekten dolarken ikinci bir "dolacak" bildirimi gelmez; yalnızca son uyarı ve eşik uyarıları kalır.
  - Ters yönde: yeni hız yöntemi gerçek veride hızı düşük okuyor (≈ 0,5×, ADR-062), bu uyarıyı geciktirir. İki hata birbirini kısmen dengeler; net etki ölçülmedi.
  - Birkaç hafta haftalık bildirimin ne zaman geldiği izlenmeli; gerekirse ufuk kısaltılır ya da yeni bir ADR ile "bir kez" kuralı haftalıkta gevşetilir.
- **Alternatifler:**
  - Haftalıkta da 60 dakika (ADR-062): uyarı fiilen eşik uyarılarından sonra gelir.
  - Kalan pencere süresinin oranı: pencere sonuna yaklaştıkça ufuk kısalır; kullanıcı sabit bir gün istedi.
  - Haftalık erken uyarıyı Adım 17'nin örüntü verisine bırakmak: o zamana kadar yalnızca mod'un tempo uyarısı (ADR-044) erken uyarır.

### ADR-064 — Atıf 24 saatlik dilimlerde yapılır; "Diğer" yalnızca güvenilir kalibrasyonla, çapraz doğrulama tek yönlü

**Durum:** Kabul edildi (2026-11-20). "(kullanıcı kararı)" işaretli maddeler Adım 14a'da, "Uygulama ayrıntıları" Adım 14b'nin başında kullanıcıyla kararlaştırıldı. Mimari §9.3'teki "aralık bazlı" v0.2 formülünün ve iki yönlü çapraz doğrulamanın yerini alır. "Zamanlama" maddesindeki "kapanmış pencereler yalnızca görüntülenince yeniden hesaplanır" kuralının yerini ADR-065 alır (arka planda yakalama).

- **Bağlam:**
  - Plan "Diğer"i her kenar aralığında `yerel = min(Δp, k·ΔC)` ile hesaplıyordu. Gerçek verinin kopyasında aralık başına oranın gürültüsü (yuvarlama, MAD ≈ %4) bu `min` yüzünden tek yönlü olarak "Diğer"e akıyor. `seven_day_breakdown`'a göre dış kullanımın ≈ %0–1 olduğu bir haftada "Diğer" aralık başına %4,3, 24 sa dilimde %0,9 çıkıyor. Fable kapsamlı limitte (API, güven 0,29) aralık başına %13–19 çıkıyor (`examples/attr_probe.rs`).
  - Hesapta 5 saatlik pencere yok (ADR-041). 5 saatlik pencerenin başlangıç hizalama kuralı veriyle doğrulanamıyor.
  - `seven_day_breakdown` kullanımı makineye göre değil ürüne göre ayırır: başka bir bilgisayardaki Claude Code orada `claude_code` sayılır (bir yan ajanın notu).
  - **Yuvarlama ofseti (bu adımın bulgusu):** Her pencerede ilk kenarlar, pencere başından itibaren birikimli `k·Σα·C` ≈ 0,5, 1,5, 2,5 … iken geliyor. Bu hem API serisinde hem `weekly_all`'ın statusline serisinde böyle. İç parçaların oranı ≈ 1,0, baş parçasınınki 1,1–1,2. Kalibrasyon farklarla çalıştığı için bundan etkilenmiyor. Nedeni belirsiz: ya statusline da en yakına yuvarlıyor, ya pencere başında ≈ 0,5 puanlık bir ofset var. ADR-060'ın ikili karşılaştırması (`S1 − S2 ∈ {0, −1}`) aşağı yuvarlamaya uyduğu için iki gözlem ancak bir ofsetle birlikte açıklanabiliyor. ADR-060 bu adımda değişmedi.
- **Karar:**
  - **Parçalar 24 saatlik dilimlerden (kullanıcı kararı).** Pencerenin kaynağı, o pencerede keskin kenarı (≤ 45 dk belirsizlik) çok olan kaynaktır; eşitlikte son kenarı yeni olan seçilir. Kenarlar pencere başlangıcına hizalı 24 sa dilimlere gruplanır. Parça, bir dilimin son kenarından kenarı olan sonraki dilimin son kenarına uzanır. Baş parçası pencere başlangıcından (%0) başlar. 5 saatlik pencerede dilim pencerenin kendisidir.
  - **Kalibre bölüşüm (kullanıcı kararı):** Kalibrasyonun bu penceredeki güveni (`confidence_for`, öğrenilmemiş aile payı kadar düşer) en az orta ise her parçada `yerel = min(Δp, k·Σα·ΔC)` hesaplanır. Yerel pay oturum ağaçlarına (`COALESCE(root_id, id)`) α ağırlıklı maliyetle bölünür, kalan "Diğer"e gider. Yerel maliyeti olmayan parçanın tamamı "Diğer"dir.
  - **Düşük güvende orantılı (kullanıcı kararı):** Güven ortanın altındaysa ya da kalibrasyon yoksa pencerenin yüzdesi pencere boyunca maliyet oranıyla dağıtılır (v0.1). "Diğer" hesaplanmaz; yalnızca hiç yerel maliyet yoksa yüzdenin tamamı "Diğer" olur. Görünüm `≈` ve silik gösterir.
  - **Korunum:** Σ oturum payı + Diğer = `peak_percent` (gösterilen birleşik değer, ADR-060). Testte tolerans 0,01 puan; özellik testi ve gerçek verinin kopyasında fark < 10⁻⁴.
  - **Tahmini geçmiş pencereler yalnızca `weekly_all` (kullanıcı kararı).** Bilinen haftalık pencerelerin reset zamanı 7'şer gün geriye taşınır (kullanım başlangıcına kadar), bilinen pencereler arasındaki boşluklar da doldurulur (`estimated = 1`). Kalibrasyon kullanılabilirse yüzde `k·Σα·C`, `peak_percent` bu tahmindir ve "Diğer" yoktur. Kullanılamazsa yalnızca maliyet gösterilir. Kapsamlı limitin snapshot'tan önce var olup olmadığı bilinmediği için türetilmez. 5 saatlik türetme yapılmaz, kod yolu da yoktur.
  - **Çapraz doğrulama tek yönlü (kullanıcı kararı):** Yalnızca `weekly_all`'da ve kalibre yöntemle yapılır. Pencerenin son `seven_day_breakdown`'ı alınır (başlangıç ±5 dk). `Diğer/toplam < Σ claude_code dışı − 10 puan` ise `suspect`. Bu, kalibrasyonun yerel kullanımı fazla saydığını gösterir ve makine sayısından bağımsızdır. `Diğer/toplam > Σ claude_code dışı + 10 puan` yalnızca `other_excess` bilgisidir (başka makine ya da kalibrasyon), uyarı üretmez.
  - **Maliyet kaynağı `usage_5m` (kullanıcı kararı).** Ham olaylar 90 günde silinir, rollup silinmez. Kova, başlangıcı parçanın son kenarından önceyse o parçaya sayılır. Subagent kırılımı için tablo tutulmaz.
  - **Şema v3 (kullanıcı kararı, `0003_attribution.sql`):** Hiç yazılmamış `attributions` yeniden kuruldu (`est_percent` NULL olabilir, `ON DELETE CASCADE`, "Diğer" = `root_session_id` NULL). Yeni `attribution_runs` tablosu pencere başına kalibrasyon sürümünü (0 = yok), yöntemi, güveni, toplamı, "Diğer"i, kırılım değerini, `suspect`, `other_excess`, `closed` ve `computed_at` alanlarını tutar. v3 veritabanını v2 build'ler açamaz.
  - **Zamanlama (kullanıcı kararı):** Boşta uyanma eklenmez. Limit thread'i kalibrasyon turunda (açılıştan 30 s sonra, sonra 15 dk'da bir) tahmini pencereleri ekler ve atıfları günceller. Yeni snapshot ya da kullanım (`Observe`, `Statusline`, `Poll`, `Indexed`, spool) geldiğinde en fazla 60 sn'de bir yeniden günceller. Atıfı olmayan pencere hesaplanır. Kapanan pencere bir kez sonlandırılır (`closed = 1`). Açık pencere yalnızca yeni veriyle güncellenir. Kalibrasyon sürümü değişince kapanmış pencereler yalnızca görüntülenince (`attribution::ensure`) yeniden hesaplanır.
  - **Uygulama ayrıntıları (kullanıcı kararı, 2026-11-20, Adım 14b başı):** Ofset deneysel sabit olarak kalır ve `attr_inspect --trace` ile izlenir; ADR-060 ile çelişkisi birkaç haftalık veriyle ayrı bir adımda çözülür. `CalibrationSuspect` bildirim üretmez.
    - **Kuyruk son parçaya katılır.** Son kenardan sonraki bölüm (`peak − son kenar` ve maliyeti) son parçanın parçasıdır. Tek başına bırakılınca diğer kaynağın yuvarlaması (ADR-060) `min` ile "Diğer"e akıyordu: açık haftada %6,8 → %5,1.
    - **Baş parçasından 0,5 puan düşülür** (`ROUNDING_OFFSET`, yukarıdaki bulgu). Bu yarım puan kullanım değil yuvarlama sayılır ve oturum paylarına orantılı eklenir; korunum gösterilen değere göre kalır. Gerçek veride "Diğer/toplam" %7,1 / %2,3 / %5,1'den %6,1 / %1,3 / %2,5'e indi; kırılım ≈ %1–4 / %0–1 / %0–3.
    - Bir `refresh` en fazla 64 pencere hesaplar (`MAX_PER_REFRESH`), en yeni pencereden başlar; kalan varsa 60 sn sonra devam eder. Uzun bir geçmişte ilk çalışma limit thread'ini bekletmez.
    - `CoreEvent::CalibrationSuspect { account, window_id }` pencere başına bir kez yayınlanır; UI'da gösterimi 14b/15'te.
    - Tahmini pencerede `hit_limit` sütunu 0 kalır (tablo yeniden kurulmaz); görünüm modeli bunu "bilinmiyor" verecek (14b).
- **Sonuç** (gerçek verinin salt okunur kopyası; yalnızca yüzde, `docs/perf/2026-11-20-attribution.md`):
  - Kapanmış 4 ve açık 2 pencerenin hepsinde korunum tam. "Diğer"in kırılımdan farkı ≤ 5,1 puan; uyarı yok.
  - Sentetik simülatörde (8 tohum × dış pay %0/5/15/30, 5 dk'lık en yakına yuvarlanmış okumalar, %4 gürültü) "Diğer" dış payı ±2 puan içinde yakalıyor.
  - Pencere başına hesap ≤ 4,4 ms, ilk dolum (12 pencere) 38 ms, değişiklik yokken tur 0,1 ms.
- **Bedel:**
  - "Diğer" en fazla bir günlük çözünürlükte bilinir. Bir gün içindeki kısa dış kullanım, aynı günün yerel gürültüsüyle karışır.
  - Yuvarlama ofseti deneysel bir sabittir. Kaynak ya da Anthropic'in hesaplaması değişirse baş parçası yeniden kayar; `attr_inspect --trace` bunu gösterir.
  - Orantılı yöntem (kapsamlı limitte bugün geçerli olan) dış kullanımı oturumlara yükler.
  - Tahmini haftaların yüzdesi bugünkü `k` ile hesaplanır; aylar önceki bir kalibrasyon sapması görünmez.
  - Kalibrasyon sürümü değişince kapanmış pencerelerin listelerdeki payları, pencereler görüntülenene kadar eski sürümle kalır. (ADR-065: kalibrasyon turu bunları arka planda yakalar; arada liste satırı `stale` taşır.)
- **Alternatifler:**
  - Aralık başına (plan): sistematik +3–4 puan (kapsamlı limitte +10–15).
  - 6 saatlik dilim: aralığa göre iyi ama 24 saatten belirgin kötü (%1,7 vs %0,9).
  - Pencere bütünü: en az gürültü, ama "Diğer"in ne zaman oluştuğu kaybolur ve oturum payları düz orantıya iner.
  - İki yönlü çapraz doğrulama (plan): başka makinede Claude Code kullanan herkeste sahte uyarı.
  - Ham olay kaynağı: kenar zamanı tam isabetli, ama 90 günden eski pencereler yeniden hesaplanamaz.
  - Ofseti "Diğer"e bırakmak: her pencerede ≈ 0,5 puan sahte dış kullanım.

### ADR-065 — Sorgu katmanı: saklanan atıfı okur, keyset sayfalar, geniş aralıkta zaman indeksini atlar

**Durum:** Önerildi (2026-11-20, Adım 14b). "(kullanıcı kararı)" işaretli maddeler 2026-11-20'de kullanıcıyla kararlaştırıldı; diğerleri bu adımın önerisidir ve kullanıcı onayı bekler. ADR-064'ün "kapanmış pencereler yalnızca görüntülenince yeniden hesaplanır" kuralının ve plandaki jiff `TimeZone::system()` notunun yerini alır.

- **Bağlam:**
  - Adım 15–17'nin sayfaları (Genel Bakış, pencere arşivi, oturumlar, projeler, ısı haritası) aynı görünüm modellerini okuyacak. Plan her sorgu için 1 yıllık sentetik veride ≤ 50 ms (p95) istiyor.
  - Atıf 14a'da pencere başına saklanıyor (`attributions`, `attribution_runs`). Kalibrasyon sürümü değişince kapanmış pencereler yalnızca görüntülenince yeniden hesaplanıyordu; listeler eski değeri gösterebiliyordu.
  - `usage_5m`'nin birincil anahtarı `(account_id, session_id, model_id, bucket_ts)`, ikincil indeksi `ix_5m_acct_bucket (account_id, bucket_ts)`. İlk ölçümde oturum listesi 2,1 s, oturum detayı 80 ms, zaman serisi 215 ms sürdü.
  - jiff'in `TimeZone::system()` işlevi Windows'ta gömülü bir tzdb ister (ikili boyutu artar); planın notu bunu varsaymıştı.
- **Karar:**
  - **Modül:** `tokenradar_core::queries` (Tauri'siz). Her çağrı bir `QueryContext { account, now, calibrations, zone }` alır. Sorgular: `list_windows`, `get_window`, `list_sessions`, `get_session`, `get_session_impact`, `list_projects`, `get_project`, `get_timeseries`, `get_heatmap`, `get_overview`.
  - **Bütün sorgular 14b'de Tauri komutu olur (kullanıcı kararı).** `app/src-tauri/src/history.rs` bağlamı çalışan çekirdekten kurar: birincil hesap, son kalibrasyonlar, bilgisayarın saat dilimi, canlı oturumlar ve plan. Sorgu `spawn_blocking` ile çalışır ve çekirdeğin kilidini tutmaz (`Store` klonlanır). Bindings yenilendi.
  - **Eskimiş atıflar arka planda yakalanır (kullanıcı kararı).** `attribution::refresh(…, catch_up)`: kalibrasyon turu, kalibrasyon sürümü eskimiş kapanmış pencereleri de yeniden hesaplar (en yeniden başlar, turda en fazla 64). Yeni veri geldiğinde yapılan güncelleme bunu yapmaz. Listeler saklanan değeri okur; sürümü güncel olmayan satır `Basis.stale` taşır. `get_window` önce `ensure` çağırır.
  - **"Bu hafta" açık `weekly_all` penceresidir (kullanıcı kararı).** "Geçen hafta", `resets_at`'i bu haftanın başlangıcına denk gelen penceredir. Haftalık penceresi olmayan hesapta son 7 gün ve önceki 7 gün kullanılır; o zaman yüzde yoktur.
  - **Plan ücreti gömülü tablodan gelir, kullanıcı değeri önce gelir (kullanıcı kararı).**
    - `pricing/plans.toml` claude.com/pricing sayfasından 2026-11-20'de doğrulandı: Ultra 200 $ (`max` + `max_20x`), Max 100 $, Pro 20 $. Koltuk başı fiyatlanan Team ve Enterprise tabloda yoktur.
    - `config.toml`'daki `[[accounts]] plan_fee_usd` tablonun önüne geçer; pozitif bir sayı değilse yok sayılır.
    - Oran = yerel takvim ayının API karşılığı maliyeti / aylık ücret.
  - **Dürüst sayı kuralı (`Basis`):**
    - Her yüzde `method`, `confidence`, `approx` (`method != calibrated`), `dim` (kalibre değilse ya da güven düşükse) ve `stale` taşır.
    - Tahmini pencerede `hit_limit` `None` ("bilinmiyor") olur.
    - Isı haritasının yüzde metriği `k·α·maliyet`tir, yöntemi `estimated`. `weekly_all` kalibrasyonu düşük güvendeyse yüzde verilmez (`cost_only`).
  - **Keyset sayfalama:**
    - İmleç `"{anahtar}:{id}"` biçimindedir (anahtar f64). SQL koşulu `k < ?k OR (k = ?k AND id < ?id)`, sıra azalan.
    - Arada yeni satır gelse de sayfa kaymaz, satır tekrarlanmaz. Bozuk imleç baştan başlar.
    - Sayfa boyutu varsayılan 50, en fazla 200.
  - **Oturum ağacı:**
    - Subagent ayrı satır olmaz. Maliyeti, modeli ve projesi ana oturumun hesabına yazılır; subagent'ın kendi `project_id`'si yoktur.
    - Oturum adı başlıktır. Başlık yoksa `proje · uuid'nin ilk 8 karakteri` kullanılır (`fallback`).
  - **Yerel saat (`LocalZone`):**
    - Windows'ta anlık ofset `SystemTimeToTzSpecificLocalTime` ile alınır. Diğer sistemlerde Adım 21'e kadar UTC kullanılır.
    - Aralık sabit ofsetli parçalara bölünür: saatlik yoklanır, değişim dakikası ikili aramayla bulunur. Saat kovası `(bucket_ts + ofset) / saat`.
    - Yarım saatlik dilimler ve yaz saati geçişleri doğru kovaya düşer.
  - **İndeks kuralı:**
    - Bir oturum ağacını adlandıran sorgular ve 60 günden geniş aralıklar `+u.bucket_ts` yazar. Böylece SQLite zaman indeksini değil birincil anahtarı kullanır.
    - Gerekçe: zaman indeksi satırları zaman sırasıyla gezer ve her biri için birincil anahtara bakar. Yoğun bir yılda ≈ 90 günden sonra bu, tüm tabloyu okumaktan pahalıdır.
    - Toplamalar önce oturum başına anahtar sırasıyla (`MATERIALIZED` CTE), sonra ağaç ya da proje başına yapılır.
    - Isı haritası satırları okuyup saat başına Rust'ta toplar. Saate göre `GROUP BY` her satırı geçici ağaçta sıralıyordu.
    - Sonuç: oturum listesi 2,1 s'den 35 ms'ye, oturum detayı 80 ms'den 0,3 ms'ye indi.
  - **Zaman serisi:**
    - Yalnızca istenen limitin okumaları `ix_snap` boyunca okunur, pencere başına füzyonlanır (ADR-060) ve LTTB ile en fazla 1.500 noktaya seyreltilir.
    - Oturum işaretleri limitin atıflarından gelir: aralıktaki pencerelerde payı olan en fazla 200 ağaç, maliyete göre sıralı.
    - İşaretin başlangıcı ve bitişi, oturumun ömrünün aralıkla kesişimidir.
- **Sonuç** (`docs/perf/2026-11-20-queries.md`):
  - Sentetik yılda (3.000 ana oturum, ≈ 2.000 subagent, ayda ≈ 150 bin olay, 72 bin okuma, 104 pencere) her sorgunun p95'i ≤ 39 ms. Zaman serisi 26 ms, ısı haritası 25 ms, Genel Bakış 4 ms, oturum detayı 0,7 ms.
  - Gerçek verinin kopyasında her sorgu ≤ 18 ms.
- **Bedel:**
  - Tüm yılı kapsayan liste sorguları tabloyu okur (sentetik yılda 30–39 ms). Süre `usage_5m` satır sayısıyla doğrusal büyür. Bütçe aşılırsa sonraki adım oturum × gün rollup tablosudur (yeni migration).
  - Arka planda yakalama, kalibrasyon değişince 15 dakikaya kadar eski değeri `stale` olarak gösterir.
  - Oturum işareti yalnızca atıfı olan pencerelerden gelir. Atıfı henüz yazılmamış pencerenin oturumları işaret almaz.
  - macOS ve Linux'ta Adım 21'e kadar saat kovaları UTC'dir.
- **Alternatifler:**
  - jiff `TimeZone::system()`: Windows'ta gömülü tzdb gerekir.
  - OFFSET sayfalama: plan yasaklıyor; arada gelen satır sayfayı kaydırır.
  - Listelerde `ensure`: bir listede onlarca pencereyi yeniden hesaplamak sayfayı bekletir.
  - Hemen rollup tablosu: ölçüm, bütçeyi şema değişikliği olmadan karşıladı.
  - İşaretleri kullanımdan hesaplamak: yıllık aralıkta 46 ms sürüyor; atıflar zaten pencere başına toplanmış durumda.

### ADR-066 — Ana pencere kabuğu: native başlık, deep link, kv'de geometri, limite bağlı grafik rengi

**Durum:** Önerildi (2026-11-20, Adım 15a). "(kullanıcı kararı)" işaretli maddeler 2026-11-20'de kullanıcıyla kararlaştırıldı. Diğerleri bu adımın önerisidir ve kullanıcı onayı bekler.

- **Bağlam:**
  - Adım 15 tek oturuma sığmıyordu: kabuk, yaşam döngüsü, grafik sarmalayıcısı ve Genel Bakış'ın altı bölümü.
  - Ana pencere üç yerden açılır: tray, panel ve CLI. CLI ayrı bir süreçtir ve uygulamaya pipe'tan değil, başlatma argümanıyla ulaşabilir.
  - Hesapta 5 saatlik pencere yok (ADR-041). 7 günlük grafikte çoğu zaman tek çizgi, haftalık limit kalır.
- **Karar:**
  - **Adım 15, 15a + 15b'ye bölünür (kullanıcı kararı).**
    - 15a: `main.html`, pencere yaşam döngüsü, kabuk ve router, kısayollar, uPlot sarmalayıcısı ve galeri, üç giriş noktası, boş rotalar.
    - 15b: Genel Bakış'ın bölümleri, boş durumlar, plan ücreti ayarı, performans ölçümü.
  - **Native başlık çubuğu + Mica (kullanıcı kararı).** Başlık `document.title`'ı izler (`on_document_title_changed`).
  - **Panelde başlıkta simge düğme (kullanıcı kararı).** Adı ve ipucu "TokenRadar'ı aç"; tray menüsündeki öğe de aynı adı taşır.
  - **Deep link modeli çekirdekte:** `tokenradar://panel` ve `tokenradar://main[/<sayfa>]` (`tokenradar_core::deeplink`). Tek örnek argümanları, ilk başlatma ve toast aynı ayrıştırmayı kullanır. Sorgu ve fazladan bölüm reddedilir.
  - **`tokenradar open [sayfa]`:** CLI, uygulamayı kendi klasöründe ya da bir üst klasörde bulur ve ayrık süreç olarak bağlantıyla başlatır. Uygulama çalışıyorsa bağlantıyı tek örnek eklentisi iletir. Pipe protokolü değişmez.
  - **Geometri `kv`'de:** `tauri-plugin-window-state` yerine `kv.main_window_geometry`. Ek bağımlılık gerekmez, ayar dosyası yazılmaz ve kayıtlı konuma başlık şeridi erişilebilirlik kontrolü uygulanır.
  - **Gizli oluştur, ilk boyamada göster:** `main_ready` ya da 1,5 sn zaman aşımı. Kapatma pencereyi gizler, 30 sn sonra webview yok edilir (mimari §2).
  - **Grafik rengi limite bağlıdır, sıraya değil (kullanıcı kararı, 2026-11-20: "en uygun ve tutarlı olan"; tek çizgide camgöbeği seçeneği reddedildi):** 5 saatlik `--signal-ok`, haftalık yeni `--series-2` (mavi), "Diğer" `--signal-other`. Bir filtre ya da 5 saatlik pencerenin yokluğu kalan çizgiyi yeniden boyamaz (dataviz kuralı). `--series-2` iki temada doğrulayıcıdan geçti: camgöbeğine karşı CVD ΔE 19,7 / 14,7, normal görüşte 21,9 / 17,6, Mica zemininde kontrast ≥ 3.
  - **Bundle:** `main.html` için 180 KB JS bütçesi, giriş + tüm tembel sayfalarla birlikte sayılır (en kötü durum). İlk ölçüm 60,4 KB.
- **Bedel:**
  - Hesapta 5 saatlik pencere olmadığı için 7 günlük grafiğin tek çizgisi mavi. Uygulamanın geri kalanında veri mürekkebi camgöbeği olduğundan bu ilk bakışta tutarsız görünebilir.
  - CLI, uygulamanın kurulum düzenine (aynı ya da üst klasör) bağlıdır; taşınmış bir CLI uygulamayı bulamaz ve bunu söyler.
  - uPlot imleç için grafik alanının ekran dikdörtgenini önbelleğe alır; grafik her kurulduğunda ve boyutlandığında `syncRect()` çağrılır (Adım 15a'da bulunan hata).
- **Alternatifler:**
  - Özel başlık çubuğu: Windows 11'in yakalama düzenleri ve erişilebilirlik için daha fazla iş.
  - Tek çizgi kalınca camgöbeği: renk sıraya bağlanmış olur, 5 saatlik pencere geri geldiğinde haftalık çizgi renk değiştirir.
  - CLI'dan pipe ile `open` işlemi: protokol sürümü değişirdi ve uygulama kapalıyken yine başlatma gerekirdi.
  - `tauri-plugin-window-state`: geometriyi kendi dosyasına yazar, ekran dışı konum kontrolü sınırlıdır.

### ADR-067 — Genel Bakış: yukarıdan aşağı yerleşim, haftanın aynı anıyla karşılaştırma, ay ilerlemesiyle plan değeri

**Durum:** Kabul edildi (2026-11-20, Adım 15b). "(kullanıcı kararı)" işaretli maddeler adım sırasında kullanıcıyla kararlaştırıldı. Diğer maddeler (token satırları, oturum şeridi, 10 sn kısma, `overview:data` ölçümü ve kalanlar) adım sonunda kullanıcı tarafından onaylandı.

- **Bağlam:**
  - Genel Bakış dört soruyu yanıtlar: şu an neredeyim, bu hafta nasıl gidiyor, limiti ne tüketiyor, ne kadar değer alıyorum. Altı bölüm tek sayfaya sığmalı.
  - Haftanın maliyeti süren bir pencereden gelir. Bunu geçen haftanın tamamıyla karşılaştırmak, hafta başında her zaman büyük bir düşüş gösterir.
  - Bu ayın maliyeti de ay sürerken birikir. Tam aylık ücrete oranlamak, ay başında oranı yanıltıcı biçimde düşük gösterir (yan ajan notu, kullanıcı iletti).
  - Kalibrasyonun ne zaman "iyi" olacağı tahmin edilemez: gözlem sayısına değil, okumaların ve kullanımın çeşitliliğine bağlıdır.
- **Karar:**
  - **Yerleşim yukarıdan aşağıdır (kullanıcı kararı).**
    - Sıra: Şimdi → Son 7 gün (grafik + oturum şeridi) → Bu hafta (maliyet ▲▼ | token türleri, model karışımı) → En çok tüketenler (Oturumlar | Projeler) → alt sırada Diğer | Plan değeri.
    - Taslak: design-language §5.
  - **Hafta, geçen haftanın aynı anıyla karşılaştırılır (kullanıcı kararı).**
    - Çekirdek yeni alanlar verir: `previous_cost_to_date` (geçen haftanın başından, bu haftada geçen süre kadarı) ve `change` (`cost / previous_cost_to_date − 1`). Geçen hafta 0 ise `change` boştur.
    - Geçen haftanın tamamı (`previous_cost`) bilgi olarak altında gösterilir.
    - Değişim yarım yüzdenin altındaysa "aynı" sayılır.
  - **Kalibrasyon boş durumunda gün yoktur, durum vardır (kullanıcı kararı).** Metin güven düzeyini ve öğrenilen aralık sayısını söyler ("Şu an düşük güven, 14 aralık"). "≈ X gün sonra" yazılmaz.
  - **Plan ücreti ayarı Kaynaklar sekmesinde, hesabın altındadır (kullanıcı kararı).**
    - Boş bırakılırsa gömülü plan tablosunun liste fiyatı kullanılır. Değer birincil hesabın `plan_fee_usd` alanına yazılır (config.toml, `save_plan_fee`).
    - Genel Bakış'taki "Plan ücretini değiştir" bağlantısı ayarları `plan` bölümünde açar (`SettingsSection::Plan`).
  - **Plan değeri, geçen ayın tam oranını ve ayın ilerlemesini gösterir (kullanıcı kararı).**
    - Çekirdek yeni alanlar verir: `month_elapsed` (0–1), `previous_month_cost` ve `previous_ratio`. Geçen ay maliyeti 0 ise ikisi de boştur.
    - Kart "Ayın %24'ü geçti · Geçen ay ×3,2 (API $640,00)" der. Bu ayın oranı tam ücrete göre kalır; ücret ay günlerine bölünmez.
  - **Token türleri satır olarak gösterilir, yığılmış çubuk olarak değil.** Önbellek okuması diğer türleri bastırır; yığılmış çubukta girdi ve çıktı görünmez olur. Her satır en büyük türe göre boyutlanır ve tek renk taşır (dataviz: büyüklük tek renk). Model karışımı aynı bileşenle, ilk 5 model ve "diğer modeller" satırıyla gösterilir.
  - **Oturum şeridi grafiğin içindedir.** Haftalık limitte payı en büyük 5 oturum, grafiğin altında aynı zaman ekseninde 4 px'lik çubuklar olarak çizilir. Çubuklar odaklanabilir düğmelerdir ve ortak ipucunda oturum adını, zaman aralığını ve `≈` payı gösterir. Konum uPlot'un çizim alanından (`bbox`) okunur.
  - **Canlı yenileme kısılır.** Sayfa `usage_appended`, `limits_updated`, `calibration_updated` ve `calibration_suspect` olaylarında en fazla 10 saniyede bir yeniden yüklenir. Pencere yeniden görünür olunca hemen yüklenir.
  - **Paylar her zaman `≈` taşır.** Kalibrasyon iyi değilse ve `basis.dim` ise soluk gösterilir; altındaki not payların nasıl bölündüğünü söyler. Eski kalibrasyonla hesaplanmış satırlar varsa ayrı bir not çıkar.
  - **"Diğer" kartı iki kaynağı yan yana koyar.** Kendi tahminimiz (yalnızca kullanılabilir kalibrasyonla, ADR-064) ve Anthropic'in `seven_day_breakdown`'ı. Aralarında 10 puandan fazla fark varsa not gösterilir.
  - **Sayfa pencere genişliğini doldurur (kullanıcı geri bildirimi, 2026-11-20):** İlk sürümdeki `max-width: 1200px` tam ekranda sağda boşluk bırakıyordu ve kaldırıldı. Izgaralar dar pencerede taşmaz (`minmax(min(…, 100%), 1fr)`).
  - **Ölçüm işareti:** Genel Bakış verisi ilk kez boyandığında `overview:data` ölçümü (gezinmenin başından) kaydedilir. `main_ready` iskelet boyandığında gelir; bütçe, verinin boyandığı anla ölçülür.
- **Sonuç** (`docs/perf/2026-11-20-main-window.md`): ikinci süreç dahil soğuk açılış medyan 434 ms (≤ 600 ms), veri boyanması 161 ms; rota geçişi medyan 9,5 ms; bellek en fazla 129,2 MB (≤ 180 MB); `main.html` JS 77,9 KB (≤ 180 KB).
- **Bedel:**
  - Haftanın aynı anıyla karşılaştırma, kullanımı haftanın başına yığan biri için hafta ortasında büyük artış, sonunda düşüş gösterebilir. Tam hafta bu yüzden altta bilgi olarak durur.
  - Ay ilerlemesi yerel saate göre hesaplanır. Ücret takvim ayına göre değil fatura dönemine göre işliyorsa ilerleme birkaç gün kayabilir.
  - Oturum şeridi en fazla 5 oturum gösterir. Kısa oturumlar 6 px'lik en küçük genişlikle çizilir, bu yüzden süreleri gerçeğinden uzun görünür.
  - Geniş pencerede büyüklük çubukları çok uzar; etiketle değer arası açılır.
- **Alternatifler:**
  - Sekmeli ya da iki sütunlu yerleşim: sayfanın "yukarıdan aşağı" okunan sırası bozulur (kullanıcı reddetti).
  - Geçen haftanın tamamıyla karşılaştırma: hafta başında her zaman düşüş gösterir.
  - Günlere bölünmüş ücret: oran ay boyunca dalgalanır, kullanıcının bildiği "aylık ücret" görünmez olur.
  - "≈ X gün sonra kalibre olur": dürüst hesaplanamaz.
  - Token türleri için halka grafik: büyüklük karşılaştırmasında açılar zor okunur (dataviz).
  - Her olayda yeniden yükleme: indeksleme sırasında saniyede birkaç sorgu.

### ADR-068 — Oturumlar: şema v4 öznitelikleri, veritabanı beyaz listeli klasör açma, aileye bağlı maliyet renkleri

**Durum:** Kabul edildi (2026-11-21, Adım 16a). "(kullanıcı kararı)" işaretli maddeler adım başında kullanıcıyla kararlaştırıldı. Diğer maddeler (aile renkleri ve canvas grafik, filtrelerin URL biçimi, yerel proje seçici, 10 sn canlı yenileme kısması ve kalanlar) adım sonunda kullanıcı tarafından onaylandı.

- **Bağlam:**
  - Plan, oturum ayrıntısında efor, hızlı mod ve farklı `cwd` değerlerini istiyor. Çekirdekte bunların hiçbiri saklanmıyordu.
  - Ön keşif (yalnızca meta veri anahtarları sayıldı):
    - Asistan kayıtları `effort` taşıyor.
    - Son 400 transcript'in 303'ünde birden fazla farklı `cwd` vardı.
    - `speed:"fast"` hiç görülmedi.
  - Ham olaylar 90 gün saklanır. Claude Code ise transcript'leri varsayılan olarak 30 gün sonra siler.
  - "Klasörü aç" düğmesi, pencereye dosya sistemi açma izni vermeden çalışmalı.
  - Maliyet grafiği model başına yığılmalı. Mevcut iki seri rengi (`--signal-ok`, `--series-2`) buna yetmiyor.
- **Karar:**
  - **Adım ikiye bölünür (kullanıcı kararı).**
    - 16a: sanal liste, Oturumlar listesi ve ayrıntısı, şema v4, rozetler, klasör açma.
    - 16b: Projeler, geçici dizin filtresi ve ayarlardaki desen düzenleyicisi, klasör ağacı, Genel Bakış bağlantıları, gerçek veriyle performans ölçümü.
  - **Öznitelikler şema v4 ile saklanır (kullanıcı kararı).**
    - `session_attrs(session_id, kind, value, first_ts, last_ts)` tablosu her oturum için `cwd`, `effort` ve `speed` (yalnızca `fast`) değerlerini, ilk ve son görüldükleri anla tutar.
    - Saklama işi bu tabloyu silmez. Böylece hızlı mod ve klasörler ham olaylardan uzun yaşar.
    - Göç canlı dosyaların okuma konumunu sıfırlar (v2'deki gibi). Yeniden okunan olaylar tekilleşmede düşer.
  - **`effort` esnek okunur.** Metin olarak gelirse alınır. Nesne, sayı ya da başka bir tür gelirse yok sayılır (`LooseStr`). Gerçek verideki 3.346 satırda `effort` bir nesneydi.
  - **"Transcript silindi" `source_files`'tan hesaplanır.** Oturumun indekslenmiş dosyası varsa ve hepsi silinmişse rozet gösterilir. Bunun için yeni bir indeks eklendi: `ix_source_session`.
  - **Klasör arka uçtan açılır (kullanıcı kararı).**
    - `open_session_dir(id, path)` yalnızca veritabanının o oturum ağacı için bildiği yolları açar: `session_attrs` içindeki `cwd` değerleri ve çözülmüş proje yolu.
    - Listede olmayan bir yol reddedilir ve uyarı günlüğe yazılır. Dizin değilse `"missing"` döner.
    - Pencereye opener izni verilmez.
  - **Maliyet grafiği model ailesine göre yığılır.**
    - Renkler sabittir ve rengi sıra değil aile belirler: Opus `--series-2`, Fable `--series-3`, Sonnet `--series-4`, Haiku `--series-5`, kalanlar `--text-3`.
    - Üç yeni token dataviz doğrulayıcısından tüm çiftlerle ve iki temada geçti. `TimelineBucket` artık `family` alanını taşır.
    - Çizim canvas üzerindedir. uPlot'ta yığılmış çubuk için ayrı bir eklenti gerekirdi.
  - **Filtreler URL parçasındadır.**
    - Biçim: `#sessions?range=7d&project=12&model=opus&live=1&q=…&sort=cost`.
    - Yalnızca varsayılan dışı değerler sabit sırayla yazılır. Okunamayan değer varsayılana düşer.
    - Her filtre değişikliği geçmişe bir kayıt ekler. Arama ise 200 ms debounce ile parçayı yerinde değiştirir.
  - **Sanal liste sona yaklaşınca sayfa ister.** Çizilen dilim sona 6 satırdan fazla yaklaşınca bir sonraki sayfa istenir. Planda `IntersectionObserver` yazıyordu; dilim zaten bilindiği için gözlemciye gerek kalmadı.
  - **Proje seçici yerel `<select>`'tir.** İlk harflerle arama yapılabilir. Aranabilir bir combobox, proje sayısı büyüyünce 16b'de yeniden değerlendirilir.
  - **Canlı yenileme kısılır.** Liste `live_sessions_changed` ve `usage_appended` olaylarında en fazla 10 saniyede bir yenilenir. Yüklenmiş satırlar korunur. Ayrıntı sayfası yalnızca oturum çalışıyorsa yenilenir.
- **Sonuç** (`docs/perf/2026-11-20-sessions.md`):
  - 10.000 satırlık listede kaydırma adımı başına iş: p95 1,7 ms, en kötü 6,4 ms (< 16 ms). Bu ölçüm stil, yerleşim ve Svelte güncellemesini kapsar; boyama dahil değildir.
  - `main.html` JS, tüm tembel sayfalarla 95,3 KB (≤ 180 KB). Oturumlar parçası 12,2 KB.
  - Tam indeksleme (yükseltmeden sonraki ilk açılış): 7,66 GiB'ta 20,1 s; `main` 17,1 s (≤ 60 s). Aynı veriyi okuyan koşularda parmak izi aynı.
- **Bedel:**
  - Efor ve klasörler yalnızca bu sürüm ilk okuduğunda transcript'i hâlâ diskte olan oturumlar için bilinir. Yaklaşık 30 günden eski oturumlarda alan boş kalır ve ayrıntı sayfası nedenini söyler (yan ajan notu, kullanıcı iletti).
  - Yükseltmeden sonraki ilk açılış bir tam indeksleme kadar sürer: bugün diskteki 7,66 GiB transcript yeniden okunur (yaklaşık 20 s).
  - Şema v4'e yükseltilmiş bir veritabanını eski sürümler açamaz (ADR-054'teki gibi).
  - Aile renkleri dört aileyle sınırlıdır. Yeni bir aile, token eklenene kadar "Diğer modeller"e düşer.
- **Alternatifler:**
  - Öznitelikleri sorgu anında transcript'ten okumak: konuşma içeriğinin bulunduğu dosyayı yeniden açmak gerekir ve silinmiş dosyalarda çalışmaz.
  - Yalnızca ilk `cwd`: oturumların çoğu dizin değiştiriyor.
  - Pencereye opener kapsamı vermek: web içeriği herhangi bir yolu açabilirdi.
  - Model başına renk: model sayısı sınırsız ve renk oturumdan oturuma değişirdi.

### ADR-069 — Projeler: SQL'de geçici dizin filtresi, çekirdekte klasör ağacı, şema v5 görünen yol

**Durum:** Kabul edildi (2026-11-21, Adım 16b). "(kullanıcı kararı)" işaretli maddeler adım sırasında kullanıcıyla kararlaştırıldı. Diğer maddeler (proje listesinin varsayılanları, URL biçimi, düz listede satırın bağlantı olmaması, ağaçta ← →, gizli sayısı sorgusu, ayrıntı yerleşimi, varsayılan desenlerin arka uçtan gelmesi) adım sonunda kullanıcı tarafından onaylandı.

- **Bağlam:**
  - Gerçek veride (salt-okunur sayım) projelerin yaklaşık %80'i `AppData\Local\Temp` altında. Bunlar Claude Code'u geçici bir klasörde çalıştıran araçların bıraktığı tek oturumluk projeler.
  - `projects.path_norm` küçük harfli bir eşleştirme anahtarı. Ekranda gösterilince yollar küçük harfle görünüyordu (yan ajan notu, kullanıcı iletti).
  - Planın kabul kriteri: gerçek veride filtre yanıtı ≤ 150 ms, kaydırma kare < 16 ms.
- **Karar:**
  - **Desenler yol parçasıdır (kullanıcı kararı).**
    - Desen yolun herhangi bir yerinde eşleşir. `*` herhangi bir metin, `%AD%` bir ortam değişkenidir; büyük/küçük harf duyarsızdır.
    - Varsayılanlar `%TEMP%` ve `\Temp\`; liste Ayarlar → Gizlilik ve veri'de düzenlenir (`[data] temp_dirs`).
    - Filtre çekirdekte SQL `LIKE … ESCAPE '\'` ile uygulanır. Tanımsız bir değişken deseni düşürür, yani hiçbir şeyi gizlemez.
  - **Kapsam (kullanıcı kararı):** Gizleme yalnızca Projeler sayfasını ve Oturumlar listesinin proje seçicisini etkiler. Oturum toplamları ve Genel Bakış hiçbir oturumu gizlemez.
  - **Klasör ağacı çekirdekte kurulur (kullanıcı kararı).** Her klasör alt ağacının toplamlarını taşır. Projesi olmayan tek çocuklu klasörler birleşir. Arayüz yalnızca açık/kapalı durumunu tutar.
  - **Şema v5: `projects.display_path` (kullanıcı kararı).**
    - Yol, Claude Code'un yazdığı haliyle saklanır; sayfa ve ağaç onu gösterir.
    - Göç canlı dosyaları yeniden okutur. Kurulu sürüm v2'de olduğundan bu, v4'ün tam indekslemesiyle aynı geçiştir.
    - Transcript'i silinmiş eski projeler küçük harfle kalır.
  - **Gerçek veri ölçümü portable kopyayla yapılır (kullanıcı kararı).** Kurulu uygulamayı kullanıcı kapatıp açar.
  - **"Haftalık" sütunu pencere başına ortalamadır (kullanıcı kararı, yan ajan notu üzerine).** Değer, aralıktaki haftalık pencerelerden alınan payların toplamının, aralığın dokunduğu haftalık pencere sayısına bölümüdür ve `≈%4,5/hafta` biçiminde gösterilir. Payda tüm satırlar için aynıdır: klasör değeri projelerinkinin toplamıdır, sıralama değişmez, uzun aralıkta %100 aşılmaz. Ayrıntı başlığı tüm zamanların ortalamasını gösterir.
  - **Gizli sayısı ayrı ve ucuz bir sorgudur.** Liste, ilk sayfada aralıkta kullanımı olan kaç projenin gizlendiğini döner ve sayfa bunu "Göster" bağlantısıyla söyler. Sayım, listenin toplamasını tekrarlamak yerine desene uyan her proje için `usage_5m` birincil anahtarında bir `EXISTS` yoklaması yapar. İlk sürüm 50 ms bütçesini aşmıştı (57 ms → 34 ms).
  - **Proje listesinin varsayılanları:** son 30 gün, son etkinliğe göre sıralı, geçici dizinler gizli, ağaç kapalı.
    - Oturumların varsayılanı 7 gündür. Projeler daha seyrek değişir; 7 günde listenin çoğu boş kalıyordu.
    - Ağaç açıldığında tüm klasörler açık başlar.
  - **Filtreler URL parçasındadır,** oturumlarla aynı kurallarla: `#projects?range=all&q=…&sort=cost&view=tree&temp=1`. Geçici dizinler varsayılan olarak gizli olduğu için `temp=1` "göster" demektir.
  - **Düz listede satır bağlantı değildir.** Oturum listesindeki gibi tıklama ya da Enter ile açılır. Ağaçta proje adı bağlantıdır, klasör satırı açılır/kapanır.
  - **Sanal liste `onkey` ve `select(index)` kazandı.** Ağaç ← ve → tuşlarını bununla işler (WAI-ARIA ağaç deseni): → kapalı klasörü açar, açık klasörde ilk çocuğa iner. ← açık klasörü kapatır, değilse üst klasöre çıkar.
  - **Proje ayrıntısı:**
    - Son 8 haftalık pencerede proje payı çubuklarla gösterilir; renk `--series-2`'dir (haftalık, ADR-066).
    - Tahmini pencereler kesikli çerçevelidir. Hiçbir haftada pay yoksa çubuklar maliyeti gösterir ve not bunu söyler.
    - Altta projeye kilitli oturum listesi vardır. Filtreleri ayrıntının URL parçasında durur.
  - **Varsayılan desenler arka uçtan gelir** (`PrefsView.default_temp_dirs`). "Varsayılanlara dön" düğmesi ön yüzde ikinci bir kopya tutmaz.
- **Sonuç** (`docs/perf/2026-11-21-projects.md`):
  - Gerçek veride filtre yanıtı: Projeler 49–110 ms, Oturumlar 28–113 ms (≤ 150 ms). Ölçüm haftalık ortalama değişikliğinden önce yapıldı.
  - Boyama dahil kare aralığı vsync'te sabit (16,7 ms), atlanan kare yok.
  - Etki tablosu 15 oturumda saklanan atıflarla birebir eşit.
  - Göçten sonraki ilk tam indeksleme ≈ 28 s (≤ 60 s).
  - `main.html` JS 105,1 KB (≤ 180 KB).
- **Bedel:**
  - Şema v5'e yükseltilmiş bir veritabanını eski sürümler açamaz (ADR-054'teki gibi).
  - Desen bir yol parçası olduğu için geniş bir desen (`\Temp\`) adında "Temp" klasörü geçen gerçek bir projeyi de gizler. Not ve "Göster" bağlantısı bunu görünür kılar.
  - Haftalık ortalama, tek haftada yoğun çalışılmış bir projeyi seyreltir. O haftanın payı proje ayrıntısındaki haftalık çubuklarda görünür.
- **Alternatifler:**
  - Düzenli ifade desenleri: güçlü ama hataya açık; SQLite'ta yerleşik değil.
  - Filtreyi arayüzde uygulamak: sayfalama bozulur, her sayfa eksik gelir.
  - Ağacı arayüzde kurmak: arayüzde hesaplama yapılmaz (`docs/architecture.md`, IPC ilkesi); toplamlar için tüm satırlar gerekir.
  - Haftalık sütunda en yoğun hafta: sezgisel, ama klasörlerde projelerin tepe değerleri toplanamaz.
  - Toplamı açık bir etiketle bırakmak: %100'ü aşan değerler kalırdı.
  - Görünen yolu `session_attrs` içindeki `cwd`'den sorgu anında türetmek: projenin her oturumunu taramak gerekir ve silinmiş transcript'lerde yine çalışmaz.

### ADR-070 — Pencereler ve Örüntüler: haftalık öncelikli sayfa, 24 saatlik günler, yalnızca 5 saatlik limit riski, CSV/JSON dışa aktarma

**Durum:** Kabul edildi (2026-11-21, Adım 17a). "(kullanıcı kararı)" işaretli maddeler adımın başında kullanıcıyla kararlaştırıldı; diğerlerini kullanıcı adım sonunda onayladı.

- **Bağlam:**
  - Plan 5 saatlik pencereleri varsayıyordu: 14 günlük şerit ve "en riskli saatler" bunlara dayanıyordu.
  - Kullanıcının hesabı doğrulanmış Ultra: 2026-11-05'ten beri 5 saatlik pencere yok, yalnızca haftalık limitler var (ADR-041).
  - Haftalık pencerenin içindeki gün dağılımı ölçülmez: statusline ve API haftanın toplamını verir, günleri vermez.
- **Karar:**
  - **Adım ikiye bölündü (kullanıcı kararı).** 17a: Pencereler, Örüntüler ve dışa aktarma. 17b: dönemsel özet ve v0.2 sürümü, ayrı bir oturumda.
  - **Pencereler sayfası haftalık önceliklidir (kullanıcı kararı).**
    - Varsayılan görünüm haftalık döngülerdir: her haftalık pencere bir satır.
    - Satırda yedi günlük şerit, tepe, limite takıldığı gün, en çok etkileyen üç proje, kapsamlı limitlerin (Fable) birer sütunu ve Anthropic'in kırılımı vardır.
    - Liste ve ayrıntı çekmecesi her pencere türünde çalışır.
    - 5 saatlik şerit yazıldı, ama sekmesi yalnızca hesapta 5 saatlik pencere varsa görünür. Sentetik üretici 5 saatlik pencere üretir, şerit onunla sınanır.
  - **Limit riski yalnızca 5 saatliktir (kullanıcı kararı).** Genel yazıldı ve sentetik veriyle sınandı. 5 saatlik penceresi olmayan hesapta kart nedenini söyleyen bir boş durum gösterir; haftalık bir karşılık uydurulmaz.
  - **Günler sıfırlanmadan başlayan 24 saatlik dilimlerdir,** takvim günleri değil.
    - Değerleri `≈` tahmindir: dilimin yerel maliyeti × kalibrasyon (`k · α`).
    - "Diğer" (claude.ai, Desktop, başka bilgisayar) dahil değildir, bu yüzden günlerin toplamı ölçülen tepenin altında kalabilir. Tablonun altındaki not bunu söyler.
    - Güvenilir kalibrasyon yoksa günler maliyeti gösterir ve not değişir.
  - **Limite takıldığı an ölçümdür:** pencerenin %100 olan ilk okuması. Tahmini pencerede "Bilinmiyor" yazılır.
  - **Çekmecedeki oturum katkıları:**
    - En büyük dört oturum ve "Diğer oturumlar" yığılı alan olarak çizilir. Dört, çünkü tasarımın doğrulanmış dört kategorik rengi var; beşinci oturum yeni bir renk isterdi.
    - Her oturumun saklanan payı, birikimli maliyetiyle orantılı olarak zamana yayılır. Değerler `≈`'dir, not bunu söyler.
    - En çok 168 kova vardır, her biri 5 dakikanın katıdır. Kapsamlı pencerede yalnızca o modelin ailesi sayılır.
    - Altındaki pay tablosu aynı verinin metin halidir; "Diğer" ayrı satırdadır, çünkü zaman profili yoktur.
  - **Çekmecede bildirimler** `notification_log`'dan gelir. Kayıt 30 gün tutulduğu için eski pencerelerde liste boştur, not bunu söyler.
  - **Riskli saat aralığı kuralı:**
    - En az 10 ölçülmüş 5 saatlik pencere gerekir. Daha azında kart "yeterli veri yok" der.
    - Aralık 3 saattir. En az 5 pencere ve 2 takılma içermeli, oranı genel oranın en az 1,5 katı olmalıdır.
    - Böyle bir aralık yoksa cümle hiçbir saatin öne çıkmadığını söyler.
  - **Haftanın ilk günü ön yüzden gelir:** `Intl.Locale#getWeekInfo`, yoksa tablo (tr → Pazartesi, en → Pazar). Isı haritası satırlarını ve model karışımının haftalarını belirler.
  - **Dışa aktarma:**
    - Oturumlar (CSV/JSON, isteğe bağlı yolları gizleme), pencereler (CSV/JSON) ve günlük toplamlar (CSV).
    - Düğme her sayfada filtre satırındadır ve sayfanın o anki aralığını ve filtrelerini kullanır.
    - CSV: UTF-8 BOM ve CRLF. TR'de `;` ayırıcı ve ondalık virgül, EN'de `,` ve nokta. Başlıklar yerelleştirilir.
    - `= + - @`, sekme ya da satır başıyla başlayan metin hücresinin başına `'` eklenir (formül enjeksiyonu koruması).
    - JSON: İngilizce anahtarlar, ISO UTC zamanlar, günler için yerel tarih.
    - Kaydetme penceresi arka uçta açılır. Dosya yolu yalnızca kullanıcının seçtiği yerdir.
    - İçerik yalnızca kullanım ve meta veridir; istem ve yanıt yoktur.
  - **Varsayılanlar:** son 12 hafta, Pencereler'de haftalık sekmesi, ısı haritasında maliyet. Filtreler URL parçasındadır (`#windows?tab=list&range=all`, `#patterns?range=4w&metric=percent`).
  - **Model karışımı payları gösterir** (haftanın API karşılığı maliyetinin yüzdesi). Mutlak tutar haftadan haftaya çok değiştiği için yığılı payda değişim daha okunur.
- **Sonuç:** `docs/perf/2026-11-21-windows-patterns.md`.
- **Bedel:**
  - Günler ölçüm değildir. Kullanıcı onları tepe değerle birebir karşılaştırmamalıdır; not ve `≈` bunu söyler.
  - Katkıların zamana yayılması bir modeldir: oturumun payı doğru, payın pencere içindeki zamanlaması maliyetle orantılı varsayılır.
  - 30 günden eski pencerelerde bildirim geçmişi yoktur.
- **Alternatifler:**
  - Haftalık pencereler için "riskli saatler" uydurmak: haftalık takılma sayısı birkaç olayı geçmez, istatistik anlamsız olur.
  - Takvim günleri: sıfırlama saatine göre ilk ve son gün kısmi kalır, yedi gün eşit karşılaştırılamaz.
  - Gün değerlerini tepeye ölçeklemek: "Diğer"in gün dağılımı bilinmediği için uydurma olurdu.
  - Beş oturumlu katkı grafiği: beşinci renk doğrulanmış paletin dışında kalırdı.
