# `tokenradar` konsol uygulaması ve Claude Code statusline'ı.

## Statusline. Kısa tutun: satır, Claude Code ile aynı terminal satırını paylaşıyor.

sl-label-five-hour = 5s
sl-label-seven-day = 7g
sl-window = { $label } { PCT($pct) }
sl-resets-in = { DUR($ms, "compact") }
sl-eta = ▲{ DUR($ms, "compact") }
sl-session-share = ≈{ PCT($pct) } bu oturum
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
projection-reset-first = önce sıfırlanır
status-busy = meşgul
status-idle = boşta
status-unknown = bilinmiyor

data-from-app = TokenRadar uygulamasından canlı veri.
data-from-db = TokenRadar uygulaması çalışmıyor; son kaydedilen değerler gösteriliyor.
data-none = Henüz TokenRadar verisi yok. Oturumların indekslenmesi için uygulamayı bir kez başlatın.
no-limits = Henüz limit verisi yok.
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
settings-changed-meanwhile = { $path } beklerken değişti; hiçbir şey yazılmadı. Komutu yeniden çalıştırın.
settings-invalid = { $path } geçerli bir JSON nesnesi değil; hiçbir şey değiştirilmedi.

## Hatalar ve yardım

error = hata: { $message }
cli-help =
    TokenRadar - yapay zekâ kodlama ajanları için kullanım limitleri
    Kullanım: tokenradar <KOMUT> [SEÇENEKLER]
    Komutlar:
      status [--json]                     Limitler, canlı oturumlar ve veri kaynakları
      limits [--json]                     Güncel limit değerleri
      sessions [--json]                   Çalışan Claude Code oturumları
      statusline [--chain]                Claude Code statusline'ı (JSON'u stdin'den okur)
      statusline install [--chain|--no-chain] [--yes] [--config-dir <DİZİN>]
      statusline uninstall [--yes] [--config-dir <DİZİN>]
    Seçenekler:
      -h, --help       Yardımı göster
      -V, --version    Sürümü göster
