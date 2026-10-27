# Adım 07 — Statusline gecikmesi ve CLI boyutu ölçümü

**Tarih:** 2026-10-27 · **Sürüm:** 0.0.1 (`step/07-ipc-cli-statusline`) · **Karar:** ADR-029 · **Araç:** `scripts/measure-statusline.ps1`

## Özet

| Ölçüt | Sonuç | Bütçe |
|---|---|---|
| `tokenradar statusline` p95, uygulama açık (IPC) | **13,3 ms** (p50 11,6 ms, en fazla 19,7 ms) | ≤ 30 ms ✅ |
| `tokenradar statusline` p95, uygulama kapalı (spool) | **25,5 ms** (p50 21,0 ms, en fazla 32,2 ms) | ≤ 40 ms ✅ |
| `tokenradar.exe` release boyutu | **3.097.600 B = 2,95 MiB** | ≤ 3 MB ✅ (MiB olarak; ondalık 3.000.000 B'a göre %3 fazla) |
| Aynı derleme, `opt-level = "s"` (deney, uygulanmadı) | 2.147.840 B = 2,05 MiB | — |

## Makine ve yöntem

| Öğe | Değer |
|---|---|
| CPU | Intel Core i5-13420H (8 çekirdek / 12 thread) |
| Build | `cargo build --release -p tokenradar-cli` (fat LTO, `opt-level = 3`, strip) |
| Komut | `.\scripts\measure-statusline.ps1` (200 ölçülen çağrı + 10 ısınma, mod başına) |

- Süre, `Process.Start` çağrısından süreç çıkışına kadar ölçülür. Bu, Claude Code'un gördüğü süredir: süreç oluşturma, stdin, IPC veya spool, biçimlendirme ve çıkış dahil.
- CLI, `portable` işaretli geçici bir kopyadan çalışır. Spool ve config geçici dizinde kalır; kullanıcının TokenRadar verisine ve Claude Code ayarlarına dokunulmaz. Payload sentetiktir: taze `resets_at` değerleri ve geçici bir config dizini içinde transcript yolu.
- **Açık mod:** Gerçek uygulama yerine `examples/ipc_serve.rs` çalıştırıldı. Bu örnek çekirdeği (`CoreRuntime`, `CoreIpc` işleyicisi, limit motoru) yalnızca IPC ile başlatır. Her çağrı `hello` + `statusline.push` + `status.get` gönderir ve yanıtı bekler.
- **Kapalı mod:** Pipe yok; okuma spool'a yazılır. Betik, kapalı modda 200, açık modda 0 spool satırı saydı. Yani her mod gerçekten kendi yolundan geçti.

## Yorum

- Kapalı mod açık moddan ~10 ms yavaş. İstemci olmayan pipe'ı hemen `NotRunning` olarak görüyor, beklemiyor. Fark büyük olasılıkla spool dosyasını açma, kilitleme ve yazma maliyetinden geliyor (olası neden: Defender'ın gerçek zamanlı taraması). **Bu neden doğrulanmadı.**
- Ölçüm, Claude Code'un 300 ms'lik debounce'u ile karşılaştırıldığında büyük bir pay bırakıyor. Zincirleme modda önceki komutun süresi (≤ 300 ms) bu sayılara eklenir ve burada ölçülmedi.
- Boyut bütçesine kalan pay ~48 KiB. CLI çekirdeğin tamamına bağlı; LTO kullanılmayan `reqwest`/`notify`/sunucu kodunu atıyor, ama SQLite (salt-okunur rapor için) ve Fluent kalıyor. `opt-level = "s"` %30 küçültüyor. Bu profil kararı Adım 13'teki boyut ölçümüne veya kullanıcı kararına bırakıldı.

## Gerçek ortam doğrulaması (yalnızca davranış; kişisel değer yok)

- Claude Code 2.1.311, gerçek `~/.claude/settings.json`'a `--no-chain` ile kuruldu. Uygulama kapalıyken 8 okuma spool'a düştü. Uygulama açılınca spool içe aktarıldı ve dizin boşaldı. Sonraki okumalar IPC'den geldi: `statusline_last_seen` 1 s, spool boş, `status` çıktısı "Live data from the TokenRadar app".
- `uninstall` sonrası `settings.json`'un SHA-256 değeri kurulum öncesiyle aynıydı (`ace1f346…`).
