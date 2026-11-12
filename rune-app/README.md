# Чиикавизатор · Chiikawizator · ちいかわメーカー

Нативное приложение на Rune для Windows и macOS: придумывайте истории с маленькими друзьями и превращайте фотографии в рисунки. Интерфейс доступен на русском, английском и японском языках.

## Установка

| Система | Установщик | Как установить |
| --- | --- | --- |
| Windows 10/11, 64 бита | `Chiikawizator-Rune_4.0.0_windows-x64-setup.exe` | Запустите установщик и выберите язык. Приложение устанавливается для текущего пользователя. |
| macOS 13+, Apple Silicon | `Chiikawizator-Rune_4.0.0_macos-arm64.dmg` | Откройте DMG и перетащите Chiikawizator в Applications. |
| macOS 13+, Intel | `Chiikawizator-Rune_4.0.0_macos-x64.dmg` | Откройте DMG и перетащите Chiikawizator в Applications. |

Python, браузер и отдельный сервер для запуска не требуются. Приложение открывается прямо на вашем компьютере; для создания рисунков нужен интернет. Для обработки фотографий выбранные изображения передаются в онлайн-генератор.

Это персональная сборка с уже настроенным доступом. Установщик предназначен для владельца и выбранных им пользователей. Загружаемые из сборочного репозитория файлы `.enc` сначала нужно расшифровать: порядок описан в [BUILDING.md](BUILDING.md).

## Создать рисунок

1. Откройте «Создать» → «По описанию» и напишите свою историю. Кнопки «Идеи для вдохновения» помогут начать.
2. Выберите героя, фон, настроение и формат. Доступны Чиикава, Хачиварэ, Усаги, Момонга, Ракко, Куримандзю, Шиса и Фурухоня, а также троица друзей и собственный герой.
3. Проверьте указанную стоимость и нажмите «Нарисовать». Это действие оплачивает получение готового оригинала.
4. Дождитесь результата: оригинал без водяного знака появится в «Результат» и сохранится в «Мои рисунки». Отдельного предпросмотра и повторного подтверждения нет.

## Чиикавизировать фотографии

Выберите «Чиикавизировать фото», добавьте до восьми PNG, JPEG или WebP и опишите общую сцену. Фотографии объединяются в один рисунок. Можно добавить отдельный референс.

- **Обычная чиикавизация** сохраняет узнаваемые детали и использует мягкие формы и рисовку Чиикавы. Этот режим выбран по умолчанию.
- **Полностью в существо** превращает человека в маленькое существо с мордочкой, ушками и лапками. Узнаваемость сохраняется через одежду, цвета и аксессуары.

Готовое изображение можно сохранить в выбранную папку. Если соединение прервалось, обновите статус сохранённой задачи. Для уже оплаченного рисунка повторная покупка не нужна; восстановление по ID находится в настройках.

## Язык и шрифты

Откройте «Настройки» и выберите русский, английский или японский. В интерфейс встроен Chiikawa Kawaii; японский вариант использует Chiikawa Kawaii JP с японскими символами. Устанавливать шрифты в систему не нужно. Системные диалоги выбора файлов оформляет сама операционная система.

## English

Chiikawizator is a native Rune app for **Windows x64 and macOS 13+**, with separate Apple Silicon and Intel installers. Open the Windows installer, or drag the app from the DMG into Applications. No Python, browser or separate server is required. An internet connection is needed to create drawings.

- **Create → From an idea:** describe a scene, choose a character and adjust the background, mood and image shape.
- **Transform photos:** combine up to eight photos into one scene. Classic transformation is the default; Full creature uses a creature face and body while retaining clothes, colors and accessories.
- Check the displayed price before choosing **Make a drawing**. The finished original is purchased automatically, then appears in **Result** and **My drawings**, without a preview step.
- Change the interface language in **Settings**. If a connection is interrupted, refresh the saved task instead of starting it again. Previously purchased drawings can also be recovered by ID.

Personal installers include configured access and are intended for their owner and designated users. For encrypted build downloads and source build instructions, see [BUILDING.md](BUILDING.md).

## 日本語

ちいかわメーカーは、**Windows 64ビット版とmacOS 13以降**で使えるRune製のアプリです。MacはApple Silicon版とIntel版があります。Windowsではインストーラーを実行し、MacではDMG内のアプリをApplicationsへドラッグしてください。Pythonやブラウザー、別のサーバーは不要です。作品を描くときはインターネット接続が必要です。

- **つくる → ことばから**：物語を書いて、友だち、背景、気持ち、画像の形を選びます。
- **写真から**：写真を8枚まで追加して、ひとつの場面にまとめます。「いつもの変身」が初期設定です。「からだも完全に変身」では、服や色、小物にその人らしさを残しながら、顔や体も小さな生きものに変わります。
- 表示された料金を確認して**描いてもらう**を押すと、完成した原本を自動的に購入します。プレビューでの確認はなく、透かしのない原本が**できあがり**と**作品たち**に表示されます。
- **設定**で日本語、英語、ロシア語を切り替えられます。接続が途切れたときは、保存済みの処理の状態を確認してください。購入済み作品はIDからも復元でき、再購入は不要です。

日本語表示には、アプリに組み込まれたChiikawa Kawaii JPを使います。配布ファイルには個人用の接続設定が含まれます。暗号化されたファイルの復元とソースからのビルドについては[BUILDING.md](BUILDING.md)をご覧ください。

---

Версия 4.0.0. Персонажи: nagano / chiikawa committee. Сведения о шрифтах и их лицензиях: [assets/fonts/NOTICE.md](assets/fonts/NOTICE.md).
