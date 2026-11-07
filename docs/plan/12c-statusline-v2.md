# Adım 12c — Statusline v2: görünüm, ek bilgi, özelleştirme, tazelik

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1.x | 12b | 1-2 oturum | ⬜ |

## Amaç

`tokenradar statusline` çıktısını yeniden tasarlamak. Kullanıcının dört talebi var:
1. **Görünüm:** mini progress bar ve eşiğe göre renk.
2. **Ek bilgi:** context doluluğu, scoped (ör. Fable) haftalık limit, oturum payı, oturum maliyeti, model.
3. **Özelleştirme:** hangi segmentlerin hangi sırayla görüneceği; tek satır / iki satır.
4. **Doğruluk ve tazelik:** her değerin kaynağı ve yaşı belli olmalı. Bayat değer soluk gösterilir, tahmin `≈` ile işaretlenir.

5 saatlik penceresi olmayan hesap (doğrulanmış Ultra) için onaylanan varsayılan satır:

```
7d ████░░░░ 41% · 3g4s │ Fable 22% │ ctx 38% │ ≈2.1% bu oturum · $4.12
```

5 saatlik penceresi olan hesapta, aynı düzende `5h` segmenti en başa eklenir.

## Önce oku

- `docs/architecture.md` §11 (`tokenradar statusline` akışı, p95 < 30 ms)
- `docs/data-sources.md` S1 (payload alanları)
- `docs/performance-budget.md` (CLI soğuk başlatma ve statusline bütçeleri)
- `docs/design-language.md` (renk token'ları, `≈` kuralı, EN/TR sözlük)
- `docs/decisions.md`: ADR-029, ADR-041
- Adım 07 ve 12b'nin devir notları

## Kapsam

- **Ingest'e dokunulmaz.** `respond()` içindeki "önce push et / spool'a yaz, sonra formatla" sırası ve `StatuslinePush` aynen kalır. Değişiklik yalnızca `Line` modeli ve `render` katmanındadır.
- `Line` modeli genişler. Her segment değeri, kaynağını (`statusline` / `api` / `estimate`) ve gözlem zamanını taşır.
  - Payload'dan: `context_window.used_percentage`, `cost.total_cost_usd`, `model.display_name`.
  - Uygulamadan (IPC `status.get`): scoped limitler, oturum payı, projeksiyon.
- Segmentler: `five_hour`, `seven_day`, `scoped`, `context`, `session_share`, `cost`, `model`. Her biri bağımsız render edilir.
- Bar: 8 hücre, Unicode blok karakterleri. `NO_COLOR` ve dar terminalde bar düşer, yüzde kalır.
- Renk: mevcut `Severity` eşikleri. Eşikler ayarlardan gelir (renk kodu sabitlenmez).
- Tazelik: uygulamadan gelen değer belirli bir yaştan (öneri: 10 dk) eskiyse soluk gösterilir. Uygulama yoksa yalnızca payload segmentleri gösterilir.
- Özelleştirme: `config.toml` içinde `[statusline]` tablosu:
  - `segments = ["seven_day", "scoped", "context", "session_share", "cost"]`
  - `layout = "single" | "double"`
  - `bar = true`
  - Bilinmeyen segment adları yok sayılır. Config okunamazsa varsayılan kullanılır, statusline asla hata basmaz.
- Genişliğe uyum: mevcut `Detail` kademeleri segment önceliğiyle genelleştirilir. Önce düşük öncelikli segmentler düşer.
- Ayarlar penceresi: "Statusline bridge" sekmesinde segment seçimi ve canlı önizleme. Önizleme metni backend'den gelir (UI'da hesap yapılmaz).
- Fluent: yeni metinler `locales/{en,tr}/cli.ftl` ve `settings.ftl` içinde.

## Kapsam dışı

- Claude Code mod'u (Adım 12d).
- Kurulu v0.1.0'ı güncellemek veya sürüm çıkarmak. Veri haftası bitene kadar dal üzerinde kalır. Sürüm kararı kullanıcınındır.
- Statusline'a yeni bir veri kaynağı eklemek.

## Görevler

- [ ] `Line` → segment listesi modeli; kaynak ve yaş alanları.
- [ ] Segment render fonksiyonları ve testleri (renk açık/kapalı, dar/geniş, tek/iki satır).
- [ ] `[statusline]` config şeması, okuma ve varsayılanlar. Config testleri.
- [ ] Tazelik ve `≈` kuralları için testler: payload taze / uygulama bayat / uygulama yok.
- [ ] Ayarlar sekmesi: segment seçimi ve önizleme (tipli `tauri-specta` komutu).
- [ ] Fluent metinleri (`en`, `tr`), TR yazımı kontrol edilir.
- [ ] Performans: statusline p95 ölçümü, `docs/perf/` altına kayıt.

## Teknik notlar ve tuzaklar

- Claude Code, yeni güncelleme geldiğinde çalışan statusline komutunu iptal eder. Config okuma soğuk başlatma bütçesine sığmalı. Okumanın maliyeti ölçülür; gerekirse yalnızca `[statusline]` tablosu ayrıştırılır.
- Payload'daki değer her zaman uygulamanın değerinden tazedir. Aynı segment için iki kaynak varsa payload kazanır (v1 kuralı korunur).
- `cost.total_cost_usd` API karşılığı maliyettir, abonelik faturası değildir. Etiket bunu yanlış anlatmamalı (design-language sözlüğüne bak).
- Gerçek spool ve pipe'a dokunmamak için testler portable kopya ve geçici dizinlerle yapılır.

## Kabul kriterleri

- Varsayılan satır, 5 saatlik penceresi olmayan hesapta yukarıdaki örnekle aynı düzende çıkar.
- `segments` ve `layout` ayarları satırı beklenen şekilde değiştirir.
- Hiçbir tahmin `≈` olmadan gösterilmez. Bayat değerler soluk gösterilir.
- Statusline p95 < 30 ms (bütçe), ölçüm kayıtlı.
- Ingest davranışı değişmez: mevcut statusline push ve spool testleri değişmeden geçer.

## Doğrulama

- `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`
- `pnpm -C app check`, `pnpm -C app test`
- Portable kopyayla sentetik payload'lar üzerinden `tokenradar statusline` çıktısının görsel kontrolü.
- `scripts/` altındaki statusline ölçüm betiği ile p95.

## Devir notu
_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
