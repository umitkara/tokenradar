# Adım 12k — Statusline düzen çekirdeği: ayraç öğeleri, boşluklar, hizalama

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1.x | 12j | 1-2 oturum | ⬜ |

## Amaç

12i ve 12j ile statusline öğe tabanlı oldu ve bir düzenleyici kazandı. Kullanıcı düzen üzerinde daha fazla söz hakkı istiyor:

- **Ayraçlar öğe olsun.** Bugün `│` ayraçlarını `compose_plain`, `value_sep` ile iki değerin arasına kendisi koyuyor. Listede görünmüyorlar ve tek tek kaldırılamıyorlar.
- **Hizalama.** Bazı öğeler sağa yaslanabilsin ya da ortalanabilsin.
- **Çok satırlı serbest düzen.** Bu adım çekirdeği kurar; satır kartları Adım 12l'dedir.

Bu adım config modelini, göçü, render'ı, şablonları ve IPC'yi kapsar. Düzenleyici arayüzü Adım 12l'dedir.

## Önce oku

- ADR-042, ADR-044, ADR-049, ADR-050
- `docs/architecture.md` §11 (statusline akışı, p95 < 30 ms)
- `docs/performance-budget.md` (CLI soğuk başlatma, CLI boyutu, statusline bütçeleri)
- `docs/data-sources.md` S1
- Adım 12i ve 12j'nin devir notları
- `crates/tokenradar-core/src/statusline/line/{prefs.rs, render.rs, presets.rs, tests.rs}`, `crates/tokenradar-cli/src/render.rs`

## Kullanıcıyla kararlaştırılanlar (2026-11-08)

- **Ayraçların hepsi öğe olur.**
  - Otomatik ayraç kalkar; listede ne görünüyorsa o çizilir.
  - Eski config'ler açılırken ayraçlar öğe olarak eklenir ve satır aynı görünür.
  - Düzenleyicide yeni bir değer eklenince, satırdaki önceki öğe de bir değerse araya otomatik bir ayraç girer (12l). Kullanıcı istemezse siler.
- **Hizalama "esnek boşluk" (`flex`) öğesiyle yapılır.** Öğe başına hiza ayarı yoktur; liste sırası tek doğru kaynaktır.
  - Bir `flex` konursa ondan sonrası sağa yaslanır.
  - İki `flex` konursa ortadaki grup gerçekten ortalanır, sağdaki grup sağa yaslanır.
  - Ayrıca sabit genişlikli bir `space` öğesi eklenir.
- **Düzenleyicide satır kartları** kullanılır (12l). Veri düz öğe listesi olarak kalır.
- **İki adım:** 12k çekirdek, 12l düzenleyici.

## Kapsam

### 0. Önkoşul gözlemi: genişlik ve kenarlar

Sağa yaslama ve ortalama, satırın gerçek genişliğini bilmeyi gerektirir. Kod yazılmadan önce gerçek Claude Code'da gözlenir. Gözlem 12i'deki `NO_COLOR` probu gibi yapılır.

- **Kurulum:** Prob, kullanıcı onayıyla settings.json akışı üzerinden kurulur: diff → onay → backup → atomic write.
- **Probun işi:**
  - Mevcut TokenRadar komutunu zincirler, ingest kesilmez.
  - `COLUMNS` değerini bir dosyaya yazar.
  - Sağ ucunda bir işaret olan üç test satırı basar. Satırların genişliği sırasıyla `COLUMNS`, `COLUMNS-1` ve `COLUMNS-2`'dir.
  - Başı boşlukla başlayan bir satır ve ortalanmış bir işaret basar.
  - `⏱`, `▲` ve `≈` gliflerini bir sütun cetvelinin yanına çizer.
- **Gözlem bitince** backup'tan geri dönülür.
- **Kullanıcı ekran görüntüsüyle şunları bildirir:**
  - Hangi genişlik sarmadan tam sağa oturuyor?
  - Baştaki boşluklar korunuyor mu?
  - Pencere boyutu değişince satır yeniden çiziliyor mu, `COLUMNS` güncelleniyor mu?
  - Sağdaki bildirim alanıyla (örneğin auto-compact uyarısı) çakışma oluyor mu?
  - Gliflerin terminalde kaç hücre kapladığı.
- **Sonuca göre:**
  - Varsayılan sağ pay (`margin`) belirlenir.
  - Glif genişlik istisnaları belirlenir.
  - Baştaki boşluk kırpılıyorsa ya da satır kararsızsa **dur, kullanıcıyla kapsamı yeniden konuş.** Örneğin `flex` yalnızca dolgu olarak çalışabilir.

### 1. Config modeli (`prefs.rs`)

```toml
[statusline]
version = 2                # 2: ayraçlar öğedir
theme = "plain"
separator = "│"            # yeni eklenen ayraçların glifi; eski build'ler için de yazılır
color = "auto"
margin = 2                 # sağ pay, 0–8; varsayılan gözlemden gelir

[[statusline.items]]
kind = "seven_day"

[[statusline.items]]
kind = "separator"
text = "│"

[[statusline.items]]
kind = "flex"              # kalan genişliği doldurur

[[statusline.items]]
kind = "space"
width = 2                  # 1–8
```

