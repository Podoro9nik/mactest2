package app.chiikawizator

import android.content.ContentValues
import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.ImageDecoder
import android.net.Uri
import android.provider.MediaStore
import android.util.AtomicFile
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import okhttp3.OkHttpClient
import okhttp3.Request
import org.json.JSONArray
import java.io.ByteArrayOutputStream
import java.io.File
import java.io.IOException
import java.util.concurrent.TimeUnit
import kotlin.math.max
import kotlin.math.roundToInt

class StudioRepository(private val context: Context) {
    private val lock = Mutex()
    private val jobsFile = AtomicFile(File(context.filesDir, "jobs.json"))
    private val images = File(context.filesDir, "images").apply { mkdirs() }
    private val client = ElyumClient(BuildConfig.ELYUM_MCP_KEY)
    // Separate client intentionally has no MCP authentication headers.
    private val media = OkHttpClient.Builder().connectTimeout(15, TimeUnit.SECONDS).readTimeout(60, TimeUnit.SECONDS)
        .callTimeout(70, TimeUnit.SECONDS).followRedirects(true).followSslRedirects(false).build()
    private val jobs = mutableListOf<StudioJob>()

    init {
        runCatching {
            val data = JSONArray(jobsFile.openRead().bufferedReader().use { it.readText() })
            for (i in 0 until data.length()) runCatching { StudioJob.fromJson(data.getJSONObject(i)) }.getOrNull()?.let { jobs += it }
        }
    }

    private fun persist(job: StudioJob): StudioJob {
        val index = jobs.indexOfFirst { it.id == job.id }
        if (index < 0) jobs.add(0, job) else jobs[index] = job
        val array = JSONArray().apply { jobs.forEach { put(it.json()) } }
        val stream = jobsFile.startWrite()
        try { stream.write(array.toString().toByteArray(Charsets.UTF_8)); jobsFile.finishWrite(stream) }
        catch (e: Exception) { jobsFile.failWrite(stream); throw StudioException("storage") }
        return job
    }

    suspend fun listJobs() = withContext(Dispatchers.IO) { lock.withLock { jobs.toList() } }
    suspend fun account() = withContext(Dispatchers.IO) { lock.withLock { client.account() } }

    suspend fun generate(request: GenerationRequest, progress: (String) -> Unit): StudioJob = withContext(Dispatchers.IO) {
        lock.withLock {
            Prompts.build(request) // Validate everything before uploading anything.
            val refs = mutableListOf<ByteArray>()
            if (request.photo) request.photos.forEach { refs += normalize(Uri.parse(it)) }
            Prompts.references(request.character, refs.size, request.customReference != null).forEach {
                val bytes = context.assets.open(it).use { stream -> stream.readBytes() }
                refs += normalizeBytes(bytes)
            }
            request.customReference?.let { refs += normalize(Uri.parse(it)) }
            persist(client.submit(request, refs, progress))
        }
    }

    suspend fun refresh(id: String): StudioJob = withContext(Dispatchers.IO) {
        lock.withLock {
            val previous = jobs.find { it.id == id } ?: StudioJob(id)
            val current = persist(client.status(previous))
            download(current)
        }
    }

    suspend fun keep(confirmed: StudioJob): StudioJob = withContext(Dispatchers.IO) {
        lock.withLock {
            // A second tap enters only after the first has persisted its outcome.
            val previous = jobs.find { it.id == confirmed.id } ?: throw StudioException("invalid_job")
            val current = persist(client.status(previous))
            KeepPolicy.check(confirmed, current)
            val unlocked = persist(client.keep(confirmed, current))
            if (unlocked.status != "completed") throw StudioException("locked")
            download(unlocked)
        }
    }

