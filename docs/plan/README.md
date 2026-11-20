# Uygulama Planı

> Her adım, **tek bir Claude Code oturumunda** uygulanmak üzere tasarlanmıştır. Her adım bağımsız okunabilir; gerekli
> bağlamı "Önce oku" bölümündeki dokümanlardan alır. Sıra önemlidir: bir adıma başlamadan önce bağımlılıklarının
> durumu ✅ olmalıdır.

## Oturum protokolü (her adımda uygulanır)

1. **Bağlamı yükle:** `CLAUDE.md` → bu dosya (durum tablosu) → ilgili adım dosyası → adımın "Önce oku" listesi. Bir önceki adımın **Devir notu** bölümünü mutlaka oku.
2. **Önkoşulları doğrula:** Bağımlı adımlar ✅ mi? Devir notlarında bu adımı etkileyen bir sapma var mı?
3. **Dal aç:** `git switch -c step/NN-kisa-ad`.
4. **Uygula:** Görev listesini sırayla işle. Plan ile gerçek arasında fark çıkarsa (API değişmiş, kütüphane davranışı farklı vb.) **dur, kullanıcıya bildir**. Kararı `docs/decisions.md` dosyasına yeni bir ADR olarak ekle, sonra devam et.
5. **Doğrula:** "Doğrulama" bölümündeki komutların tamamı geçmeli. Performans bütçesi olan adımlarda ölçüm `docs/perf/` altına kaydedilir.
6. **Kapat:**
   - Adım dosyasının sonundaki **Devir notu** bölümünü doldur: ne yapıldı, plandan sapmalar, açık konular, sonraki adıma notlar.
   - Bu dosyadaki durum tablosunu güncelle.
   - Conventional commit'lerle commit at, `main`'e fast-forward merge yap.
7. **Kapsam disiplini:** Adımın "Kapsam dışı" listesindeki işler yapılmaz. Yapılması gerektiği düşünülürse devir notuna yazılır.

## Yol haritası ve durum

**Durum işaretleri:** ⬜ başlanmadı · 🟨 sürüyor · ✅ tamamlandı · ⛔ engellendi

### v0.1 — Radar Çekirdeği (MVP)

