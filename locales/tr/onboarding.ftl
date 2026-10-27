# İlk çalıştırma penceresi (Adım 11c): dört adım. Pencere statusline köprüsü, API onayı ve
# klasör seçici için settings.ftl'deki metinleri de kullanır.

onboarding-steps = Kurulum adımları
# $n şu anki adım, $total adım sayısıdır.
onboarding-step = Adım { NUM($n, 0) } / { NUM($total, 0) }
onboarding-back = Geri
onboarding-next = İleri
onboarding-skip = Atla
onboarding-finish = Bitir

## 1. Hoş geldiniz

onb-welcome-title = TokenRadar'a hoş geldiniz
onb-welcome-pitch = Claude Code limitlerinizi görev çubuğundan izleyin: her pencere ne kadar dolu, ne zaman yenilenecek ve hangi oturumlar tüketiyor.
onb-welcome-local = Her şey bu bilgisayarda kalır. Hesap yok, telemetri yok.

## 2. Claude Code

onb-claude-title = Claude Code
onb-claude-found = Claude Code bulundu.
onb-claude-indexing = Oturumlarınız okunuyor… { PCT($pct) }
onb-claude-recent-ready = Mevcut pencere ve bu hafta hazır.
onb-claude-indexed = Tüm oturumlar okundu.
onb-claude-waiting = Okuma birazdan başlayacak.
onb-claude-more-dirs = { NUM($n, 0) } klasör daha izleniyor.
onb-claude-not-found = Claude Code bulunamadı.
onb-claude-not-found-hint = TokenRadar, kullanıcı klasörünüzdeki .claude klasörüne ve CLAUDE_CONFIG_DIR değişkenine bakar. Claude Code verisini başka bir yerde tutuyorsa o klasörü seçin.

## 3. Limit verisi

onb-limits-title = Limit verisi
onb-limits-intro = Claude Code limit yüzdelerini loglarına yazmaz. TokenRadar bu yüzdeleri iki yoldan alabilir; birini, ikisini birden ya da hiçbirini kullanabilirsiniz.
onb-limits-api-optional = İsteğe bağlı
onb-limits-api-show = Nasıl çalıştığını okuyun
onb-limits-ready-statusline = Limitler statusline köprüsünden geliyor.
onb-limits-ready-api = Limitler kullanım API'sinden geliyor.
onb-limits-skipped = İkisi de kurulmazsa limit yüzdeleri görünmez; TokenRadar yalnızca token ve maliyet takibi yapar. İkisini de daha sonra Ayarlar › Veri kaynakları'ndan açabilirsiniz.

## 4. Tray

onb-tray-title = TokenRadar'ı bildirim alanında bulun
onb-tray-intro = TokenRadar görev çubuğunun bildirim alanında yaşar. Windows 11 yeni simgeleri önce ^ okunun altına gizler.
onb-tray-figure = Bildirim alanı ^ okunun altında açılmış görev çubuğu; TokenRadar halkası içinde.
onb-tray-show = Simgeyi her zaman göster
onb-tray-shown = Simge görev çubuğunda.
onb-tray-not-done = Windows, TokenRadar'ın simgeyi taşımasına izin vermedi.
onb-tray-manual = Ayarlar › Kişiselleştirme › Görev çubuğu › Diğer sistem tepsisi simgeleri bölümünü açın ve TokenRadar'ı açık konuma getirin.
onb-tray-manual-win10 = Ayarlar › Kişiselleştirme › Görev çubuğu › Görev çubuğunda görünecek simgeleri seçin bölümünü açın ve TokenRadar'ı açık konuma getirin.
onb-tray-open-settings = Görev çubuğu ayarlarını aç
onb-tray-autostart = Windows ile başlat
onb-tray-autostart-hint = Limitler yalnızca TokenRadar çalışırken izlenir.
onb-tray-done-hint = Panel için simgeye tıklayın; ayarlar ve bu rehber için sağ tıklayın.