- **Yeni türler** (grup `Layout`):
  - `space`: `width` kadar boşluk çizer. `width` 1–8, varsayılan 1.
  - `flex`: esnek boşluk.
  - `Item`'a `width: u8` eklenir. Yalnızca `space` kullanır; diğer türlerde normalize edilir.
- **`version`:** `StatuslinePrefs`'e eklenir, her zaman `2` yazılır.
- **Okuma göçü** (`items` var ama `version < 2`):
  - Her satırda yan yana duran iki değer öğesinin arasına bir `separator` öğesi eklenir. `custom_text` de değer sayılır.
  - Glif global `separator`'dan gelir. Global ayraç boşsa onun yerine `space` eklenir.
- **12c göçü:** `legacy()` türetmesi de aynı göçten geçer. `items` olmayan 12c tablosu da aynı görünür.
- **Şablonlar** (`presets.rs`) da aynı göçten geçer.
- **Global `separator` kalır.** Yeni eklenen ayraçların glifi olarak kullanılır ve eski build'ler için yazılmaya devam eder. 12c anahtarlarının (`segments`, `layout`, `bar`, `pace`) türetilmesi değişmez.
- **`margin: u8`** eklenir: sağ pay, 0–8.
- **Sınırlar:** `MAX_ITEMS` 48'den 64'e çıkar, çünkü ayraçlar artık öğe sayısını yaklaşık ikiye katlıyor. `KNOWN_KEYS` güncellenir. `RawItem` ve `Raw` alanları esnek okumayla eklenir (ADR-042: okuma hiçbir zaman başarısız olmaz).

### 2. Render (`render.rs`)

- **Birleştirme kuralı (`compose_plain`):**
  - `value_sep`'in otomatik eklenmesi kalkar.
  - Yan yana iki öğe arasına tek bir boşluk konur.
  - `space` ve `flex` kendi genişliklerini taşır; yanlarına ayrıca boşluk eklenmez.
- **Ayraç daraltma:**
  - Bir ayraç yalnızca aynı bölgede iki gösterilen öğenin arasındaysa çizilir. Bölge, `flex`'ler arasındaki kısımdır.
  - Bölgenin başındaki ve sonundaki ayraçlar düşer. Art arda gelen ayraçlar bire iner.
  - Bu kural, değeri olmayan bir öğenin (örneğin bu hesapta olmayan `five_hour`) `│ │` bırakmasını önler.
  - Bugünkü otomatik ayraçla aynı sonucu verdiği için göçten sonra satırın görünen metni değişmez.
- **Bölgeler ve hizalama:** `flex` satırı bölgelere ayırır. Kullanılabilir genişlik `W = width − margin`'dir.
  - **0 flex:** satır bugünkü gibi çizilir.
  - **1 flex:** sol bölge solda, sağ bölge sağa yaslı.
  - **2 flex:** sol, orta ve sağ bölge.
    - Orta bölge `(W − w) / 2` sütununa konur.
    - Komşularından en az bir boşluk uzakta kalacak şekilde sıkıştırılır.
  - **3 ve daha fazla flex:** artan boşluk eşit dağıtılır, kalan sütunlar sondaki flex'e verilir.
  - **Genişlik bilinmiyorsa** (`COLUMNS` yok): `flex` tek bir boşluk olarak çizilir.
- **`fit()`:**
  - Ölçüm sırasında her `flex` en az 1 sütun sayılır.
  - Düşme sırası aynı kalır. `drop_at`, öğenin önündeki ayraç, metin ve `space` öğelerini birlikte atar. `flex` hiçbir zaman düşmez.
  - Satır sığdıktan sonra artan boşluk `flex`'lere dağıtılır.
- **Görüntü genişliği:**
  - `chars().count()` yerine `unicode-width` crate'i kullanılır. Bu yeni bir bağımlılıktır; CLI boyut farkı ölçülür.
  - Gözlemde farklı genişlik çıkan glifler için küçük bir istisna tablosu tutulur.
- **Powerline:**
  - `flex`, blok zincirini kapatır: önce bitiş glifi, sonra arka plansız boşluk, ardından yeni zincir.
  - Son `flex`'ten sonraki bölge sola bakan gliflerle çizilir (U+E0B2; yuvarlak temada U+E0B6).
  - `space` arka plansız boşluk olarak çizilir. `separator` bugünkü gibi atlanır.
  - Renk kapalıyken plain temaya düşme kuralı aynı kalır.
- **Span'ler:** `flex` ve `space` span'leri `item` indeksini taşır. 12l önizlemesinde bu span'lere tıklanınca ilgili öğe seçilebilir.

### 3. Şablonlar ve IPC

