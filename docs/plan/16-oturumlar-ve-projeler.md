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

_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
