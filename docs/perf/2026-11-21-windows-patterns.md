# 2026-11-21 — Pencereler ve Örüntüler (Adım 17a)

## Ne ölçüldü

- Yeni sorguların gecikmesi (`benches/queries.rs`), 1 yıllık sentetik veride. Bütçe: p95 ≤ 50 ms.
- `main.html` paket boyutu (`scripts/check-bundle-size.mjs`).

## Yöntem

- `cargo bench -p tokenradar-core --bench queries -- --quick`. Her sorgu önce 40 kez çalışır, p50 ve p95 yazılır.
- İki veritabanı ölçüldü:
  - **Yeni:** sentetik üretici artık 5 saatlik pencere de üretiyor (en yoğun beşte biri %100'e ulaşacak biçimde). `TR_SYNTH_DB=%TEMP%\tr-synth-17a.db`.
  - **Eski:** Adım 14'ün veritabanı, 5 saatlik pencere yok. 5 saatlik sorgular orada boş döner.
- Aynı makinede, Vite sunucusu kapalıyken iki veritabanı arka arkaya ölçüldü. Kurulu uygulama ve başka araçlar açıktı.
- Bu nota yalnızca süreler yazıldı.

## Sonuçlar

Yeni sorgular, yeni veritabanı (son koşu; ms):

| Sorgu | p50 | p95 |
|---|---|---|
| `list_weeks/12w` | 15,0 | 21,3 |
| `list_weeks/year` | 40,2 | 46,6 |
| `window_strip/12w` | 9,1 | 10,7 |
| `window_strip/year` | 7,8 | 9,8 |
| `get_window/five_hour` (katkılar ve bildirimler dahil) | 6,5 | 7,3 |
| `get_window` (haftalık) | 6,8 | 8,6 |
| `limit_risk/year` | 0,13 | 0,17 |
| `model_mix_trend/year` | 18,0 | 19,7 |
| `get_heatmap/percent_year` (saat dağılımı dahil) | 19,5 | 21,4 |
| `export/daily_csv_year` | 31,9 | 34,5 |

- `12w` satırları ilk koşudan; son koşuda yalnızca yıllık durumlar vardı.
- `main.html`: JS 124,8 KB / 180 KB (gzip), CSS 15,4 KB.

## Yorum

- **Şerit ilk sürümde bütçeyi aştı:** `window_strip/year` p95 66,9 ms. Her pencere için satır ve ilk üç oturum ayrı sorgulardı (yılda ~2.000 ifade). Satırlar tek sorguya, ilk üç oturum `ROW_NUMBER() OVER (PARTITION BY window_id …)` ile tek sorguya indi: 9,8 ms.
- **`list_weeks/year` sınıra yakın** (≈ 47 ms). Varsayılan aralık 12 haftadır (≈ 21 ms). "Tümü" bir yıllık geçmişte bu süreyi verir. Pencere başına üç küçük sorgu (projeler, kapsamlı tepe, kırılım) toplu sorguya çevrilebilir; gerekirse 17b'de yapılır.
- **Değişmeyen sorgular gürültülü ölçüldü:** `list_sessions/*` ve `list_projects/*` üç koşuda p95 35–93 ms arasında oynadı.
  - Aynı sorgular eski veritabanında da aynı biçimde bütçeyi aştı (`list_sessions/cost` 84,6 ms, `list_projects/cost` 82,7 ms). Aynı koşuda `list_sessions/last_active` 41 ms idi.
  - Yani bu adımın verisi ya da kodu değil, makine yükü. Adım 16b'nin notu da aynı gürültüyü kaydetmişti (`list_sessions/cost` bir koşuda 81,7 ms).
  - v0.2 performans raporu (17b) bu sorguları sessiz bir makinede yeniden ölçmelidir.
- **Gerçek veri koşusu:** Release derlemesinin portable kopyası, gerçek verinin kopyasıyla açıldı. Göçler ve tam indeksleme ≈ 20 s sürdü. Son üç haftanın gün değerleri bağımsız bir hesapla birebir eşleşti (devir notu). Süre ölçülmedi.
- Bellek ölçümü (`measure-memory.ps1 -Scenario main-window`) bu notta yok; v0.2 raporuna (17b) kaldı.
