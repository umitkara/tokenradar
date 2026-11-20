# Görsel Dil — "Native kabuk + enstrüman"

> Token değerleri Adım 09'da gerçek Mica zemin üzerinde denenerek sabitlendi (ADR-031). Kaynak `app/src/lib/styles/tokens.css`'tir;
> bu doküman onunla aynı tutulur. Kontrast eşikleri `tokens.test.ts` tarafından test edilir. Bileşenlerin tümü geliştirme
> derlemesindeki galeride (`gallery.html`) görülebilir; ekran görüntüleri `docs/plan/img/09-gallery-*.png`.

## 1. Fikir

- **Kabuk sessizdir.** Pencere zemini işletim sistemine aittir: Windows 11'de Mica (panelde Acrylic), macOS'ta vibrancy. Kontroller, yazı tipi ve köşe yarıçapları platformla uyumludur. Kullanıcı uygulamayı "Windows'un bir parçası" gibi hisseder.
- **Veri konuşur.** Limitler "enstrüman" diliyle gösterilir: iç içe halkalar, ince tick'ler, mono ve tabular rakamlar, fosfor tonlu vurgu rengi. Radar metaforu dekorasyon için kullanılmaz; bilgi taşıdığı yerde kullanılır. Tarama efekti yalnızca veri güncellendiği anda bir kez görünür.
- **Tek görsel gramer, her yüzeyde aynı:** **dış halka/üst çubuk = haftalık**, **iç halka/alt çubuk = 5 saatlik pencere**. Tray ikonu, panel, widget ve ana pencere bu eşlemeyi asla bozmaz.

## 2. Renk

OKLCH ile tanımlanır. Her token CSS `light-dark(açık, koyu)` çiftidir; etkin taraf `color-scheme` ile seçilir.

| Token | Koyu tema | Açık tema | Kullanım |
|---|---|---|---|
| `--surface-0` | Mica (şeffaf) / Win10 yedek `oklch(0.20 0.01 250)` | Mica / yedek `oklch(0.97 0.005 250)` | Pencere zemini (`--surface-0-solid`, Rust `SOLID_*` ile aynı) |
| `--surface-1` | `oklch(0.26 0.01 250 / 0.72)` | `oklch(1 0 0 / 0.70)` | Kartlar |
| `--surface-2` | `oklch(0.31 0.01 250 / 0.80)` | `oklch(1 0 0 / 0.90)` | İkincil düğme, hover |
| `--stroke` | `oklch(1 0 0 / 0.09)` | `oklch(0 0 0 / 0.08)` | Kart ve kontrol kenarı |
| `--text-1` | `oklch(0.96 0.005 250)` | `oklch(0.22 0.01 250)` | Birincil metin |
| `--text-2` | `oklch(0.78 0.01 250)` | `oklch(0.42 0.01 250)` | İkincil metin |
| `--text-3` | `oklch(0.68 0.01 250)` | `oklch(0.50 0.01 250)` | Soluk (bayat, düşük güven); yalnızca kart üzerinde |
| `--track` | `oklch(0.42 0.01 250 / 0.6)` | `oklch(0.86 0.01 250)` | Halka ve çubuk zemini |
| `--signal-ok` | `oklch(0.82 0.13 190)` (fosfor camgöbeği) | `oklch(0.50 0.11 200)` | Normal durum, veri mürekkebi; `--accent` |
| `--signal-warn` | `oklch(0.84 0.15 80)` (amber) | `oklch(0.53 0.13 65)` | Uyarı |
| `--signal-crit` | `oklch(0.72 0.18 25)` (mercan kırmızı) | `oklch(0.52 0.19 25)` | Kritik |
| `--signal-est` | `--text-2` + kesikli çizgi | aynı | Tahmini değerler |
| `--signal-other` | `oklch(0.74 0.07 300)` (soluk mor) | `oklch(0.50 0.09 300)` | "Diğer" kovası (claude.ai/Desktop) |
| `--series-2` | `oklch(0.66 0.13 260)` (mavi) | `oklch(0.53 0.18 275)` | Ana pencere grafiklerinde haftalık limitin çizgisi (Adım 15a, ADR-066) |
| `--on-accent` | `oklch(0.20 0.01 250)` | `oklch(1 0 0)` | Vurgu dolgusu üzerindeki metin (birincil düğme, toggle) |

**Kontrast ölçümü:** Kartlar, Mica/Acrylic'in açık ve koyu duvar kağıtlarıyla alabileceği zemin aralığı üzerine bindirilerek ölçüldü (açık: `L` 0.84–1.0, koyu: `L` 0.17–0.38; odak kaybındaki düz renk dahil). `--text-1`/`--text-2` hem kartta hem çıplak zeminde ≥ 4.5:1, `--text-3` kartta ≥ 4.5:1, sinyaller ≥ 3:1. Sinyal renkleri **asla gövde metni olarak kullanılmaz**; yalnızca yay, çubuk ve glif çizer. Başlangıç önerisindeki açık tema `ok`/`warn` değerleri ve koyu `--text-3` bu eşiklerin altında kaldığı için koyulaştırıldı/açıldı.

**Durum eşikleri** (varsayılan, ayarlanabilir): `< 75` ok, `75–90` warn, `≥ 90` crit.

**Renk körlüğü:** Durum asla yalnızca renkle ifade edilmez.
- Kritik durumda halkanın ucunda bir çentik ve metinde ikon bulunur.
- Uyarı durumunda üçgen glif kullanılır.
- Tahmini değerler kesikli çizgi ve "≈" işaretiyle gösterilir.

**Kontrast:** Metinde WCAG AA (4.5:1) sağlanır. Veri çizgileri zeminle en az 3:1 kontrast oluşturur.

## 3. Tipografi

