# Adım 12g — Claude Code mod'unu kurma ve kaldırma

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1.x | 12f | 1 oturum | ⬜ |

## Amaç

Mod şimdiye kadar yalnızca repodan, `claude --plugin-dir integrations/claude-code` ile yükleniyordu. Statusline köprüsü gibi, mod da TokenRadar'dan kurulup kaldırılabilmeli: ilk kullanımda, ayarlarda ve CLI'dan.

## Önce oku

- ADR-009 (settings.json'a dokunma kuralları), ADR-029, ADR-043, ADR-046
- `crates/tokenradar-core/src/statusline/install.rs` ve `settings.rs`
- `app/src-tauri/src/sources.rs`, `app/src/pages/settings/StatuslineDialog.svelte`, `SourcesTab.svelte`, `app/src/pages/onboarding/LimitsStep.svelte`
- `app/src-tauri/windows/hooks.nsh`
- `plugin-authoring` skill'inin reference.md'si: `CLAUDE_CODE_PLUGIN_DIRS`, `pluginConfigs`

## Kullanıcıyla kararlaştırılanlar (2026-11-07)

- **Mekanizma:** Claude Code'un `settings.json` dosyasının `env` bloğuna `CLAUDE_CODE_PLUGIN_DIRS` eklenir; değeri kurulum klasöründeki mod klasörüdür. Değişken zaten başka yollar içeriyorsa onlar korunur. Statusline'daki gibi: diff → onay → yedek → atomik yazma. Yerel marketplace ve `claude plugin install` kullanılmaz.
- **İlk kullanım:** Statusline adımının altında ayrı bir seçenek olarak sunulur. Varsayılan olarak seçili değildir; kullanıcı kendisi kurar.
- **Ayarlar:** "Veri kaynakları" sekmesinde statusline'ın yanında kur/kaldır.

## Kapsam

- Mod dosyaları kurulum paketine girer (`<kurulum>\claude-code`): `plugin.json`, `hooks/`, `types/index.d.ts`. Testler girmez. Uygulama güncellenince mod da güncellenir.
- Core: `env.CLAUDE_CODE_PLUGIN_DIRS` için plan/uygula/durum; yalnızca bu anahtar değişir, dosyanın geri kalanı byte byte korunur.
- CLI: `tokenradar mod install|uninstall [--yes]`.
- Uygulama: IPC komutları, ilk kullanım kartı, ayarlar bölümü, diff diyaloğu.
- Mod: CLI ayarı varsayılandaysa önce kendi klasörünün yanındaki `..\bin\tokenradar.exe`'yi kullanır; yoksa PATH'teki `tokenradar`.
- Kaldırıcı: kaldırırken (güncellemede değil) mod girdisini de geri alır.

## Kapsam dışı

- Marketplace yayını, `marketplace.json`.
- Kullanıcının `pluginConfigs` ayarlarına dokunmak.

## Doğrulama

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
- `pnpm -C app check`, `pnpm -C app lint`, `pnpm -C app test`
- `claude plugin validate integrations/claude-code`, `tsc -p integrations/claude-code`, `claude plugin test integrations/claude-code`
- `pnpm tauri build`; kurulum paketinde `claude-code\` klasörü bulunmalı.

## Devir notu
_(Oturum sonunda doldurulur)_
