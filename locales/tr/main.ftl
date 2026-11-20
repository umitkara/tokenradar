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

## Oturumlar (Adım 16a)

sessions-filters = Filtreler
sessions-search = Oturumlarda ara
sessions-search-placeholder = Ad, proje ya da oturum kimliği
sessions-range = Zaman aralığı
sessions-range-today = Bugün
sessions-range-7d = 7 gün
sessions-range-30d = 30 gün
sessions-range-all = Tümü
sessions-range-custom = Özel
sessions-from = Başlangıç
sessions-to = Bitiş
sessions-project = Proje
sessions-project-all = Tüm projeler
sessions-model = Model
sessions-model-all = Tüm modeller
sessions-live = Yalnızca çalışanlar
sessions-sort = Sıralama
sessions-sort-last-active = Son etkinlik
sessions-sort-started = Başlangıç
sessions-sort-cost = Maliyet
sessions-sort-tokens = Token
sessions-sort-impact = Etki
sessions-list = Oturumlar
sessions-col-name = Oturum
sessions-col-start = Başlangıç
sessions-col-duration = Süre
sessions-col-models = Modeller
sessions-col-cost = Maliyet
sessions-col-tokens = Token
sessions-col-impact = Etki
sessions-col-subagents = Alt ajanlar
sessions-running = Çalışıyor
sessions-ago = { DUR($ms) } önce
sessions-just-now = Az önce
sessions-usd = ${ NUM($usd, 2) }
sessions-impact-weekly = Bu aralıkta haftalık limitteki pay. Kullanımdan bölüştürülür, bu yüzden tahmindir.
sessions-impact-session = Bu aralıkta 5 saatlik pencerelerdeki payların toplamı.
sessions-impact-dim = Düşük güven: kalibrasyon hâlâ öğreniyor.
sessions-deleted = Transcript silindi
sessions-deleted-tip = Claude Code bu oturumun transcript dosyasını sildi. Daha önce kaydedilen toplamlar duruyor.
sessions-loading-more = Daha fazla oturum yükleniyor…
sessions-empty = Bu aralıkta oturum yok
sessions-empty-body = Claude Code kullanım yazar yazmaz oturumlar burada görünür.
sessions-empty-filtered = Bu filtrelere uyan oturum yok
sessions-empty-filtered-body = Daha uzun bir zaman aralığı seçin ya da aramayı temizleyin.
sessions-clear = Filtreleri temizle

## Oturum ayrıntısı (Adım 16a)

session-back = Oturumlara dön
session-breadcrumb = Konum
session-not-found = Bu oturum artık veritabanında yok.
# $level Claude Code'un /effort değeridir; bilinmeyen değerler olduğu gibi gösterilir.
session-effort = Efor: { $level ->
        [low] düşük
        [medium] orta
        [high] yüksek
        [xhigh] çok yüksek
       *[other] { $level }
    }
session-fast = Hızlı mod
session-fast-tip = Bu oturumda en az bir istek hızlı modda çalıştı.
session-chart = Zamana göre maliyet
# $ms bir çubuğun kapsadığı süredir.
session-chart-bucket = Bir çubuk { DUR($ms) }
# $total biçimlendirilmiş maliyettir ("$4.20").
session-chart-alt = Bu oturumun zamana göre maliyeti, model ailesine göre yığılmış. Toplam { $total }.
session-chart-empty = Bu oturum için kaydedilmiş maliyet yok.
session-chart-other = Diğer modeller
session-chart-window = Limit penceresi
session-impact = Limit payları
session-impact-window = Pencere
session-impact-share = Pay
session-impact-peak = Tepe
session-impact-hit = Limite takıldı mı
session-impact-yes = Evet
session-impact-no = Hayır
session-impact-unknown = Bilinmiyor
session-impact-estimated = Tahmini pencere, ölçüm yok.
session-impact-empty = Bu oturum bilinen bir limit penceresine denk gelmedi.
session-impact-note = Paylar kullanımdan bölüştürülür ve ≈ ile gösterilir; soluk değerlerin güveni düşüktür.
session-subagents = Alt ajanlar
session-subagents-empty = Alt ajan yok.
session-subagent-untitled = Adsız alt ajan
session-cache = Cache
session-cache-hit = İsabet oranı
session-cache-write-5m = 5 dk cache yazma
session-cache-write-1h = 1 sa cache yazma
session-cache-savings = Tasarruf
session-cache-savings-value = ≈${ NUM($usd, 2) }
session-cache-savings-tip = Cache'ten okunanlar düz girdi olarak API fiyatıyla ne tutardı, eksi gerçekte ne tuttu.
session-meta = Ayrıntılar
session-branch = Git dalı
session-version = Claude Code sürümü
session-entrypoint = Başlatıldığı yer
session-dirs = Klasörler
session-dirs-none = Kayıtlı klasör yok. Klasörler ve efor yalnızca bu sürüm ilk okuduğunda transcript'i hâlâ diskte olan oturumlar için bilinir.
session-open-dir = Klasörü aç
session-dir-missing = Bu klasör artık yok.
session-dir-failed = Klasör açılamadı.
session-tokens = Token

