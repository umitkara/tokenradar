# Adım 11 — Bildirimler, ayarlar ve onboarding

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1 | 10 | 2 oturum | ⬜ |

> Bu adım büyük. Gerekirse **11a (bildirimler + ayarlar altyapısı)** ve **11b (ayarlar UI + onboarding + doctor)** olarak iki oturuma bölün. Bölünürse devir notu her iki parça için ayrı ayrı yazılır.

## Amaç

v0.1'i kullanıcıya hazır hale getirmek:
- Akıllı ve gürültüsüz bildirimler: eşik, projeksiyon ve reset.
- Eksiksiz bir ayarlar penceresi.
- Statusline köprüsünü ve API onayını açıkça anlatan bir ilk çalıştırma deneyimi.
- Tanılama paketi (`doctor`).

## Önce oku

- `docs/architecture.md` §5 (Notifier), §10, §11 (install/uninstall), §13, §14
- `docs/design-language.md` §7, §8, §9
- `docs/decisions.md`: ADR-005, ADR-006, ADR-009, ADR-013
- Adım 07'nin devir notu (statusline kurulum API'si)

## Kapsam

- **`Notifier`** (core):
  - Kurallar, histerezis ve tekilleştirme (`notification_log`).
  - Susturma ve sessiz saatler.
  - Yerelleştirilmiş metinler (Rust i18n).
- **Bildirim gönderimi:** `tauri-plugin-notification`. Tıklayınca panel açılır.
- **Config:** `config.toml` şeması (`schema_version`), doğrulama, canlı uygulama (`SettingsChanged` olayı), migration.
- **Ayarlar penceresi (`settings.html`):** Sekmeler: Genel · Veri kaynakları · Bildirimler · Gizlilik ve veri · Hakkında.
- **Onboarding penceresi (`onboarding.html`):** İlk çalıştırmada açılır, 4 adımdan oluşur.
- **Autostart:** `tauri-plugin-autostart`.
- **Tray görünürlük yardımcısı:** Windows 11'de ikonu taşma menüsünden çıkarıp görev çubuğuna sabitleme (kullanıcı onayıyla).
- **`tokenradar doctor` ve ayarlardaki "Tanılama paketi oluştur" butonu.**

## Kapsam dışı

- Periyodik özet bildirimi. Analitik gerektirdiği için Adım 17'de yapılacak.
- Çoklu hesap UI'ı (Adım 19). Ancak ayar şeması hesap listesini **şimdiden** desteklemeli.

## Görevler

### Bildirimler

- [ ] Kural motoru (`notifier.rs`). Girdi olarak `LimitEngine` olaylarını alır.
  - **Eşik kuralları:** Her limit türü için ayrı bir eşik listesi tutulur (varsayılan `[75, 90, 100]`). Bir eşik, **pencere başına bir kez** tetiklenir. Tekilleştirme anahtarı: `rule:kind:scope:resets_at:threshold`. Yeni pencere başlayınca kurallar yeniden silahlanır.
  - **Projeksiyon kuralı:** `will_hit_before_reset && eta_100 ≤ 60 dk && confidence ≥ orta` koşulunda tetiklenir. Pencere başına en fazla 1 kez. Ancak eta 15 dakikanın altına düşerse ikinci bir "son uyarı" verilebilir (ayarlanabilir).
  - **Reset kuralı:** Yalnızca o pencerede tepe değer ≥ `reset_notify_min` olduysa (varsayılan %80) veya limite takılındıysa tetiklenir. Böylece her 5 saatte bir anlamsız bildirim gelmez.
  - **Haftalık limitler:** Aynı kurallar uygulanır, ama metinler ve eşikler ayrıdır.
  - **Susturma ve sessiz saatler:** Bu sürelerde tetiklenen bildirimler kaydedilir ama gösterilmez. Susturma bitince yalnızca **hâlâ geçerli** olanlar tek bir özet bildirim olarak gösterilir.
