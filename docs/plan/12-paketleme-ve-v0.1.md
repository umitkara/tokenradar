# Adım 12 — Paketleme, güncelleme ve v0.1 sürümü

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1 | 11 | 1-2 oturum | ⬜ |

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

_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
