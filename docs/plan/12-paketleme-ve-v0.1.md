# Adım 12 — Paketleme, güncelleme ve v0.1 sürümü

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1 | 11 | 1-2 oturum | 🟨 kod tamam; sürüm kapıları açık (devir notu) |

## Amaç

TokenRadar'ı ilk kez dış dünyaya çıkarmak. Bu adımda şunlar yapılır:
- Kullanıcı bazlı NSIS installer (CLI'ı PATH'e ekler) ve portable zip.
- İmzalı (minisign) otomatik güncelleme.
- GitHub reposu ve CI/CD.
- winget ve Scoop manifestleri.
- İngilizce README ve topluluk dosyaları.
- Tüm performans bütçesinin son kez doğrulanması ve **v0.1.0** etiketi.

## Önce oku

- `docs/decisions.md`: ADR-001, ADR-012, ADR-013, ADR-015, ADR-017
- `docs/performance-budget.md` (tamamı; bu adım tam doğrulama yapar)
- `docs/vision.md` (README için)
- Önceki tüm adımların devir notları (açık konular listesi)

## Kapsam

- **Tauri bundler, NSIS:** `installMode: "currentUser"`, Başlat menüsü kısayolu (AppUserModelID), CLI'ın `bin\` altına kopyalanması ve kullanıcı PATH'ine eklenmesi, kaldırmada temizlik.
- **Portable zip:** `tokenradar-app.exe`, `bin\tokenradar.exe` ve `portable` bayrak dosyası.
- **Updater:** `tauri-plugin-updater`, minisign anahtarı, GitHub Releases üzerinde `latest.json`, günlük kontrol, Hakkında sekmesinde "Güncellemeler" bölümü.
- **GitHub:** Repo oluşturma (kullanıcı onayıyla), push, CI yeşil, `release.yml` (tag `v*` → build → imzalı updater artefaktları → **taslak** release).
- **Dağıtım manifestleri:** winget (`wingetcreate`) ve Scoop (ayrı bir bucket reposu veya ana repoda `bucket/`).
- **Dokümanlar:**
  - `README.md` (EN): ekran görüntüsü/GIF, özellikler, kurulum, statusline kurulumu, gizlilik bildirimi, SSS
  - `CHANGELOG.md`, `CONTRIBUTING.md`, `SECURITY.md`, issue/PR şablonları
  - `docs/en/architecture-summary.md` (ADR-015)
- **Tam performans doğrulaması** ve `docs/perf/<tarih>-v0.1.0.md`.

## Kapsam dışı

- Authenticode kod imzalama (ADR-012; SignPath başvurusu proje olgunlaşınca yapılacak).
- macOS ve Linux paketleri (Adım 21).

## Görevler

### Paketleme

- [ ] `tauri.conf.json > bundle`:
  - `targets: ["nsis"]`, `windows.nsis.installMode: "currentUser"`
  - Dil seçenekleri: English ve Turkish
  - Ürün ikonu (çok boyutlu `.ico`; tasarım dilindeki halka motifi)
  - `publisher`, `copyright`, `shortDescription`
- [ ] CLI'ın pakete eklenmesi: `bundle.resources` veya `externalBin` ile `bin\tokenradar.exe`. **`externalBin` binary adına target triple ekler**; istenen yolu ve adı elde etmek için en temiz yöntemi seç ve belgele.
- [ ] NSIS hook'ları (`installerHooks`):
  - `NSIS_HOOK_POSTINSTALL`: `$INSTDIR\bin` dizinini kullanıcı PATH'ine (`HKCU\Environment\Path`) ekler. **Tekrar eklemeyi önler** ve 2047 karakter sınırını kontrol eder. Ardından `WM_SETTINGCHANGE` yayınlar.
  - `NSIS_HOOK_PREUNINSTALL`: PATH'ten kaldırır, autostart kaydını siler. Veri dizinini silmek için kullanıcıya sorar (varsayılan: koru). **Statusline kuruluysa uyarır**: önce `tokenradar statusline uninstall` çalıştırılır; komut başarısız olursa kullanıcıya talimat verilir.
- [ ] Portable zip script'i (`scripts/package-portable.ps1`).
- [ ] Yükseltme senaryosu testi: v0.0.x kurulu → yeni installer → veriler ve config korunur, migration'lar çalışır.

### Güncelleme

