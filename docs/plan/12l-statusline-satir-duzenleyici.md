# Adım 12l — Statusline satır düzenleyicisi: satır kartları, boşluk ve ayraç öğeleri

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1.x | 12k | 1 oturum | ✅ |

## Amaç

Adım 12k'nin çekirdeğini (ayraç öğeleri, `space`, `flex`, sağ pay) düzenleyiciye getirmek. Kullanıcı birden çok satırı rahatça kurabilmek ve öğeleri sağa yaslayıp ortalayabilmek istiyor. Bugünkü düz listede satır sonu yalnızca bir çizgi olarak görünüyor; satır sayısı arttıkça listeyi takip etmek zorlaşıyor.

## Önce oku

- ADR-019, ADR-050, ADR-051
- `docs/design-language.md` (token'lar, bileşenler, EN/TR sözlük)
- Adım 12k'nin devir notu
- `app/src/pages/settings/StatuslineTab.svelte`
- `app/src/pages/settings/statusline/{ItemList.svelte, AddPalette.svelte, ItemEditor.svelte, TermPreview.svelte, editor.svelte.ts}`
- `app/src/pages/settings/view.ts`, `view.test.ts`

## Kullanıcıyla kararlaştırılanlar (2026-11-08)

- **Satır kartları:** Her satır ayrı bir kutuda durur. Satır eklenip silinebilir; öğeler satırlar arasında taşınabilir.
- **Otomatik ayraç:** Yeni bir değer eklenince, satırdaki önceki öğe de bir değerse araya bir ayraç girer. Kullanıcı istemezse siler.
- **Hizalama `flex` öğesiyle yapılır.** Öğe başına hiza seçici yoktur.

## Kapsam

1. **Satır kartları:**
   - Veri düz öğe listesi olarak kalır. Kartlar, listenin `line_break` öğelerine göre gruplanmış bir görünümüdür. Geri alma, kayıt ve TOML biçimi değişmez.
   - Yeni `view.ts` yardımcıları:
     - `lines(items)`: kart başına öğe indeksleri.
     - `addLine`: sona bir satır ekler.
     - `removeLine`: satırı öğeleriyle birlikte siler. Onay istenmez; işlem geri alınabilir.
     - `moveToLine`: bir öğeyi başka bir satıra taşır.
   - Her kartın bir başlığı ("Satır 1") ve satırı silme düğmesi vardır. Kartların altında "+ Satır ekle" bulunur.
   - `ItemList` kart başına çizilir. Sürükle-bırak kartlar arasında da çalışır.
   - Alt+↑/↓ kart sınırını geçerek öğeyi komşu satıra taşır. `aria-live` duyurusu satır numarasını da söyler.
2. **Boşluk öğeleri listede:**
   - `flex`, "⟷ Esnek boşluk" satırı olarak görünür. Altındaki ipucu, satırdaki `flex` sayısına göre değişir: "Sonrası sağa yaslanır" ya da "Ortadaki grup ortalanır". Bu sunum mantığıdır, `view.ts` içinde yazılır ve test edilir.
   - `space` satırı kendi genişliğini gösterir. Ayraç satırı glifini gösterir.
3. **Ekleme paleti:**
   - Düzen grubu şunları içerir: Ayraç, Boşluk, Esnek boşluk, Özel metin.
   - `line_break` paletten çıkar; yerine kartların altındaki "+ Satır ekle" gelir.
   - `addItem` bir değer eklerken, satırdaki önceki öğe de bir değerse önüne bir ayraç koyar.
4. **Öğe editörü:**
   - `space` için genişlik seçici (1–8).
   - Ayraç için glif alanı ve "Tüm ayraçlara uygula" düğmesi. Global ayraç alanı toolbar'dan kalkar.
   - `flex` için yalnızca açıklama gösterilir.
5. **Toolbar:** "Sağ pay" (0–8) eklenir. İpucu: "Satır gerçek terminalde sağdan taşıyorsa artırın."
6. **Önizleme:**
   - Seçilen genişlik kutusu hizayı gösterir.
   - `flex` ve `space` span'leri tıklanınca öğeyi seçer; hover vurgusu iki yönde çalışır.
   - Font notunun yanına hizalama notu eklenir: "Gerçek terminalde iki hücre kaplayan glifler hizayı kaydırabilir."
7. **Fluent:** EN ve TR metinler `settings.ftl` içindedir. Mevcut "editörün ürettiği her anahtar EN dosyasında var" testi yeni anahtarları da kapsar.

## Kapsam dışı

- Çekirdek ve render değişiklikleri (12k'da kalır). Gerekirse devir notuna yazılır.
- Sürekli animasyon (ADR-019).
- Kurulu sürümü güncellemek veya sürüm çıkarmak.

## Görevler

- [x] `view.ts`: `lines`, `addLine`, `removeLine`, `moveToLine`, otomatik ayraç, `flex` ipucu; testleri.
- [x] Satır kartları: kart başına `ItemList`, kartlar arası sürükleme ve klavye taşıması, duyurular.
- [x] Ekleme paleti: düzen grubu, "+ Satır ekle".
- [x] Öğe editörü: `space` genişliği, ayraç glifi ve "Tüm ayraçlara uygula", `flex` açıklaması.
- [x] Toolbar: sağ pay; global ayraç alanının kaldırılması.
- [x] Önizleme: `flex`/`space` seçimi, hizalama notu.
- [x] Fluent (EN/TR).
- [x] İlk çalışan düzenin ekran görüntüsü kullanıcıya gösterilir ve onaylanır, ardından cila.

## Teknik notlar ve tuzaklar

- **UI'da hesap yapılmaz.** Hizalama, bölgeler ve dar genişlik davranışı backend önizlemesinden gelir. `flex` ipucu yalnızca sayıma dayalı bir sunum metnidir.
- **Boş satır:** Art arda iki `line_break` boş bir kart üretir. Kart görünür ve içine öğe bırakılabilir. Render boş satırı zaten çizmez.
- **Son görünür değer öğesi korunur** (12j kuralı). `removeLine` bu öğeyi silecekse düğme pasif olur.
- **Sürükle-bırak:** Klavye ve ekran okuyucuyla da kullanılabilmeli. Geri bildirim statik bir yer tutucudur, animasyon yoktur.
- **Ekran görüntüleri** CDP ile alınır; SendKeys kullanılmaz. Reduced motion bu makinede açık.

## Kabul kriterleri

- Satır eklemek, silmek ve öğeyi satırlar arasında taşımak önizlemeye hemen yansır, kısa süre içinde kaydedilir ve geri alınabilir.
- Değer eklemek otomatik ayraç koyar; ayraç tek tek silinebilir.
- `flex` eklemek önizlemede sağa yaslamayı ve ortalamayı gösterir.
- Bütün etkileşimler klavyeyle yapılabilir.
- EN ve TR ekran görüntüleri kayıtlıdır.

## Doğrulama

- `pnpm -C app check`, `pnpm -C app lint`, `pnpm -C app test`
- `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (binding değişirse)
- Debug build ve portable mod ile CDP üzerinden EN ve TR ekran görüntüleri (`img/12l-*.png`).

## Devir notu

**Tarih:** 2026-11-08 · **Karar:** ADR-052 · **Ekran görüntüleri:** `img/12l-statusline-en.png` (açık, 80 sütun, `flex` seçili), `img/12l-statusline-tr-dark.png` (koyu, ikinci kart ve ayraç editörü)

### Yapılanlar
- **`view.ts`:**
  - `lines`, `placeOf`, `addToLine`, `addLine`, `canRemoveLine`, `removeLine`, `moveToLine`, `dropInto`, `flexHintKey` ve `applySeparator` eklendi.
  - `addItem` otomatik ayraç koyuyor. Palet `line_break`'i sunmuyor ve düzen grubunu Ayraç, Boşluk, Esnek boşluk, Özel metin sırasıyla veriyor.
  - Önizleme ve seçiciler için `SPACE_WIDTHS`, `MARGINS` ve `RESERVED_COLUMNS` sabitleri eklendi.
- **Testler:** `view.test.ts` içinde yeni "statusline lines" grubu var (7 test): kart bölme (gizli satır sonu dahil), otomatik ayraç (iki yan, boş kart, düzen öğesi, gizli komşu, dolu liste), satır ekleme ve silme kuralları, kartlar arası taşıma ve bırakma, Alt+ok takası, `flex` ipucu, tüm ayraçlara uygulama. Fluent anahtar testi `flex` ipuçlarını da kapsıyor. Frontend testleri 113'ten 120'ye çıktı.
- **`ItemList`:**
  - Öğeler kart başına çiziliyor: başlık, silme düğmesi, kart başına "+ Öğe ekle", boş kart yer tutucusu ve en altta "+ Satır ekle".
  - Sürükle-bırak kartlar arasında da çalışıyor. ↑/↓ kartlar boyunca geziniyor; Alt+↑/↓ kart sınırını geçiyor.
  - Duyurular satır numarasını söylüyor. Satır ekleme ve silme de duyuruluyor.
  - Düzen satırları 30 px. `flex` satırı `⟷` işaretini ve ipucunu, `space` satırı genişliğini, ayraç satırı glifini gösteriyor.
- **`ItemEditor`:** `space` için 1–8 seçici, ayraç için "Tüm ayraçlara uygula" ve `flex` için satırdaki etkisini anlatan not.
- **`StatuslineTab`:**
  - Global ayraç alanı kaldırıldı. "Sağ pay" (0–8) önizleme çubuğunda.
  - Hizalama notu, satırda etkin bir `flex` varken gösteriliyor. Palet başlığı hangi satıra eklendiğini söylüyor.
- **`TermPreview`:** Satırın boş bıraktığı sağ sütunlar (4 + `margin`) taralı. Metni olmayan span'ler seçiliyken ve üzerine gelinince tonlanıyor; tıklanınca öğeyi seçiyor.
- **Fluent (EN/TR):** Yeni anahtarlar eklendi. Kullanılmayan `statusline-separator`, `statusline-add-title`, `statusline-items` ve `statusline-moved` silindi.
- **Dokümanlar:** ADR-052, design-language (kartlar, düzen satırları, önizlemede pay ve sözlük), CHANGELOG (12j'nin eksik kaydı da bu girdiye dahil edildi).

### Sapmalar (kullanıcı ilk düzenin ekran görüntüsünde onayladı)
- **Sağ pay önizleme çubuğunda,** araç çubuğunda değil. Araç çubuğunda Dışa/İçe aktar düğmelerini alt satıra itiyordu.
- **Otomatik ayraç iki yana da konuyor:** önceki öğe değerse öne, sonraki öğe değerse arkaya.
- **Düzen satırları ince** (30 px). Plan bunu öngörmüyordu; ayraçlar listeyi uzatıyordu.
- **Kart başına "+ Öğe ekle":** Plan tek bir ekleme düğmesi varsayıyordu. Boş bir karta ekleyebilmek için kart başına düğme gerekti.
- **Alt+↑/↓ için ayrı bir kod yazılmadı.** Komşu bir satır sonuyla takas, öğeyi zaten komşu karta geçiriyor. `moveToLine` sürükleme için kullanılıyor.
- **12j'den kalan bir hata düzeltildi:** Liste uzayınca bütün sayfa kayıyordu. Ekran okuyucu duyurusu (`position: absolute`) kaydırılan sütunun dışına taşıyordu; sütun artık konumlanmış.
- **Çekirdek ve binding değişmedi.** Bu yüzden `cargo` komutları çalıştırılmadı.

### Doğrulama
- `pnpm -C app check`, `lint` ve `test` temiz (120 test).
- Debug build, portable mod ve `TOKENRADAR_FAKE_LIMITS=panel:no-five-hour` ile CDP üzerinden (kurulu uygulama kapalıydı) şunlar denendi:
  - Gerçek tıklamalarla `flex` eklendi, satır eklendi ve ikinci satıra iki değer eklendi. Araya ayraç kendiliğinden girdi; önizleme sağa yaslamayı gösterdi.
  - Alt+↓ son öğeyi 2. satırın başına taşıdı. Duyuru "moved to line 2, position 1 of 4" oldu ve odak öğeyle gitti. ↑ yalnızca gezindi, Alt+↑ öğeyi geri getirdi.
  - Satır silindi ve duyuruldu. Ctrl+Z satırı geri getirdi.
  - Gerçek fare olaylarıyla 2. satırdan 1. satırın başına sürükleme yapıldı. Bırakma çizgisi doğru kartta göründü.
  - 80 sütunda taralı pay bölgesi görüldü. TR'de araç çubuğu artık tek satır (12j'nin açık konusu).
- Ekran görüntüleri DWM sınırları ve `CopyFromScreen` ile alındı, alttaki 8 px kırpıldı.

### Açık konular
- **Gerçek satır yeni sürüme kadar değişmez** (12k'dan sürüyor). Kurulu v0.1.0 CLI, `space`, `flex` ve `margin`'i bilmiyor.
- **Sürüklerken liste kendiliğinden kaymıyor.** Uzun bir listede uzak bir karta sürüklemek için önce kaydırmak ya da Alt+ok kullanmak gerekiyor. Gerekirse kenarda otomatik kaydırma eklenebilir; ADR-019 kapsamında kısa ve veri odaklı olmalı.
- **Ekran okuyucuyla gerçek deneme yapılmadı** (Narrator). Duyurular CDP ile okundu.
- **Bir öğe kartlar arasında taşınınca otomatik ayraç eklenmiyor.** Kural yalnızca eklemede çalışıyor; taşınan değerin yanına ayracı kullanıcı ekliyor.
- **12i'den kalanlar sürüyor:** Powerline'da `▲` rengi, stdin'de UTF-8 BOM, gerçek Nerd Font ile görsel kontrol.

### Sonraki adıma notlar
- Statusline v3 zinciri (12i–12l) tamamlandı. Sürüm hazırlığında CHANGELOG'daki Unreleased girdileri tek bir statusline başlığı altında toplanabilir. Sürüm notu daralma sonrası `…` kesilmesini anlatmalı (12k).
- Kartlar bir görünüm olduğu için TOML'da elle yazılan art arda iki `line_break` boş bir kart olarak görünür. Bu bilerek böyle.
