# Adım 14a — Atıfın gerçek veride incelenmesi

**Tarih:** 2026-11-20 · **Karar:** ADR-064 · **Makine:** geliştirme makinesi (Windows 11), release build

## Yöntem

- Veri: gerçek veritabanının `sqlite3 -readonly … ".backup"` ile alınmış kopyası. Rapor maliyet, token, oturum sayısı, oturum adı ve yol içermez. Yalnızca limit yüzdeleri, yöntemler ve süreler yazıldı.
- İnceleme: `cargo run --release -p tokenradar-core --example attr_inspect -- --db <kopya>`. Kopya salt okunur açılır. Kalibrasyon bellekte, uygulamanın şu anki kuralıyla hesaplanır.
- Ayrıntı: `--trace` her ölçülmüş pencerenin parçalarını ve ilk kenarlardaki birikimli `k·Σα·C` değerini basar (yuvarlama ofseti).
- Dilim karşılaştırması: `cargo run --release -p tokenradar-core --example attr_probe -- <kopya>`.
- Uçtan uca: kopyanın bir kopyası `Store::open` ile v3'e göç ettirildi, `fill_estimated` ve iki kez `refresh` çalıştırıldı (geçici örnek, depoya girmedi).

## Toplama düzeyi ("Diğer"in pencereye oranı)

| Limit | Aralık başına (plan) | 6 sa | 24 sa | Pencere |
|---|---|---|---|---|
| `weekly_all`, dış kullanım ≈ %0–1 olan hafta | %4,3 | %1,7 | %0,9 | en düşük |
| `weekly_scoped` Fable (API, güven 0,29) | %13–19 | — | — | — |

Aralık başına `min(Δp, k·ΔC)` gürültünün pozitif yarısını "Diğer"e topluyor. 24 saatlik dilim bu yanlılığın çoğunu kaldırıyor ve "Diğer"in hangi gün oluştuğunu koruyor.

## Pencereler (son hâl)

| Pencere | Yöntem | Güven | Diğer / toplam | `seven_day_breakdown` (claude_code dışı) | İşaret |
|---|---|---|---|---|---|
| `weekly_all`, kapanmış 1 (kayıt pencere ortasında başladı) | kalibre | 0,95 | %6,1 | %1,0 | — |
| `weekly_all`, kapanmış 2 | kalibre | 0,95 | %1,3 | %1,0 | — |
| `weekly_all`, açık | kalibre | 0,95 | %2,5 | %0,0 | — |
| `weekly_scoped` Fable, kapanmış 2 + açık | orantılı | 0,28 | — | — | — |
| `weekly_all`, tahmini 6 hafta | tahmini | 0,95 | — | — | — |

- **Korunum:** her pencerede `Σ pay + Diğer − toplam` < 10⁻⁴ puan.
- **Gelişim:** kuyruk ayrı ve ofset yokken "Diğer/toplam" %7,2 / %2,4 / %6,8 idi. Kuyruk son parçaya katılınca %7,1 / %2,3 / %5,1, baş parçasından 0,5 puan düşülünce yukarıdaki değerler.
- **Yuvarlama ofseti:** her ölçülmüş pencerede ilk kenarlar birikimli `k·Σα·C` ≈ 0,5, 1,5, 2,5 iken geliyor. İç parçalarda `Δp / k·Σα·ΔC` ≈ 1,0, baş parçasında 1,1–1,2.
- Kapanmış 1. haftada sohbet kullanımı hafta içinde ≈ %4'e kadar çıktı, kayıt da pencerenin ortasında başladı. Daha yüksek "Diğer" bununla uyumlu.
- Fable kapsamlı limitte kalibrasyon güveni orta altında, bu yüzden orantılı dağıtım ve `≈` geçerli.

## Süreler

| İş | Süre |
|---|---|
| Pencere başına `compute` | 0,6–4,4 ms |
| İlk `fill_estimated` + `refresh` (12 pencere, yazma dahil) | 38 ms |
| Değişiklik yokken `refresh` | 0,1 ms |

Atıf limit thread'inde çalışır, UI yolunda değildir. Sorgu API'lerinin ≤ 50 ms bütçesi ve sentetik büyük veriyle benchmark Adım 14b'de.

## Sentetik regresyon

`attribution::tests::other_catches_external_usage_within_two_points`: 8 tohum × dış pay %0 / 5 / 15 / 30, 5 dakikalık en yakına yuvarlanmış okumalar ve %4 gürültüyle "Diğer" dış payı ±2 puan içinde yakalıyor.
