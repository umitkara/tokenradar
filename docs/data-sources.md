# Veri Kaynakları — Gözlemlenen Formatlar ve Tuzaklar

> Bu doküman 2026-10-26/27 tarihinde gerçek bir Windows 11 makinesinde (Claude Code **v2.1.310**, Max 20x plan)
> yapılan keşfe dayanır. Formatlar belgelenmemiş olabilir ve değişebilir. Her parser **toleranslı** olmalı:
> bilinmeyen alanları yok saymalı, eksik alanlarda çökmemeli, tanımadığı şemaya rastladığında bunu loglayıp sinyal üretmeli.
> Örneklerdeki kimlikler ve değerler temsilidir.

## Özet tablo

| # | Kaynak | Ne verir | Erişim | Güven | Rol |
|---|---|---|---|---|---|
| S1 | **Statusline köprüsü** | 5s ve haftalık % (**ondalıklı**), `resets_at`, `session_id`, model, maliyet, bağlam | Claude Code her yanıttan sonra komutumuzu çağırır (300 ms debounce) | Yüksek, **belgelenmiş** | **Birincil limit kaynağı** |
| S2 | **OAuth usage API** | Tüm limitler (`session`, `weekly_all`, `weekly_scoped`), yüzey kırılımı, extra usage | `GET /api/oauth/usage` + Claude Code OAuth token'ı (salt-okunur) | Yüksek ama **belgelenmemiş** | Tamamlayıcı, **açık onay gerekir** |
| S3 | **Transcript JSONL** | Mesaj bazlı token kullanımı, model, oturum, cwd, branch, zaman | `<config>/projects/**/*.jsonl` | Yüksek (ham veri) | Token, oturum ve proje analitiği |
| S4 | **Canlı oturum dosyaları** | Çalışan oturumlar: pid, durum (`busy`/`idle`), ad, cwd | `<config>/sessions/<pid>.json` | Orta (pid canlılığı doğrulanmalı) | Canlı oturum listesi |
| S5 | **Kimlik bilgisi meta verisi** | `subscriptionType`, `rateLimitTier`, `expiresAt` | `<config>/.credentials.json` (Windows/Linux), macOS'ta **Keychain** | Yüksek | Plan algılama, S2 için token |
| S6 | **Codex rollout JSONL** (v0.4) | `rate_limits.primary/secondary` (% + pencere + reset), token kullanımı | `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl` | Yüksek | Codex provider'ı |

`<config>` varsayılan olarak `~/.claude` dizinidir. `CLAUDE_CONFIG_DIR` ortam değişkeni bunu değiştirir (çoklu hesap senaryosu).

---

## S1 — Statusline köprüsü (birincil)

Kaynak: https://code.claude.com/docs/en/statusline

Claude Code, `settings.json` içindeki `statusLine.command` komutunu çalıştırır ve **stdin** üzerinden JSON gönderir. Bizim CLI'ımız (`tokenradar statusline`) bu JSON'u alır, daemon'a iletir ve bir statusline satırı basar.

**Ne zaman çağrılır:** Oturum başlangıcında, her yeni assistant mesajında, `/compact` sonrasında, izin modu değiştiğinde, `resets_at` zamanı geldiğinde, prompt cache süresi dolduğunda ve isteğe bağlı `refreshInterval` periyodunda. Güncellemeler **300 ms debounce** edilir. Yeni bir güncelleme geldiğinde **çalışmakta olan script iptal edilir**, bu yüzden CLI çok hızlı olmalı.

**Bizim için kritik alanlar:**

```jsonc
{
  "session_id": "a813d09f-…",
  "session_name": "tokenradar planning",        // olmayabilir
  "transcript_path": "C:\\Users\\…\\.claude\\projects\\…\\a813d09f-….jsonl",
  "cwd": "D:\\Projects\\…", "workspace": { "current_dir": "…", "project_dir": "…", "repo": { "host": "github.com", "owner": "…", "name": "…" } },
  "model": { "id": "claude-opus-5-5", "display_name": "Opus 5.5" },
  "version": "2.1.310",
  "cost": { "total_cost_usd": 4.12, "total_duration_ms": 1234567, "total_api_duration_ms": 345678 },
  "context_window": { "used_percentage": 41, "context_window_size": 200000, "current_usage": { … } },
  "fast_mode": false, "effort": { "level": "high" },
  "rate_limits": {                                 // YALNIZCA Pro/Max ve oturumdaki ilk API yanıtından sonra
    "five_hour": { "used_percentage": 23.5, "resets_at": 1738425600 },   // epoch saniye
    "seven_day": { "used_percentage": 41.2, "resets_at": 1738857600 }
    // "spend_limit": { … }  yalnızca Claude apps gateway arkasında
  },
  "prompt_cache": { … }
}
```

