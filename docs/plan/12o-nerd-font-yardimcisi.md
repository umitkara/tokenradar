# Adım 12o — Nerd Font yardımcısı: tespit, indirme, kurulum ve terminal ayarı

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1.x | 12n | 2 oturum | ✅ |

## Amaç

Statusline ayarlarında, kullanıcının terminaline bir Nerd Font kurup seçmesine yardım eden bir "Terminal fontu" kartı eklemek. Kart fontu tespit eder, açık onayla indirip kurar ve Windows Terminal ile VS Code'da seçer.

Bu adım, Adım 12p'nin (simge setleri ve Nerd bar stilleri) ön koşuludur. Powerline teması 12i'den beri hiç gerçek bir Nerd Font ile görülmedi; bu makinede Nerd Font yok. Simgeler ve barlar da kör yapılmasın diye font önce gelir.

## Önce oku

- ADR-009, ADR-013, ADR-029, ADR-049, ADR-050, ADR-053
- Adım 12n'nin devir notu ("Sonraki adıma notlar": store'a bağlı değeri `StatusGet` cevabına eklemenin yolu)
- `crates/tokenradar-core/src/statusline/settings.rs` (cerrahi JSON düzenleme, diff)
- `crates/tokenradar-core/src/statusline/install.rs` (diff → onay → yedek → atomik yazma akışı)
- `crates/tokenradar-core/src/providers/claude/usage_api/client.rs` (mevcut `reqwest` kullanımı)
- `docs/design-language.md` "Terminal önizlemesi" (`--font-term`)
- `docs/performance-budget.md` (`tokenradar-app.exe` ve installer boyutu)

## Kullanıcıyla kararlaştırılanlar (2026-11-08)

- **Font uygulama içinden indirilir.** Kullanıcının açık onayıyla, sabit sürüm ve SHA-256 doğrulamasıyla. Tarayıcıya yönlendirme ve kurulum paketine gömme seçenekleri reddedildi.
- **Terminal ayarına yazılır:** Windows Terminal ve VS Code, diff → onay → yedek → atomik yazma akışıyla. Diğer terminaller için yalnızca yönerge gösterilir.
- **Font enjekte etmek mümkün değil.** Fontu terminal emülatörü seçer; Claude Code `settings.json`'unda font ayarı yok, CLI terminalin fontunu sorgulayamaz. Kart bu yüzden "kur + terminalde seç" yolunu izler.

## Kapsam

### 1. Font tespiti (core, Tauri'siz)

- Sistem (`C:\Windows\Fonts`) ve kullanıcı (`%LOCALAPPDATA%\Microsoft\Windows\Fonts`) font dosyaları ve HKCU/HKLM font kayıtları taranır.
- Her dosyanın cmap'inde üç kod noktası aranır:
  - U+E0A0 (Powerline branch): Powerline glifleri var.
  - U+F418 (`oct-git_branch`): tam Nerd Font, ≥ 3.0.
  - U+EE00 (`extra-progress_empty_left`): Nerd Fonts ≥ 3.4 (2025-04-24). Progress glifleri bu sürümde eklendi (glyphnames.json ile doğrulandı).
- Sonuç üç kademedir: **yok** / **var, < 3.4** / **var, ≥ 3.4**. Mono varyantı ayrıca işaretlenir (aile adında `Mono`).
- Cascadia Mono, Cascadia Code ve Consolas'ta bu üç kod noktası yok (2026-11-08 taraması). Segoe UI Symbol'de U+E0A0 ve U+E0B0 var, ama kendi PUA simgeleri olabilir. Tespit yalnızca U+E0A0'a dayanmamalı.

### 2. Terminal tespiti (CLI → uygulama)

