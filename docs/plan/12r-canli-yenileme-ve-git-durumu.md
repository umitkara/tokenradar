# Adım 12r — Statusline canlı yenileme ve git durumu

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1.x | 12q | 1–2 oturum | ⬜ |

## Amaç

Statusline'a zamanla değişen iki şey eklemek:

1. **Canlı yenileme.** Claude Code statusline'ı yalnızca olaylarda yeniden çizer: yeni mesaj, permission mode değişimi, vim modu. Oturum boştayken satır donar. Bu adım kuruluma isteğe bağlı `refreshInterval` ekler. Böylece `cache_timer` öğesinin geri sayım biçimi ve limitlerin sıfırlanma geri sayımı boştayken de doğru kalır.
2. **Git durumu.** `git_status` öğesi staged, değişmiş ve izlenmeyen dosya sayısını ve upstream'e göre önde/geride sayısını gösterir. Bu, [claude-code-tool-manager](https://github.com/tylergraydev/claude-code-tool-manager)'da olup bizde olmayan tek segmenttir.

İki iş aynı adımda, çünkü ikisi de statusline'ın çalışma maliyetini değiştirir ve birlikte ölçülmesi gerekir. Yenileme aralığı git durumunun ne sıklıkla hesaplanacağını da belirler.

## Önce oku

- `CLAUDE.md` "User config is sacred" (diff → onay → backup → atomic write), ADR-013, ADR-038, ADR-049, ADR-059
- Adım 12q'nun devir notu
- `docs/performance-budget.md`: statusline p95, boşta CPU, uyanma kuralı
- Resmi şema: <https://code.claude.com/docs/en/statusline> ("How status lines work", `refreshInterval`, "Cache expensive operations")
- `crates/tokenradar-core/src/statusline/{install.rs, settings.rs, git.rs}`, `crates/tokenradar-cli/src/statusline.rs` (`deliver`)
- `crates/tokenradar-core/src/ipc/{proto.rs, view.rs}` (`StatusQuery`, `StatusReply`)
- `crates/tokenradar-core/src/doctor/mod.rs`

## Kullanıcıyla kararlaştırılanlar (2026-11-11)

- **Kapsam:** Araştırmada önerilen öğelerin hepsi ("hepsini içeren bir plan hazırla"). Bu adım onların kalanını, yani canlı yenileme, geri sayım ve git durumunu içerir.
- **Canlı yenileme varsayılan olarak kapalı;** seçenekler 5, 10 ve 30 saniye.
- **Git durumu daemon'da, önbellekli hesaplanır;** daemon kapalıyken öğe gizlenir.

## Kapsam

### 1. `refreshInterval` kurulumu

- **Ayar:** `[accounts.*.statusline]` altında `refresh_interval_sec: Option<u16>`. Seçenekler: kapalı (varsayılan), 5, 10, 30. Esnek okuma (ADR-042): diğer değerler en yakın seçeneğe yuvarlanır, 0 kapalıdır.
- **Kurulum ve güncelleme:**
  - `CommandValue` isteğe bağlı `refreshInterval` alanını yazar.
  - Değer değişince mevcut akış çalışır: diff → kullanıcı onayı → backup → atomic write.
  - Kullanıcının `statusLine` nesnesine eklediği başka anahtarlar (`padding`, `hideVimModeIndicator`) **korunur**. Bugün `set_member` nesnenin tamamını yazıyorsa, yalnızca bizim anahtarlarımızı değiştiren bir birleştirmeye geçilir.
- **Kaldırma:** `refreshInterval` bizim yazdığımız değerse kaldırılır. Kullanıcı kendisi yazdıysa, yani kurulumdan önce de varsa, dokunulmaz. Bu bilgi kurulum kaydında tutulur.
- **Zincir uyarısı:** `--chain` ile zincirlenmiş önceki komut da her aralıkta çalışır. Ayarlar ekranı bunu söyler.
- **Doctor:** Etkin aralığı ve ayarla `settings.json` arasındaki farkı raporlar.
- **Düzenleyici:** Satır ayarlarına "Canlı yenileme" seçicisi. Açıklama metni bedeli söyler: "Her N saniyede bir `tokenradar` çalışır".

