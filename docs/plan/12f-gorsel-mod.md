# Adım 12f — Görsel mod: panel, görsel bant ve diğer görsel yüzeyler

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1.x | 12e | 1 oturum | ⬜ |

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
_(Oturum sonunda doldurulur)_
