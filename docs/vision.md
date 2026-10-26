# TokenRadar — Vizyon

> **TokenRadar — usage limits for AI coding agents.**
> AI kodlama ajanlarının (önce Claude Code, sonra Codex ve diğerleri) kullanım limitlerini, oturumlarını ve projelerini
> gerçek zamanlı izleyen; görünmez ayak izine sahip, göze hitap eden, cross-platform bir masaüstü uygulaması.

## Problem

Claude Pro/Max kullanıcıları iki tür limitle çalışır:

- **5 saatlik kayan oturum penceresi.** İlk mesajla başlar ve `resets_at` anında yenilenir.
- **Haftalık limitler.** Biri tüm modelleri kapsar (`weekly_all`). Bunlara ek olarak model bazlı kotalar (`weekly_scoped`, örneğin Fable) bulunabilir. Haftalık limitler hesap bazında sabit bir zamanda yenilenir.

Bugün bu limitler hakkında bilgi parça parça geliyor: `/usage` komutu anlık bir yüzde gösteriyor, ccusage geçmiş token sayılarını listeliyor, statusline ise tek bir satır sunuyor. Şu soruları **hiçbir araç** yanıtlamıyor:

- Hangi oturum ve proje 5 saatlik pencerenin ne kadarını, haftalık limitin ne kadarını tüketti?
- Bu hızla limite ne zaman takılırım? Reset'ten önce mi, sonra mı?
- Günün hangi saatlerinde ve haftanın hangi günlerinde en çok tüketiyorum?
- Geçmiş pencerelerimden hangileri limite takıldı, neden?
- Claude Code dışındaki tüketim (claude.ai, Desktop, Cowork) limitimin ne kadarını yiyor?

Ayrıca mevcut araçların çoğu macOS menü çubuğu uygulaması veya tarayıcı eklentisi. **Windows'ta kaliteli, native hisli bir çözüm yok.**

## Hedef kullanıcı

- **Birincil:** Claude Code'u yoğun kullanan, Pro/Max aboneliği olan geliştiriciler. Windows öncelikli, ardından macOS ve Linux.
- **İkincil (v0.4+):** Codex kullanıcıları ve birden fazla AI kodlama aracını birlikte kullananlar.
- **Örnek profil:** Max 20x kullanıcısı. Ayda ~11,7 GB JSONL üretiyor, ~1.500 proje klasörü ve ~330 bin assistant kaydı var. Uygulama bu ölçekte rahat çalışmalı.

## İlkeler (pazarlık konusu değil)

1. **Dürüst sayılar.** Ölçülen değer ile tahmin edilen değer görsel olarak her zaman ayrılır. Her sayı kaynağını (statusline / API / tahmin) ve tahminse güven düzeyini taşır. Yanlış ama kendinden emin bir sayı, hiç sayı olmamasından daha kötüdür.
2. **Görünmez ayak izi.** Performans bütçeleri ([performance-budget.md](performance-budget.md)) birer **sürüm kapısıdır**. Bütçeyi aşan bir özellik yayınlanmaz.
3. **Mahremiyet varsayılandır.** Telemetri yok. Veri makineden çıkmaz. Claude Code kimlik bilgisi yalnızca açık onayla ve **salt-okunur** kullanılır: asla kopyalanmaz, loglanmaz, refresh edilmez. Ağ trafiği yalnızca `api.anthropic.com` (onaylıysa) ve güncelleme kontrolü (GitHub Releases) ile sınırlıdır.
4. **Native kabuk, karakterli veri dili.** Pencere zemini işletim sistemiyle kaynaşır (Mica/vibrancy). Veri görselleştirme ise kendine özgü bir "radar/enstrüman" dili kullanır.
5. **İki dil birinci sınıftır.** İngilizce ve Türkçe ilk günden desteklenir. Türkçe dilbilgisi doğru uygulanır: ünlü uyumlu ekler (`%72'si`, `5'te`, `6'da`), `%` işareti sayının önünde, `İ/ı` büyük-küçük harf dönüşümü.
6. **Provider-agnostik çekirdek.** Claude ilk provider'dır ama çekirdek hiçbir yerde "Claude" varsaymaz.
7. **Limitleri aşmaya yardım etmeyiz.** Hesap rotasyonu, limit atlatma veya trafik dinleme (proxy/MITM) kapsam dışıdır ve öyle kalır.

## Sürüm yol haritası

| Sürüm | Ad | Özü |
|---|---|---|
| **v0.1** | Radar Çekirdeği (MVP) | Tray'de canlı ikili halka, tray paneli, statusline köprüsü + onaylı API, artımlı log indeksleyici, canlı oturumlar, bildirimler (eşik / projeksiyon / reset), EN/TR, CLI, installer + otomatik güncelleme. **% geçmişi kaydı ilk günden başlar.** |
| **v0.2** | Analitik ve Atıf | Kalibrasyon motoru, oturum/proje bazlı limit etkisi atfı, ana pencere: genel bakış, oturumlar, projeler, pencere arşivi, ısı haritası, % zaman serisi, periyodik özet. |
| **v0.3** | Genişleme | Mini widget (kompakt şerit), çoklu Claude hesabı, WSL desteği. |
| **v0.4** | Codex | Codex provider'ı (limitler doğrudan yerel loglardan), provider seçici. |
| **v0.5** | Cross-platform | macOS (Keychain, menü çubuğu template ikonu, notarization) ve Linux (AppIndicator, AppImage/deb). |

Ayrıntılı adımlar: [plan/README.md](plan/README.md)

## Kapsam dışı (şimdilik)

- Bulut senkronizasyonu, ekip/organizasyon panoları, hesap paylaşımı.
- API key ile faturalandırılan kullanım için fatura takibi. Maliyet ($) yalnızca **ikincil bir "API karşılığı" metrik** olarak gösterilir.
- Uzak makinelerdeki (SSH) oturumların toplanması. İleride headless çekirdek ile mümkün olabilir.
- OpenTelemetry alıcısı. Değerlendirildi ve ertelendi; gerekçesi [decisions.md](decisions.md) içinde (ADR-014).

## Başarı ölçütleri

- **Doğruluk:** Panel ve tray'de gösterilen 5 saatlik ve haftalık %, Claude Code `/usage` çıktısıyla ±1 puan içinde eşleşir.
- **Atıf (v0.2):** Kalibre edilmiş atıf, geriye dönük testte (backtest) pencere başına ortalama mutlak hata ≤ 3 puan verir.
- **Ayak izi:** Tray modunda RAM ≤ 25 MB ve boşta CPU ≈ %0. Bütün bütçeler [performance-budget.md](performance-budget.md) içinde.
- **Kullanılabilirlik:** İlk kurulumdan sonra 60 saniye içinde güncel limitler görünür. Geçmişin indekslenmesi arka planda sürerken uygulama kullanılabilir durumdadır.
