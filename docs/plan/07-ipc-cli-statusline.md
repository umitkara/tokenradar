# Adım 07 — IPC, CLI ve statusline köprüsü

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1 | 06 | 1-2 oturum | ⬜ |

## Amaç

Birincil limit kaynağı olan **statusline köprüsünü (S1)** devreye almak. Bu adım üç parçadan oluşuyor:

1. Uygulama içinde güvenli bir IPC sunucusu.
2. Hızlı bir konsol CLI'ı (`tokenradar.exe`).
3. Kullanıcının mevcut statusline'ını bozmadan kurulum ve kaldırma.

Uygulama kapalıyken bile veri kaybolmaz: CLI veriyi spool dosyasına yazar.

## Önce oku

- `docs/data-sources.md` S1
- `docs/architecture.md` §11, §13
- `docs/decisions.md`: ADR-005, ADR-009, ADR-017
- `docs/performance-budget.md` (statusline gecikmesi, CLI boyutu)

## Kapsam

- `core::ipc`: protokol tipleri (sürümlü), framing, istemci yardımcıları.
- Uygulama tarafı: IPC sunucusu (`interprocess`, tokio) ve kullanıcıya özel DACL.
- `tokenradar-cli` komutları: `statusline`, `status [--json]`, `limits [--json]`, `sessions [--json]`, `statusline install`, `statusline uninstall`.
- Spool: yazma (CLI) ve içe aktarma (uygulama başlangıcında ve periyodik).
- Statusline çıktı biçimlendirici (EN/TR, ANSI, `COLUMNS` ile genişlik uyumu).

## Kapsam dışı

- Ayarlar UI'ındaki kurulum diyaloğu (Adım 11). Burada yalnızca CLI ve çekirdek API'si yazılır, UI bu API'yi çağıracak.
- `doctor` komutu (Adım 11).

## Görevler

### IPC

- [ ] Protokol (`core::ipc::proto`):
  - Satır sonuyla ayrılmış JSON.
  - `Request { v: u8, id: u32, op: Op }` ve `Response { v, id, ok, data | error }`.
  - `Op` değerleri: `Hello { client, version }`, `StatuslinePush(StatuslinePayload)` (yanıt beklenmez), `StatusGet`, `LimitsGet`, `SessionsLive`.
  - **İleri uyumluluk:** Bilinmeyen `op` → `error: "unsupported"`.
