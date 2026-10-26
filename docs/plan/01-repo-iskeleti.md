# Adım 01 — Repo iskeleti ve araç zinciri

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1 | — | 1 oturum | ⬜ |

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

- [ ] Ortamı doğrula: `rustup` stable + MSVC build tools, Node LTS, `pnpm`, WebView2 runtime. Eksikleri kullanıcıya bildir.
- [ ] **Uygulama kimliğini kullanıcıyla netleştir** (Tauri `identifier`, AppUserModelID ve veri dizinlerini belirler; sonradan değiştirmek zahmetli). Öneri: `io.github.<github-kullanıcı-adı>.tokenradar`.
- [ ] Kök `Cargo.toml` workspace'ini oluştur:
  - `resolver = "3"`, edition 2024, `[workspace.package]` (version 0.0.1, license MIT, repository).
  - `[workspace.lints.rust]` / `[workspace.lints.clippy]`: `unsafe_code = "deny"` (Windows FFI gereken crate'lerde yerel `allow` ile), `clippy::pedantic` "warn" (gürültülü olanları seçerek kapat), `unwrap_used = "warn"`.
  - Release profili: `lto = "fat"`, `codegen-units = 1`, `strip = true`, `panic = "abort"`, `opt-level = 3`. Boyut ve hız dengesi Adım 02'de ölçülecek; `opt-level = "s"` alternatifi not edilir.
- [ ] `crates/tokenradar-core`: boş modül ağacı (`domain`, `store`, `providers`, `indexer`, `limits`, `ipc`, `pricing`) ve `lib.rs`. Her modülde tek satırlık doküman yorumu bulunsun.
- [ ] `crates/tokenradar-i18n`: boş API iskeleti (`Localizer` tipi, `Locale` enum'u: `En`, `Tr`).
- [ ] `crates/tokenradar-cli`: `tokenradar --version` çıktısı veren minimal bir `main`. Argüman ayrıştırıcı için `lexopt` veya `pico-args` seç; `clap` yalnızca boyut ölçümü kabul edilebilir çıkarsa kullanılır.
- [ ] Tauri 2 uygulaması (`app/`):
  - `pnpm create tauri-app` (Svelte + TS) veya elle kurulum. **SvelteKit kullanma.**
  - `tauri.conf.json`: `app.windows = []` (başlangıçta pencere açılmaz), `productName: "TokenRadar"`, `mainBinaryName: "tokenradar-app"`, sıkı CSP (`default-src 'self'; style-src 'self' 'unsafe-inline'`). Inline stil ihtiyacı varsa gerekçesiyle not et.
  - `vite.config.ts`: `build.rollupOptions.input` ile `panel.html` ve `settings.html` girişleri.
  - Release'te `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`.
  - Tray: `TrayIconBuilder` + statik ikon, sol tık → `panel` penceresini oluştur/göster/gizle (frameless, 360×480, `skipTaskbar`), sağ tık → menü (Paneli aç / Çıkış).
  - `tauri-plugin-single-instance`: ikinci başlatmada paneli aç.
  - Capabilities: yalnızca `panel` ve `settings` pencereleri için minimal izinler.
- [ ] Kalite araçları:
  - `rustfmt.toml` (`max_width = 100`, `imports_granularity = "Crate"`)
  - `biome.json`
  - `svelte-check` ve Vitest kurulumu, örnek birer test
  - `deny.toml` (izin verilen lisanslar: MIT, Apache-2.0, BSD-2/3, ISC, Unicode-3.0, Zlib, MPL-2.0 (yalnızca dosya bazlı), OFL-1.1 (font))
- [ ] Kök `package.json` scriptleri: `check:all` sırasıyla `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `pnpm -C app lint`, `pnpm -C app check` ve `pnpm -C app test` çalıştırır.
- [ ] `.github/workflows/ci.yml`: `windows-latest` + cache (rust ve pnpm). `check:all` + `pnpm tauri build --no-bundle` adımlarını içersin.
- [ ] Repo dosyaları: `.editorconfig`, `.gitattributes` (`* text=auto eol=lf`, `*.ps1 eol=crlf`), `.gitignore` güncellemesi (target, node_modules, dist, `*.db`, `.claude/settings.local.json`).
- [ ] `README.md` dosyasına "Development" bölümü ekle (İngilizce): önkoşullar, `pnpm i`, `pnpm dev`.

## Teknik notlar ve tuzaklar

- **Pencere olmadan başlatma:** Tauri 2'de `windows: []` ile uygulama pencere açmaz. Ancak son pencere kapandığında uygulamanın **çıkmaması** için `RunEvent::ExitRequested` olayında `api.prevent_exit()` çağrılmalı. Çıkış yalnızca menüden yapılır.
- **Tray olayları:** Windows'ta `TrayIconEvent::Click` hem down hem up için gelir. Yalnızca `button_state == Up` ve `MouseButton::Left` durumunu işle.
- **Panel odak kaybı:** Bu adımda yalnızca "tekrar tıklayınca gizle" yeterli. Blur ile kapanma Adım 10'da yapılacak.
- **CLI ile GUI binary adları:** ADR-017'ye uy. `tokenradar` adını CLI'a ver, GUI `tokenradar-app` olsun.
- `pnpm tauri dev` ilk derlemede uzun sürer. Bu normal.

## Kabul kriterleri

- [ ] `pnpm dev`: uygulama pencere açmadan başlar, tray ikonu görünür. Sol tık paneli açar/kapatır, menüden "Çıkış" uygulamayı kapatır.
- [ ] İkinci kez başlatma yeni süreç açmaz; mevcut paneli açar.
- [ ] `pnpm check:all` sıfır uyarıyla geçer.
- [ ] `pnpm tauri build --no-bundle` release binary üretir. `tokenradar.exe --version` çalışır.
- [ ] İlk commit'ler conventional commit formatında atılmış.

## Doğrulama

```powershell
pnpm i
pnpm check:all
pnpm tauri build --no-bundle
.\target\release\tokenradar.exe --version
Get-Item .\target\release\tokenradar-app.exe, .\target\release\tokenradar.exe | Select Name, Length
```

## Devir notu

_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
