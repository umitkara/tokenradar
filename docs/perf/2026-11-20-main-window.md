# Adım 15b — Ana pencere ve Genel Bakış: açılış, rota geçişi, bellek, bundle

**Tarih:** 2026-11-20 · **Dal:** `step/15b-overview` · **Karar:** ADR-067

## Özet

| Ölçüt | Sonuç | Bütçe |
|---|---|---|
| Soğuk açılış, webview oluşturmadan ilk boyamaya (`main_ready`, 5 açılış) | **72,6 ms** medyan (72,0–77,2) | — |
| Soğuk açılış, gezinmeden Genel Bakış verisinin boyanmasına (`overview:data`) | **161 ms** medyan (142–166) | — |
| Soğuk açılış, ikinci süreci başlatmadan pencerenin görünmesine (kötümser üst sınır) | **434 ms** medyan (416–496) | ≤ 600 ms ✅ |
| Sıcak açılış, aynı yol (gizli pencere, TTL içinde) | **35,6 ms** medyan (29,5–57,6) | — |
| Rota geçişi, sayfa çerçevesi (`route:*`, 25 geçiş) | **9,5 ms** medyan (3,5–25,1) | ≤ 100 ms ✅ |
| Genel Bakış'a geçiş, veriyle boyanana kadar (5 geçiş) | **42,7 ms** medyan (40,3–50,4) | ≤ 100 ms ✅ |
| Ana pencere açık, private WS (20 s, 1 s aralık) | **118,3–129,2 MB** | ≤ 180 MB ✅ |
| Pencere kapandıktan ve `warm_ttl` dolduktan sonra | **13,3 MB** (tek süreç, webview yok) | — |
| `main.html` JS, giriş + tüm tembel sayfalar (gzip) | **77,9 KB** (15a: 60,4) · CSS 6,9 KB | ≤ 180 KB ✅ |

## Yöntem

- **Makine:** Adım 10 kaydındakiyle aynı (i5-13420H, 16 GB, %125 ölçek). WebView2 GPU kapalı (ADR-032).
- **Build:** `pnpm tauri build --no-bundle` (release). Bir portable kopya kullanıldı (`portable` işareti, gerçek veritabanının bir kopyası `data/` altında). Kurulu uygulama ölçüm süresince kapatıldı (kullanıcı onayıyla), sonra yeniden başlatıldı. Kopya ve gerçek veriden alınan inceleme görüntüleri ölçümden sonra silindi.
- **Senaryo:** Uygulama tray'de çalışırken ana pencere `tokenradar-app.exe tokenradar://main/overview` ile açıldı. Bu ikinci bir süreç başlatır ve bağlantıyı tek örnek eklentisiyle çalışan uygulamaya iletir. Her turda:
  1. Soğuk açılış: `cold open` günlük satırı (webview oluşturmadan `main_ready`'ye), CDP ile `overview:data` ölçümü ve süreç başlatmadan `IsWindowVisible`'a kadar geçen süre.
  2. Pencere `WM_CLOSE` ile kapatıldı (gizlenir). 2 s sonra aynı komutla yeniden açıldı (sıcak).
  3. Yeniden kapatıldı ve `warm_ttl` (30 s) dolana kadar 40 s beklendi.
- **Hangi sayı bütçeye karşılık gelir:** Bütçe tray ya da panelden açılışı kastediyor. O yolda ikinci süreç başlatılmaz; ölçümde başlatıldığı için 434 ms kötümser bir üst sınırdır. `main_ready` iskelet boyandığında gelir; verinin boyanması `overview:data` ile ölçülür (gezinme webview oluşturulurken başladığı için bu süre webview oluşturmanın çoğunu da içerir).
- **Rota geçişi:** Açık pencerede CDP ile navigasyon bağlantılarına tıklandı (5 tur × 5 sayfa). `route:*` ölçümü tıklamadan yeni sayfanın ilk boyamasına kadardır. Genel Bakış'a dönüşte sayfa yeniden sorgu yaptığı için, veriyle boyanma ayrıca ölçüldü (`.pair` görünene kadar, bir sonraki kare).
- **Bellek:** `scripts/measure-memory.ps1 -RootPid <app> -DurationSec 20`, pencere açık ve Genel Bakış'ta. Sonra pencere kapatıldı, 40 s sonra 10 s örneklendi.
- **Bundle:** `scripts/check-bundle-size.mjs`.
- **Ham veri:** `raw/2026-11-20-main-window-opens.csv` (açılışlar), `raw/2026-11-20-main-window-open.csv` ve `raw/2026-11-20-main-window-after-ttl.csv` (bellek). Kişisel değer yok; yalnızca süreler ve bellek.

## Notlar

- İlk iki ölçüm koşusu yarıda kaldı. Birinde rota seçicisi yanlıştı (`#/windows`; doğrusu `#windows`). Diğerinde pencere ölçüm sırasında elle kullanıldı. Bu koşuların sayıları kullanılmadı. Yukarıdaki açılış satırları üçüncü koşudan, rota satırları ayrı bir koşudandır.
- 1. ve 2. turdaki ≈ 495 ms'lik süreç başlatma süreleri, 3–5. turlardan (416–434 ms) yüksek. `cold open` ve `overview:data` aynı aralıkta kaldığı için fark süreç başlatmada (disk ve Defender önbelleği); ayrıca incelenmedi.
- Bellek, panel Ayrıntı ölçümündeki panel açık değerinden (82,2 MB) ≈ 40 MB fazla. Ana pencere daha büyük ve uPlot kanvası içeriyor; 180 MB bütçesinin altında.
- Sanallaştırma bu adımda yok: Genel Bakış'ta en fazla 5 + 5 satır var. Büyük listeler Adım 16'da gelir.
