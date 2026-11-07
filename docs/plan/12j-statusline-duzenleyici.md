# Adım 12j — Statusline düzenleyicisi: ayrı ayarlar sayfası ve UX iyileştirmesi

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1.x | 12i | 1-2 oturum | ✅ |

## Amaç

Adım 12i'nin çekirdeği için bir düzenleyici yapılacak. Bu düzenleyici, `claude-code-tool-manager` içindeki builder kadar ayrıntılı olacak. Kullanıcı ayrıca mevcut düzenleme alanının kullanıcı deneyiminin iyileştirilmesini istedi.

**12c ekranında görülen sorunlar** (`img/12c-settings-statusline-en.png`):
- Önizleme bir segmentin ortasından sarıyor (`API` / `$4.12`) ve sayfa kaydırılınca görünümden çıkıyor.
- Önizleme bir terminale benzemiyor. Uyarı, kritik ve bayat durumları önizlemede görülemiyor.
- Sıralama yalnızca ↑/↓ düğmeleriyle yapılıyor.
- Kapalı segmentler aynı listeye karışıyor. Hesapta olmayan segment açık görünüyor ve ilk sırayı tutuyor.
- Geri alma ve varsayılana dönme yok.
- Önizlemedeki bir parçanın listede hangi satıra karşılık geldiği görülmüyor.
- İki seçenekli ayar için açılır liste kullanılmış.

## Önce oku

