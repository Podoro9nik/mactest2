Unicode true
ManifestDPIAware true
RequestExecutionLevel user
SetCompressor /SOLID lzma
SetCompressorDictSize 64
!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "x64.nsh"
!ifndef APP_ROOT
  !error "APP_ROOT must name the Rune application directory."
!endif
!ifndef EXE_PATH
  !error "EXE_PATH must name the built executable."
!endif
!ifndef OUTPUT_PATH
  !error "OUTPUT_PATH must name the installer output."
!endif

!define APP_NAME "Chiikawizator Rune"
!define APP_VERSION "4.0.0"
!define APP_REG "Software\ChiikawizatorRune"
!define UNINSTALL_REG "Software\Microsoft\Windows\CurrentVersion\Uninstall\ChiikawizatorRune"
Name "${APP_NAME}"
OutFile "${OUTPUT_PATH}"
InstallDir "$LOCALAPPDATA\Programs\Chiikawizator Rune"
InstallDirRegKey HKCU "${APP_REG}" "InstallDir"
Icon "${APP_ROOT}\assets\icons\icon.ico"
UninstallIcon "${APP_ROOT}\assets\icons\icon.ico"
VIProductVersion "4.0.0.0"
VIAddVersionKey /LANG=1033 "ProductName" "${APP_NAME}"
VIAddVersionKey /LANG=1033 "FileDescription" "Chiikawizator Rune installer"
VIAddVersionKey /LANG=1033 "FileVersion" "${APP_VERSION}"
VIAddVersionKey /LANG=1033 "ProductVersion" "${APP_VERSION}"
VIAddVersionKey /LANG=1033 "LegalCopyright" "Chiikawizator contributors"

!define MUI_ICON "${APP_ROOT}\assets\icons\icon.ico"
!define MUI_UNICON "${APP_ROOT}\assets\icons\icon.ico"
!define MUI_ABORTWARNING
!define MUI_LANGDLL_REGISTRY_ROOT HKCU
!define MUI_LANGDLL_REGISTRY_KEY "${APP_REG}"
!define MUI_LANGDLL_REGISTRY_VALUENAME "InstallerLanguage"
!define MUI_LANGDLL_ALWAYSSHOW
!define MUI_FINISHPAGE_RUN "$INSTDIR\chiikawizator-rune.exe"
!define MUI_FINISHPAGE_RUN_NOTCHECKED
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH
!insertmacro MUI_LANGUAGE "Russian"
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "Japanese"
!insertmacro MUI_RESERVEFILE_LANGDLL
SetFont /LANG=${LANG_RUSSIAN} "Chiikawa Kawaii" 10
SetFont /LANG=${LANG_ENGLISH} "Chiikawa Kawaii" 10
SetFont /LANG=${LANG_JAPANESE} "Chiikawa Kawaii JP" 10
LangString X64Required ${LANG_RUSSIAN} "Для приложения необходима 64-разрядная Windows."
LangString X64Required ${LANG_ENGLISH} "This application requires 64-bit Windows."
LangString X64Required ${LANG_JAPANESE} "このアプリには64ビット版Windowsが必要です。"

Function .onInit
    SetShellVarContext current
    InitPluginsDir
    File /oname=$PLUGINSDIR\Chiikawa.ttf "${APP_ROOT}\assets\fonts\ChiikawaKawaii-Regular.ttf"
    File /oname=$PLUGINSDIR\ChiikawaJP.ttf "${APP_ROOT}\assets\fonts\ChiikawaKawaiiJP-Regular.ttf"
    System::Call 'gdi32::AddFontResourceExW(w "$PLUGINSDIR\Chiikawa.ttf", i 0x10, p 0) i .r0'
    System::Call 'gdi32::AddFontResourceExW(w "$PLUGINSDIR\ChiikawaJP.ttf", i 0x10, p 0) i .r0'
    !insertmacro MUI_LANGDLL_DISPLAY
    ${IfNot} ${RunningX64}
        Abort "$(X64Required)"
    ${EndIf}
FunctionEnd

