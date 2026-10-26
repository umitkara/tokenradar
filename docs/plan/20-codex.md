# Adım 20 — Codex provider'ı

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.4 | 19 | 1-2 oturum | ⬜ |

## Amaç

`Provider` soyutlamasının gerçekten provider'dan bağımsız olduğunu ikinci bir sağlayıcıyla kanıtlamak: OpenAI Codex (CLI/TUI). Codex, limit yüzdelerini **yerel loglara doğrudan** yazdığı için bu provider'da API çağrısı, token veya kalibrasyon önkoşulu gerekmez. Kullanıcı Claude ve Codex limitlerini aynı tray'de ve panelde görebilmeli.

## Önce oku

- `docs/data-sources.md` S6
- `docs/architecture.md` §5 (indeksleme), §7 (limit modeli ve füzyon), §15 (Provider trait'i)
- `docs/decisions.md`: ADR-005, ADR-007, ADR-008
- Adım 19 devir notu (hesap modeli; Codex hesabı da bir `accounts` satırıdır)

## Gözlemlenen format (planlama oturumu, 2026-10-27, Codex CLI 0.142.4)

Bu bölümdeki bilgiler yalnızca **anahtar adları** incelenerek elde edildi; içerik okunmadı.

- **Konum:** `~/.codex/sessions/YYYY/MM/DD/rollout-<ISO ts>-<uuid>.jsonl` (bu makinede 272 dosya, 161 MB) ve `~/.codex/archived_sessions/rollout-*.jsonl` (düz dizin). `CODEX_HOME` ortam değişkeni kök dizini değiştirir.
- **Satır zarfı:** `{ "timestamp": "...", "type": "...", "payload": {...} }`.
  - `type` değerleri: `session_meta`, `turn_context`, `event_msg`, `response_item`.
  - `payload.type` (yalnızca `event_msg` ve `response_item` için): `token_count`, `task_started`, `task_complete`, `turn_aborted`, `user_message`, `agent_message`, `function_call`, `reasoning`…
- **`session_meta`** (dosya başına bir kez): `id`, `session_id`, `cwd`, `git`, `cli_version`, `originator` (`codex-tui`…), `source` (`cli`…), `model_provider`, `timestamp`. **`base_instructions` alanı okunmaz.**
- **`turn_context`** (tur başına): `model` (örneğin `gpt-5.5`), `effort`, `cwd`, `timezone`, `turn_id`…
- **`event_msg` / `token_count`:**
  ```jsonc
  "payload": {
    "type": "token_count",
    "info": {
      "total_token_usage": { "input_tokens", "cached_input_tokens", "output_tokens", "reasoning_output_tokens", "total_tokens" },
      "last_token_usage":  { /* aynı alanlar */ },
      "model_context_window": <int>
    },                                   // info null olabilir
    "rate_limits": {
      "limit_id": "codex", "limit_name": null,
      "primary":   { "used_percent": 64.0, "window_minutes": 300,   "resets_at": 1782853839 },
      "secondary": { "used_percent": 10.0, "window_minutes": 10080, "resets_at": 1783440639 },
      "credits": null, "individual_limit": null, "plan_type": "plus", "rate_limit_reached_type": null
    }
  }
  ```
- **Diğer dosyalar:**
  - `session_index.jsonl`: `{id, thread_name, updated_at}`. Oturum adı buradan alınır.
  - `state_5.sqlite`, `logs_2.sqlite` ve diğer SQLite dosyaları (WAL modunda, Codex açıkken kilitli olabilir). **Bu adımda okunmaz**; yalnızca keşif notu düşülür.
  - `auth.json` **hiçbir koşulda okunmaz**. Plan bilgisi `rate_limits.plan_type` alanından gelir.

## Kapsam

- `providers/codex/` modülü: keşif, `parse_line`, olaylardan kullanım ve limit gözlemi üretimi, oturum ve proje eşlemesi.
- `models` tablosuna OpenAI modelleri ve fiyatları; maliyet formülünün Codex varyantı.
- Canlı oturum sezgisi (Codex'te `sessions/*.json` benzeri bir pid dosyası yok).
- UI: provider seçici; tray'de birincil provider; limit satırlarında provider rozeti.
- i18n: Codex terimleri (EN/TR).

## Kapsam dışı

- Codex'in SQLite durum dosyalarını parse etmek. Ancak keşif notu yazılır; ileride gerekirse ADR ile karar verilir.
- Codex IDE eklentisi ve Codex Cloud görevleri: bu görevler yerel rollout dosyası üretmiyorsa izlenemez. Bu durum belgelenir.
- Codex için statusline köprüsü: Codex'te eşdeğer bir mekanizma varsa yalnızca keşif notu düşülür.
- Codex için kalibrasyon ve atıf: Bu adımda yalnızca **doğrudan** limit gösterimi ve kullanım istatistikleri yapılır. Atıf için ayrı bir adım (`20b`) gerekebilir; devir notunda değerlendirilir.

## Görevler

### Keşif ve doğrulama

- [ ] Codex'in güncel sürümünde formatı yeniden doğrula. Format değişmişse `data-sources.md` S6'yı güncelle.
- [ ] **Kümülatif mi, tur bazlı mı?** `total_token_usage` oturum boyunca kümülatif mi? `last_token_usage` son API çağrısının değeri mi? Ardışık iki `token_count` olayında `total[n] - total[n-1] == last[n]` eşitliğini kontrol et. Aynı değerle tekrar eden olaylar var mı (çift sayım riski)?
- [ ] **`cached_input_tokens`, `input_tokens` içinde mi?** OpenAI usage modelinde cached token'lar genellikle input'un **alt kümesidir**. Doğrula; maliyet formülü buna bağlıdır.
- [ ] **`reasoning_output_tokens`, `output_tokens` içinde mi?** Aynı soruyu bu alan için de yanıtla.
- [ ] **`rate_limits` hangi sıklıkta geliyor?** Her `token_count` olayında mı? `null` olabiliyor mu? `resets_at` saniye cinsinden Unix zamanı mı?
- [ ] `limit_id`, `limit_name` ve `individual_limit` alanları başka değerler alabiliyor mu (örneğin model bazlı ayrı limitler)? Alabiliyorsa `weekly_scoped` / `scope` alanına eşlenir.
- [ ] `archived_sessions` dosyalarının formatı aynı mı? Arşivlenen dosya `sessions` dizininden **taşınıyor** mu (inode/mtime değişimi)? Taşınıyorsa çift indekslemeyi önlemek için kimlik dosya yolu değil, `session_meta.id` olmalı.
- [ ] **Satır boyutu:** `function_call_output` ve `base_instructions` satırları çok büyük olabilir. `memchr` ön filtresi (`"token_count"`, `"session_meta"`, `"turn_context"`) satırların büyük çoğunluğunu parse etmeden atlamalıdır. Satır uzunluğu dağılımını ölç.

### Çekirdek

- [ ] **`CodexProvider: Provider`:**
  - `discover()`: `CODEX_HOME` → `~/.codex`. WSL'deki `~/.codex` dizinleri Adım 19 mekanizmasıyla önerilir.
  - `watch_roots()`: `sessions/` (özyinelemeli) ve `archived_sessions/`.
  - `parse_line()`: Yukarıdaki üç satır türünü işler; diğer satırlar atlanır.
- [ ] **Olay modeli:**
  - Her `token_count` olayı → `last_token_usage` değerinden bir `UsageEvent`. Tekilleştirme anahtarı: `xxh3-128(session_id + olay_timestamp + total_tokens)`.
  - Kümülatif toplam, doğrulama ve onarım için tutulur: eksik olay varsa fark `total` değerinden hesaplanır.
  - Model ve effort bilgisi en son `turn_context` satırından alınır (dosya başına durum).
- [ ] **Limit gözlemi:** `rate_limits` → iki `LimitObservation` üretilir:
  - `primary` (`window_minutes = 300`) → `session`; `secondary` (`10080`) → `weekly_all`.
  - Eşleme `window_minutes` değerine göre yapılır, alan adına göre değil. Bilinmeyen pencere süresi → `scope = "codex:<limit_id>:<dk>"` ile genel limit olarak eklenir ve uyarı loglanır.
  - `percent`: `used_percent` alanı ondalıklıdır; bu, Claude API'sinin tam sayı değerlerinden daha yüksek çözünürlük sağlar. `source = CodexLog`.
  - Yalnızca **en yeni** olaydan gelen gözlem "güncel" kabul edilir. Eski dosyaları indekslerken üretilen gözlemler `limit_snapshots` geçmişine yazılır ama güncel değeri ezmez (`observed_at` karşılaştırması).
  - `rate_limit_reached_type` dolu ise → limite takılma olayı (bildirim kuralı tetiklenir).
- [ ] **Oturum ve proje:** `session_meta.cwd` → proje (Claude ile aynı `projects` tablosu; provider sütunu sayesinde aynı klasör iki provider'da tek proje olarak görünür). Oturum adı `session_index.jsonl` → `thread_name` alanından alınır.
- [ ] **Canlı oturum sezgisi:** Son yazma ≤ 2 dk ve son olay `task_complete` / `turn_aborted` değilse oturum "busy" sayılır. Son yazma ≤ 30 dk ise "idle", aksi halde tamamlanmış. Bu bir **sezgidir**; UI'da Claude'un pid doğrulamalı canlı göstergesinden ayırt edilir.
  - İsteğe bağlı iyileştirme: `codex.exe` süreci var mı (`sysinfo`)? Varsa ve sezgi "tamamlanmış" diyorsa idle'a yükseltilir.
- [ ] **Polling:** Codex'te S2 benzeri bir API yok. Limit verisi dosya izleme olaylarıyla gelir; ek zamanlayıcı gerekmez. Codex hiç kullanılmıyorsa sıfır maliyet.

### Fiyatlandırma

- [ ] OpenAI model fiyatları (`gpt-5.5`, `gpt-5.5-codex` vb.) **resmî fiyat sayfasından** doğrulanır ve `pricing.toml` dosyasına kaynak URL'si ve tarihiyle eklenir. Bilinmeyen model → maliyet "bilinmiyor" olarak gösterilir, token'lar yine sayılır.
- [ ] Codex maliyet formülü, doğrulama sonuçlarına göre (cached input alt küme ise):
  `C = p_in(m)·(input − cached) + p_cached(m)·cached + p_out(m)·output`. Reasoning token'ları output içindeyse ayrıca eklenmez.
- [ ] Tutarlılık: Maliyet modeli Claude'dakiyle **aynı rolü** oynar (API karşılığı değer, ikincil metrik). UI'da provider'lar arası maliyet karşılaştırması yapılırken "farklı fiyat modelleri" notu gösterilir.

### UI ve i18n

- [ ] **Panel:** Üstte provider sekmeleri (`Claude · Codex`); yalnızca birden fazla provider etkinse görünür. Codex limit satırları aynı `LimitBar` bileşenini kullanır; kaynak rozeti "log" olur.
- [ ] **Tray:**
  - Ayar: birincil provider, veya "en kritik limit" (varsayılan; tüm provider ve hesaplar arasından).
  - Tooltip tüm provider'ları satır satır listeler.
  - İkon tasarımı değişmez. Gösterilen provider tooltip'in ilk satırında yazar.
- [ ] **Ana pencere:**
  - Genel Bakış'ta provider filtresi.
  - Oturumlar ve Projeler ekranlarında provider sütunu ve filtresi.
  - Pencereler ekranı provider bazındadır; pencere modelleri farklıdır.
- [ ] **Ayarlar → Sağlayıcılar:** Claude / Codex etkinleştirme, dizinler, bildirim kuralları (provider başına).
- [ ] **Onboarding:** `~/.codex` bulunursa "Codex kullanımınızı da izleyelim mi?" önerisi gösterilir. Varsayılan yanıt evet, çünkü kimlik bilgisi gerekmez.
- [ ] i18n: `provider-codex`, `source-codex-log`, plan adları (`plus`, `pro`…). Codex limitleri için de "5 saatlik" ve "haftalık" terimleri kullanılır (aynı kavramlar).

### Testler

- [ ] Fixture: `testdata/codex/` altında **sentetik** rollout dosyası. Gerçek dosyalardan yalnızca yapı alınır; içerik alanları (`message`, `function_call_output`, `base_instructions`) sahte metinle doldurulur.
- [ ] Kümülatif → delta dönüşümü, eksik olay onarımı, tekrar eden olay tekilleştirmesi.
- [ ] `window_minutes` eşlemesi (300, 10080, bilinmeyen değer).
- [ ] Eski dosya indekslemenin güncel limiti ezmediği senaryo.
- [ ] Arşive taşınan dosyanın çift sayılmaması.
- [ ] Canlı oturum sezgisi durum makinesi (sahte saat).
- [ ] **Regresyon:** Claude provider testlerinin tamamı değişmeden geçer. Çekirdekte `if provider == ...` dallanması **yoktur**; bu kod incelemesinde kontrol edilir.

## Teknik notlar ve tuzaklar

- **Gizlilik:** Rollout dosyaları kullanıcı mesajlarını, araç çıktılarını ve sistem talimatlarını içerir. Parser yalnızca `token_count`, `session_meta` (içerik alanları hariç) ve `turn_context` satırlarını seçici serde ile okur. Diğer satırlar `memchr` filtresinden geçmez, yani hiç deserialize edilmez.
- **`git` alanı:** `session_meta.git` repo URL'si içerebilir. Yalnızca branch alınır; URL saklanmaz.
- **Dakika cinsinden pencere:** Codex pencere süresini açıkça bildirir (`window_minutes`). Pencere başlangıcı `resets_at − window_minutes` ile hesaplanır. Bu, Claude'daki pencere hizalama belirsizliğinden (Adım 14) daha nettir.
- **Saat dilimi:** `turn_context.timezone` var, ama zaman damgaları UTC'dir. Gün sınırları her zaman sistemin yerel saat dilimiyle hesaplanır (Claude ile tutarlı).
- **Codex açıkken SQLite dosyaları kilitli:** Bu adımda SQLite okunmadığı için sorun yok. İleride okunursa `?mode=ro&immutable=0` ve `SQLITE_BUSY` geri çekilmesi gerekir.

## Kabul kriterleri

- [ ] Gerçek Codex verisiyle 5 saatlik ve haftalık yüzdeler, Codex TUI'nin `/status` çıktısıyla ±1 puan içinde eşleşir (3 farklı anda kontrol edilir; devir notuna yazılır).
- [ ] Token toplamları, en az 3 oturumda dosyanın son `total_token_usage` değeriyle birebir eşleşir.
- [ ] Panelde Claude ve Codex aynı anda doğru görünür. Tray "en kritik" modunda doğru provider'ı seçer.
- [ ] Codex kurulu değilken sıfır ek maliyet: dosya izleme yok, zamanlayıcı yok.
- [ ] İndeksleme performansı: 161 MB Codex verisi ≤ 15 sn (soğuk) içinde işlenir. Gerçek değer ölçülür ve `docs/perf/` altına kaydedilir.
- [ ] Çekirdekte provider'a özgü dallanma yoktur.

## Doğrulama

```powershell
cargo test -p tokenradar-core codex
cargo test --workspace
pnpm -C app test; pnpm -C app check
pnpm dev   # Panel → Codex sekmesi; codex /status ile karşılaştır
.\target\release\tokenradar.exe status --provider codex --json
```

## Devir notu

_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
