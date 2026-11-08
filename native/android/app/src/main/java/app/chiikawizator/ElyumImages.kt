package app.chiikawizator

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.ImageDecoder
import java.io.ByteArrayOutputStream
import java.nio.ByteBuffer
import kotlin.math.max
import kotlin.math.roundToInt

/** Prepare reference bytes before any request is sent. Original files are never changed. */
internal object ElyumImages {
    fun fromBytes(bytes: ByteArray): ByteArray = prepare(ImageDecoder.createSource(ByteBuffer.wrap(bytes)))

    fun prepare(source: ImageDecoder.Source): ByteArray {
        // ImageDecoder applies EXIF orientation before returning this software bitmap.
        val decoded = ImageDecoder.decodeBitmap(source) { decoder, info, _ ->
            if (info.size.width.toLong() * info.size.height > 40_000_000) throw StudioException("image_too_large")
            decoder.allocator = ImageDecoder.ALLOCATOR_SOFTWARE
            decoder.isMutableRequired = false
            val scale = minOf(1.0, 2048.0 / max(info.size.width, info.size.height))
            decoder.setTargetSize(max(1, (info.size.width * scale).roundToInt()), max(1, (info.size.height * scale).roundToInt()))
        }
        var flat = Bitmap.createBitmap(decoded.width, decoded.height, Bitmap.Config.ARGB_8888)
        try {
            Canvas(flat).apply { drawColor(Color.WHITE); drawBitmap(decoded, 0f, 0f, null) }
        } finally { decoded.recycle() }
        try {
            while (true) {
                for (quality in intArrayOf(92, 85)) {
                    val encoded = ByteArrayOutputStream()
                    if (!flat.compress(Bitmap.CompressFormat.JPEG, quality, encoded)) throw StudioException("invalid_image")
                    if (encoded.size() <= MAX_ELYUM_IMAGE_BYTES) return encoded.toByteArray()
                }
                val width = flat.width * 3 / 4
                val height = flat.height * 3 / 4
                if (minOf(width, height) < 64) throw StudioException("image_too_large")
                val reduced = Bitmap.createScaledBitmap(flat, width, height, true)
                flat.recycle()
                flat = reduced
            }
        } finally { flat.recycle() }
    }
}
