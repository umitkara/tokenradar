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
- **Animasyon:** Değer değiştiğinde 400 ms ease-out ile geçiş yapılır ve bir kez radar taraması (conic gradient, 600 ms) oynar. **Sürekli animasyon yoktur.** Bu kural boşta CPU bütçesi gereği zorunludur.

### Limit satırı

`[ikon] Etiket ........ %72  ·  1 sa 48 dk` ve altında ince bir çubuk. Scoped limitler (örneğin "Fable · haftalık") girintili bir alt satırda gösterilir.
- **Panelde (`stacked`):** Halkanın yanındaki dar sütunda geri sayım çubuğun altına iner: `Etiket ..... %72`, çubuk, `1 sa 48 dk sonra yenilenir`. Haftalık ve scoped limitler mutlak zamanı gösterir (`Cum 17:35'te yenilenir`). (ADR-032)

### Oturum satırı

İki satır (ADR-032; tek satır 360 px'te çevirilerle sığmıyordu):
```
● ad  proje                 ≈ %9 bu pencere
  Meşgul · 12,4k tok/dk
```
- Ad sırası: oturum adı → başlık → proje → "Adsız oturum". Proje ad olarak kullanıldıysa tekrar yazılmaz.
- Durum noktası: busy → nabız atan nokta (yalnızca 2 kez atar, sürekli değil); idle → içi boş daire. Canlı olmayan satırlar soluktur.

### Kaynak rozeti

Küçük bir rozet: `canlı` (S1), `API` (S2) veya `tahmini`. Rozetin tooltip'i son güncelleme zamanını gösterir. Panelde tek rozet alt bilgidedir ve birincil limitin (5 saatlik, yoksa haftalık) kaynağını gösterir; tahmini değerler satırlarda ayrıca `≈` ile işaretlenir.

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
- **Tooltip:** Çok satırlı, en fazla 127 UTF-16 birimi. Sığmazsa önce başlık düşer, sonra kısa biçim kullanılır.
  ```
  TokenRadar
  5s: %72 · 1 sa 48 dk sonra yenilenir
  Haftalık: %34 · Çar 10:59'da yenilenir
  ```
  Kapsamlı haftalık limitler (`Fable · haftalık: %80`) yalnızca %75 ve üstündeyse ayrı bir satır olarak eklenir. Tahminler `≈` taşır.

### Widget (kompakt şerit, v0.3)

İki çubuk, yüzde ve geri sayımdan oluşur. Boyutu 220×44 px'tir, köşe yarıçapı 8 px. Arka planı Acrylic'tir; Windows 10'da opak bir yedek kullanılır.

## 5. Yerleşim ve şekil

- **Grid:** 4 px tabanlı; boşluk değerleri 4/8/12/16/24.
- **Köşe yarıçapı (hiyerarşiye göre):** pencerelerde ve kartlarda 8 px (Windows 11 standardı), düğme/tooltip/iskelet gibi kontrollerde 4 px, rozet ve toggle'da 999 px.
- **Panel:** 360 px genişlik, yükseklik içeriğe göre ve en fazla 600 px. Tray'e hizalanır (görev çubuğu konumu ve çoklu monitör DPI'ı hesaba katılır).
- **Ana pencere (v0.2):** Sol tarafta dar bir navigasyon (ikon + etiket) ve içerik alanı bulunur. Minimum boyut 900×600'dür.

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
| Burn rate | Tüketim hızı | |
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