- [ ] Bildirim metinleri (EN/TR `.ftl`). Örnekler:
  - "5 saatlik pencere %90'a ulaştı — 1 sa 12 dk sonra yenilenir"
  - "Bu hızla 38 dk içinde dolacak, reset 1 sa 52 dk sonra"
  - "5 saatlik pencere yenilendi — tam kapasite"
- [ ] Windows toast notları: Bildirimlerin uygulama adı ve ikonuyla görünmesi için AppUserModelID gerekir; installer (Adım 12) Başlat menüsü kısayolunu bu ID ile oluşturur. Geliştirme modunda başlık "PowerShell" veya benzeri olabilir; bunu belgele. Portable mod için de davranışı belgele.
- [ ] Testler (sahte saat): Eşik geçişi; pencere başına tek tetikleme; yeni pencerede yeniden silahlanma; projeksiyon histerezisi; susturma sonrası özet; reset filtresi.

### Config ve ayarlar

- [ ] `config.toml` şeması:
  - `schema_version`
  - `[general]`: `locale` (system/en/tr), `theme`, `autostart`, `panel_warm_ttl`
  - `[[accounts]]`: provider, config_dir, label, `api_consent { granted, granted_at, consent_version }`, `statusline { installed, previous_command, chain, chain_position }`
  - `[notifications]`: eşikler (her tür için), `projection` (bool), `final_warning_min`, `reset_notify_min`, `quiet_hours { start, end }`
  - `[data]`: `raw_retention_days`, `update_check`
  - `[advanced]`: `log_level`
  - Bilinmeyen alanlar yazımda **korunur** (`toml_edit`).
