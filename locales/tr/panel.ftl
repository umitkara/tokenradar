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
