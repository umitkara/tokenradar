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

## Kullanıcıyla kararlaştırılanlar (2026-11-20)

- **Adım 16a + 16b'ye bölünür.**
  - 16a: sanal liste, Oturumlar listesi ve Oturum detayı, şema v4 (oturum `cwd` ve effort değerleri), fast mode ve "transcript silindi" rozetleri, klasör açma komutu.
  - 16b: Projeler listesi ve Proje detayı, geçici dizin filtresi ve desen düzenleyicisi, klasör ağacı, Genel Bakış bağlantıları, gerçek veriyle performans ölçümü.
- **Eksik veri şema v4 ile eklenir.** Çekirdek effort'u ve oturumun farklı `cwd` değerlerini saklamıyordu. Ön keşifte (yalnızca alan adları sayıldı) assistant kayıtlarında `effort` alanı vardı (`low`/`medium`/`high`/`xhigh`). Son 400 transcript'in 303'ünde birden fazla farklı `cwd` vardı. `speed: "fast"` hiç görülmedi.
- **"Klasörü aç" bir arka uç komutudur.** Yol yalnızca veritabanından okunur; frontend rastgele bir yol gönderemez ve pencereye `opener` izni verilmez.
- **Geçici dizin desenleri Ayarlar'da düzenlenir** (16b).

### 16b öncesi kararlar (2026-11-21)

Gerçek veride (salt-okunur sayım) projelerin yaklaşık %80'i `AppData\Local\Temp` altında.

- **Desen biçimi:** yol parçası. Desen yolun herhangi bir yerinde eşleşir; `*` joker, `%TEMP%` gibi ortam değişkenleri açılır, büyük/küçük harf duyarsız. Varsayılanlar `%TEMP%` ve `\Temp\`. Filtre çekirdekte SQL ile uygulanır.
- **Kapsam:** Projeler sayfası ve Oturumlar listesindeki proje seçicisi. Oturum listesi ve Genel Bakış toplamları hiçbir oturumu gizlemez.
- **Klasör ağacı çekirdekte kurulur:** ortak önek ağacı, tek çocuklu zincirler birleşik, her klasörde toplamlar. UI yalnızca aç/kapa durumunu tutar.
- **Gerçek veri ölçümü:** kurulu uygulamayı kullanıcı kapatıp açar; ölçüm sırası gelince haber verilir.
- **Şema v5: görünen yol.** `projects` tablosu orijinal yazımlı yolu da saklar; sayfa ve ağaç onu gösterir. Kurulu sürüm hâlâ v2'de olduğu için v4'ün tam indekslemesiyle aynı yükseltmede dolar. Transcript'i silinmiş eski projeler küçük harfle kalır (yan ajan notu üzerine kullanıcı kararı).

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

### 16a (2026-11-20, ADR-068)

**Yapılanlar:**
- **Çekirdek, şema v4** (`0004_session_attrs.sql`):
  - `session_attrs` tablosu: oturum başına farklı `cwd`, `effort` ve hızlı mod değerleri, ilk ve son görüldükleri anla.
  - `ix_source_session` indeksi.
  - Okuma konumları sıfırlanır; ilk açılış bir tam indeksleme kadar sürer.
- **Çekirdek, okuma ve sorgular:**
  - Kayıt ayrıştırıcı `effort`'u esnek okur (`LooseStr`; nesne ya da sayı gelirse yok sayılır).
  - İndeksleyici öznitelikleri parça başına toplar, coordinator tek transaction'da yazar.
  - `SessionRow.transcript_deleted`.
  - `SessionDetail` yeni alanlar: `cwds`, `efforts`, `fast`.
  - `TimelineBucket.family`.
  - `session_dirs(id)`: ağacın `cwd`'leri ve çözülmüş proje yolu.
- **Uygulama:** `open_session_dir(id, path)`. Yalnızca `session_dirs`'teki yolları açar; olmayan dizin için `"missing"` döner. Pencereye opener izni verilmedi.
- **Frontend:**
  - `lib/virtual.ts` ve `VirtualList.svelte`: sabit satır yüksekliği, ARIA grid, klavye, sona yaklaşınca sayfa isteme.
  - Router `?query` taşır; sayfalar `{param, query}` alır.
  - `sessions/filter.ts`: filtrelerin URL parçasına yazılıp okunması.
  - `sessions/view.ts`: ortadan kısaltma, model etiketi, etki seçimi, aile yığınları, bantlar, eksen.
  - `SessionList`, `SessionDetail`, `CostBars`, `Badge`.
  - `routes/Sessions.svelte`: liste ve `#sessions/<id>` ayrıntısı.