- Statusline köprüsü her çağrıda yalnızca terminal **türünü** gönderir: `WT_SESSION` varsa `windows_terminal`, `TERM_PROGRAM=vscode` ise `vscode`, aksi halde `other`. Değişken değerleri, yollar ve içerik gönderilmez.
- Yol, 12n'nin notundaki desendir: IPC isteğinde `serde(default)` bir alan. Eski CLI ile yeni uygulama ve tersi çalışır.
- **Adım başında gözlenir:** Claude Code'un statusline komutuna bu ortam değişkenlerini geçirip geçirmediği (12c'deki `NO_COLOR` gözlemi gibi). Geçirmiyorsa tespit "bilinmiyor" olur ve kart iki terminali de gösterir.

### 3. İndir ve kur

- **Font:** CaskaydiaCove Nerd Font **Mono**, Nerd Fonts ≥ 3.4. Görünüşü Windows Terminal'in varsayılanı Cascadia ile aynı, artı simgeler. Mono varyantında simgeler tek hücre çizilir, bu da `unicode-width` sayımıyla uyumlu.
- **Adım başında doğrulanır:** Microsoft'un kendi "Cascadia Code NF" sürümü U+EE00 ve U+F418 içeriyor mu? İçeriyorsa ikinci seçenek olur.
- **Kaynak:** `github.com/ryanoasis/nerd-fonts` release varlığı (`CascadiaCode.zip`). Sürüm ve zip'in SHA-256'sı koda sabitlenir. Hash tutmazsa hiçbir dosya yazılmaz.
- **Onay:** İndirme tek tıkla başlamaz. Önce boyut, kaynak ve lisans (OFL-1.1) gösterilir, sonra kullanıcı onaylar.
- **Çıkarma:** Zip'ten yalnızca adı sabit listedeki Mono TTF'ler (Regular, Bold, Italic, BoldItalic) alınır. Zip-slip olmaz: hedef dosya adları zip'ten değil koddan gelir.
- **Kurulum (Windows, yönetici izni yok):** `%LOCALAPPDATA%\Microsoft\Windows\Fonts` altına kopyalanır, `HKCU\Software\Microsoft\Windows NT\CurrentVersion\Fonts` altına kaydedilir, `AddFontResourceW` + `WM_FONTCHANGE` yayını yapılır.
- **macOS ve Linux:** `~/Library/Fonts` ve `~/.local/share/fonts` + `fc-cache`. Bu adımda yalnızca belgelenir, Adım 21'de uygulanır.

### 4. Terminalde seç

- **Windows Terminal:** `profiles.defaults.font.face`.
  - Yollar: Store paketi (`Packages\Microsoft.WindowsTerminal_8wekyb3d8bbwe\LocalState`), Preview paketi ve paketsiz kurulum (`%LOCALAPPDATA%\Microsoft\Windows Terminal`).
  - Hangi dosyaların var olduğu gösterilir; kullanıcı seçer.
- **VS Code:** `%APPDATA%\Code\User\settings.json` içinde `terminal.integrated.fontFamily`. Bu anahtar yoksa VS Code editör fontunu kullanır. Yalnızca terminal anahtarı yazılır, editör fontuna dokunulmaz.
- **Akış:** Claude `settings.json` kurulumunun aynısı: diff göster → kullanıcı onaylar → zaman damgalı yedek → atomik yazma. Geri al düğmesi yedeği geri koyar.
- **Diğer terminaller** (JetBrains, conhost, macOS/Linux terminalleri): adım adım yönerge, Fluent'te (EN/TR).
- **Durum satırı:** Terminalin ayar dosyası salt okunur okunur ve seçili font adı, kurulu bir Nerd Font ailesiyle karşılaştırılır: "Windows Terminal'de seçili ✓ / değil".

### 5. Kart (ayarlar → statusline)

- Üç satır: **Font** (yok / < 3.4 / ≥ 3.4, Mono mu), **Terminal** (tespit edilen tür), **Seçili mi** (terminal başına). Her satırın yanında tek bir eylem.
- Powerline teması seçiliyken font yoksa kart uyarı gösterir. 12j'deki "simgeler kutu görünüyorsa…" notunun yerini alır.
- Yeniden başlatma ipucu: Windows Terminal ve VS Code yeni fontu ancak yeniden açılınca görebilir. Adım başında gözlenir ve gerekiyorsa yazılır.

