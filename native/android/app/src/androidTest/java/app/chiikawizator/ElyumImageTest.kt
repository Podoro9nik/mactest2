package app.chiikawizator

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Color
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Assume.assumeTrue
import org.junit.Test
import org.junit.runner.RunWith
import java.io.ByteArrayOutputStream
import java.util.Base64

@RunWith(AndroidJUnit4::class)
class ElyumImageTest {
    private fun encoded(bitmap: Bitmap, format: Bitmap.CompressFormat): ByteArray =
        ByteArrayOutputStream().also { assertTrue(bitmap.compress(format, 100, it)) }.toByteArray()

    private fun noise(width: Int, height: Int): ByteArray {
        var seed = 81
        fun channel(): Int {
            seed = seed xor (seed shl 13)
            seed = seed xor (seed ushr 17)
            seed = seed xor (seed shl 5)
            return seed and 255
        }
        val pixels = IntArray(width * height) { Color.rgb(channel(), channel(), channel()) }
        val bitmap = Bitmap.createBitmap(pixels, width, height, Bitmap.Config.ARGB_8888)
        return try { encoded(bitmap, Bitmap.CompressFormat.PNG) } finally { bitmap.recycle() }
    }

    @Test fun noisyPhotoFitsFullMcpJsonBody() {
        val source = noise(2048, 2048)
        assertTrue(source.size > MAX_MCP_BODY_BYTES)
        val jpeg = ElyumImages.fromBytes(source)
        assertTrue(jpeg.size <= MAX_ELYUM_IMAGE_BYTES)
        assertEquals(0xff, jpeg[0].toInt() and 255)
        assertEquals(0xd8, jpeg[1].toInt() and 255)
        val bitmap = BitmapFactory.decodeByteArray(jpeg, 0, jpeg.size)
        try {
            assertTrue(bitmap.width <= 2048 && bitmap.height <= 2048)
            assertEquals(bitmap.width, bitmap.height)
        } finally { bitmap.recycle() }
        val body = JSONObject().put("jsonrpc", "2.0").put("id", Int.MAX_VALUE).put("method", "tools/call")
            .put("params", JSONObject().put("name", "elyum_upload").put("arguments", JSONObject()
                .put("base64", Base64.getEncoder().encodeToString(jpeg)).put("mimeType", "image/jpeg").put("filename", "reference-10.jpg")))
        assertTrue(body.toString().toByteArray(Charsets.UTF_8).size < MAX_MCP_BODY_BYTES)
    }

    @Test fun transparencyIsWhiteAndExifOrientationIsApplied() {
        val transparent = Bitmap.createBitmap(256, 256, Bitmap.Config.ARGB_8888)
        val jpeg = try { ElyumImages.fromBytes(encoded(transparent, Bitmap.CompressFormat.PNG)) } finally { transparent.recycle() }
        val white = BitmapFactory.decodeByteArray(jpeg, 0, jpeg.size)
        try {
            val pixel = white.getPixel(128, 128)
            assertTrue(Color.red(pixel) >= 250 && Color.green(pixel) >= 250 && Color.blue(pixel) >= 250)
        } finally { white.recycle() }

        val landscape = Bitmap.createBitmap(512, 256, Bitmap.Config.ARGB_8888).apply { eraseColor(Color.RED) }
        val original = try { encoded(landscape, Bitmap.CompressFormat.JPEG) } finally { landscape.recycle() }
        // A minimal EXIF TIFF with orientation 6 (90 degrees clockwise).
        val exif = byteArrayOf(69,120,105,102,0,0,73,73,42,0,8,0,0,0,1,0,18,1,3,0,1,0,0,0,6,0,0,0,0,0,0,0)
        val withExif = byteArrayOf(0xff.toByte(),0xd8.toByte(),0xff.toByte(),0xe1.toByte(),0,34) + exif + original.copyOfRange(2, original.size)
        val rotated = ElyumImages.fromBytes(withExif)
        val portrait = BitmapFactory.decodeByteArray(rotated, 0, rotated.size)
        try { assertEquals(256, portrait.width); assertEquals(512, portrait.height) } finally { portrait.recycle() }
    }

    @Test fun livePreparedPhotoUploadWithoutGenerationOrPayment() {
        assumeTrue(InstrumentationRegistry.getArguments().getString("liveUpload") == "true")
        val bytes = ElyumImages.fromBytes(noise(1600, 1200))
        val response = ElyumClient(BuildConfig.ELYUM_MCP_KEY).tool("elyum_upload", JSONObject()
            .put("base64", Base64.getEncoder().encodeToString(bytes)).put("mimeType", "image/jpeg")
            .put("filename", "chiikawizator-android-upload-check.jpg"))
        assertTrue(mediaUrl(response.nullableString("url")).startsWith("https://"))
    }
}
