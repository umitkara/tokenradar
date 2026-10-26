# Adım 21 — macOS ve Linux

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.5 | 20 | 2-3 oturum | ⬜ |

> Önerilen bölünme: **21a (CI matrisi + platform soyutlamaları + Linux)**, **21b (macOS + imzalama + v0.5 sürümü)**. Platforma erişim yoksa (örneğin Mac yoksa) ilgili kısım CI ile derlenir, manuel test açık konu olarak işaretlenir ve **kullanıcıya bildirilir**.

## Amaç

Windows'a özel kısımları platform soyutlamalarının arkasına almak ve macOS ile Linux için birinci sınıf destek sağlamak. Her platformda "native kabuk" hissi korunmalı: macOS'ta menü çubuğu, Linux'ta AppIndicator. Performans bütçeleri her platformda ayrı ölçülür.

## Önce oku

- `docs/architecture.md` §2 (süreç modeli), §10 (IPC), §12 (güvenlik), §13 (dosya konumları)
- `docs/performance-budget.md` (platform notları)
- `docs/decisions.md`: ADR-002, ADR-004, ADR-012, ADR-021
- Adım 02 spike raporu (WebView2 sonuçları; WebKit için tekrarlanacak)
- Adım 08 (tray ikonu çizimi), Adım 11 (bildirimler, otomatik başlatma), Adım 12 (paketleme ve updater)

## Kapsam

- **CI matrisi:** `windows-latest`, `macos-latest` (arm64) + `macos-13` (x64) veya universal binary, `ubuntu-22.04`. Derleme, test ve paketleme yapılır.
- **Platform soyutlamaları** (`crates/tokenradar-core/src/platform/`): IPC taşıması, arka plan önceliği, kimlik bilgisi okuma, süreç doğrulama, otomatik başlatma, tam ekran tespiti, dosya konumları.
- **macOS:** menü çubuğu template ikonu, Keychain okuma, `LSUIElement`, vibrancy, imzalama ve notarization, dmg, `SMAppService`.
- **Linux:** AppIndicator / StatusNotifierItem, XDG yolları, Unix socket, AppImage + deb, webkit2gtk ölçümü.
- **Statusline CLI:** Her platformda `tokenradar statusline` komutu. Kurulumda yol tırnaklama kuralları platforma göre değişir.
- v0.5.0 sürümü.

## Kapsam dışı

- Mac App Store ve Flatpak / Snap dağıtımı (talep gelirse ADR ile değerlendirilir).
- Widget'ın macOS ve Linux'a taşınması: Bu adımda yalnızca "çalışıyor mu" kontrolü yapılır. Native çizim yoluna geçildiyse (ADR-011) ayrı bir adım gerekir.
- Wayland'de global kısayol ve tıklama geçirgenliği: Destek sınırlı; belgelenir.

## Görevler

### Ortak: soyutlamalar ve CI

- [ ] Windows'a özel kodu ara: `#[cfg(windows)]`, `windows-sys` çağrıları, `\\.\pipe`, `%LOCALAPPDATA%`. Her biri için bir trait veya `cfg` modülü oluştur. Windows davranışı **değişmez**; regresyon testi zorunlu.
- [ ] **IPC:** Unix'te `$XDG_RUNTIME_DIR/tokenradar.sock` (Linux) veya `~/Library/Application Support/TokenRadar/tokenradar.sock` (macOS).
  - İzinler `0600`; üst dizin `0700`.
  - Bağlantıda `SO_PEERCRED` (Linux) veya `getpeereid` (macOS) ile UID doğrulaması yapılır.
  - NDJSON protokolü aynıdır.
- [ ] **Arka plan önceliği:** Linux'ta `ioprio_set(IOPRIO_CLASS_IDLE)` + `nice(10)` indeksleyici iş parçacığı için; macOS'ta `setpriority(PRIO_DARWIN_THREAD, 0, PRIO_DARWIN_BG)` veya QoS `background`.
- [ ] **Süreç doğrulama (S4):** `sessions/*.json` dosyalarındaki `procStart` değeri Windows'ta FILETIME'dır. Diğer platformlarda formatı **doğrula**: başka bir saat birimi mi kullanılıyor? Linux'ta `/proc/<pid>/stat` (starttime, jiffy cinsinden), macOS'ta `proc_pidinfo` (`PROC_PIDTBSDINFO`) ile karşılaştırılır.
- [ ] **Dosya konumları:**
  - Linux: `$XDG_DATA_HOME/tokenradar` (DB), `$XDG_CONFIG_HOME/tokenradar` (ayarlar), `$XDG_STATE_HOME/tokenradar` (loglar).
  - macOS: `~/Library/Application Support/TokenRadar`, `~/Library/Logs/TokenRadar`.
  - `directories` crate'i kullanılır; Windows yolları değişmez.
