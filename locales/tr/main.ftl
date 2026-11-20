# Ana pencere (Adım 15): navigasyon, sayfa başlıkları ve sayfaların boş durumları.

main-nav = Sayfalar
main-nav-overview = Genel Bakış
main-nav-windows = Pencereler
main-nav-sessions = Oturumlar
main-nav-projects = Projeler
main-nav-patterns = Örüntüler
main-nav-settings = Ayarlar
# Pencere başlığı: $page sayfanın adıdır.
main-title = { $page } – TokenRadar
main-loading = Yükleniyor…
main-load-failed = Bu sayfa yüklenemedi. Pencereyi kapatıp yeniden açın.

## Sonraki bir sürümde gelecek sayfalar.

main-soon-windows = Limit pencerelerinin arşivi geliyor
main-soon-windows-body = Kapanan her 5 saatlik ve haftalık pencere: ne kadar doldu, neyin tükettiği.
main-soon-sessions = Oturum listesi geliyor
main-soon-sessions-body = Her Claude Code oturumu, maliyeti ve limitlerinizdeki payıyla.
main-soon-projects = Proje listesi geliyor
main-soon-projects-body = Projeleriniz, oturumları, maliyetleri ve limitlerinizdeki paylarıyla.
main-soon-patterns = Kullanım örüntüleri geliyor
main-soon-patterns-body = Claude Code'u haftanın hangi saatlerinde en çok kullandığınız, ısı haritası olarak.

## Grafikler

main-chart-7d = Son 7 gün
# Yüzde grafiğinin ekran okuyucu özeti. $name limit, $pct son değeridir.
chart-alt = { $name }, son 7 gün, son değer { PCT($pct) }
chart-empty = Bu aralıkta henüz okuma yok.
# Açıklama ve ipucu: limitine ulaşmış pencere.
chart-hit-limit = Limite ulaşıldı
chart-spans = Oturumlar
chart-span-share = Haftalık limitteki payı
# Şeritteki bir oturumun ekran okuyucu adı. $value payıdır.
chart-span-alt = { $name }: { $value }

## Genel Bakış (Adım 15b)

overview-now = Şimdi
overview-calibration = Kalibrasyon: { $level ->
        [high] yüksek
        [medium] orta
       *[low] düşük
    } güven
overview-calibration-tip = Kullanımın bu limite nasıl sayıldığı { $n } aralıktan öğrenildi.
overview-calibration-none = Kalibrasyon henüz yok
overview-estimates-only = Yalnızca tahmini değerler var. Ölçülen değerler için statusline köprüsünü kurun ya da API erişimine izin verin.
# Grafiğin altındaki oturum şeridinin açıklaması.
overview-spans-note = Alttaki çizgiler haftalık limitte en büyük paya sahip 5 oturum. Üzerine gelince adı ve payı görünür.

overview-week = Bu hafta
# $at hafta penceresinin başladığı yerel gün ve saattir ("Çar 11:00").
overview-week-since = Hafta başlangıcı { $at }
overview-week-rolling = Haftalık pencere bilinmiyor; son 7 gün
overview-cost = API karşılığı maliyet
overview-cost-value = API ${ NUM($usd, 2) }
overview-cost-tip = API fiyatlarıyla hesaplanır; fatura değildir.
overview-change = { $dir ->
        [up] ▲ { PCT($pct) }
        [down] ▼ { PCT($pct) }
       *[flat] Değişmedi
    }
overview-change-basis = geçen haftanın aynı anına göre
overview-change-none = Karşılaştıracak geçen hafta yok
overview-previous = Geçen haftanın tamamı: API ${ NUM($usd, 2) }
overview-tokens = Token
overview-token-input = Girdi
overview-token-output = Çıktı
overview-token-cache-read = Cache okuma
overview-token-cache-write = Cache yazma
# Kısaltılmış sayı: $unit büyüklüktür.
overview-count = { $unit ->
        [k] { NUM($n, 1) } bin
        [m] { NUM($n, 1) } Mn
        [b] { NUM($n, 1) } Mr
       *[one] { NUM($n, 0) }
    }
overview-models = Model karışımı
overview-model-rest = Diğer modeller
overview-week-empty = Bu hafta henüz kullanım yok.

overview-top = En çok tüketenler
overview-top-sessions = Oturumlar
overview-top-projects = Projeler
overview-top-share = Haftalık limitteki payı
overview-top-cost = Bu haftaki API karşılığı maliyet
overview-top-trend = Bu haftanın günlerine göre maliyet
overview-learning = Paylar şimdilik maliyete oranla dağıtılıyor ve yaklaşıktır. Kalibrasyon güveni orta olunca kesinleşir. Şu an { $level ->
        [high] yüksek
        [medium] orta
       *[low] düşük
    } güven, { $n } aralık.
overview-learning-none = Paylar şimdilik maliyete oranla dağıtılıyor ve yaklaşıktır. Kalibrasyon öğrenilince kesinleşir.
overview-learning-no-week = Haftalık pencere bilinmediği için pay yok; sıralama maliyete göre.
overview-stale = Paylar eski bir kalibrasyonla hesaplandı; arka planda yenileniyor.

overview-other = Diğer kullanım
overview-other-body = Bu bilgisayardaki Claude Code dışındaki kullanım: claude.ai, Desktop, Cowork ya da başka bir bilgisayar.
overview-other-ours = Bizim tahminimiz
overview-other-ours-value = Haftalık limitin { TR_SUFFIX(PCT($pct), "poss") }
overview-other-ours-share = Toplam kullanımın { TR_SUFFIX(PCT($pct), "poss") }
overview-other-api = Anthropic'in kırılımı
overview-other-none = Kendi tahminimiz kalibrasyon güveni orta olunca hesaplanır.
overview-other-no-breakdown = Kırılım yalnızca kullanım API'si açıkken gelir.
overview-other-suspect = Tahminimiz kırılımdan 10 puandan fazla düşük. Kalibrasyon bu bilgisayardaki kullanımı fazla sayıyor olabilir.
overview-other-excess = Tahminimiz kırılımdan yüksek. Başka bir bilgisayardaki Claude Code da burada "Diğer" olarak görünür.
overview-surface-claude-code = Claude Code
overview-surface-chat = claude.ai
overview-surface-cowork = Cowork
overview-surface-other = Diğer

overview-plan = Plan değeri
overview-plan-month = Bu ay API karşılığı
overview-plan-fee = { $source ->
        [user] Plan ücreti (sizin girdiğiniz)
       *[table] Plan ücreti ({ $plan })
    }
overview-plan-usd = ${ NUM($usd, 2) }
overview-plan-ratio = ×{ NUM($x, 1) }
overview-plan-ratio-label = Ücretin katı
overview-plan-note = Bu sayılar bilgi amaçlıdır: API fiyatlarıyla hesaplanır, faturanız değildir.
overview-plan-no-fee = Bu planın ücreti bilinmiyor. Ayarlardan girebilirsiniz.
overview-plan-elapsed = Ayın { TR_SUFFIX(PCT($pct), "poss") } geçti
overview-plan-previous = Geçen ay ×{ NUM($x, 1) } (API ${ NUM($usd, 2) })
overview-plan-edit = Plan ücretini düzenle
