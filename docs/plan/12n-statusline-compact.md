# Adım 12n — Statusline compact öğesi: oturumdaki compact sayısı ve auto-compact limiti

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1.x | 12m | 1–2 oturum | ✅ |

## Amaç

Bir oturumda kaç kez compact yapıldığını (otomatik ve elle ayrı ayrı) ve tanımlıysa auto-compact limitini statusline'da göstermek.

## Önce oku

- ADR-006, ADR-042, ADR-049, ADR-051
- `docs/data-sources.md` S3: "Compact sınırı" ve "Otomatik compact eşiği" maddeleri (gözlem 2026-11-08)
- Adım 12m'nin devir notu
- `crates/tokenradar-core/src/providers/claude/record.rs` (ön filtre, seçici serde)
- `crates/tokenradar-core/src/ipc/view.rs` (`StatusReply`, `SessionInfo`)
- `crates/tokenradar-cli/src/statusline.rs` (`deliver`: `StatusGet` turu)
- `docs/performance-budget.md` (statusline soğuk başlatma ve indeksleyici bütçeleri)

## Kullanıcıyla kararlaştırılanlar (2026-11-08)

- **İstenen iki değer:** oturumdaki compact sayısı ve tanımlıysa auto-compact limiti.
- **"Compact'a kalan" gibi bir değer yok.** Önerildi, kullanıcı reddetti.
- **Limit yalnızca tanımlıysa gösterilir.** Ayar yoksa öğenin limit kısmı görünmez. Varsayılan pencere tahmin edilmez.

## Kapsam

### 1. Veri: compact sayısı (indeksleyici ve store)

- **Ön filtre:** memchr ile `"compact_boundary"` aranır. Eşleşen satır, yalnızca şu alanları okuyan seçici bir struct'a ayrıştırılır: `uuid`, `sessionId`, `timestamp`, `isSidechain`, `compactMetadata.trigger`, `compactMetadata.preTokens`.
- **İçerik okunmaz.** Compact özetini taşıyan mesaj kaydı ayrıştırılmaz.
- **Store:** Migration ile bir `compact_events` tablosu eklenir.
  - Sütunlar: `uuid` (birincil anahtar), `session_id`, `at`, `trigger` (`auto` | `manual` | `other`), `pre_tokens`.
  - `uuid` anahtarı, aynı kaydın birden çok dosyada görünmesini (S3 tuzak 11, `--resume`) çift sayımdan korur.
- **Yalnızca ana oturum:** `isSidechain = true` olan kayıtlar sayılmaz.
- **Geriye dönük doldurma:** Mevcut indeks offset'leri bu satırları atlamış olabilir. Migration sonrasında tam yeniden indeksleme mi yoksa yalnızca compact için hafif bir tarama mı gerektiği adım başında ölçülür ve ADR'ye yazılır.

### 2. Veri: auto-compact limiti (CLI)

- **Okunan ayarlar:** `autoCompactEnabled` ve `autoCompactWindow`. Etkin değer şu sırayla birleştirilir: managed policy > `<cwd>/.claude/settings.local.json` > `<cwd>/.claude/settings.json` > kullanıcı `settings.json`.
- **Ortam değişkeni override'ları:** Adım başında Claude Code belgelerinden doğrulanır. Doğrulanmayan değişkene dayanılmaz.
- **Salt okunur:** Bu dosyalara yazılmaz. Seçici serde yalnızca bu iki anahtarı okur.
- **Kaynak etiketi:** Gösterilen değer config değeridir, `≈` almaz.
- **Tetiklenme noktası gösterilmez.** Gözlenen ≈%90 oranı belgelenmemiş bir iç ayrıntıdır.

### 3. IPC

- `SessionInfo` içine `compacts: Option<CompactCount { auto: u32, manual: u32 }>` eklenir, `#[serde(default)]` ile. Eski uygulamadan gelen cevapta alan yoksa sayı bilinmiyor demektir; 0 gösterilmez.
- **Yeni pipe turu yoktur.** Değer, statusline'ın her çağrıda zaten yaptığı `StatusGet` turunda döner.
- Uygulama çalışmıyorsa sayı yoktur ve öğenin sayı kısmı gizlenir. Limit kısmı CLI'da okunduğu için yine görünür.

### 4. Öğe: `compact`

| Biçim | Örnek | Not |
|---|---|---|
| `total` (varsayılan) | `⟲3 · 300k` | Toplam sayı ve tanımlıysa limit |
| `split` | `⟲2 oto 1 elle · 300k` | Otomatik ve elle ayrı |

