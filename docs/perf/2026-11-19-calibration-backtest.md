# Adım 13 — Kalibrasyon backtest'i ve yeniden hesaplama maliyeti

**Tarih:** 2026-11-19 · **Karar:** ADR-061 · **Makine:** geliştirme makinesi (Windows 11), release build

## Yöntem

- Veri: gerçek veritabanının `sqlite3 -readonly … ".backup"` ile alınmış kopyası, son 21 gün. Rapor aralık, olay ya da tahmin sayısı, `k`, maliyet ve yol içermez; yalnızca hata ölçütleri ve süreler.
- Komut: `cargo run --release -p tokenradar-core --example backtest -- --db <kopya>`.
- **Pencere içi walk-forward (kullanıcı kararı):** 30 dakikada bir, o ana kadarki veriden `k` **ve** `α` öğrenilir. Sonraki 6 sa ve 24 sa'in yerel maliyeti bu parametrelerle yüzdeye çevrilir ve aynı pencerenin gerçek birikimli maksimumuyla karşılaştırılır. Ufuk pencere sonunu geçerse tahmin atlanır.
- **Model:** `Δp ≈ k · Σ α_f · ΔC_f`. **H1:** tek `k`, `α = 1`.
- **Hata:** tahmin − gerçek, yüzde puanı. Sapma (bias) ortalama işaretli hatadır.
- Güven alt kümeleri, uygulamanın kullanacağı güvenle ayrılır: öğrenilen güven × (son maliyette ağırlığı bilinen ailelerin payı). "Kullanılan (≥ orta)" uygulamanın `k0` yerine kalibre `k`'yı seçeceği tahminlerdir.

## Sonuç

### `weekly_all` (statusline serisi)

| Ufuk · model · alt küme | MAE (puan) | p90 mutlak (puan) | sapma (puan) |
|---|---|---|---|
| 24 sa · **model · tümü** | **0,75** | 1,55 | **−0,46** |
| 24 sa · H1 · tümü | 1,58 | 2,83 | −0,66 |
| 24 sa · model · kullanılan (≥ orta) | 0,62 | 1,29 | −0,35 |
| 24 sa · model · güven yüksek | 0,41 | 0,91 | −0,10 |
| 24 sa · model · güven orta | 0,78 | 1,50 | −0,55 |
| 24 sa · model · güven düşük | 1,82 | 3,45 | −1,32 |
| 24 sa · model · çoğunlukla Opus | 0,49 | 0,98 | +0,10 |
| 24 sa · model · diğer aileler ≥ %30 | 0,97 | 2,04 | −0,93 |
| 6 sa · model · tümü | 0,41 | 0,85 | −0,19 |
| 6 sa · H1 · tümü | 0,84 | 2,09 | −0,35 |
| 6 sa · model · güven yüksek | 0,31 | 0,70 | −0,04 |

- Walk-forward'da öğrenilen `α_fable` aralığı: 1,87–2,64. Son tahmin: `α_fable` 2,30, `α_sonnet` 1,41, güven yüksek, MAD/k 0,04.
- Kapanan haftalık pencerenin son değerinde hata +0,6 puan.

### `weekly_scoped model:Fable` (API serisi)

| Ufuk · alt küme | MAE (puan) | p90 mutlak (puan) | sapma (puan) |
|---|---|---|---|
| 24 sa · **tümü** | **0,75** | 1,92 | **−0,02** |
| 24 sa · güven düşük (tümü) | 0,75 | 1,92 | −0,02 |
| 6 sa · tümü | 0,28 | 0,95 | −0,14 |

- Son tahmin: güven orta (0,43), MAD/k 0,26. Kapanan pencerenin son değerinde hata −0,3 puan.
- Kapsamlı limit yalnızca kendi ailesinin maliyetini sayar, `α` öğrenmez. Kapsamlı limite maliyet yedeği eklenmedi (kullanıcı kararı); bu sayı yalnızca doğruluk kaydıdır.

### Kabul ölçütü (MAE ≤ 1,5, |sapma| ≤ 0,5, 24 sa)

| Limit | MAE | sapma | Sonuç |
|---|---|---|---|
| `weekly_all` | 0,75 | −0,46 | Geçti (sapma sınıra yakın) |
| `weekly_scoped model:Fable` | 0,75 | −0,02 | Geçti |

- H1 24 sa'te MAE 1,58 ile ölçütü geçmez. Aile çarpanı hatayı yarıya indirir.
- Sapmanın büyük kısmı, ağırlığı henüz öğrenilmemiş bir ailenin yoğun kullanıldığı günlerden gelir (güven düşük: −1,32). Uygulama bu tahminlerde kalibre `k`'yı kullanmaz; kullandığı alt kümede sapma −0,35.

### Sentetik doğrulama (40 tohum)

- Bilinen `k` = 0,045, `α_fable` = 2,2, %10 adım gürültüsü, saatlerin %10'unda dış tüketim, statusline aşağı / API en yakın tam sayıya yuvarlama.
- En kötü `k` hatası %4,7; en kötü `α_fable` hatası %20; 40 koşuda 0 yanlış sapma alarmı; 40 koşunun tamamında güven yüksek.
- `k` ×1,5 sıçradığında sapma 20 koşuda 5–12 saatte algılandı.

## Yeniden hesaplama maliyeti

| Ölçüm | Değer |
|---|---|
| Bir `calibrate` çalışması (21 günlük geriye bakış, iki limit), gerçek DB kopyası | medyan 19,7 ms, en çok 24,6 ms (7 koşu, iki ayrı oturum) |

- Limit thread'inde, açılıştan 30 s sonra ve sonra 15 dakikada bir, **yalnızca yeni snapshot geldiyse** çalışır. Boştayken (oturum yok) hiç hesap yapmaz; yalnızca 15 dakikada bir uyanır. Boşta CPU bütçesi (10 dk ortalaması < %0,1) etkilenmez.
- Aktif kullanımda en kötü ihtimalle 15 dakikada ≈ 20 ms: ≈ %0,002 CPU.
- Açılışta yalnızca saklı son satırlar okunur (tek sorgu); tray ikonu yolunda değildir.