- [ ] Ayarlar penceresi:
  - **Genel:** dil (anlık uygulanır), tema, Windows ile başlat, panelin sıcak tutulma süresi.
  - **Veri kaynakları:**
    - Claude dizini: algılanan yol, oturum ve dosya sayısı, "Değiştir…" butonu.
    - **Statusline köprüsü:** durum (kurulu / değil / son görülme). "Kur…" butonu `settings.json` diff'ini bir diyalogda gösterir ve onay alır. "Zincirleme" seçeneği mevcut komutu korur. Mevcut komut usage API'sini çağırıyorsa (örneğin `statusline.py`) bilgi notu gösterilir. "Kaldır" seçeneği.
    - **API erişimi:** Kapalı/açık anahtarı ve açık dilli bir açıklama: ne okunur, ne okunmaz, nereye gider, refresh yapılmaz, istediğin zaman kapatabilirsin. Açmak için açık onay butonu gerekir; onay zaman damgası ve metin sürümüyle saklanır. Ayrıca polling durumu: son istek, sonraki istek, geri çekilme.
  - **Bildirimler:** eşik çipleri (ekle/sil), projeksiyon, son uyarı, reset bildirimi eşiği, sessiz saatler, "Test bildirimi gönder" butonu.
  - **Gizlilik ve veri:**
    - Veritabanı boyutu, ham olay saklama süresi.
    - "Tüm TokenRadar verisini sil" (çift onay; Claude Code dosyalarına **dokunulmaz**).
    - Ağ özeti: "Bu uygulama yalnızca şu adreslere bağlanır: …"
    - "Tanılama paketi oluştur".
  - **Hakkında:** sürüm, güncelleme kontrolü (updater Adım 12'de bağlanacak, şimdilik yer tutucu), lisanslar (üçüncü parti lisans listesi build sırasında üretilir), GitHub linki.
- [ ] Panel ve tray, ayar değişikliklerini anında yansıtır (`SettingsChanged` olayı).

### Onboarding

- [ ] İlk çalıştırma tespiti: `kv.onboarding_done` yoksa onboarding açılır. Tray'den tekrar açılabilir.
- [ ] Adımlar:
  1. **Hoş geldin:** dil seçimi (algılanan dil önceden seçili) ve tek cümlelik değer önerisi.
  2. **Claude Code bulundu:** dizin, oturum ve proje sayısı. İndeksleme bu adımda arka planda başlar ve ilerleme halkası gösterilir. Claude Code bulunamazsa yol seçme ekranı açılır.
  3. **Limit verisi:**
     - Statusline köprüsü kartı ("önerilen", diff önizlemesi, tek tık kurulum).
     - API kartı (opsiyonel, ayrıntılı açıklama, onay).
     - İkisi de atlanabilir. Atlanırsa sonuçları açıkça yazılır: "limit yüzdeleri görünmez; yalnızca token ve maliyet takibi yapılır".
  4. **Tray ipucu ve başlatma:**
     - Windows 11 taşma menüsünü gösteren küçük bir görsel.
     - "Simgeyi her zaman göster" butonu (aşağıdaki yardımcı).
     - "Windows ile başlat" seçeneği (varsayılan açık).
     - Bitti.
- [ ] **Tray sabitleme yardımcısı:** Windows 11'de `HKCU\Control Panel\NotifyIconSettings\<id>\IsPromoted = 1`. Bu yarı-belgelenmiş bir mekanizma; uygulamamıza ait kaydı `ExecutablePath` alanıyla bul. Yalnızca kullanıcı butona basınca yazılır. Başarısız olursa manuel talimat gösterilir. Davranış ADR olarak belgelenir.

### Doctor

- [ ] `tokenradar doctor [--out <zip>]` ve ayarlardaki buton. Pakete girenler:
  - Sürümler (uygulama, Claude Code son görülen sürüm, WebView2, işletim sistemi)
  - Yollar, dosya ve olay sayıları, DB boyutu
  - Son 200 log satırı (**redaksiyon testi yapılmış**)
  - Config (sırlar hariç)
  - Polling durumu, şema sapma sayaçları, son performans ölçümleri
  
  Token, içerik ve proje yolları paket dışında kalır; yollar hash'lenir.
- [ ] Test: Doctor çıktısında `sk-ant-` deseni ve fixture'lardaki örnek içerik metinleri bulunmamalı.

## Teknik notlar ve tuzaklar

- **Ayar penceresi ve bellek:** Ayarlar penceresinin `warm_ttl` değeri 0'dır; kapanınca hemen yok edilir. Panel ve ayarlar aynı anda açıksa bellek bütçesi "ana pencere" satırına göre değerlendirilir.
- **API onay metni sürümlüdür:** Metin değişirse (örneğin yeni bir veri alanı eklenirse) onay yeniden istenir.
- **Diff diyaloğu:** JSON diff'i satır bazlı ve renkli gösterilir. Kullanıcı neyin değiştiğini **görmeden** hiçbir şey yazılmaz.

## Kabul kriterleri

- [ ] Bildirim kuralları testleri geçer. Gerçek kullanımda bir gün boyunca gürültü değerlendirmesi yapılır: tetiklenen bildirimler listelenir ve gereksiz olan kalmadığı devir notunda belirtilir.
- [ ] Onboarding sıfır kurulumdan (`%LOCALAPPDATA%\TokenRadar` silinmiş durumda) sonuna kadar akar. Statusline kurulumu diff onayıyla çalışır ve panelde canlı veri görünür.
- [ ] Ayar değişiklikleri yeniden başlatma gerektirmeden uygulanır (dil, tema, eşikler).
- [ ] "Tüm verileri sil" yalnızca TokenRadar verisini siler. Claude Code dizini dokunulmadan kalır.
- [ ] Doctor paketinde sır, içerik veya açık yol bulunmaz.

## Doğrulama

```powershell
cargo test -p tokenradar-core notifier config
pnpm -C app test
Remove-Item -Recurse "$env:LOCALAPPDATA\TokenRadar" -WhatIf   # önce WhatIf; yedek alıp gerçekten sil, onboarding'i test et
pnpm dev
.\target\release\tokenradar.exe doctor --out "$env:TEMP\tr-doctor.zip"
```

## Devir notu

_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
