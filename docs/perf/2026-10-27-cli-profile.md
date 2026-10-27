# Adım 11b — CLI boyut profili ve uygulama boyutu

**Tarih:** 2026-10-27 · **Sürüm:** 0.0.1 (`step/11b-settings-doctor`) · **Karar:** ADR-034 · **Araç:** `scripts/measure-statusline.ps1`

## Özet

| Ölçüt | Sonuç | Bütçe |
|---|---|---|
| `tokenradar.exe`, `--release` (`opt-level = 3`, doctor dahil) | 3.289.600 B = 3,14 MiB | ≤ 3 MB ❌ |
| `tokenradar.exe`, `--profile release-cli` (`opt-level = "s"`) | **2.273.280 B = 2,17 MiB** | ≤ 3 MB ✅ |
| Statusline p95, açık mod (IPC), `release-cli` | **17,3 / 16,0 ms** (iki koşu) | ≤ 30 ms ✅ |
| Statusline p95, kapalı mod (spool), `release-cli` | **30,9 / 27,0 ms** (iki koşu) | ≤ 40 ms ✅ |
| `tokenradar-app.exe`, `--release` (dialog + opener eklentileri dahil) | **12.623.360 B = 12,04 MiB** | ≤ 15 MB ✅ |

## Yöntem

- Makine ve ölçüm yöntemi Adım 07 ile aynı (`2026-10-27-statusline.md`): 200 ölçülen çağrı + 10 ısınma, `portable` işaretli geçici kopya, sentetik payload.
- Profiller sırayla iki kez ölçüldü (`-Profile release` ve `-Profile release-cli`, `-NoBuild`), böylece makinenin anlık durumu iki profile eşit dağıldı.
- `--release` ile karşılaştırma koşuları: IPC p95 14,7 / 16,4 ms, spool p95 30,4 / 27,5 ms.
- İlk spool koşusunda bir kez 112,7 ms p95 görüldü. Bu, `cargo build`'in hemen ardından gelen bir gürültüydü: yeni ikili dosyalar diskte henüz taranıyordu. Tekrarlanan koşularda görülmedi ve tabloya alınmadı.

## Yorum

- `opt-level = "s"` boyutu %31 küçültüyor. Gecikmede fark ölçüm gürültüsünün içinde: IPC'de iki profil 14–17 ms aralığında. CLI'nın süresinin büyük kısmı süreç oluşturma ve pipe/dosya açma; derleyici optimizasyonu bunları etkilemiyor.
- Uygulama `opt-level = 3` ile kalıyor. Dialog ve opener eklentileri dahil 12,04 MiB, yani bütçeye ~3 MB pay var.
- Adım 12: `tauri.conf.json`'daki `beforeBuildCommand` CLI'yı hâlâ `--release` ile derliyor. Paketleme `--profile release-cli` çıktısını (`target\release-cli\tokenradar.exe`) kullanmalı.
