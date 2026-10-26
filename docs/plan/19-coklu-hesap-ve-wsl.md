# Adım 19 — Çoklu hesap ve WSL

| Sürüm | Bağımlılık | Tahmini süre | Durum |
|---|---|---|---|
| v0.3 | 12 | 1-2 oturum | ⬜ |

## Amaç

Birden fazla Claude hesabını desteklemek: kişisel ve iş hesapları, `CLAUDE_CONFIG_DIR` ile ayrılmış kurulumlar. Ayrıca WSL içinde çalışan Claude Code oturumlarını, **kapalı dağıtımları uyandırmadan** takip etmek.

## Önce oku

- `docs/architecture.md` §6 (`accounts` tablosu), §11 (statusline hesap eşlemesi), §14
- `docs/data-sources.md` S1, S4, S5
- `docs/decisions.md`: ADR-006, ADR-018

## Kapsam

- **Hesap keşfi:**
  - Varsayılan `~/.claude`.
  - Kullanıcı ortamındaki `CLAUDE_CONFIG_DIR` (`HKCU\Environment` ve süreç ortamı).
  - Ayarlardan elle eklenen dizinler.
  - Çalışan WSL dağıtımlarındaki `~/.claude` dizinleri.
- **Hesap kimliği ve etiketi:**
  - `.claude.json` içindeki `oauthAccount.accountUuid` (kararlı kimlik), `displayName` ve `organizationName`.
  - Aynı hesap iki dizinde bulunursa uyarı verilir.
- **Hesap başına ayrı:** kaynaklar, S2 onayı, statusline kurulumu, kalibrasyon ve bildirim kuralları.
- **UI:**
  - Panelde ve ana pencerede hesap seçici. Tray ikonu "birincil hesabı" veya "en kritik hesabı" gösterir (ayarlanabilir).
  - Ayarlarda hesap yönetimi.
