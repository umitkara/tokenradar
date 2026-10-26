# Adım 15 — Ana pencere kabuğu ve Genel Bakış

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.2 | 14 | 1-2 oturum | ⬜ |

## Amaç

Analitik deneyiminin kapısı olan ana pencereyi yapmak. Bu adım iki parçadan oluşur:

1. **Kabuk:** navigasyon, yönlendirme (routing), pencere yaşam döngüsü, tutarlı yerleşim.
2. **Genel Bakış ekranı:** güncel limitler, 7 günlük % zaman serisi (oturum işaretleriyle), bu haftanın özeti, en yüksek etkili oturum ve projeler, "Diğer" payı, plan değeri kartı ve kalibrasyon güveni.

## Önce oku

- `docs/design-language.md` (tamamı; özellikle §4, §5, §8)
- `docs/architecture.md` §2 (yaşam döngüsü), §5 ("UI'da hesaplama yok" ilkesi)
- `docs/performance-budget.md` (ana pencere satırları, bundle bütçesi)
- Adım 14'teki `get_overview` ve `get_timeseries` görünüm modelleri
- `dataviz` skill'i (grafik renkleri, işaretler, eksenler)

## Kapsam

- `main.html` girişi:
  - Pencere yapılandırması: Mica, çerçeve dekorasyonu (özel başlık çubuğu veya native; karar ver ve gerekçelendir), minimum boyut 900×600, boyut ve konum hatırlama.
  - `warm_ttl` 30 s.
- **Kabuk:**
  - Sol navigasyon: Genel Bakış · Pencereler · Oturumlar · Projeler · Örüntüler · (alt kısımda) Ayarlar.
  - Hash tabanlı, kütüphanesiz minimal router. Rotalar tembel yüklenir (dinamik `import()`).
- **Grafik altyapısı:**
  - uPlot sarmalayıcı (tema token'larından renk alır, EN/TR eksen biçimlendirmesi yapar, tooltip içerir).
  - Ortak `ChartTooltip` bileşeni.
- **Genel Bakış ekranı** (aşağıdaki görevler).
- **Giriş noktaları:**
  - Panelde "Detaylar" butonu (artık görünür).
  - Tray menüsünde "TokenRadar'ı aç".
  - CLI komutu `tokenradar open`.

## Kapsam dışı

- Oturumlar ve Projeler (Adım 16), Pencereler ve Örüntüler (Adım 17). Bu adımda bu rotalar boş durum ekranlarıyla yer alır.

## Görevler

- [ ] Tasarım: Genel Bakış yerleşimi `frontend-design` veya `impeccable` skill'i ile tasarlanır; ardından `design-language.md` §5'e bir yerleşim taslağı eklenir. **Kontrol listesi:** Bilgi hiyerarşisi şu soruları sırayla yanıtlamalı: (1) Şu an neredeyim? (2) Bu hafta nasıl gidiyor? (3) Ne tüketiyor? (4) Ne kadar değer alıyorum?
- [ ] **Kabuk:**
  - Navigasyon etiketleri i18n'den gelir.
  - Klavye kısayolları: `Ctrl+1..5` ile rotalar arasında geçiş, `Ctrl+,` ile ayarlar.
  - Pencere başlığı rotayla güncellenir.
  - Odak yönetimi: rota değişince ana başlığa odaklanılır (ekran okuyucu için).
- [ ] **uPlot sarmalayıcı:**
  - Tema değişikliğinde yeniden renklendirir.
  - `ResizeObserver` ile boyutlanır.
  - Yalnızca görünürken çizer.
  - Seri renkleri: 5s için `--signal-ok`, haftalık için ikincil ton; eşik çizgileri (75/90) soluk kesikli; pencere sınırları dikey bantlar olarak çizilir.
- [ ] **Genel Bakış bölümleri:**
  1. **Şimdi:** Büyük ikili halka + limit satırları (paneldeki bileşenlerin büyük varyantı) + projeksiyon. Kaynak ve kalibrasyon güveni rozetleri.
  2. **7 gün:** 5s ve haftalık % serisi. Oturum işaretleri (başlangıç ve bitiş noktaları; hover ile oturum adı ve payı). Limite takılan pencereler kırmızı bantla gösterilir. Tahmini bölümler kesikli çizilir.
  3. **Bu hafta:**
     - API karşılığı maliyet.
     - Token kırılımı (input / output / cache read / cache write) yığılmış çubuk olarak.
     - Model karışımı yatay çubuk olarak; donut kullanılmaz.
     - Geçen haftayla karşılaştırma (▲▼ %).
  4. **En çok tüketenler:** Haftalık limite etkisi en yüksek 5 oturum ve 5 proje: `≈ %` değeri, maliyet, mini sparkline. Tıklanınca ilgili detay sayfası açılır (Adım 16'da etkinleşir).
  5. **Diğer:** claude.ai, Desktop ve Cowork payı. Kendi tahminimiz ve `seven_day_breakdown` yan yana gösterilir; şüpheli fark varsa bilgi notu eklenir.
  6. **Plan değeri:** "Bu ay API karşılığı $X · Plan ücreti $Y · ×Z". Ayarlarda plan ücreti düzenlenebilir. "Bu sayılar bilgi amaçlıdır" notu bulunur.
- [ ] **Boş durumlar:**
  - Kalibrasyon verisi yetersiz: "Atıflar ≈ X gün sonra kesinleşecek" + mevcut veri.
  - Yalnızca tahmini veri var.
  - Hiç snapshot yok.
- [ ] **Performans:**
  - Ana pencere soğuk açılış ≤ 600 ms, bellek ≤ 180 MB.
  - Bundle ≤ 180 KB JS (gzip; uPlot dahil).
  - Rota geçişi ≤ 100 ms.
  - Ölç ve raporla.
- [ ] **Testler:** Router (Vitest), grafik veri dönüşümleri, biçimlendirme. Galeri sayfasına grafik bileşenleri eklenir.

## Teknik notlar ve tuzaklar

- **uPlot ve CSS değişkenleri:** uPlot canvas üzerine çizdiği için CSS değişkenlerini doğrudan okuyamaz. Tema değişince `getComputedStyle` ile renkler okunur ve grafik yeniden oluşturulur.
- **Büyük seriler:** Seyreltme backend'de (LTTB) yapılır. Frontend asla ham veri işlemez.
- **Pencere boyutu ve konumu:** `tauri-plugin-window-state` veya kendi `kv` kaydı kullanılabilir. Monitör çıkarılırsa pencerenin ekran dışında kalmaması için sınır kontrolü yapılmalı.
- **Ana pencere ve panel aynı anda açık olabilir.** Olay abonelikleri pencere başına ayrı tutulmalı ve gereksiz yere çoğaltılmamalı.

## Kabul kriterleri

- [ ] Genel Bakış, gerçek veriyle 4 soruyu da tek ekranda yanıtlar (devir notunda ekran görüntüsü).
- [ ] Tüm sayılar kaynak ve tahmin durumunu doğru gösterir; "≈" işareti ve kesikli çizgi tutarlı kullanılır.
- [ ] Performans ve bundle bütçeleri sağlanır.
- [ ] EN ve TR biçimlendirmeleri doğrudur (eksen etiketleri, tarih ve saat, para birimi).

## Doğrulama

```powershell
pnpm -C app test; pnpm -C app check
pnpm -C app build; node scripts/check-bundle-size.mjs
pnpm dev
.\scripts\measure-memory.ps1 -ProcessName tokenradar-app -Scenario main-window
```

## Devir notu

_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