- [ ] **CI:**
  - GitHub Actions matrisi.
  - Linux bağımlılıkları: `libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`.
  - Artefaktlar yüklenir. Boyut kontrolü (`check-bundle-size`) her platformda çalışır.
- [ ] **Font:** Segoe UI Variable yalnızca Windows'ta var. Font yığını: `"Segoe UI Variable", -apple-system, "SF Pro Text", system-ui, "Inter", "Cantarell", sans-serif`. Gömülü mono font her yerde aynıdır. `design-language.md` §3 güncellenir.

### macOS

- [ ] **Menü çubuğu ikonu:**
  - Template image (monokrom) olmalı; sistem açık ve koyu temaya göre renklendirir. Bu yüzden durum renklerle değil **şekille** ifade edilir:
    - İkili halka doluluk yayları korunur.
    - Uyarı → halkanın dışına küçük bir nokta.
    - Kritik → içi dolu merkez.
  - İsteğe bağlı ayar: "Renkli ikon" (template olmayan). `design-language.md` §4'teki tray ikonu bölümüne macOS varyantı eklenir.
  - Retina için 18 pt @1x/@2x (`tiny-skia` ile 18 px ve 36 px).
  - İsteğe bağlı: İkonun yanında metin (`%72`). `NSStatusItem` başlığı ile yapılır; ayarlanabilir, varsayılan kapalı.
- [ ] **Kimlik bilgisi (S2):** Keychain'de `Claude Code-credentials` kaydı (`security-framework` crate'i).
  - İlk erişimde macOS izin penceresi çıkar ("TokenRadar wants to access…"). Bu davranış onboarding'de **önceden açıklanır**. "Her zaman izin ver" seçilmezse her polling'de pencere çıkabilir; bu durumda S2 için kullanıcıya uyarı gösterilir.
  - Okunan JSON'dan yalnızca `accessToken` ve plan alanları alınır; `refreshToken` atlanır (ADR-006).
  - **Doğrula:** Kayıt adı ve hesap alanı güncel Claude Code sürümünde aynı mı? `CLAUDE_CONFIG_DIR` ile çoklu hesapta Keychain kaydı nasıl ayrılıyor? Sonucu `data-sources.md` S5'e işle.
- [ ] **Uygulama davranışı:**
  - `LSUIElement = true`: Dock ikonu yok. Ana pencere açıkken Dock ikonu gösterilsin mi? Varsayılan: hayır. `ActivationPolicy::Accessory`; ana pencere açılınca `Regular` politikasına geçiş seçeneği sunulur.
  - Panel: menü çubuğu ikonunun altında konumlanır; `NSVisualEffectView` vibrancy (Tauri `window-vibrancy`).