- [ ] Minisign anahtar çifti: Kullanıcı **kendi makinesinde** `pnpm tauri signer generate` komutunu çalıştırır. Özel anahtar ve parola **GitHub Secrets**'a eklenir, açık anahtar `tauri.conf.json` dosyasına yazılır. Özel anahtar asla repoya girmez; bunu kullanıcıya açıkça hatırlat.
- [ ] Endpoint: `https://github.com/<owner>/tokenradar/releases/latest/download/latest.json`.
- [ ] Davranış:
  - Günde en fazla bir kontrol (ayardan kapatılabilir).
  - Güncelleme bulunursa tray menüsünde ve Hakkında sekmesinde gösterilir. **Sessiz yükleme yapılmaz**; kullanıcı "Yükle ve yeniden başlat" seçeneğine basar.
  - Sürüm notları gösterilir.
- [ ] Test: Yerel bir sahte endpoint ile (`latest.json` + imzalı artefakt) güncelleme akışı uçtan uca denenir.

### GitHub ve CI/CD

- [ ] **Ad çakışması kontrolü:** Planlama araştırmasında "TokenRadar" / "Token Radar" adını kullanan başka ürünler görüldü (ai.cc, PeerPush listeleri). Kullanıcı adı korumayı seçti. Public sürümden önce GitHub, winget, Scoop ve crates.io'da adın müsait olup olmadığını kontrol et; sonuçları kullanıcıya sun. Paket kimlikleri (winget `Publisher.TokenRadar`, Scoop manifest adı) bu sonuca göre belirlenir.
- [ ] **Kullanıcıya sor ve onay al:** repo adı, sahibi, görünürlük (public), açıklama ve konu etiketleri (`claude-code`, `usage-tracker`, `tauri`, `rust`, `windows`, `tray`). Repo `gh repo create` ile oluşturulur. **Push, kullanıcı onayından sonra yapılır.**
- [ ] `ci.yml` (Adım 01) yeşil olmalı. Gerekli düzeltmeleri yap.
- [ ] `release.yml`:
  - `v*` tag'i → Windows build (NSIS + portable).
  - `TAURI_SIGNING_PRIVATE_KEY` ile updater imzası.
  - SHA256 dosyaları ve `latest.json` üretimi.
  - **Taslak** release oluşturulur; yayınlama işlemini kullanıcı manuel yapar.
- [ ] Dependabot (cargo, npm, github-actions) + `cargo-deny` ve `pnpm audit` adımları.

### Dağıtım manifestleri

- [ ] winget: `wingetcreate new` ile manifest hazırlanır. Gönderim (PR), release yayınlandıktan sonra kullanıcının onayıyla yapılır.
- [ ] Scoop: `bucket/tokenradar.json` (portable zip; `bin` → `tokenradar.exe` shim, kısayol → `tokenradar-app.exe`, `persist` → veri dizini). Scoop kurulumu portable mod gibi davranmalı.

### Dokümanlar

- [ ] `README.md` (EN):
  - Tek cümlelik tanım + hero görseli (panel ve tray).
  - Özellikler (v0.1 gerçekleri; gelecek özellikler "Roadmap" başlığı altında).
  - Kurulum: winget / Scoop / installer / portable.
  - **Statusline bridge** açıklaması.
  - **Privacy:** okunan ve okunmayan veriler, ağ hedefleri, token politikası.
  - FAQ: "Is this allowed?" (yalnızca yerel dosyalar ve kullanıcının kendi oturumunu kullanır; limit atlatma yapmaz). "Why does SmartScreen warn me?"
  - Contributing ve License.
- [ ] `CHANGELOG.md` (Keep a Changelog), `CONTRIBUTING.md` (geliştirme ortamı, adım planı yapısı, commit kuralları), `SECURITY.md` (güvenlik bildirimi kanalı).
- [ ] `docs/en/architecture-summary.md`: mimari, veri kaynakları ve gizlilik özeti.

### Son doğrulama ve sürüm

- [ ] **Performans:** `performance-budget.md` içindeki v0.1 satırlarının tamamı ölçülür ve rapor yazılır. Bütçe aşımı varsa sürüm **durdurulur**: ya düzeltilir ya da kullanıcı onayıyla bütçe revize edilir (gerekçeli ADR).
- [ ] **Temiz makine testi:** Windows Sandbox veya ayrı bir kullanıcı hesabında installer → onboarding → statusline → panel → bildirim → güncelleme kontrolü → kaldırma.
- [ ] Sürüm numarası `0.1.0` olarak güncellenir (workspace, `package.json`, `tauri.conf.json`), CHANGELOG yazılır. **Tag ve push kullanıcı onayıyla yapılır.**

