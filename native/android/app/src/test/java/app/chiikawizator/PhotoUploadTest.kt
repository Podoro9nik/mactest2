package app.chiikawizator

import okhttp3.Interceptor
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Protocol as HttpProtocol
import okhttp3.Response
import okhttp3.ResponseBody.Companion.toResponseBody
import okio.Buffer
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import java.io.IOException
import java.util.Base64

/** Exercise upload requests through the real client without reaching Elyum. */
class PhotoUploadTest {
    private enum class UploadReply { SUCCESS, TOO_LARGE, DISCONNECT, MALFORMED, MISSING_URL }

    private class Transport(private val uploadReply: UploadReply = UploadReply.SUCCESS) : Interceptor {
        val calls = mutableListOf<String>()
        val arguments = mutableListOf<Pair<String, JSONObject>>()
        val uploadSizes = mutableListOf<Long>()

        override fun intercept(chain: Interceptor.Chain): Response {
            val request = chain.request()
            assertEquals("https://elyum.ai/mcp", request.url.toString())
            assertEquals("Bearer test-key", request.header("Authorization"))
            val buffer = Buffer()
            request.body!!.writeTo(buffer)
            val bodySize = buffer.size
            val message = JSONObject(buffer.readUtf8())
            val method = message.getString("method")
            val params = message.getJSONObject("params")
            val name = params.optString("name")
            calls += name.ifBlank { method }
            if (name.isNotBlank()) arguments += name to params.getJSONObject("arguments")
            var status = if (method.startsWith("notifications/")) 202 else 200
            var malformed = false
            val result = when (method) {
                "initialize" -> JSONObject().put("protocolVersion", "2025-03-26")
                "notifications/initialized" -> JSONObject()
                "tools/call" -> {
                    val data = when (name) {
                        "elyum_account" -> JSONObject().put("credits", 100)
                        "elyum_estimate" -> JSONObject().put("credits", 9)
                        "elyum_upload" -> {
                            uploadSizes += bodySize
                            assertTrue("The entire UTF-8 JSON body must fit", bodySize <= MAX_MCP_BODY_BYTES)
                            when (uploadReply) {
                                UploadReply.DISCONNECT -> throw IOException("private diagnostic must not reach UI")
                                UploadReply.TOO_LARGE -> {
                                    status = 413
                                    JSONObject().put("privateDiagnostic", "must not reach UI")
                                }
                                UploadReply.MALFORMED -> {
                                    malformed = true
                                    JSONObject()
                                }
                                UploadReply.MISSING_URL -> JSONObject()
                                UploadReply.SUCCESS -> JSONObject().put("url", "/media/reference-${uploadSizes.size}.jpg")
                            }
                        }
                        "elyum_make_image" -> JSONObject().put("jobId", "g_123abc")
                        else -> error("Unexpected tool $name; upload tests must never keep or spend")
                    }
                    JSONObject().put("structuredContent", data)
                }
                else -> error("Unexpected method $method")
            }
            val response = JSONObject().put("jsonrpc", "2.0").put("id", message.opt("id")).put("result", result)
            val content = if (malformed) "not-json: private diagnostic" else response.toString()
            return Response.Builder().request(request).protocol(HttpProtocol.HTTP_1_1)
                .code(status).message("Fixture").header("Mcp-Session-Id", "test-session")
                .header("Content-Type", "application/json")
                .body(content.toResponseBody("application/json".toMediaType())).build()
        }
    }

    private fun client(transport: Transport) = ElyumClient("test-key", OkHttpClient.Builder()
        .retryOnConnectionFailure(false).addInterceptor(transport).build())

    private fun expectError(code: String, action: () -> Unit) {
        try { action(); fail("Expected $code") }
        catch (e: StudioException) { assertEquals(code, e.code) }
    }

