# Chiikawizator for Android

Standalone Kotlin / Jetpack Compose application for Android 10 and later (API 29–36). It connects directly to **https://elyum.ai/mcp** with Nano Banana 2.1. It uses no Python runtime or VPS.

Russian, English and Japanese are available from the top-right language menu. The supplied Chiikawa font, official character references and the shared app icon are bundled. The Android app includes text generation, one scene from up to eight photos, classic and complete-creature transformations, supplementary reference upload, saved task history, recovery by Elyum job ID, gallery export and native sharing.

Generation stops at the watermarked preview. The **Keep original** action displays its credit price and asks for confirmation. The app refreshes the task and checks that the price and generation ID still match before the one paid write. An already unlocked task is downloaded without another keep call. Failed downloads and interrupted waiting can be recovered from history. Stopping waiting does not cancel generation at Elyum.

Codex Plan Subscription uses the official desktop app-server and is available in the Windows/macOS application, not Android.

## Build

Requirements: JDK 17, Android SDK platform 36 and build tools 35 or later. The Gradle 8.13 wrapper is included. Local Windows SDK location belongs in untracked `local.properties`, for example `sdk.dir=C:/Users/name/AppData/Local/Android/Sdk`.

```powershell
# From the repository root: loads local MCP/signing configuration without printing it.
./native/build-android.ps1

# Source validation / debug build with no embedded key:
./native/android/gradlew.bat -p native/android :app:testDebugUnitTest :app:assembleDebug
```

For independent builds provide `CHIIKAWA_EMBEDDED_MCP_KEY`. Release signing accepts `CHIIKAWA_ANDROID_KEYSTORE`, `CHIIKAWA_ANDROID_STORE_PASSWORD`, `CHIIKAWA_ANDROID_KEY_ALIAS` (default `chiikawizator`), and `CHIIKAWA_ANDROID_KEY_PASSWORD` (defaults to store password). Do not commit these values or signing files. Keep the same signing key for future updates. A key embedded in an installed client can be extracted from the application package; this packaging follows the user's explicit request.

Release output: `app/build/outputs/apk/release/app-release.apk`. Without signing configuration, Gradle produces `app-release-unsigned.apk` instead; it cannot be installed until signed.

Unit tests cover locked preview handling, price changes, invalid prices, repeat recovery after keep, MCP response validation, multiple-photo prompts and translation coverage. Instrumented tests exercise language switching, clickable ideas, retained scene state, task recovery controls and separate photo transformation modes. Run with an emulator/device using `:app:connectedDebugAndroidTest`.

The image picker grants access only to selected images. Gallery export uses MediaStore and requests no broad storage access. History and generated files remain in private app storage until explicitly exported or shared. Device backups are disabled for these private records.

Build references: [AGP 8.13 compatibility](https://developer.android.com/build/releases/agp-8-13-0-release-notes), [Compose compiler](https://developer.android.com/jetpack/androidx/releases/compose-kotlin), [Elyum Studio MCP](https://elyum.ai/docs/mcp).
