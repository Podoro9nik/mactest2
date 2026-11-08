package app.chiikawizator

import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class AppFlowTest {
    @get:Rule val app = createAndroidComposeRule<MainActivity>()

    private fun language(code: String) {
        app.onNodeWithTag("language").performClick()
        app.onNodeWithTag("lang_$code").performClick()
    }

    @Test fun ideasAreClickableAndLanguageDoesNotResetScene() {
        language("en")
        app.onNodeWithTag("idea_picnic").performScrollTo().performClick()
        app.onNodeWithTag("description").assertTextContains(Copy.text("prompt_picnic", "en"))
        language("ja")
        app.onNodeWithTag("description").assertTextContains(Copy.text("prompt_picnic", "en"))
        app.onNodeWithText(Copy.text("description", "ja")).assertExists()
        language("ru")
        app.onNodeWithText(Copy.text("description", "ru")).assertExists()
    }

    @Test fun switchingScreensKeepsFormAndShowsRecovery() {
        language("en")
        app.onNodeWithTag("description").performTextInput("My picnic scene")
        app.onNodeWithTag("tab_result").performClick()
        app.onNodeWithText(Copy.text("empty_result", "en")).assertExists()
        app.onNodeWithTag("tab_history").performClick()
        app.onNodeWithTag("job_id").assertExists()
        app.onNodeWithTag("tab_create").performClick()
        app.onNodeWithTag("description").assertTextContains("My picnic scene")
    }

    @Test fun photoModeHasNativeMultiPhotoPickerAndSeparateCreatureMode() {
        language("en")
        app.onNodeWithTag("photo_mode").performClick()
        app.onNodeWithTag("add_photos").assertExists()
        app.onNodeWithText(Copy.text("classic", "en")).assertExists()
        app.onNodeWithText(Copy.text("full_creature", "en")).performScrollTo().performClick()
        app.onNodeWithText(Copy.text("creature_help", "en")).assertExists()
    }
}