- **Tasarım:** üç yeni seri token'ı (`--series-3/4/5`). dataviz doğrulayıcısıyla tüm çiftler, iki tema ve en zor kart zemini denetlendi; `tokens.test.ts` kontrast denetimine eklendi.
- **Galeri:** sentetik oturum listesi ve ayrıntısı (`gallery/sessions.ts`). `?rows=N` liste boyunu ayarlar.
- **Belgeler:**
  - architecture §6: şema v4.
  - design-language: token tablosu, oturum maliyet grafiği, oturum listesi, sözlük.
  - ADR-068.
  - `docs/perf/2026-11-20-sessions.md`.

**Doğrulama:**
- Rust: `cargo fmt --check` ve `cargo clippy --workspace --all-targets -D warnings` temiz. Core 586, app 104, CLI 29 test geçti.
- Frontend: `pnpm -C app test` 205/205 geçti; `check` ve `lint` temiz.
- Build: `pnpm -C app build` başarılı. `check-bundle-size`: `main.html` JS 95,3 / 180 KB (giriş ve tüm tembel sayfalar, gzip); Oturumlar parçası 12,2 KB.
- 10.000 satır: kaydırma adımı başına iş p95 1,7 ms, en kötü 6,4 ms (headless Edge, galeri).
- Tam indeksleme (gerçek dizin, geçici DB, A/B): şema v4 20,1 s, `main` 17,1 s (2. tur; CPU ikisinde de 6,5 s civarı; ≤ 60 s). Aynı veriyi okuyan koşuların parmak izi aynı: kullanım toplamları değişmedi.
- Görsel inceleme: galeri headless Edge'de açık/EN ve koyu/TR çekildi.
  - Bulunup düzeltilenler:
    - Grafik saat etiketleri son yığının renginde çiziliyordu.
    - TR'de süre sütunu iki satıra bölünüyordu.
    - Efor düzeyleri çevrilmemişti.
    - Sütun ve cache terimleri sözlükle uyuşmuyordu ("Pay" → "Etki", "Önbellek" → "Cache").
  - Sentetik ayrıntıda limite ulaşmış 5 saatlik pencere oturumun tamamını kapladığı için grafik zemini baştan sona kırmızımsı görünüyor. Veri böyle olduğu için beklenen davranış.
- **Yapılmayan:** `pnpm dev` ile gerçek uygulamada gezinti.
  - Bu derleme gerçek veritabanını v4'e yükseltir; kurulu sürüm sonra o veritabanını açamaz (ADR-054'teki gibi).
  - Gezinti, 15a'daki gibi gerçek veritabanının portable bir kopyasıyla ve kurulu uygulama kullanıcı onayıyla kapatılarak 16b'nin gerçek veri ölçümüyle birlikte yapılacak. Doğrulamadaki "proje → geri" adımı zaten 16b'nin kapsamında.

**Sapmalar:**
- Adım ikiye bölündü; öznitelikler şema v4 ile saklanıyor; klasör arka uçtan açılıyor (kullanıcı kararları).
- Maliyet grafiği uPlot değil canvas, yığınlar model değil aile (ADR-068).
- Sanal liste sayfayı `IntersectionObserver` yerine çizilen dilimin sona yakınlığıyla istiyor.
- Proje seçici aranabilir combobox değil, yerel `<select>` (ilk harflerle arama).
- Kabul kriterlerinden "filtre yanıtı ≤ 150 ms (gerçek veri)" ve "etki tablosu Adım 14 atıflarıyla tutarlı (5 oturum)" 16b'nin gerçek veri koşusunda denetlenecek. Etki tablosu aynı saklanan atıfı okuyor (`get_session`), ayrı bir hesap yapmıyor.

**Karar (oturum sonunda, 2026-11-21):** Aşağıdaki öneriler kullanıcı tarafından onaylandı; ADR-068 kabul edildi.

**Onaylanan öneriler (ADR-068):**
- Aileye bağlı dört renk ve canvas grafik.
- Filtrelerin URL biçimi; aramanın geçmişe kayıt eklemeden parçayı değiştirmesi.
- Yerel proje seçici.
- Liste ve ayrıntıda 10 sn'lik canlı yenileme kısması.

