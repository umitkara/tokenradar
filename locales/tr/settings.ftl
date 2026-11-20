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

## Genel: masaüstü widget'ı (Adım 18)

general-widget = Masaüstü widget'ı
general-widget-enabled = Widget'ı göster
general-widget-enabled-hint = Limit çubuklarını gösteren, hep üstte duran küçük bir şerit. İstediğiniz yere sürükleyin; çift tıklama paneli açar, sağ tıklama menüsünü gösterir.
general-widget-opacity = Arka plan opaklığı
general-widget-opacity-option = { PCT($pct) }
general-widget-pace = Zaman işareti
general-widget-pace-hint = Her çubuktaki işaret, pencerenin süresinin ne kadarının geçtiğini gösterir.
general-widget-fullscreen = Tam ekran uygulamalarda gizle
general-widget-fullscreen-hint = Oyunların, videoların ve sunumların üstüne çıkmaz.
general-widget-click-through = Tıklama geçirgenliği
# $hotkey kısayoldur, örneğin "Ctrl+Alt+Shift+R".
general-widget-click-through-hint = Tıklamalar alttaki pencerelere gider. { $hotkey } ya da tray menüsü widget'ı yeniden tıklanabilir yapar.
general-widget-hotkey = Tıklama geçirgenliği kısayolu
general-widget-hotkey-hint = Widget görünürken her yerde çalışır. Tıklayın, sonra tuşlara basın.
general-widget-hotkey-recording = Tuşlara basın…
general-widget-hotkey-invalid = Ctrl, Alt ya da Win ile bir harf, rakam veya F1–F24 kullanın. Esc vazgeçer.

## Genel: widget'ın görev çubuğu modu (Adım 18b)

general-widget-mode = Konum
general-widget-mode-floating = Yüzen
general-widget-mode-taskbar = Görev çubuğunda
general-widget-mode-taskbar-hint = Şerit görev çubuğunda, bildirim alanının solunda durur. Görev çubuğu yanda ya da üstteyse, kendiliğinden gizleniyorsa veya yer yoksa widget yüzer.
general-widget-align = Hizalama
general-widget-align-tray = Bildirim alanının yanında
general-widget-align-left = Sol uçta
general-widget-offset = Uzaklık
general-widget-offset-hint = Şeridin hizalandığı yerden ne kadar uzak durduğu. Şeridi görev çubuğu boyunca sürüklemek de bunu ayarlar.
# $px mantıksal piksel cinsinden bir uzaklıktır.
general-widget-offset-option = { $px } px
general-widget-fullscreen-docked-hint = Görev çubuğundayken widget tam ekran uygulamalarda her zaman gizlenir.

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
statusline-refresh = Canlı yenileme
statusline-refresh-off = Kapalı
statusline-refresh-every = { $n } sn
statusline-refresh-cost = Bir oturum açıkken, boşta olsa bile her { $n } saniyede bir TokenRadar çalışır.
statusline-refresh-off-hint = Satır yalnızca bir mesajdan sonra yeniden çizilir. Arada geri sayımlar durur.
statusline-refresh-differs = settings.json { $n } sn diyor; Claude Code dosyaya uyar.
statusline-refresh-differs-off = settings.json'da canlı yenileme yok; Claude Code dosyaya uyar.
statusline-refresh-saved = Kaydedildi.
statusline-dialog-refresh = Canlı yenilemeyi değiştir
statusline-confirm-refresh = Değiştir
statusline-refresh-chain-note = Zincirlenmiş komutunuz da her yenilemede çalışır.
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
statusline-icons = Simgeler
statusline-icons-text = Metin
statusline-icons-unicode = Unicode
statusline-icons-nerd = Nerd Font
statusline-icons-suggest = Powerline bir Nerd Font ister. Aynı font etiketlerin yerine simge de çizebilir.
statusline-icons-use-nerd = Nerd Font simgelerini kullan
statusline-icons-no-font = Nerd Font simgeleri bir Nerd Font ister ve kurulu bir Nerd Font yok. Başka fontlarda hem burada hem Claude Code'da kutu olarak görünürler.
statusline-icons-not-mono = Kurulu Nerd Font bir Mono varyantı değil; bir simge yandaki hücreye taşabilir.
statusline-presets = Şablonlar…
statusline-export = Dışa aktar: satırı [statusline] tablosu olarak kopyala
statusline-exported = Panoya [statusline] tablosu olarak kopyalandı.
statusline-export-failed = Panoya yazılamadı. Tekrar deneyin.
statusline-import = Bir [statusline] tablosu içe aktar…
statusline-preview = Örnek değerlerle önizleme
statusline-state = Örnek durum
statusline-state-normal = Normal
statusline-state-warn = Uyarı
statusline-state-crit = Kritik
statusline-state-stale = Bayat
statusline-width = Terminal genişliği
statusline-columns = { $n } sütun
statusline-font-note = Bu tema bir Nerd Font ister ve kurulu bir Nerd Font yok. Başka fontlarda glifleri hem burada hem Claude Code'da kutu olarak görünür.
statusline-margin = Sağ pay
statusline-margin-hint = Satır gerçek terminalde sağdan taşıyorsa artırın.
statusline-reserved = Satır son { $n } sütunu boş bırakır.
statusline-align-note = Gerçek terminalde iki hücre kaplayan glifler hizayı kaydırabilir.
statusline-select-hint = Değiştirmek için bir öğe seçin.