## Projeler (Adım 16b)

projects-search = Projelerde ara
projects-search-placeholder = Ad ya da yolun bir parçası
projects-group = Klasöre göre grupla
projects-hide-temp = Geçici dizinleri gizle
projects-sort-last-active = Son aktivite
projects-sort-cost = Maliyet
projects-sort-impact = Haftalık etki
projects-sort-sessions = Oturum sayısı
projects-hidden = Geçici dizinlerdeki { NUM($n, 0) } proje gizli.
projects-show-temp = Göster
projects-list = Projeler
projects-tree = Klasöre göre projeler
projects-col-name = Proje
projects-col-path = Yol
projects-col-sessions = Oturum
projects-col-last = Son aktivite
projects-col-cost = Maliyet
projects-col-weekly = Haftalık ort.
projects-col-trend = 8 hafta
projects-weekly-tip = Haftalık limitten ortalama pay: projenin bu aralıktaki her haftalık pencereden aldığı pay, pencere sayısına bölünür. Kullanımdan bölündüğü için tahmindir.
projects-trend-tip = Son 8 haftanın haftalık maliyeti, toplam ${ NUM($usd, 2) }
# A weekly share averaged per weekly window.
projects-per-week = { $value }/hafta
projects-count = { NUM($n, 0) } proje
projects-expanded = açık
projects-collapsed = kapalı
projects-loading = Projeler yükleniyor…
projects-empty = Bu aralıkta proje yok
projects-empty-body = Claude Code bir klasörde kullanım yazdığı anda projeler burada görünür.
projects-empty-filtered = Bu filtrelere uyan proje yok
projects-empty-filtered-body = Daha uzun bir zaman aralığı seçin, aramayı temizleyin ya da geçici dizinleri gösterin.

## Proje ayrıntısı (Adım 16b)

project-back = Projelere dön
project-not-found = Bu proje artık veritabanında yok.
project-cost = Maliyet, tüm zamanlar
project-weekly-total = Haftalık ortalama, tüm zamanlar
project-weeks = Haftalık etki
project-weeks-empty = Henüz kayıtlı haftalık pencere yok.
project-weeks-note = Her haftalık pencereden alınan pay, kullanımdan bölünür (≈). Kesikli çubuklar tahmini pencerelerdir.
project-weeks-by-cost = Henüz pay yok: kalibrasyon hâlâ öğreniyor. Çubuklar her haftanın maliyetini gösterir.
project-week-estimated = tahmini pencere
project-models-empty = Kayıtlı kullanım yok.
project-top = En pahalı oturumlar
project-top-empty = Oturum yok.
project-sessions = Oturumlar

## Pencereler (Adım 17a)

