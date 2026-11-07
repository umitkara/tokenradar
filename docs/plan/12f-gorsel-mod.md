# Adım 12f — Görsel mod: panel, görsel bant ve diğer görsel yüzeyler

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1.x | 12e | 1 oturum | ✅ |

## Amaç

Adım 12d'nin modu yalnızca metin kullanıyor: bant renkli satırlardan oluşuyor, `/radar` düz metin döndürüyor (bu metin modelin bağlamına da giriyor). Mod API'sinin görsel yüzeyleri kullanılacak. Kullanıcı kararı: **yalnızca zaman çizelgesiyle sınırlı kalınmaz**, mod'un görsel olanakları baştan değerlendirilir.

## Önce oku

- ADR-019 (sürekli animasyon yok), ADR-041, ADR-042, ADR-043, ADR-044
- Adım 12d ve 12e devir notları
- `plugin-authoring` skill'i: `Pane`, `AbovePrompt`, `Box`/`Text` (kenarlık, arka plan, tema renkleri), `Raster` (terminal), `Svg` (masaüstü, VS Code, mobil), `$.ui.open`, `$.ui.status`
- `docs/design-language.md` (renk token'ları ve terminal karşılıkları)

## Kullanıcıyla kararlaştırılanlar (2026-11-07)

- **`/radar` bir panel açar.** Panelde limit kartları bulunur: renkli bar, zaman işareti, kaynak ve projeksiyon. Altında haftalık zaman şeridi yer alır: kullanım dolgusu, "şimdi" işareti, gün sınırları ve gün sonu değerleri. Masaüstü ve VS Code yüzeyinde ayrıca `Svg` grafik gösterilir: kullanım eğrisi ve zaman çizgisi.
- **Bant tek satıra iner:** kenarlıklı, uyarı renginde, mini bar ve zaman işaretiyle. "Ayrıntı" düğmesi paneli açar, "Gizle" düğmesi bandı gizler.
- **Yalnızca zaman çizelgesiyle sınırlı kalınmaz.** Adımın başında diğer görsel yüzeyler kullanıcıyla değerlendirilir. Adaylar:
  - **Canlı oturumlar:** Panelde her oturumun payı ve maliyeti için küçük barlar.
  - **Bağlam penceresi:** Panelde context doluluğu barı.
  - **`$.ui.status`:** Statusline köprüsü kurulu değilse modun kendi kısa durum girdisi. Statusline ile çift gösterim olmamalı.
  - **Kaynak ve tazelik rozetleri:** Statusline / API / tahmin ayrımı renkli rozetlerle gösterilir; eski değerler soluk görünür.
  - **Reset anı:** Pencere sıfırlanınca bir kez görünen bir bilgi satırı.
  - **Mobil ve masaüstü yüzeyleri:** Aynı ağaç her yüzeyde çalışmalı ve testlerde dört yüzeyle denenmeli.

## Kapsam (taslak, adım başında netleşir)

- Panel (`Pane`) ve görsel bant.
- `Svg` grafik için geçmiş kullanım verisi gerekir. CLI'a salt-okunur bir komut eklenir (ör. `tokenradar history --json`, veritabanındaki `limit_snapshots` kayıtlarından). IPC'ye dokunulmaz. Bu karar adım başında ADR olarak yazılır.
- `/radar` metni kısa kalır ("panel açıldı" ve özet), böylece modelin bağlamına daha az şey girer.

## Kapsam dışı

- Marketplace yayını.
- Sürekli animasyon (ADR-019).
- Konuşma içeriğine erişen her şey. `ToolUse` satırına çizim yapılmaz, çünkü props alanı tool girdisini taşır.

## Devir notu

**Tarih:** 2026-11-07 · **Durum:** ✅ Tamamlandı

**Kullanıcı kararları (adım başında):** Canlı oturum barları, bağlam barı, kaynak/tazelik rozetleri ve reset satırı eklendi. `$.ui.status` yalnızca köprü kurulu değilken gösterilir. Geçmiş veri `tokenradar history --json`'dan gelir. Terminalde `Raster`, diğer yüzeylerde `Svg` kullanılır. Hepsi ADR-046'da.

**Yapılanlar:**
- **Core ve CLI:**
  - `limits::history::window_series` eklendi. Mevcut pencerenin aynı reset'e ait okumalarını alır ve 168 dilimin her birinden sonuncusunu tutar. `absent`, reset'i geçmiş ve penceresiz limitler dışarıda kalır.
  - `ReadOnlyStore::snapshots_between` eklendi.
  - `tokenradar history [--json]` eklendi: salt-okunur, IPC'ye dokunmaz. Yardım satırı EN ve TR.
- **Mod (`integrations/claude-code`):**
  - `hooks/draw.ts` (yeni): bar, haftalık şerit, `Svg` grafik, `Raster` hücreleri, base64.
  - `hooks/view.tsx` (yeni): panel, kart, rozet, canlı oturumlar, bant ve reset ağaçları.
  - `hooks/register.tsx` yeniden yazıldı: atom'lar, `Pane` ve `AbovePrompt` render'ı, `/radar`.
  - `hooks/model.ts` genişletildi: `parseHistory`, `bridgeInstalled`, `stepResets`, `resetNotice`, `statusText`, `summary`, canlı oturumlar. Oturum `title` alanı okunmaz.
- **Ekran görüntüleriyle yapılan düzeltmeler:**
  - Panel kırpılıyordu, `rows` verildi.
  - İstenmeden konan genişlik sınırları kaldırıldı; panel artık tam genişlik.
  - "API" rozeti "kullanım API'si" oldu.
  - `⏱` kartlardan çıkarıldı; çift genişlikte çiziliyordu.
- **Zaman şeridi:**
  - İlk sürüm reset'ten başlayan 24 saatlik parçaları yalnızca gün adıyla etiketliyordu. Çar 11:00'de sıfırlanan pencere gece yarısından başlıyormuş gibi okunuyordu.
  - Kullanıcının "Kota Zaman Çizelgesi" HTML'ine göre iki görünüm birleştirildi:
    - Takvim günleri: gece yarısında `┊`, ilk ve son gün kısmi.
    - Reset'ten itibaren her 24 saatte `│` ve altında zaman çizgisinin değeri (%14, %29 …), "▲ şimdi" ile aynı satırda.
  - Grafiğin altında açıklama satırı var: sıfırlanma günü ve saati, işaretlerin anlamı, grafik anahtarı.
- **Terminal grafiği:**
  - Önce noktalar, sonra braille denendi. Bu fontta ikisi de kesik bir merdiven gibi göründü.
  - Son hâli barlar gibi: şimdiye kadar geçen zaman gri sekizlik bloklarla, kullanım önünde renkli.
  - Grafik ve şerit aynı genişliği ve aynı yuvarlamayı kullanır, gün çizgileri aynı sütunlara düşer. Bunu bir test denetliyor.
- **Canlı oturumlar:** Bar yalnızca karşılaştıracak bir şey varken çizilir: ≈pay biliniyorsa ya da en az iki oturumun maliyeti varsa.
- **Belgeler:** ADR-046 yazıldı. Architecture §11 (mod ve `history`), design-language sözlüğü, README (mod bölümü, CLI) ve CHANGELOG güncellendi.

**Doğrulama:**
- `cargo fmt --check`, clippy (`-D warnings`) ve `cargo test --workspace`: temiz.
- `pnpm -C app check`, `pnpm -C app lint` ve `pnpm -C app test` (87): temiz.
- `claude plugin validate`: geçti. `tsc -p integrations/claude-code`: temiz. `claude plugin test`: 38/38.
- Terminal paneli kullanıcının ekran görüntüleriyle elle denendi; kullanıcı onayladı.

**Ölçülmeyenler ve riskler:**
- `Svg` grafiğin masaüstü, VS Code ve mobildeki görünümü ile renk şemasını izleyip izlemediği elle denenmedi; yalnızca testlerde dört yüzeyle doğrulandı.
- `Raster` tema bilgisi almaz. Gri ton (`#5a5f64`) ve sinyal renkleri orta açıklıkta seçildi. Açık temalı terminalde denenmedi.
- Gün sınırları sabit saat dilimi ofsetiyle hesaplanır; pencere içinde yaz saati değişirse sonraki sınırlar bir saat kayar (ADR-046).
- Yan tarafa yerleşen panel tam yükseklikte açılır, içerik kısa kalınca altı boş görünür. API yüksekliği yalnızca açılışta alıyor.
- Kart barlarındaki zaman işareti ile şeritteki "▲ şimdi" aynı sütunda değil, çünkü kart barı daha dar. Kullanıcıya soruldu, böyle kaldı.
- **Geçici kurulum:** Elle denemede mod'un CLI yolu (`/config` → TokenRadar CLI) geçici bir kopyaya, `%LOCALAPPDATA%\Temp\tokenradar-12f\tokenradar.exe`'ye yöneltildi. Bu yol yayınlanmış ya da kurulu CLI'a geri alınmalı. Kopya ondan sonra silinebilir.

**Kapsam dışı kalanlar:**
- **Fable (`weekly_scoped`) kartı:** Usage API'den gelir ve Fable'ın haftalık limitin en fazla %50'sini kullanabileceğini gösterir (destek makalesi: "Claude Fable models on your plan"). Fable kullanımı normal haftalık limitten de düşer. Kullanıcı kartı olduğu gibi bıraktı. İstenirse ileride haftalık kartın alt satırı yapılabilir; `docs/data-sources.md`'deki `weekly_scoped` notuna bu %50 tavan bilgisi eklenebilir.
- Marketplace yayını, sürekli animasyon ve `ToolUse` çizimi bu adımda yapılmadı (planda kapsam dışı).

**Sonraki adım:** 13 (kalibrasyon motoru). En az bir haftalık veri gerektirir.