    private fun download(job: StudioJob): StudioJob {
        if (job.status !in setOf("preview", "completed")) return job
        if (job.imagePath != null && File(job.imagePath).isFile) return job
        val url = job.resultUrl ?: throw StudioException("invalid_response")
        val prefix = if (job.status == "preview") "preview" else "chiikawa"
        val destination = File(images, "${prefix}_${job.id}.png")
        try {
            media.newCall(Request.Builder().url(mediaUrl(url)).get().build()).execute().use { response ->
                if (!response.isSuccessful) throw StudioException("download")
                val body = response.body ?: throw StudioException("download")
                if (body.contentLength() > 35 * 1024 * 1024) throw StudioException("image_too_large")
                val bytes = ByteArrayOutputStream()
                body.byteStream().use { input ->
                    val chunk = ByteArray(8192)
                    while (true) {
                        val read = input.read(chunk)
                        if (read < 0) break
                        if (bytes.size() + read > 35 * 1024 * 1024) throw StudioException("image_too_large")
                        bytes.write(chunk, 0, read)
                    }
                }
                val data = bytes.toByteArray()
                val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
                BitmapFactory.decodeByteArray(data, 0, data.size, bounds)
                if (bounds.outWidth.toLong() * bounds.outHeight > 40_000_000 || bounds.outWidth <= 0) throw StudioException("image_too_large")
                val bitmap = BitmapFactory.decodeByteArray(data, 0, data.size) ?: throw StudioException("download")
                val atomic = AtomicFile(destination)
                val output = atomic.startWrite()
                try {
                    if (!bitmap.compress(Bitmap.CompressFormat.PNG, 100, output)) throw StudioException("storage")
                    atomic.finishWrite(output)
                } catch (e: Exception) { atomic.failWrite(output); throw e }
                finally { bitmap.recycle() }
            }
        } catch (e: IOException) { throw StudioException("download") }
        return persist(job.copy(imagePath = destination.absolutePath))
    }

    private fun normalize(uri: Uri): ByteArray = try {
        val source = ImageDecoder.createSource(context.contentResolver, uri)
        normalizedBitmap(source)
    } catch (e: IOException) { throw StudioException("invalid_image") }
    private fun normalizeBytes(bytes: ByteArray) = normalizedBitmap(ImageDecoder.createSource(java.nio.ByteBuffer.wrap(bytes)))
    private fun normalizedBitmap(source: ImageDecoder.Source): ByteArray {
        val bitmap = ImageDecoder.decodeBitmap(source) { decoder, info, _ ->
            if (info.size.width.toLong() * info.size.height > 40_000_000) throw StudioException("image_too_large")
            decoder.allocator = ImageDecoder.ALLOCATOR_SOFTWARE
            decoder.isMutableRequired = false
            val scale = minOf(1.0, 2048.0 / max(info.size.width, info.size.height))
            decoder.setTargetSize(max(1, (info.size.width * scale).roundToInt()), max(1, (info.size.height * scale).roundToInt()))
        }
        val flat = Bitmap.createBitmap(bitmap.width, bitmap.height, Bitmap.Config.ARGB_8888)
        Canvas(flat).apply { drawColor(Color.WHITE); drawBitmap(bitmap, 0f, 0f, null) }
        val result = ByteArrayOutputStream()
        flat.compress(Bitmap.CompressFormat.JPEG, 92, result)
        bitmap.recycle(); flat.recycle()
        if (result.size() > 10 * 1024 * 1024) throw StudioException("image_too_large")
        return result.toByteArray()
    }

    suspend fun export(job: StudioJob): Uri = withContext(Dispatchers.IO) {
        val file = safeImage(job)
        val resolver = context.contentResolver
        val uri = resolver.insert(MediaStore.Images.Media.EXTERNAL_CONTENT_URI, ContentValues().apply {
            put(MediaStore.Images.Media.DISPLAY_NAME, file.name)
            put(MediaStore.Images.Media.MIME_TYPE, "image/png")
            put(MediaStore.Images.Media.RELATIVE_PATH, "Pictures/Chiikawizator")
            put(MediaStore.Images.Media.IS_PENDING, 1)
        }) ?: throw StudioException("storage")
        try {
            resolver.openOutputStream(uri)?.use { output -> file.inputStream().use { it.copyTo(output) } } ?: throw StudioException("storage")
            resolver.update(uri, ContentValues().apply { put(MediaStore.Images.Media.IS_PENDING, 0) }, null, null)
            uri
        } catch (e: Exception) { resolver.delete(uri, null, null); throw StudioException("storage") }
    }

    fun safeImage(job: StudioJob): File {
        val file = job.imagePath?.let { File(it).canonicalFile } ?: throw StudioException("download")
        if (file.parentFile != images.canonicalFile || !file.isFile) throw StudioException("download")
        return file
    }
}
