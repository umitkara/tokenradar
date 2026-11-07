# Tray paneli (Adım 10). Boş ve hata durumları design-language.md bölüm 9'u izler.

panel-live-sessions = Çalışan oturumlar
panel-top-sessions = Bu pencerede en çok
panel-no-live-sessions = Şu an aktif oturum yok.
panel-open-settings = Ayarlar
panel-close = Kapat
# Üst bilgideki çalışan Claude Code oturumları.
panel-active = { $count } aktif
# $plan "Max 20x" gibi bir plan adı, $label hesap adıdır (ör. "Claude").
panel-account = { $label } · { $plan }
# Listenin gösterdiğinden fazla satır.
panel-more = +{ $count } daha
panel-limits = Limitler
panel-sessions = Oturumlar
panel-loading = Kullanım yükleniyor…
panel-extra-usage = Ek kullanım açık: limit aşımı ayrıca ücretlendirilir.
# Hesapta 5 saatlik pencere yok (örneğin doğrulanmış Ultra hesapları); yalnızca haftalık limitler geçerli.
panel-no-five-hour = Bu hesapta 5 saatlik pencere yok. Yalnızca haftalık limitler geçerli.

## Limitlerin altındaki projeksiyon. $kind yalnızca haftalık limitte "weekly" olur.

# $eta %100'e kalan süre, $reset pencerenin yenilenmesine kalan süredir; projeksiyon az okumaya
# dayanıyorsa $approx "yes" olur.
panel-projection-will-hit = { $kind ->
    [weekly] ▲ Haftalık: bu hızla { $approx ->
        [yes] ≈{ DUR($eta) }
       *[no] { DUR($eta) }
    } içinde dolacak, reset { DUR($reset) } sonra
   *[five_hour] ▲ Bu hızla { $approx ->
        [yes] ≈{ DUR($eta) }
       *[no] { DUR($eta) }
    } içinde dolacak, reset { DUR($reset) } sonra
}
panel-projection-at-reset = { $kind ->
    [weekly] Haftalık: bu hızla reset'e kadar ≈{ PCT($pct) }
   *[five_hour] Bu hızla reset'e kadar ≈{ PCT($pct) }
}
panel-projection-idle = Tüketim yok
# Projeksiyon az okumaya dayanıyorsa ipucu olarak gösterilir.
panel-projection-low = Henüz az okuma var; kaba bir tahmin.

## Alt bilgi

panel-refresh = Yenile
panel-refresh-wait = Bir dakikadan kısa süre önce yenilendi
panel-refresh-blocked = API'ye yeniden ulaşılana kadar yenileme duraklatıldı
panel-mute = Bildirimleri 1 saat sustur
panel-unmute = Bildirimleri aç

## İlk açılış ve eksik veri

# $pct son 30 günün ne kadarının işlendiğidir.
panel-indexing = Son 30 günün oturumları işleniyor… { PCT($pct) }
panel-indexing-ready = Mevcut pencere hazır
panel-no-claude = Claude Code bulunamadı.
panel-no-claude-hint = Klasörünü seçin ya da TokenRadar'ı başlatmadan önce CLAUDE_CONFIG_DIR değişkenini ayarlayın.
panel-choose-folder = Klasör seç
panel-no-limits = Henüz limit verisi yok
panel-install-statusline = Statusline köprüsünü kur (önerilen)
panel-allow-api = API erişimine izin ver
panel-estimate-mode = O zamana kadar limitler oturum kayıtlarından tahmin edilir.

## Veri sorunları. Sakin dil, hata rengi yok.

panel-token-stale = Claude Code'u bir kez çalıştırınca güncellenecek.
# $ms son değerin ne kadar önce geldiğidir.
panel-offline = Çevrimdışı · son değer { DUR($ms) } önce
panel-schema-drift = Claude Code güncellenmiş olabilir; bazı veriler okunamıyor.
panel-diagnostics = Tanılamayı göster

## Panelin iki görünümü (ADR-048)

panel-views = Panel görünümü
panel-view-summary = Özet
panel-view-detail = Ayrıntı

## Ayrıntı görünümü: her limit zamana karşı, haftalık zaman şeridi, kullanım grafiği

detail-loading = Ayrıntılar yükleniyor…
detail-pace = Geçen zaman { PCT($elapsed) } · { $dir ->
    [ahead] zaman çizgisinin { NUM($points, 0) } puan önünde
    [behind] zaman çizgisinin { NUM($points, 0) } puan gerisinde
   *[even] zaman çizgisiyle aynı hızda
}
detail-strip-title = { $name } · geçen zaman { PCT($elapsed) }
detail-chart-title = { $name }, pencere boyunca
detail-now = şimdi
detail-strip-reset = Sıfırlanma { $at }
detail-legend-midnight = gece yarısı
detail-legend-period = reset'ten itibaren her 24 saat, altında zaman çizgisinin değeri
detail-legend-time = zaman çizgisi
detail-legend-use = kullanım
detail-chart-empty = Bu pencereye ait kayıtlı okuma henüz yok.
detail-chart-alt = { $name } kullanımının pencere boyunca zaman çizgisiyle karşılaştırması: kullanılan { PCT($used) }, geçen zaman { PCT($elapsed) }.
detail-session-cost = API ${ NUM($usd, 2) }
detail-session-cost-tip = Bu penceredeki API karşılığı maliyet; fatura değildir.
detail-estimate-note = ≈ işareti ölçümü değil tahmini gösterir.