statusline-item-hidden = gizli
statusline-segment-absent = bu hesapta yok
statusline-reorder = { $name } öğesini taşı: sürükleyin ya da ↑ ve ↓ tuşlarına basın
statusline-hide = Gizle
statusline-show = Göster
statusline-remove = Kaldır
statusline-add = Öğe ekle
statusline-line = Satır { $n }
statusline-line-items = Satır { $n } öğeleri, sırayla
statusline-line-remove = { $n }. satırı öğeleriyle birlikte kaldır
statusline-line-removed = { $n }. satır kaldırıldı. Geri al ile geri gelir.
statusline-line-add = Satır ekle
statusline-line-added = { $n }. satır eklendi.
statusline-line-empty = Boş satır. Bir öğe ekleyin ya da buraya sürükleyin.
statusline-add-to-line = { $n }. satıra öğe ekle
statusline-moved-line = { $name } taşındı: satır { $line }, { $count } öğe içinde { $position }. sıra.
statusline-move-up = Yukarı taşı
statusline-move-down = Aşağı taşı

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
statusline-kind-daily-budget = Günlük bütçe
statusline-kind-daily-budget-hint = Haftalık limitten kalanın sıfırlanmaya kadar kalan günlere bölümü. Kullanımınız zamanın önüne geçince sarıya döner.
statusline-kind-burn-rate = Tüketim hızı
statusline-kind-burn-rate-hint = Bu oturumun saat başına API karşılığı maliyeti. İlk beş dakikadan sonra gösterilir.
statusline-kind-session-name = Oturum adı
statusline-kind-session-name-hint = Bu Claude Code oturumunun adı, verilmişse.
statusline-kind-cache-ratio = Cache oranı
statusline-kind-cache-ratio-hint = Son istekte girdinin ne kadarının prompt cache'ten okunduğu.
statusline-kind-output-style = Output style
statusline-kind-output-style-hint = Bu oturumun kullandığı output style.
statusline-kind-compact = Compact sayısı
statusline-kind-compact-hint = Bu oturumda kaç kez compact yapıldığı ve Claude Code ayarlarında tanımladıysanız auto-compact penceresi. Sayı için uygulamanın açık olması gerekir.
statusline-kind-effort = Efor
statusline-kind-effort-hint = Oturumun /effort ile ayarlanan akıl yürütme eforu. Tek bir istemdeki "ultrathink" gibi sözcükler ve alt ajanların kendi eforu bunu değiştirmez. Efor düzeyi olmayan modellerde gizlenir.
statusline-kind-cache-timer = Cache süresi
statusline-kind-cache-timer-hint = İstek gelmezse prompt cache'in ne zaman soğuyacağı: yerel saat ya da kalan süre olarak. Sonraki istekler cache'i yeniden yazar.
statusline-kind-cache-misses = Cache kaçırmaları
statusline-kind-cache-misses-hint = Bu oturumda kaç isteğin prompt cache'i kaçırdığı ve sonuncusunun muhtemel nedeni.
statusline-kind-recache-cost = Yeniden cache maliyeti
statusline-kind-recache-cost-hint = Cache soğuduysa sonraki isteğin yeniden yazacağı token'lar. Compact'tan hemen sonra gizlenir.
statusline-kind-spend-limit = Harcama limiti
statusline-kind-spend-limit-hint = Harcama limitinin ne kadarının kullanıldığı. Yalnızca limit koyan bir Claude apps ağ geçidinin arkasında görünür; dolar tutarı birkaç dakika geriden gelebilir.
statusline-kind-pr = Pull request
statusline-kind-pr-hint = Geçerli dalın açık pull request'i (ya da GitLab merge request'i) ve inceleme durumu.
statusline-kind-worktree = Worktree
statusline-kind-worktree-hint = Oturumun çalıştığı bağlı git worktree'si. Ana çalışma ağacında gizlenir.
statusline-kind-repo = Depo
statusline-kind-repo-hint = Deponun sahibi ve adı, origin remote'undan okunur.
statusline-kind-git-status = Git durumu
statusline-kind-git-status-hint = Hazırlanmış (+), değişmiş (~) ve izlenmeyen (?) dosyalar, çakışmalar (!) ve upstream'e göre ileride ve geride olan commit'ler. Bunun için git'i uygulama çalıştırır, en fazla birkaç saniyede bir; uygulama çalışmıyorsa gizlenir.
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
statusline-format-remaining = Kalan
statusline-format-tokens-left = Kalan token
statusline-format-total = Toplam
statusline-format-split = Oto ve elle
statusline-format-level = Düzey
statusline-format-flags = Hızlı ve düşünme ile
statusline-format-count = Sayı
statusline-format-cause = Nedeniyle
statusline-format-amount = Dolar
statusline-format-name = Ad
statusline-format-host = Sunucuyla
statusline-format-with-effort = Eforla
statusline-format-last = Son istek
statusline-format-session = Tüm oturum
statusline-format-with-added = Eklenen klasörlerle
statusline-format-clock = Saat
statusline-format-countdown = Geri sayım
statusline-format-dirty = Değişiklik işareti
statusline-format-ahead-behind = İleride ve geride
statusline-untracked = İzlenmeyen dosyaları say
statusline-untracked-hint = Çok büyük depolarda kapatın: git izlenmeyen dosya taramasını atlar ve satır sayı yerine ?? gösterir. Git çok yavaş kalırsa uygulama da bunu kendiliğinden kapatır.
statusline-countdown-frozen = Canlı yenileme kapalıysa, oturum boştayken geri sayım durur: Claude Code satırı yalnızca bir mesajdan sonra çizer. Canlı yenilemeyi açın ya da saati kullanın.
statusline-label = Etiket
statusline-label-none = Etiket yok
statusline-label-hint = Boş bırakılırsa kutuda soluk görünen varsayılan etiket kullanılır.
statusline-glyph = Glif
statusline-apply-all = Tüm ayraçlara uygula
statusline-apply-all-hint = Sonra eklenen ayraçlar da bu glifi alır.
statusline-space-width = Genişlik
statusline-flex-right = Sonrası sağa yaslanır.
statusline-flex-center = Ortadaki grup ortalanır.
statusline-flex-spread = Boş genişlik aralarında paylaştırılır.
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
statusline-reset-format = Sıfırlanma
statusline-reset-format-countdown = Geri sayım
statusline-reset-format-clock = Saat
statusline-bar-width = Çubuk genişliği
statusline-bar-style = Çubuk stili
statusline-bar-style-blocks = Blok
statusline-bar-style-thin = İnce
statusline-bar-style-dots = Nokta
statusline-bar-style-capsule = Kapsül
statusline-bar-style-solid = Dolgu
statusline-capsule-no-font = Kapsül bir Nerd Font (3.4 ya da sonrası) ister ve kurulu bir Nerd Font yok. Başka fontlarda kutu olarak görünür.
statusline-capsule-old-font = Kapsül Nerd Fonts 3.4 ya da sonrasını ister. Kurulu font daha eski ve kutu çiziyor.
statusline-bar-in-blocks = Powerline blokları yalnızca kapsülü çizer, bu çubuk gösterilmez.
statusline-pace = Zaman işareti
statusline-pace-hint = Pencerenin zaman olarak ne kadarının geçtiği: çubukta bir işaret, değerin yanında ⏱. Kullanımınızla karşılaştırın.

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
notify-projection-hint = Mevcut hız limiti bir saat içinde dolduracaksa (haftalık limitlerde: bir gün içinde ve hız iyice belliyse); pencere başına en fazla bir kez.
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