| # | Adım | Bağımlılık | Durum |
|---|---|---|---|
| 01 | [Repo iskeleti ve araç zinciri](01-repo-iskeleti.md) | — | ✅ |
| 02 | [Performans spike'ı ve mimari kapı](02-performans-spike.md) | 01 | ✅ |
| 03 | [Çekirdek domain modeli ve SQLite deposu](03-domain-ve-depo.md) | 01 | ✅ |
| 04 | [Claude JSONL indeksleyici](04-jsonl-indeksleyici.md) | 02, 03 | ✅ |
| 05 | [Dosya izleme, canlı oturumlar, olay veri yolu](05-izleme-ve-canli-oturumlar.md) | 04 | ✅ |
| 06 | [Limit motoru ve OAuth usage API istemcisi](06-limit-motoru-ve-api.md) | 03, 05 | ✅ |
| 07 | [IPC, CLI ve statusline köprüsü](07-ipc-cli-statusline.md) | 06 | ✅ |
| 08 | [Tray: dinamik ikili halka ve menü](08-tray.md) | 06 | ✅ |
| 09 | [Frontend temeli: token'lar, tema, i18n, tipli köprü](09-frontend-temeli.md) | 02, 03 | ✅ |
| 10 | [Tray paneli](10-tray-paneli.md) | 07, 08, 09 | ✅ |
| 11 | [Bildirimler, ayarlar ve onboarding](11-bildirim-ayar-onboarding.md) | 10 | ✅ (11a, 11b, 11c). Açık işler Adım 12'de kurulu build ile kapandı |
| 12 | [Paketleme, güncelleme ve v0.1 sürümü](12-paketleme-ve-v0.1.md) | 11 | ✅ v0.1.0 yayınlandı (2026-10-30). Açık: winget-pkgs PR'ı |

### v0.1.x — Opsiyonel pencereler ve statusline v2

Doğrulanmış Ultra hesaplarında 5 saatlik pencere kalktı (ADR-041). Bu adımlar veri toplama haftası sürerken dalda geliştirilir; kurulu sürüme dokunulmaz.

| # | Adım | Bağımlılık | Durum |
|---|---|---|---|
| 12b | [Opsiyonel limit pencereleri: çekirdek, tray ve panel](12b-opsiyonel-pencereler.md) | 12 | ✅ (2026-11-07). Açık: haftalık oturum payı → Adım 14 |
| 12c | [Statusline v2: görünüm, ek bilgi, özelleştirme, tazelik](12c-statusline-v2.md) | 12b | ✅ (2026-11-07). Açık: gerçek Claude Code'da `NO_COLOR` gözlemi |
| 12d | [Claude Code mod'u (tamamlayıcı)](12d-claude-code-modu.md) | 12c | ✅ (2026-11-07, ADR-043). Açık: gerçek eşik geçişinde bant ve toast gözlemi |
| 12e | [Zaman temposu: statusline işareti ve mod'da tempo](12e-zaman-temposu.md) | 12d | ✅ |
| 12f | [Görsel mod: panel, görsel bant ve diğer görsel yüzeyler](12f-gorsel-mod.md) | 12e | ✅ |
| 12g | [Claude Code mod'unu kurma ve kaldırma](12g-mod-kurulumu.md) | 12f | ✅ |
| 12h | [Tray panelinde ikinci görünüm: Ayrıntı](12h-panel-ayrinti.md) | 12f | ✅ (2026-11-07, ADR-048) |
| 12i | [Statusline v3 çekirdeği: öğe modeli, biçimler, renkler, Powerline, yeni segmentler](12i-statusline-v3-cekirdek.md) | 12h | ✅ (2026-11-08, ADR-049) |
| 12j | [Statusline düzenleyicisi: ayrı ayarlar sayfası ve UX iyileştirmesi](12j-statusline-duzenleyici.md) | 12i | ✅ (2026-11-08, ADR-050). Açık: yeni sürüm gerekli |
| 12k | [Statusline düzen çekirdeği: ayraç öğeleri, boşluklar, hizalama](12k-statusline-duzen-cekirdek.md) | 12j | ✅ (2026-11-08, ADR-051). Açık: yeni sürüm gerekli |
| 12l | [Statusline satır düzenleyicisi: satır kartları, boşluk ve ayraç öğeleri](12l-statusline-satir-duzenleyici.md) | 12k | ✅ (2026-11-08, ADR-052). Açık: yeni sürüm gerekli |
| 12m | [Statusline zenginleştirme: yeni öğeler ve biçimler](12m-statusline-zenginlestirme.md) | 12l | ✅ (2026-11-08, ADR-053). Açık: yeni sürüm gerekli |
| 12n | [Statusline compact öğesi: oturumdaki compact sayısı ve auto-compact limiti](12n-statusline-compact.md) | 12m | ✅ (2026-11-08, ADR-054). Açık: yeni sürüm gerekli; şema v2 sürüm notu |
| 12o | [Nerd Font yardımcısı: tespit, indirme, kurulum ve terminal ayarı](12o-nerd-font-yardimcisi.md) | 12n | ✅ (2026-11-09, ADR-055). Açık: WT/VS Code'da yeniden başlatma gözlemi; yeni CLI gerekli |
| 12p | [Statusline simge setleri ve Nerd bar stilleri](12p-simge-setleri-ve-bar-stilleri.md) | 12o | ✅ (2026-11-09, ADR-056). Açık: VS Code terminali görülmedi; öğe listesinde kapsül işareti tek renkli |
| 12q | [Statusline payload alanları: efor, prompt cache, harcama limiti, PR ve depo](12q-statusline-payload-alanlari.md) | 12p | ✅ (2026-11-11, ADR-059). Açık: yeni glifler gerçek terminalde görülmedi; `pr.*` gözlenmedi |
| 12r | [Statusline canlı yenileme ve git durumu](12r-canli-yenileme-ve-git-durumu.md) | 12q | ✅ (2026-11-12, ADR-059). Açık: kullanıcının gerçek boşta koşusu; uygulama kapalıyken p95 aykırılığı |
| 12s | [Limit yüzdelerinin yuvarlama farkını füzyonda gidermek](12s-limit-yuvarlama-fuzyonu.md) | 12r | ✅ (2026-11-12, ADR-060). Açık: `/usage` ile yan yana karşılaştırma yeni sürümde; mod'un değeri |
| 12t | [Tam sayı okumalarda hız tahmini ve güvene bağlı uyarı](12t-tam-sayi-hiz-tahmini.md) | 12s | ✅ (2026-11-20, ADR-062). Açık: gerçek veride hız gerçeğin altında (≈ 0,5×), izlenmeli; haftalık 24 sa ufku (ADR-063) birkaç hafta izlenmeli; yeni sürüm gerekli |

### v0.2 — Analitik ve Atıf

| # | Adım | Bağımlılık | Durum |
|---|---|---|---|
| 13 | [Kalibrasyon motoru](13-kalibrasyon.md) | 12 (+ ≥ 1 hafta veri) | ✅ (2026-11-19, ADR-061). 24 sa MAE 0,75; açık: `weekly_all` sapması sınıra yakın, `α_sonnet` izlenmeli |
| 14 | [Atıf motoru ve pencere arşivi](14-atif-ve-pencereler.md) | 13 | ✅ (2026-11-20, ADR-064, ADR-065). 14a: atıf, pencere sonlandırma, tahmini haftalar, çapraz doğrulama · 14b: sorgu API'leri ve komutları, sentetik üretici, her sorgu p95 ≤ 39 ms; açık: ≥ 10 kapanmış pencere incelemesi veri bekliyor |
| 15 | [Ana pencere kabuğu ve Genel Bakış](15-ana-pencere-genel-bakis.md) | 14 | ✅ (2026-11-20, ADR-066, ADR-067). 15a: kabuk, yaşam döngüsü, deep link ve `tokenradar open`, uPlot sarmalayıcısı · 15b: Genel Bakış bölümleri, plan ücreti, soğuk açılış 434 ms, bellek ≤ 129 MB; açık: 75 eşik rengi, ipucu örtmesi denenmedi |
| 16 | [Oturumlar ve Projeler](16-oturumlar-ve-projeler.md) | 15 | ⬜ |
| 17 | [Pencereler, Örüntüler, özet ve v0.2 sürümü](17-pencereler-oruntuler-v0.2.md) | 16 | ⬜ |

### v0.3 — Genişleme

| # | Adım | Bağımlılık | Durum |
|---|---|---|---|
| 18 | [Native kompakt şerit (yüzen mod)](18-widget.md) | 12 | ✅ (2026-11-11, ADR-058). Açık: sürükleme, menü, kısayol, tam ekran, %150/HC ve sanal masaüstü elle denenmedi |
| 18b | [Widget'ın görev çubuğu modu](18b-gorev-cubugu-modu.md) | 18 | ✅ (2026-11-11, ADR-058). Açık: hover, sürükleme, menüden mod değişimi, koyu görev çubuğu ve %100/%150 elle denenmedi |
| 19 | [Çoklu hesap ve WSL](19-coklu-hesap-ve-wsl.md) | 12 | ⬜ |

### v0.4 — Codex

| # | Adım | Bağımlılık | Durum |
|---|---|---|---|
| 20 | [Codex provider'ı](20-codex.md) | 19 | ⬜ |

### v0.5 — Cross-platform

| # | Adım | Bağımlılık | Durum |
|---|---|---|---|
| 21 | [macOS ve Linux](21-macos-linux.md) | 20 | ⬜ |

```
01 ─┬─ 02 ─┬───────────── 04 ── 05 ── 06 ─┬─ 07 ─┐
    │      │                              └─ 08 ─┼─ 10 ── 11 ── 12 ─┬─ 13 ── 14 ── 15 ── 16 ── 17
    ├─ 03 ─┴─ (04) (06)                          │                  ├─ 18
    │                                            │                  └─ 19 ── 20 ── 21
    └──────────── 09 ────────────────────────────┘
```

**Paralellik:** 03 ve 09, 02'den sonra birbirinden bağımsız ilerleyebilir. 07 ve 08 de birbirinden bağımsızdır. 18 ile 19 v0.1'den sonra herhangi bir zamanda yapılabilir.

## Adım dosyası şablonu

```markdown
# Adım NN — Başlık

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|

## Amaç
## Önce oku
## Kapsam
## Kapsam dışı
## Görevler
- [ ] …
## Teknik notlar ve tuzaklar
## Kabul kriterleri
## Doğrulama
## Devir notu
_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
```
