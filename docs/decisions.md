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
**Durum:** Kabul (Adım 02 performans kapısına bağlı)
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