**Tuzaklar:**
- `rate_limits` ve içindeki her pencere **bağımsız olarak eksik** olabilir. Claude Code, `resets_at` geçtiğinde pencereyi JSON'dan düşürür.
- Statusline **model bazlı haftalık limiti (`weekly_scoped`) ve yüzey kırılımını vermez.** Bunlar için S2 gerekir.
- Değerler hesap genelidir. Eşzamanlı birden çok oturum varsa, bir oturumun gördüğü Δ% diğer oturumların tüketimini de içerir.
- Kullanıcının mevcut bir statusline komutu olabilir (örneğin `statusline.py`). Kurulum sırasında bu komut **zincirleme modunda** korunmalı (bkz. ADR-009).
- Çıktı genişliği için `COLUMNS` ortam değişkeni kullanılmalı. ANSI renkleri ve OSC 8 linkleri destekleniyor.

---

## S2 — OAuth usage API (tamamlayıcı, onaylı)

```
GET https://api.anthropic.com/api/oauth/usage
Authorization: Bearer <claudeAiOauth.accessToken>      # sk-ant-oat… ile başlar
anthropic-beta: oauth-2025-04-20
Content-Type: application/json
```

Yanıtta `ratelimit` veya `retry-after` başlığı gözlemlenmedi.

**Gözlemlenen yanıt (2026-10-26, kısaltılmış):**

```jsonc
{
  // ESKİ düz alanlar — hâlâ dolu olabilir ama BİRİNCİL kaynak olarak "limits" dizisi kullanılmalı
  "five_hour":  { "utilization": 5.0,  "resets_at": "2026-10-26T22:29:59.942902+00:00", "limit_dollars": null, … },
  "seven_day":  { "utilization": 66.0, "resets_at": "2026-10-28T07:59:59.942923+00:00", … },
  "seven_day_opus": null, "seven_day_sonnet": null, "seven_day_oauth_apps": null, "seven_day_cowork": null,

  // Kod adlı, anlamı bilinmeyen alanlar (iguana_necktie, amber_gauge, tangelo, …) → YOK SAY
  "iguana_necktie": { "utilization": 0.0, "limit_dollars": 250, "used_dollars": 0.0, … },
  "pace": null,

  // YENİ ve BİRİNCİL: genel limit dizisi
  "limits": [
    { "kind": "session",       "group": "session", "percent": 5,  "severity": "normal",
      "resets_at": "2026-10-26T22:29:59.942902+00:00", "scope": null, "is_active": false },
    { "kind": "weekly_all",    "group": "weekly",  "percent": 66, "severity": "normal",
      "resets_at": "2026-10-28T07:59:59.942923+00:00", "scope": null, "is_active": true },
    { "kind": "weekly_scoped", "group": "weekly",  "percent": 0,  "severity": "normal",
      "resets_at": "2026-10-28T08:00:00+00:00",
      "scope": { "model": { "id": null, "display_name": "Fable" }, "surface": null }, "is_active": false }
  ],

  // Haftalık pencerede yüzey bazlı tüketim payı — "Diğer" kovası için ground truth
  "seven_day_breakdown": {
    "as_of": "2026-10-26T20:58:31Z", "window_started_at": "2026-10-21T07:59:59Z",
    "rows": [ { "key": "claude_code", "display_name": "Claude Code", "percent": 99 },
              { "key": "chat", "percent": 0 }, { "key": "cowork", "percent": 1 }, { "key": "other", "percent": 0 } ]
  },
  "weekly_scoped_shares": [ { "limit_index": 2, "used_percent_of_weekly": 0, "allowance_percent_of_weekly": 50 } ],

  "extra_usage": { "is_enabled": false, "user_disabled": true, … },
  "spend": { "used": { "amount_minor": 0, "currency": "USD", "exponent": 2 }, "enabled": false, … },
  "member_dashboard_available": false
}
```

