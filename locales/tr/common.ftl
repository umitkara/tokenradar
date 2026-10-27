# Tüm pencerelerin ortak metinleri: limit adları, değerler, kaynaklar, oturumlar ve bileşenler.

app-name = TokenRadar

## Limitler. Dış halka ve üst çubuk her zaman haftalık; iç halka ve alt çubuk 5 saatlik
## penceredir.

limit-five-hour = 5 saatlik pencere
limit-five-hour-short = 5s
limit-weekly = Haftalık limit
limit-weekly-short = 7g
# Tek bir modelin haftalık limiti. $name modelin adıdır, örneğin "Fable".
limit-weekly-scoped = { $name } · haftalık

# Bir yüzde; tahminler her zaman "≈" taşır.
value-pct = { $estimate ->
    [yes] ≈{ PCT($pct) }
   *[no] { PCT($pct) }
}
value-none = –

resets-in = { DUR($ms) } sonra yenilenir
# Halkanın ortasındaki değerin altında; "sonra yenilenir" sığmaz, yalnızca kalan süre.
resets-in-short = { DUR($ms) }
# $at bir gün ve saattir, örneğin "Çar 10:59".
resets-at = { TR_SUFFIX($at, "loc") } yenilenir
resets-now = Şimdi yenileniyor
projection-full-in = Bu hızla { DUR($ms) } içinde dolacak
projection-at-reset = Yenilenme anında öngörülen { PCT($pct) }

## Ekran okuyucu cümleleri: halka ve çubuklar (design-language.md bölüm 7).

aria-limit = { $estimate ->
    [yes] { $name } yaklaşık { PCT($pct) } dolu (tahmini)
   *[no] { $name } { PCT($pct) } dolu
}
aria-limit-resets = { $estimate ->
    [yes] { $name } yaklaşık { PCT($pct) } dolu (tahmini), { DUR($ms, "long") } sonra yenilenecek
   *[no] { $name } { PCT($pct) } dolu, { DUR($ms, "long") } sonra yenilenecek
}
aria-limit-none = { $name }: henüz veri yok
aria-ring = { $outer }. { $inner }.

severity-warn = Uyarı
severity-crit = Kritik

## Bir sayının nereden geldiği.

source-statusline = Canlı
source-api = API
source-estimate = Tahmini
source-codex-log = Codex günlüğü
source-updated = { DUR($ms) } önce güncellendi
source-updated-now = Az önce güncellendi

## Oturumlar

session-busy = Meşgul
session-idle = Boşta
session-unknown = Bilinmiyor
session-untitled = Adsız oturum
# Bir oturumun mevcut 5 saatlik penceredeki tahmini payı.
session-share = ≈{ PCT($pct) } bu pencere
# $k dakikada bin token cinsindendir.
session-rate = { NUM($k, 1) }k tok/dk

## Genel durumlar

loading = Yükleniyor…
no-limit-data = Henüz limit verisi yok