### 6. Önizleme fontu

- Kurulu bir Nerd Font varsa `--font-term` ilk sırada onu kullanır. Önizleme için font gömmek gerekmez.

### 7. Powerline'ın ilk gerçek kontrolü

- Font bu makineye kurulduktan sonra mevcut `powerline` ve `powerline_round` temaları Windows Terminal ve VS Code'da ilk kez gerçek fontla görülür. 12i–12n'den kalan "gerçek Nerd Font ile görsel kontrol" maddesi kapanır.

## Kapsam dışı

- Simge setleri ve yeni bar stilleri (Adım 12p).
- macOS ve Linux kurulumu (Adım 21).
- JetBrains ve diğer terminallerin ayarına yazmak.
- Font güncellemesini kendiliğinden yapmak. Sabit sürüm yalnızca uygulama güncellemesiyle değişir.
- Birden çok font arasından seçim galerisi.
- Kurulu sürümü güncellemek ya da sürüm çıkarmak.

## Görevler

- [x] Adım başı gözlemleri: `WT_SESSION`/`TERM_PROGRAM` statusline komutuna geliyor mu, "Cascadia Code NF" kapsaması, WT/VS Code'un yeni fontu görmek için yeniden başlatma isteyip istemediği. Sabit sürüm ve SHA-256 belirlenir.
- [x] ADR-055: ADR-013 allowlist'ine Nerd Fonts release varlığı (onay, sabit sürüm, SHA-256, lisans); terminal ayar dosyalarına yazma kuralları.
- [x] Core `terminal_font/`: font tarama (cmap), terminal ayar yolları, okuyucu; testler (sentetik font dosyaları ve geçici dizinler).
- [x] Core: JSONC desteği (yorumlar, sondaki virgül) ve iç içe yol ile cerrahi düzenleme; `settings.rs` deseninden. Yorum ve sıra koruma testleri.
- [x] Core: indirme (`reqwest`), hash doğrulama, sabit adlı çıkarma, kullanıcı başına kurulum; hash hatası ve çevrimdışı testleri.
- [x] CLI ve IPC: terminal türü alanı (`serde(default)`), eski/yeni uyum testi.
- [x] Uygulama: "Terminal fontu" kartı, onay ve diff diyalogları, yönergeler; Fluent (EN/TR); bindings.
- [x] Bu makineye fontu kart üzerinden kur; Powerline temalarını Windows Terminal ve VS Code'da gerçek fontla gör. (Kurulum ve ayarlar yapıldı; gerçek fontla görüntü uygulamanın önizlemesinden. Terminallerde gözlem açık, devir notuna bakın.)
- [x] Ölçüm: uygulama boyutu (zip bağımlılığı) ve CLI statusline gecikmesi (`docs/perf/`).
- [x] Ekran görüntüsü onayı, ADR ve devir notu.

## Teknik notlar ve tuzaklar

- **JSONC:** Windows Terminal'in ürettiği `settings.json` yorum satırıyla başlar; VS Code ayarları yorum ve sondaki virgül içerebilir. Bugünkü `settings.rs` tarayıcısı `serde_json` ile doğruluyor ve düz JSON bekliyor. Tarayıcı yorumları atlamalı, ama dosya asla yeniden serileştirilmemeli.
- **Eski WT biçimi:** `profiles` nesne değil dizi olabilir; o zaman `defaults` yoktur. Bu durumda yazılmaz, yönerge gösterilir.
- **Aile adı tam eşleşmeli:** WT ve VS Code ailenin `name` tablosundaki adını ister (`CaskaydiaCove Nerd Font Mono`). Ad dosyadan okunur, elle yazılmaz.
- **VS Code ayar değerinde yedek fontlar:** `fontFamily` virgüllü bir liste. Mevcut değer korunup başa eklenmeli mi, yoksa değiştirilmeli mi, diff'te açıkça gösterilir.
- **Ağ:** Zaman aşımı, çevrimdışı ve proxy hataları anlaşılır bir durum olarak gösterilir. Kısmi indirme diske yazılmaz; doğrulanmadan hiçbir dosya font klasörüne gitmez.
- **Lisans:** Font dağıtılmıyor, kaynaktan indiriliyor. Yine de OFL-1.1 metni ve "CaskaydiaCove" adının nedeni (Reserved Font Name) ADR'ye yazılır.
- **Gizlilik:** Terminal türü bir enum'dur; ortam değişkeni değeri, oturum kimliği ya da yol gönderilmez ve loglanmaz.
- **Gerçek kullanıcı dosyaları:** Testler geçici dizinlerde çalışır. Bu makinedeki gerçek WT/VS Code ayarına yalnızca kart üzerinden, kullanıcının onayıyla yazılır; önce yedek alınır.