### 2. Yenilemenin bedeli: push tekrarları

- Her yenileme bir CLI başlatması, bir pipe turu ve bir `statusline.push` demektir. Boştaki bir oturumda payload değişmez.
- **Görev:** Daemon'un push işleyicisi bir önceki push ile aynı içerikteki payload'ı store'a yazmaz, yalnızca `last_seen` değerini bellekte tazeler. Aynılık, `observed_at` hariç payload hash'iyle belirlenir. Bugün zaten böyleyse ölçümle gösterilir.
- **Spool:** Daemon kapalıyken her tick spool'a bir satır ekler. Aynı oturumdan art arda gelen aynı payload spool'a yazılmaz; kontrol yalnızca son satıra bakar ve dosya kilidi almaz.
- **Ölçüm:** 10 dakika, açık ama boşta bir Claude Code oturumu, 5 saniyelik aralık.
  - Uygulamanın CPU'su ve uyanmaları (`measure-threads.ps1`).
  - Store yazma sayısı.
  - CLI p95.
  - Sentetik koşu ajan tarafından, gerçek boşta koşu kullanıcı tarafından yapılır (bkz. tuzaklar).

### 3. Geri sayım biçimleri

- **`cache_timer` için `countdown`:** `4m12s` ya da `4m`. Saniye yalnızca kalan süre 2 dakikanın altındaysa gösterilir.
- **Bayatlık kuralı:**
  - Yenileme kapalıyken editör `countdown` seçeneğinin yanında "boştayken donar, canlı yenileme önerilir" uyarısını gösterir.
  - Satır, geri sayımın son çizim anından beri bayatlamış olduğunu bilemez. Bu yüzden yenileme kapalıysa `countdown` varsayılan olmaz, `clock` kalır.
- **Limit geri sayımları:** Mevcut `countdown` sıfırlanma biçimi değişmez. Yenileme açıkken doğal olarak tazelenir.

### 4. `git_status` öğesi

| `kind` | Grup | Örnek | Biçimler |
|---|---|---|---|
| `git_status` | Proje | `+2 ~3 ?1 ↑1` | `compact` (varsayılan), `dirty` (`●` / boş), `ahead_behind` (`↑1 ↓2`) |

