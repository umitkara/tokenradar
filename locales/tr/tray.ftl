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
tray-menu-onboarding = Başlangıç rehberi…
tray-menu-about = TokenRadar hakkında
tray-menu-update = Güncelleme var: { $version }…
# Yalnızca geliştirme derlemelerinde.
tray-menu-gallery = Bileşen galerisi (geliştirme)
tray-menu-quit = Çıkış
tray-menu-widget-show = Widget'ı göster
tray-menu-widget-hide = Widget'ı gizle
# Widget tıklamaları geçirirken görünür; onu yeniden tıklanabilir yapar.
tray-menu-widget-interactive = Widget'ı etkileşimli yap

## Widget: masaüstündeki küçük şerit (Adım 18). Etiketler çok kısa tutulur.

widget-label-weekly = 7g
widget-label-five-hour = 5s
# Okuması olmayan bir limit.
widget-value-none = –
widget-menu-open-panel = Paneli aç
# $hotkey onu yeniden kapatan kısayoldur, örneğin "Ctrl+Alt+Shift+R". Sekme onu sağa yaslar.
widget-menu-click-through = Tıklama geçirgenliği	{ $hotkey }
widget-menu-opacity = Opaklık
widget-menu-opacity-option = { PCT($pct) }
widget-menu-settings = Widget ayarları…
widget-menu-hide = Widget'ı gizle
