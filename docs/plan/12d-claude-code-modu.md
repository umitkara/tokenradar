# Adım 12d — Claude Code mod'u (tamamlayıcı)

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1.x | 12c | 1 oturum | ✅ |

## Amaç

Claude Code'un yeni **mod** (plugin hooks) API'si, Claude Code arayüzüne panel, prompt üstü bant, toast ve slash komutu eklemeye izin veriyor. Kullanıcının kararıyla mod **tamamlayıcıdır**:
- Statusline köprüsü birincil veri kaynağı olarak kalır.
- Mod görselleştirir ve uyarır. Veri yine TokenRadar'dan gelir.
- Mod yoksa veya desteklenmeyen bir ortamdaysa hiçbir şey kaybolmaz.

## Önce oku

- `docs/decisions.md`: ADR-006, ADR-013, ADR-041 ve bu adımda yazılacak ADR-042
- `docs/architecture.md` §11 (CLI ve IPC)
- Adım 12c'nin devir notu (segment modeli, `status --json` çıktısı)
- Claude Code mod API'si: `plugin-authoring` skill'i ve ürettiği `claude-code.d.ts` tipleri (`session.measure`, `$.session.usage()`, `AbovePrompt`, `$.ui.toast`, `$.command.register`, `$.process.run`)

## Kapsam

- Repo içinde `integrations/claude-code/` altında mod: `.claude-plugin/plugin.json`, `hooks/hooks.json`, `hooks/register.tsx`, `types/index.d.ts`, testler.
- **Veri yolu:**
  - `session.measure` olayı ile Claude Code'un kendi `rateLimits` ve context değerleri (ücretsiz, push).
  - Projeksiyon, scoped limit ve oturum payı için `$.process.run(["tokenradar", "status", "--json"])`.
  - Çağrı seyrek yapılır: `session.measure` başına en fazla bir kez ve en az 30 s arayla. `tokenradar` PATH'te yoksa mod sessizce yalnızca Claude Code değerleriyle çalışır.
