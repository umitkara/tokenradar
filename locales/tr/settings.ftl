# Ayarlar penceresi (Adım 11b): beş sekme, statusline diyaloğu ve silme diyaloğu.

settings-title = Ayarlar
settings-tabs = Ayar bölümleri
tab-general = Genel
tab-sources = Veri kaynakları
tab-statusline = Statusline
tab-notifications = Bildirimler
tab-privacy = Gizlilik ve veri
tab-about = Hakkında

settings-save-failed = Ayar kaydedilemedi. Tekrar deneyin.
settings-cancel = Vazgeç
settings-close = Kapat
# Dil adları iki dosyada da kendi dillerinde yazılır.
language-en = English
language-tr = Türkçe

## Boyutlar. Birimi pencere seçer; $n o birimdeki değerdir.

size-bytes = { NUM($n, 0) } bayt
size-kb = { NUM($n, 0) } KB
size-mb = { NUM($n, 1) } MB
size-gb = { NUM($n, 2) } GB

## Genel

general-language = Dil
general-language-system = Windows ile aynı
general-theme = Tema
general-theme-system = Windows ile aynı
general-theme-light = Açık
general-theme-dark = Koyu
general-autostart = Windows ile başlat
general-autostart-hint = Oturum açtığınızda TokenRadar tray'de başlar.
general-autostart-unknown = Başlangıç ayarı okunamadı.
general-warm = Kapatılan paneli hazır tut
general-warm-hint = Kapatılan panel bu süre boyunca bellekte kalır ve anında açılır. Kısa süre daha az bellek kullanır.
general-warm-off = Hemen boşalt
general-warm-option = { DUR($ms) }

## Veri kaynakları: Claude Code klasörleri

sources-dirs = Claude Code klasörleri
sources-dirs-hint = TokenRadar kullanım kayıtlarını bu klasörlerden okur. Klasörlerde hiçbir şeyi değiştirmez.
sources-dir-primary = Birincil
sources-dir-chosen = Sizin seçiminiz
sources-dir-env = CLAUDE_CONFIG_DIR'den
sources-dir-stats = { NUM($sessions, 0) } oturum · { NUM($projects, 0) } proje · { NUM($files, 0) } dosya
sources-dir-last = Son etkinlik { DUR($ms) } önce
sources-dir-version = Claude Code { $version }
sources-dir-pending = Henüz işlenmedi.
sources-dir-none = Claude Code klasörü bulunamadı. Bir klasör seçin ya da TokenRadar'ı başlatmadan önce CLAUDE_CONFIG_DIR değişkenini ayarlayın.
sources-dir-missing = Seçtiğiniz klasör artık yok; algılanan klasörler kullanılıyor.
sources-change = Klasör seç…
sources-reset-dir = Algılanan klasörü kullan
sources-change-note = Seçtiğiniz klasör birincil olur: panel onun limitlerini gösterir. Diğer klasörler izlenmeye devam eder.
sources-pick-not-dir = Seçilen öğe bir klasör değil.
sources-pick-not-claude = Bu klasörde projects klasörü de settings.json da yok. Claude Code klasörünü seçin (genellikle kullanıcı klasörünüzdeki .claude).
sources-pick-failed = Klasör kullanılamadı. Tekrar deneyin.

## Veri kaynakları: statusline köprüsü

statusline-title = Statusline köprüsü
statusline-hint = Claude Code canlı limit yüzdelerini statusline komutuna verir. Köprü kuruluysa TokenRadar tahmin yerine bu ölçülmüş değerleri gösterir.
statusline-recommended = Önerilen
statusline-no-directory = Önce bir Claude Code klasörü seçin.
statusline-not-installed = Kurulu değil.
statusline-current = Şu anki statusline komutunuz:
statusline-installed = Kurulu.
statusline-installed-chained = Kurulu. Önceki komutunuz da çalışmaya devam ediyor:
statusline-installed-elsewhere = Statusline olarak TokenRadar çalışıyor, ama buradan kurulmamış. Eklendiği yoldan kaldırın.
statusline-last-seen = Son okuma { DUR($ms) } önce
statusline-never-seen = Henüz okuma yok. Claude Code ilk okumayı bir sonraki yanıtıyla gönderir.
statusline-install = Kur…
statusline-uninstall = Kaldır…
statusline-cli-missing = Uygulamanın yanında tokenradar komutu bulunamadı; köprü buradan kurulamıyor.

