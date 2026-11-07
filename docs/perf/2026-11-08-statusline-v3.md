# Adım 12i — Statusline v3 gecikmesi ve CLI boyutu

**Tarih:** 2026-11-08 · **Dal:** `step/12i-statusline-v3` · **Karar:** ADR-049 · **Araç:** `scripts/measure-statusline.ps1`

## Özet

| Ölçüt | 12i, varsayılan config (iki koşu) | `main` (iki koşu, aynı oturum) | 12i, Detailed + git dalı | Bütçe |
|---|---|---|---|---|
| p95, uygulama açık (IPC) | **15,0 / 16,2 ms** (p50 13,4 / 13,5) | 14,9 / 13,8 ms (p50 13,0 / 12,8) | 14,4 / 16,6 / 14,9 ms; bir koşuda 68,9 (aşağıya bakın) | ≤ 30 ms ✅ |
| p95, uygulama kapalı (spool) | **29,1 / 29,9 ms** (p50 25,6 / 25,2) | 29,5 / 28,7 ms (p50 25,6 / 24,3) | 29,1 / 30,5 ms | ≤ 40 ms ✅ |
| `tokenradar.exe` (`release-cli`) | **2.411.520 B = 2,30 MiB** | 2.346.496 B = 2,24 MiB | — | ≤ 3 MB ✅ |

**Sonuç:**
- Öğe modeli, yeni segmentler ve `[[statusline.items]]` okuması bütçeyi etkilemiyor.
- IPC modunda p50 yaklaşık 0,5 ms yüksek. Fark koşular arasındaki gürültüyle aynı büyüklükte, ama iki koşuda da aynı yönde.
- Boyut 63,5 KiB arttı. Artışın çoğu yeni biçimlendiriciler, TOML yazıcısı ve şablonlar.

## Makine ve yöntem

| Öğe | Değer |
|---|---|
| CPU | Intel Core i5-13420H (8 çekirdek / 12 thread) |
| Build | `cargo build --profile release-cli -p tokenradar-cli` (ADR-034) |
| Komut | `.\scripts\measure-statusline.ps1 -NoBuild [-Cli <main exe>] [-Config detailed.toml -Cwd <repo>]`; mod başına 200 ölçülen çağrı, 10 ısınma çağrısı |

- **A/B aynı oturumda koşuldu.** `main` (e3f7ec3) ayrı bir git worktree'de aynı profille derlendi. Sıra main/12i/main/12i idi; ardından Detailed koşuları geldi.
- **Betiğe iki parametre eklendi:**
  - `-Config`: portable kopyaya bir `config.toml` koyar.
  - `-Cwd`: payload'daki `cwd` alanını değiştirir.
- **Detailed koşusu** Detailed şablonunun öğeleriyle yapıldı: iki satır, renkli model/cwd/git dalı, satır değişimi, süre. `cwd` bu repoydu, böylece her çağrı `.git/HEAD` dosyasını okudu. Ölçülen ikinci satır: `Opus 5.5 │ TokenRadar │ git step/12i-statusline-v3`.
- **Renk ölçüme dahil değil.** Betik `NO_COLOR=1` ile çalışıyor. ANSI kodları ve Powerline blokları yalnızca span'lerin metne çevrilmesi; maliyetleri ihmal edilebilir.
- **Kurulu uygulama ölçüm boyunca kapatıldı** (kullanıcı onayıyla, yaklaşık 3 dk). Portable kopya da kullanıcının gerçek pipe'ına bağlanır (ADR-042 ölçümündeki gibi). "Kapalı" modda her koşuda 200, "açık" modda 0 spool satırı sayıldı.

## Detailed koşusundaki 68,9 ms

- İkinci Detailed koşusunun "açık" modunda p95 68,9 ms, en kötü çağrı 88,9 ms çıktı. Aynı koşunun p50'si 13,5 ms'ydi.
- Aynı yapılandırmayla iki tekrar koşusu 16,6 ve 14,9 ms verdi. Aralarına konan varsayılan koşular 15,1 ve 14,6 ms verdi.
- Bu yüzden değer, makinedeki geçici bir yük olarak değerlendirildi; kod yolundan kaynaklanmıyor. Tekrarlanmadığı için ayrıca incelenmedi.
- Bu koşuda satırda `▲4s` göründü. Ölçüm sunucusu 200 çağrıda 10'dan 89'a çıkan sentetik okumalardan dakikalar içinde dolacak bir projeksiyon hesaplıyor. Bu ölçümün yapay bir sonucu.

## Config okumanın maliyeti

- Öğe dizisi aynı `config.toml` ayrıştırmasında okunuyor; ayrı bir dosya okuması yok.
- Detailed şablonundaki 12 öğe, varsayılan config'e göre ölçülebilir bir fark yaratmadı.