span-4w = 4 hafta
span-12w = 12 hafta
span-all = Tümü
span-custom = Özel
windows-tabs = Görünüm
windows-tab-weeks = Haftalık döngüler
windows-tab-strip = 5 saatlik pencereler
windows-tab-list = Liste
windows-filters = Pencere filtreleri
windows-empty = Bu aralıkta pencere yok.
windows-col-week = Hafta
windows-col-days = Günler
windows-col-peak = Tepe
windows-col-hit = Limite takıldı
windows-col-projects = En çok etkileyen projeler
windows-col-surfaces = Anthropic'in kırılımı
windows-col-start = Başlangıç
windows-col-end = Bitiş
windows-col-limit = Limit
windows-col-sessions = Oturum
windows-col-cost = Maliyet
windows-col-other = Diğer
windows-hit-no = Hayır
windows-hit-unknown = Bilinmiyor
windows-estimated = Tahmini
windows-open = Sürüyor
windows-limit-five = 5 saatlik pencere
windows-days-note = Günler, sıfırlanmadan başlayan 24 saatlik dilimlerdir. Değerleri bu bilgisayardaki Claude Code kullanımının ≈ tahminidir ve "Diğer"i içermez; bu yüzden toplamları tepe değerin altında kalabilir.
windows-days-by-cost = Henüz güvenilir bir kalibrasyon yok, bu yüzden günler maliyeti gösteriyor.
windows-day-tip = { $when } · { $value } · { $cost }
windows-day-future = { $when } · henüz gelmedi
windows-project-none = Proje yok
windows-strip-none-title = Bu hesapta 5 saatlik pencere yok
windows-strip-none-body = Doğrulanmış Ultra hesaplarında yalnızca haftalık limitler var. Onları haftalık döngüler gösteriyor.
windows-strip-legend = Dolgu tepe değeri gösterir. Kırmızı kenarlık pencerenin limite takıldığını, kesikli kenarlık tahmini olduğunu söyler.
windows-strip-days = Günler
windows-block-tip = { $from }–{ $to } · tepe { $peak }{ $hit ->
    [yes] {" "}· limite takıldı
   *[no] {""}
}
window-close = Kapat
window-curve = Kullanım
window-curve-none = Bu pencerenin okuması yok: kayıtlardan tahmin edildi.
window-contrib = Oturum katkıları
window-contrib-rest = Diğer oturumlar
window-contrib-note = Her oturumun payı, maliyetiyle aynı biçimde zamana yayıldı.
window-contrib-cost = Bu pencerede bölüştürülecek bir yüzde yok, bu yüzden katkılar maliyeti gösteriyor.
window-contrib-empty = Bu pencerede Claude Code kullanımı yok.
window-shares = Paylar
window-share-other = Diğer (claude.ai, Desktop)
window-shares-empty = Henüz atıf yapılmadı.
window-alerts = Bildirimler
window-alerts-empty = Bu pencere için bildirim yok. Kayıt 30 gün tutulur.
window-alert = { $rule ->
    [threshold] %{ $threshold } aşıldı
    [projection] Limite ulaşacağı öngörüldü
    [final] Limitten önceki son uyarı
    [reset] Limit sıfırlandı
   *[other] { $rule }
}
window-not-found = Bu pencere artık yok.

## Örüntüler (Adım 17a)

patterns-filters = Örüntü filtreleri
patterns-heatmap = Haftanın günü ve saat
patterns-metric = Ölçü
patterns-metric-percent = ≈ % haftalık
patterns-metric-cost = Maliyet
patterns-metric-tokens = Token
patterns-heat-label = Haftanın gününe ve saate göre kullanım, yerel saat
patterns-cell-tip = { $day } { $hour } · toplam { $total } · { $days ->
    [0] etkinlik yok
   *[other] { $days } etkin günde ortalama { $mean }
}
patterns-heat-legend-low = Az
patterns-heat-legend-high = Çok
patterns-percent-none = Henüz güvenilir bir kalibrasyon yok, bu yüzden ≈ % gösterilemiyor. Maliyet ya da token seçin.
patterns-empty = Bu aralıkta kullanım yok.
patterns-hours = Gün içi dağılım
patterns-weekday = Hafta içi
patterns-weekend = Hafta sonu
patterns-hours-note = Aralıktaki hafta içi ve hafta sonu günlerinin gün başına ortalaması.
patterns-hour-tip = { $hour } · hafta içi { $weekday } · hafta sonu { $weekend }
patterns-risk = Limite takılma riski en yüksek saatler
patterns-risk-none-title = 5 saatlik pencere yok
patterns-risk-none-body = Bu bölüm 5 saatlik pencerelere bakar. Doğrulanmış Ultra hesaplarında yalnızca haftalık limitler var.
patterns-risk-few = Henüz yeterli veri yok: { $min } pencerenin { $n } tanesi var.
patterns-risk-statement = Pencereleriniz en çok { $from }-{ $to } arasında başlayınca limite takılıyor ({ $n } pencerenin { $hit } tanesi).
patterns-risk-flat = Öne çıkan bir başlangıç saati yok: { $n } pencerenin { $hit } tanesi limite takıldı.
patterns-risk-hit = Limite takıldı
patterns-risk-high = %90'a ulaştı
patterns-risk-windows = Tüm pencereler
patterns-risk-tip = { $hour } · { $n } pencere · { $hit } takıldı · { $high } tanesi %90 ve üstü
patterns-mix = Haftalara göre model karışımı
patterns-mix-note = Her haftanın API karşılığı maliyetindeki pay.
patterns-mix-tip = { $when } haftası · { $parts }

## Dışa aktarma (Adım 17a)

export-button = Dışa aktar…
export-title = { $what ->
    [sessions] Oturumları
    [windows] Pencereleri
   *[daily] Günlük toplamları
} dışa aktar
export-format = Biçim
export-hide-paths = Proje yollarını gizle
export-scope = Bu sayfada görünen aralığı ve filtreleri kullanır.
export-privacy = Yalnızca kullanım ve meta veri. İstem ya da yanıt yok.
export-save = Kaydet…
export-saved = Kaydedildi: { $path }
export-failed = Dosya yazılamadı: { $error }