statusline-dialog-install = Statusline köprüsünü kur
statusline-dialog-uninstall = Statusline köprüsünü kaldır
statusline-dialog-intro = TokenRadar aşağıdaki dosyayı değiştirecek. Siz onaylamadan hiçbir şey yazılmaz; önce bir yedek alınır.
statusline-diff = Değişiklikler
statusline-chain = Şu anki statusline komutumu koru
statusline-chain-hint = Kendi satırınız kalır, TokenRadar'ınki yanına eklenir.
statusline-chain-note = Komutunuz bugün ne yapıyorsa yapmaya devam eder; kendi ağ istekleri de buna dahildir. Örneğin usage API'sini çağıran bir betik onu çağırmayı sürdürür.
statusline-mangled = Claude Code bu komutu ters eğik çizgileri silen Git Bash ile çalıştırabilir. Kurulumdan sonra kendi satırınızın hâlâ göründüğünü kontrol edin.
statusline-restores = Önceki statusline komutunuz geri gelir.
statusline-confirm-install = Kur
statusline-confirm-uninstall = Kaldır
statusline-done = Tamamlandı. Eski dosyanın yedeği: { $path }
statusline-already = Köprü zaten kurulu.
statusline-nothing = Köprü kurulu değil; kaldırılacak bir şey yok.
statusline-error-no-directory = Claude Code klasörü bulunamadı.
statusline-error-cli-missing = Uygulamanın yanında tokenradar komutu bulunamadı.
statusline-error-invalid-settings = settings.json geçerli bir JSON değil, bu yüzden dokunulmadı. Bir düzenleyicide düzeltip tekrar deneyin.
statusline-error-changed-meanwhile = Bu pencere açıkken settings.json değişti. Hiçbir şey yazılmadı; yeni değişiklikleri gözden geçirin.
statusline-error-stale-plan = Bu değişiklikler artık güncel değil. Hiçbir şey yazılmadı; yeniden gözden geçirin.
statusline-error-io = Dosya yazılamadı. Hiçbir şey değişmedi.

## Veri kaynakları: Claude Code mod'u

mod-title = Claude Code içinde panel
mod-experimental = Deneysel
mod-hint = Her Claude Code oturumunda limitlerinizi gösteren bir yan panel, istemin üstünde bir uyarı bandı ve /radar komutu. Claude Code bunu uygulamanın klasöründen yükler; başka bir şey indirilmez.
mod-installed = Kurulu. Claude Code şuradan yüklüyor:
mod-missing = Mod dosyaları uygulamanın yanında değil; buradan kurulamıyor.
mod-dialog-install = Claude Code mod'unu kur
mod-dialog-uninstall = Claude Code mod'unu kaldır
mod-dialog-restart = Ardından Claude Code'u yeniden başlatın. Bu mod için daha önce --plugin-dir kullandıysanız artık kullanmayın, yoksa mod iki kez yüklenir.
mod-kept = { $count ->
    [one] Listedeki diğer bir eklenti klasörü yerinde kalır.
   *[other] Listedeki diğer { $count } eklenti klasörü yerinde kalır.
}
mod-already = Mod zaten kurulu.
mod-nothing = Mod kurulu değil; kaldırılacak bir şey yok.
mod-done-install = Kuruldu. Yüklenmesi için Claude Code'u yeniden başlatın.
mod-done-uninstall = Kaldırıldı. Devre dışı kalması için Claude Code'u yeniden başlatın.
mod-backup = Eski dosyanın yedeği: { $path }

## Statusline düzenleyicisi (Adım 12j)

statusline-look-moved = Satırın ne göstereceğini ve nasıl görüneceğini kendi sayfasında seçin.
statusline-edit-look = Görünümü düzenle →
statusline-editor-title = Statusline
statusline-autosave = Değişiklikler otomatik kaydedilir.
statusline-editor-not-installed = Köprü kurulu değil, bu yüzden Claude Code bu satırı henüz göstermiyor.
statusline-editor-to-sources = Veri kaynakları'ndan kurun
statusline-undo = Geri al
statusline-undo-hint = Son değişikliği geri al (Ctrl+Z)
statusline-restore-default = Varsayılana dön…
statusline-theme = Tema
statusline-theme-plain = Düz
statusline-theme-powerline = Powerline
statusline-theme-powerline-round = Yuvarlak
statusline-colors = Renkler
statusline-color-mode-auto = Otomatik
statusline-color-mode-always = Her zaman
statusline-color-mode-never = Kapalı
statusline-separator = Yeni ayraçlar
statusline-presets = Şablonlar…
statusline-export = Dışa aktar
statusline-exported = Panoya [statusline] tablosu olarak kopyalandı.
statusline-export-failed = Panoya yazılamadı. Tekrar deneyin.
statusline-import = İçe aktar…
statusline-preview = Örnek değerlerle önizleme
statusline-state = Örnek durum
statusline-state-normal = Normal
statusline-state-warn = Uyarı
statusline-state-crit = Kritik
statusline-state-stale = Bayat
statusline-width = Terminal genişliği
statusline-columns = { $n } sütun
statusline-font-note = Powerline glifleri terminalinizde bir Nerd Font ya da Powerline fontu gerektirir. Başka fontlarda hem burada hem Claude Code'da kutu olarak görünür.
statusline-select-hint = Değiştirmek için bir öğe seçin.

