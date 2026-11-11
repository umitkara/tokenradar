# Adım 18b — Widget'ın görev çubuğu modu: bellek ve CPU

**Tarih:** 2026-11-11 · **Sürüm:** 0.1.0 (`step/18b-taskbar-mode`) · **Karar:** ADR-058 · **Karşılaştırma:** [2026-11-11-widget.md](2026-11-11-widget.md) (Adım 18)

## Özet

| Ölçüt | Widget kapalı (18) | Yüzen (18) | Görev çubuğu (18b) | Fark (kapalıya göre) | Bütçe |
|---|---|---|---|---|---|
| Private working set, medyan (10 dk) | 8,24 MB | 9,97 MB | 10,01 MB | **+1,77 MB** | ≤ +8 MB ✅ |
| Private working set, tepe | 8,32 MB | 10,02 MB | 10,11 MB | +1,79 MB | — |
| Private bytes, medyan | 9,58 MB | 12,53 MB | 12,50 MB | +2,92 MB | — |
| CPU süresi, 600 s (tüm süreç) | 0,063 s | 0,094 s | 0,203 s | +0,140 s | — |
| Boşta CPU, tek çekirdeğe göre | %0,011 | %0,016 | %0,034 | **+%0,023** | < %0,1 ✅ |

- **Bellek:** Görev çubuğu modu yüzen moda göre yeni bir şey tutmuyor; fark (+0,04 MB) ölçüm gürültüsü düzeyinde.
- **CPU:** Yüzen moda göre ≈ 2 kat (+0,11 s / 10 dk), ama bütçenin üçte biri. Görev çubuğu modu 5 sn'de bir iki `FindWindowW` ve iki `GetWindowRect` ekliyor. Tam ekran yoklaması da bu modda her zaman açık; yüzen ölçümde de açıktı (varsayılan). Farkın tamamı bu çağrılara bağlanamaz, aşağıdaki koşul notuna bakın.

## Makine ve yöntem

| Öğe | Değer |
|---|---|
| CPU | Intel Core i5-13420H (8 çekirdek / 12 thread), 16 GB RAM |
| Ekran | Tek monitör, 1920×1200, %125 ölçek, görev çubuğu altta (48 mantıksal px), açık görev çubuğu teması |
| Build | `pnpm -C app tauri build --no-bundle --features spike` (release) |
| Betik | `scripts/measure-memory.ps1 -RootPid <pid> -DurationSec 600` |

- Exe `portable` işaretli geçici bir klasöre kopyalandı; config yalnızca `locale = "tr"`, `theme = "dark"`. `TOKENRADAR_FAKE_LIMITS=widget:both:taskbar`: iki satır, bildirim alanının solunda kenetli. Başlangıçtan 20 s sonra 600 s örneklendi (1176 örnek).
- Sahte mod çekirdeği başlatmaz (veritabanı, indeksleme, API ve pipe yok); 18'deki ölçümle aynı koşul.
- **Ham veri:** `raw/2026-11-11-widget-taskbar.csv`. Sahte veridir; kişisel kullanım içermez.

## Koşul notu ve eksikler

- **Aynı build ile yüzen mod karşılaştırması tamamlanamadı.** Görev çubuğu koşusundan hemen sonra başlatılan yüzen mod koşusu, sistem belleği azaldığı için ≈ 150 s'de durduruldu. O koşunun verileri (medyan private WS 8,21 MB) bellek baskısı altında toplandığı için kullanılmadı. Tablodaki yüzen mod değerleri Adım 18'in build'inden.
- **Bellek baskısı görev çubuğu koşusunu da etkilemiş olabilir.** Sayfa kırpma private WS'yi düşürür; CPU süresine sayfa hataları eklenebilir. CPU farkının bir kısmı bundan gelebilir. Değerler bütçenin çok altında olduğundan sonuç değişmez; daha kesin bir karşılaştırma için iki mod sakin bir makinede arka arkaya ölçülmeli.
- **Bağlam değişimleri ölçülemedi.** Widget thread'ini bulmak için kullanılan `FindWindowW("TokenRadarWidget")` ölçüm betiğinde pencereyi bulamadı (0 döndü). Bu yüzden 18'deki "uyanma" tablosunun karşılığı yok. Mantıksal uyanma yüzen modla aynı: 5 sn'de bir zamanlayıcı ve dakika zamanlayıcısı. Fare görev çubuğu üzerindeyken ek mesaj yok; yalnızca şeridin üstündeyken hover mesajları geliyor.

## Görsel kontrol

`CopyFromScreen` ile görev çubuğu bölgesi (%125, açık görev çubuğu):

- Bildirim alanının 8 px solunda iki satır; zemin görünmüyor, görev çubuğuyla kaynaşıyor (`docs/plan/img/18b-taskbar-en-light.png`).
- Explorer yeniden başlatma (kullanıcı onayıyla): görev çubuğu yokken widget yüzüyor, ≈ 5 sn içinde yeniden kenetleniyor. İlk denemede widget, log'a göre kenetli olduğu hâlde en az 5 sn görünmedi (büyük olasılıkla yeni görev çubuğunun altında kaldı); yeniden kenetlenince `HWND_TOPMOST` ile öne alma eklendikten sonra saniyede bir alınan görüntülerde widget sürekli görünür.

Koyu görev çubuğu, %100 ve %150 ölçek, yüksek kontrast ve kompakt tek satır yalnızca golden testlerde görüldü (`testdata/widget/docked-*`).
