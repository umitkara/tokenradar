# Masaüstü bildirimleri (Rust notifier). Kısa, sakin ve eyleme dönük (design-language.md §8).
#
# $limit: five_hour | weekly | scoped. $name: kapsamlı haftalık limitin modeli, ör. "Fable".

# $what, aynı limit için biçimlenmiş notify-limit mesajıdır (Fluent terimleri değişken alamaz).

notify-limit = { $limit ->
    [weekly] Haftalık limit
    [scoped] { $name } haftalık limiti
   *[five_hour] 5 saatlik pencere
}

## Eşik. $pct, limitin ulaştığı eşiktir.

notify-threshold-title = { $pct ->
    [100] { $what } doldu
   *[other] { $what } { TR_SUFFIX(PCT($pct), "dat") } ulaştı
}

## Projeksiyon. $eta, dolmaya kalan süre (ms).

notify-projection-title = Bu hızla { $limit ->
    [weekly] haftalık limit
    [scoped] { $name } haftalık limiti
   *[five_hour] 5 saatlik pencere
} { DUR($eta) } içinde dolacak
notify-final-title = { $what } { DUR($eta) } içinde dolacak

## Limitin yenileneceği zaman. $ms bir süre; $at gün ve saat, ör. "Çar 10:59".

notify-resets-in = { DUR($ms) } sonra yenilenir.
notify-resets-at = { TR_SUFFIX($at, "loc") } yenilenir.

## Reset

notify-reset-title = { $what } yenilendi
notify-reset-body = Tam kapasite.

## Susturma veya sessiz saatlerden sonra: birden çok bildirim tek bildirimde. Gövde başlıkları listeler.

notify-summary-title = Duraklatılmışken { $count } bildirim

## Ayarlardaki "Test bildirimi gönder".

notify-test-title = TokenRadar test bildirimi
notify-test-body = Bildirimler çalışıyor. Limit uyarıları böyle görünecek.
