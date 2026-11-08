# `tokenradar` konsol uygulaması ve Claude Code statusline'ı.

## Statusline. Kısa tutun: satır, Claude Code ile aynı terminal satırını paylaşıyor.

sl-label-five-hour = 5s
sl-label-seven-day = 7g
sl-percent = { PCT($pct) }
sl-resets-in = { DUR($ms, "compact") }
sl-eta = ▲{ DUR($ms, "compact") }
# Pencerenin zaman olarak ne kadarının geçtiği (kullanım değil, saat).
sl-elapsed = ⏱{ PCT($pct) }
sl-session-share = ≈{ PCT($pct) } bu oturum
# Bağlam penceresinin doluluğu. `ctx` kısa formu her dilde aynı kalır.
sl-label-context = ctx
# Oturumun API fiyatlarıyla maliyeti (`cost.total_cost_usd`); fatura değildir. `API` işareti kalmalı.
sl-cost = API ${ NUM($usd, 2) }
sl-cost-precise = API ${ NUM($usd, 4) }
# Bağlam penceresindeki token'lar: girdi (önbellek dahil) ve son çıktı.
sl-label-tokens-in = girdi
sl-label-tokens-out = çıktı
# Oturum süresi ve API'yi bekleyerek geçen süre.
sl-label-duration = süre
sl-label-api-duration = api
sl-label-git = git
sl-duration = { DUR($ms, "compact") }
# Sayılar. `k` ve `M` kısa formları her dilde aynı kalır.
sl-number = { NUM($n, 0) }
sl-thousands = { NUM($n, $d) }k
sl-millions = { NUM($n, $d) }M
# Adım 12m. Bir limitin ya da bağlam penceresinin kalanı.
sl-remaining = { PCT($pct) } kaldı
# $n önceden biçimlendirilmiş bir sayı (`185k`).
sl-tokens-left = { $n } kaldı
# Haftalık pencerede kalan payın sıfırlanmaya kadar güne düşen kısmı.
sl-daily-budget = { PCT($pct) }/gün
# Oturumun API fiyatlarıyla saatlik maliyeti; `API` işareti kalmalı.
sl-burn-rate = API ${ NUM($usd, 2) }/sa
# Son istekte girdinin önbellekten okunan payı. `cache` kısa formu kalır.
sl-label-cache = cache
# Sıfırlanma yerel saatle: `Pzt 09:00`.
sl-reset-clock = { $day } { $time }
sl-weekday-mon = Pzt
sl-weekday-tue = Sal
sl-weekday-wed = Çar
sl-weekday-thu = Per
sl-weekday-fri = Cum
sl-weekday-sat = Cmt
sl-weekday-sun = Paz
sl-no-limits = henüz limit verisi yok

## Tablolar

col-limit = Limit
col-used = Kullanım
col-resets = Sıfırlanma
col-source = Kaynak
col-projection = Bu hızla
col-session = Oturum
col-project = Proje
col-status = Durum
col-share = Pencere payı
col-cost = Maliyet

limit-session = 5 saatlik
limit-weekly-all = Haftalık
limit-weekly-scoped = Haftalık ({ $scope })
limit-other = { $kind }

source-statusline = statusline
source-api = API
source-estimate = tahmin
source-codex-log = Codex günlüğü

projection-full-in = { DUR($ms) } içinde dolar
projection-reset-first = dolmaz
status-busy = meşgul
status-idle = boşta
status-unknown = bilinmiyor

data-from-app = TokenRadar uygulamasından canlı veri.
data-from-db = TokenRadar uygulaması çalışmıyor; son kaydedilen değerler gösteriliyor.
data-none = Henüz TokenRadar verisi yok. Oturumların indekslenmesi için uygulamayı bir kez başlatın.
no-limits = Henüz limit verisi yok.
limit-absent = { $name }: bu hesapta yok.
no-live-sessions = Çalışan Claude Code oturumu yok.
updated-ago = { DUR($ms) } önce güncellendi
estimate-note = ≈ işareti ölçümü değil tahmini gösterir.
statusline-last-seen = Statusline köprüsü en son { DUR($ms) } önce görüldü.
statusline-never-seen = Statusline köprüsü henüz görülmedi. Kurmak için: tokenradar statusline install

## statusline install / uninstall