## Kabul kriterleri

- Nerd Font yokken kart "yok" diyor; kurulumdan sonra "≥ 3.4, Mono" diyor.
- Hash tutmayan bir indirme hiçbir dosya yazmıyor.
- WT ve VS Code ayarına yazma diff gösteriyor, onay istiyor, yedek alıyor; yorumlar ve anahtar sırası korunuyor; geri al çalışıyor.
- Terminal türü statusline çağrısından geliyor (ya da gözlem öyle değilse "bilinmiyor" gösteriliyor).
- Powerline temaları bu makinede gerçek fontla görüldü ve ekran görüntüsüyle kayıtlı.
- Boyut ve gecikme ölçümleri bütçede.

## Doğrulama

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
- `pnpm -C app check`, `pnpm -C app lint`, `pnpm -C app test`
- Uygulama boyutu ve statusline gecikmesi ölçümleri.
- Debug build ve portable mod ile CDP üzerinden kartın EN ve TR ekran görüntüleri (`img/12o-*.png`). Windows Terminal ve VS Code'da Powerline'ın gerçek fontla görüntüsü.

## Devir notu

**Tarih:** 2026-11-09 · **Karar:** ADR-055 · **Ölçüm:** `docs/perf/2026-11-09-nerd-font.md` · **Ekran görüntüleri:** `img/12o-font-dialog-en.png` (açık tema, EN; diyalog, font seçildikten sonra), `img/12o-statusline-tr-dark.png` (koyu tema, TR; Powerline önizlemesi gerçek fontla)

### Yapılanlar
- **Adım başı gözlemleri:**
  - Claude Code statusline komutuna kendi ortamını geçiriyor: WT'de `WT_SESSION` geliyor. VS Code `TERM_PROGRAM=vscode` koyuyor ve kendisini başlatan WT'den `WT_SESSION`'ı devralabiliyor; VS Code işareti önce bakılıyor.
  - Microsoft'un "Cascadia Code NF"si U+F418 içeriyor, U+EE00 içermiyor (3.4 öncesi kapsama); ikinci seçenek olmadı.
  - Sabit sürüm Nerd Fonts v3.5.1, `CascadiaMono.tar.xz`; arşivin ve dört dosyanın SHA-256'sı koda yazıldı.
