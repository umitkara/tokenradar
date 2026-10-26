# Adım 16 — Oturumlar ve Projeler

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.2 | 15 | 1-2 oturum | ⬜ |

## Amaç

Binlerce oturum ve 1.500+ proje üzerinde akıcı çalışan liste ve detay ekranlarını yapmak. Kullanıcı bu ekranlarda:
- Herhangi bir oturumun 5 saatlik pencerelere ve haftalık limite etkisini,
- Model ve cache verimliliğini,
- Subagent ağacını

görebilmeli. Projeler için aynı bilgi toplu halde sunulur.

## Önce oku

- `docs/design-language.md` §4, §8
- `docs/architecture.md` §5, §13 (içerik **asla** gösterilmez)
- Adım 14'teki sorgu API'leri: `list_sessions`, `get_session`, `get_session_impact`, `list_projects`, `get_project`

## Kapsam

- **Sanal liste bileşeni:** hafif ve kütüphanesiz. Sabit satır yüksekliği, keyset sayfalamasıyla sonsuz kaydırma, klavye navigasyonu.
- **Oturumlar ekranı:**
  - Filtreler: tarih aralığı, proje, model, durum (canlı / tamamlanmış), arama (ad ve başlık).
  - Sıralama.
  - Kolonlar.
  - Satır tıklandığında detay sayfası açılır.
- **Oturum detayı:**
  - Başlık bilgileri.
  - 5 dakikalık maliyet zaman çizelgesi (model bazında yığılmış).
  - Etki tablosu (dokunduğu pencereler ve paylar, haftalık pay).
  - Subagent ağacı.
  - Cache verimliliği.
  - Meta veri (git branch, Claude Code sürümü, entrypoint).
- **Projeler ekranı:**
  - Gruplama: üst klasöre göre ağaç veya düz liste.
  - Arama, sıralama, toplu metrikler.
- **Proje detayı:** Haftalık trend, oturum listesi (filtrelenmiş Oturumlar bileşeni), en pahalı oturumlar, model karışımı.

## Kapsam dışı

- Pencereler ve Örüntüler ekranları, dışa aktarma (Adım 17).
- Oturum içeriği, prompt veya yanıt gösterimi: **hiçbir sürümde yok** (vizyon ilke 3).

## Görevler

- [ ] **Sanal liste:**
  - `VirtualList<T>`: görünür satırlar + tampon. `IntersectionObserver` ile bir sonraki sayfa yüklenir.
  - Klavye: ↑↓ satır seçer, `Enter` detayı açar, `Home`/`End` başa veya sona gider.
  - Erişilebilirlik: `role="grid"` ve satır `aria-*` öznitelikleri.
  - Ölçüm: 10.000 satırlık listede kaydırma sırasında kare süresi < 16 ms.
- [ ] **Oturumlar ekranı:**
  - **Kolonlar:** Durum noktası · Ad (+ proje rozeti) · Başlangıç (göreli ve mutlak tooltip) · Süre · Modeller (küçük rozetler) · Maliyet · Token · **Etki** (5s pencere payı toplamı; haftalık ≈ %) · Subagent sayısı.
  - **Filtre çubuğu:** Tarih aralığı ön ayarları (Bugün / 7 gün / 30 gün / Tümü / Özel), proje seçici (aranabilir), model, "yalnızca canlı". Arama kutusu: `Ctrl+F`, 200 ms debounce.
  - Filtreler URL hash'inde tutulur: geri/ileri çalışır ve bağlantı paylaşılabilir.
  - Canlı oturumlar `LiveSessionsChanged` olayıyla güncellenir.
