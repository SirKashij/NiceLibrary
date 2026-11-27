; Hooks into the NSIS installer Tauri generates (bundle.windows.nsis.installerHooks
; in tauri.conf.json), so uninstalling through Windows leaves nothing of the app
; behind. Tauri's uninstaller already removes the program, its shortcuts, its
; entry in the installed apps and the "Start with Windows" value under the
; Run key (named after productName), and, if the user ticks "Delete the
; application data", the folders in %APPDATA% and %LOCALAPPDATA%.
;
; An update runs the uninstaller too ($UpdateMode = 1); it must keep all of this.
; The names below are checked against the app's own by unit tests in
; src-tauri/src/ports/autostart_windows.rs and secrets_windows.rs.

!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    ; Whether the user switched "Start with Windows" off in the Task Manager.
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run" "NiceLibrary"
  ${EndIf}

  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    ; The Steam Web API key in the Credential Manager is app data as well.
    nsExec::Exec 'cmdkey /delete:io.github.sirkashij.nicelibrary/steam-web-api-key'
    Pop $0
  ${EndIf}
!macroend