statusline-items = Öğeler, sırayla
statusline-item-hidden = gizli
statusline-segment-absent = bu hesapta yok
statusline-reorder = { $name } öğesini taşı: sürükleyin ya da ↑ ve ↓ tuşlarına basın
statusline-moved = { $name }, { $count } öğe içinde { $position }. sıraya taşındı.
statusline-hide = Gizle
statusline-show = Göster
statusline-remove = Kaldır
statusline-add = Öğe ekle
statusline-move-up = Yukarı taşı
statusline-move-down = Aşağı taşı

statusline-add-title = Öğe ekle
statusline-add-used = satırda
statusline-add-full = Satırda en fazla { $max } öğe olabilir.
statusline-group-limit = Limitler
statusline-group-session = Oturum
statusline-group-project = Proje
statusline-group-layout = Düzen

statusline-kind-five-hour = 5 saatlik pencere
statusline-kind-five-hour-hint = 5 saatlik pencerenin ne kadarının kullanıldığı ve reset'e kalan süre.
statusline-kind-seven-day = Haftalık limit
statusline-kind-seven-day-hint = Haftalık limitin ne kadarının kullanıldığı ve reset'e kalan süre.
statusline-kind-scoped = Model limitleri
statusline-kind-scoped-hint = Bir modelin ya da yüzeyin haftalık limitleri, örneğin Fable'ınki. Claude Code bildirdiğinde gösterilir.
statusline-kind-context = Bağlam penceresi
statusline-kind-context-hint = Bu oturumun bağlam penceresinin ne kadar dolu olduğu.
statusline-kind-session-share = Bu oturumun payı
statusline-kind-session-share-hint = Bu oturumun 5 saatlik pencereden ne kadar kullandığı. TokenRadar bilene kadar gizlidir.
statusline-kind-cost = API karşılığı maliyet
statusline-kind-cost-hint = Bu oturumun API fiyatlarıyla ne kadar tutacağı. Fatura değildir, bu yüzden API işareti kalır.
statusline-kind-model = Model
statusline-kind-model-hint = Bu oturumun çalıştığı model.
statusline-kind-tokens-in = Girdi token'ları
statusline-kind-tokens-in-hint = Bu oturumda modele gönderilen token'lar.
statusline-kind-tokens-out = Çıktı token'ları
statusline-kind-tokens-out-hint = Bu oturumda modelin yazdığı token'lar.
statusline-kind-duration = Oturum süresi
statusline-kind-duration-hint = Bu oturumun ne kadar süredir çalıştığı.
statusline-kind-api-duration = API süresi
statusline-kind-api-duration-hint = Bu oturumun modeli ne kadar beklediği.
statusline-kind-lines-changed = Değişen satırlar
statusline-kind-lines-changed-hint = Bu oturumda eklenen ve silinen satırlar.
statusline-kind-vim-mode = Vim modu
statusline-kind-vim-mode-hint = Vim modu açıksa editörün Vim kipi.
statusline-kind-version = Claude Code sürümü
statusline-kind-version-hint = Bu oturumu çalıştıran Claude Code sürümü.
statusline-kind-agent-name = Ajan
statusline-kind-agent-name-hint = Oturum bir ajan çalıştırıyorsa ajanın adı.
statusline-kind-session-id = Oturum kimliği
statusline-kind-session-id-hint = Claude Code'un bu oturuma verdiği kimlik.
statusline-kind-cwd = Çalışma klasörü
statusline-kind-cwd-hint = Claude Code'un çalıştığı klasör.
statusline-kind-project-dir = Proje klasörü
statusline-kind-project-dir-hint = Oturumun başlatıldığı klasör.
statusline-kind-git-branch = Git dalı
statusline-kind-git-branch-hint = Projede çıkış yapılmış dal. git çalıştırılmadan depodan okunur.
statusline-kind-separator = Ayraç
statusline-kind-separator-hint = Öğeler arasında sizin seçtiğiniz bir glif.
statusline-kind-line-break = Satır sonu
statusline-kind-line-break-hint = Yeni bir satır başlatır. Sonraki öğeler alt satırda gösterilir.
statusline-kind-custom-text = Özel metin
statusline-kind-custom-text-hint = Sizin seçtiğiniz sabit bir metin.
statusline-kind-space = Boşluk
statusline-kind-space-hint = İki öğe arasında boş sütunlar.
statusline-kind-flex = Esnek boşluk
statusline-kind-flex-hint = Boş genişliği doldurur. Sonraki öğeler sağa yaslanır; iki esnek boşluğun arası ortalanır.

