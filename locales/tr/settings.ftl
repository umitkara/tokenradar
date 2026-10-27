# Ayarlar penceresi (Adım 11b): beş sekme, statusline diyaloğu ve silme diyaloğu.

settings-title = Ayarlar
settings-tabs = Ayar bölümleri
tab-general = Genel
tab-sources = Veri kaynakları
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
about-update-later = Güncelleme denetimi sonraki bir sürümde gelecek. O zamana kadar yeni sürümler sürümler sayfasında.
about-releases = Sürümler sayfası
about-license = Lisans
about-license-text = TokenRadar, MIT lisansıyla açık kaynaklıdır.
about-third-party = Üçüncü parti lisansların listesi yayın sürümleriyle birlikte gelir.
about-repo = Kaynak kodu
about-issues = Sorun bildir
about-link-failed = Sayfa açılamadı.