## Dönemsel özetler (Adım 17b)

notify-digest-weekly = Haftalık özet
notify-digest-weekly-hint = Geçen limit haftası: tepe değer, limite takılma, API karşılığı maliyet, en çok etkileyen proje ve önceki haftaya göre fark. Tıklanınca Genel Bakış açılır. Uygulama o an kapalıysa iki gün içinde açıldığında gelir.
notify-digest-when = Gönderim
notify-digest-after-reset = Haftalık reset'ten bir saat sonra
notify-digest-custom = Seçtiğim gün ve saatte
notify-digest-day = Gün
notify-digest-time = Saat
notify-digest-preview = Geçen haftanın özetini şimdi göster
notify-digest-preview-none = Henüz yenilenmiş bir haftalık pencere yok.
notify-digest-preview-failed = Özet oluşturulamadı.
notify-digest-daily = Günlük özet
notify-digest-daily-hint = Dünün kullanımı: haftalık limitten ≈ puan, API karşılığı maliyet, oturum sayısı ve en çok kullanılan proje.

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

## Terminal fontu yardımcısı (Adım 12o)

font-open = Terminal fontu…
font-title = Terminal fontu
font-intro = Terminaliniz statusline'ı kendi fontuyla çizer. TokenRadar bu fontu kendisi değiştiremez, ama bir Nerd Font kurup Windows Terminal ve VS Code'da seçebilir.
font-preview-with = Önizleme { $family } ile çiziliyor; terminaliniz bu fontu seçince aynısını görürsünüz.
font-preview-restart = { $family } kurulu. Önizleme onu TokenRadar yeniden başlayınca çizer; programlar yeni bir fontu ancak açılırken görür.
font-section-font = Nerd Font
font-status-none = Kurulu bir Nerd Font yok. Powerline ayraçları ve simgeler kutu olarak görünür.
font-status-old = { $family } kurulu, ama Nerd Fonts 3.4'ten eski; yeni ilerleme glifleri yok.
font-status-full = { $family } kurulu.
font-wide-icons = Bu varyantta simgeler bir hücreden geniş, sütunlar kayabilir. "Mono" varyantı simgeleri tek hücrede tutar.
font-download = İndir ve kur…
font-consent = TokenRadar bu dosyayı yalnızca siz onaylayınca GitHub'dan indirir. Dosyayı sabitlenmiş bir SHA-256 ile doğrular ve dört font dosyasını yalnızca sizin Windows hesabınıza kurar. Yönetici izni gerekmez.
font-download-confirm = İndir ve kur
font-downloading = İndiriliyor
font-progress = %{ $percent }
font-installed = { $family } kuruldu. Aşağıdan terminalinizde seçin.
font-install-unsupported = Bu sistemde TokenRadar'dan font kurmak henüz mümkün değil.
font-section-terminal = Terminal
font-terminal-seen = Statusline en son { $terminal } içinden çağrıldı.
font-terminal-unknown = TokenRadar açıldığından beri statusline çağrısı görülmedi. Claude Code'u bir kez kullanıp burayı yeniden açın.
font-terminal-windows-terminal = Windows Terminal
font-terminal-vscode = VS Code
font-terminal-other = başka bir terminal
font-section-settings = Terminal ayarları
font-no-targets = Windows Terminal ya da VS Code ayarı bulunamadı.
font-target-windows-terminal = Windows Terminal
font-target-windows-terminal-preview = Windows Terminal Preview
font-target-windows-terminal-unpackaged = Windows Terminal (taşınabilir)
font-target-vscode = VS Code
font-target-vscode-insiders = VS Code Insiders
font-in-use = statusline burada
font-current = Font: { $font }
font-current-default = Font: { $font } (terminalin varsayılanı)
font-tier-none = Nerd Font değil
font-tier-old = Nerd Font, 3.4 öncesi
font-tier-full = Nerd Font
font-overrides = { $count ->
    [one] Bir profil kendi fontunu belirliyor ve onu korur.
   *[other] { $count } profil kendi fontunu belirliyor ve onu korur.
}
font-problem-unreadable = Ayar dosyası okunamadı.
font-problem-invalid = Ayar dosyası geçerli bir JSON değil, TokenRadar ona dokunmaz.
font-problem-legacy = Bu dosya Windows Terminal'in eski biçiminde. Fontu terminalin kendi ayarlarından seçin.
font-select = Fontu seç
font-undo = Geri al
font-back = Geri
font-confirm-select = Fontu seç
font-confirm-undo = Değişikliği geri al
font-restart-hint = Ardından terminalin tüm pencerelerini kapatıp yeniden açın: programlar yeni kurulan bir fontu ancak açılırken görür.
font-selected = Font { $terminal } içinde seçildi.
font-undone = Önceki font { $terminal } içinde geri geldi.
font-already-selected = Bu font orada zaten seçili.
font-nothing-to-undo = Orada geri alınacak bir şey yok.
font-other-title = Diğer terminaller
font-other-body = Terminalin ayarlarında font olarak "{ $family }" seçin. JetBrains IDE'leri: Settings → Tools → Terminal. Konsol penceresi: başlık çubuğu → Özellikler → Yazı tipi.
font-error-busy = Zaten bir indirme sürüyor.
font-error-cancelled = İndirme iptal edildi.
font-error-connect = GitHub'a bağlanılamadı. Bağlantınızı ya da proxy ayarını kontrol edin.
font-error-timeout = İndirme zaman aşımına uğradı.
font-error-status = GitHub { $code } durum koduyla yanıt verdi.
font-error-request = İndirme başarısız oldu.
font-error-checksum = Dosya beklenen sağlama toplamıyla eşleşmiyor. Hiçbir şey kurulmadı.
font-error-archive = İndirilen arşiv okunamadı. Hiçbir şey kurulmadı.
font-error-unsupported = Bu sistemde kullanılamıyor.
font-error-no-font = Önce bir Nerd Font kurun.
font-error-invalid-settings = Ayar dosyası geçerli bir JSON değil. Hiçbir şey yazılmadı.
font-error-legacy-settings = Bu dosya Windows Terminal'in eski biçiminde. Hiçbir şey yazılmadı.
font-error-changed-meanwhile = Dosya, diff gösterildikten sonra değişti. Hiçbir şey yazılmadı. Güncel değişiklik aşağıda.
font-error-stale-plan = Bu değişiklik zaten uygulandı ya da yenisiyle değişti.
font-error-io = Dosya okunamadı ya da yazılamadı.
font-section-restore = Hepsini geri al
font-restore-body = TokenRadar'ın değiştirdiği terminal fontlarını eski haline getirir ve kurduğu fontu kaldırır.
font-restore = Eski haline getir…
font-restore-intro = Bu işlem her şeyi TokenRadar'dan önceki haline getirir:
font-restore-default = { $font } (terminalin varsayılanı)
font-restore-files = Kurulan fontu kaldır ({ $count ->
    [one] bir dosya
   *[other] { $count } dosya
}):
font-restore-note = Her ayar dosyasının önce yedeği alınır. Fontu hâlâ kullanan bir terminal varsa onaylamadan önce kapatın.
font-confirm-restore = Eski haline getir
font-restored = Her şey eski haline döndü. Ayar dosyalarının yedekleri her dosyanın yanında duruyor.
font-error-font-in-use = Font dosyası hâlâ bir programda açık, bu yüzden font kurulu kaldı. Terminal ayarları zaten geri alındı. Terminallerinizi kapatıp yeniden deneyin.