- **UI metni:** `"Segoe UI Variable Text", "Segoe UI", system-ui` (Windows); macOS'ta `-apple-system`. Ek font yüklenmez.
- **Okuma değerleri (enstrüman rakamları):** **Geist Mono** (OFL, v1.8.2), `wght` ekseni korunarak alt kümesi alınmış variable woff2: **12,8 KB** (bütçe ≤ 25 KB). Kapsam: yazdırılabilir ASCII, Türkçe harfler (`çğıöşüÇĞİÖŞÜ`), `NBSP ≈ ▲ ▼ · − …`. Korunan OpenType özellikleri: `kern`, `tnum`, `zero`, `case`. Üretim: `scripts/subset-font.py`; lisans `app/src/lib/fonts/OFL.txt`. Mono yüz yalnızca okuma değerlerinde (`.readout`) kullanılır, etiket ve cümlelerde kullanılmaz.
- Tüm sayılarda `font-variant-numeric: tabular-nums` kullanılır, böylece geri sayım sırasında metin titremez.

**Ölçek:**

| Stil | Değer | Kullanım |
|---|---|---|
| `display` | 32/36 mono | Halka merkezi |
| `title` | 15/20 semibold | |
| `body` | 13/18 | |
| `caption` | 11/14 | |
| `micro` | 10/12 | Tick etiketleri |

## 4. Bileşenler

### Ring gauge (ikili halka)

- **Dış halka (haftalık):** kalınlık 6 px. **İç halka (5 saatlik pencere):** kalınlık 10 px. Halkalar arasında 4 px boşluk bulunur. 25/50/75 noktalarında 1 px tick'ler yer alır.
- **Projeksiyon yayı:** Mevcut hızla reset anında ulaşılacak yüzdeyi, dolu yayın devamında **soluk bir "hayalet" yay** olarak gösterir. Hayalet yay 100'ü aşıyorsa uç noktada crit rengi ve ▲ glifi kullanılır.
- **Merkez:** Büyük punto (`display`) ile 5 saatlik %, altında küçük punto ile yalnızca kalan süre (`1 sa 48 dk`); "… sonra yenilenir" cümlesi halka boşluğuna sığmaz, limit satırında tam cümle kullanılır.
- **Geometri (bileşen, 168 px tasarım boyu):** dış halka yarıçapı 77, iç halka 65; tick'ler 60–80 arası, yayların altında çizilir (yalnızca track üzerinde görünür). Tahmini yay `--signal-est` rengindedir ve maske ile kesikli çizilir. Kritik çentik: yayın sonundan önce %1,5'lik boşluk ve %1,5'lik uç. Hayalet yay %30 opaklıktadır; 100'ü aşarsa crit rengine döner ve 12 yönünde ▲ çizilir. Bayat veride yaylar %50 opaklık, merkez rakam `--text-3`.
- **Tek halka (5 saatlik pencere yok, ADR-041):** Hesapta 5 saatlik pencere yoksa haftalık limit iki halkanın bandını tek başına doldurur: yarıçap 70, kalınlık 20 (dış halkanın dış kenarından iç halkanın iç kenarına). Merkezde haftalık % ve kalan süre gösterilir. Boş bir iç halka çizilmez; boş iç halka "henüz okuma yok" demektir, "pencere yok" değil. Halkanın altında bilgi notu yer alır: "Bu hesapta 5 saatlik pencere yok. Yalnızca haftalık limitler geçerli."
- **Animasyon:** Değer değiştiğinde 400 ms ease-out ile geçiş yapılır ve bir kez radar taraması (conic gradient, 600 ms) oynar. **Sürekli animasyon yoktur.** Bu kural boşta CPU bütçesi gereği zorunludur.

### Limit satırı

`[ikon] Etiket ........ %72  ·  1 sa 48 dk` ve altında ince bir çubuk. Scoped limitler (örneğin "Fable · haftalık") girintili bir alt satırda gösterilir.
- **Zaman işareti (ADR-044, ADR-048):** Çubuğun üzerinde, pencerenin geçen zamanını gösteren 2 px'lik dikey çizgi (`--text-1`, kart renginde ince bir hâle ile). Saatle yer değiştirir, geçiş efekti yoktur. Penceresi bilinmeyen limitte çizilmez.
- **Panelde (`stacked`):** Halkanın yanındaki dar sütunda geri sayım çubuğun altına iner: `Etiket ..... %72`, çubuk, `1 sa 48 dk sonra yenilenir`. Haftalık ve scoped limitler mutlak zamanı gösterir (`Cum 17:35'te yenilenir`). (ADR-032)

### Oturum satırı