statusline-absent-note = Bu hesapta 5 saatlik pencere yok, bu yüzden öğe satıra girmiyor. Pencere geri gelirse öğe de görünür.
statusline-hidden-note = Gizli: ayarlarıyla birlikte duruyor ama gösterilmiyor.
statusline-format = Biçim
statusline-format-bar = Çubuk
statusline-format-percent = Yüzde
statusline-format-fraction = Kesir
statusline-format-two-decimals = 0,00
statusline-format-four-decimals = 0,0000
statusline-format-short = Kısa
statusline-format-full = Tam
statusline-format-compact = Kısaltılmış
statusline-format-hms = sa:dk:sn
statusline-format-both = Eklenen ve silinen
statusline-format-net = Net
statusline-format-basename = Klasör adı
statusline-format-home = ~ yolu
statusline-label = Etiket
statusline-label-none = Etiket yok
statusline-label-hint = Boş bırakılırsa kutuda soluk görünen varsayılan etiket kullanılır.
statusline-glyph = Glif
statusline-text = Metin
statusline-color = Renk
statusline-color-auto = Otomatik
statusline-color-threshold-note = Bu, %75'in altındaki renktir. Burada ne seçerseniz seçin, değer %75'te sarıya, %90'da kırmızıya döner. Tahminler ve bayat değerler gri kalır.
statusline-color-black = Siyah
statusline-color-red = Kırmızı
statusline-color-green = Yeşil
statusline-color-yellow = Sarı
statusline-color-blue = Mavi
statusline-color-magenta = Macenta
statusline-color-cyan = Camgöbeği
statusline-color-white = Beyaz
statusline-color-gray = Gri
statusline-color-bright-red = Parlak kırmızı
statusline-color-bright-green = Parlak yeşil
statusline-color-bright-yellow = Parlak sarı
statusline-color-bright-blue = Parlak mavi
statusline-color-bright-magenta = Parlak macenta
statusline-color-bright-cyan = Parlak camgöbeği
statusline-color-bright-white = Parlak beyaz
statusline-reset-countdown = Reset geri sayımı
statusline-pace = Zaman işareti
statusline-pace-hint = Pencerenin zaman olarak ne kadarının geçtiği: çubukta │, değerin yanında ⏱. Kullanımınızla karşılaştırın.

statusline-presets-title = Şablonlar
statusline-presets-intro = Bir şablon bütün öğeleri, temayı ve renkleri değiştirir. Geri al satırınızı geri getirir.
statusline-presets-confirm = Uygula
statusline-default-title = Varsayılan satıra dön
statusline-default-intro = Öğeleriniz, temanız ve renkleriniz varsayılan satırla değiştirilir. Geri al hepsini geri getirir.
statusline-default-confirm = Varsayılana dön
statusline-preset-default = Varsayılan
statusline-preset-default-hint = Limitler, bağlam, bu oturumun payı ve maliyet.
statusline-preset-minimal = Sade
statusline-preset-minimal-hint = Yalnızca limitler.
statusline-preset-detailed = Ayrıntılı
statusline-preset-detailed-hint = İlk satırda limitler ve oturum, ikinci satırda proje.
statusline-preset-two-line = İki satır
statusline-preset-two-line-hint = Önce limitler, altta oturum.
statusline-preset-powerline = Powerline
statusline-preset-powerline-hint = Renkli bloklar. Nerd Font gerektirir.
statusline-preset-split = İki yana
statusline-preset-split-hint = Limitler solda; model, klasör ve dal sağda.