## Teknik notlar ve tuzaklar

- **PATH düzenleme:** NSIS'in `EnVar` eklentisi veya registry ile yapılabilir. Ham registry yazımında `REG_EXPAND_SZ` tipinin korunması gerekir; aksi halde `%USERPROFILE%` gibi genişletmeler bozulur.
- **SmartScreen:** İmzasız installer ilk günlerde "Windows kişisel bilgisayarınızı korudu" uyarısı çıkarır. README'de bunun nedeni ve nasıl geçileceği açıklanır. winget ve Scoop kurulumlarında genelde bu uyarı çıkmaz.
- **Updater ve portable:** Portable modda updater devre dışı bırakılır; yalnızca bildirim gösterilir.
- **AppUserModelID:** Toast bildirimlerinin doğru görünmesi için Başlat menüsü kısayolunun AUMID taşıması gerekir. Tauri bunu yapıyor mu? Doğrula.

## Kabul kriterleri

- [ ] Temiz makinede kurulum → kullanım → güncelleme → kaldırma akışı sorunsuz. PATH ve kayıt temizliği doğrulanmış.
- [ ] v0.1 performans bütçesinin tamamı sağlanmış ve raporlanmış.
- [ ] CI ve release iş akışları yeşil. Taslak release, imzalı updater artefaktlarını içeriyor.
- [ ] README gizlilik ve token politikasını açıkça anlatıyor.
- [ ] Kullanıcı onayıyla `v0.1.0` tag'i atılmış ve release yayınlanmış.

## Doğrulama

```powershell
pnpm tauri build
.\scripts\package-portable.ps1
.\scripts\measure-memory.ps1 -ProcessName tokenradar-app -Scenario full
gh workflow run release.yml   # veya tag push sonrası otomatik
```

## Devir notu

**Tarih:** 2026-10-28 · **Dal:** `step/12-packaging-v0.1` · **Karar:** ADR-036, ADR-037

#### Kullanıcı kararları
- Adım tek oturumda yapıldı. Adım 11'in üç açık işi kurulu build ile, **tag'den önce** yapılacak (sürüm kapısı).
- Ad TokenRadar olarak kaldı.
  - Repo `umitkara/tokenradar` (public).
  - winget kimliği `umitkara.TokenRadar`.
  - Scoop manifesti ana repodaki `bucket/` altında.
- **Ad kontrolü (2026-10-28):**
  - GitHub'da `umitkara/tokenradar`, winget'te ve crates.io'da `tokenradar` adı boşta.
  - GitHub'da aynı adı taşıyan, aynı alanda iki küçük proje var: bir Claude Code dashboard'u ve bir macOS menü çubuğu uygulaması. İkisi de 0 yıldızlı.
  - Ad korundu.

#### Yapılanlar
- **Installer (NSIS, currentUser):**
  - `app/src-tauri/windows/installer.nsi`: Tauri şablonunun kopyası. İşaretli iki değişiklik var:
    - Kurulum dizini `%LOCALAPPDATA%\Programs\TokenRadar`.
    - Yeniden kurulumda eski kaldırıcı her zaman `/UPDATE` ile çalışır.
  - `scripts/check-nsis-template.mjs`, kopyayı kurulu tauri-cli şablonuyla karşılaştırır. CI'da çalışır.
  - `windows/hooks.nsh`:
    - Kurulumda `tokenradar path add` çalışır.
    - Kaldırmada sırasıyla: uygulama kapatılır, `statusline uninstall --all --yes` çalışır, `path remove` çalışır, autostart değerleri silinir.
    - Veri yalnızca "Uygulama verilerini sil" kutusu işaretliyse silinir.
    - Mesajlar TR ve EN.
  - CLI pakete `bundle.resources` ile `bin\tokenradar.exe` olarak girer. `beforeBuildCommand`, CLI'ı `release-cli` profiliyle derler ve lisans listesini üretir.
  - Paket bilgileri ayarlandı: diller English ve Turkish, `publisher`, `copyright`, açıklamalar, ikon.