- **Core `jsonc.rs`:** Yorumları ve sondaki virgülleri aynı uzunlukta boşluğa çeviren bir kopya üzerinde yürüyüp değişikliği özgün metne ekleyen cerrahi düzenleyici (değer okuma, iç içe yol yazma, anahtar silme, boş kalan nesneyi temizleme). `statusline/settings.rs` bunu kullanacak biçimde sadeleşti.
- **Core `terminal_font/`:**
  - `fonts.rs`: `ttf-parser` ile yalnızca `name`, `cmap`, `hhea`, `hmtx` tabloları; TTC desteği; kademe (yok / 3.4 öncesi / tam) ve mono tespiti.
  - `download.rs`: `reqwest` ile yalnızca izinli iki host, en çok 3 HTTPS yönlendirmesi, boyut sınırı, SHA-256; `lzma-rust2` ve en küçük bir ustar okuyucu; adlar koddan gelir. `is_pinned`.
  - `install.rs`: kullanıcı başına kurulum (atomik yazma, HKCU değeri, `AddFontResourceW`, `WM_FONTCHANGE`), `installed` ve `uninstall` (yalnızca sabit hash'li dosyalar; dosya kullanımdaysa font oturuma geri eklenir, `InUse` döner).
  - `terminals.rs`: WT (Store, Preview, paketsiz) ve VS Code (Stable, Insiders) yolları, okuma (`TargetState`, sorunlar, profil sayısı), `plan_set`, `plan_undo` (en yeni yedekten; `.undo.bak` kaynak olmaz), `plan_restore` ve `FontPlan::effect`.
- **CLI ve IPC:** `StatusQuery.terminal` (`windows_terminal` / `vscode` / `other`, `serde(default, skip_serializing_if)`); runtime görülen türleri ve sonuncusunu bellekte tutuyor.
- **Uygulama:** `terminal_font.rs` (görünüm modeli, token'lı tek bekleyen plan, indirme ilerlemesi olayı, `plan_restore`/`apply_restore`); IPC komutları; bindings yenilendi. `TerminalFontDialog.svelte`: Nerd Font, Terminal, Terminal ayarları ve Hepsini geri al bölümleri; indirme onayı; diff ve geri dönüş özeti görünümü. Önizlemenin altındaki açıklamada "Terminal fontu…" bağlantısı. `font.ts`: `canDraw` probu.
- **Tek tıkla geri dönüş (kullanıcı isteği, adım içinde):** Bir düğme özet açar (terminal başına önce → sonra, kaldırılacak font dosyaları); tek onay önce ayarları, sonra fontu geri alır. Yalnızca hâlâ TokenRadar'ın olan değişiklikler geri alınır.
- **Fluent (EN/TR):** `font-*` anahtarları (bölümler, durumlar, hatalar, yönergeler, geri dönüş, yeniden başlatma ipuçları).
- **Testler:** jsonc (7), fonts (4), download (5), install (3), terminals (8), IPC uyumu, runtime, CLI; ön yüz `font.test.ts` (126 test toplam).
- **Bu makinede gerçek kurulum (kullanıcı onayıyla):** Font debug uygulamanın kartıyla kuruldu (dört dosya, HKCU değerleri). WT ve VS Code ayarları iki diff gösterilip onaylandıktan sonra yazıldı; yedekler yanlarında. Kart ikisinde de "Nerd Font" gösteriyor. Geri dönüş özeti 2 terminal ve 4 dosya listeledi; uygulanmadı.
- **Dokümanlar:** ADR-055; data-sources (terminal ortamı, fontlar, WT ve VS Code ayarları); architecture §5, §11, §13; design-language (sözlük ve diyalog notu); perf kaydı ve bütçe satırları; CHANGELOG.

### Sapmalar
- **Font CaskaydiaMono, arşiv tar.xz** (plan: CaskaydiaCove, `CascadiaCode.zip`). CaskaydiaCove ligatürlü; statusline'da `->`, `!=` birleşmesin. tar.xz daha küçük; kullanıcı seçti (ADR-055).
- **Font kayıt defteri taranmıyor,** yalnızca iki font klasörü. Başka yerden kurulmuş Nerd Font'lar "yok" görünebilir (ADR-055 bedeli).
- **Mono tespiti aile adından değil,** U+F418 genişliğinin 'A' ile karşılaştırılmasından.
- **Tek tıkla geri dönüş eklendi** (kullanıcı isteği). Dosya başına diff yerine tek özet onayı: diff → onay kuralından bilinçli sapma, ADR-055'te yazılı.
- **Kart yerine diyalog; giriş önizleme açıklamasında.** Font düğmesi önce araç çubuğundaydı ve çubuğu iki satıra taşırdı (benim gerilemem, ekran görüntüsünde görüldü); bağlantı önizlemenin altındaki açıklamaya taşındı.
- **Önizleme yanlış iddiada bulunuyordu:** Font yeni kurulduğunda WebView onu çizemezken açıklama "önizleme bu fontu kullanıyor" diyordu. `canDraw` probu eklendi; çizemiyorsa açıklama yeniden başlatmayı söylüyor.
- **WT ve VS Code'da Powerline'ın gerçek fontla görüntüsü alınmadı.** Gerçek fontla görüntü uygulamanın önizlemesinden (yeniden başlatmadan sonra). Terminallerde gözlem kullanıcıya kaldı (aşağıda).
- **Kurulu uygulama ölçüm ve ekran görüntüleri için kapatıldı,** sonra yeniden başlatıldı (kullanıcı onayıyla).

### Doğrulama
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` temiz; `cargo test --workspace` 504 test (çekirdek 367 → 394; gerçek makineye dokunan testler `ignored`).
- `pnpm -C app check`, `lint` ve `test` temiz (126 test).
- Boyut: uygulama 14.477.312 B (13,81 MiB, +321.536 B), CLI 2.483.200 B (+1.536 B). Statusline p95 açık 13,6–14,5 ms, kapalı 23,7–27,4 ms (bir koşuda tekrarlanmayan 74,1 ms). Hepsi bütçede.
- Kurulum: doğrulama ve çıkarma tepe private 102,2 MiB, 0,30–0,33 s; ardından bellek geri veriliyor. Font taraması (426 yüz) ilk koşu 294 ms, sıcak 18–19 ms.
- WebView2 canvas ölçümü: kurulumdan sonra genişlik değişmedi (22,0), uygulama yeniden başlayınca 23,4; iki aile adı da çözülüyor.
- Debug build ve CDP üzerinden diyalog EN ve önizleme TR/koyu ekran görüntüleri; kullanıcıya gösterildi.

### Açık konular
- **WT ve VS Code'un yeni fontu görmek için yeniden başlatma isteyip istemediği gözlenmedi** (kullanıcı bakmadı). İpucu metni yeniden başlatmayı kesin bir adım olarak yazıyor; Chromium için ölçüldü.
- **Terminal türü yeni CLI'ı bekliyor.** Kurulu v0.1.0 CLI alanı göndermiyor; kart "çağrı görülmedi" diyor.
- **Geri dönüşün uygulanması gerçek makinede denenmedi,** fontun kullanımdayken (`InUse`) davranışı da. Testler geçici dizinlerde ve sabit hash eşleşmesinde.
- **NSIS installer boyutu ölçülmedi.**
- **Kurulumun geçici belleği ≈ 102 MiB.** Bütçe satırı yok; tray tabanıyla "ilk indeksleme ≤ 120 MB" tavanına dayanıyor. Ayrı süreçte çıkarma ya da zip ile düşürülebilir; karar bekliyor.
- **Bu makinenin durumu:** Font kurulu; WT ve VS Code ayarlarında CaskaydiaMono Nerd Font Mono seçili, TokenRadar yedekleri yanlarında. Geri dönmek için diyalogdaki "Eski haline getir…" ya da terminal başına "Geri al".
- **macOS ve Linux kurulumu** Adım 21'de.

### Sonraki adıma notlar (12p)
- Bar parçaları Powerline blokları içinde çizilmiyor (`render.rs` notu); `capsule` ve `solid` bunu ele almalı.
- Kurulu fontun kademesi `TerminalFontView`'da var; `capsule` uyarısı için onu kullanın (3.4 öncesinde U+EE00 yok).
- Önizleme ancak uygulama yeniden başlayınca yeni fontu çizer. Görsel kontrolü uygulamayı yeniden başlattıktan sonra yapın; `canDraw` açıklaması bunu gösterir.
- Simgelerin hücre genişliği için gerçek font artık bu makinede: WT ve VS Code'da Mono varyantla kontrol edilebilir.