statusline-import-title = Statusline içe aktar
statusline-import-intro = Bir [statusline] tablosu yapıştırın, örneğin başka bir bilgisayardan dışa aktarılmış olanı. Uygulayana kadar hiçbir şey değişmez.
statusline-import-read = Oku
statusline-import-error = Bu geçerli bir TOML tablosu değil. Ayrıştırıcının mesajı:
statusline-import-preview = Nasıl görünüyor
statusline-import-ignored = { $count } anahtar atlandı:
statusline-import-apply = Uygula

## Veri kaynakları: usage API

api-title = Usage API
api-hint = Statusline köprüsü yoksa TokenRadar limitlerinizi Anthropic'e sorabilir. Siz izin verene kadar kapalı kalır.
api-what-label = Ne olur
api-what = TokenRadar birkaç dakikada bir api.anthropic.com'dan güncel limit yüzdelerinizi ister. Bu, Claude Code'un yaptığı isteğin aynısıdır.
api-reads-label = Ne okunur
api-reads = Claude Code'un .credentials.json dosyasındaki erişim token'ı, yalnızca istek anında. Hiçbir zaman saklanmaz, loglanmaz, gösterilmez.
api-never-label = Ne asla okunmaz
api-never = Refresh token. TokenRadar oturumunuzu asla yenilemez veya değiştirmez. Token'ın süresi dolunca Claude Code'un yenilemesini bekler.
api-where-label = Nereye gider
api-where = Yalnızca HTTPS ile api.anthropic.com'a. Bilgisayarınızdan başka hiçbir şey çıkmaz.
api-revoke = İstediğiniz zaman kapatabilirsiniz; istekler hemen durur.
api-allow = API erişimine izin ver
api-turn-off = API erişimini kapat
api-allowed = { DUR($ms) } önce izin verildi.
api-outdated = Bu açıklama siz onay verdikten sonra değişti; API erişimi bu yüzden kapalı. Yeniden okuyup izin verirseniz tekrar açılır.
api-state-off = Kapalı: API çağrılmıyor.
api-state-active = Etkin.
api-state-token-stale = Claude Code'un oturumu yenilemesi bekleniyor.
api-state-backoff = Sunucu isteği erteletti; sonraki deneme { DUR($ms) } sonra.
api-state-offline = Çevrimdışı; sonraki deneme { DUR($ms) } sonra.
api-state-error = Son yanıt kullanılamadı; sonraki deneme { DUR($ms) } sonra.
api-last-success = Son başarılı istek { DUR($ms) } önce.
api-requests = Başlangıçtan beri { NUM($requests, 0) } istek, { NUM($errors, 0) } hata.
api-last-error = Son hata: { $category }
api-status-unknown = Durum, TokenRadar başladığında gösterilir.

## Bildirimler

notify-enabled = Bildirimleri göster
notify-five = 5 saatlik pencere
notify-weekly = Haftalık limitler
notify-thresholds-hint = Kullanım her yüzdeyi geçtiğinde, pencere başına bir kez bildirim. En fazla { $max } eşik.
notify-threshold-remove = { PCT($pct) } eşiğini kaldır
notify-threshold-new = Yüzde olarak yeni eşik
notify-threshold-add = Ekle
notify-threshold-invalid = 1 ile 100 arasında bir tam sayı girin.
notify-threshold-full = En fazla { $max } eşik eklenebilir.
notify-projection = Limit reset'ten önce dolacaksa uyar
notify-projection-hint = Mevcut hız limiti bir saat içinde dolduracaksa; pencere başına en fazla bir kez.
notify-final = Son uyarı
notify-final-off = Kapalı
notify-final-option = Dolmadan { DUR($ms) } önce
notify-reset = Reset'leri bildir
notify-reset-hint = Yalnızca en az bu seviyeye çıkmış bir pencereden sonra; böylece sakin geçen her pencere için bildirim gelmez.
notify-reset-always = Her pencereden sonra
notify-reset-option = { PCT($pct) } sonrası
notify-quiet = Sessiz saatler
notify-quiet-hint = Bildirimler kaydedilir ama gösterilmez. Hâlâ geçerli olanlar sonra tek seferde gösterilir.
notify-quiet-from = Başlangıç
notify-quiet-to = Bitiş
notify-test = Test bildirimi gönder
notify-test-sent = Gönderildi. Bir şey görünmediyse Windows ayarlarında TokenRadar bildirimlerini kontrol edin.

## Gizlilik ve veri

