package app.chiikawizator

import org.json.JSONArray
import org.json.JSONObject
import java.time.Instant

const val MODEL = "nano-banana-2.1"
class StudioException(val code: String) : Exception(code)

data class GenerationRequest(
    val description: String, val photo: Boolean = false, val photos: List<String> = emptyList(),
    val customReference: String? = null, val character: String = "own", val background: String = "scene",
    val emotion: String = "scene", val strength: Int = 50, val fullCreature: Boolean = false,
    val aspectRatio: String = "1:1", val resolution: String = "2K", val creditMode: Boolean = false
)

data class StudioJob(
    val id: String, val status: String = "queued", val genId: String = "", val cost: Double? = null,
    val resultUrl: String? = null, val imagePath: String? = null,
    val createdAt: String = Instant.now().toString(), val description: String = "",
    val error: String? = null
) {
    fun json() = JSONObject().put("id", id).put("status", status).put("genId", genId)
        .put("cost", cost ?: JSONObject.NULL).put("resultUrl", resultUrl ?: JSONObject.NULL)
        .put("imagePath", imagePath ?: JSONObject.NULL).put("createdAt", createdAt)
        .put("description", description).put("error", error ?: JSONObject.NULL)

    companion object {
        fun fromJson(o: JSONObject): StudioJob {
            val id = o.getString("id")
            if (!isJobId(id)) throw StudioException("invalid_job")
            return StudioJob(id, o.optString("status", "queued"), o.optString("genId"),
                number(o, "cost"), o.nullableString("resultUrl"), o.nullableString("imagePath"),
                o.optString("createdAt", Instant.now().toString()), o.optString("description"), o.nullableString("error"))
        }
    }
}

fun JSONObject.nullableString(name: String): String? = if (isNull(name)) null else optString(name).takeIf { it.isNotBlank() }
fun number(o: JSONObject, name: String): Double? = (o.opt(name) as? Number)?.toDouble()?.takeIf { it.isFinite() && it >= 0 }
fun isJobId(id: String) = Regex("g_[A-Za-z0-9_-]{6,120}").matches(id)
fun mediaUrl(raw: String?): String {
    val value = raw?.let { if (it.startsWith("/media/")) "https://elyum.ai$it" else it } ?: throw StudioException("invalid_response")
    val uri = runCatching { java.net.URI(value) }.getOrNull()
    if (uri?.scheme != "https" || uri.host.isNullOrBlank() || uri.userInfo != null) throw StudioException("invalid_response")
    return value
}

object Protocol {
    fun result(message: JSONObject, id: Int): JSONObject {
        if (message.optInt("id", -1) != id || message.has("error")) throw StudioException("invalid_response")
        return message.optJSONObject("result") ?: throw StudioException("invalid_response")
    }

    fun toolData(result: JSONObject): JSONObject {
        var data = result.optJSONObject("structuredContent")
        if (data == null) {
            val content = result.optJSONArray("content") ?: JSONArray()
            for (i in 0 until content.length()) {
                val item = content.optJSONObject(i) ?: continue
                if (item.optString("type") == "text") {
                    data = runCatching { JSONObject(item.getString("text")) }.getOrNull()
                    if (data != null) break
                }
            }
        }
        val value = data ?: throw StudioException("invalid_response")
        if (result.optBoolean("isError") || (value.has("error") && !value.isNull("error"))) {
            val error = value.optJSONObject("error") ?: value
            val code = error.optString("code").takeIf { it in knownErrors } ?: httpError(error.optInt("status", 0))
            throw StudioException(code)
        }
        return value
    }

    val knownErrors = setOf("auth", "scope", "browser_only", "key_cap", "maintenance", "renewal_failed", "prompt_too_long", "locked", "insufficient_credits", "content_refused")
    fun httpError(status: Int) = when (status) {
        401 -> "auth"; 402 -> "insufficient_credits"; 403 -> "scope"; 404 -> "not_found"; 429 -> "rate_limit"; else -> "provider_error"
    }

    fun task(data: JSONObject, previous: StudioJob): StudioJob {
        when (data.optString("status")) {
            "queued", "pending" -> return previous.copy(status = "queued", error = null)
            "running", "processing" -> return previous.copy(status = "running", error = null)
            "failed", "error", "canceled", "cancelled" -> return previous.copy(status = "failed", error = "generation_failed")
            "done", "completed", "succeeded" -> Unit
            else -> throw StudioException("invalid_response")
        }
        val value = data.optJSONObject("result") ?: data
        val model = value.nullableString("model") ?: data.nullableString("model")
        if (model != null && model !in setOf(MODEL, "nano-banana-2-1", "nano-banana-2-1-edit")) throw StudioException("wrong_model")
        val outputs = value.optJSONArray("outputs") ?: value.optJSONArray("images") ?: throw StudioException("invalid_response")
        if (outputs.length() == 0) throw StudioException("invalid_response")
        val output = outputs.optJSONObject(0) ?: JSONObject().put("url", outputs.optString(0))
        val locked = output.optBoolean("locked", value.optBoolean("locked", data.optBoolean("locked", true)))
        val url = if (locked) output.nullableString("previewUrl") ?: output.nullableString("url") else output.nullableString("url")
        val status = if (locked) "preview" else "completed"
        return previous.copy(status = status,
            genId = value.nullableString("genId") ?: data.nullableString("genId") ?: previous.genId,
            cost = number(output, "credits") ?: number(value, "unlockCredits") ?: number(data, "unlockCredits") ?: number(value, "credits"),
            resultUrl = mediaUrl(url), imagePath = if (previous.status == status) previous.imagePath else null, error = null)
    }
}

object KeepPolicy {
    /** Called after a fresh server read and before the single irreversible keep call. */
    fun check(confirmed: StudioJob, current: StudioJob) {
        if (current.id != confirmed.id) throw StudioException("invalid_job")
        if (current.status == "completed") return
        if (confirmed.status != "preview" || current.status != "preview") throw StudioException("preview_required")
        if (!Regex("g_[a-f0-9]+").matches(current.genId)) throw StudioException("invalid_job")
        val old = confirmed.cost ?: throw StudioException("unknown_price")
        val now = current.cost ?: throw StudioException("unknown_price")
        if (!old.isFinite() || !now.isFinite() || old < 0 || now < 0) throw StudioException("unknown_price")
        if (old != now || current.genId != confirmed.genId) throw StudioException("price_changed")
    }
}
