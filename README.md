# Chiikawizator 3

Standalone Chiikawa illustration studio: Rust/Tauri for Windows and macOS, Kotlin/Jetpack Compose for Android. No Python installation, Gradio server or VPS. Internet access to the chosen provider is required.

## Русский

Выберите русский, английский или японский язык в приложении. Создайте сцену по описанию или загрузите до 8 фотографий: они объединятся в одну сцену. Обычная чиикавизация включена по умолчанию; полное превращение в существо выбирается отдельно.

Elyum MCP использует только Nano Banana 2.1. Сначала приложение показывает превью и цену. Оригинал без водяных знаков оплачивается и сохраняется только после явного подтверждения. Проверка задачи и восстановление истории не подтверждают оплату. После сбоя скачивания оплаченный оригинал можно получить повторно без новой генерации.

На Windows и macOS доступен Codex Plan Subscription через официальный комплектный Codex CLI: выполните собственный вход через ChatGPT. На Android используется Elyum; настольный Codex app-server на Android не поддерживается. Учётные данные ChatGPT в установщики не включаются.

Персональные сборки содержат предоставленный владельцем ключ Elyum. Встроенный ключ можно извлечь из приложения: не распространяйте такие установщики публично. В репозитории ключа нет; для GitHub Actions используется secret `CHIIKAWA_EMBEDDED_MCP_KEY`.

## English

Choose Russian, English or Japanese in the app. Describe a scene or combine up to 8 photos into one illustration. Classic transformation is the default; full creature transformation is a separate option.

Elyum MCP offers Nano Banana 2.1. Review the watermarked preview and price before confirming Keep. Only that confirmation purchases the clean original. Checking or restoring jobs never purchases a preview automatically.

Desktop apps also support Codex Plan Subscription using the bundled official CLI and your own ChatGPT sign-in. Android uses Elyum. Personal builds embed the owner's Elyum key and should be kept private; source code uses a build environment variable instead of storing credentials.

## 日本語

アプリ内でロシア語・英語・日本語を切り替えられます。文章からイラストを作成するか、最大8枚の写真を1つの場面にまとめます。通常の変身が初期設定です。体形や顔も完全に変えるモードは別に選択できます。

Elyum MCPではNano Banana 2.1を使用します。まず透かし付きプレビューと料金を確認してください。明示的に保存を承認した場合のみクレジットを消費し、透かしのない原本を保存します。状態の確認や履歴の復元だけで購入されることはありません。

WindowsとmacOSでは、同梱の公式Codex CLIを使ってChatGPTにログインし、Codex Plan Subscriptionも利用できます。AndroidではElyumを使用します。個人向けビルドには所有者のElyumキーが含まれるため、公開配布しないでください。

## Build

- Windows: Rust stable, Visual Studio C++ build tools, Node 22+, then `./native/build-windows.ps1`. Produces an NSIS installer; it installs WebView2 if needed. Python is not required at runtime or for native builds.
- Android: Java 17, Android SDK 36, then `./native/build-android.ps1`. The Gradle wrapper downloads the pinned build toolchain. Signing variables are `CHIIKAWA_ANDROID_KEYSTORE`, `CHIIKAWA_ANDROID_KEY_ALIAS`, `CHIIKAWA_ANDROID_STORE_PASSWORD`, `CHIIKAWA_ANDROID_KEY_PASSWORD`. Keep the keystore for future updates.
- macOS: `.github/workflows/native-build.yml` uses actual GitHub-hosted macOS runners for Intel and Apple Silicon, runs Rust tests, builds DMG files, launches a native WebView smoke test, verifies each DMG, and uploads encrypted artifacts. Set the repository secret `CHIIKAWA_EMBEDDED_MCP_KEY` first. Because installers embed a personal key, public CI uploads only AES-256-GCM encrypted `.dmg.enc` files. Restore a DMG locally with `node native/scripts/artifact-crypto.mjs decrypt INPUT.dmg.enc OUTPUT.dmg`, using the same environment variable. Actions does not publish a GitHub Release.
- Local macOS: set `CHIIKAWA_EMBEDDED_MCP_KEY`, run `npm ci` in `native`, run `node native/scripts/prepare-codex.mjs` from the repository root, then `../node_modules/.bin/tauri build --bundles app,dmg` in `native/desktop`.

Windows/macOS packages are not signed with a commercial publisher/Apple Developer certificate. macOS notarization requires the owner's Apple Developer credentials; this project does not supply them. Android release APKs are signed with the locally generated distribution key.

The application uses the supplied ChiikawaKawaii font for display text and platform fallbacks for missing glyphs. Official reference art: [Chiikawa Official](https://www.chiikawaofficial.com/characters), © nagano / chiikawa committee. This is an unofficial creative application. Model output is probabilistic; reference matching is not guaranteed.

Provider documentation: [Elyum Studio MCP](https://elyum.ai/docs/mcp). Desktop packaging: [Tauri Windows installer](https://v2.tauri.app/distribute/windows-installer/), [Tauri DMG](https://v2.tauri.app/distribute/dmg/). Official Codex binaries are pinned to 0.147.0 with archive SHA-256 checks and accompanying licenses.