install-no-config-dir = Claude Code yapılandırma dizini bulunamadı: { $path }
install-already = Statusline köprüsü { $path } içinde zaten kurulu.
install-confirm = Bu değişiklik { $path } dosyasına uygulansın mı? [e/H]
install-done = Kuruldu. Yedek: { $backup }
install-chained = Önceki statusline komutunuz çalışmaya devam ediyor; çıktısı TokenRadar satırının üstünde gösterilir.
install-chain-hint = Önceki statusline komutunuzun yerini TokenRadar alıyor. Korumak için --chain ile yeniden çalıştırın.
install-previous-mangled = Not: Önceki statusline komutunuzdaki ters bölüleri Git Bash siliyor, bu yüzden komut büyük olasılıkla bugün de hiçbir şey yazdırmıyor: { $command }. Yolu düz bölülerle (/) yazın ya da --no-chain ile kurun.
install-cancelled = Hiçbir şey değiştirilmedi.
install-not-interactive = Terminal değil; sormadan uygulamak için --yes verin.
uninstall-not-installed = Statusline köprüsü { $path } içinde kurulu değil.
uninstall-done = Kaldırıldı. Yedek: { $backup }
uninstall-restored = Önceki statusline ayarınız geri yüklendi.
uninstall-none = TokenRadar'ın statusline köprüsü hiçbir yerde kurulu değil.

## mod install / uninstall

mod-missing = Claude Code mod'u bu tokenradar'ın yanında değil; kurulu uygulamayla gelir.
mod-already = Claude Code mod'u { $path } içinde zaten listeli.
mod-done = Kuruldu. Claude Code mod'u artık her oturumda yükler. Yedek: { $backup }
mod-restart = Yüklenmesi için Claude Code'u yeniden başlatın. Bu mod için ayrıca --plugin-dir kullanmayın, yoksa iki kez yüklenir.
mod-not-installed = Claude Code mod'u { $path } içinde listeli değil.
mod-none = TokenRadar'ın Claude Code mod'u hiçbir yerde kurulu değil.
settings-changed-meanwhile = { $path } beklerken değişti; hiçbir şey yazılmadı. Komutu yeniden çalıştırın.
settings-invalid = { $path } geçerli bir JSON nesnesi değil; hiçbir şey değiştirilmedi.

## path add / remove

path-added = { $path } kullanıcı PATH'inize eklendi. Yeni terminaller tokenradar'ı bulur.
path-already = { $path } zaten kullanıcı PATH'inizde.
path-removed = { $path } kullanıcı PATH'inizden kaldırıldı.
path-absent = { $path } kullanıcı PATH'inizde değil.
path-too-long = Kullanıcı PATH'iniz { $length } karakter olurdu; bu, Windows araçlarının güvenle işleyebildiği { $max } sınırını aşıyor. Hiçbir şey değiştirilmedi. { $path } dizinini elle ekleyin ya da PATH'inizi kısaltın.
path-unsupported = Bu komut yalnızca Windows'ta kullanılabilir.

## Hatalar ve yardım

doctor-done = Tanılama paketi yazıldı: { $path }
    Token, konuşma içeriği veya okunabilir yol içermez. Paylaşmadan önce açıp kontrol edin.
error = hata: { $message }
cli-help =
    TokenRadar - yapay zekâ kodlama ajanları için kullanım limitleri
    Kullanım: tokenradar <KOMUT> [SEÇENEKLER]
    Komutlar:
      status [--json]                     Limitler, canlı oturumlar ve veri kaynakları
      limits [--json]                     Güncel limit değerleri
      sessions [--json]                   Çalışan Claude Code oturumları
      history [--json]                    Geçerli pencerelerin okumaları, grafik için (JSON)
      statusline [--chain]                Claude Code statusline'ı (JSON'u stdin'den okur)
      statusline install [--chain|--no-chain] [--yes] [--config-dir <DİZİN>]
      statusline uninstall [--yes] [--config-dir <DİZİN> | --all]
      mod install [--yes] [--config-dir <DİZİN>]
                                          TokenRadar'ın Claude Code mod'unu her oturumda yükle
      mod uninstall [--yes] [--config-dir <DİZİN> | --all]
      doctor [--out <ZIP>]                Sorun bildirirken paylaşılacak tanılama paketi
      path add|remove                     Bu tokenradar'ı kullanıcı PATH'inize ekler ya da kaldırır
    Seçenekler:
      -h, --help       Yardımı göster
      -V, --version    Sürümü göster