İki satır (ADR-032; tek satır 360 px'te çevirilerle sığmıyordu):
```
● ad  proje                 ≈ %9 bu pencere
  Meşgul · 12,4k tok/dk
```
- Ad sırası: oturum adı → başlık → proje → "Adsız oturum". Proje ad olarak kullanıldıysa tekrar yazılmaz.
- **Ayrıntı görünümünde (ADR-048):** İkinci satıra API karşılığı maliyet eklenir (`API $2,57`, tooltip'te "fatura değil"). Karşılaştıracak bir şey varken altında 3 px'lik bir bar çizilir: ≈pay kesikli (`--signal-est`), maliyet payı düz.
- Durum noktası: busy → nabız atan nokta (yalnızca 2 kez atar, sürekli değil); idle → içi boş daire. Canlı olmayan satırlar soluktur.

### Kaynak rozeti

Küçük bir rozet: `canlı` (S1), `API` (S2) veya `tahmini`. Rozetin tooltip'i son güncelleme zamanını gösterir. Özet görünümünde tek rozet alt bilgidedir ve birincil limitin (5 saatlik, yoksa haftalık) kaynağını gösterir; tahmini değerler satırlarda ayrıca `≈` ile işaretlenir. Ayrıntı görünümünde her limitin kendi rozeti vardır.

### Görünüm geçişi (panel)

Başlığın altında iki eşit düğmeli bir segment kontrolü: `Özet | Ayrıntı` (ADR-048). Seçili düğme `--track` zeminli ve kalındır. Klavye: ←/→, `Ctrl+Tab`. Limit verisi yokken gösterilmez. Pencereye sığmayan içerik (Ayrıntı, çok oturum) panelin içinde kayar; başlık ve geçiş de kayar. Kaydırma çubuğu ince ve oksuzdur (`scrollbar-width: thin`, `--text-3`).

### Zaman şeridi ve kullanım grafiği (panel, Ayrıntı)

- **Şerit:** 12 px yüksekliğinde, 3 px köşeli. Dolgu limitin sinyal rengindedir. Yerel gece yarıları noktalı (`--text-2`), reset'ten itibaren her 24 saat düz 1 px çizgi. Üstte gün adları (gün yeterince genişse), altta 24 saatlik işaretlerde zaman çizgisinin değeri (`--text-3`) ve kalın "▲ şimdi". Bir değer "şimdi" işaretine kelimenin uzandığı tarafta %20'den, öbür tarafta %10'dan fazla yaklaşmaz.
- **Grafik:** Şeritle aynı genişlikte, 96 px yüksekliğinde satır içi SVG. Kullanım eğrisi 2 px, altı %18 dolgulu; zaman çizgisi `--text-3` kesikli; 75/90 eşikleri sinyal renklerinde, soluk ve noktalı; gün çizgileri `--stroke`; "şimdi" çizgisi `--text-2`. Kenar çizgileri ölçekten bağımsızdır (`vector-effect: non-scaling-stroke`).
- **Açıklama:** Grafiğin altında: reset zamanı (limit satırlarındaki `Sal 21:52'de yenilenir` kalıbıyla), noktalı/düz çizgilerin anlamı, zaman çizgisi ve kullanım için küçük örnek işaretler.

### Ana pencere kabuğu (Adım 15a)

- Native başlık çubuğu + Mica (kullanıcı kararı). Başlık `Sayfa – TokenRadar` biçimindedir ve `document.title`'ı izler.
- Solda 200 px navigasyon: 16 px simge + etiket, 36 px satır. Seçili satır `--surface-2` zeminli, yarı kalın, simgesi `--accent` renginde; solda ayarlar penceresindeki gibi 3 px'lik seçim hapı. Ayarlar en altta ayrı bir düğmedir ve ayarlar penceresini açar.
- İçerik: `--type-heading` (600 20/28) sayfa başlığı, altında kartlar; boşluk `--space-4`.
- Kısayollar: `Ctrl+1..5` sayfalar, `Ctrl+,` ayarlar.

### Zaman grafiği (ana pencere, uPlot)

- Çizgiler 2 px; düz serinin altı %14 dolgulu, tahmin kesikli (`5 4`) ve dolgusuz.
- **Renk limite bağlıdır, sıraya değil** (ADR-066): 5 saatlik `--signal-ok`, haftalık `--series-2`, "Diğer" `--signal-other`. Tek çizgi kaldığında da haftalık mavi kalır.
- Bantlar: limit pencereleri dönüşümlü olarak `--text-1` %4 ile, limitine ulaşmış pencere `--signal-crit` %14 ile, tahmini pencere `--text-2` %8 ile boyanır.
- 75/90 eşikleri `--signal-warn` / `--signal-crit` %60, kesikli. Izgara `--stroke`, eksen yazısı `--text-2`, 11 px.
- İmleç dikey kesikli çizgi (`--text-3`) ve ortak ipucu: başlıkta zaman (`Per 19 Kas 14:05`), her çizgi için 12 px'lik örnek işaret + ad + değer (tahminde `≈`). Sağ kenara yaklaşınca ipucu sola döner.
- İki ve daha fazla çizgide grafiğin altında açıklama (legend) bulunur; tek çizgide kartın başlığı adı verir.
- Grafik yalnızca ekrandayken kurulur. Tema ya da dil değişince yeniden kurulur; genişlik değişince yalnızca boyutlanır. Animasyon yoktur.

### Oturum şeridi (zaman grafiği, Adım 15b)

- Grafiğin altında, aynı zaman ekseninde 10 px'lik bir şerit. Haftalık limitte payı en büyük 5 oturum (5 saatlik pencere yalnızca haftalık yoksa) 4 px'lik `--text-2` çubuklarla çizilir; payı bilinmeyen oturum daha soluktur.
- Konum ve genişlik uPlot'un çizim alanından (`bbox`) okunur; çok kısa oturum en az 6 px'tir.
- Her çubuk bir düğmedir: imleç ya da klavye odağı ortak ipucunu açar (oturum adı, zaman aralığı, `≈` pay). Renk taşımaz; kimliği ad verir.

### Büyüklük listesi (Genel Bakış)

- Etiket, en büyük satıra göre boyutlanan 6 px'lik çubuk, sağda değer. Tüm satırlar tek renk (`--signal-ok`): iş büyüklük, kimlik değil.
- Çubuğun veri ucu 3 px yuvarlak, taban düz; iz (track) yoktur. Değer `--text-2`, tabular rakam. İpucu tam değeri ve payı verir.
- Token türleri bu listeyle gösterilir, yığılmış çubukla değil: cache okuma diğer türleri bastırır (ADR-067). Model karışımı ilk 5 model + "Diğer modeller".

### Kıvılcım çizgisi (sparkline)

- 64×20 px, 1,5 px `--text-3` çizgi; son nokta 2 px yarıçaplı `--accent` daire. Eksen, ızgara ve etiket yoktur; satırın ipucu ne gösterdiğini söyler ("Bu haftanın günlerine göre maliyet").
- Düz seri tabana yatar. Yüksek kontrastta çizgi `CanvasText` olur.

### Segment kontrolü (ayarlar)

`Segmented` bileşeni iki ila dört kısa seçenek içindir; açılır liste bunlar için kullanılmaz. `radiogroup` olarak çalışır: oklar seçimi değiştirir, `Tab` gruptan çıkar. Seçili düğme `--surface-2` zeminli ve kalındır, altında 2 px `--accent` çizgi vardır. Kullanıldığı yerler: statusline teması, renk modu, örnek durum, terminal genişliği, öğe biçimi, sıfırlanma biçimi, çubuk genişliği ve çubuk stili. Bir seçenek isteğe bağlı bir glif örneği taşıyabilir; örnek `--font-term` ile, metinden önce ve `aria-hidden` olarak çizilir (Adım 12m: çubuk stilleri `██░░░`, `▬▬▭▭▭`, `⣿⣿⣀⣀⣀`).

### Statusline düzenleyicisi (ayarlar, Adım 12j)

- **Terminal önizlemesi:** Uygulama temasından bağımsız olarak koyudur. `--term-*` paleti Windows Terminal'in varsayılan 16 ANSI rengini kullanır, font `--font-term`'dir (Cascadia, varsa Nerd Font ya da PL yapısı). Satır sarmaz; seçilen genişlik kesikli bir sağ kenarla gösterilir ve taşan satır yatay kayar. Seçili öğenin span'lerinin altı çizilir; üzerine gelinen öğe 1 px çerçeveyle vurgulanır. Powerline bloklarında da görünsün diye arka plan değil çerçeve kullanılır.
- **Liste:** Seçili satır `--surface-2` zeminli ve kenarlıklıdır. Üzerine gelinen satır, metin renginin %7'siyle tonlanır, çünkü açık temada yüzeyler beyaz üstüne beyazdır. Sürüklemede bırakılacak yer 2 px `--accent` çizgiyle gösterilir; hareket yoktur (ADR-019). Gizli ve hesapta olmayan öğeler `--text-3` ile yazılır.
- **Renk örnekleri:** Terminal zemininde 22 px kareler. Seçili örneğin çevresinde `--term-fg` halka vardır.
- **Satır kartları (Adım 12l, ADR-052):** Her çıktı satırı `--surface-1` zeminli, `--radius-card` köşeli ve kenarlıklı bir karttır. Başlıkta "Satır N" (`--type-caption`, kalın) ve satırı silme düğmesi, altta kenarlıksız "+ Öğe ekle" bulunur. Kartların altında kesikli kenarlıklı "+ Satır ekle" yer alır. Boş kart, kesikli kenarlıklı bir yer tutucu gösterir ve sürüklenen öğeyi kabul eder.
- **Düzen satırları:** Ayraç, boşluk, esnek boşluk ve gizli satır sonu satırları 30 px'tir (değer satırları 36 px). Adları `--type-caption` ve `--text-2` ile yazılır. Esnek boşluk `⟷` işaretini ve satırdaki etkisini ("Sonrası sağa yaslanır"), boşluk genişliğini, ayraç glifini gösterir.
- **Önizlemede pay ve boşluklar:** Satırın boş bıraktığı sağ sütunlar (Claude Code'un 4 sütunu ve sağ pay) kenara kadar soluk çapraz taramayla gösterilir. Metni olmayan span'ler (boşluk, esnek boşluk) seçiliyken ve üzerine gelinince hafif beyaz bir tonla görünür olur.
- **Çubuk ve sıfırlanma ayarları (Adım 12m, ADR-053):** Çubuk genişliği (4/8/12) ve stili yalnızca biçim `bar` iken görünür. Sıfırlanma biçimi (Geri sayım / Saat), sıfırlanma gösteriliyorsa limit pencerelerinde görünür. Stil örnekleri UI'da çizilmez; backend'in `bar_sample` çıktısıdır.
- **Simgeler ve yeni çubuk stilleri (Adım 12p, ADR-056):** Simge seti seçici araç çubuğunda renk seçicinin ardından gelir. Nerd seti seçiliyken ya da tema Powerline iken ve bir Nerd Font kuruluyken açıklama satırı fontla ilgili notu ya da "Nerd Font simgelerini kullan" bağlantısını gösterir. Çubuk stili seçicisinin altındaki not, Powerline'da kapsül dışı stillerin çizilmediğini ya da kapsülün font ihtiyacını söyler. Dolgu örneği seçicide `--term-*` arka plan renkleriyle çizilir; renkler backend'in span'lerinden gelir. Düz temada öğe listesi de dolgu çubuğunun hücrelerini renkli çizer; diğer satırlar tek renktir. Araç çubuğu tek satırda kalsın diye "Şablonlar…" başlık satırında "Varsayılana dön…"ün yanındadır; Dışa aktar ve İçe aktar araç çubuğunun sonunda `IconButton`'dır (16 px yukarı ve aşağı ok, ipucu ve erişilebilir ad tam cümle). Bar stili seçicisi dar sütunda iki satıra kırılır (`flex-wrap`).
- **Terminal fontu (Adım 12o, ADR-055):** Giriş, önizlemenin altındaki açıklama satırında bir bağlantıdır (araç çubuğu tek satırda kalsın diye). Diyalog dört bölümdür: Nerd Font (durum, onay kutusu, ilerleme), Terminal, Terminal ayarları (satır başına Fontu seç / Geri al), Hepsini geri al. İndirme onayı ayrı bir `--surface-1` kutuda aile, sürüm, boyut, lisans ve tam URL'yi gösterir; ilerleme çubuğu `accent-color: var(--accent)`'tir. Diff ve geri dönüş özeti aynı diyalogun ikinci görünümüdür. Kurulu Nerd Font `--font-term`'in başına geçer; pencere onu çizemiyorsa (yeni kurulduysa) açıklama yeniden başlatmayı söyler.

### Tray ikonu

- **Boyutlar:** İkon, görev çubuğunun DPI'ına göre `SM_CXSMICON` boyutunda çizilir (16 @%100, 20 @%125, 24 @%150, 32 @%200). Golden testler bu dört boyutu kapsar.
- **Geometri** (`s = px / 16`; kalınlıklar tam piksele yuvarlanır, halkalar 12 yönünden saat yönünde dolar):

  | px | Dış halka | Boşluk | İç halka | Çentik | Nokta (çap) | Halo | Duraklat çubuğu |
  |---|---|---|---|---|---|---|---|
  | 16 | 2 | 1 | 3 | 1,5 | 5 | 1 | 1 × 5 |
  | 20 | 3 | 1 | 4 | 1,9 | 6 | 1 | 1 × 6 |
  | 24 | 3 | 2 | 5 | 2,3 | 8 | 2 | 2 × 8 |
  | 32 | 4 | 2 | 6 | 3 | 10 | 2 | 2 × 10 |

- **Renk:** Her halka kendi önemine göre renklenir (ok / warn / crit, eşikler 75/90). Track tray'de **opaktır**: koyu `rgb(68 72 77)`, açık `rgb(201 206 212)`. Paneldeki yarı saydam track 16 px'te görev çubuğunda kayboluyordu (ADR-030).
- **Tema uyumu:** Görev çubuğu teması `SystemUsesLightTheme` değerinden okunur. Yüksek kontrastta ikon tek renktir: mürekkep `COLOR_WINDOWTEXT`, track 1 px ve tam piksele hizalı. Tema, ölçek veya kontrast değişince ikon yeniden çizilir.
- **Durum işaretleri:**
  - Kritik: kritik olan her halkanın ucunda küçük bir boşluk (çentik). Halka doluysa çentik 12 yönündedir.
  - Veri bayat (> 30 dk): halkalar %50 opaklıkta çizilir; işaretler tam opak kalır.
  - Hata: sağ üst köşede kritik renkte bir nokta.
  - Bildirimler susturulmuş: sağ altta iki dikey çubuk (duraklat işareti).
  - Hata noktası ve duraklat işareti saydam bir halo ile halkalardan ayrılır.
  - Veri yok: yalnızca track'ler ve ortada küçük bir nokta.
  - 5 saatlik pencere yok (ADR-041): tek halka; kalınlık dış halka + boşluk + iç halka (16 px'te 6 px), aynı merkez deliği. Tooltip'te 5 saatlik satır yer almaz.
- **Tooltip:** Çok satırlı, en fazla 127 UTF-16 birimi. Sığmazsa önce başlık düşer, sonra kısa biçim kullanılır.
  ```
  TokenRadar
  5s: %72 · 1 sa 48 dk sonra yenilenir
  Haftalık: %34 · Çar 10:59'da yenilenir
  ```
  Kapsamlı haftalık limitler (`Fable · haftalık: %80`) yalnızca %75 ve üstündeyse ayrı bir satır olarak eklenir. Tahminler `≈` taşır.

### Widget (kompakt şerit, v0.3)

Native çizilir (`tiny-skia` + layered window, ADR-058). Üst çubuk haftalık, alt çubuk 5 saatlik pencere; 5 saatlik pencere yoksa tek çubuk çizilir (ADR-041). Yüzde ve dakika çözünürlüğünde geri sayım gösterilir; `≈`, bayatlık ve önem renkleri paneldekiyle aynıdır.

- **Yüzen mod:** 220×44 px (iki satır) veya 220×28 px (tek satır), köşe yarıçapı 8 px. Zemin yarı saydam tema rengidir; layered pencerede Acrylic kullanılamaz.
  - Satırlar: `7g ███░░ %34 · Çar` ve `5s ████░ %72 · 1:48`. Sıfırlama 24 saatten uzaksa gün kısaltması, değilse yukarı yuvarlanmış `s:dd` gösterilir.
  - Metin DirectWrite ve sistem fontuyla çizilir (`Segoe UI Variable Text`, yoksa `Segoe UI`), tabular rakamlar. Değer `text-1` ve medium, etiket ve sıfırlama `text-2`.
  - Renkler uygulama temasından gelir: tema "Sistem" ise Windows'un uygulama teması (`AppsUseLightTheme`), panelle aynı. Yüksek kontrastta sistem renkleri kullanılır.
  - Opaklık (%40–100) yalnızca zemini etkiler; metin ve çubuklar her zaman opaktır.
  - Bayat değer: çubuk %50 opak, değer `text-3`. Tahmin: kesikli çubuk ve `≈`. Okunamayan limit: sağ üstte hata noktası.
- **Görev çubuğu modu (Adım 18b):** Varsayılan olarak bildirim alanının 8 px solunda durur; "Sol uçta" hizalaması ve ofset (mantıksal px) ayarlanabilir. Sürükleme yalnızca görev çubuğu boyunca yataydır ve ofseti kaydeder.
  - Yükseklik görev çubuğunun yüksekliğidir; içerik dikeyde ortalanır. Ölçek görev çubuğunun DPI'ından gelir.
  - Zemin şeffaftır (alfa 1: görünmez ama fareyi alır). Hover vurgusu görev çubuğu düğmelerinin ince dolgusudur: açıkta siyah α 10, koyuda beyaz α 15, köşe yarıçapı 4, üstten ve alttan 4 px içeride. Yüksek kontrastta opak plaka kalır.
  - Renkler uygulama temasından değil görev çubuğu temasından (`SystemUsesLightTheme`) gelir.
  - Görev çubuğu ≥ 40 mantıksal px ise iki satır, değilse tek satır kullanılır: `7g %34 ███░░ · 5s %72 ████░` (çubuk 40 px, ayırıcının iki yanında 8 px). 5 saatlik pencere yoksa yalnızca haftalık gösterilir.
  - Zemin olmadığı için zaman işaretinin halesi kutu çizmez, çubuğu keser.
  - Yandaki, üstteki, kendiliğinden gizlenen veya sığmayan görev çubuğunda yüzen moda düşülür ve bu bir kez bildirilir.

## 5. Yerleşim ve şekil

- **Grid:** 4 px tabanlı; boşluk değerleri 4/8/12/16/24.
- **Köşe yarıçapı (hiyerarşiye göre):** pencerelerde ve kartlarda 8 px (Windows 11 standardı), düğme/tooltip/iskelet gibi kontrollerde 4 px, rozet ve toggle'da 999 px.
- **Panel:** 360 px genişlik, yükseklik içeriğe göre ve en fazla 600 px. Tray'e hizalanır (görev çubuğu konumu ve çoklu monitör DPI'ı hesaba katılır).
- **Ana pencere (v0.2):** Sol tarafta dar bir navigasyon (ikon + etiket) ve içerik alanı bulunur. Minimum boyut 900×600'dür.
- **Genel Bakış (Adım 15b, ADR-067):** yukarıdan aşağı okunur ve pencere genişliğini doldurur (sınır yok). Kartlar arası `--space-4`; ızgara sütunları dar pencerede taşmaz (`minmax(min(300px, 100%), 1fr)`). 640 px'ten dar içerikte "Bu hafta" tek sütuna iner; alt sıra 320 px'in altında alt alta dizilir.

```
┌ Şimdi ───────────────────────────────────── [Canlı · 2 dk önce] ┐
│  ◯ halka    Haftalık  ███████░░░░░  %58   Çar 10:59'da yenilenir │
│  (168 px)   Bu hızla limite ulaşmaz.                             │
│             Kalibrasyon: orta güven                              │
└──────────────────────────────────────────────────────────────────┘
┌ Son 7 gün ───────────────────────────────────────────────────────┐
│  zaman grafiği (220 px): pencere bantları, 75/90 eşikleri        │
│  ▬▬   ▬      ▬▬▬▬     ▬      oturum şeridi (en çok 5)            │
└──────────────────────────────────────────────────────────────────┘
┌ Bu hafta ──────────────────────────── Hafta başlangıcı Çar 11:00 ┐
│  API karşılığı maliyet   │ Token       Girdi      ██         1,2 Mn│
│  $412,30                 │             Çıktı      █       640 bin │
│  ▲ %12 geçen haftanın    │             Cache okuma ██████████ 48 Mn│
│  aynı anına göre         │ Model karışımı  opus ███████   $380   │
│  Geçen haftanın tamamı   │                 sonnet █        $32   │
└──────────────────────────────────────────────────────────────────┘
┌ En çok tüketenler ───────────────────────────────────────────────┐
│  Oturumlar                        │ Projeler                     │
│  ad · proje    ≈ %8,4  $96  ⌒╱    │ proje   ≈ %21  $240  ⌒╱      │
│  (5 satır)                        │ (5 satır)                    │
│  Paylar şimdilik maliyete oranla dağıtılıyor… (kalibrasyon notu) │
└──────────────────────────────────────────────────────────────────┘
┌ Diğer kullanım ─────────────┐ ┌ Plan değeri ─────────────────────┐
│ Tahminimiz  ≈ %6            │ │ Bu ay $620 · Ücret $200 · ×3,1   │
│ Anthropic'in kırılımı       │ │ Ayın %64'ü geçti · Geçen ay ×4,0 │
│  claude.ai %4 · Cowork %1   │ │ Bilgi amaçlı, fatura değil.      │
└─────────────────────────────┘ └──────────────────────────────────┘
```

## 6. Hareket

| Token | Değer | Kullanım |
|---|---|---|
| `--dur-fast` | 120 ms | Hover, basma |
| `--dur-base` | 200 ms | Panel açılma ve kapanma |
| `--dur-slow` | 400 ms | Değer geçişleri |
| `--dur-sweep` | 600 ms | Tek seferlik radar taraması |
| `--ease-out` | `cubic-bezier(0, 0, 0, 1)` | Windows'un yavaşlayan eğrisi; tüm geçişler |

- `prefers-reduced-motion` tercihine tam uyulur: tüm süre token'ları 0 ms olur, tarama ve busy nabzı hiç oynamaz. Windows'ta "Animasyon efektleri" kapalıysa WebView2 bunu `reduce` olarak bildirir.
- Tarama ilk görüntülemede oynamaz; yalnızca değer değiştiğinde. Busy noktası durum busy'ye geçtiğinde iki kez atar. Adım 09'da ölçüldü: değişimden 2 sn sonra `document.getAnimations()` boştur.
- Panel açılışında 8 px yukarı kayma ve opaklık geçişi uygulanır. Kapanışta efekt yoktur, panel anında kapanır.

## 7. Erişilebilirlik

- Panel ve pencerelerin tamamı klavyeyle kullanılabilir. `Esc` paneli kapatır, `Tab` sırası mantıklıdır ve odak halkaları görünür.
- **Ekran okuyucu:** Halka bileşenleri tam cümlelik bir `aria-label` taşır. Örnek: "5 saatlik pencere yüzde 72 dolu, 1 saat 48 dakika sonra yenilenecek".
- Windows yüksek kontrast modu (`forced-colors`) desteklenir.
- Metin ölçeklemesi %200'e kadar desteklenir. Panel bu durumda kaydırılabilir hale gelir.

## 8. Metin tonu ve terim sözlüğü

**Ton:** Kısa, kesin ve sakin. Panik yaratılmaz. Uyarılar eyleme dönüştürülebilir bilgi içerir.

- ✅ "Bu hızla 38 dk içinde dolacak, reset 1 sa 52 dk sonra."
- ❌ "DİKKAT! Limit dolmak üzere!!"

| EN | TR | Not |
|---|---|---|
| 5-hour window | 5 saatlik pencere | Kısa form: "5s" |
| Weekly limit | Haftalık limit | Kısa form: "7g" |
| Resets in 1h 48m | 1 sa 48 dk sonra yenilenir | |
| Resets Wed 10:59 | Çar 10:59'da yenilenir | `TR_SUFFIX` kullanılır |
| Burn rate | Tüketim hızı | Statusline'da `API $12.01/h` / `API $12.01/sa`; ilk 5 dakikada gösterilmez |
| Projected | Öngörülen | |
| Estimated | Tahmini | Her zaman "≈" ile birlikte |
| Session | Oturum | |
| Subagent | Alt ajan | |
| Project | Proje | |
| Other (claude.ai, Desktop) | Diğer (claude.ai, Desktop) | |
| Impact | Etki | "Bu oturumun etkisi: ≈ %23" |
| Calibration | Kalibrasyon | |
| Live / API / Estimated | Canlı / API / Tahmini | Kaynak rozetleri |
| API-equivalent cost | API karşılığı maliyet | İkincil metrik |
| Overview | Genel Bakış | Ana pencerenin ilk sayfası |
| This week / Top consumers | Bu hafta / En çok tüketenler | Genel Bakış bölümleri |
| against the same point last week | geçen haftanın aynı anına göre | `▲ %12` / `▼ %12`; yarım yüzdenin altı "Değişmedi" (ADR-067) |
| Model mix | Model karışımı | Fazlası "Diğer modeller" |
| Input / Output / Cache read / Cache write | Girdi / Çıktı / Cache okuma / Cache yazma | Token türleri; büyük sayılar `12,4 bin`, `3,1 Mn`, `1,2 Mr` |
| Sessions (chart lane) | Oturumlar | Grafiğin altındaki oturum şeridi |
| Plan value / Plan fee | Plan değeri / Plan ücreti | "×3,1" ücretin katı; "Ayın %64'ü geçti"; her zaman "bilgi amaçlı, fatura değil" notuyla |
| Context window | Bağlam penceresi | Statusline'da kısa form her dilde "ctx" |
| Time mark (statusline) | Zaman işareti | Bar üstünde `│`, metinde `⏱ 43%` / `⏱ %43`; geçen zaman bir ölçümdür, `≈` almaz |
| Time line (mod `/radar`, panel) | Zaman çizgisi | |
| Time strip (mod pane, panel) | Zaman şeridi | Haftalık pencere: takvim günleri (`┊` gece yarısı) ve reset'ten itibaren her 24 saat (`│`), "▲ şimdi" işareti ve reset saatiyle |
| Details (mod band) | Ayrıntı | Paneli açan düğme |
| Summary / Details (panel views) | Özet / Ayrıntı | Tray panelinin iki görünümü (ADR-048) |
| Time gone | Geçen zaman | "Geçen zaman %54 · zaman çizgisinin 10 puan önünde"; tam sayı |
| Limit reset (mod band) | Limit sıfırlandı | "✓ Haftalık limit sıfırlandı", bir kez |
| (statusline cost) | `API $4,12` | `API` işareti, değerin fatura olmadığını söyler; çıkarılmaz |
| Item (statusline) | Öğe | Satırın bir parçası: limit, oturum, proje ya da düzen öğesi (ADR-049) |
| Format / Label / Color | Biçim / Etiket / Renk | Öğe ayarları; limitlerde renk yalnızca normal durumun rengidir |
| Line break / Separator / Custom text / Space / Flexible space | Satır sonu / Ayraç / Özel metin / Boşluk / Esnek boşluk | Düzen öğeleri |
| Line (editor card) | Satır | Düzenleyicide her çıktı satırı bir karttır: "Satır 2" |
| Right margin | Sağ pay | Claude Code'un 4 sütununa ek olarak sağda boş bırakılan sütunlar |
| Theme: Plain / Powerline / Rounded | Tema: Düz / Powerline / Yuvarlak | |
| Preset | Şablon | Varsayılan, Sade, Ayrıntılı, İki satır, Powerline |
| Export / Import | Dışa aktar / İçe aktar | `[statusline]` TOML metni; içe aktarma kaydetmeden önizler. Statusline araç çubuğunda simge düğmesi; ipucu: "Dışa aktar: satırı [statusline] tablosu olarak kopyala" |
| Daily budget | Günlük bütçe | Haftalık payın kalan günlere bölümü: `20%/day` / `%20/gün`; pencerenin kaynağını devralır, tahminse `≈` |
| Cache ratio | Cache oranı | Son istekte cache'ten okunan girdi: `cache 82%` / `cache %82`; "cache" çevrilmez |
| Session name / Output style | Oturum adı / Output style | "Output style" Claude Code'un terimi, çevrilmez |
| Left / Tokens left | Kalan / Kalan token | Biçim adları; satırda `58% left` / `%58 kaldı`, `124k left` / `124k kaldı` |
| Reset as: Countdown / Clock time | Sıfırlanma: Geri sayım / Saat | Saat biçimi yerel saattir: `Tue 04:20` / `Sal 04:20` |
| Bar width / Bar style: Blocks / Thin / Dots / Capsule / Solid | Çubuk genişliği / Çubuk stili: Blok / İnce / Nokta / Kapsül / Dolgu | Kapsül Nerd Fonts ≥ 3.4 ister ve Powerline bloğunda çizilen tek stildir; Dolgu arka plan hücreleriyle çizilir, font istemez (ADR-056) |
| Icons: Text / Unicode / Nerd Font | Simgeler: Metin / Unicode / Nerd Font | Etiketin yerine geçen simge seti; kullanıcının etiketi her zaman önde. `≈`, `API` ve "bu oturum" hiçbir sette değişmez (ADR-056) |
| Compactions / Total / Auto and manual | Compact sayısı / Toplam / Oto ve elle | Satırda `⟲ 2 auto 1 manual · 300k` / `⟲ 2 oto 1 elle · 300k`; "compact" çevrilmez. Bilinmeyen tetikleyici `other` / `diğer` |
| Auto off | Oto kapalı | Auto-compact kapalıysa pencere yerine: `⟲ 3 · auto off` / `⟲ 3 · oto kapalı` |
| Effort / low, medium, high, xhigh, max | Efor / düşük, orta, yüksek, çok yüksek, maks | Oturumun `/effort` düzeyi (Adım 12q). "Hızlı ve düşünme ile" biçimi: `max fast no thinking` / `maks hızlı düşünme yok` |
| Cache timer / cold | Cache süresi / soğuk | Prompt cache'in soğuyacağı yerel saat: `cache →14:32 1h`; soğuduysa `cache cold` / `cache soğuk` |
| Cache misses / miss | Cache kaçırmaları / kaçırma | `2 misses · tools changed` / `2 kaçırma · araçlar değişti`. Nedenler: tools changed / araçlar değişti, system prompt changed / sistem istemi değişti, 5m cache expired / 5 dk süre doldu, likely server side / muhtemelen sunucu |
| Recache cost / if cold | Yeniden cache maliyeti / soğursa | `if cold 45k` / `soğursa 45k`; tahmin değil, Claude Code'un sayısı (`≈` yok) |
| Spend limit | Harcama limiti | Satır etiketi `$` her dilde aynı; "Dolar" biçimi `$21/$50` |
| Pull request / approved, changes requested, pending, draft | Pull request / onaylandı, değişiklik istendi, bekliyor, taslak | Satırda `#123 ✓`, `#45 ⨯`, `#45 ∙`; GitLab'da `!45`. "Pull request" ve "worktree" çevrilmez, etiket `wt` |
| Repository | Depo | `owner/name`; "Sunucuyla" biçimi `github.com/owner/name` |
| Live refresh | Canlı yenileme | Statusline sayfasındaki seçici: Kapalı / 5 sn / 10 sn / 30 sn; açıklama bedeli söyler: "her N saniyede bir TokenRadar çalışır" (ADR-059) |
| Git status / dirty mark / ahead and behind | Git durumu / değişiklik işareti / ileride ve geride | `+2 ~3 ?1 !1 ⇡1 ⇣2`, temizse `✓`; işaret `*` (Nerd: oct-dot_fill); taranmayan izlenmeyen dosyalar `??`. "staged" → hazırlanmış, "untracked" → izlenmeyen |
| Countdown (cache timer) | Geri sayım | `cache 4m`, son iki dakikada saniyeyle `cache 1m12s` / `cache 1dk12sn`; canlı yenileme kapalıyken boştayken durur, editör bunu söyler |
| Terminal font | Terminal fontu | Ayarlardaki yardımcı ve diyalog başlığı (ADR-055) |
| Nerd Font | Nerd Font | Çevrilmez. Kademeler: "Nerd Font değil" / "Nerd Font, 3.4 öncesi" / "Nerd Font" |
| Select font / Undo | Fontu seç / Geri al | Terminal satırındaki eylemler; ikisi de diff ve onayla |
| Undo everything / Restore previous state… | Hepsini geri al / Eski haline getir… | Tek tıkla geri dönüş: ayarlar ve kurulan font, tek bir özet onayıyla |
| (statusline labels) | `in` / `out` / `time` → `girdi` / `çıktı` / `süre` | Token ve süre öğelerinin varsayılan etiketleri; `ctx`, `api`, `git` her dilde aynı |

**Türkçe yazım kuralları:**
- Yüzde işareti sayıdan önce yazılır: `%72`.
- Ondalık ayırıcı virgüldür: `%23,5`. Binlik ayırıcı noktadır: `12.400`.
- Kısaltmalar: `sa`, `dk`, `sn`.
- Günler: `Pzt Sal Çar Per Cum Cmt Paz`.
- Ekler her zaman `TR_SUFFIX` fonksiyonuyla üretilir: `5'te`, `6'da`, `%72'si`. Asla elle yazılmaz.

## 9. Boş durumlar ve hata durumları

| Durum | Gösterim |
|---|---|
| İlk açılış, indeksleme sürüyor | İlerleme halkası + "Son 30 günün oturumları işleniyor… %42" + "Mevcut pencere hazır" |
| Claude Code bulunamadı | Yol seçme butonu + `CLAUDE_CONFIG_DIR` açıklaması |
| Limit verisi yok (statusline kurulmamış, API onayı yok) | İki seçenek: "Statusline köprüsünü kur (önerilen)" / "API erişimine izin ver" + loglardan tahmin modu (kesikli halka) |
| Token bayat (401) | "Claude Code'u bir kez çalıştırınca güncellenecek" (sakin, hata rengi kullanılmaz) |
| Çevrimdışı | Son değer gösterilir, yanında "X dk önce" ve bulut-çarpı ikonu |
| Format değişikliği şüphesi | "Claude Code güncellenmiş olabilir; bazı veriler okunamıyor" + tanılama butonu |
| Fiyatı bilinmeyen model (ADR-057) | "claude-… için henüz fiyat yok. TokenRadar güncellenene kadar maliyet tahminidir." (sakin, bilgi ikonu) |
