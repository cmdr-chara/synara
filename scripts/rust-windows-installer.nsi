; Native Rust/GPUI Synara installer. Build-time values are supplied by
; scripts/build-rust-windows-artifact.sh with makensis -D defines.
Unicode true
SetCompressor lzma

!ifndef APP_NAME
  !error "APP_NAME is required"
!endif
!ifndef VERSION
  !error "VERSION is required"
!endif
!ifndef EXECUTABLE_NAME
  !error "EXECUTABLE_NAME is required"
!endif
!ifndef INSTALL_GUID
  !error "INSTALL_GUID is required"
!endif
!ifndef SHELL_BIN
  !error "SHELL_BIN is required"
!endif
!ifndef BACKEND_BIN
  !error "BACKEND_BIN is required"
!endif
!ifndef UPDATE_CONFIG
  !error "UPDATE_CONFIG is required"
!endif
!ifndef ICON_FILE
  !error "ICON_FILE is required"
!endif
!ifndef OUT_FILE
  !error "OUT_FILE is required"
!endif
!ifndef PUBLISHER
  !define PUBLISHER "Synara"
!endif

!include LogicLib.nsh
!include FileFunc.nsh

Name "${APP_NAME}"
OutFile "${OUT_FILE}"
InstallDir "$LOCALAPPDATA\Programs\${APP_NAME}"
RequestExecutionLevel user
ShowInstDetails show
ShowUninstDetails show
Icon "${ICON_FILE}"
UninstallIcon "${ICON_FILE}"

Var ForceRun

Function .onInit
  StrCpy $ForceRun "0"
  ${GetParameters} $R0
  ClearErrors
  ${GetOptions} $R0 "--force-run" $R1
  ${IfNot} ${Errors}
    StrCpy $ForceRun "1"
  ${EndIf}
  ClearErrors
  ${GetOptions} $R0 "--updated" $R1
  ${IfNot} ${Errors}
    ; The shell hands off to this installer immediately before exiting. Give
    ; the old process a short deterministic window to release its executable.
    Sleep 1500
  ${EndIf}
FunctionEnd

Function .onInstSuccess
  ${If} $ForceRun == "1"
    ExecShell "" "$INSTDIR\${EXECUTABLE_NAME}"
  ${EndIf}
FunctionEnd

Section "Install"
  SetShellVarContext current
  SetOutPath "$INSTDIR"
  File "/oname=${EXECUTABLE_NAME}" "${SHELL_BIN}"
  File "/oname=synara-backend.exe" "${BACKEND_BIN}"
  File "/oname=synara-restore-migration-backup.exe" "${BACKEND_BIN}"
  File "/oname=app-update.yml" "${UPDATE_CONFIG}"
  File "/oname=app.ico" "${ICON_FILE}"

  WriteUninstaller "$INSTDIR\Uninstall ${APP_NAME}.exe"
  CreateDirectory "$SMPROGRAMS\Synara"
  CreateShortcut "$SMPROGRAMS\Synara\${APP_NAME}.lnk" "$INSTDIR\${EXECUTABLE_NAME}" "" "$INSTDIR\app.ico" 0
  CreateShortcut "$DESKTOP\${APP_NAME}.lnk" "$INSTDIR\${EXECUTABLE_NAME}" "" "$INSTDIR\app.ico" 0

  WriteRegStr HKCU "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\${INSTALL_GUID}" "DisplayName" "${APP_NAME}"
  WriteRegStr HKCU "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\${INSTALL_GUID}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\${INSTALL_GUID}" "Publisher" "${PUBLISHER}"
  WriteRegStr HKCU "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\${INSTALL_GUID}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\${INSTALL_GUID}" "DisplayIcon" "$INSTDIR\${EXECUTABLE_NAME}"
  WriteRegStr HKCU "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\${INSTALL_GUID}" "UninstallString" "$INSTDIR\Uninstall ${APP_NAME}.exe"
  WriteRegDWORD HKCU "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\${INSTALL_GUID}" "NoModify" 1
  WriteRegDWORD HKCU "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\${INSTALL_GUID}" "NoRepair" 1
SectionEnd

Section "Uninstall"
  SetShellVarContext current
  Delete "$SMPROGRAMS\Synara\${APP_NAME}.lnk"
  Delete "$DESKTOP\${APP_NAME}.lnk"
  RMDir "$SMPROGRAMS\Synara"
  Delete "$INSTDIR\${EXECUTABLE_NAME}"
  Delete "$INSTDIR\synara-backend.exe"
  Delete "$INSTDIR\synara-restore-migration-backup.exe"
  Delete "$INSTDIR\app-update.yml"
  Delete "$INSTDIR\app.ico"
  Delete "$INSTDIR\Uninstall ${APP_NAME}.exe"
  RMDir "$INSTDIR"
  DeleteRegKey HKCU "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\${INSTALL_GUID}"
SectionEnd
