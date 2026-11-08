# Adım 12n — Statusline compact öğesi: oturumdaki compact sayısı ve auto-compact limiti

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1.x | 12m | 1–2 oturum | ⬜ |

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

- [ ] Adım başı: ortam değişkeni override'larını belgelerden doğrula; geriye dönük doldurma yolunu ölç.
- [ ] Core: kayıt tipi, ön filtre dalı, seçici struct; anonim fixture ile testler.
- [ ] Store: migration, `compact_events`, oturum başına sayım sorgusu; testler.
- [ ] IPC: `CompactCount`, `SessionInfo` alanı, eski cevaplarla uyum testi.
- [ ] CLI: ayar hiyerarşisi okuyucusu (geçici dizinlerle testli).
- [ ] Render ve prefs: `compact` öğesi, iki biçim, `fit()`; testler.
- [ ] Düzenleyici, Fluent, örnek veri, bindings.
- [ ] Ölçüm: indeksleyici verimi ve statusline soğuk başlatması (`docs/perf/`).
- [ ] Ekran görüntüsü onayı, ADR ve devir notu.

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

_Adım bitince doldurulur._
