# Tray ikonu: tooltip ve sağ tık menüsü.

## Tooltip. Windows 127 karakterden sonrasını keser; tam metin daha uzunsa uygulama kısa
## biçimlere geçer.

tray-title = TokenRadar
# Bir limitin yüzdesi; tahminler "≈" taşır.
tray-value = { $estimate ->
    [yes] ≈{ PCT($pct) }
   *[no] { PCT($pct) }
}
tray-five-hour = 5s: { $value }
tray-five-hour-resets = 5s: { $value } · { DUR($ms) } sonra yenilenir
tray-five-hour-resets-short = 5s: { $value } ({ DUR($ms) })
tray-weekly = Haftalık: { $value }
# $at bir gün ve saattir, örneğin "Çar 10:59".
tray-weekly-resets = Haftalık: { $value } · { TR_SUFFIX($at, "loc") } yenilenir
tray-weekly-resets-short = Haftalık: { $value } ({ $at })
# Tek bir modelin haftalık limiti, %75'ten itibaren gösterilir. $name modelin adıdır, örneğin "Fable".
tray-weekly-scoped = { $name } · haftalık: { $value }
tray-weekly-scoped-resets = { $name } · haftalık: { $value } · { TR_SUFFIX($at, "loc") } yenilenir
tray-weekly-scoped-short = { $name } · 7g: { $value }
tray-no-data = Henüz limit verisi yok
tray-stale = Son veri: { $at }
tray-error = Limit verisi alınamıyor
tray-error-short = Veri alınamıyor
# $until bir saattir, örneğin "10:59".
tray-muted = Bildirimler { TR_SUFFIX($until, "dat") } kadar susturuldu
tray-muted-short = Sessiz: { TR_SUFFIX($until, "dat") } kadar

## Sağ tık menüsü

tray-menu-open-panel = Paneli aç
tray-menu-mute = Bildirimleri 1 saat sustur
tray-menu-unmute = Susturmayı kaldır
tray-menu-settings = Ayarlar…
tray-menu-about = TokenRadar hakkında
# Yalnızca geliştirme derlemelerinde.
tray-menu-gallery = Bileşen galerisi (geliştirme)
tray-menu-quit = Çıkış