- **Yüzeyler:**
  - `AbovePrompt` bandı: yalnızca bir eşik aşıldığında (ör. haftalık ≥ %80 veya reset'ten önce %100 projeksiyonu) görünür, aksi halde hiçbir şey çizmez.
  - Toast: eşik geçişlerinde bir kez. Masaüstü uygulamasının bildirimleriyle çift bildirim olmaması için kural yazılır.
  - `/radar` slash komutu: limitlerin, oturum payının ve kaynakların ayrıntılı dökümü.
  - Pane (opsiyonel): geniş terminalde canlı limit paneli. Bütçe ve değer değerlendirilir, gerekmiyorsa yapılmaz.
- **İlkeler:**
  - Mod konuşma içeriği okumaz. `prompt.submit`, `prompt.compose` ve tool olaylarına hook eklenmez.
  - Ağ çağrısı yapmaz (ADR-013). Kimlik bilgisine dokunmaz (ADR-006).
  - Her değer kaynağıyla gösterilir, tahminler `≈` ile işaretlenir.
  - Metinler EN/TR. Mod'un Fluent'i nasıl kullanacağı (veya CLI'dan hazır metin alacağı) bu adımda kararlaştırılır.
- ADR-042: modun rolü, veri yolu ve dağıtımı.

## Kapsam dışı

- Modu bir marketplace'te yayınlamak veya repoya `marketplace.json` eklemek. Bu dışa dönük bir adımdır, kullanıcının onayı gerekir.
- Statusline köprüsünü modla değiştirmek (kullanıcı "tamamlayıcı" rolü seçti).
- Masaüstü uygulamasında mod kurulumu için arayüz.

## Görevler

- [x] Geliştirme sırasında modu `--plugin-dir integrations/claude-code` veya oturumun dev-mods klasörüyle yükle.
- [x] `tokenradar status --json` çıktısının modun ihtiyacını karşıladığını doğrula. Eksik alan varsa CLI'a eklenir, IPC'ye dokunulmaz.
- [x] `register.tsx`: `session.start`, `session.measure`, bant, toast, `/radar`.
- [x] `types/index.d.ts`: `$.state` sözleşmesi.
- [x] Testler (`claude plugin test`), `claude plugin validate`, `tsc -p`.
- [x] README'ye "Claude Code mod'u (deneysel)" bölümü (İngilizce).
- [x] ADR-042.

## Teknik notlar ve tuzaklar

- Mod API'si yeni ve muhtemelen deneysel. API değişirse TokenRadar'ın kendisi etkilenmemeli. Mod ayrı bir klasörde durur, Rust ve uygulama build'ine bağlanmaz.
- `$.session.usage().rateLimits`, Claude Code'un son API yanıtındaki değerdir. 5 saatlik penceresi olmayan hesapta yalnızca `seven_day` gelir (ADR-041).
- Mod yalnızca Claude Code açıkken çalışır. Geçmişi ve projeksiyonu kendisi hesaplamaz, TokenRadar'dan alır.
- Mod yeniden yüklendiğinde `register` tekrar çalışır. Modül değişkenleri sıfırlanır, `$.state` korunur.

## Kabul kriterleri

- Mod yüklüyken eşik altında bant görünmez. Eşik aşılınca bant görünür ve toast bir kez çıkar.
- `/radar` doğru kaynak etiketleriyle döküm verir. TokenRadar çalışmıyorken de çalışır (yalnızca Claude Code değerleriyle, bunu belirterek).
- `claude plugin validate`, `tsc` ve `claude plugin test` temiz.

## Doğrulama

- `claude plugin validate integrations/claude-code`
- `tsc -p integrations/claude-code`
- `claude plugin test integrations/claude-code`
- Gerçek bir Claude Code oturumunda elle deneme (kullanıcıyla).

## Devir notu

**Tarih:** 2026-11-07 · **Dal:** `step/12d-claude-code-mod` · **Karar:** ADR-043 (plan ADR-042 diyordu; o numarayı 12c kullandı)

#### Kullanıcı kararları
- **Metinler:** Mod kendi küçük EN/TR tablosunu taşır. Dil, CLI'ın `status --json` çıktısına eklenen `locale` alanından gelir.
- **Toast:** Uygulama açıksa (`origin: "app"`) toast gösterilmez, bildirimi uygulama gönderir. Uygulama kapalıysa veya CLI yoksa toast mod'dan gelir.
- **Bant:** Tray ile aynı sabit eşikler kullanılır (75/90). Reset'ten önce dolma projeksiyonu uyarı seviyesi sayılır. "Gizle" düğmesi bandı bir sonraki seviyeye kadar gizler.
- **Pane yapılmadı.**
- **Elle denemede bulunan ifade hatası:** "önce sıfırlanır" / "resets first" etiketi "beklenenden önce sıfırlanacak" gibi okunuyordu. Kullanıcı kararıyla "dolmaz" / "won't fill" oldu. Değişiklik `cli.ftl` ve mod'da yapıldı; hesap aynı kaldı.

#### Yapılanlar
- **Mod (`integrations/claude-code/`):**
  - `.claude-plugin/plugin.json`: `userConfig.cli` alanı, varsayılanı `tokenradar`.
  - `hooks/register.tsx`: `session.start`, `session.measure`, `AbovePrompt` bandı, toast ve `/radar`.
  - `hooks/model.ts` (saf kurallar): ayrıştırma, birleştirme, eşikler, `stepAlerts` ve metinler.
  - `hooks/i18n.ts`: EN/TR tablosu, `num`/`pct`/`dur` portu.
  - `types/index.d.ts`: `$.state` sözleşmesi (`measure`, `radar`, `alerts`), hepsi `shape` etiketli.
  - `tsconfig.json`, `.gitignore` (`.claude-plugin/types/` yok sayılır).
- **Birleştirme kuralı:** Bir pencere için Claude Code değeri kazanır (`Claude Code` etiketiyle), TokenRadar'ın projeksiyonu bu değere eklenir. Claude Code'un bildirmediği pencereler (scoped) TokenRadar'dan kendi kaynağıyla gelir. `absent` listesindeki pencereler gösterilmez. Reset'i geçmiş pencere `≈ 0 %` olur ve uyarı üretmez.
- **Oturum payı:** Mod kendi oturumunu `live_sessions` içinde `session_uuid` ile buluyor; CLI'a `--session` gerekmedi.
- **CLI:** `status --json` çıktısına `locale` alanı eklendi; IPC yanıtı değişmedi.
- **Dokümanlar:**
  - ADR-043.
  - architecture §3 (repo düzeni), §11 (mod ve `locale`), §12 (i18n istisnası).
  - README'ye "Claude Code mod (experimental)" bölümü.
  - CHANGELOG (Unreleased).

#### Doğrulama
- `claude plugin validate integrations/claude-code`: temiz. `/radar` hook'u "answers its own command" olarak görünüyor, gating hook değil.
- `tsc -p integrations/claude-code` (TypeScript 6.0.3, `app/node_modules`): temiz.
- `claude plugin test integrations/claude-code`: 16 test geçti.
  - `model.test.ts` (9): biçimler, eski ve bozuk CLI çıktısı, birleştirme, reset, eşikler, toast durum makinesi, döküm.
  - `register.test.tsx` (7): eşik altında bant yok; eşik geçişinde bant ve seviye başına tek toast; uygulama açıkken toast yok; Gizle; 30 sn'de bir CLI çağrısı; CLI yokken `/radar`; TR `/radar`.
- `cargo fmt --check`, `cargo clippy -p tokenradar-cli --all-targets -- -D warnings`: temiz. `cargo test -p tokenradar-cli -p tokenradar-i18n`: CLI 22 + 2, i18n 16 + 6.
- **Elle deneme (kullanıcıyla, `claude --plugin-dir`):**
  - Kurulu v0.1.0 CLI'ıyla `/radar` İngilizce geldi.
  - `/config` ile yeni build seçilince `/radar` Türkçe geldi ve seçenek değişikliği modu yeniden yükledi.
  - Haftalık eşiğin altında olduğu için bant görünmedi.
  - Kullanıcı sorun bildirmedi; yalnızca yukarıdaki etiket ifadesini düzelttirdi.

#### Plandan sapmalar
- **ADR numarası** 042 değil 043 oldu.
- **`userConfig.cli` eklendi.** Plan yalnızca PATH'teki `tokenradar`'ı öngörüyordu. Bu alan, portable kurulumlar ve yeni CLI'ı 0.1.1 kurulmadan denemek için gerekli.
- **`session.start` de CLI'ı çağırıyor** (30 sn aralığına dahil), çünkü `/radar` açıklamasının dili oradan geliyor. `/radar` her çağrıda taze veri ister.
- **`/radar` çıktısı modelin bağlamına da giriyor.** Claude Code bunu yalnızca kendi yerleşik eklentilerinde gizleyebiliyor. Çıktıda yalnızca limit sayıları var, konuşma içeriği yok.
- **12c notundaki öneri** (mod span modelini kullansın) uygulanmadı. Mod satır çizmiyor, bant ve döküm kendi metinleri.

#### Açık konular ve sonraki adımlar için notlar
- **0.1.1:** Kurulu CLI'da `locale` ve `absent` yok. 0.1.1 kurulana kadar mod İngilizce konuşur ve "bu hesapta yok" satırını göstermez.
- **Gerçek bant ve toast gözlemi:** Bant ve toast yalnızca testlerde doğrulandı; gerçek bir eşik geçişi henüz gözlenmedi. Haftalık %75 aşıldığında bakılmalı.
- **CI:** Mod kontrolleri CI'da yok, çünkü `claude` CLI gerekiyor. Mod dosyası değiştiğinde üç komut elle çalıştırılır.
- **Tipler:** `tsc` için `.claude-plugin/types/` klasörünün dolu olması gerekir. Claude Code bu klasörü modu yükleyince yazar; mod hiç yüklenmediyse skill'in ürettiği `claude-code.d.ts` oraya `claude-code/index.d.ts` olarak kopyalanabilir.
- **Ortak test vektörleri:** `testdata/i18n-vectors.json` mod testlerinden okunamıyor (dosya erişimi yok). Biçim kuralları değişirse `hooks/i18n.ts` elle güncellenmeli.
- **"Bu hızla" sütunu:** Panel gibi "sıfırlanırken ≈%X" göstermek daha bilgilendirici olurdu. Bunun için IPC'ye projected-at-reset alanı gerekir; Adım 13/14'te düşünülebilir.
- **Marketplace yayını** dışa dönük bir adım; kullanıcının onayını bekliyor.
- **Geçici dosya:** `%LOCALAPPDATA%\Temp\tokenradar-12d\tokenradar.exe` yalnızca deneme içindi, silinebilir. Mod'un `/config` seçeneği hâlâ o yolu gösteriyorsa `tokenradar` olarak geri alınmalı.