## Plan ücreti (Adım 15b)

plan-title = Plan ücreti
plan-hint = Genel Bakış'taki plan değeri, bu ayın API karşılığı maliyetini bu aylık ücretle karşılaştırır.
plan-table = Planınızın liste fiyatı: { $plan }, ayda ${ NUM($usd, 2) }.
plan-table-none = Planınızın fiyatı bilinmiyor (örneğin Team ya da Enterprise). Ücreti kendiniz girin.
plan-fee = Aylık ücret (USD)
plan-fee-hint = Boş bırakırsanız liste fiyatı kullanılır.
plan-save = Kaydet
plan-reset = Liste fiyatına dön
plan-invalid = { TR_SUFFIX(0, "abl") } büyük, en fazla iki ondalıklı bir tutar girin.
plan-saved = Kaydedildi.

## Geçici dizinler (Adım 16b)

temp-dirs = Geçici dizinler
temp-dirs-hint = Bu klasörlerdeki projeler Projeler sayfasında ve Oturumlar listesinin proje seçicisinde gizlenir; oturumları ve maliyetleri her yerde sayılmaya devam eder. Desen yolun herhangi bir parçasıyla eşleşir, büyük/küçük harfe bakmaz: * herhangi bir metin, %TEMP% bir ortam değişkeni yerine geçer.
temp-dirs-none = Desen yok: tüm projeler gösterilir.
temp-dirs-new = Yeni desen
temp-dirs-placeholder = Örneğin \scratch\ ya da %TEMP%
temp-dirs-add = Ekle
temp-dirs-remove = { $pattern } desenini kaldır
temp-dirs-reset = Varsayılanlara dön