- **Limit ekranı:** `autoCompactEnabled = false` ise limit yerine "oto kapalı" yazar. Limit tanımlı değilse limit kısmı çizilmez.
- **Sıfır:** Sayı 0 ise `⟲0` gösterilir; bu bilgi değerlidir. Sayı bilinmiyorsa sayı kısmı hiç çizilmez.
- **Gruplama:** Oturum grubundadır. Dar terminalde oturum öğeleriyle birlikte düşer; önce limit kısmı, sonra öğenin tamamı.
- **Renk:** Kullanıcı rengi. Eşik rengi yoktur.
- **Fluent (EN/TR):** Öğe adı, `oto`/`elle`, "oto kapalı".

### 5. Düzenleyici

- Palet, oturum grubunda `compact` öğesini gösterir. Öğe editöründe biçim seçici vardır.
- Örnek veri (`Line::sample_in`) sayıyı ve limiti üretir.

## Kapsam dışı

- Compact'a kalan token ya da tetiklenme noktası tahmini.
- Compact geçmişi grafiği, `preTokens` trendi (Adım 16'daki oturum ayrıntısı için aday).
- Tray ve panelde compact gösterimi (istenirse ayrı adım).
- Ayar dosyalarına yazmak.

## Görevler

- [x] Adım başı: ortam değişkeni override'larını belgelerden doğrula; geriye dönük doldurma yolunu ölç.
- [x] Core: kayıt tipi, ön filtre dalı, seçici struct; anonim fixture ile testler.
- [x] Store: migration, `compact_events`, oturum başına sayım sorgusu; testler.
- [x] IPC: `CompactCount`, `SessionInfo` alanı, eski cevaplarla uyum testi.
- [x] CLI: ayar hiyerarşisi okuyucusu (geçici dizinlerle testli).
- [x] Render ve prefs: `compact` öğesi, iki biçim, `fit()`; testler.
- [x] Düzenleyici, Fluent, örnek veri, bindings.
- [x] Ölçüm: indeksleyici verimi ve statusline soğuk başlatması (`docs/perf/`).
- [x] Ekran görüntüsü onayı, ADR ve devir notu.

## Teknik notlar ve tuzaklar

- **Fixture'lar sentetik olmalı** (`testdata/README.md`). Compact kaydı, gözlenen anahtar yapısıyla elle yazılır. Gerçek transcript satırı kopyalanmaz.
- **`trigger` bilinmeyen bir değer alırsa** `other` olarak saklanır ve toplama dahil edilir. Esnek okuma kuralı (ADR-042) bunu gerektirir.
- **Ön filtre maliyeti:** İkinci bir memchr deseni bütün satırlara uygulanır. İndeksleyici verim bütçesi yeniden ölçülür.
- **Soğuk başlatma:** CLI 2–4 küçük JSON dosyası daha okur. Dosya yoksa okuma hızlı başarısız olmalı. Ölçülür.
- **Ölçüm commit'leri** token sayısı, compact sayısı ya da yol içermez.

## Kabul kriterleri

- Sentetik transcript'te otomatik ve elle compact'lar doğru sayılıyor; `--resume` kopyası çift sayılmıyor, sidechain sayılmıyor.
- Statusline'da `⟲N · 300k` görünüyor. Ayar yoksa limit kısmı çizilmiyor, `autoCompactEnabled = false` ise "oto kapalı" yazıyor.
- Uygulama kapalıyken sayı kısmı gizleniyor, statusline bozulmuyor.
- Eski bir uygulamadan gelen cevapla CLI çalışıyor.
- Bütçe ölçümleri `docs/perf/` altında kayıtlı.

## Doğrulama

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
- `pnpm -C app check`, `pnpm -C app lint`, `pnpm -C app test`
- İndeksleyici ve statusline soğuk başlatma ölçümleri.
- Debug build ve portable mod ile CDP üzerinden EN ve TR ekran görüntüleri (`img/12n-*.png`).

## Devir notu

**Tarih:** 2026-11-08 · **Karar:** ADR-054 · **Ölçüm:** `docs/perf/2026-11-08-statusline-compact.md` · **Ekran görüntüleri:** `img/12n-statusline-en.png` (açık tema, EN), `img/12n-statusline-tr-dark.png` (koyu tema, TR); ikisinde de compact öğesi paletten eklenmiş ve "Oto ve elle" biçimi seçili

### Yapılanlar
- **Adım başı:** Ortam değişkenleri ve ayar sırası Claude Code belgelerinden doğrulandı (`DISABLE_AUTO_COMPACT`, `DISABLE_COMPACT`, `CLAUDE_CODE_AUTO_COMPACT_WINDOW`, managed `.d` dizini, `modelSettings`). Backfill yolu ölçüldü: ayrı compact taraması tam yeniden indekslemeden hızlı değildi. Kullanıcı tam yeniden indekslemeyi ve belgelenen dosya ve ortam kaynaklarını seçti.
- **Core, kayıt:** `ParseOutcome::Compact(CompactRecord)`, `"compact_boundary"` ön filtresi, seçici `CompactLine`/`CompactMetadata`. Kayıt yalnızca üst düzeyde `type == "system"` ve `subtype == "compact_boundary"` ise kabul edilir.
- **Domain:** `CompactTrigger` (`auto` | `manual` | `other`) ve `CompactEvent`.
- **İndeksleyici:** `RawCompact` ve `Chunk.compacts`. Sidechain kayıtları ve subagent dosyaları düşer. `uuid` yoksa satırın xxh3-128 hash'i kullanılır.
- **Store, şema v2:** `0002_compact_events.sql`: `compact_events`, `ix_compact_session`, `ix_sessions_uuid` ve canlı dosyaların okuma konumunu sıfırlama. `insert_compacts` (`ON CONFLICT DO NOTHING`), `compact_counts` ve `Store::compact_counts`.
- **IPC:** `CompactCount { auto, manual, other }`, `SessionInfo.compacts` (`serde(default)`) ve `SessionInfo::compacts_only`. `CoreIpc` artık `Store` taşıyor; `add_compacts`, `StatusGet` cevabına sayıyı ekliyor.
- **Statusline:** `auto_compact.rs` (`AutoCompact`, `Locations`, `resolve`, `model_matches`, `parse_env_window`). `SegmentKind::Compact`, `Format::Total`/`Split`, `Line.compacts`/`auto_compact`. Örnek veride `⟲3 · 300k` var. `render.rs`: compact arm'ı; `fit()` önce pencere kısmını, sonra öğeyi düşürüyor.
- **CLI:** `CompactSources`, `config_dir_of`. Ayar dosyaları yalnızca öğe satırdaysa okunuyor.
- **Fluent (EN/TR):** `sl-compact-*`, `statusline-kind-compact(-hint)`, `statusline-format-total`/`split`.
- **Uygulama:** Bindings yenilendi; `view.test.ts` listeleri güncellendi.
- **Testler:**
  - Kayıt (2), store (2: trigger'a göre sayım ve kopya, v1'den yükseltme), indeksleyici (1: resume kopyası, sidechain, subagent, idempotent yeniden okuma), IPC uyumu (1), bridge (`compact_tests`), `auto_compact` (8), render (`tests/v12n.rs`, 5), CLI (2).
  - Çekirdek 347 → 367 test.
- **Ölçüm araçları:** `examples/compact_scan.rs`. `scripts/measure-statusline.ps1` sentetik bir compact sınırı ve `autoCompactWindow` içeren `settings.json` yazıyor.
- **Dokümanlar:** ADR-054, data-sources S3 (compact sahipliği, ayar kaynakları ve göremediklerimiz), architecture §6 (şema v2), perf kaydı ve bütçe satırları, design-language sözlüğü, CHANGELOG (yeni öğe ve şema v2 notu).

### Sapmalar
- **Sütunun adı `session_uuid`,** plandaki gibi `session_id` değil. Sütun, kaydın kendi `sessionId`'sini tutuyor. Dosya oturumu seçilseydi `--resume` kopyalarında sahiplik, worker'ların hangi dosyayı önce bitirdiğine bağlı olurdu (ADR-054).
- **`CompactCount`'ta `other` alanı var.** Plan yalnızca `auto` ve `manual` öngörmüştü. Bilinmeyen tetikleyiciler toplama giriyor ve `split` biçiminde ayrı gösteriliyor.
- **`modelSettings.<model>.autoCompactWindow` da okunuyor** (belgede var). Ortam değişkenleri ayar dosyalarının `env` bloklarından değil, süreç ortamından okunuyor.
- **`ix_sessions_uuid` indeksi eklendi.** Sayım sorgusu oturumu `uuid` ile buluyor.
- **Ölçüm sırasında bir komut hatası:** `Set-Content` konumsal parametrelerle çağrıldı ve kurulu `tokenradar-app.exe`'ye yazmaya çalıştı. Dosya kullanımda olduğu için Windows reddetti. Exe'nin boyutu, zamanı ve sürümü doğrulandı (değişmedi). Ölçüm betiğinde dosyalar artık `[IO.File]::WriteAllText` ile yazılıyor.
- **Kurulu uygulama iki kez kapatıldı** (statusline ölçümü ve ekran görüntüleri), ikisinde de yeniden başlatıldı. Gerçek DB hiç açılmadı: indeksleyici geçici DB'lere, statusline betiği portable kopyaya, ekran görüntüsü için debug build `target\debug` altında portable modda çalıştı.
- **Önceki bir oturumdan kalan `vite` dev sunucusu** (port 1420) dev build'in açılmasını engelledi; durduruldu.

### Doğrulama
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` temiz; `cargo test --workspace` 477 test (çekirdek 367).
- `pnpm -C app check`, `lint` ve `test` temiz (120 test).
- İndeksleyici: tam indeksleme 14,8–15,1 s (main 14,5–15,2 s), RecentReady 0,62–0,64 s, tepe private 51–58 MiB. Şema yükseltmesi 14,5 s; parmak izi tam indekslemeyle aynı. DB +192 KiB.
- Statusline: açık p95 13,5–13,6 ms (main 13,4–13,6), kapalı 26,5–27,0 ms (main 24,6–25,5). Compact öğesiyle açık 13,5–17,0 ms, kapalı 26,4–28,6 ms; bir koşuda tekrarlanmayan 31,7 / 75,2 ms çıktı (sistem yükü, p50 değişmedi). CLI 2.481.664 B (+22.528 B). Hepsi bütçede.
- Eski CLI yeni sunucuyla, yeni CLI eski cevapla çalışıyor (A/B koşusu ve `serde(default)` testi).
- Debug build, portable mod ve `TOKENRADAR_FAKE_LIMITS=panel:no-five-hour` ile CDP üzerinden compact öğesi paletten eklendi ve "Oto ve elle" seçildi. Önizleme EN'de `⟲2 auto 1 manual · 300k`, TR'de `⟲2 oto 1 elle · 300k` çizdi. Ekran görüntüleri kullanıcıya gösterildi ve onaylandı.

### Açık konular
- **Şema v2 sürüm notu:** Yeni sürüm DB'yi açınca v0.1.0 o DB'yi açamaz; geri dönmek için DB silinmeli (CHANGELOG'da yazılı). Release notlarında da öne çıkarılmalı.
- **Dev build'ler gerçek veriye karşı çalıştırılmamalı:** Bir kez açılırsa kurulu v0.1.0'ın DB'si yükselir. Portable mod ya da geçici DB kullanılmalı.
- **Gerçek satır yeni sürüme kadar değişmez.** Kurulu v0.1.0 CLI `compact` öğesini atlıyor. Eski bir build config'i yeniden yazarsa öğe kaybolur (12i, 12k, 12m ile aynı).
- **Göremediğimiz ayar kaynakları:** kayıt defteri ve sunucu tarafı managed policy'ler, `--autocompact` ve `--settings` bayrakları, Bedrock/Vertex model id'leriyle `modelSettings` eşleşmesi.
- **Managed dizini yalnızca Windows'ta** (`%ProgramFiles%\ClaudeCode`); macOS ve Linux yolları Adım 21'de eklenmeli.
- **Ekran görüntüsünde görülenler** (kullanıcı olduğu gibi onayladı):
  - Paletten eklenen öğe seçili öğenin arkasına girdi; satır 1 bu yüzden 120 sütuna sığmayıp yatay kaydı.
  - Compact metni Düz temada "Otomatik" renkte (beyaz).
  - Dil değişince geri alma geçmişi sıfırlanıyor.
- **Portable dev verisi** (`target\debug\data\config.toml`) çekim sırasında EN, açık tema ve compact öğesiyle kaldı; zararsız.
- **12i, 12l ve 12m'den kalanlar sürüyor.**

### Sonraki adıma notlar
- Store'a bağlı yeni bir statusline değeri eklenecekse izlenecek yol: `SessionInfo`'ya `serde(default)` alanı, değeri `bridge.rs`'teki `add_compacts` gibi `StatusGet` cevabına ekleyen bir adım, `Line`'da `Option` alan. Böylece eski uygulama ve eski CLI uyumu korunur.
- Yeni bir migration okuma konumlarını sıfırlıyorsa yükseltme süresi ölçülmeli ve CHANGELOG'a yazılmalı.
