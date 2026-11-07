# Adım 12h — Panelin Ayrıntı görünümü: açılış, bellek, bundle ve okuma süresi

**Tarih:** 2026-11-07 · **Dal:** `step/12h-panel-ayrinti` · **Karar:** ADR-048

## Özet

| Ölçüt | Sonuç | Bütçe |
|---|---|---|
| Soğuk açılış, Ayrıntı görünümü (9 açılış, 3 koşu × 3 tur) | **379,6 ms** medyan (346,9–515,2) | ≤ 500 ms ✅ |
| Sıcak açılış (9 açılış) | **47,2 ms** medyan (34,0–56,4) | ≤ 50 ms ✅ |
| Panel açık, private WS (tur medyanlarının en büyüğü) | **82,2 MB** (tepe 87,5) | ≤ 130 MB ✅ |
| `warm_ttl + 10 s` sonra | **8,1–10,8 MB** (aşağıdaki not) | ≤ 30 MB ✅ |
| Panel sayfası ilk yük (gzip) | **42,8 KB JS · 4,6 KB CSS** (main: 39,2 · 4,3) | ≤ 60 · ≤ 15 KB ✅ |
| Ayrıntı parçası, tembel (gzip) | 3,6 KB JS · 1,2 KB CSS | — |
| Ayrıntı verisi okuma (`ReadOnlyStore` + `panel_detail`) | medyan 1,29 ms (en az 1,24, soğuk 13,65) | — |

## Yöntem

- **Makine:** Adım 10 kaydındakiyle aynı (i5-13420H, 16 GB, %125 ölçek). WebView2 GPU kapalı (ADR-032).
- **Build:** `pnpm tauri build --no-bundle --features spike` (release). Portable bir kopya (`portable` işareti, `data/config.toml` içinde `panel_view = "detail"`) kullanıldı; kurulu uygulama ölçüm süresince kapatıldı (kullanıcı onayıyla).
- **Betik:** `scripts/measure-panel.ps1 -Runs 3 -Tag 2026-11-07-panel-detail`, sahte sahne `panel:ok`. Senaryo Adım 10'daki gibi. Panel her açılışta Ayrıntı görünümüyle açılır, yani soğuk açılış tembel parçanın yüklenmesini ve `get_panel_detail` çağrısını da içerir.
- **Bundle:** `scripts/check-bundle-size.mjs` (statik importlar, gzip 9). Main değeri, `app/src` ve `locales` geçici olarak main'den alınıp aynı betikle ölçüldü.
- **Okuma süresi:** Gerçek veritabanında, salt okunur, geçici bir örnek programla tekrarlı okuma: bir haftalık `snapshots_between` + `panel_detail`. Program ölçümden sonra silindi. Kişisel değer kaydedilmedi; yalnızca süreler var.
- **Ham veri:** `raw/2026-11-07-panel-detail-*`.

## Notlar

- **1. koşunun 3. turundaki `ttl3_ws` = 80,4 MB bir sızıntı değil.** Log, pencerenin TTL ile yok edildiğini gösteriyor (`warm ttl over; destroying`, bellek 10,4 MB). 5 s sonra bir hover ön ısıtması (`prewarm start`) ve bir açılış geliyor; büyük olasılıkla ölçüm sırasında imleç tray ikonunun üzerinden geçti (log bunu ayırt etmiyor). Bu tur dışarıda bırakıldığında `warm_ttl + 10 s` değeri 8,1–10,8 MB.
- 3. koşunun 3. turundaki 515 ms'lik soğuk açılış tek bir aykırı değer; diğer sekiz açılış 347–413 ms. Adım 10'un özet görünümü ölçümü 407 ms medyandı. Ayrıntının tembel parçası ve veri çağrısı soğuk açılışı ölçülebilir biçimde uzatmadı.
- Panel açık bellek Adım 10'dakinden (76,6 MB) ~6 MB fazla; Ayrıntı'nın daha büyük DOM'u (SVG, şerit, kartlar) beklenen kaynak, ayrıca ayrıştırılmadı.
