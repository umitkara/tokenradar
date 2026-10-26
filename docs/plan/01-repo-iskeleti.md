# Adım 01 — Repo iskeleti ve araç zinciri

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1 | — | 1 oturum | ✅ |

## Amaç

Çalışan, boş ama doğru yapılandırılmış bir monorepo kurmak. Hedef durum:
- Cargo workspace ve dört crate hazır.
- Tauri 2 + Svelte 5 uygulaması **pencere açmadan** başlıyor.
- Tray'de statik bir ikon görünüyor; tıklayınca "hello" paneli açılıyor.
- Lint, format, test ve CI tek komutla çalışıyor.

## Önce oku

- `docs/vision.md`
- `docs/architecture.md` §2, §3, §4
- `docs/decisions.md`: ADR-002, 004, 015, 017

## Kapsam

- Cargo workspace:
  - `crates/tokenradar-core` (lib)
  - `crates/tokenradar-i18n` (lib)
  - `crates/tokenradar-cli` (bin: `tokenradar`)
  - `app/src-tauri` (bin: `tokenradar-app`)
- Svelte 5 + Vite + TypeScript frontend, **çok sayfalı** derleme. Bu adımda `panel.html` ve `settings.html` yer tutucuları yeterli.
- Tray ikonu (statik PNG) + sol tık ile panel penceresinin açılıp kapanması + sağ tık menüsü (Panel, Çıkış).
- `tauri-plugin-single-instance`.
- Kalite araçları:
  - Rust: rustfmt, clippy (workspace lints)
  - TS/Svelte: Biome, svelte-check, Vitest
  - Bağımlılıklar: cargo-deny
- Kök `package.json` scriptleri: `dev`, `build`, `lint`, `fmt`, `test`, `check:all`.
- GitHub Actions CI tanımı (`windows-latest`). Repo henüz GitHub'da değil; dosya hazırlanır, Adım 12'de devreye girer.
- Release profili optimizasyonları.

## Kapsam dışı

- Gerçek iş mantığı, veritabanı, i18n içeriği (yalnızca crate iskeleti).
- Tasarım token'ları ve bileşenler (Adım 09).
- Installer ve updater (Adım 12).

## Görevler