- [ ] **Oturum detayı:**
  - **Başlık:** Ad, proje, tarih aralığı, durum. Rozetler: model(ler), effort ve fast mode (varsa).
  - **Zaman çizelgesi:** uPlot yığılmış çubuk (5 dk; model bazında maliyet). Arka planda pencere sınırları ve limite takılma bantları.
  - **Etki tablosu:**

    | Pencere | Pay | Pencere tepe değeri | Limite takıldı mı? |
    |---|---|---|---|

    Haftalık döngü payı ayrı satırda gösterilir. Tahmini değerler "≈" ile, düşük güvenli değerler soluk renkte gösterilir.
  - **Subagent ağacı:** Ana oturum → subagent'lar (ad/kimlik, maliyet, token, model). Toplamlar ana oturumda.
  - **Cache verimliliği:**
    - `cache_read / (input + cache_read + cache_write)` oranı.
    - 5m ve 1h cache yazım dağılımı.
    - "Cache sayesinde tasarruf ≈ $X" (cache okuma ile tam input fiyatı arasındaki fark).
  - **Meta veri:** git branch, Claude Code sürümü, entrypoint, `cwd`. `cwd` için "klasörü aç" butonu sunulur; `opener` beyaz listesi buna izin verecek şekilde ayarlanır.
- [ ] **Projeler ekranı:**
  - Varsayılan görünüm düz listedir. "Klasöre göre grupla" seçeneği ortak önek ağacını gösterir (`D:\Projects\Araclar\aktif\…`).
  - **Kolonlar:** Ad · Yol (kısaltılmış) · Oturum sayısı · Son aktivite · Maliyet (aralık) · Haftalık etki · Trend sparkline'ı.
  - Arama; yol parçalarıyla eşleşir.
- [ ] **Proje detayı:**
  - Haftalık etki trendi (son 8 hafta).
  - Oturum listesi (Oturumlar bileşeni, proje filtresi kilitli).
  - Model karışımı.
  - En pahalı 5 oturum.
- [ ] **Navigasyon:** Genel Bakış'taki "en çok tüketenler" kartları ilgili detay sayfalarına bağlanır. Detay sayfalarında "← Geri" ve breadcrumb bulunur.
- [ ] **Testler:**
  - Sanal liste mantığı (Vitest).
  - Filtrenin hash'e serileştirilmesi.
  - Biçimlendirme.
  - Büyük veri performansı (Adım 14'teki sentetik DB ile manuel ölçüm).

## Teknik notlar ve tuzaklar

- **Oturum adları** çok uzun olabilir. Ortadan kısaltılır (`…`); tam ad tooltip'te gösterilir.
- **`cwd` farklılıkları:** Bir oturum çalışma sırasında dizin değiştirebilir. Proje ataması **ilk** `cwd` değerine göre yapılır. Farklı `cwd` değerleri detay sayfasında listelenir.
- **Silinmiş transcript'ler:** Claude Code'un sildiği dosyalara ait oturumlar listede görünmeye devam eder, yanında "transcript silindi" rozeti bulunur. Ham olaylar saklama süresi dışındaysa zaman çizelgesi `usage_5m` verisinden çizilir.
- **Proje sayısı:** 1.500 projeden çoğu tek oturumluk ve geçici olabilir (örneğin `AppData\Local\Temp\…`). "Geçici dizinleri gizle" filtresi varsayılan olarak açık olsun. Desenler `%TEMP%`, `\Temp\` ve ayarlardan özelleştirilebilir liste.

## Kabul kriterleri

- [ ] Gerçek veriyle (binlerce oturum) liste kaydırma ve filtreleme akıcıdır (kare < 16 ms, filtre yanıtı ≤ 150 ms).
- [ ] Oturum detayında etki tablosu Adım 14 atıflarıyla birebir tutarlıdır (rastgele 5 oturum kontrol edilir).
- [ ] Hiçbir ekranda konuşma içeriği gösterilmez.
- [ ] Klavye ve ekran okuyucu ile temel akışlar (listeden detaya geçiş ve geri dönüş) kullanılabilir.

## Doğrulama

```powershell
pnpm -C app test; pnpm -C app check
pnpm dev   # Oturumlar → filtre → detay → proje → geri
```

## Devir notu

_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
