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

- [ ] Core: `refresh_interval_sec` ayarı, esnek okuma; `CommandValue` ve `statusLine` birleştirmesi; kurulum kaydı ve kaldırma; testler (kullanıcı anahtarları korunur, kullanıcının kendi `refreshInterval`'ı korunur).
- [ ] Core: push tekrarı tespiti (daemon ve spool); testler.
- [ ] Core: `GitStatus`, porcelain v2 ayrıştırıcı (sentetik çıktılarla: detached HEAD, upstream yok, rename, conflict, submodule), önbellek, zaman aşımı, arka plan hesabı; testler.
- [ ] IPC: `StatusQuery.cwd`, `StatusReply.git`; eski/yeni uygulama ve CLI uyumluluk testleri.
- [ ] Render: `git_status` biçimleri, `cache_timer` `countdown`, `fit()` sırası; testler.
- [ ] Doctor: yenileme aralığı, git bulunamadı, izlenmeyen dosya taraması kapandı.
- [ ] Bindings ve düzenleyici: "Canlı yenileme" seçicisi, `countdown` uyarısı, `git_status` öğe editörü, Fluent.
- [ ] Ölçümler (bkz. Doğrulama) ve `docs/perf/` kaydı.
- [ ] İlk çalışan hâlin ekran görüntüsü kullanıcıya gösterilir ve onaylanır, ardından cila.
- [ ] ADR-059 sonuçları ve devir notu.

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

_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
