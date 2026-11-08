# Adım 12l — Statusline satır düzenleyicisi: satır kartları, boşluk ve ayraç öğeleri

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1.x | 12k | 1 oturum | ⬜ |

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

- [ ] `view.ts`: `lines`, `addLine`, `removeLine`, `moveToLine`, otomatik ayraç, `flex` ipucu; testleri.
- [ ] Satır kartları: kart başına `ItemList`, kartlar arası sürükleme ve klavye taşıması, duyurular.
- [ ] Ekleme paleti: düzen grubu, "+ Satır ekle".
- [ ] Öğe editörü: `space` genişliği, ayraç glifi ve "Tüm ayraçlara uygula", `flex` açıklaması.
- [ ] Toolbar: sağ pay; global ayraç alanının kaldırılması.
- [ ] Önizleme: `flex`/`space` seçimi, hizalama notu.
- [ ] Fluent (EN/TR).
- [ ] İlk çalışan düzenin ekran görüntüsü kullanıcıya gösterilir ve onaylanır, ardından cila.

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

_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
