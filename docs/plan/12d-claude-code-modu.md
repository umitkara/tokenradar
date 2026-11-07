# Adım 12d — Claude Code mod'u (tamamlayıcı)

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.1.x | 12c | 1 oturum | ⬜ |

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

- [ ] Geliştirme sırasında modu `--plugin-dir integrations/claude-code` veya oturumun dev-mods klasörüyle yükle.
- [ ] `tokenradar status --json` çıktısının modun ihtiyacını karşıladığını doğrula. Eksik alan varsa CLI'a eklenir, IPC'ye dokunulmaz.
- [ ] `register.tsx`: `session.start`, `session.measure`, bant, toast, `/radar`.
- [ ] `types/index.d.ts`: `$.state` sözleşmesi.
- [ ] Testler (`claude plugin test`), `claude plugin validate`, `tsc -p`.
- [ ] README'ye "Claude Code mod'u (deneysel)" bölümü (İngilizce).
- [ ] ADR-042.

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
_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