**Kurallar ve tuzaklar:**
- **Parse önceliği:** önce `limits[]` dizisi okunur. Dizi yoksa veya boşsa düz alanlara (`five_hour`, `seven_day`, `seven_day_*`) düşülür. İki şemayı da kapsayan fixture testleri zorunludur.
- `limits[].percent` **tam sayıdır** (1 puanlık kuantizasyon). Kalibrasyonda bunu hesaba katmak gerekir, bkz. [architecture.md §9](architecture.md#9-ağırlıklı-token-kalibrasyon-ve-atıf). S1'deki değerler ondalıklı olduğu için S1 tercih edilir.
- `weekly_scoped` kaydı bir `scope.model.display_name` taşır (örneğin "Fable"). Bu limite yalnızca ilgili model ailesinin tüketimi sayılır.
- `severity` alanı şimdilik `normal` olarak gözlemlendi. Diğer değerleri (warning/critical vb.) UI renk durumuna eşlenmeli, ama renk eşikleri **bizim** ayarlarımızdan gelir.
- `is_active` alanının anlamı net değil. Muhtemelen "şu an bağlayıcı olan limit" demek. Saklanır ama karar mantığında kullanılmaz.
- **Token yönetimi:**
  - Access token yaklaşık 1 saatte bir dolar. Claude Code aktifken kendisi refresh eder.
  - **Biz ASLA refresh etmeyiz.** Refresh token'lar rotating; biz refresh edersek Claude Code'un elindeki token geçersizleşebilir.
  - 401 alındığında token "bayat" olarak işaretlenir. `.credentials.json` dosyasının mtime'ı değişince (Claude Code yeni token yazınca) tekrar denenir.
- Token yalnızca istek anında bellekte okunur. Diske yazılmaz, loglanmaz, IPC'den geçmez. Kullanımdan sonra `zeroize` edilir.
- 429 → üstel geri çekilme (60 s → 30 dk). Polling politikası için [architecture.md §10](architecture.md#10-polling-politikası).

---

## S3 — Transcript JSONL (`<config>/projects/**/*.jsonl`)

**Dizin yapısı:**
```
projects/
  D--Projects-Araclar-aktif-TokenRadar/            # cwd'nin kayıplı kodlaması (":" "\" "/" → "-")
    <sessionId>.jsonl                            # ana oturum (dosya adı = sessionId)
    <sessionId>/subagents/agent-<id>.jsonl          # o oturumun subagent'ları (isSidechain: true)
    <sessionId>/tool-results/…                       # büyük araç çıktıları (indekslenmez)
```

**Ölçek (gözlem):** 5.191 dosya, **11,7 GB**, 1.502 proje klasörü, 2.754 subagent dosyası ve **~331 bin assistant satırı**. En eski dosya bir ay öncesine ait, çünkü Claude Code `cleanupPeriodDays` (varsayılan 30) ayarına göre eski dosyaları siliyor. Bu da **günde ~400 MB** yeni veri demek.

**Assistant kaydı (ilgili alanlar):**

```jsonc
{
  "type": "assistant",
  "uuid": "…", "parentUuid": "…",
  "sessionId": "a813d09f-…", "isSidechain": false,
  "timestamp": "2026-10-26T20:41:12.345Z",
  "cwd": "D:\\Projects\\Araclar\\aktif\\TokenRadar", "gitBranch": "main",
  "version": "2.1.310", "entrypoint": "cli",
  "requestId": "req_011CgRVxaMgGPT7pip6Kd3Jm",
  "requestedModel": "…", "effort": "…", "advisorModel": "…",
  "message": {
    "id": "msg_011CgRVxbLid8regWW9LR3vZ",
    "model": "claude-opus-5-5",
    "content": [ /* thinking / text / tool_use — İNDEKSLENMEZ, büyük */ ],
    "usage": {
      "input_tokens": 2,
      "cache_creation_input_tokens": 13986,
      "cache_read_input_tokens": 19701,
      "output_tokens": 1117,
      "output_tokens_details": { "thinking_tokens": 511 },
      "cache_creation": { "ephemeral_1h_input_tokens": 13986, "ephemeral_5m_input_tokens": 0 },
      "server_tool_use": { "web_search_requests": 0, "web_fetch_requests": 0 },
      "service_tier": "standard", "speed": "standard", "inference_geo": "global",
      "iterations": [ { "input_tokens": 2, "output_tokens": 1117, … , "type": "message" } ],
      "fallback_credit": null
    }
  }
}
```

**Kritik tuzaklar (test vakası olarak yazılmalı):**

1. **Tekrarlanan kayıtlar:** Aynı `(message.id, requestId)` çifti birden çok satırda görünür. Gözlemde satırların **%55-63'ü** tekrardı. Bunun nedeni, Claude Code'un her içerik bloğu için ayrı bir satır yazması.
2. **Tekrarlar her zaman aynı değil:** Bazı tekrarlarda `usage` **kısmidir** (streaming ara durumu, `iterations: []`). Tekilleştirme kuralı: anahtar başına **`output_tokens` değeri en büyük olan kayıt** kazanır. "İlk kaydı tut" yaklaşımı yanlıştır. Eşitlikte aynı oturum içinde son görülen satır kazanır. Farklı oturumlar arasında ise sıra bağımsız bir kural uygulanır: önce ana oturum, sonra küçük `uuid` (ADR-026).
3. **Satır boyutu:** Tek bir satır onlarca KB olabilir (thinking imzaları, araç çıktıları). Tam parse pahalı olduğu için önce `memchr` ile `"usage"` ön filtresi uygulanır, ardından yalnızca gereken alanlar seçici deserialize edilir (`serde` + `IgnoredAny`; `content` asla materyalize edilmez).
4. **Subagent'lar:** `subagents/agent-*.jsonl` dosyalarında `isSidechain: true` bulunur. Dosyanın üst dizini ana oturumun id'sidir. Oturum toplamları hesaplanırken **subagent tüketimi ana oturuma eklenir** ve ağaç yapısı korunur. Adım 04'te doğrulandı: subagent dosyalarındaki satırların %100'ünde `sessionId` üst dizin adına (ana oturuma), `agentId` ise dosya adındaki `<id>`'ye eşit.
5. **Proje kimliği:** Dizin adı (`D--Projects-…`) kayıplı bir kodlamadır. Proje kimliği olarak kayıtlardaki **`cwd`** kullanılır (normalize edilmiş, büyük-küçük harf duyarsız Windows yolu). Dizin adı yalnızca yedek kaynak.
6. **Yarım satırlar:** Dosyaya yazılmakta olan son satır yarım olabilir. Offset yalnızca `\n` ile biten tam satırların sonuna ilerletilir.
7. **Kesme ve silme:** Dosya küçülürse (truncate) veya silinirse offset sıfırlanır ve dosya yeniden işlenir. Tekilleştirme sayesinde bu idempotent kalır. Claude Code'un sildiği dosyaların agregasyonları **bizde kalır**.
8. **Fiyatlandırmayı etkileyen alanlar:** `speed` (fast mode, `"fast"` değeri olası), `service_tier`, `cache_creation.ephemeral_1h` ve `ephemeral_5m` ayrımı. Ağırlıklı token hesabında hepsi kullanılır.
9. **`iterations` ve advisor:** Kayıtların çoğunda `iterations` tek elemanlıdır veya hiç yoktur. Adım 04'te 13 satırda üç elemanlı dizi görüldü: `message` + `advisor_message` + `message`. Üst düzey `usage` yalnızca `type: "message"` elemanlarının toplamıdır. `advisor_message` elemanı kendi `model` alanını taşır (ör. advisor olarak çağrılan başka bir model) ve üst düzey toplama **dahil değildir**. İndeksleyici her advisor elemanını kendi modeliyle **ayrı bir olay** olarak kaydeder (anahtar: `MsgKey::from_ids_tagged(message.id, requestId, "advisor#<i>")`, ADR-026). Yalnızca üst düzey `usage` okunursa advisor tüketimi kaybolur.
10. **Usage taşıyan başka kayıtlar:** `attachment.attachment.usage` ve `user.toolUseResult.usage` alanları subagent özetleri taşır. Bunlar zaten subagent dosyasında sayılan tüketimin özetidir, **sayılmaz**. Ön filtre `"usage"` geçen satırda üst düzey `type` alanını kontrol eder; yalnızca `assistant` kabul edilir.
11. **Aynı anahtar birden fazla dosyada:** Anahtarların ~%0,16'sı birden fazla dosyada görüldü. Desenler: ana + ana (`--resume`), subagent + subagent ve ana + subagent (`fork-context-ref`, fork edilen bağlam subagent dosyasına kopyalanır). Çoğunda `output_tokens` eşittir; bu yüzden eşitlik kuralı sıra bağımsız olmalıdır (tuzak 2).
12. **Çok uzun satırlar:** `api-request-blob` satırları onlarca MB'a çıkabilir. 4 MiB'ı aşan satırlar belleğe alınmadan atlanır. Gerçek veride atlanan satırların tamamı `api-request-blob` idi. `{"type":"` ile başlamayan atlanmış satır "şüpheli" sayılır ve uyarı loglanır.

**Adım 04 keşif bulguları (gerçek veri, 5.192 dosya, 11,4 GiB, ~1,28 milyon satır):**

| Kayıt tipi | Satır payı | Not |
|---|---|---|
| `assistant` | %25,9 | Tek tüketim kaynağı (`message.usage`). |
| `attachment` | %25,9 | Bazılarında subagent özeti `usage` var; sayılmaz (tuzak 10). |
| `user` | %16,9 | `toolUseResult.usage` subagent özeti; sayılmaz. |
| `api-request-blob` | %8,4 | `usage` yok. En uzun satırlar bunlar (tuzak 12). |
| `api-request` | %5,0 | `usage` yok. Assistant kayıtlarıyla çift sayım riski yok. |
| `atis-latch`, `mode`, `queue-operation`, `permission-mode`, `system`, `cost-state`, `api-request-shape`, `bridge-session`, `file-history-*`, `pr-link`, `tools-used`, `continued-in`, `agent-color` | toplam ~%11 | Yok sayılır. |
| `last-prompt` | %4,2 | **İstem metni içerir; asla parse edilmez.** |
| `ai-title` | %2,2 | Başlık (kaynak `ai`). |
| `agent-name` | %0,18 | Başlık (kaynak `agent`). |
| `custom-title` | <%0,01 | Başlık (kaynak `custom`, `/rename`). |
| `fork-context-ref` | <%0,01 | Fork edilen bağlam; tuzak 11. |

- **Başlık önceliği:** `custom-title` > `agent-name` > `ai-title`. Aynı kaynaktan gelen başlıklarda en son görülen kazanır. Başlık kayıtları yalnızca ana oturum dosyalarında görüldü. `sessions/*.json` içindeki `name` alanı S4 kapsamındadır (Adım 05). Başlık için içerik okunmaz.
- **`speed`:** Kayıtların ~%82'sinde `"standard"`, kalanında alan yok. `"fast"` gözlemlenmedi; desteklenir ve `Speed::Fast` olarak eşlenir.
- **`inference_geo`:** `global` veya `not_available`. `us` gözlemlenmedi (ADR-025 "Bedel").
- **`requestId` eksikliği:** Çok az sayıda satırda (`< %0,01`) `requestId` yok. Anahtar `(message.id, "")` olarak üretilir. `message.id` eksik satır görülmedi; görülürse anahtar `uuid` hash'inden üretilir ve `fallback_keys` sayacı artar.
- **`cwd`:** Her dosyada en az bir satırda `cwd` var. Yine de `cwd`'siz dosyalar için dizin adından `unresolved:<dizin>` yedeği uygulanır.


---

## S4 — Canlı oturum dosyaları (`<config>/sessions/`)

```jsonc
// sessions/6728.json   (dosya adı = pid; yanında <pid>.<hash>.key dosyası bulunur — OKUNMAZ)
{
  "pid": 6728, "sessionId": "a813d09f-…", "cwd": "D:\\Projects\\…",
  "startedAt": 1793046978868, "updatedAt": 1793047433375,
  "version": "2.1.310", "kind": "interactive", "entrypoint": "cli",
  "name": "tokenradar-a6", "nameSource": "derived",
  "status": "busy",                      // busy | idle (gözlenen)
  "statusUpdatedAt": 1793047433375,
  "messagingSocketPath": "\\\\.\\pipe\\LOCAL\\cc-msg-…"   // KULLANILMAZ
}
```

- Süreç çöktüğünde dosya geride kalabilir. Bu yüzden **pid canlılığı** (ve mümkünse `procStart` eşleşmesi) doğrulanmalı.
- `.key` dosyaları ve `messagingSocketPath` Claude Code'un iç IPC mekanizmasıdır; **dokunulmaz**.

## S5 — Kimlik bilgisi meta verisi

`.credentials.json` → `claudeAiOauth`: `accessToken`, `refreshToken`, `expiresAt`, `refreshTokenExpiresAt`, `scopes`, `subscriptionType` (örneğin `max`), `subscriptionTypeSource`, `rateLimitTier` (örneğin `default_claude_max_20x`).

- Plan algılaması için yalnızca `subscriptionType` ve `rateLimitTier` okunur. Bunun için onay gerekmez, çünkü token'a dokunulmuyor.
- `accessToken` yalnızca S2 onayı varsa, istek anında okunur. `refreshToken` **hiçbir koşulda** okunmaz; parser bu alanı atlar.
- macOS'ta (v0.5) bu bilgiler Keychain'deki "Claude Code-credentials" kaydında tutulur.

## S6 — Codex (v0.4)

`~/.codex/sessions/YYYY/MM/DD/rollout-<ts>-<uuid>.jsonl` dosyalarındaki `token_count` olayları limit bilgisini **doğrudan** içerir:

```jsonc
"rate_limits": {
  "limit_id": "codex",
  "primary":   { "used_percent": 64.0, "window_minutes": 300,   "resets_at": 1782853839 },   // 5 saat
  "secondary": { "used_percent": 10.0, "window_minutes": 10080, "resets_at": 1783440639 },   // 7 gün
  "credits": null, "plan_type": "plus", "rate_limit_reached_type": null
}
```

Codex tarafında API çağrısı gerekmez, limit verisi yerel loglarda. `~/.codex/archived_sessions/` dizini ve `*.sqlite` dosyaları da var; Adım 20'de incelenecek.

Ek gözlemler (2026-10-27, Codex CLI 0.142.4, yalnızca anahtar adları incelendi):

- Satır zarfı `{timestamp, type, payload}` biçimindedir. İlgili satırlar: `session_meta` (dosya başına bir kez; `id`, `cwd`, `git`, `cli_version`, `originator`), `turn_context` (`model`, `effort`, `cwd`), `event_msg` + `payload.type = "token_count"` (`info.total_token_usage` / `info.last_token_usage`: `input_tokens`, `cached_input_tokens`, `output_tokens`, `reasoning_output_tokens`, `total_tokens`).
- Oturum adları `~/.codex/session_index.jsonl` dosyasındadır (`{id, thread_name, updated_at}`).
- `session_meta.base_instructions` alanı ve `response_item` satırları içerik taşır; **okunmaz**. `auth.json` **okunmaz**; plan bilgisi `rate_limits.plan_type` alanından gelir.
- Tam alan listesi ve açık sorular: [Adım 20](plan/20-codex.md).

---

## Kaynak füzyonu (özet)

Ayrıntılar [architecture.md §7](architecture.md#7-limit-modeli-ve-kaynak-füzyonu) içinde.

- Her limit gözlemi bir `LimitSnapshot { account, limit_kind, scope, percent, resets_at, source, observed_at }` olarak saklanır.
- **Güncel değer kuralı:** aynı limit için en yeni gözlem kazanır. Gözlemler 30 saniye içinde çakışırsa öncelik **S1 > S2 > tahmin** sırasıyla belirlenir.
- `weekly_scoped` limitleri ve yüzey kırılımı yalnızca S2'den gelir. S2 kapalıysa bu bilgiler UI'da "kullanılamıyor (API onayı gerekli)" olarak gösterilir.
