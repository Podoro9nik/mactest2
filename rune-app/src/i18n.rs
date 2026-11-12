#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Lang {
    Ru,
    En,
    Ja,
}

impl Lang {
    pub fn code(self) -> &'static str {
        match self {
            Self::Ru => "ru",
            Self::En => "en",
            Self::Ja => "ja",
        }
    }
    pub fn pick<'a>(self, ru: &'a str, en: &'a str, ja: &'a str) -> &'a str {
        match self {
            Self::Ru => ru,
            Self::En => en,
            Self::Ja => ja,
        }
    }
    pub fn text(self, key: &str) -> &'static str {
        let values = match key {
            "app" => ["Чиикавизатор", "Chiikawizator", "ちいかわメーカー"],
            "tagline" => [
                "Маленькие друзья, большие истории",
                "Little friends, big stories",
                "小さな友だち、大きな物語",
            ],
            "create" => ["Создать", "Create", "つくる"],
            "result" => ["Результат", "Result", "できあがり"],
            "history" => ["Мои рисунки", "My drawings", "作品たち"],
            "settings" => ["Настройки", "Settings", "設定"],
            "text_mode" => ["По описанию", "From an idea", "ことばから"],
            "photo_mode" => ["Чиикавизировать фото", "Transform photos", "写真から"],
            "title" => [
                "Какую историю нарисуем?",
                "What shall we draw?",
                "どんな物語にする？",
            ],
            "subtitle" => [
                "Выберите героя и добавьте немного воображения.",
                "Choose a friend and add a little imagination.",
                "友だちを選んで、想像をふくらませよう。",
            ],
            "description" => ["Ваша история", "Your story", "あなたの物語"],
            "placeholder" => [
                "Чиикава и друзья устроили пикник под сакурой...",
                "Chiikawa and friends have a picnic under cherry blossoms...",
                "ちいかわたちが桜の下でピクニック...",
            ],
            "ideas" => [
                "Идеи для вдохновения",
                "A little inspiration",
                "アイデアのヒント",
            ],
            "picnic" => ["Пикник", "Picnic", "ピクニック"],
            "stars" => ["Звёздная ночь", "Starry night", "星の夜"],
            "bakery" => ["Маленькая пекарня", "Little bakery", "小さなパン屋"],
            "photos" => ["Ваши фотографии", "Your photos", "あなたの写真"],
            "add_photo" => ["Добавить фото", "Add a photo", "写真を追加"],
            "photo_help" => [
                "До 8 фотографий объединятся в одну сцену. Можно перетащить файлы сюда.",
                "Combine up to 8 photos in one scene. You can drop files here.",
                "写真は8枚まで。ひとつの場面にまとめます。ここにファイルをドロップできます。",
            ],
            "remove" => ["Убрать", "Remove", "削除"],
            "classic" => [
                "Обычная чиикавизация",
                "Classic transformation",
                "いつもの変身",
            ],
            "creature" => [
                "Полностью в существо",
                "Full creature",
                "からだも完全に変身",
            ],
            "classic_help" => [
                "Узнаваемые детали, мягкие формы и рисовка Чиикавы.",
                "Familiar details, soft shapes and Chiikawa linework.",
                "面影を残して、ちいかわの形と絵柄に。",
            ],
            "creature_help" => [
                "Мордочка, ушки и маленькие лапки. Узнаваемость через одежду и аксессуары.",
                "A little muzzle, ears and paws. Keep identity through clothes and accessories.",
                "顔も耳も小さな手足も変身。服や小物でその人らしさを残します。",
            ],
            "characters" => [
                "Кто будет в вашей истории?",
                "Who is in your story?",
                "だれのお話にする？",
            ],
            "own" => ["Свой герой", "Your own", "オリジナル"],
            "trio" => ["Троица друзей", "The three friends", "なかよし3人組"],
            "chiikawa" => ["Чиикава", "Chiikawa", "ちいかわ"],
            "hachiware" => ["Хачиварэ", "Hachiware", "ハチワレ"],
            "usagi" => ["Усаги", "Usagi", "うさぎ"],
            "momonga" => ["Момонга", "Momonga", "モモンガ"],
            "rakko" => ["Ракко", "Rakko", "ラッコ"],
            "kurimanju" => ["Куримандзю", "Kurimanju", "くりまんじゅう"],
            "shisa" => ["Шиса", "Shisa", "シーサー"],
            "furuhonya" => ["Фурухоня", "Furuhonya", "古本屋"],
            "details" => ["Последние штрихи", "Finishing touches", "仕上げのひと工夫"],
            "background" => ["Фон", "Background", "背景"],
            "scene" => ["По описанию", "From the story", "物語に合わせる"],
            "white" => ["Белый", "White", "白"],
            "meadow" => ["Лужайка", "Meadow", "草原"],
            "room" => ["Уютная комната", "Cozy room", "くつろぐお部屋"],
            "sticker" => ["Стикер", "Sticker", "ステッカー"],
            "emotion" => ["Настроение", "Mood", "気持ち"],
            "happy" => ["Радость", "Happy", "うれしい"],
            "shy" => ["Смущение", "Shy", "はずかしい"],
            "tears" => ["Слёзки", "Little tears", "なみだ"],
            "surprised" => ["Удивление", "Surprised", "びっくり"],
            "ratio" => ["Формат", "Shape", "画像の形"],
            "resolution" => ["Размер", "Size", "大きさ"],
            "strength" => ["Сила превращения", "Transformation strength", "変身の強さ"],
            "reference" => [
                "Дополнительный референс",
                "Extra reference",
                "追加の参考画像",
            ],
            "add_reference" => ["Выбрать референс", "Choose a reference", "参考画像を選ぶ"],
            "generate" => ["Нарисовать", "Make a drawing", "描いてもらう"],
            "credits" => ["кредитов", "credits", "クレジット"],
            "price_loading" => [
                "Узнаём стоимость...",
                "Checking the price...",
                "料金を確認中...",
            ],
            "price_retry" => ["Узнать стоимость", "Check price", "料金を確認"],
            "payment_note" => [
                "После нажатия оригинал будет получен автоматически за указанную сумму.",
                "Your click purchases the finished original automatically at this price.",
                "押すと、表示された料金で完成した原本を自動的に受け取ります。",
            ],
            "ready_hint" => [
                "Здесь появится ваш рисунок",
                "Your drawing will appear here",
                "ここに作品ができあがります",
            ],
            "ready_help" => [
                "Готовый оригинал сразу появится здесь и сохранится в ваших рисунках.",
                "The finished original appears here and is saved in My drawings.",
                "完成した原本がここに表示され、作品たちに保存されます。",
            ],
            "preparing" => [
                "Готовим вашу историю...",
                "Preparing your story...",
                "物語を準備中...",
            ],
            "uploading" => [
                "Передаём фотографии...",
                "Sending your photos...",
                "写真を送信中...",
            ],
            "generating" => [
                "Рисуем маленькое чудо...",
                "Drawing a little wonder...",
                "小さな世界を描いています...",
            ],
            "unlocking" => [
                "Получаем оригинал...",
                "Getting the original...",
                "原本を受け取り中...",
            ],
            "downloading" => [
                "Сохраняем рисунок...",
                "Saving your drawing...",
                "作品を保存中...",
            ],
            "completed" => [
                "Ваш рисунок готов",
                "Your drawing is ready",
                "作品ができました",
            ],
            "failed" => [
                "Не получилось",
                "Something went wrong",
                "うまくいきませんでした",
            ],
            "export" => ["Сохранить файл", "Save a file", "ファイルを保存"],
            "resume" => ["Обновить статус", "Refresh status", "状態を確認"],
            "new_drawing" => ["Ещё один рисунок", "Another drawing", "もう一枚つくる"],
            "open" => ["Открыть", "Open", "開く"],
            "empty_history" => [
                "Пока здесь тихо. Давайте создадим первый рисунок!",
                "A quiet little space. Let's make your first drawing!",
                "まだ何もありません。最初の一枚をつくろう！",
            ],
            "history_help" => [
                "Ваши оригиналы хранятся на этом компьютере.",
                "Your originals are stored on this computer.",
                "原本はこのコンピューターに保存されています。",
            ],
            "language" => ["Язык", "Language", "言語"],
            "connection" => ["Подключение", "Connection", "接続"],
            "connected" => [
                "Всё готово к рисованию",
                "Ready to draw",
                "描く準備ができました",
            ],
            "checking" => [
                "Проверяем подключение...",
                "Checking connection...",
                "接続を確認中...",
            ],
            "check" => ["Проверить", "Check", "確認"],
            "storage" => ["Папка с рисунками", "Drawing folder", "作品の保存先"],
            "restore" => [
                "Восстановить рисунок по ID",
                "Recover a drawing by ID",
                "IDから作品を復元",
            ],
            "restore_help" => [
                "Вставьте ID ранее оплаченного рисунка. Повторной оплаты не будет.",
                "Paste the ID of a previously purchased drawing. There is no second charge.",
                "購入済み作品のIDを入力します。再び料金はかかりません。",
            ],
            "restore_button" => ["Восстановить", "Recover", "復元"],
            "version" => ["Версия", "Version", "バージョン"],
            "saved" => ["Файл сохранён", "File saved", "保存しました"],
            "all_local" => [
                "Приложение работает на вашем компьютере. Для рисования нужен интернет.",
                "The app runs on your computer. Drawing needs an internet connection.",
                "アプリはこのコンピューターで動きます。描くときはインターネットが必要です。",
            ],
            "font_note" => [
                "Шрифт Chiikawa Kawaii и японская адаптация Chiikawa Kawaii JP.",
                "Chiikawa Kawaii font with the Chiikawa Kawaii JP Japanese adaptation.",
                "Chiikawa Kawaiiと日本語版のChiikawa Kawaii JPを使用。",
            ],
            "credits_note" => [
                "Неофициальное творческое приложение. Персонажи: nagano / chiikawa committee.",
                "An unofficial creative app. Characters: nagano / chiikawa committee.",
                "非公式の創作アプリです。キャラクター：nagano / chiikawa committee。",
            ],
            "footer" => [
                "Чуть больше чиикавы в каждом дне.",
                "A little more Chiikawa every day.",
                "毎日に、ちょっとちいかわ。",
            ],
            _ => ["", "", ""],
        };
        values[match self {
            Self::Ru => 0,
            Self::En => 1,
            Self::Ja => 2,
        }]
    }

    pub fn error(self, code: &str) -> &'static str {
        match code {
            "photos_required" => self.pick("Добавьте хотя бы одну фотографию.", "Add at least one photo.", "写真を一枚以上追加してください。"),
            "too_many_photos" => self.pick("Можно добавить не больше 8 фото.", "You can add up to 8 photos.", "写真は8枚まで追加できます。"),
            "description_required" => self.pick("Опишите вашу идею или выберите подсказку.", "Describe an idea or choose an inspiration.", "物語を書くか、ヒントを選んでください。"),
            "description_too_long" => self.pick("Описание слишком длинное. Максимум 1200 символов.", "Keep your description under 1200 characters.", "説明は1200文字以内にしてください。"),
            "image_invalid" | "image_too_small" | "image_too_large" | "unsupported_image" => self.pick("Не удалось прочитать фото. Выберите PNG, JPEG или WebP до 35 МБ.", "Choose a PNG, JPEG or WebP image under 35 MB.", "35MB以内のPNG、JPEG、WebP画像を選んでください。"),
            "upload_too_large" | "upload_failed" | "upload_response" => self.pick("Фото не загрузилось. Генерация не начиналась. Попробуйте снова.", "The photo did not upload. Generation has not started. Please try again.", "写真を送信できませんでした。生成は始まっていません。もう一度お試しください。"),
            "insufficient_credits" => self.pick("Для рисунка недостаточно кредитов.", "There are not enough credits for this drawing.", "この作品に必要なクレジットが足りません。"),
            "unknown_price" | "price_changed" => self.pick("Стоимость изменилась или пока неизвестна. Оплата не выполнялась.", "The price changed or is unavailable. No payment was made.", "料金が変わったか、確認できません。支払いは行っていません。"),
            "keep_uncertain" => self.pick("Ответ об оплате не получен. Обновите статус: повторной оплаты не будет.", "Payment confirmation was not received. Refresh status; it will not be charged again.", "支払いの応答がありません。状態を確認してください。再び支払うことはありません。"),
            "submit_uncertain" => self.pick("Отправку не удалось подтвердить. Проверьте историю перед новой попыткой.", "Submission could not be confirmed. Check history before trying again.", "送信を確認できませんでした。もう一度試す前に履歴を確認してください。"),
            "download_failed" => self.pick("Рисунок готов, но не скачался. Обновите статус, чтобы получить его без повторной оплаты.", "The drawing is ready. Refresh status to download it without another payment.", "作品は完成しています。状態を確認すると、再購入せずに保存できます。"),
            "missing_key" | "auth" | "scope" | "browser_only" => self.pick("Подключение недоступно. Проверьте настройки доступа.", "The connection is unavailable. Check access settings.", "接続できません。アクセス設定を確認してください。"),
            "already_running" | "instance_running" => self.pick("Чиикавизатор уже открыт в другом окне.", "Chiikawizator is already open in another window.", "ちいかわメーカーは別のウィンドウで開いています。"),
            "read_only_job" | "preview_not_ready" | "original_locked" | "legacy_locked" => self.pick("Этот рисунок ещё не оплачен. Его нельзя получить без новой оплаты.", "This drawing has not been purchased yet.", "この作品はまだ購入されていません。"),
            "invalid_job_id" | "job_not_found" | "invalid_task_id" | "task_not_found" => self.pick("Не удалось найти рисунок с таким ID.", "No drawing was found for this ID.", "このIDの作品が見つかりません。"),
            "content_refused" => self.pick("Запрос отклонён. Измените описание или фотографии.", "The request was declined. Change the description or photos.", "リクエストを受け付けられません。説明や写真を変更してください。"),
            "rate_limit" | "key_cap" | "maintenance" | "renewal_failed" => self.pick("Сервис временно недоступен или достигнут лимит. Попробуйте позже.", "The service is unavailable or a limit was reached. Please try later.", "一時的に利用できないか、上限に達しました。時間をおいてお試しください。"),
            "waiting_paused" | "interrupted" => self.pick("Ожидание прервано. Задача сохранена: обновите статус позже.", "Waiting paused. The task is saved; refresh its status later.", "待機を中断しました。処理は保存済みです。あとで状態を確認してください。"),
            "save_failed" | "storage_error" => self.pick("Не удалось сохранить файл. Проверьте доступ к папке и свободное место.", "Could not save the file. Check folder access and free space.", "保存できません。フォルダーへのアクセスと空き容量を確認してください。"),
            "generation_in_progress" => self.pick("Дождитесь завершения текущего рисунка.", "Wait for the current drawing to finish.", "現在の作品が完成するまでお待ちください。"),
            "generation_failed" => self.pick("Не удалось создать рисунок. Измените описание или повторите позже.", "The drawing could not be created. Change the description or try later.", "作品を生成できませんでした。説明を変えるか、あとでお試しください。"),
            "result_not_ready" => self.pick("Рисунок ещё не готов. Обновите статус позже.", "The drawing is not ready yet. Refresh status later.", "作品はまだ完成していません。あとで状態を確認してください。"),
            "invalid_settings" => self.pick("Проверьте выбранные параметры рисунка.", "Check the selected drawing settings.", "作品の設定を確認してください。"),
            _ => self.pick("Не удалось выполнить действие. Проверьте соединение и повторите позже.", "The action could not be completed. Check your connection and try later.", "操作を完了できませんでした。接続を確認し、あとでお試しください。"),
        }
    }
}