- **CLI:**
  - `tokenradar path add|remove` (`user_path.rs`):
    - `REG_EXPAND_SZ` tipi korunur ve yineleme önlenir.
    - Sonuç 2047 karakteri aşacaksa çıkış kodu 3 olur.
    - Değişiklikten sonra `WM_SETTINGCHANGE` yayınlanır.
  - `statusline uninstall --all`, `config.toml`'de kurulu görünen her hesabı geri alır.
  - `paths.rs`: `bin\` altındaki CLI, portable işaretini bir üst dizinde de bulur.
- **Updater (`updater.rs`):**
  - `tauri-plugin-updater` (rustls) kullanılır.
  - Günlük denetim yalnızca release build'de çalışır: açılıştan 120 s sonra başlar, zaman `kv.update_checked_at` içinde tutulur, ayardan kapatılabilir.
  - Durum `UpdateView` ve `UpdateChanged` ile taşınır. Komutlar: `get_update`, `check_for_update`, `install_update`.
  - Arayüz:
    - Tray menüsünün başında "Güncelleme var: X…" girdisi.
    - Hakkında'da Güncellemeler kartı: durum satırları, sürüm notları, "Şimdi denetle", "Kur ve yeniden başlat".
    - Portable kopyada bunun yerine sürümler sayfası önerilir.
  - Kurulum `/UPDATE /P` ile çalışır; uygulama çıkmadan önce çekirdek kapatılır.
- **Lisanslar:**
  - `scripts/third-party-notices.mjs` üç kaynağı birleştirir: Rust bağımlılık grafiği, Vite'ın bundle'a koyduğu npm paketleri ve font.
  - Çıktı: 347 bileşen, 229 ayrı lisans metni.
  - Hakkında'da "Üçüncü parti lisanslar" düğmesi var.
- **Portable zip:** `scripts/package-portable.ps1`, `TokenRadar_<sürüm>_x64-portable.zip` dosyasını üretir. İçerik: `tokenradar-app.exe`, `bin\tokenradar.exe`, `portable`, `LICENSE.txt`, `THIRD-PARTY-NOTICES.txt`.
- **CI/CD:**
  - `ci.yml`: tam bundle ve portable zip derlenir, NSIS şablonu kontrol edilir. Ayrıca bir `pnpm audit` işi var.
  - `release.yml`, `v*` tag'iyle tetiklenir. Adımlar sırasıyla:
    1. Tag ile sürüm karşılaştırılır.
    2. `check:all` çalışır.
    3. İmzalı build alınır (`tauri.release.conf.json` overlay'i).
    4. Bundle bütçesi kontrol edilir.
    5. Portable zip üretilir.
    6. `release-assets.mjs` çalışır.
    7. Artifact yüklenir.
    8. **Taslak** release oluşturulur.
  - `scripts/release-assets.mjs`:
    - Installer imzasını `tauri.conf.json`'daki açık anahtarla doğrular.
    - `latest.json`, `SHA256SUMS.txt`, `notes.md` (CHANGELOG bölümü) ve Scoop/winget manifestlerini üretir.
  - Dependabot (cargo, npm, github-actions) eklendi.
  - `deny.toml`: RUSTSEC-2024-0436 (`paste`) gerekçesiyle yok sayıldı. `cargo deny` bu adımdan önce de bu yüzden kırmızıydı.
- **Dokümanlar:**
  - README (EN): özellikler, kurulum, statusline, gizlilik, SSS.
  - CHANGELOG.
  - CONTRIBUTING: geliştirme ortamı ve sürüm çıkarma adımları.
  - SECURITY: GitHub üzerinden özel bildirim.
  - Issue ve PR şablonları.
  - `docs/en/architecture-summary.md`.
- **Sürüm:** 0.1.0. Workspace `Cargo.toml` ve `app/package.json` güncellendi; `tauri.conf.json` sürümü Cargo'dan alır.

#### Doğrulama
- **Testler ve denetimler** (`pnpm check:all` eşdeğeri):
  - fmt ve clippy `-D warnings` temiz.
  - `cargo test --workspace`: core 252, app 57 + 2 ignored, CLI 25 + 2, i18n 16 + 6.
  - Biome ve svelte-check temiz, Vitest 81 test geçti.
  - `cargo deny check` ve `pnpm audit --audit-level high` temiz.
- **NSIS şablonu:** `check-nsis-template.mjs`, tauri-cli 2.12.4 şablonuyla eşleşiyor.
- **İmza doğrulaması:**
  - Geçici bir test anahtarıyla imzalanan installer doğrulandı (key id `84566286825034C5`).
  - Tek baytı değiştirilmiş installer reddedildi.
- **Kurulum testi (bu makinede, kullanıcı onayıyla):**
  - **İlk kurulum:** Yerel endpoint overlay'li 0.0.9 test build'i `/S` ile kuruldu.
    - Dosyalar `Programs\TokenRadar` altında.
    - PATH'e `bin` eklendi; tip `ExpandString` olarak kaldı.
    - Başlat kısayolu ve Kaldır kaydı oluştu.
  - **Yeniden kurulum:** Aynı sürüm tekrar kuruldu. PATH'te tek girdi kaldı, veri korundu. `path add` tekrar çalıştırıldığında değişiklik olmadı.
  - **Güncelleme:**
    - Uygulama içinden (CDP ile) "Check now" çalıştırıldı; 0.1.0 bulundu ve sürüm notları göründü.
    - "Install and restart" ile imzalı installer indirildi ve passive kuruldu.
    - Uygulama kendiliğinden yeniden başladı. Kaldır kaydı 0.1.0 oldu.
  - **Kaldırma** (`uninstall.exe /S`):
    - PATH test öncesiyle bayt bayt aynı.
    - Run ve StartupApproved değerleri, kısayol, Kaldır kaydı ve kurulum dizini silindi.
    - Veri dizini ve kullanıcının kendi statusline'ı korundu.
  - Bu sırada bulunan kusur düzeltildi: yeniden başlatmadan sonra "Son denetim" satırı görünmüyordu.
- **Performans:** `docs/perf/2026-10-28-v0.1.0.md`. Ölçülen bütün v0.1 satırları bütçe içinde. İstisna panelin soğuk açılışıydı: medyan 440 ms, Adım 11c build'i aynı gün 468 ms. Kullanıcı kararıyla bütçe 400 ms'den 500 ms'ye çıktı (ADR-037).

#### Plandan sapmalar
- **Kurulum dizini** `%LOCALAPPDATA%\Programs\TokenRadar` oldu. Tauri'nin varsayılanı veri dizinimizle çakışıyordu; bunun için şablon kopyalandı (ADR-036).
- **PATH** NSIS yerine CLI'dan (`tokenradar path add|remove`) düzenleniyor. Sebep: Tauri'nin NSIS'i 1024 karakterlik dizgelerle derlenmiş (ADR-036).
- **Veri silme sorusu** ayrı bir hook diyaloğu değil; Tauri kaldırıcısındaki "Uygulama verilerini sil" kutusu kullanılıyor. Varsayılan: veri korunur.
- **winget manifestleri** `wingetcreate` ile değil, `release-assets.mjs` ile üretiliyor (1.12.0 şeması; `wingetcreate` bu makinede kurulu değil). Gönderim release sonrasında yapılacak.
- **`bucket/tokenradar.json`** henüz commit'lenmedi. Hash'in CI'ın ürettiği zip'ten gelmesi gerekiyor (CONTRIBUTING → Releasing, 3. adım).
- **Ölçüm komutu:** Plandaki `measure-memory.ps1 -Scenario full` komutu mevcut değil. Ölçüm `measure-memory.ps1`, `measure-panel.ps1` ve `measure-statusline.ps1` ile yapıldı.
- **README'de ekran görüntüsü yok.** Gerçek veriyle çekilen görüntü kişisel kullanım bilgisi içerir; görüntü sahte sahneyle (`TOKENRADAR_FAKE_LIMITS=panel:ok`) çekilmeli.

#### Açık işler (tag'den önce, sürüm kapısı)
1. **İmzalama anahtarı:**
   - Kullanıcı kendi makinesinde `pnpm -C app tauri signer generate -w <repo dışı yol>` komutunu çalıştırır.
   - **Anahtar dosyasını ve parolayı repo dışında, güvenli bir yerde (örneğin bir şifre yöneticisinde) saklayın.** GitHub Secrets'taki değer geri okunamaz. Anahtar kaybolursa kurulu hiçbir kopya güncellenemez.
   - ✅ 2026-10-28: Kullanıcı anahtarı üretti. Açık anahtar (key id `C960EC8ABFFE2454`) `tauri.conf.json > plugins.updater.pubkey` alanında. Testlerde kullanılan geçici anahtar hiç commit'lenmedi ve silindi.
   - ✅ Özel anahtar `TAURI_SIGNING_PRIVATE_KEY`, parolası `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` secret'ında (kullanıcı ekledi). Tag'siz, elle tetiklenen `release.yml` çalıştırması installer'ı imzaladı; `release-assets.mjs` imzayı açık anahtarla doğruladı ("signature verified with key C960EC8ABFFE2454"). Hiçbir şey yayınlanmadı.
2. ✅ **Repo** (2026-10-28):
   - `umitkara/tokenradar` herkese açık olarak oluşturuldu. Konu etiketleri eklendi, "Private vulnerability reporting" açık.
   - İlk push'tan önce geçmişin tamamı yeniden yazıldı. Yazar ve committer e-postası artık GitHub'ın noreply adresi; kullanıcının kararıyla kişisel e-posta herkese açık commit'lere girmedi.
   - İlk CI çalıştırmaları temiz makinede üç sorun buldu. Üçü de düzeltildi; CI ve release yeşil:
     - Bundle kaynakları (CLI ve notices) yalnızca `tauri build` sırasında üretiliyordu, bu yüzden `cargo test` temiz klonda düşüyordu. `build.rs` artık debug build'lerde işaretli yer tutucu yazıyor, release build'leri yer tutucuyu reddediyor.
     - Runner yerleşik Yönetici hesabıyla çalışıyor. Pipe DACL testi SID'i SDDL metninde arıyordu, Windows ise SID'i `LA` kısaltmasıyla yazıyor. Test artık `canonical_sid` ile karşılaştırıyor.
     - Önbelleksiz release işinde Git Bash'in açılması 300 ms'lik zincir süresini aştı. Süre `Deps`'e taşındı; çıktıyı kontrol eden testler 10 s bekliyor.
   - Dependabot ilk taramada major sürüm PR'ları açtı (TypeScript 7, `@types/node` 26, `windows-core` 0.100, `sha2` 0.11). Bunlar birleştirilmedi; v0.1 sonrasında tek tek değerlendirilmeli.
3. **Adım 11'in üç açık işi** kurulu build ile yapılır:
   - Bir günlük bildirim gürültüsü değerlendirmesi. Kurulu kopyada AUMID'li kısayol var; toast tıklaması da bu sırada denenmeli.
   - Gerçek boşta bellek ölçümü (Claude Code kapalıyken).
   - Onboarding'den gerçek statusline kurulumu ve panelde canlı veri.
4. **Temiz makine testi:** Windows Home'da Sandbox yok. Ayrı bir kullanıcı hesabında şu akış denenir: installer → onboarding → statusline → panel → bildirim → güncelleme kontrolü → kaldırma.
5. **Yayın** (kullanıcı onayıyla):
   1. `v0.1.0` tag'i atılır; taslak release oluşur.
   2. `bucket/tokenradar.json` artifact'tan alınıp commit'lenir.
   3. Release yayınlanır.
   4. winget-pkgs PR'ı açılır (onayla).
6. ✅ **README görselleri** (2026-10-28):
   - Panel `panel:ok` sahnesiyle, debug build'de çekildi. `docs/img/panel-{light,dark}.png` README'de `<picture>` ile okuyucunun temasına göre gösteriliyor.
   - Debug exe'nin yanına geçici `portable` işareti kondu. Tema değişikliği (`set_theme`) böylece kullanıcının gerçek `config.toml` dosyasına değil `target/debug/data` altına yazıldı.
   - **Sapma:** Tray görseli eklenmedi. Elimizdeki tek kaynak `tray_contact_sheet` kontak sayfası; bu sayfa büyütülmüş 16–32 px simgelerden oluştuğu için README'de bulanık ve teknik görünüyor. Simge README'de metinle anlatılıyor.

#### Sonraki adımlar için notlar
- **Toast tıklaması:** Bildirim Merkezi'nden toast'a tıklamak uygulamayı açmıyor, çünkü COM activator yok. Seçenekler: protocol activation (`tokenradar://`) veya Tauri deep-link eklentisi.
- **Portable ve Scoop:**
  - AUMID'li kısayol olmadığı için toast görünmeyebilir.
  - Scoop'ta iki şey doğrulanmalı: `current_exe()` yolunun `current` junction'ını kullandığı ve statusline komutunun güncellemelerden sonra geçerli kaldığı. Bu makinede Scoop kurulu değil.
- **`paste` advisory'si:** Tauri ve specta bir fork'a geçince `deny.toml`'den kaldırılmalı.
- **Tauri yükseltmeleri:** `check-nsis-template.mjs` kırmızı olur. Kopya, yeni şablondan işaretli değişiklikler uygulanarak yeniden oluşturulur.
- **Build provenance:** Release iş akışına `actions/attest-build-provenance` eklenebilir. İmzasız installer için ek güven sağlar.