- ADR-019, ADR-042, ADR-049
- `docs/design-language.md` (token'lar, bileşenler, EN/TR sözlük)
- Adım 12i'nin devir notu
- `app/src/pages/settings/` (`SourcesTab.svelte`, `StatuslineLook.svelte`, `view.ts`, `state.svelte.ts`), `app/src/lib/components/`

## Kullanıcıyla kararlaştırılanlar (2026-11-07)

- **Ayrı "Statusline" sayfası:**
  - Ayarlar kenar çubuğuna yeni bir bölüm eklenir.
  - Kurulum durumu ve diff akışı Veri kaynakları'nda kalır. Gömülü `StatuslineLook` kalkar, yerine "Görünümü düzenle →" bağlantısı gelir.
- **Kaydetme:**
  - Değişiklikler kısa bir debounce ile otomatik kaydedilir.
  - "Geri al" ve "Varsayılana dön" düğmeleri bulunur.
  - Şablon uygulama ve içe aktarma onay ister.

## Kapsam

1. **Sabit önizleme şeridi:**
   - Koyu zeminli bir terminal görünümüdür. Temadan bağımsız bir `--term-*` paleti ve terminal fontu kullanılır.
   - Satır sarmaz. Taşarsa yatay kaydırma çıkar.
   - Durum seçici: Normal, Uyarı, Kritik, Bayat.
   - Genişlik seçici: 120, 80, 50 sütun.
   - Span'ler kendi `item` indeksini taşır. Bir parçaya tıklamak öğeyi seçer; hover iki yönde vurgular.
   - Powerline glifleri için not gösterilir.
2. **İki sütun:**
   - **Sol, öğe listesi:**
     - Yalnızca aktif öğeler listelenir.
     - Sürükleme tutamacı, Alt+↑/↓ ve ↑/↓ düğmeleriyle sıralanır.
     - Gizle/göster ve sil düğmeleri bulunur. `line_break` bir ayırıcı olarak çizilir.
     - "+ Öğe ekle" gruplu bir palet açar: Limitler · Oturum · Proje · Düzen. Her öğenin açıklaması ve örnek çıktısı vardır. Hesapta olmayan pencere pasif görünür.
   - **Sağ, seçili öğenin ayarları:**
     - Biçim (segmented control), etiket (boşsa placeholder'da varsayılan) ve renk örnekleri. Limit renklerinde uyarı ve kritik önceliği açıklanır.
     - Reset ve tempo anahtarları, ayraç karakteri, özel metin.
3. **Araç çubuğu:**
   - Tema (Düz | Powerline | Yuvarlak), renk modu, şablon seçici.
   - Dışa aktar (TOML panoya kopyalanır) ve içe aktar (metin → önizleme ve uyarılar → uygula).
   - Geri al, Varsayılana dön.
4. **Saf mantık `view.ts` içindedir:** taşıma, ekleme/silme, geri alma yığını, debounce birleştirmesi. Testleri yazılır.
5. **Metinler:** EN ve TR metinler `settings.ftl` içindedir.

## Kapsam dışı

- Çekirdek ve render değişiklikleri (12i'de kalır). Gerekirse devir notuna yazılır.
- Sürekli animasyon (ADR-019).
- Ağdan galeri.

## Görevler

- [x] Ayarlar kenar çubuğunda "Statusline" bölümü, Veri kaynakları'ndan bağlantı.
- [x] Önizleme şeridi: terminal paleti, durum ve genişlik seçicileri, parça–öğe bağı.
- [x] Öğe listesi: sıralama (sürükleme ve klavye), gizle, sil, öğe ekleme paleti.
- [x] Seçili öğe editörü.
- [x] Araç çubuğu: tema, renk modu, şablon, dışa/içe aktarma, geri al, varsayılan.
- [x] Segmented control bileşeni (yoksa).
- [x] `view.ts` mantığı ve testleri.
- [x] Fluent (EN/TR).
- [x] İlk çalışan düzenin ekran görüntüsü kullanıcıya gösterilir ve onaylanır, ardından cila.

## Teknik notlar ve tuzaklar

- **UI'da hesap yapılmaz.** Önizleme, durum varyantları ve dar genişlik davranışı backend'den gelir.
- **Debounce:** Kaydetme ile önizleme isteği yarışmamalı. Önizleme, kaydedilmemiş taslak prefs ile çizilir.
- **Geri alma yığını** pencere ömrü boyunca tutulur, diske yazılmaz.
- **Sürükle-bırak:** Klavye ve ekran okuyucu ile de kullanılabilmeli. Geri bildirim statik bir yer tutucudur, animasyon yoktur.
- **Ekran görüntüleri** CDP ile alınır, SendKeys kullanılmaz. Reduced motion bu makinede açık.

## Kabul kriterleri

- Bir öğeyi taşımak, eklemek, biçimini, etiketini veya rengini değiştirmek önizlemeye hemen yansır ve kısa süre içinde kaydedilir.
- Geri al son değişikliği geri getirir. Varsayılana dön, onaydan sonra 12c varsayılanını yükler.
- Durum ve genişlik seçicileri backend çıktısını değiştirir.
- Bütün etkileşimler klavyeyle yapılabilir.
- EN ve TR ekran görüntüleri kayıtlıdır.

## Doğrulama

- `pnpm -C app check`, `pnpm -C app lint`, `pnpm -C app test`
- `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (binding değişirse)
- Debug build ve portable mod ile CDP üzerinden EN ve TR ekran görüntüleri (`img/12j-*.png`).

## Devir notu

**Tarih:** 2026-11-08 · **Karar:** ADR-050 · **Ekran görüntüleri:** `img/12j-statusline-en.png` (açık), `img/12j-statusline-tr-dark.png` (koyu)

### Yapılanlar
- **Ayrı "Statusline" sekmesi** (`StatuslineTab.svelte`). Veri kaynakları'ndaki gömülü `StatuslineLook` kaldırıldı; yerine "Görünümü düzenle →" düğmesi geldi. Köprü kurulu değilse düzenleyici bunu söylüyor ve Veri kaynakları'na bağlantı veriyor.
- **Önizleme şeridi** (`statusline/TermPreview.svelte`):
  - `--term-*` paleti (Windows Terminal'in varsayılan 16 rengi) ve `--font-term` kullanıyor.
  - Satır sarmıyor, seçilen genişlik kesikli kenarla gösteriliyor ve taşan satır yatay kayıyor.
  - Durum ve genişlik seçicileri backend çıktısını değiştiriyor.
  - Span tıklanınca öğe seçiliyor; hover önizleme ile liste arasında iki yönde vurguluyor.
  - Powerline'da font notu gösteriliyor.
- **Öğe listesi** (`ItemList.svelte`):
  - Pointer ile sürükleme; bırakılacak yer statik bir çizgiyle gösteriliyor.
  - Klavye: Alt+↑/↓ taşıyor, ↑/↓ satırlar arasında geziyor, `Delete` kaldırıyor. Tutamaç odaktayken ↑/↓ taşıyor. Her taşıma `aria-live` ile duyuruluyor.
  - Gizle/göster ve kaldır düğmeleri var; son görünür değer öğesi korunuyor. Satır sonu bir ayırıcı olarak çiziliyor; hesapta olmayan öğe soluk ve etiketli.
- **Ekleme paleti** (`AddPalette.svelte`): Limitler, Oturum, Proje ve Düzen grupları. Her öğenin açıklaması ve katalogdan gelen örneği var. Kullanılan değer öğesi, hesapta olmayan pencere ve dolu liste pasif.
- **Öğe editörü** (`ItemEditor.svelte`): biçim (segment kontrolü), etiket (placeholder'da varsayılan), ayraç glifi ya da özel metin, renk örnekleri ve eşik notu, Reset ve Zaman işareti anahtarları, ↑/↓.
- **Araç çubuğu:** tema, renk modu, ayraç (yalnızca Düz temada), Şablonlar… (önizlemeli onay), Dışa aktar (panoya TOML), İçe aktar… (oku → önizleme ve atlanan anahtarlar → uygula), Geri al (Ctrl+Z), Varsayılana dön… (önizlemeli onay).
- **Durum** (`statusline/editor.svelte.ts`): taslak, geri alma yığını, seçim, önizleme isteği (eski yanıtlar sıra numarasıyla düşürülüyor), debounce'lu kayıt. Pencere gizlenince ya da sayfadan çıkılınca bekleyen kayıt hemen yapılıyor.
- **Saf mantık `view.ts` içinde:** `moveItem`, `dropIndex`, `addItem`, `removeItem`, `toggleHidden`, `canDrop`, `palette`, `itemText`, `spanClasses`, `record`/`undo`, `createSaver`. 12c köprü yardımcıları kaldırıldı.
- **`Segmented.svelte`** bileşeni eklendi.
- **Fluent:** `settings.ftl` içinde EN ve TR statusline düzenleyicisi bölümü. 12c'nin `statusline-segment-*`, `-layout*`, `-bar*` ve `-look*` anahtarları kaldırıldı.
- **Testler:** frontend 112 (`view.test.ts`: öğe kuralları, palet, önizleme sınıfları, geri alma, kayıt; editörün ürettiği her anahtarın EN dosyasında olduğunu denetleyen test). Rust testleri değişmedi ve geçiyor.

### Sapmalar (kullanıcı ilk düzenin ekran görüntüsünde onayladı)
- **Ayarlar penceresi 960×680 oldu** (önceden 800×600; en küçük boyut 640×480). 800 px'te araç çubuğu dağılıyor ve etiketler simgelerin üstüne taşıyordu. Dar pencerede sütunlar alt alta geçiyor.
- **↑/↓ düğmeleri seçili öğenin başlığında**, satırlarda değil. Satırda dört simge sığmıyordu.
- **Çekirdek ve render değişmedi.** Binding'ler yeniden üretilmedi.

### Doğrulama
- `pnpm -C app check`, `lint` ve `test` temiz. `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` ve `cargo test --workspace` temiz.
- Debug build, portable mod ve `TOKENRADAR_FAKE_LIMITS=panel:no-five-hour` ile CDP üzerinden şunlar denendi:
  - Renk değişikliği yaklaşık 0,4 sn sonra `config.toml`'a yazıldı ve Geri al etkinleşti.
  - Uyarı durumunda sarı, kullanıcı rengini geçti.
  - Alt+↓ taşıdı, odak taşınan satırı izledi ve duyuru okundu. Ctrl+Z geri aldı ve kayıt da geri döndü.
  - Gerçek fare olaylarıyla sürükle-bırak çalıştı.
  - Powerline/Kritik görünümü, ekleme paleti ve şablon diyaloğu görsel olarak kontrol edildi.
- Ekran görüntüleri DWM sınırları ve `CopyFromScreen` ile alındı (Mica görünsün diye). Alttaki 8 px kırpıldı.

### Açık konular
- **Gerçek satır, yeni sürüm çıkana kadar değişmez.** Claude Code'daki satırı kurulu v0.1.0 CLI çiziyor; v0.1.0 yalnızca `segments`, `layout`, `bar` ve `pace` alanlarını okuyor. Renk, biçim ve tema değişiklikleri şimdilik yalnızca önizlemede görünür.
- **TR'de araç çubuğu 960 px'te iki satıra iniyor** ("İçe aktar…" alta kayıyor). Kırılma düzgün ve kullanılabilir. Gerekirse düğmeler başlık satırına taşınabilir.
- **Bu makinede Nerd Font yok.** Powerline glifleri önizlemede de kutu görünüyor; not bunu söylüyor. Gerçek bir Nerd Font ile görsel kontrol yapılmadı.
- **12i'nin açık konuları sürüyor:** Powerline'da `▲` projeksiyonunun rengi kayboluyor, stdin'de UTF-8 BOM'a dayanıklılık gerekiyor. Bu adımın kapsamı çekirdek değişikliği içermiyordu.
- **Ekran okuyucuyla gerçek deneme yapılmadı** (Narrator). Klavye yolları ve `aria-live` duyurusu CDP ile doğrulandı.
- Ayraç alanında varsayılan değer `│`, girdi kutusunda imlece benziyor. Küçük bir görsel cila konusu.

### Sonraki adıma notlar
- Yeni bir v0.1.x sürümü, 12b–12j'yi kurulu satıra getirir. Sürüm notunda düzenleyici ve eski CLI uyumu (ADR-049 geriye uyum) anlatılmalı.
- Sahte modda pipe sunucusu açılmıyor; kurulu CLI okumaları gerçek spool'a yazıyor ve kurulu uygulama başlarken onları alıyor. Debug ekran görüntüleri bu yüzden veri kaybettirmiyor (2026-11-08'de kodla doğrulandı).