**Bilinen sınırlar:**
- Efor ve klasörler yalnızca bu sürüm ilk okuduğunda transcript'i hâlâ diskte olan oturumlar için bilinir. Daha eski oturumlarda ayrıntı sayfası "Kayıtlı klasör yok" der ve nedenini açıklar (yan ajan notu, kullanıcı iletti).
- Yükseltmeden sonraki ilk açılış bir tam indeksleme kadar sürer.

**16b'ye notlar:**
- `SessionList` bir proje kilidi almıyor; proje ayrıntısı için `toFilter(q, now, locked)` hazır. Bileşene `locked` prop'u eklenmeli ve proje seçici gizlenmeli.
- Genel Bakış'taki "en çok tüketenler" bağlantıları: `hrefFor("sessions", String(id))`.
- `listMemory.query`, ayrıntıdan "Oturumlara dön" ile aynı filtrelere döner. Projeler için benzeri gerekebilir.
- Gerçek veri koşusu (portable kopya):
  - 10.000+ satırda boyama dahil kare süresi.
  - Filtre yanıtı.
  - 5 oturumda etki tablosunun atıflarla karşılaştırılması.
  - Klasör açma ve `"missing"` durumu.
- `main-soon-projects*` anahtarları Projeler sayfası gelince kaldırılmalı.

### 16b

**Yapılanlar (2026-11-21, `step/16b-projects`, ADR-069):**
- **Çekirdek:**
  - Şema v5: `projects.display_path`. Yollar orijinal yazımla görünür; göç canlı dosyaları yeniden okutur.
  - `[data] temp_dirs` desenleri (`temp_like_patterns`) ve SQL `LIKE` filtresi. `list_projects` ilk sayfada gizlenen proje sayısını döner (`hidden`).
  - `project_tree`: büyük/küçük harf duyarsız klasör ağacı, düğüm toplamları, tek çocuklu zincirlerin birleşmesi.
  - "Haftalık" değeri pencere başına ortalama.
  - Benchmark'a `list_projects/hide_temp` ve `project_tree/cost` eklendi.
- **Ana pencere:**
  - Projeler sayfası: düz liste ya da klasör ağacı, arama (Ctrl+F), aralık, sıralama, "Geçici dizinleri gizle" ve gizli proje notu. Filtreler URL parçasında (`#projects?…`).
  - Proje ayrıntısı: başlık okumaları, son 8 haftalık pencerede pay, model karışımı, en pahalı 5 oturum, projeye kilitli oturum listesi, "← Projelere dön".
  - `SessionList` için `locked` ve `base` prop'ları eklendi. `VirtualList` için `onkey` ve `select(index)` eklendi (ağaçta ← →).
  - Genel Bakış'taki "En çok tüketenler" adları ayrıntı sayfalarına bağlanıyor.
  - `main-soon-projects*` anahtarları kaldırıldı.
- **Ayarlar → Gizlilik ve veri:** "Geçici dizinler" kartı. Desen eklenir, kaldırılır, "Varsayılanlara dön" ile sıfırlanır; varsayılanlar arka uçtan gelir (`PrefsView.default_temp_dirs`).
- **Galeri:** Projeler listesi, ağaç ve ayrıntı (sentetik). Ayrıntıdaki oturum listesi de sentetik satırlarla dolu.
- **Testler:**
  - Rust: desen genişletme, ağaç kurma, gizli sayısı, haftalık ortalama, görünen yol, yapılandırma.
  - Vitest: proje filtresi URL gidiş-dönüşü, görünür düğümler, üst düğüm, hafta çubukları.
- **Gerçek veri koşusu (`docs/perf/2026-11-21-projects.md`):**
  - Kurulum: release derlemesinin portable kopyası ve gerçek veritabanının kopyası. Kurulu uygulamayı kullanıcı kapatıp açtı; kopya koşudan sonra silindi.
  - Filtre yanıtı: Projeler 49–110 ms, Oturumlar 28–113 ms.
  - Boyama dahil kare aralığı vsync'te sabit, atlanan kare yok.
  - Etki tablosu 15 oturumda saklanan atıflarla birebir eşit.
  - Klasör açma: var olan klasör açıldı, silinmiş olan `"missing"`, bilinmeyen yol `"not allowed"` döndü.
  - Gezinti: liste → ayrıntı → geri, aynı filtrelerle.
  - Göçten sonraki ilk tam indeksleme ≈ 28 s.
