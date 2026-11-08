# Adım 12k — Statusline düzeni: gecikme ve CLI boyutu

**Tarih:** 2026-11-08 · **Dal:** `step/12k-statusline-layout-core` · **Karar:** ADR-051 · **Araç:** `scripts/measure-statusline.ps1`

## Özet

| Ölçüt | 12k, varsayılan config (iki koşu) | `main` (iki koşu, aynı oturum) | 12k, Split + `COLUMNS=120` (iki koşu) | Bütçe |
|---|---|---|---|---|
| p95, uygulama açık (IPC) | **12,6 / 12,8 ms** (p50 11,9 / 11,9) | 13,2 / 12,6 ms (p50 12,0 / 11,9) | 12,9 / 13,1 ms (p50 12,1 / 12,2) | ≤ 30 ms ✅ |
| p95, uygulama kapalı (spool) | **23,5 / 25,5 ms** (p50 19,3 / 20,4) | 24,4 / 26,9 ms (p50 20,6 / 20,5) | 27,5 / 24,1 ms (p50 21,2 / 19,3) | ≤ 40 ms ✅ |
| `tokenradar.exe` (`release-cli`) | **2.440.704 B = 2,33 MiB** | 2.411.520 B = 2,30 MiB | — | ≤ 3 MB ✅ |

**Sonuç:**
- Ayraç öğeleri, bölgeler, `flex` yerleşimi ve `unicode-width` ile ölçüm, gecikmede ölçülebilir bir fark yaratmıyor. Farklar koşular arasındaki gürültüden küçük ve yönleri tutarsız.
- Boyut 29.184 B (28,5 KiB, %1,2) arttı. Artışın çoğu `unicode-width`'in genişlik tabloları. `cjk` özelliği kapalı olduğu için belirsiz genişlik tablosu derlenmiyor.

## Makine ve yöntem

| Öğe | Değer |
|---|---|
| CPU | Intel Core i5-13420H (8 çekirdek / 12 thread) |
| Build | `cargo build --profile release-cli -p tokenradar-cli` (ADR-034) |
| Komut | `.\scripts\measure-statusline.ps1 -NoBuild [-Cli <main exe>] [-Config split.toml -Cwd <repo>]`; mod başına 200 ölçülen çağrı, 10 ısınma çağrısı |

- **A/B aynı oturumda koşuldu.** `main` (338e624) ayrı bir git worktree'de aynı profille derlendi. Sıra main/12k/main/12k idi; ardından Split koşuları geldi.
- **Varsayılan koşularda `COLUMNS` yoktu.** Böylece sonuçlar 12i ölçümüyle kıyaslanabilir. Genişlik bilinmediği için `fit()` atlanır; bu, 12i'deki yoldur.
- **Split koşuları** `COLUMNS=120` ve repo `cwd`'siyle yapıldı. Split şablonunun öğeleri kullanıldı: üç limit, `flex`, renkli model, cwd ve git dalı. Böylece her çağrı ölçümden, bölgelerden, boşluk dağıtımından geçti ve `.git/HEAD` dosyasını okudu.
  - Ölçülen satır 116 sütundu (`120 − 4`). Sağ bölge kenara oturdu: `5h 49% ⏱60% · 1h59m │ 7d 41.2% ⏱57.1% · 2d23h            Opus 5.5 │ TokenRadar │ git step/12k-statusline-layout-core`.
  - Aynı config'le `main`'in CLI'ı da koşuldu (tek koşu). `main`, `flex` türünü ve `version` anahtarını tanımıyor. Esnek okuma bilinmeyen öğeyi atladı ve satırı kendi otomatik ayracıyla çizdi. Bu, eski build uyumunun beklenen davranışı. Bu koşunun "açık" modunda p95 55,7 ms çıktı (p50 17,5). 12i'deki 68,9 ms gibi tekrarlanmayan bir yük olarak değerlendirildi; ölçülen dalın kodu değil.
- **Renk ölçüme dahil değil.** Betik `NO_COLOR=1` ile çalışıyor. ANSI kodları ve satır başındaki `ESC[0m` yalnızca span'lerin metne çevrilmesi.
- **Kurulu uygulama ölçüm boyunca kapatıldı** (kullanıcı onayıyla, yaklaşık 3 dk). "Kapalı" modda her koşuda 200, "açık" modda 0 spool satırı sayıldı.

## Göçün maliyeti

- `version`'sız bir tabloda ayraç göçü her okumada yapılır. Uygulama tabloyu bir kez yazınca `version = 2` olur ve göç bir daha çalışmaz.
- Varsayılan koşulardaki config'de `[statusline]` tablosu yoktu. Bu durumda satır `legacy()` ile türetilir; o da aynı göçten geçer. Fark ölçülemedi.