- [ ] Pipe adı: `\\.\pipe\tokenradar-<sha256(kullanıcı SID)[..12]>`. SID, `GetTokenInformation(TokenUser)` ile alınır.
- [ ] **Güvenlik:**
  - Sunucu pipe'ı açık bir security descriptor ile oluşturulur: SDDL `D:P(A;;GA;;;<UserSID>)`; gerekiyorsa SYSTEM eklenir.
  - `PIPE_REJECT_REMOTE_CLIENTS` bayrağı set edilir.
  - `interprocess` kütüphanesinin bu ayarları destekleyip desteklemediği doğrulanmalı. Desteklemiyorsa `windows-sys` ile pipe oluşturulup tokio'ya sarılır (gerekçeli `unsafe`).
  - Erişim izinleri bir testle doğrulanmalı (en azından SDDL'nin okunup karşılaştırılması).
- [ ] Sunucu: eşzamanlı istemciler, istemci başına 2 s okuma zaman aşımı, en fazla 1 MB mesaj boyutu. Hatalı mesaj bağlantıyı kapatır ama sunucuyu etkilemez.

### Statusline akışı

- [ ] `StatuslinePayload`: S1 JSON'unun ihtiyaç duyulan alt kümesi:
  - `session_id`, `session_name`, `transcript_path`, `cwd`
  - `model.id`, `cost.total_cost_usd`, `context_window.used_percentage`
  - `rate_limits.{five_hour,seven_day}.{used_percentage,resets_at}`, `version`
  
  Ham JSON **saklanmaz**, yalnızca bu alanlar alınır.
- [ ] Uygulama tarafı: `StatuslinePush` → `LimitObservation` (`source: Statusline`, `session_uuid`) → `LimitEngine::ingest`. Hesap eşlemesi `transcript_path` önekinin hesabın `config_dir` değeriyle eşleşmesine göre yapılır (v0.3'te çoklu hesap için kritik; şimdiden doğru yazılsın).
- [ ] `tokenradar statusline` (CLI):
  1. stdin'i oku (en fazla 1 MB, 200 ms zaman aşımı).
  2. Pipe'a 50 ms zaman aşımıyla bağlan, `Hello` gönder (sürüm), ardından `StatuslinePush` gönder ve `StatusGet` ile zenginleştirme verisini al (oturumun ≈ pay değeri, projeksiyon).
  3. Satırı biçimlendir. Varsayılan şablon: `◔ 5s %72 · 1sa48dk ▲38dk │ 7g %34 │ ≈%9 bu oturum`. Dil config'den okunur, renkler eşiklere göre seçilir, `COLUMNS` dar ise kısaltılmış biçim kullanılır.
  4. Daemon yoksa: `spool/statusline-YYYYMMDD.jsonl` dosyasına tek satır ekle (dosya kilidi, günlük en fazla 5 MB, 14 gün saklama) ve satırı yalnızca stdin verisiyle biçimlendir.
  5. `--chain`: config'deki `previous_command`, **aynı stdin ile** çalıştırılır (300 ms zaman aşımı). Çıktısı bizim satırımızın önüne veya arkasına eklenir (config: `chain_position`).
- [ ] Spool içe aktarma: Uygulama başlangıcında ve 5 dakikada bir `spool/*.jsonl` dosyaları okunur. Gözlemler zaman damgalarıyla `ingest` edilir, ardından dosyalar silinir (içe aktarma idempotent: aynı `observed_at` + session tekrarlanmaz).
- [ ] `statusline install`:
  1. `settings.json` dosyasının yolunu bul (`CLAUDE_CONFIG_DIR` veya `~/.claude`).
  2. Mevcut `statusLine` değerini oku.
  3. Varsa `previous_command` olarak config'e kaydet.
  4. Yeni değeri hazırla: `{"type":"command","command":"<CLI tam yolu> statusline[ --chain]"}`. `refreshInterval` eklenmez; olay güdümlü güncellemeler yeterli.
  5. **Diff göster** (CLI: unified diff, `--yes` olmadan onay ister).
  6. Yedek al: `settings.json.tokenradar-<ts>.bak`.
  7. Atomik yaz: geçici dosya + `ReplaceFileW`/rename. Anahtar sırası korunur (`serde_json` `preserve_order`), girinti 2 boşluk.
- [ ] `statusline uninstall`: `previous_command` geri yazılır, kayıt yoksa `statusLine` anahtarı silinir. Yine diff gösterilir ve yedek alınır.
- [ ] **Windows'ta komut çalıştırma doğrulaması:** Claude Code'un statusline komutunu Windows'ta hangi kabukla çalıştırdığı (cmd, PowerShell, Git Bash) belirlenmeli. Mevcut kurulumdaki örnek `python C:\Users\…\.claude\statusline.py` ters bölülerle yazılmış ve çalışıyor. **Boşluk içeren yollar** (`C:\Users\Ad Soyad\…`) için doğru tırnaklama gerçek bir Claude Code oturumunda test edilmeli. Bulgu `data-sources.md` S1'e eklenmeli.

### Diğer CLI komutları

- [ ] `status`, `limits`, `sessions`: Daemon varsa IPC üzerinden, yoksa SQLite'tan **salt-okunur** (`SQLITE_OPEN_READ_ONLY`, WAL okuyucusu) veri alınır. `--json` makine çıktısı (kararlı şema, `"schema": 1`) verir. Varsayılan çıktı insan okuması için hizalı bir tablodur.
- [ ] CLI i18n: `tokenradar-i18n` crate'i, `.ftl` dosyalarını `include_str!` ile gömer. Dil config'den, yoksa sistem dilinden seçilir.
- [ ] CLI boyutu ≤ 3 MB. Bunun için minimal bağımlılık kullanılır: `tokio` kullanılmaz, IPC istemcisi senkron çalışır. HTTP ve Tauri bağımlılığı yoktur.

### Ölçüm ve test

- [ ] `scripts/measure-statusline.ps1`: 200 çağrı (örnek stdin ile) → p50/p95 değerleri. Ölçüm, daemon açıkken ve kapalıyken ayrı yapılır.
- [ ] Testler:
  - Protokol serde gidiş-dönüşü (round-trip)
  - Sürüm uyuşmazlığı
  - Hatalı mesaj
  - Spool yazma ve içe aktarma idempotensi
  - settings.json düzenleme: anahtar sırasının korunması, önceki komutun saklanması, uninstall ile geri dönüş
  - Biçimlendirici için EN/TR snapshot'ları ve dar genişlik
  - Zincirleme modunda zaman aşımı

## Teknik notlar ve tuzaklar

- Claude Code yeni bir güncelleme geldiğinde **çalışmakta olan statusline script'ini iptal eder**. CLI ne kadar hızlı olursa veri kaybı o kadar az olur. Bu yüzden push işlemi **ilk iş** olarak yapılır, biçimlendirme sonra gelir.
- GUI alt sistemli bir exe statusline için uygun değildir (konsol yok). Statusline için yalnızca `tokenradar.exe` (console) kullanılır.
- Windows'un varsayılan named pipe DACL'i *Everyone* ve *Anonymous* gruplarına okuma izni verir. Açık bir security descriptor şarttır.
- `settings.json` dosyasını Claude Code çalışırken değiştirmek güvenlidir; Claude Code ayarları yeniden yükler. Yine de atomik yazma zorunludur.
- Kullanıcının `statusline.py` dosyası usage API'sini 15 dakikalık cache ile çağırıyor. Zincirleme modda bu çağrılar devam eder. Kurulum diyaloğu (Adım 11) bu durumu fark edip kullanıcıya *"TokenRadar bu bilgiyi zaten sağlıyor, eski script'i zincirden çıkarabilirsiniz"* önerisinde bulunabilir.

## Kabul kriterleri

- [ ] Gerçek bir Claude Code oturumunda statusline TokenRadar satırını gösterir ve tray/uygulama yüzdeleri her yanıttan sonra güncellenir.
- [ ] `statusline` p95 ≤ 30 ms (daemon açık) ve ≤ 40 ms (daemon kapalı).
- [ ] Uygulama kapalıyken 10 yanıt üretilip uygulama açıldığında, spool'daki 10 gözlem snapshot tablosuna aktarılır.
- [ ] `install` → `uninstall` döngüsü sonrası `settings.json` dosyası, anahtar sırası dahil orijinaliyle **bayt düzeyinde aynıdır** (girinti farkı olursa belgelenir).
- [ ] Pipe yalnızca mevcut kullanıcı tarafından açılabilir.

## Doğrulama

```powershell
cargo test -p tokenradar-core ipc
cargo test -p tokenradar-cli
cargo build --release -p tokenradar-cli; (Get-Item .\target\release\tokenradar.exe).Length
Get-Content testdata\statusline\sample.json | .\target\release\tokenradar.exe statusline
.\scripts\measure-statusline.ps1
.\target\release\tokenradar.exe statusline install   # diff'i incele, onayla; sonra uninstall
```

## Devir notu

_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
