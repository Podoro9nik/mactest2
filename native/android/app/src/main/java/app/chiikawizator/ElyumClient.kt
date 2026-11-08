package app.chiikawizator

import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONArray
import org.json.JSONObject
import java.io.IOException
import java.util.UUID
import java.util.concurrent.TimeUnit

/** Studio MCP only. The credential is attached to exactly one fixed origin. */
class ElyumClient(private val key: String, private val api: OkHttpClient = OkHttpClient.Builder()
    .connectTimeout(15, TimeUnit.SECONDS).readTimeout(65, TimeUnit.SECONDS).writeTimeout(65, TimeUnit.SECONDS).callTimeout(90, TimeUnit.SECONDS)
    .followRedirects(false).followSslRedirects(false).retryOnConnectionFailure(false).build()) {
    private var nextId = 0
    private var initialized = false
    private var session: String? = null
    private var protocol: String? = null
    private val reads = setOf("elyum_account", "elyum_estimate", "elyum_wait", "elyum_job_status")
    private val writes = setOf("elyum_upload", "elyum_make_image", "elyum_keep")

    private fun rpc(method: String, params: JSONObject, notification: Boolean = false, retry: Boolean = false): JSONObject {
        if (key.isBlank()) throw StudioException("missing_key")
        val id = ++nextId
        val body = JSONObject().put("jsonrpc", "2.0").put("method", method).put("params", params)
        if (!notification) body.put("id", id)
        val encodedBody = body.toString().toByteArray(Charsets.UTF_8)
        if (method == "tools/call" && params.optString("name") == "elyum_upload" && encodedBody.size > MAX_MCP_BODY_BYTES) {
            throw StudioException("upload_too_large")
        }
        val attempts = if (retry) 3 else 1
        for (attempt in 0 until attempts) {
            try {
                val request = Request.Builder().url("https://elyum.ai/mcp")
                    .header("Authorization", "Bearer ${key.trim()}")
                    .header("Accept", "application/json, text/event-stream")
                    .post(encodedBody.toRequestBody("application/json".toMediaType()))
                session?.let { request.header("Mcp-Session-Id", it) }
                protocol?.let { request.header("MCP-Protocol-Version", it) }
                api.newCall(request.build()).execute().use { response ->
                    if (!response.isSuccessful) {
                        if (retry && response.code in setOf(429, 500, 502, 503, 504) && attempt < attempts - 1) {
                            Thread.sleep((attempt + 1) * 700L)
                            return@use
                        }
                        throw StudioException(Protocol.httpError(response.code))
                    }
                    response.header("Mcp-Session-Id")?.let { session = it }
                    if (notification) return JSONObject()
                    val responseBody = response.body ?: throw StudioException("invalid_response")
                    if (response.header("Content-Type", "").orEmpty().contains("text/event-stream")) {
                        val reader = responseBody.charStream().buffered()
                        val data = StringBuilder()
                        var consumed = 0
                        while (true) {
                            val line = reader.readLine()
                            if (line == null || line.isBlank()) {
                                if (data.isNotEmpty()) {
                                    val message = runCatching { JSONObject(data.toString()) }.getOrNull()
                                    data.clear()
                                    if (message?.optInt("id", -1) == id) return Protocol.result(message, id)
                                }
                                if (line == null) break
                            } else if (line.startsWith("data:")) {
                                consumed += line.length
                                if (consumed > 2_000_000) throw StudioException("invalid_response")
                                if (data.isNotEmpty()) data.append('\n')
                                data.append(line.drop(5).trimStart())
                            }
                        }
                        throw StudioException("invalid_response")
                    }
                    if (responseBody.contentLength() > 2_000_000) throw StudioException("invalid_response")
                    return Protocol.result(JSONObject(responseBody.string()), id)
                }
            } catch (e: IOException) {
                if (attempt == attempts - 1) throw StudioException("network")
                Thread.sleep((attempt + 1) * 700L)
            } catch (e: org.json.JSONException) { throw StudioException("invalid_response") }
        }
        throw StudioException("network")
    }

    private fun initialize() {
        if (initialized) return
        val result = rpc("initialize", JSONObject().put("protocolVersion", "2025-03-26").put("capabilities", JSONObject())
            .put("clientInfo", JSONObject().put("name", "Chiikawizator-Android").put("version", BuildConfig.VERSION_NAME)), retry = true)
        protocol = result.nullableString("protocolVersion") ?: throw StudioException("invalid_response")
        rpc("notifications/initialized", JSONObject(), notification = true)
        initialized = true
    }

    fun tool(name: String, args: JSONObject): JSONObject {
        if (name !in reads && name !in writes) throw StudioException("invalid_request")
        initialize()
        val params = JSONObject().put("name", name).put("arguments", args)
        return try {
            Protocol.toolData(rpc("tools/call", params, retry = name in reads))
        } catch (e: StudioException) {
            if (e.code != "not_found") throw e
            // Streamable HTTP sessions can expire while the app is backgrounded.
            // Only reads are replayed; a user action is required to retry any write.
            initialized = false; session = null; protocol = null
            if (name !in reads) throw e
            initialize()
            Protocol.toolData(rpc("tools/call", params, retry = true))
        }
    }

    fun account() = tool("elyum_account", JSONObject())

    fun submit(r: GenerationRequest, references: List<ByteArray>, onProgress: (String) -> Unit): StudioJob {
        val prompt = Prompts.build(r)
        if (references.size > 10) throw StudioException("too_many_photos")
        onProgress("checking_account")
        account()
        val estimate = tool("elyum_estimate", JSONObject().put("kind", "image").put("model", MODEL)
            .put("images", references.size).put("resolution", r.resolution).put("prompt", prompt))
        if (estimate.optBoolean("browserOnly") && !r.creditMode) throw StudioException("browser_only")
        val urls = JSONArray()
        references.forEachIndexed { index, bytes ->
            onProgress("uploading")
            try {
                if (bytes.size > MAX_ELYUM_IMAGE_BYTES) throw StudioException("upload_too_large")
                val data = tool("elyum_upload", JSONObject().put("base64", java.util.Base64.getEncoder().encodeToString(bytes))
                    .put("mimeType", "image/jpeg").put("filename", "reference-${index + 1}.jpg"))
                urls.put(mediaUrl(data.nullableString("url")))
            } catch (e: StudioException) {
                throw StudioException(when (e.code) {
                    "network" -> "upload_failed"
                    "invalid_response" -> "upload_response"
                    else -> e.code
                })
            }
        }
        onProgress("submitting")
        val params = JSONObject().put("model", MODEL).put("prompt", prompt).put("aspectRatio", r.aspectRatio)
            .put("resolution", r.resolution).put("clientRef", UUID.randomUUID().toString())
        if (urls.length() > 0) params.put("imageUrls", urls)
        if (r.creditMode) params.put("creditMode", true)
        val data = tool("elyum_make_image", params)
        val id = data.nullableString("jobId") ?: throw StudioException("invalid_response")
        if (!isJobId(id)) throw StudioException("invalid_job")
        return StudioJob(id, cost = number(estimate, "credits"), description = r.description)
    }

    fun status(job: StudioJob): StudioJob {
        if (!isJobId(job.id)) throw StudioException("invalid_job")
        val data = tool("elyum_wait", JSONObject().put("jobId", job.id).put("timeoutSeconds", 10).put("thumbnails", false))
        return Protocol.task(data, job)
    }

    /** Caller must persist refresh/unlock before downloading. Never retry this write. */
    fun keep(confirmed: StudioJob, current: StudioJob): StudioJob {
        KeepPolicy.check(confirmed, current)
        if (current.status == "completed") return current
        val data = tool("elyum_keep", JSONObject().put("genId", current.genId).put("index", 0))
        val url = data.nullableString("url")
        return if (url != null) current.copy(status = "completed", resultUrl = mediaUrl(url), imagePath = null, error = null)
        else status(current)
    }
}
