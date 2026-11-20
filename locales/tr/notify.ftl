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

## Dönemsel özetler (Adım 17b). Tıklanınca Genel Bakış açılır.
## $approx: yes | no, yüzdenin tahmin olup olmadığı (≈ ile gösterilir).

digest-weekly-title = Geçen hafta: tepe { $approx ->
    [yes] ≈
   *[no] {""}
}{ PCT($pct) }
digest-weekly-title-nopct = Geçen haftanın özeti
# $windows tane 5 saatlik pencereden $hit tanesi %100'e ulaştı.
digest-five-hour-hits = { $hit ->
    [0] Hiçbir 5 saatlik pencere limite takılmadı
   *[other] { $windows } adet 5 saatlik pencereden { $hit } tanesi limite takıldı
}
# $at: gün ve saat, ör. "Çar 14:20".
digest-weekly-hit-at = Haftalık limite { TR_SUFFIX($at, "loc") } takıldı
digest-weekly-hit = Haftalık limite takıldı
digest-weekly-not-hit = Haftalık limite takılmadı
digest-weekly-hit-unknown = Limite takılıp takılmadığı bilinmiyor (tahmini hafta)
# $name: kapsamlı haftalık limitin modeli, ör. "Fable".
digest-scoped-hit = { $name } doldu
digest-scoped-peak = { $name } { $approx ->
    [yes] ≈
   *[no] {""}
}{ PCT($pct) }
digest-cost = API karşılığı ${ NUM($usd, 2) }
digest-top-project = En çok: { $name }
# $parts: aşağıdaki karşılaştırmalar, ", " ile birleştirilmiş.
digest-delta = Önceki haftaya göre: { $parts }
# $delta: işaretli sayı, ör. "+12" veya "−8"; maliyette işaretli yüzde.
digest-delta-peak = tepe { $delta } puan
digest-delta-cost = maliyet { $delta }
# $pts: dün haftalık limitten harcanan tahmini puan.
digest-daily-title = Dün: haftalık limitin ≈{ NUM($pts, 1) } puanı
digest-daily-title-cost = Dün: { $cost }
digest-sessions = { $count } oturum