Section "Application" SEC_APP
    SetShellVarContext current
    SetOutPath "$INSTDIR"
    File /oname=chiikawizator-rune.exe "${EXE_PATH}"
    SetOutPath "$INSTDIR\resources"
    File /oname=icon.ico "${APP_ROOT}\assets\icons\icon.ico"
    File /oname=RUNE-LICENSE.txt "${APP_ROOT}\vendor\rune\LICENSE"
    File "${APP_ROOT}\assets\fonts\OFL-Yomogi.txt"
    File /oname=FONT-NOTICE.md "${APP_ROOT}\assets\fonts\NOTICE.md"
    File "${APP_ROOT}\assets\fonts\ChiikawaKawaii-Regular.ttf"
    File "${APP_ROOT}\assets\fonts\ChiikawaKawaiiJP-Regular.ttf"
    SetOutPath "$INSTDIR"
    WriteUninstaller "$INSTDIR\Uninstall.exe"
    CreateDirectory "$SMPROGRAMS\${APP_NAME}"
    CreateShortcut "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk" "$INSTDIR\chiikawizator-rune.exe" "" "$INSTDIR\resources\icon.ico"
    CreateShortcut "$SMPROGRAMS\${APP_NAME}\Uninstall.lnk" "$INSTDIR\Uninstall.exe"
    CreateShortcut "$DESKTOP\${APP_NAME}.lnk" "$INSTDIR\chiikawizator-rune.exe" "" "$INSTDIR\resources\icon.ico"
    WriteRegStr HKCU "${APP_REG}" "InstallDir" "$INSTDIR"
    WriteRegStr HKCU "${UNINSTALL_REG}" "DisplayName" "${APP_NAME}"
    WriteRegStr HKCU "${UNINSTALL_REG}" "DisplayVersion" "${APP_VERSION}"
    WriteRegStr HKCU "${UNINSTALL_REG}" "InstallLocation" "$INSTDIR"
    WriteRegStr HKCU "${UNINSTALL_REG}" "DisplayIcon" "$INSTDIR\resources\icon.ico"
    WriteRegStr HKCU "${UNINSTALL_REG}" "UninstallString" '$\"$INSTDIR\Uninstall.exe$\"'
    WriteRegStr HKCU "${UNINSTALL_REG}" "QuietUninstallString" '$\"$INSTDIR\Uninstall.exe$\" /S'
    WriteRegDWORD HKCU "${UNINSTALL_REG}" "NoModify" 1
    WriteRegDWORD HKCU "${UNINSTALL_REG}" "NoRepair" 1
SectionEnd

Function un.onInit
    SetShellVarContext current
    InitPluginsDir
    CopyFiles /SILENT "$INSTDIR\resources\ChiikawaKawaii-Regular.ttf" "$PLUGINSDIR\Chiikawa.ttf"
    CopyFiles /SILENT "$INSTDIR\resources\ChiikawaKawaiiJP-Regular.ttf" "$PLUGINSDIR\ChiikawaJP.ttf"
    System::Call 'gdi32::AddFontResourceExW(w "$PLUGINSDIR\Chiikawa.ttf", i 0x10, p 0) i .r0'
    System::Call 'gdi32::AddFontResourceExW(w "$PLUGINSDIR\ChiikawaJP.ttf", i 0x10, p 0) i .r0'
    !insertmacro MUI_UNGETLANGUAGE
FunctionEnd

Section "Uninstall"
    SetShellVarContext current
    Delete "$DESKTOP\${APP_NAME}.lnk"
    Delete "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk"
    Delete "$SMPROGRAMS\${APP_NAME}\Uninstall.lnk"
    RMDir "$SMPROGRAMS\${APP_NAME}"
    Delete "$INSTDIR\chiikawizator-rune.exe"
    Delete "$INSTDIR\resources\icon.ico"
    Delete "$INSTDIR\resources\RUNE-LICENSE.txt"
    Delete "$INSTDIR\resources\OFL-Yomogi.txt"
    Delete "$INSTDIR\resources\FONT-NOTICE.md"
    Delete "$INSTDIR\resources\ChiikawaKawaii-Regular.ttf"
    Delete "$INSTDIR\resources\ChiikawaKawaiiJP-Regular.ttf"
    RMDir "$INSTDIR\resources"
    Delete "$INSTDIR\Uninstall.exe"
    RMDir "$INSTDIR"
    DeleteRegKey HKCU "${UNINSTALL_REG}"
    DeleteRegKey HKCU "${APP_REG}"
SectionEnd