- **Görsel inceleme** (galeri, açık/EN ve koyu/TR). Düzeltilen kusurlar:
  - En pahalı oturumlarda sütunlar hizasızdı; tek ızgaraya alındı.
  - Klasör satırındaki "N proje" mono yazı tipindeydi.
  - Açık klasörde → ilk çocuğa inmiyordu.
  - Galeride gömülü oturum listesi boş kalıyordu.

**Kullanıcıyla kararlaştırılanlar:**
- Adım başındaki dört karar ve şema v5 (yukarıda, "16b öncesi kararlar").
- **"Haftalık" sütunu pencere başına ortalamadır** (yan ajan notu üzerine). Değer toplam bölü aralığın dokunduğu haftalık pencere sayısıdır ve `≈%4,5/hafta` biçiminde gösterilir. Öncesinde değer toplamdı; 30 günlük varsayılanda bile %100'ü aşabiliyordu.

**Karar (oturum sonunda, 2026-11-21):** Aşağıdaki öneriler kullanıcı tarafından onaylandı; ADR-069 kabul edildi.

**Onaylanan öneriler (ADR-069):**
- Proje listesinin varsayılanları: son 30 gün, son etkinliğe göre sıralı, ağaç kapalı. Ağaç açılınca bütün klasörler açık başlar.
- Filtrelerin URL biçimi; `temp=1` "göster" demektir.
- Düz listede satırlar bağlantı değildir, oturum listesi gibi tıklama ya da Enter ile açılır. Ağaçta proje adı bağlantıdır.
- `VirtualList`'e `onkey` ve `select` eklenmesi, ağaçta WAI-ARIA ← → davranışı.
- Gizli proje sayısı için ayrı `EXISTS` sorgusu.
- Proje ayrıntısının yerleşimi: hafta çubukları `--series-2`, tahmini pencereler kesikli, 8'den az haftada sağa yaslı.
- Varsayılan desenlerin `PrefsView` ile gönderilmesi.

**Sapmalar:**
- Gerçek veride 10.000 satırlık liste yok: en uzun liste birkaç bin satır. Boyama dahil ölçüm o listeyle yapıldı. 10.000 satır yalnızca sentetik olarak (boyama hariç) ölçüldü; 10.000 satırda boyamanın da bütçede kaldığı bir çıkarım.
- Haftalık ortalamaya geçiş gerçek veri ölçümünden sonra yapıldı. Filtre yanıtı bu değişiklikten sonra gerçek veride yeniden ölçülmedi; sentetik sorgu süreleri bütçede.
- Kabul kriterindeki "breadcrumb", 16a'daki gibi tek bir "← Projelere dön" bağlantısı (gezinti işaretli `nav`).

**Bilinen sınırlar:**
- Transcript'i silinmiş eski projeler küçük harfli yolla kalır.
- Geniş bir desen (`\Temp\`), adında "Temp" klasörü geçen gerçek bir projeyi de gizler; not ve "Göster" bağlantısı bunu görünür kılar.
- Haftalık ortalama, tek haftada yoğun çalışılmış bir projeyi seyreltir. O haftanın payı ayrıntıdaki çubuklarda görünür.
- Oturum ayrıntısından "geri", projeden gelinse bile Oturumlar listesine döner.
- Ağacın kökü yalnızca "D:" diye görünür (`D:\` daha doğal olurdu). Dar pencerede sıralama seçicisi filtre satırında ikinci satıra kayar (oturum listesinde de aynı).

**17'ye notlar:**
- Kurulu sürüm hâlâ şema v2'de. v0.2 ile gelen ilk açılış v3, v4 ve v5 göçlerini birlikte yapar ve bir tam indeksleme kadar sürer (gerçek veride ≈ 28 s).
- Gerçek veri koşusu betikleri `%TEMP%\tr16b` altında (kurulum, filtre ve kare ölçümü, etki karşılaştırması, klasör açma). Git'te değiller.
- Pencereler ve Örüntüler sayfaları, Projeler'in filtre ve URL yardımcılarını (`sessions/filter.ts` → `rangeFilter`, `parseRange`, `writeRange`) kullanabilir.
