# Görsel Dil — "Native kabuk + enstrüman"

> Bu doküman yön belirler. Kesin token değerleri Adım 09'da, tasarım skill'leri (`frontend-design`, `impeccable`)
> kullanılarak ve gerçek ekranda Mica üzerinde test edilerek sabitlenir. Aşağıdaki değerler başlangıç önerisidir.

## 1. Fikir

- **Kabuk sessizdir.** Pencere zemini işletim sistemine aittir: Windows 11'de Mica (panelde Acrylic), macOS'ta vibrancy. Kontroller, yazı tipi ve köşe yarıçapları platformla uyumludur. Kullanıcı uygulamayı "Windows'un bir parçası" gibi hisseder.
- **Veri konuşur.** Limitler "enstrüman" diliyle gösterilir: iç içe halkalar, ince tick'ler, mono ve tabular rakamlar, fosfor tonlu vurgu rengi. Radar metaforu dekorasyon için kullanılmaz; bilgi taşıdığı yerde kullanılır. Tarama efekti yalnızca veri güncellendiği anda bir kez görünür.
- **Tek görsel gramer, her yüzeyde aynı:** **dış halka/üst çubuk = haftalık**, **iç halka/alt çubuk = 5 saatlik pencere**. Tray ikonu, panel, widget ve ana pencere bu eşlemeyi asla bozmaz.

## 2. Renk

OKLCH ile tanımlanır. Değerler başlangıç önerisidir.

| Token | Koyu tema | Açık tema | Kullanım |
|---|---|---|---|
| `--surface-0` | Mica (şeffaf) / yedek `oklch(0.20 0.01 250)` | Mica / yedek `oklch(0.97 0.005 250)` | Pencere zemini |
| `--surface-1` | `oklch(0.26 0.01 250 / 0.72)` | `oklch(1 0 0 / 0.70)` | Kartlar |
| `--text-1` | `oklch(0.96 0.005 250)` | `oklch(0.22 0.01 250)` | Birincil metin |
| `--text-2` | `oklch(0.75 0.01 250)` | `oklch(0.45 0.01 250)` | İkincil metin |
| `--track` | `oklch(0.40 0.01 250 / 0.5)` | `oklch(0.85 0.01 250)` | Halka ve çubuk zemini |
| `--signal-ok` | `oklch(0.82 0.13 190)` (fosfor camgöbeği) | `oklch(0.58 0.12 195)` | Normal durum, veri mürekkebi |
| `--signal-warn` | `oklch(0.84 0.15 80)` (amber) | `oklch(0.66 0.15 70)` | Uyarı |
| `--signal-crit` | `oklch(0.70 0.19 25)` (mercan kırmızı) | `oklch(0.56 0.20 25)` | Kritik |
| `--signal-est` | `--text-2` + kesikli çizgi | aynı | Tahmini değerler |
| `--signal-other` | `oklch(0.70 0.06 300)` (soluk mor) | `oklch(0.55 0.08 300)` | "Diğer" kovası (claude.ai/Desktop) |

**Durum eşikleri** (varsayılan, ayarlanabilir): `< 75` ok, `75–90` warn, `≥ 90` crit.

**Renk körlüğü:** Durum asla yalnızca renkle ifade edilmez.
- Kritik durumda halkanın ucunda bir çentik ve metinde ikon bulunur.
- Uyarı durumunda üçgen glif kullanılır.
- Tahmini değerler kesikli çizgi ve "≈" işaretiyle gösterilir.

**Kontrast:** Metinde WCAG AA (4.5:1) sağlanır. Veri çizgileri zeminle en az 3:1 kontrast oluşturur.

## 3. Tipografi

- **UI metni:** `"Segoe UI Variable Text", "Segoe UI", system-ui` (Windows); macOS'ta `-apple-system`. Ek font yüklenmez.
- **Okuma değerleri (enstrüman rakamları):** Gömülü ve **alt kümesi alınmış** bir mono variable font (adaylar: Geist Mono, JetBrains Mono). Yalnızca rakamlar, `% , . : - + ≈ ▲ ▼` ve harfler `s d h m g sa dk`. Hedef boyut ≤ 25 KB woff2. Kesin seçim Adım 09'da yapılır.
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
- **Merkez:** Büyük punto ile 5 saatlik %, altında küçük punto ile reset geri sayımı.
- **Animasyon:** Değer değiştiğinde 400 ms ease-out ile geçiş yapılır ve bir kez radar taraması (conic gradient, 600 ms) oynar. **Sürekli animasyon yoktur.** Bu kural boşta CPU bütçesi gereği zorunludur.

### Limit satırı

`[ikon] Etiket ........ %72  ·  1 sa 48 dk` ve altında ince bir çubuk. Scoped limitler (örneğin "Fable · haftalık") girintili bir alt satırda gösterilir.

### Oturum satırı

`● ad (proje)   busy   ≈ %9 bu pencere   12,4k tok/dk`
- Durum noktası: busy → nabız atan nokta (yalnızca 2 kez atar, sürekli değil); idle → içi boş daire.

### Kaynak rozeti

Her limit kartının köşesinde küçük bir rozet bulunur: `canlı` (S1), `API` (S2) veya `tahmini`. Rozetin tooltip'i son güncelleme zamanını gösterir.

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
- **Köşe yarıçapı:** pencerelerde 8 px (Windows 11 standardı), kartlarda 8 px, rozetlerde 999 px.
- **Panel:** 360 px genişlik, yükseklik içeriğe göre ve en fazla 600 px. Tray'e hizalanır (görev çubuğu konumu ve çoklu monitör DPI'ı hesaba katılır).
- **Ana pencere (v0.2):** Sol tarafta dar bir navigasyon (ikon + etiket) ve içerik alanı bulunur. Minimum boyut 900×600'dür.

## 6. Hareket

| Token | Değer | Kullanım |
|---|---|---|
| `--dur-fast` | 120 ms | Hover, basma |
| `--dur-base` | 200 ms | Panel açılma ve kapanma |
| `--dur-slow` | 400 ms | Değer geçişleri |

- `prefers-reduced-motion` tercihine tam uyulur: geçişler anlık olur, tarama efekti oynamaz.
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
