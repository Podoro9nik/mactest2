package app.chiikawizator

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class StudioTest {
    private val pending = StudioJob("g_123abc", cost = 9.0)
    private fun payload(locked: Boolean = true) = JSONObject("""{"status":"done","model":"nano-banana-2-1-edit","genId":"g_123abc","outputs":[{"url":"/media/g_123abc/p0.jpg","credits":9,"locked":$locked}]}""")
    private fun expectError(code: String, action: () -> Unit) {
        try { action(); fail("Expected $code") } catch (e: StudioException) { assertEquals(code, e.code) }
    }

    @Test fun lockedResponseRemainsPreview() {
        val job = Protocol.task(payload(), pending)
        assertEquals("preview", job.status)
        assertEquals(9.0, job.cost!!, 0.001)
        assertEquals("https://elyum.ai/media/g_123abc/p0.jpg", job.resultUrl)
    }
    @Test fun absentLockFlagFailsClosed() {
        val data = payload().also { it.getJSONArray("outputs").getJSONObject(0).remove("locked") }
        assertEquals("preview", Protocol.task(data, pending).status)
    }
    @Test fun unlockingClearsPreviewPathBeforeDownload() {
        val preview = Protocol.task(payload(), pending).copy(imagePath = "preview.png")
        val original = Protocol.task(payload(false), preview)
        assertEquals("completed", original.status)
        assertNull(original.imagePath)
    }
    @Test fun keepsSameConfirmedPriceOnly() {
        val job = Protocol.task(payload(), pending)
        KeepPolicy.check(job, job)
        expectError("price_changed") { KeepPolicy.check(job, job.copy(cost = 12.0)) }
        expectError("price_changed") { KeepPolicy.check(job, job.copy(genId = "g_abcdef")) }
    }
    @Test fun missingAndNonfinitePricesCannotBeCharged() {
        val job = Protocol.task(payload(), pending)
        expectError("unknown_price") { KeepPolicy.check(job, job.copy(cost = null)) }
        expectError("unknown_price") { KeepPolicy.check(job.copy(cost = Double.NaN), job) }
        expectError("unknown_price") { KeepPolicy.check(job, job.copy(cost = Double.POSITIVE_INFINITY)) }
    }
    @Test fun alreadyKeptCanRecoverWithoutAnotherConfirmationCharge() {
        val preview = Protocol.task(payload(), pending)
        KeepPolicy.check(preview, preview.copy(status = "completed", cost = null))
    }
    @Test fun queuedTaskCannotBeKept() { expectError("preview_required") { KeepPolicy.check(pending, pending) } }
    @Test fun differentTaskCannotReuseConfirmation() {
        val job = Protocol.task(payload(), pending)
        expectError("invalid_job") { KeepPolicy.check(job, job.copy(id = "g_abcdef")) }
    }
    @Test fun estimatedPriceIsNotTreatedAsActualPreviewPrice() {
        val data = payload().also { it.getJSONArray("outputs").getJSONObject(0).remove("credits") }
        assertNull(Protocol.task(data, pending).cost)
    }
    @Test fun otherModelsRejected() {
        expectError("wrong_model") { Protocol.task(payload().put("model", "other-model"), pending) }
    }
    @Test fun protocolMatchesRequestIdAndStructuredPayload() {
        val result = JSONObject("""{"id":4,"result":{"structuredContent":{"jobId":"g_123abc"}}}""")
        assertEquals("g_123abc", Protocol.toolData(Protocol.result(result, 4)).getString("jobId"))
        expectError("invalid_response") { Protocol.result(result, 7) }
    }
    @Test fun textContentResponseAndFlatErrors() {
        val result = JSONObject("""{"content":[{"type":"text","text":"{\"status\":\"running\"}"}]}""")
        assertEquals("running", Protocol.toolData(result).getString("status"))
        expectError("insufficient_credits") { Protocol.toolData(JSONObject("""{"isError":true,"structuredContent":{"error":"Not enough","code":"insufficient_credits"}}""")) }
    }
    @Test fun credentialsNeverAcceptedInMediaUrl() {
        expectError("invalid_response") { mediaUrl("https://secret@example.org/image.png") }
        expectError("invalid_response") { mediaUrl("http://elyum.ai/image.png") }
    }
    @Test fun multiplePhotosOneSceneAndCompactRefs() {
        val request = GenerationRequest("A picnic", photo = true, photos = List(8) { "photo$it" }, customReference = "ref")
        assertTrue(Prompts.build(request).contains("Images 1 through 8"))
        assertTrue(Prompts.build(request).contains("ONE coherent new scene, not a collage"))
        assertEquals(listOf("trio-fullbody.png"), Prompts.references("own", 8, true))
    }
    @Test fun defaultTransformationAndCreatureModeAreDistinct() {
        val request = GenerationRequest("Friends", photo = true, photos = listOf("photo"), strength = 30)
        assertFalse(Prompts.build(request).contains("No human neck"))
        assertTrue(Prompts.build(request.copy(fullCreature = true)).contains("No human neck"))
        assertTrue(Prompts.build(request.copy(fullCreature = true)).contains("Preserve identity ONLY"))
    }
    @Test fun allLanguagesHaveEveryTranslation() {
        for (key in Copy.keys()) for (lang in listOf("ru", "en", "ja")) {
            assertTrue("Missing $lang:$key", Copy.text(key, lang).isNotBlank())
        }
    }
    @Test fun invalidRequestsFailBeforeProviderCall() {
        expectError("description_required") { Prompts.build(GenerationRequest("")) }
        expectError("photos_required") { Prompts.build(GenerationRequest("", photo = true)) }
        expectError("invalid_request") { Prompts.build(GenerationRequest("a scene", aspectRatio = "garbage")) }
    }
    @Test fun persistedJobRetainsLockedStateAndActualPrice() {
        val job = Protocol.task(payload(), pending).copy(imagePath = "preview.png")
        assertEquals(job, StudioJob.fromJson(job.json()))
    }
}