- [x] Ortamı doğrula: `rustup` stable + MSVC build tools, Node LTS, `pnpm`, WebView2 runtime. Eksikleri kullanıcıya bildir.
- [x] **Uygulama kimliğini kullanıcıyla netleştir** (Tauri `identifier`, AppUserModelID ve veri dizinlerini belirler; sonradan değiştirmek zahmetli). Öneri: `io.github.<github-kullanıcı-adı>.tokenradar`.
- [x] Kök `Cargo.toml` workspace'ini oluştur:
  - `resolver = "3"`, edition 2024, `[workspace.package]` (version 0.0.1, license MIT, repository).
  - `[workspace.lints.rust]` / `[workspace.lints.clippy]`: `unsafe_code = "deny"` (Windows FFI gereken crate'lerde yerel `allow` ile), `clippy::pedantic` "warn" (gürültülü olanları seçerek kapat), `unwrap_used = "warn"`.
  - Release profili: `lto = "fat"`, `codegen-units = 1`, `strip = true`, `panic = "abort"`, `opt-level = 3`. Boyut ve hız dengesi Adım 02'de ölçülecek; `opt-level = "s"` alternatifi not edilir.
- [x] `crates/tokenradar-core`: boş modül ağacı (`domain`, `store`, `providers`, `indexer`, `limits`, `ipc`, `pricing`) ve `lib.rs`. Her modülde tek satırlık doküman yorumu bulunsun.
- [x] `crates/tokenradar-i18n`: boş API iskeleti (`Localizer` tipi, `Locale` enum'u: `En`, `Tr`).
- [x] `crates/tokenradar-cli`: `tokenradar --version` çıktısı veren minimal bir `main`. Argüman ayrıştırıcı için `lexopt` veya `pico-args` seç; `clap` yalnızca boyut ölçümü kabul edilebilir çıkarsa kullanılır.
- [x] Tauri 2 uygulaması (`app/`):
  - `pnpm create tauri-app` (Svelte + TS) veya elle kurulum. **SvelteKit kullanma.**
  - `tauri.conf.json`: `app.windows = []` (başlangıçta pencere açılmaz), `productName: "TokenRadar"`, `mainBinaryName: "tokenradar-app"`, sıkı CSP (`default-src 'self'; style-src 'self' 'unsafe-inline'`). Inline stil ihtiyacı varsa gerekçesiyle not et.
  - `vite.config.ts`: `build.rollupOptions.input` ile `panel.html` ve `settings.html` girişleri.
  - Release'te `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`.
  - Tray: `TrayIconBuilder` + statik ikon, sol tık → `panel` penceresini oluştur/göster/gizle (frameless, 360×480, `skipTaskbar`), sağ tık → menü (Paneli aç / Çıkış).
  - `tauri-plugin-single-instance`: ikinci başlatmada paneli aç.
  - Capabilities: yalnızca `panel` ve `settings` pencereleri için minimal izinler.
- [x] Kalite araçları:
  - `rustfmt.toml` (`max_width = 100`, `imports_granularity = "Crate"`)
  - `biome.json`
  - `svelte-check` ve Vitest kurulumu, örnek birer test
  - `deny.toml` (izin verilen lisanslar: MIT, Apache-2.0, BSD-2/3, ISC, Unicode-3.0, Zlib, MPL-2.0 (yalnızca dosya bazlı), OFL-1.1 (font))
- [x] Kök `package.json` scriptleri: `check:all` sırasıyla `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `pnpm -C app lint`, `pnpm -C app check` ve `pnpm -C app test` çalıştırır.
- [x] `.github/workflows/ci.yml`: `windows-latest` + cache (rust ve pnpm). `check:all` + `pnpm tauri build --no-bundle` adımlarını içersin.
- [x] Repo dosyaları: `.editorconfig`, `.gitattributes` (`* text=auto eol=lf`, `*.ps1 eol=crlf`), `.gitignore` güncellemesi (target, node_modules, dist, `*.db`, `.claude/settings.local.json`).
- [x] `README.md` dosyasına "Development" bölümü ekle (İngilizce): önkoşullar, `pnpm i`, `pnpm dev`.

## Teknik notlar ve tuzaklar

- **Pencere olmadan başlatma:** Tauri 2'de `windows: []` ile uygulama pencere açmaz. Ancak son pencere kapandığında uygulamanın **çıkmaması** için `RunEvent::ExitRequested` olayında `api.prevent_exit()` çağrılmalı. Çıkış yalnızca menüden yapılır.
- **Tray olayları:** Windows'ta `TrayIconEvent::Click` hem down hem up için gelir. Yalnızca `button_state == Up` ve `MouseButton::Left` durumunu işle.
- **Panel odak kaybı:** Bu adımda yalnızca "tekrar tıklayınca gizle" yeterli. Blur ile kapanma Adım 10'da yapılacak.
- **CLI ile GUI binary adları:** ADR-017'ye uy. `tokenradar` adını CLI'a ver, GUI `tokenradar-app` olsun.
- `pnpm tauri dev` ilk derlemede uzun sürer. Bu normal.

## Kabul kriterleri

- [x] `pnpm dev`: uygulama pencere açmadan başlar, tray ikonu görünür. Sol tık paneli açar/kapatır, menüden "Çıkış" uygulamayı kapatır.
- [x] İkinci kez başlatma yeni süreç açmaz; mevcut paneli açar.
- [x] `pnpm check:all` sıfır uyarıyla geçer.
- [x] `pnpm tauri build --no-bundle` release binary üretir. `tokenradar.exe --version` çalışır.
- [x] İlk commit'ler conventional commit formatında atılmış.

## Doğrulama

```powershell
pnpm i
pnpm check:all
pnpm tauri build --no-bundle
.\target\release\tokenradar.exe --version
Get-Item .\target\release\tokenradar-app.exe, .\target\release\tokenradar.exe | Select Name, Length
```

## Devir notu

**Tarih:** 2026-10-27 · **Dal:** `step/01-repo-skeleton`

### Yapılanlar
- **Workspace:** `resolver = "3"`, edition 2024, `rust-version = "1.99"`, workspace lint'leri (`unsafe_code = "deny"`, `clippy::pedantic` + `unwrap_used` warn; `must_use_candidate`, `missing_errors_doc`, `missing_panics_doc` kapalı), plandaki release profili.
- **Uygulama kimliği:** `io.github.umitkara.tokenradar` (kullanıcıyla kararlaştırıldı). Repo URL'si: `https://github.com/umitkara/TokenRadar` (repo henüz yok).
- **Crate'ler:** `tokenradar-core` (7 boş modül), `tokenradar-i18n` (`Locale { En, Tr }`, `Localizer`), `tokenradar-cli` (`lexopt`; `--version`/`--help`, bilinmeyen argümanda çıkış kodu 2, entegrasyon testi), `tokenradar-app`. Bağımlılık yönü mimariye uygun: `cli → core`, `app → core + i18n`, `core → i18n`.
- **Tauri kabuğu:** `windows: []`, tray (`include_image!` ile derleme anında gömülen 32 px PNG), sol tık (yalnızca `Up`) panel aç/gizle, sağ tık menüsü, `single-instance` (ikinci başlatma paneli açar), `ExitRequested { code: None }` → `prevent_exit()`. Panel: 360×480, çerçevesiz, `skipTaskbar`, `alwaysOnTop`. Capability: `panel` + `settings`, **boş izin listesi** (varsayılan red; izinler özellik geldikçe eklenir).
- **Frontend:** Vite 8 çok sayfalı (`panel.html`, `settings.html`), Svelte 5, Biome, svelte-check (`--fail-on-warnings`), Vitest (`svelte/server` ile render testi).
- **İkonlar:** `app/icons-src/*.svg` kaynaklarından `pnpm tauri icon` ile üretildi. Android/iOS/MSIX çıktıları silindi.
- **Kalite:** `pnpm check:all` sıfır uyarı; `cargo deny check` temiz; CI (`.github/workflows/ci.yml`) hazır, Adım 12'de devreye girer.
- **Ölçümler (bilgi amaçlı; resmi ölçüm Adım 02'de):** `tokenradar-app.exe` 4.313.088 B (4,1 MiB), `tokenradar.exe` 135.168 B (132 KiB). Tam release derlemesi ~3 dk 40 sn. Boşta ve panel kapalıyken WebView2 alt süreci yok; panel açılınca 1 `msedgewebview2` alt süreci görünüyor.
- **Elle doğrulandı (kullanıcı):** Tray sol tık aç/kapa, sağ tık menüsü, "Quit". **Otomatik doğrulandı:** pencere açmadan başlama, tek örnek, panel kapatılınca sürecin yaşaması, `pnpm dev`.

### Plandan sapmalar
- **ADR-022:** Rust `1.99.1` olarak sabitlendi (`rust-toolchain.toml`); Vite 8 nedeniyle `build.rolldownOptions`; TypeScript `^6` (svelte-check TS 7'yi desteklemiyor); `rustfmt.toml` içinden `imports_granularity` kaldırıldı (yalnızca nightly'de çalışıyor).
- **`beforeBuildCommand` CLI'ı da derliyor** (`cargo build --release --locked -p tokenradar-cli`): `tauri build` yalnızca uygulama paketini derliyor. Adım 12'de installer'ın CLI'ı `bin\` içine koyabilmesi için de gerekli.
- **`clippy.toml`:** `doc-valid-idents` listesine `TokenRadar` eklendi (pedantic `doc_markdown` ürün adını kod sanıyordu).
- **`cargo deny` `check:all` içinde değil:** Advisory veritabanı ağ gerektiriyor. Ayrı `pnpm deny` scripti ve CI'da ayrı bir `deny` işi (Linux, çünkü action bir container) var. `unused-allowed-license = "allow"`: lisans listesi bir politika.
- **Biome ve Svelte:** Tam Svelte desteği deneysel olduğu için kullanılmıyor. Biome yalnızca `<script>` bloğunu biçimlendiriyor (girintisiz). `.svelte` için `useConst`, `useImportType`, `noUnusedVariables`, `noUnusedImports` kapalı, çünkü şablondaki kullanımları göremiyor. Şablon denetimi svelte-check'te.
- **Svelte bileşenlerinde `<script lang="ts">` zorunlu:** Script bloğu olmayan bir bileşen svelte-check'te `implicitly has an 'any' type` hatası veriyor.

### Açık konular
- **Yer tutucu UI metinleri** (CLAUDE.md "UI metni hard-code edilmez" kuralına geçici istisna): Tray menüsü "Open panel"/"Quit" ve tooltip → **Adım 08** (`TODO(step-08)`). Panel/ayarlar içerikleri → Adım 09/10/11. CLI yardım metni → Adım 07.
- Hata raporlama şimdilik `eprintln!` (`TODO(step-05)`). Release'te GUI alt sistemi nedeniyle bu çıktı görünmüyor.
- `bundle.active = false`. Installer ve updater Adım 12'de.
- CI dosyası henüz hiç çalışmadı (repo GitHub'da yok). İlk push'ta action sürümleri (checkout v7, setup-node v7, pnpm/action-setup v6, rust-cache v2, cargo-deny-action v2) ve `rustup toolchain install` adımı doğrulanmalı.

### Sonraki adıma notlar (Adım 02)
- Release profili planla aynı (`opt-level = 3`, fat LTO). Adım 02'de `opt-level = "s"` karşılaştırması yapılacak. Referans: app 4,1 MiB.
- Panel `WebviewWindowBuilder` ile istek üzerine oluşturuluyor. Tray tıklaması onu yalnızca **gizliyor**; `WM_CLOSE` (Alt+F4) ise yok ediyor. `warm_ttl` mantığı yok. WebView2 süreçlerinin sonlanma ölçümü bu temel üzerinde yapılabilir: paneli aç → `CloseMainWindow` → alt süreç sayısını izle.
- Ölçüm scriptleri `scripts/` altına yazılacak. Dizin henüz yok.