- **WSL:**
  - Çalışan dağıtımların tespiti.
  - UNC yolları üzerinden okuma.
  - Bildirim yerine stat polling (çünkü dosya izleme olayları WSL'de güvenilir değil).
  - WSL içi statusline kurulumu (Windows CLI'ı interop üzerinden çağrılır).

## Kapsam dışı

- Uzak makineler (SSH) — vizyon dokümanında kapsam dışı.
- macOS Keychain ile çoklu hesap (Adım 21).

## Görevler

### Hesaplar

- [ ] **Keşif doğrulaması:** `CLAUDE_CONFIG_DIR` ayarlıyken `.claude.json` dosyası nerede tutuluyor? (Varsayılan durumda dosya `~/.claude.json` konumunda, yani `.claude` dizininin **dışında**.) Gerçek bir test kurulumuyla doğrula ve `data-sources.md` S5'e işle.
- [ ] `.claude.json` okuma:
  - Yalnızca `oauthAccount.{accountUuid, displayName, organizationName, emailAddress}` alanları okunur. **Diğer alanlar okunmaz**; dosya büyük olabilir ve içinde proje geçmişi bulunur.
  - E-posta yalnızca etiket önerisi olarak yerelde kullanılır; hiçbir yere gönderilmez.
  - Etiket kullanıcı tarafından değiştirilebilir.
- [ ] `accounts` tablosu: `account_uuid` sütunu için migration (yeni ve null olabilir), ardından birleştirme mantığı.
- [ ] **Statusline hesap eşlemesi:** `transcript_path` → en uzun önek eşleşmesine göre `config_dir` → hesap. Eşleşme yoksa varsayılan hesap kullanılır ve uyarı sayacı artırılır.
- [ ] **S2 hesap başına:**
  - Her hesabın kendi `.credentials.json` dosyası, onayı ve zamanlayıcısı vardır.
  - **Toplam polling hızı da sınırlanır:** hesap sayısı arttıkça aralıklar ölçeklenir.
- [ ] **UI:**
  - Panel üst bilgisinde hesap seçici (açılır menü): etiket, plan ve küçük ikili halka.
  - "Tüm hesaplar" görünümü **yalnızca** maliyet ve token için toplam gösterir; limit yüzdeleri hesap bazındadır ve toplanamaz.
  - Tray için ayar: birincil hesap veya en kritik hesap (varsayılan: en kritik). Tooltip tüm hesapları listeler.
  - Ayarlar → Hesaplar: listele, etiketi düzenle, dizin ekle/kaldır, her hesap için statusline ve API durumu.
- [ ] Bildirim metinlerine hesap etiketi eklenir (birden fazla hesap varsa).

### WSL

- [ ] **Çalışan dağıtımlar:** `wsl.exe -l --running -q`. Çıktı **UTF-16LE** kodlamalıdır ve NUL baytları içerir; doğru decode edilmeli. WSL kurulu değilse veya çağrı başarısızsa sessizce atlanır.
  - Çağrı sıklığı: 60 sn'de bir, yalnızca WSL kaynağı yapılandırılmışsa veya ilk keşifte.
  - **Kapalı dağıtıma asla erişilmez** (ADR-018).
- [ ] **Keşif:** `\\wsl.localhost\<distro>\home\<kullanıcı>\.claude` dizinleri (`/home/*` taranır; `/root` yalnızca erişilebilirse).
  - Bulunan dizinler kullanıcıya **önerilir**; otomatik olarak eklenmez. Onboarding veya ayarlar "WSL'de Claude Code bulundu: Ubuntu (alice)" önerisini gösterir.
- [ ] **İzleme:** UNC (9P) üzerinde `ReadDirectoryChangesW` güvenilir değil. WSL kaynakları için stat polling yapılır: 30 sn'de bir, yalnızca dağıtım çalışıyorsa.
  - Dağıtım durunca kaynak "çevrimdışı" olarak işaretlenir. Son veri gösterilir, yanında "WSL kapalı" rozeti bulunur.
- [ ] **Performans:** 9P üzerinden okuma yavaştır. İlk indeksleme hızını ölç ve ilerleme göstergesine "WSL" etiketi ekle. Arka plan önceliği burada da geçerli.
- [ ] **Canlı oturumlar:** WSL içindeki `sessions/*.json` okunur. Ancak pid'ler Linux pid'leri olduğu için Windows'tan doğrulanamaz.
  - Doğrulama yöntemi: `updatedAt` tazeliği (≤ 2 dk) ve `status` alanı.
  - Bu sınırlama UI'da belirtilir.
- [ ] **WSL statusline kurulumu:**
  - WSL içindeki `settings.json` dosyasına (UNC yolu üzerinden) komut yazılır: `/mnt/c/Users/<u>/AppData/Local/Programs/TokenRadar/bin/tokenradar.exe statusline`. Yol, kurulum dizinine göre `wslpath` mantığıyla dönüştürülür.
  - Interop kapalıysa (`/etc/wsl.conf` → `interop.enabled=false`) bu yöntem çalışmaz; kullanıcıya bildirilir.
  - **Ölçüm:** Interop üzerinden exe çağırmanın gecikmesi. Bütçe dışında kalırsa (> 100 ms) belgele. v0.5'te Linux CLI derlemesi bir alternatif olabilir.
- [ ] **Hesap eşlemesi:** WSL'den gelen `transcript_path` bir Linux yoludur (`/home/alice/.claude/...`). Eşleme, dağıtım bağlamıyla yapılır. Statusline push mesajına `wsl_distro` alanı eklenir; `WSL_DISTRO_NAME` ortam değişkeninden okunur.

### Testler

- [ ] Hesap keşfi (sahte dizinler ve ortam değişkenleri).
- [ ] `transcript_path` → hesap eşleme (Windows yolu, UNC yolu, Linux yolu + dağıtım).
- [ ] UTF-16 `wsl.exe` çıktısının decode edilmesi (fixture).
- [ ] Kapalı dağıtıma erişilmediğinin testi: sahte `wsl` komutu boş liste döndürürse hiçbir UNC çağrısı yapılmaz.
- [ ] Polling hız sınırının çoklu hesapla ölçeklenmesi.

## Teknik notlar ve tuzaklar

- **Aynı hesap, iki kurulum:** Bir kullanıcı aynı hesabı hem Windows'ta hem WSL'de kullanabilir. Bu durumda limitler **aynıdır** ve `accountUuid` eşleşir. UI bu iki kaynağı tek hesap altında birleştirmeli; aynı limit iki kez gösterilmemeli. Atıfta maliyetler birleştirilir, oturumlarda kaynak rozeti (Windows/WSL) gösterilir.
- **Bu makinede WSL kurulu değil** (planlama oturumunda gözlemlendi). Test için WSL kurulumu gerekebilir; kullanıcıya sor. Kurulamıyorsa fixture ve sahte komutlarla ilerle, manuel testi açık konu olarak not et.

## Kabul kriterleri

- [ ] İki farklı `CLAUDE_CONFIG_DIR` hesabı aynı anda doğru izlenir. Panel hesap değiştirince doğru limitleri gösterir.
- [ ] Aynı hesap iki kurulumda tek hesap olarak görünür.
- [ ] WSL: çalışan dağıtımda indeksleme ve canlı oturum çalışır. Dağıtım kapalıyken hiçbir erişim olmaz (Process Monitor veya `wsl -l --running` ile doğrulanır).
- [ ] WSL statusline köprüsü çalışır veya sınırlaması belgelenmiştir.

## Doğrulama

```powershell
cargo test -p tokenradar-core accounts wsl
$env:CLAUDE_CONFIG_DIR="$env:USERPROFILE\.claude-work"; claude   # ikinci hesap testi
wsl -l --running
```

## Devir notu

_(Oturum sonunda doldurulur: yapılanlar · sapmalar · açık konular · sonraki adıma notlar)_
