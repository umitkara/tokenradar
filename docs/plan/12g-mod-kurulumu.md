# Adım 12g — Claude Code mod'unu kurma ve kaldırma

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1.x | 12f | 1 oturum | ✅ |

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

**Tarih:** 2026-11-07 · **Durum:** ✅ Tamamlandı

**Kullanıcı kararları:** Mekanizma `env.CLAUDE_CODE_PLUGIN_DIRS`; ilk kullanımda statusline kartının altında ayrı kart, varsayılan olarak kurulmaz (ADR-047).

**Yapılanlar:**
- **Core:** `statusline::claude_mod` eklendi: `plan_install`, `plan_uninstall`, `state`, `mod_dir_near`, `ModPlan::apply`. Plan uygulanırken dosya yeniden okunur, değişmişse yazılmaz, yedek alınır, atomik yazılır. 6 test: bayt düzeyinde geri dönüş, diğer yolların korunması, eksik dosya, yabancı değer, değişmiş dosya, mod klasörünün bulunması.
- **CLI:** `tokenradar mod install|uninstall [--yes] [--config-dir <DİZİN> | --all]`. Onay sorusu `confirm()` olarak statusline ile ortak.
- **Uygulama:**
  - `plan_claude_mod`/`apply_claude_mod` komutları. `Pending` artık statusline planını ya da mod planını tutuyor; bir planın token'ı diğerini uygulayamaz.
  - `DataSourcesView.claude_mod` (`no_directory`, `missing`, `not_installed`, `installed`).
  - Debug derlemede mod klasörü repodaki `integrations/claude-code`.
  - `StatuslineDialog` artık `target` alıyor. `ClaudeModCard` hem ilk kullanımda hem Ayarlar → Veri kaynakları'nda (`#mod`) kullanılıyor.
- **Paket:** `tauri.conf.json` resources mod'u `<kurulum>\claude-code`'a koyuyor (testler hariç). Kaldırıcı, güncellemede değil yalnızca kaldırırken `mod uninstall --all --yes` çalıştırıyor ve klasörü siliyor.
- **Mod:** `TokenRadar CLI` varsayılandaysa önce `..\bin\tokenradar.exe`'yi (`$.fs.exists`) kullanıyor, yoksa PATH'teki komutu. 3 yeni test (toplam 41).
- **Belgeler:** ADR-047; architecture §11 (mod ve CLI listesi) ve repo düzeni satırı; README mod bölümü; CHANGELOG.

**Doğrulama:**
- `cargo fmt --check`, clippy (`-D warnings`), `cargo test --workspace`: temiz (core 294, app 61, cli 22+2).
- `pnpm -C app check`, `lint`, `test` (87): temiz.
- `claude plugin validate`: geçti (repo ve kurulu kopya). `tsc`: temiz. `claude plugin test`: 41/41.
- `pnpm tauri build`: kurulum paketi (5,4 MiB) üretildi. Kuruldu; `claude-code\` altında `plugin.json`, `hooks/` (6 dosya), `types/index.d.ts` var. `bin\tokenradar.exe --version` 0.1.0.
- Kullanıcının onayıyla mod kurulu CLI ile `settings.json`'a eklendi (diff yalnızca `env` içinde tek satır, yedek alındı). Eski `pluginConfigs."tokenradar@inline".cli` geçici yolu, yine onayla ve yedekle, elle kaldırıldı.

**Ölçülmeyenler ve riskler:**
- Mod'un `CLAUDE_CODE_PLUGIN_DIRS` ile yüklenmesi ve kurulu CLI'ı bulması bu oturumda canlı denenmedi: Claude Code'un yeniden başlatılması gerekiyor. Kullanıcı yeniden başlattıktan sonra `/radar` ile görmeli.
- Kaldırıcının `mod uninstall --all` adımı gerçek kaldırmayla denenmedi; CLI komutu testlerle doğrulandı.
- Uygulama arayüzündeki kart ekran görüntüsüyle incelenmedi; svelte-check ve testlerle doğrulandı.
- `--plugin-dir` ile çift yükleme algılanamaz (ADR-047 bedeli).

**Kapsam dışı kalanlar:** Marketplace yayını; kullanıcının `pluginConfigs` ayarlarına uygulamadan dokunmak (bu oturumdaki temizlik kullanıcının açık onayıyla elle yapıldı, ürün özelliği değil).

**Sonraki adım:** Durum tablosundaki sıradaki adım.
