# Adım 12j — Statusline düzenleyicisi: ayrı ayarlar sayfası ve UX iyileştirmesi

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1.x | 12i | 1-2 oturum | ⬜ |

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

- [ ] Ayarlar kenar çubuğunda "Statusline" bölümü, Veri kaynakları'ndan bağlantı.
- [ ] Önizleme şeridi: terminal paleti, durum ve genişlik seçicileri, parça–öğe bağı.
- [ ] Öğe listesi: sıralama (sürükleme ve klavye), gizle, sil, öğe ekleme paleti.
- [ ] Seçili öğe editörü.
- [ ] Araç çubuğu: tema, renk modu, şablon, dışa/içe aktarma, geri al, varsayılan.
- [ ] Segmented control bileşeni (yoksa).
- [ ] `view.ts` mantığı ve testleri.
- [ ] Fluent (EN/TR).
- [ ] İlk çalışan düzenin ekran görüntüsü kullanıcıya gösterilir ve onaylanır, ardından cila.

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

_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