    @Test fun photosAndStyleUploadInOrderBeforeGenerationWithMatchingJpegNames() {
        val transport = Transport()
        // Bitmap compression is tested on Android; these distinct bytes exercise transport order.
        val references = listOf("first-photo", "second-photo", "style-reference").map { it.toByteArray() }
        val request = GenerationRequest("Friends", photo = true, photos = listOf("photo1", "photo2"), character = "chiikawa")
        val job = client(transport).submit(request, references) {}
        assertEquals("g_123abc", job.id)
        assertEquals(listOf("initialize", "notifications/initialized", "elyum_account", "elyum_estimate",
            "elyum_upload", "elyum_upload", "elyum_upload", "elyum_make_image"), transport.calls)
        val uploads = transport.arguments.filter { it.first == "elyum_upload" }.map { it.second }
        uploads.forEachIndexed { index, upload ->
            assertEquals("image/jpeg", upload.getString("mimeType"))
            assertEquals("reference-${index + 1}.jpg", upload.getString("filename"))
            assertArrayEquals(references[index], Base64.getDecoder().decode(upload.getString("base64")))
        }
        val submitted = transport.arguments.last().second
        val urls = submitted.getJSONArray("imageUrls")
        assertEquals(3, urls.length())
        for (index in 0..2) assertEquals("https://elyum.ai/media/reference-${index + 1}.jpg", urls.getString(index))
        assertFalse(transport.calls.contains("elyum_keep"))
    }

    @Test fun uploadFailureNeverSubmitsGenerationOrRetriesTheWrite() {
        for ((reply, error) in listOf(UploadReply.TOO_LARGE to "upload_too_large",
            UploadReply.DISCONNECT to "upload_failed", UploadReply.MALFORMED to "upload_response",
            UploadReply.MISSING_URL to "upload_response")) {
            val transport = Transport(reply)
            expectError(error) {
                client(transport).submit(GenerationRequest("Friends"), listOf("photo".toByteArray(), "style".toByteArray())) {}
            }
            assertEquals(1, transport.calls.count { it == "elyum_upload" })
            assertFalse(transport.calls.contains("elyum_make_image"))
            assertFalse(transport.calls.contains("elyum_keep"))
        }
    }

    @Test fun base64ThatFillsBodyLimitIsRejectedBeforeUploadBecauseEnvelopeAlsoCounts() {
        val transport = Transport()
        val upload = JSONObject().put("base64", "A".repeat(MAX_MCP_BODY_BYTES))
            .put("mimeType", "image/jpeg").put("filename", "reference.jpg")
        expectError("upload_too_large") { client(transport).tool("elyum_upload", upload) }
        assertFalse(transport.calls.contains("elyum_upload"))
        assertFalse(transport.calls.contains("elyum_make_image"))
    }

    @Test fun maximumPreparedImageFitsFullSerializedRequest() {
        val transport = Transport()
        val bytes = ByteArray(MAX_ELYUM_IMAGE_BYTES) { (it % 251).toByte() }
        val job = client(transport).submit(GenerationRequest("Friends"), listOf(bytes)) {}
        assertEquals("g_123abc", job.id)
        assertEquals(1, transport.uploadSizes.size)
        assertTrue(transport.uploadSizes.single() < MAX_MCP_BODY_BYTES)
    }

    @Test fun payloadErrorsStaySpecificWithoutLeakingResponseDiagnostics() {
        assertEquals("upload_too_large", Protocol.httpError(413))
        val error = JSONObject().put("error", "private diagnostic").put("status", 413)
        for (result in listOf(
            JSONObject().put("isError", true).put("structuredContent", error),
            JSONObject().put("isError", true).put("content", org.json.JSONArray().put(
                JSONObject().put("type", "text").put("text", error.toString()))))) {
            expectError("upload_too_large") { Protocol.toolData(result) }
        }
    }

    @Test fun uploadRecoveryMessagesAreLocalizedAndDistinctFromInternetErrors() {
        for (code in listOf("upload_too_large", "upload_failed", "upload_response")) {
            val translations = listOf("ru", "en", "ja").map { language ->
                val message = Copy.error(code, language)
                assertNotEquals(code, message)
                assertNotEquals(Copy.error("network", language), message)
                assertNotEquals(Copy.error("provider_error", language), message)
                message
            }
            assertEquals(3, translations.toSet().size)
            assertTrue(translations[0].contains("Генерация не запускалась"))
            assertTrue(translations[1].contains("Generation has not started"))
            assertTrue(translations[2].contains("生成は開始されていません"))
        }
    }
}