- [ ] **Otomatik başlatma:** `SMAppService.mainApp` (macOS 13+). Tauri autostart eklentisi LaunchAgent kullanıyorsa bu davranış tercih edilebilir; karar ver.
- [ ] **Tam ekran tespiti** (widget için): `NSWorkspace` aktif uygulama + pencere tam ekran durumu. Widget macOS'ta "deneysel" olarak işaretlenebilir.
- [ ] **İmzalama ve notarization:**
  - Apple Developer ID gerekir (**yıllık $99**). Gatekeeper, imzasız uygulamaları "hasarlı" olarak gösterir; kullanıcı deneyimi kötüdür.
  - **Kullanıcıya sor:** Developer hesabı açılacak mı? Açılmayacaksa README'de `xattr -dr com.apple.quarantine` talimatı yer alır (önerilmez ama belgelenir).
  - Notarization için `notarytool` (CI secret'ları: App Store Connect API anahtarı).
  - Updater imzası (minisign) bundan bağımsızdır ve aynen kalır.
- [ ] **Paket:** `.dmg` (universal veya iki mimari ayrı). Homebrew cask, sürüm sonrası opsiyonel; kullanıcı onayıyla.

### Linux

- [ ] **Tray:** `libayatana-appindicator` (StatusNotifierItem).
  - **GNOME** varsayılan olarak tray göstermez; "AppIndicator and KStatusNotifierItem Support" eklentisi gerekir. Onboarding bu durumu tespit eder (`XDG_CURRENT_DESKTOP`) ve kullanıcıyı bilgilendirir.
  - Tray yoksa uygulama yedek moda geçer: ana pencere normal pencere olarak açılır ve sistem bildirimleri çalışmaya devam eder.
  - AppIndicator'da **sol tık olayı yoktur**; tıklama menüyü açar. Panel menüdeki "Paneli aç" öğesinden açılır. Bu fark belgelenir.
  - İkon: AppIndicator ikonları dosya yolu veya tema adı ister. Dinamik ikon `$XDG_RUNTIME_DIR` altına PNG olarak yazılır ve **yalnızca değiştiğinde** güncellenir (yazma sayısı düşük tutulur).
- [ ] **WebKitGTK ölçümü:** Adım 02'deki ölçüm senaryoları tekrarlanır: tray-only, panel ve ana pencere. WebKitGTK, WebView2'den farklı bir bellek profiline sahiptir. Bütçe aşılırsa `performance-budget.md` dosyasına platform satırı eklenir; gerekçesi ADR ile yazılır.
  - Bilinen sorunlar: Bazı GPU ve Wayland kombinasyonlarında DMABUF renderer kaynaklı boş pencere sorunu var. Yedek olarak `WEBKIT_DISABLE_DMABUF_RENDERER=1`; belgelenir.
- [ ] **Bildirimler:** `notify-rust` (D-Bus). Eylem butonları masaüstü ortamına göre değişir; yoksa butonsuz bildirim gönderilir.
- [ ] **Otomatik başlatma:** `~/.config/autostart/tokenradar.desktop`.
- [ ] **Paket:** AppImage (updater ile uyumlu) + `.deb`. `.rpm` opsiyonel.

### Statusline ve CLI

- [ ] `tokenradar statusline install`: macOS ve Linux'ta `~/.claude/settings.json` komutu mutlak yol olarak yazılır. Boşluk içeren yollar POSIX kabuk kurallarına göre tırnaklanır.
- [ ] Gecikme ölçümü (p95 ≤ 30 ms) her platformda tekrarlanır.
- [ ] WSL notu: Adım 19'da WSL için Windows exe'si interop üzerinden çağrılıyordu. Linux CLI binary'si artık var; WSL içinde native Linux CLI kullanımı seçenek olarak sunulur. Ancak bu CLI, Windows'taki daemon'a Unix socket üzerinden bağlanamaz. Spool dosyası (UNC yolu) veya interop köprüsü gerekir. Karar ver ve ADR yaz.

### v0.5.0 sürümü

- [ ] Her platform için performans raporu (`docs/perf/v0.5-<platform>.md`).
- [ ] README: platform tablosu, kurulum talimatları, bilinen sınırlamalar (GNOME tray, macOS Keychain izni, Wayland).
- [ ] CHANGELOG ve sürüm. Tag ve release **kullanıcı onayıyla** yapılır (Adım 12 prosedürü).

## Teknik notlar ve tuzaklar

- **Tauri tray olayları platforma göre farklıdır:** macOS'ta sol ve sağ tık ayrılabilir. Linux'ta tıklama olayı gelmez. Panel açma mantığı bu yüzden "menüden aç" yolunu her zaman destekler.
- **Panel konumlandırma:** Windows'ta tray genellikle sağ alttadır; macOS'ta menü çubuğu üstte, Linux'ta panelin konumu masaüstü ortamına bağlıdır. Panel, ikon dikdörtgenine göre konumlanır. Dikdörtgen bilinmiyorsa imleç konumu kullanılır.
- **Dosya izleme:** macOS'ta FSEvents (`notify` crate'i) olayları gruplar ve gecikmeli iletir; bu, statusline yoksa canlılığı etkiler. Linux'ta inotify watch limiti (`fs.inotify.max_user_watches`) binlerce proje dizininde aşılabilir. Bu durumda yalnızca kök dizin ve en son kullanılan N proje izlenir, geri kalanı periyodik stat ile taranır. Limit aşımı loglanır ve UI'da bildirilir.
- **Apple Silicon ve Intel:** Universal binary boyutu yaklaşık iki katına çıkar. Installer bütçesi (≤ 12 MB) mimari başına uygulanır.

## Kabul kriterleri

- [ ] Üç platformda CI yeşil: derleme, test ve paketleme. Windows davranışında regresyon yok.
- [ ] macOS: menü çubuğu ikonu açık ve koyu temada okunur; Keychain akışı belgelenmiş ve çalışır; notarize edilmiş dmg (Developer hesabı varsa) veya belgelenmiş alternatif.
- [ ] Linux (Ubuntu 22.04 GNOME + eklenti, KDE Plasma): tray, panel, bildirimler ve otomatik başlatma çalışır. Tray yoksa yedek mod devreye girer.
- [ ] Platform performans raporları yazılmıştır. Bütçe sapmaları ADR ile gerekçelendirilmiştir.
- [ ] Statusline köprüsü üç platformda p95 ≤ 30 ms.

## Doğrulama

```bash
cargo test --workspace
pnpm -C app test && pnpm -C app check
pnpm tauri build
./scripts/measure-memory.sh --scenario tray-only    # macOS/Linux eşdeğeri; bu adımda yazılır
tokenradar status --json
```

## Devir notu

_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