- **Nerede hesaplanır: daemon (ADR-059).**
  - `StatusQuery` isteğe bağlı `cwd` taşır.
  - `StatusReply` isteğe bağlı `git: Option<GitStatus>` döner (`serde(default)`, eski uygulamalarla uyumlu, 0.1.1'deki `absent` deseni).
  - Daemon kapalıyken (spool yolu) öğe gizlenir. Bayat ya da tahmini bir sayı gösterilmez.
- **Komut:** `git --no-optional-locks status --porcelain=v2 --branch -z`
  - `--no-optional-locks`, kullanıcının kendi git komutlarıyla `index.lock` yarışını önler.
  - `CREATE_NO_WINDOW` ile konsol penceresi açılmaz.
  - 2 saniye zaman aşımı; aşılırsa süreç sonlandırılır ve öğe o tur için gizlenir.
  - Çalışma dizini, repo kökü ve `GIT_DIR` 12i'deki `git.rs` mantığıyla bulunur. Worktree ve submodule desteklenir.
- **Önbellek:**
  - Anahtar repo köküdür.
  - Önbellek şu durumlarda geçersiz olur: `.git/HEAD` ya da `.git/index` mtime değişti, ya da 5 saniyelik TTL doldu. TTL, izlenmeyen dosyaların index'e dokunmadan değişebilmesi yüzünden var.
  - En fazla 8 repo tutulur, LRU.
  - Hesap pipe yanıtını bekletmez. İstek geldiğinde önbellekte taze değer varsa döner, yoksa arka planda başlatılır ve o tur öğe gizli kalır. Bir sonraki çizimde görünür.
- **Büyük repolar:** Öğe alanı `untracked: bool` (varsayılan `true`). `false` iken `-uno` kullanılır. İki ardışık zaman aşımında daemon o repo için izlenmeyen dosya taramasını kendiliğinden kapatır ve öğe `?` yerine `?…` gösterir. Bu durum doctor'da da görünür.
- **`git` yoksa** öğe gizlenir; doctor `git` bulunamadı der. PATH dışında git aranmaz.
- **Gizlilik:** Yalnızca sayılar tutulur. Dosya adları porcelain çıktısından okunur ama saklanmaz, log'a ve IPC'ye girmez.

### 5. Model, geriye uyum ve şablonlar

- `config.toml` `version = 2` olarak kalır. Yeni alanlar isteğe bağlıdır.
- Şablonlar değişmez. `git_status` hiçbir varsayılan şablona girmez.
- Örnek veri `git_status` için `+2 ~3 ?1 ↑1` üretir.
- Fluent (EN/TR): öğe adı ve ipucu, biçim adları, "Canlı yenileme" seçicisi ve açıklamaları, `countdown` uyarısı, doctor satırları.

## Kapsam dışı

- 1 saniyelik yenileme. Bütçe açısından savunulamaz; resmi minimum 1 olsa da seçenek olarak sunulmaz.
- PR ve CI durumunu `gh` ile sorgulamak (ADR-013). `pr` öğesi 12q'da payload'dan gelir.
- Jujutsu (`jj`) desteği.
- Git durumunu CLI'da senkron hesaplamak (ADR-059, alternatifler).
- Kurulu sürümü güncellemek ya da sürüm çıkarmak.

## Görevler

- [x] Core: `refresh_interval_sec` ayarı, esnek okuma; `CommandValue` ve `statusLine` birleştirmesi; kurulum kaydı ve kaldırma; testler (kullanıcı anahtarları korunur, kullanıcının kendi `refreshInterval`'ı korunur).
- [x] Core: push tekrarı tespiti (daemon ve spool); testler.
- [x] Core: `GitStatus`, porcelain v2 ayrıştırıcı (sentetik çıktılarla: detached HEAD, upstream yok, rename, conflict, submodule), önbellek, zaman aşımı, arka plan hesabı; testler.
- [x] IPC: `StatusQuery.cwd`, `StatusReply.git`; eski/yeni uygulama ve CLI uyumluluk testleri.
- [x] Render: `git_status` biçimleri, `cache_timer` `countdown`, `fit()` sırası; testler.
- [x] Doctor: yenileme aralığı, git bulunamadı, izlenmeyen dosya taraması kapandı.
- [x] Bindings ve düzenleyici: "Canlı yenileme" seçicisi, `countdown` uyarısı, `git_status` öğe editörü, Fluent.
- [x] Ölçümler (bkz. Doğrulama) ve `docs/perf/` kaydı.
- [x] İlk çalışan hâlin ekran görüntüsü kullanıcıya gösterilir ve onaylanır, ardından cila.
- [x] ADR-059 sonuçları ve devir notu.

## Teknik notlar ve tuzaklar

- **`index.lock` yarışı:** `git status` index'i tazelemek için kilit alabilir. Kullanıcı aynı anda `git commit` çalıştırırsa "index.lock exists" hatası alır. `--no-optional-locks` zorunludur; bir test komut satırında bu bayrağın varlığını denetler.
- **Windows'ta süreç başlatma pahalıdır** (≈ 20–80 ms). CLI'da senkron çalıştırmak p95 ≤ 30 ms bütçesini tek başına aşar; bu yüzden daemon'da ve önbellekli.
- **Ajan oturumu transcript yazar:** Boşta ölçümü ajan yapamaz, çünkü kendi oturumu boşta değildir. Ajan portable bir sentetik koşu yapar, gerçek boşta koşuyu kullanıcı yapar ve sonucu paylaşır.
- **Gerçek veriye dokunulmaz:** CLI testleri portable kopya ile yapılır. Gerçek `settings.json` yalnızca kullanıcı onayıyla ve backup ile değişir.
- **Kişisel kullanım git'e girmez:** Ölçüm kayıtlarına repo adı, yol ya da dosya sayısı yazılmaz, yalnızca süreler ve sayaç oranları yazılır.
- **Boşta bütçesi** "oturum yok" durumu içindir (< %0,1). Yenileme yalnızca bir Claude Code oturumu açıkken çalışır; yine de açık ama boştaki oturum için ölçülen değer kayda geçer ve bütçe tablosuna yeni bir satır eklenir.

## Kabul kriterleri

- Yenileme aralığı ayarlardan açılıp kapatılabiliyor. `settings.json` değişikliği diff ve onayla yapılıyor, kullanıcının diğer anahtarları korunuyor, kaldırma bizim eklediğimizi geri alıyor.
- Boştaki bir oturumda 5 saniyelik aralıkla store'a tekrar yazımı yok. CPU ve uyanma sonuçları `docs/perf/`'te.
- `git_status` daemon açıkken doğru sayıları gösteriyor, daemon kapalıyken gizleniyor. Kullanıcının `git commit` işlemi asla `index.lock` hatası almıyor.
- `tokenradar statusline` p95 bütçede kalıyor (daemon açık ≤ 30 ms, kapalı ≤ 40 ms), `git_status` öğesi ekliyken de.
- EN ve TR ekran görüntüleri kayıtlı.

## Doğrulama

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
- `pnpm -C app check`, `pnpm -C app lint`, `pnpm -C app test`
- `scripts/measure-statusline.ps1`: `git_status` kapalı ve açık, önbellek sıcak ve soğuk.
- `scripts/measure-threads.ps1 -DurationSec 600`: yenileme kapalı ve 5 saniye (sentetik); kullanıcının gerçek boşta koşusu.
- Debug build ve portable mod ile CDP üzerinden EN ve TR ekran görüntüleri (`img/12r-*.png`).

## Devir notu

**Tarih:** 2026-11-11 · **Karar:** ADR-059 · **Ölçüm:** `docs/perf/2026-11-11-live-refresh-git-status.md` · **Ekran görüntüleri:** `img/12r-git-status-en.png`, `img/12r-refresh-dialog-en.png`, `img/12r-countdown-tr-dark.png`

**Yapılanlar:**
- **Canlı yenileme:** `[accounts.*.statusline] refresh_interval_sec` (kapalı, 5, 10, 30; esnek okuma, diğer değerler en yakına yuvarlanır, ≤ 0 kapalı). Kurulum mevcut bir `statusLine` nesnesine birleşir (`padding`, `hideVimModeIndicator`, kullanıcının `refreshInterval`'ı korunur; yalnızca `type`, `command` ve bizim aralığımız yazılır). Yeni `plan_refresh` yalnızca `statusLine.refreshInterval`'ı değiştirir (`settings::set_nested`). `effective_refresh` dosyadaki değeri okur.
- **Uygulama:** Statusline sayfasında "Canlı yenileme" seçicisi (köprü kuruluyken); bedel metni, dosyadaki değer farklıysa uyarı. `settings.json` değişikliği mevcut diff diyaloğuyla (`plan_statusline_refresh` → `apply_statusline`), yalnızca uygulama ayarı değişiyorsa doğrudan kaydedilir. Zincirlenmiş komut varsa diyalog onun da her yenilemede çalışacağını söyler.
- **Push tekrarı:** `StatuslinePayload::content_key` (serileştirilmiş payload'ın hash'i; süre alanları serileştirilmez). CLI köprüsü oturum başına son anahtarı hatırlar (en fazla 64), tekrar `LimitsMsg::StatuslineRepeat` olur; limit thread'i snapshot'ı tekrarda en fazla dakikada bir kurar. Spool son satıra bakar (16 KiB kuyruk, kilitsiz) ve aynı oturum + içerik heartbeat içinde gelirse yazmaz.
- **`git_status`:** `statusline/git_status.rs`: porcelain v2 `-z` ayrıştırıcı, `--no-optional-locks` ile `run_git` (CREATE_NO_WINDOW, 2 s zaman aşımı, 32 MiB çıktı sınırı), `GitStatusCache` (repo kökü anahtarı, 8 repo LRU, 5 s TTL, `HEAD`/`index` mtime, 1 dk bayat sonuç, iki zaman aşımında `-uno`, git yoksa bir daha denenmez). IPC: `StatusQuery.{cwd, git_untracked}`, `StatusReply.git`. CLI klasörü yalnızca etkin bir `git_status` öğesi varken gönderir.
- **Render:** `git_status` biçimleri `compact` (`+2 ~3 ?1 !1 ⇡1 ⇣2`, temizse `✓`), `dirty` (`*`, Nerd U+F444), `ahead_behind`; `cache_timer` `clock` (varsayılan) ve `countdown` (`4m`, son iki dakikada `1m12s`). Öğe alanı `untracked` (yalnızca `false` iken yazılır). Editör: biçimler, `untracked` anahtarı, yenileme kapalıyken `countdown` uyarısı.
- **Doctor:** her Claude dizini için seçilen ve dosyadaki aralık (`statusline_refresh`), `git_available`, çalışma klasöründe `git_untracked_skipped`.
- **Testler:** çekirdek 471, CLI 27 (klasör yalnızca öğeyle gider; uygulama yokken öğe gizli), uygulama 101, frontend 133. Yeni `line/tests/v12r.rs` (8 test), `git_status/tests.rs` (15 test, gerçek git dahil).

- **Ekran görüntüleri:** debug build, portable mod ve sahte bir Claude dizini (`CLAUDE_CONFIG_DIR`) ile CDP üzerinden; `PrintWindow` ile yakalandı. Diff diyaloğundaki geçici yol, görüntü repoya girmeden önce DOM'da `C:\Users\you\.claude` ile değiştirildi (kullanıcı adı içeriyordu). Diyalog onaylanınca sahte `settings.json` yedeklenip yazıldı; `padding` korundu. Portable veri dizini ve işaret dosyası sonra silindi; kurulu uygulama yeniden başlatıldı.
- **Görüntülerde bulunup düzeltilenler** (`fix(app)` commit'i):
  - Önizlemede seçili `git_status` öğesinin parçaları arasındaki boşluklar gri kutu gibi boyanıyordu. Boşluk tonu artık yalnızca tümüyle boş öğelere (boşluk, esnek boşluk) uygulanıyor.
  - Yenileme bildirimi ("Done. Backup of the old file: …") çözülmüş dizge olarak saklandığı için dil değişince İngilizce kalıyor ve kalıcı olduğu için bedel/kapalı açıklamasını gizliyordu. Artık çizimde çevriliyor ve 8 saniyede kayboluyor.
  - Seçicinin görünür etiketi yoktu; "Canlı yenileme" etiketi eklendi.
- **Kullanıcı incelemesinden sonra (2026-11-12):** Diyalog görüntüsü `fix(app)` commit'inden önce çekildiği için etiketi göstermiyordu; diyalog ve TR koyu görüntüsü güncel build ile yeniden çekildi. TR geri sayım uyarısındaki "kapalıyken oturum boştayken" ifadesi "kapalıysa, oturum boştayken" oldu. Diyalog önizlemenin üstünü örttüğü için önizleme boş görünüyor; sağda kalan siyah alan satırların boş kısmı, hata değil. Kullanıcı görüntüleri bu düzeltmelerle onayladı.
- **Görüntülerde kalan, bu adımdan önceki tuhaflıklar:** önizlemede 120 sütunda da yatay kaydırma çubuğu var (ekran `120ch` genişliğinde, panel daha dar); öğe listesinde uzun satırlar `…` ile kesiliyor (ör. haftalık limit). Türkçe arayüzde varsayılan etiket `cache` İngilizce kalıyor (önceki adımların kararı).

**Sapmalar (ADR-059 sonuçlarında):**
- Heartbeat kaldı (kullanıcı kararı); tekrarda snapshot dakikada bir kurulur.
- Glifler: `⇡`/`⇣`/`*`/`??` (plan: `↑`/`↓`/`●`/`?…`; hepsi Ambiguous).
- Bayat sonuç (en fazla 1 dk) yeniden hesap sürerken sunulur.
- Kurulum kaydına alan eklenmedi; `previous` kullanıcının `refreshInterval`'ını zaten taşıyor.
- Yalnızca uygulama ayarı değişiyorsa diff gösterilmez.

**Açık konular:**
- **Uygulama kapalıyken p95 aykırılığı:** 12r'nin 11 "kapalı" koşusundan 3'ü tümüyle yavaşladı (p95 59–65 ms, p50 de yükseldi), `main`'in 10 koşusunda görülmedi. Medyan bütçede (33,0 ms). Spool kuyruk okumasını kapatan bir A/B ile yeniden bakılmalı.
- **Gerçek boşta koşu kullanıcıda:** açık ama boştaki bir Claude Code oturumu, canlı yenileme 5 sn, 10 dk: `.\scripts\measure-threads.ps1 -DurationSec 600` (kurulu uygulamada, yenileme açıkken; sürüm güncellemesinden sonra).
- **Kaynağı belirsiz test:** `config::tests::the_refresh_interval_is_read_leniently` oturum sırasında, bilerek yazılmadan ve bozuk çok satırlı bir dizgeyle dosyada belirdi; başka oturum yoktu. Dizge düzeltilip test tutuldu (büyük olasılıkla bir heredoc kalıntısı).
- Kurulu uygulama 12r'yi içermiyor; canlı yenileme ve `git_status` bir sonraki sürümle gelir.
- **Statusline ile API arasında 1 puanlık yuvarlama farkı (2026-11-12, kullanıcı fark etti; Adım 12s):** Panel haftalık limiti Claude Code'un `/usage` ekranından 1 puan düşük gösterebiliyor. Her iki kaynak da yalnızca tam sayı veriyor. Gerçek veride ±2 dk içindeki API/statusline çiftlerinde fark yaklaşık yarı yarıya 0 ya da −1 (statusline − API); +1 hiç görülmedi. Bu dağılım şuna uyuyor: API en yakın tam sayıya yuvarlıyor, statusline aşağı yuvarlıyor (floor). Bu, belgelenmiş bir davranış değil, veriden yapılan bir çıkarım. Füzyon kuralına göre (30 s içinde statusline, sonra en yeni okuma) panel çoğu zaman statusline değerini gösteriyor ve API yoklamasından hemen sonra 1 puan yukarı gidip geri gelebiliyor.
  - **Öneri (yeni ADR gerekir, ayrı adım):** Okumaları aralık olarak birleştirmek: statusline `n` → `[n, n+1)`, API `m` → `[m−0,5, m+0,5)`. Gösterilen değer kesişimden üretilir (ör. `[5,5, 6)` → 6). Kesişim boşsa (API yoklamaları arasında statusline yükseldiyse) en yeni okumanın aralığına geçilir. Böylece değer `/usage` ile tutarlı olur ve tahmine dönmeden iki ölçümün birleşimi kalır. Daha zayıf seçenekler: API'yi hep tercih etmek (tazelik kaybı) ya da statusline'a +0,5 eklemek (ölçümü tahminle değiştirmek).
  - Önce doğrulanmalı: API ve statusline okumalarının çiftleri `limit_snapshots`'tan toplanıp floor/round varsayımının tüm limit türlerinde (5 saat dahil, bu hesapta yok) geçerli olup olmadığına bakılmalı.

**Sonraki adıma notlar:**
- `git_status` hiçbir şablonda yok (plan gereği). Yeni öğe ekleyen adımlar `SegmentKind::ALL` (39) ve `Format::ALL` (32) sayılarını günceller.
- Glif seçerken `width_cjk() == Some(1)` testi şart; `…`, `↑`, `●` Ambiguous.
- `CoreIpc.status` artık `query.cwd` varsa git önbelleğine bakar; `StatusQuery`'yi kuran her yer `..StatusQuery::default()` kullanmalı.
- 5 sn yenileme + `git_status` git'i neredeyse her çizimde çalıştırır (TTL 5 sn). Büyük depolarda maliyet artarsa TTL'yi yenileme aralığına göre büyütmek düşünülebilir (ör. TTL = max(5 s, aralık × 2)); bu adımda değiştirilmedi.
- Hiç istemci yokken `core-ipc` 10 dakikada 131 kez uyandı; `main` ile karşılaştırılmadı.