- **Yeni şablon:** `split`. Solda limitler, ardından `flex`, sağda model, cwd ve git dalı.
- **Katalog:** `statusline_catalog`, `space` ve `flex`'i de döndürür.
- **IPC tipleri:** `version`, `margin` ve `width` alanlarıyla genişler. Binding'ler yeniden üretilir.
- **Frontend:** Yalnızca derlenecek kadar uyarlanır. `newItem` yeni alanları doldurur. Toolbar'daki ayraç alanı yeni eklenen ayraçların glifini ayarlar. Asıl arayüz 12l'dedir.

### 4. Fluent ve dokümanlar

- `cli.ftl` dosyasına gerekirse yeni anahtar eklenir; EN ve TR eşit olur.
- ADR-051 yazılır: ayraç öğeleri, göç, `flex` bölgeleri, görüntü genişliği, sağ pay, gözlem sonucu.
- Güncellenecek yerler:
  - architecture §11
  - `docs/data-sources.md` S1 (`COLUMNS` ve kenar davranışı gözlemi)
  - README `[statusline]` örneği
  - CHANGELOG (Unreleased)

## Kapsam dışı

- Düzenleyici arayüzü ve satır kartları (Adım 12l).
- 12i'den kalan açık konular: Powerline'da `▲` rengi, stdin'de UTF-8 BOM.
- `⏱` glifini değiştirmek. Gözlem gerekli gösterirse devir notuna yazılır.
- Öğe başına hiza ayarı (karar: `flex` öğesi).
- Kurulu sürümü güncellemek veya sürüm çıkarmak.

## Görevler

- [ ] Genişlik ve kenar gözlemi (kullanıcıyla); sonuçlar bu listeye yazılır.
- [ ] `space` ve `flex` türleri, `Item::width`, `version`, `margin`, esnek okuma.
- [ ] Ayraç göçü (12c, 12i biçimi, şablonlar) ve yazma.
- [ ] `compose_plain` yeni birleştirme kuralı ve ayraç daraltma.
- [ ] Bölgeler ve hizalama; `fit()` uyarlaması; görüntü genişliği.
- [ ] Powerline bölgeleri.
- [ ] `split` şablonu, katalog, IPC tipleri, binding'ler, frontend derlenme uyarlaması.
- [ ] Fluent (EN/TR), ADR-051, dokümanlar.
- [ ] Performans ölçümü (`main`'e karşı A/B) ve CLI boyut farkı, `docs/perf/` kaydı.

## Teknik notlar ve tuzaklar

- **Geriye uyum testi:** 12c ve 12i'nin render testleri, `version` alanı olmayan config'lerle aynı görünen metni vermelidir. ANSI dizileri yalnızca boşlukların boyanmasında farklılaşabilir. Testler görünen metni karşılaştırır.
- **Göç bir kez yapılır.** Göçten sonra yazılan dosyada `version = 2` bulunur. Aynı dosya tekrar okunduğunda göç bir daha uygulanmaz, ayraçlar çoğalmaz. Bunun için ayrı bir test yazılır.
- **Eski build uyumu:** Kurulu v0.1.0 yalnızca 12c anahtarlarını okur ve kendi otomatik ayracını çizer. `space` ve `flex` onun için yoktur.
- **Genişlik bütçesi:** `unicode-width` tabloları CLI boyutunu büyütür. Fark ölçülür; bütçe aşılırsa dur ve kullanıcıya bildir.
- **Kenar durumları:** Boş bölge, art arda iki `flex`, yalnızca `flex` içeren satır ve dar terminalde orta bölgenin sığmaması test edilir.
- **Gerçek spool ve pipe'a dokunulmaz:** Testler portable kopya ve geçici dizinlerle yapılır.
- **Gerçek veriyle ölçüm:** Ölçüm commit'lerine token toplamı, sayı veya yol yazılmaz.

## Kabul kriterleri

- `version` alanı olmayan her config (12c ve 12i biçimi) göçten sonra aynı görünen metni verir.
- Değeri olmayan bir öğe çift ayraç bırakmaz. Bölge sınırındaki ayraç çizilmez.
- 1, 2 ve 3 `flex` için konumlar doğrudur. Dar genişlikte her `flex` en az 1 sütundur.
- Powerline bölgeleri doğru çizilir. Renk kapalıyken plain temaya düşülür.
- Geniş glif içeren bir satır `W`'yi aşmaz.
- Dışa aktar → içe aktar gidiş-dönüşü aynı prefs'i verir.
- Statusline p95 < 30 ms; CLI boyut farkı kayıtlıdır.
- Ingest davranışı değişmez.

## Doğrulama

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
- `pnpm -C app check`, `pnpm -C app lint`, `pnpm -C app test`
- Core'u doğrudan çağıran geçici bir örnekle görsel kontrol (12i'deki gibi; commit'lenmez): `COLUMNS=120/80/50`, plain ve Powerline, 1 ve 2 `flex`.
- Kullanıcı `split` şablonunu gerçek Claude Code'da dener. Kurulu CLI yeni anahtarları bilmediği için bu deneme, yerel build kurulduktan sonra yapılır.
- `scripts/` altındaki statusline ölçüm betiği `-Config` ile çalıştırılır.

## Devir notu

_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
