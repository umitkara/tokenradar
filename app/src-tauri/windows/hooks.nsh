; TokenRadar installer hooks (Step 12, ADR-036), included by installer.nsi.
;
; The CLI does the risky edits: NSIS strings stop at 1024 characters, so reading a long user PATH
; here and writing it back would cut it. `tokenradar path add|remove` keeps the value's type and
; length intact. Messages are picked by the installer language (Turkish 1055, English otherwise).

!define TR_RUN_KEY "Software\Microsoft\Windows\CurrentVersion\Run"
!define TR_STARTUP_APPROVED_KEY "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run"
; Value name tauri-plugin-autostart writes (main.rs).
!define TR_AUTOSTART_VALUE "TokenRadar"
; Paths::platform() in tokenradar-core; the install directory is kept apart from it.
!define TR_DATA_DIR "$LOCALAPPDATA\TokenRadar"

!macro NSIS_HOOK_POSTINSTALL
  ; Idempotent, so updates run it too: the CLI skips a directory that is already on PATH.
  nsExec::ExecToLog '"$INSTDIR\bin\tokenradar.exe" path add'
  Pop $0
  ${If} $0 == 3
  ${AndIf} $PassiveMode <> 1
  ${AndIfNot} ${Silent}
    ${If} $LANGUAGE == 1055
      MessageBox MB_ICONEXCLAMATION|MB_OK "Kullanıcı PATH'iniz çok uzun olduğu için tokenradar komutu PATH'e eklenmedi.$\n$\nKomutu kullanmak için şu dizini PATH'e elle ekleyin:$\n$INSTDIR\bin"
    ${Else}
      MessageBox MB_ICONEXCLAMATION|MB_OK "Your user PATH is too long, so the tokenradar command was not added to it.$\n$\nTo use the command, add this directory to PATH yourself:$\n$INSTDIR\bin"
    ${EndIf}
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; An update (and, with our template, every uninstall the installer starts) keeps everything.
  ${If} $UpdateMode <> 1
    ; Close the app first, so cancelling here leaves Claude Code's settings and PATH untouched.
    !insertmacro CheckIfAppIsRunning "$INSTDIR\${MAINBINARYNAME}.exe" "${PRODUCTNAME}"

    ; Claude Code would otherwise run a statusline command that no longer exists. The CLI restores
    ; the user's previous command and keeps a backup of settings.json (ADR-009).
    nsExec::ExecToLog '"$INSTDIR\bin\tokenradar.exe" statusline uninstall --all --yes'
    Pop $0
    ${If} $0 != 0
    ${AndIf} $PassiveMode <> 1
    ${AndIfNot} ${Silent}
      ${If} $LANGUAGE == 1055
        MessageBox MB_ICONEXCLAMATION|MB_OK "TokenRadar'ın statusline köprüsü Claude Code ayarlarından kaldırılamadı.$\n$\nKaldırma tamamlandıktan sonra Claude Code'un settings.json dosyasını açın (genellikle %USERPROFILE%\.claude\settings.json) ve tokenradar.exe çalıştıran $\"statusLine$\" girdisini silin. Yedekler aynı dizinde settings.json.tokenradar-….bak adıyla durur."
      ${Else}
        MessageBox MB_ICONEXCLAMATION|MB_OK "TokenRadar could not remove its statusline bridge from Claude Code's settings.$\n$\nAfter the uninstall, open Claude Code's settings.json (usually %USERPROFILE%\.claude\settings.json) and delete the $\"statusLine$\" entry that runs tokenradar.exe. Backups sit next to it as settings.json.tokenradar-….bak."
      ${EndIf}
    ${EndIf}

    nsExec::ExecToLog '"$INSTDIR\bin\tokenradar.exe" path remove'
    Pop $0

    DeleteRegValue HKCU "${TR_RUN_KEY}" "${TR_AUTOSTART_VALUE}"
    DeleteRegValue HKCU "${TR_STARTUP_APPROVED_KEY}" "${TR_AUTOSTART_VALUE}"
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; Tauri's "Delete the application data" box (unchecked by default) also clears TokenRadar's
  ; own data directory; Tauri itself only removes the WebView2 data under the identifier.
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    SetShellVarContext current
    RmDir /r "${TR_DATA_DIR}"
  ${EndIf}
!macroend
