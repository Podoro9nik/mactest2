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

/** Exercise the actual transport without sending data or credits to a service. */
class McpFlowTest {
    private class StudioTransport : Interceptor {
        val calls = mutableListOf<String>()
        var failKeep = false
        var sse = false
        var unlocked = false
        override fun intercept(chain: Interceptor.Chain): Response {
            val request = chain.request()
            assertEquals("https://elyum.ai/mcp", request.url.toString())
            assertEquals("Bearer test-key", request.header("Authorization"))
            val buffer = Buffer()
            request.body!!.writeTo(buffer)
            val message = JSONObject(buffer.readUtf8())
            val method = message.getString("method")
            val params = message.getJSONObject("params")
            val name = params.optString("name")
            if (method != "initialize") {
                assertEquals("test-session", request.header("Mcp-Session-Id"))
                assertEquals("2025-03-26", request.header("MCP-Protocol-Version"))
            }
            calls += if (name.isNotBlank()) name else method
            val result = when (method) {
                "initialize" -> JSONObject().put("protocolVersion", "2025-03-26")
                "notifications/initialized" -> JSONObject()
                "tools/call" -> {
                    val data = when (name) {
                        "elyum_account" -> JSONObject().put("credits", 100)
                        "elyum_estimate" -> JSONObject().put("credits", 9)
                        "elyum_make_image" -> JSONObject().put("jobId", "g_123abc")
                        "elyum_wait" -> JSONObject("""{"status":"done","genId":"g_123abc","model":"nano-banana-2-1","outputs":[{"url":"https://elyum.ai/media/g_123abc/0.jpg","credits":9,"locked":${!unlocked}}]}""")
                        "elyum_keep" -> {
                            if (failKeep) throw IOException("lost response")
                            unlocked = true
                            JSONObject().put("url", "https://elyum.ai/media/g_123abc/0.jpg")
                        }
                        else -> error("Unexpected tool $name")
                    }
                    JSONObject().put("structuredContent", data)
                }
                else -> error("Unexpected method")
            }
            val response = JSONObject().put("jsonrpc", "2.0").put("id", message.opt("id")).put("result", result)
            val content = if (sse) "data: {\"jsonrpc\":\"2.0\",\"method\":\"notifications/progress\"}\n\ndata: $response\n\n" else response.toString()
            return Response.Builder().request(request).protocol(HttpProtocol.HTTP_1_1).code(if (method.startsWith("notifications/")) 202 else 200)
                .message("OK").header("Mcp-Session-Id", "test-session")
                .header("Content-Type", if (sse) "text/event-stream" else "application/json")
                .body(content.toResponseBody("application/json".toMediaType())).build()
        }
    }

    @Test fun generationAndPollingNeverKeepAutomatically() {
        val fake = StudioTransport()
        val client = ElyumClient("test-key", OkHttpClient.Builder().addInterceptor(fake).build())
        val submitted = client.submit(GenerationRequest("Picnic"), emptyList()) {}
        val preview = client.status(submitted)
        assertEquals("preview", preview.status)
        assertFalse(fake.calls.contains("elyum_keep"))
        assertEquals("completed", client.keep(preview, preview).status)
        assertEquals(1, fake.calls.count { it == "elyum_keep" })
        val completed = client.status(preview)
        client.keep(preview, completed)
        assertEquals("A recovered original must not be charged twice", 1, fake.calls.count { it == "elyum_keep" })
    }

    @Test fun paidWriteWithLostResponseIsNotRetried() {
        val fake = StudioTransport().apply { failKeep = true }
        val client = ElyumClient("test-key", OkHttpClient.Builder().addInterceptor(fake).build())
        val preview = StudioJob("g_123abc", status = "preview", genId = "g_123abc", cost = 9.0)
        try { client.keep(preview, preview); fail("Expected network error") }
        catch (e: StudioException) { assertEquals("network", e.code) }
        assertEquals(1, fake.calls.count { it == "elyum_keep" })
    }

    @Test fun sseProgressIsIgnoredUntilMatchingResponse() {
        val fake = StudioTransport().apply { sse = true }
        val client = ElyumClient("test-key", OkHttpClient.Builder().addInterceptor(fake).build())
        assertEquals(100, client.account().getInt("credits"))
    }
}
