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
| 09 | [Frontend temeli: token'lar, tema, i18n, tipli köprü](09-frontend-temeli.md) | 02, 03 | ⬜ |
| 10 | [Tray paneli](10-tray-paneli.md) | 07, 08, 09 | ⬜ |
| 11 | [Bildirimler, ayarlar ve onboarding](11-bildirim-ayar-onboarding.md) | 10 | ⬜ |
| 12 | [Paketleme, güncelleme ve v0.1 sürümü](12-paketleme-ve-v0.1.md) | 11 | ⬜ |

### v0.2 — Analitik ve Atıf

| # | Adım | Bağımlılık | Durum |
|---|---|---|---|
| 13 | [Kalibrasyon motoru](13-kalibrasyon.md) | 12 (+ ≥ 1 hafta veri) | ⬜ |
| 14 | [Atıf motoru ve pencere arşivi](14-atif-ve-pencereler.md) | 13 | ⬜ |
| 15 | [Ana pencere kabuğu ve Genel Bakış](15-ana-pencere-genel-bakis.md) | 14 | ⬜ |
| 16 | [Oturumlar ve Projeler](16-oturumlar-ve-projeler.md) | 15 | ⬜ |
| 17 | [Pencereler, Örüntüler, özet ve v0.2 sürümü](17-pencereler-oruntuler-v0.2.md) | 16 | ⬜ |

### v0.3 — Genişleme

| # | Adım | Bağımlılık | Durum |
|---|---|---|---|
| 18 | [Mini widget](18-widget.md) | 12 | ⬜ |
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
