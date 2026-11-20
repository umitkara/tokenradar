# Adım 12t — Eski ve yeni hız yönteminin gerçek veride karşılaştırılması

**Tarih:** 2026-11-20 · **Karar:** ADR-062 · **Makine:** geliştirme makinesi (Windows 11), release build

## Yöntem

- Veri: gerçek veritabanının `sqlite3 -readonly … ".backup"` ile alınmış kopyası, tüm geçmiş. Rapor okuma, an ya da tahmin sayısı, yüzde, zaman ve yol içermez. Plan gereği sonuçlar biçim olarak ve yuvarlanmış oranlarla yazıldı.
- Komut: `cargo run --release -p tokenradar-core --example rate_compare -- --db <kopya>`.
- Her limitin her kaynağı (statusline, API) pencere pencere, saklandığı sırayla yeniden oynatılır. En fazla 15 dakikada bir iki hız, uygulamanın o anda bileceği gibi alınır:
  - **Eski:** aynı kaynağın son 30 dakikadaki okumalarının Theil–Sen eğimi.
  - **Yeni:** `rate::Steps` kenar hızı.
- **Gerçek:** o andan sonraki 3 saatte birikimli maksimumun yükselişi / 180 dakika. 3 saat sonrasına en fazla 30 dakika kala bir okuma yoksa an atlanır.
- **Oran:** tahmin / gerçek, yalnızca gerçek > 0 olan anlarda (kullanım olan anlara doğru seçilmiş; yeni yöntem geçmişe baktığı için oranı aşağı çeker).
- **Mutlak hata:** |tahmin − gerçek| × 180, puan; gerçeği olan her an, kullanım olmayanlar dahil. Hız yoksa 0 sayılır.
- **Gidip gelme:** ardışık anlarda `will_hit_before_reset` değişimleri, eski yönteme oranla.

## Sonuç

| Limit · kaynak | Oran: eski | Oran: yeni | Mutlak hata (medyan): eski → yeni | Gidip gelme: yeni / eski |
|---|---|---|---|---|
| `weekly_all` · statusline | ≈ 6× | ≈ 0,5× | ≈ yarıya iner | ≈ %5 |
| `weekly_all` · API | ≈ 6× | ≈ 0,6× | daha düşük | ≈ %2–10 |
| `weekly_scoped` Fable · API | ≈ 3,5× | ≈ 0,4× | eşit (kullanım çoğunlukla yok) | ≈ %10 |

- **Eski yöntem gerçeğin birkaç katı, yeni yöntem aynı mertebede ama altında.** Bulgudaki 2,5–3 kat hata gerçek veride de görülüyor, daha da büyük.
- **Durma tavanı:** tavansız kenar hızı oranı ≈ 0,7–0,85×. Tavan aktif anlarda hızı biraz daha düşürüyor, ama API serisinde kullanım olmayan anlar dahil mutlak hatayı belirgin azaltıyor (tavansız hâli eski yöntemden kötü).
- **Gidip gelme** eskisinin onda birinin altına iniyor; `≥ Medium` alt kümesinde API'de daha da az.
- **Kapsamlı limit:** seyrek API adımları yüzünden yeni yöntem eskisine göre daha az zaman hız veriyor. Eskiden görünen öngörü çoğunlukla birkaç kat yüksekti.

## Sentetik regresyon

`rate.rs` `whole_percent_polls_every_fifteen_minutes_give_a_steady_projection` ve motor testi `quarter_hour_api_polls_project_near_the_true_rate_without_flapping`: 5–19 dk aralıklı, en yakına yuvarlanmış API okumaları, saatte 1 puan. Dört kenardan sonra hız her anda gerçeğin %20'si içinde; öngörü bir kez açılıyor, kapanmıyor. Aynı tür okumalarda eski 30 dakikalık Theil–Sen gerçeğin 2,5 katı.

## Maliyet

Kenar hızı her snapshot'ta kaynak başına en fazla 4 kenar üzerinde sabit işlem; Theil–Sen artık tam sayı okumalarla beslenmediği için çağrı başına iş azaldı. Bütçeli bir yol değişmedi, ayrı ölçüm gerekmedi.
