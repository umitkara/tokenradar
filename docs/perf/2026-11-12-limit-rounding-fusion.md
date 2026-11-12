# Adım 12s — Limit yuvarlama füzyonu: motor açılışı

**Tarih:** 2026-11-12 · **Karar:** ADR-060 · **Makine:** geliştirme makinesi (Windows 11), release build

## Ne değişti

`LimitEngine::new` hız geçmişi için son 30 dakikanın snapshot'larını okuyordu. Birleşik değer için artık mevcut pencerenin tüm okumalarını gerektiriyor: tek bir `snapshots_between(now − 1 hafta, now)` okuması ve her limit için katlama. Motor `Runtime::start` içinde senkron kurulur, yani uygulama başlangıcının (tray ikonu ≤ 300 ms) yolunda.

## Ölçüm

Geçici bir örnek program (commit'lenmedi) `LimitEngine::new`'ü 21 kez çalıştırdı. Gerçek veritabanının `.backup` ile alınmış bir kopyası kullanıldı; satır sayısı ve yüzdeler yazılmaz.

| Sürüm | min | medyan | max |
|---|---|---|---|
| `main` (30 dk okuma) | 1,88 ms | 1,97 ms | 2,58 ms |
| 12s (1 haftalık okuma + katlama) | 3,26 ms | 3,39 ms | 5,00 ms |

Fark medyanda ≈ 1,4 ms, başlangıç bütçesinin (300 ms) yanında önemsiz. Veri büyüdükçe okuma bir haftayla sınırlı kalır (en uzun pencere).

## Bütçeye dokunmayan yollar

- Statusline satırı (`line::window` → `lifted`): birkaç karşılaştırma; p95 bütçesi etkilenmez, ayrıca ölçülmedi.
- CLI `status` uygulama kapalıyken: `latest_snapshots_by_source` tek bir pencereli sorgu (`latest`'in kaynak bazlı hâli).