privacy-storage = Depolama
privacy-data-dir = Veri klasörü
privacy-portable = Taşınabilir: veriler uygulamanın yanında durur.
privacy-db = Veritabanı
privacy-logs = Log dosyaları
privacy-retention = Ayrıntılı kullanım kayıtlarının saklanma süresi
privacy-retention-hint = Daha eski kayıtlar silinir. 5 dakikalık özetler saklanır; limitler ve geçmiş bozulmaz.
privacy-retention-days = { NUM($days, 0) } gün
privacy-network = Ağ
privacy-network-intro = TokenRadar yalnızca şu adreslere bağlanır:
privacy-network-api = api.anthropic.com: limit yüzdeleriniz, yalnızca izninizle.
privacy-network-github = github.com: yeni sürümleri denetlemek için (sonraki bir sürümden itibaren).
privacy-network-on = Açık
privacy-network-off = Kapalı
privacy-network-none = Telemetri yok, analitik yok, başka hiçbir şey yok.
privacy-delete = TokenRadar verisini sil
privacy-delete-hint = TokenRadar'ın veritabanını, loglarını ve ayarlarını siler. Claude Code dosyalarına dokunulmaz.
privacy-delete-button = Tüm verileri sil…
privacy-delete-title = Tüm TokenRadar verisi silinsin mi?
privacy-delete-what = Silinenler: kullanım veritabanı, bekleyen statusline okumaları, log dosyaları ve TokenRadar ayarlarınız.
privacy-delete-kept = Kalanlar: Claude Code'un tüm dosyaları ve köprünün temizce kaldırılabilmesi için statusline kurulum kaydı. Ardından TokenRadar oturumlarınızı baştan okur.
privacy-delete-check = Bunun geri alınamayacağını anlıyorum.
privacy-delete-confirm = Her şeyi sil
privacy-delete-done = TokenRadar verisi silindi.
privacy-delete-partial = Bazı dosyalar silinemedi; kullanımda olabilirler. Diğer TokenRadar pencerelerini kapatıp tekrar deneyin.
privacy-diagnostics = Tanılama paketi
privacy-diagnostics-hint = Hata bildirimine eklenecek bir zip dosyası: sürümler, sayılar, boyutlar ve son 200 log satırı. Klasör yolları kısa kodlarla değiştirilir. Token'lar, konuşma içeriği, oturum başlıkları ve proje adları asla eklenmez. Hiçbir şey gönderilmez; dosyanın nereye gideceğine siz karar verirsiniz.
privacy-diagnostics-button = Tanılama paketi oluştur…
privacy-diagnostics-done = Kaydedildi: { $path }
privacy-diagnostics-failed = Paket oluşturulamadı.
privacy-log-level = Log ayrıntısı
log-level-error = Yalnızca hatalar
log-level-warn = Uyarılar
log-level-info = Normal
log-level-debug = Ayrıntılı
log-level-trace = Her şey

## Hakkında

about-version = Sürüm { $version }
about-tagline = Yapay zekâ kodlama ajanlarının kullanım limitlerini izler. Yalnızca yerel, telemetri yok.
about-updates = Güncellemeler
about-update-check = GitHub'da yeni sürümleri denetle
about-update-check-hint = Günde bir kez, yalnızca GitHub Releases'e bağlanır. Siz tıklamadan hiçbir şey kurulmaz.
about-update-now = Şimdi denetle
about-update-install = Kur ve yeniden başlat
about-update-checking = Güncellemeler denetleniyor…
about-update-current = TokenRadar güncel.
about-update-available = { $version } sürümü çıktı.
about-update-notes = Sürüm notları
about-update-downloading = Güncelleme indiriliyor…
about-update-downloading-pct = Güncelleme indiriliyor: { PCT($pct) }
about-update-check-failed = Denetim başarısız oldu. GitHub'a ulaşılamıyor olabilir.
about-update-install-failed = Güncelleme kurulamadı. Yeniden deneyin ya da sürümler sayfasından indirin.
about-update-portable = Bu taşınabilir bir kopya: yeni zip dosyasını sürümler sayfasından indirin.
about-update-checked = Son denetim { DUR($ms) } önce.
about-releases = Sürümler sayfası
about-license = Lisans
about-license-text = TokenRadar, MIT lisansıyla açık kaynaklıdır.
about-third-party = TokenRadar, kendi lisanslarıyla dağıtılan açık kaynak bileşenler içerir.
about-third-party-open = Üçüncü parti lisanslar
about-third-party-missing = Üçüncü parti lisansların listesi yayın sürümleriyle birlikte gelir.
about-repo = Kaynak kodu
about-issues = Sorun bildir
about-link-failed = Sayfa açılamadı.
