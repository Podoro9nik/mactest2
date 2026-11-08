package app.chiikawizator

import android.content.Intent
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.ImageDecoder
import android.net.Uri
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.content.FileProvider
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.File
import kotlin.math.max

private val Cream = Color(0xFFFFFAF3)
private val Paper = Color(0xFFFFFDF9)
private val Ink = Color(0xFF433A37)
private val Pink = Color(0xFFA94463)
private val Blush = Color(0xFFFCE7EC)
private val Blue = Color(0xFFE8F4F8)
private val Border = Color(0xFF9C8580)
private val Danger = Color(0xFF9E263D)
private val Kawaii = FontFamily(Font(R.font.chiikawa_kawaii))

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent { Chiikawizator() }
    }
}

@Composable
fun Chiikawizator(vm: StudioViewModel = viewModel()) {
    val state by vm.state.collectAsStateWithLifecycle()
    val t: (String) -> String = { Copy.text(it, state.language) }
    var languages by remember { mutableStateOf(false) }
    MaterialTheme(colorScheme = lightColorScheme(primary = Pink, onPrimary = Color.White,
        primaryContainer = Blush, onPrimaryContainer = Ink, secondaryContainer = Blue,
        onSecondaryContainer = Ink, background = Cream, onBackground = Ink, surface = Paper,
        onSurface = Ink, outline = Border, error = Danger)) {
        Scaffold(containerColor = Cream, topBar = {
            Column(Modifier.statusBarsPadding().background(Paper)) {
                Row(Modifier.fillMaxWidth().widthIn(max = 900.dp).padding(horizontal = 16.dp, vertical = 8.dp),
                    verticalAlignment = Alignment.CenterVertically) {
                    AssetImage("chiikawa.png", null, Modifier.size(42.dp))
                    Text(t("app"), Modifier.weight(1f).padding(start = 8.dp), fontFamily = Kawaii,
                        fontSize = 23.sp, maxLines = 1, overflow = TextOverflow.Ellipsis)
                    Box {
                        TextButton(onClick = { languages = true }, modifier = Modifier.testTag("language")) {
                            Text(when (state.language) { "ru" -> "Русский"; "ja" -> "日本語"; else -> "English" })
                        }
                        DropdownMenu(expanded = languages, onDismissRequest = { languages = false }) {
                            listOf("ru" to "Русский", "en" to "English", "ja" to "日本語").forEach { (code, label) ->
                                DropdownMenuItem(text = { Text(label) }, onClick = { vm.language(code); languages = false }, modifier = Modifier.testTag("lang_$code"))
                            }
                        }
                    }
                }
                Row(Modifier.fillMaxWidth().padding(horizontal = 8.dp), horizontalArrangement = Arrangement.SpaceEvenly) {
                    listOf("create", "result", "history", "settings").forEachIndexed { index, key ->
                        TextButton(onClick = { vm.tab(index) }, modifier = Modifier.weight(1f).testTag("tab_$key"),
                            contentPadding = PaddingValues(horizontal = 4.dp, vertical = 12.dp),
                            colors = ButtonDefaults.textButtonColors(containerColor = if (state.tab == index) Blush else Color.Transparent)) {
                            Text(t(key), fontWeight = if (state.tab == index) FontWeight.Bold else FontWeight.Normal,
                                fontSize = 13.sp, maxLines = 1, overflow = TextOverflow.Ellipsis)
                        }
                    }
                }
                HorizontalDivider(color = Blush)
            }
        }) { padding ->
            Box(Modifier.fillMaxSize().padding(padding), contentAlignment = Alignment.TopCenter) {
                Column(Modifier.widthIn(max = 840.dp).fillMaxSize()) {
                    if (state.error != null || state.notice != null) {
                        Surface(color = if (state.error != null) Blush else Blue, modifier = Modifier.fillMaxWidth()) {
                            Column(Modifier.padding(horizontal = 18.dp, vertical = 10.dp)) {
                                Text(state.error?.let { Copy.error(it, state.language) } ?: t(state.notice!!), style = MaterialTheme.typography.bodyMedium)
                                TextButton(onClick = vm::clearMessage) { Text(t("dismiss")) }
                            }
                        }
                    }
                    // Retain form composition while moving to result/history so source selections survive.
                    Box(Modifier.weight(1f)) {
                        CreateScreen(state, vm, t, visible = state.tab == 0)
                        when (state.tab) {
                            1 -> ResultScreen(state, vm, t)
                            2 -> HistoryScreen(state, vm, t)
                            3 -> SettingsScreen(state, vm, t)
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun CreateScreen(state: ScreenState, vm: StudioViewModel, t: (String) -> String, visible: Boolean) {
    var photo by rememberSaveable { mutableStateOf(false) }
    var description by rememberSaveable { mutableStateOf("") }
    var photos by rememberSaveable { mutableStateOf(arrayListOf<String>()) }
    var reference by rememberSaveable { mutableStateOf<String?>(null) }
    var character by rememberSaveable { mutableStateOf("own") }
    var background by rememberSaveable { mutableStateOf("scene") }
    var emotion by rememberSaveable { mutableStateOf("scene") }
    var fullCreature by rememberSaveable { mutableStateOf(false) }
    var strength by rememberSaveable { mutableFloatStateOf(50f) }
    var ratio by rememberSaveable { mutableStateOf("1:1") }
    var resolution by rememberSaveable { mutableStateOf("2K") }
    var details by rememberSaveable { mutableStateOf(false) }
    val context = LocalContext.current
    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenMultipleDocuments()) { uris ->
        uris.forEach { runCatching { context.contentResolver.takePersistableUriPermission(it, Intent.FLAG_GRANT_READ_URI_PERMISSION) } }
        val combined = (photos + uris.map { it.toString() }).distinct()
        if (combined.size > 8) vm.error("too_many_photos")
        photos = ArrayList(combined.take(8))
    }
    val referencePicker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        if (uri != null) {
            runCatching { context.contentResolver.takePersistableUriPermission(uri, Intent.FLAG_GRANT_READ_URI_PERMISSION) }
            reference = uri.toString()
        }
    }
    if (!visible) return
    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(20.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Column(Modifier.weight(1f)) {
                Text(t("headline"), fontFamily = Kawaii, fontSize = 30.sp, lineHeight = 37.sp)
                Text(t("intro"), Modifier.padding(top = 8.dp), style = MaterialTheme.typography.bodyMedium)
            }
            AssetImage("trio-fullbody.png", null, Modifier.width(110.dp).height(100.dp).padding(start = 12.dp))
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            FilterChip(!photo, { photo = false }, label = { Text(t("text_mode")) }, modifier = Modifier.testTag("text_mode"))
            FilterChip(photo, { photo = true }, label = { Text(t("photo_mode")) }, modifier = Modifier.testTag("photo_mode"))
        }
        if (photo) {
            Text(t("photos"), style = MaterialTheme.typography.titleMedium)
            Text(t("photo_help"), style = MaterialTheme.typography.bodySmall)
            if (photos.isNotEmpty()) Row(Modifier.horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                photos.forEachIndexed { index, uri ->
                    Column(horizontalAlignment = Alignment.CenterHorizontally) {
                        LocalImage(uri, "${t("photos")} ${index + 1}", Modifier.size(88.dp).clip(RoundedCornerShape(12.dp)), crop = true, failureText = t("invalid_image"))
                        TextButton(onClick = { photos = ArrayList(photos.filter { it != uri }) }) { Text("${t("remove")} ${index + 1}", fontSize = 12.sp) }
                    }
                }
            }
            OutlinedButton(onClick = { picker.launch(arrayOf("image/*")) }, enabled = photos.size < 8, modifier = Modifier.fillMaxWidth().testTag("add_photos")) {
                Text("${t("add_photos")} (${photos.size}/8)")
            }
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                FilterChip(!fullCreature, { fullCreature = false }, label = { Text(t("classic"), fontSize = 12.sp) })
                FilterChip(fullCreature, { fullCreature = true }, label = { Text(t("full_creature"), fontSize = 12.sp) })
            }
            if (fullCreature) Text(t("creature_help"), style = MaterialTheme.typography.bodySmall)
        }
        OutlinedTextField(description, { description = it.take(1400) }, label = { Text(t("description")) },
            placeholder = { Text(t("placeholder")) }, modifier = Modifier.fillMaxWidth().testTag("description"), minLines = 3, maxLines = 6,
            shape = RoundedCornerShape(16.dp), supportingText = { Text("${description.length}/1400") })
        Text(t("ideas"), style = MaterialTheme.typography.labelLarge)
        Row(Modifier.horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            listOf("picnic", "bakery", "stars").forEach { idea ->
                SuggestionChip(onClick = { description = t("prompt_$idea") }, label = { Text(t("idea_$idea")) }, modifier = Modifier.testTag("idea_$idea"))
            }
        }
        Text(t("character"), style = MaterialTheme.typography.titleMedium)
        Row(Modifier.horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            listOf("own", "chiikawa", "hachiware", "usagi", "trio").forEach { key ->
                Surface(onClick = { character = key }, color = if (character == key) Blush else Paper,
                    shape = RoundedCornerShape(16.dp), border = BorderStroke(if (character == key) 2.dp else 1.dp, if (character == key) Pink else Border)) {
                    Column(Modifier.width(92.dp).padding(8.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                        AssetImage(when (key) { "own", "trio" -> "trio-fullbody.png"; else -> "$key.png" }, null, Modifier.size(52.dp))
                        Text(t(key), Modifier.padding(top = 4.dp), fontSize = 11.sp, maxLines = 1)
                    }
                }
            }
        }
        TextButton(onClick = { details = !details }, modifier = Modifier.testTag("details")) { Text(t("options") + if (details) " −" else " +") }
        if (details) {
            Choice(t("ratio"), ratio, listOf("1:1", "3:4", "4:3", "9:16", "16:9"), { it }, { ratio = it })
            Choice(t("resolution"), resolution, listOf("1K", "2K", "4K"), { it }, { resolution = it })
            Choice(t("background"), background, listOf("scene", "white", "meadow", "room", "sticker"), t, { background = it })
            Choice(t("emotion"), emotion, listOf("scene", "happy", "shy", "tears", "surprised"), t, { emotion = it })
            if (photo) {
                Text("${t("strength")}: ${strength.toInt()}")
                Slider(strength, { strength = it }, valueRange = 0f..100f, steps = 19)
            }
            Text(t("reference"), style = MaterialTheme.typography.titleMedium)
            Text(t("reference_help"), style = MaterialTheme.typography.bodySmall)
            if (reference != null) Row(verticalAlignment = Alignment.CenterVertically) {
                LocalImage(reference!!, t("reference"), Modifier.size(70.dp).clip(RoundedCornerShape(12.dp)), crop = true, failureText = t("invalid_image"))
                TextButton(onClick = { reference = null }) { Text(t("remove")) }
            }
            OutlinedButton(onClick = { referencePicker.launch(arrayOf("image/*")) }) { Text(t("add_reference")) }
        }
        Button(onClick = {
            vm.generate(GenerationRequest(description, photo, photos.toList(), reference, character, background, emotion,
                strength.toInt(), fullCreature, ratio, resolution, state.creditMode))
        }, enabled = !state.busy, modifier = Modifier.fillMaxWidth().heightIn(min = 52.dp).testTag("generate"), shape = RoundedCornerShape(16.dp)) {
            Text(if (state.busy) t(state.phase) else t("generate"), fontSize = 16.sp)
        }
        Text(t("preview_note"), style = MaterialTheme.typography.bodySmall)
        Text("Nano Banana 2.1 / Elyum MCP", style = MaterialTheme.typography.labelMedium)
        Spacer(Modifier.height(12.dp))
    }
}

@Composable
private fun ResultScreen(state: ScreenState, vm: StudioViewModel, t: (String) -> String) {
    val job = state.selected
    var confirm by remember { mutableStateOf<StudioJob?>(null) }
    val context = LocalContext.current
    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(20.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
        Text(t("result"), fontFamily = Kawaii, fontSize = 30.sp)
        if (job == null) {
            AssetImage("trio-fullbody.png", null, Modifier.fillMaxWidth().height(190.dp).padding(24.dp))
            Text(t("empty_result"), style = MaterialTheme.typography.titleLarge)
            Text(t("empty_result_help"), style = MaterialTheme.typography.bodyMedium)
            OutlinedButton(onClick = { vm.tab(0) }) { Text(t("create")) }
        } else {
            Text(t(job.status), style = MaterialTheme.typography.titleLarge)
            if (job.imagePath != null) Surface(color = Paper, border = BorderStroke(1.dp, Border), shape = RoundedCornerShape(20.dp)) {
                LocalImage(job.imagePath, t(job.status), Modifier.fillMaxWidth().heightIn(min = 220.dp, max = 650.dp).aspectRatio(1f), crop = false, failureText = t("invalid_image"))
            }
            else if (!state.busy) Text(t("empty_result_help"), style = MaterialTheme.typography.bodyMedium)
            if (state.busy) {
                LinearProgressIndicator(Modifier.fillMaxWidth(), color = Pink, trackColor = Blush)
                Text(t(state.phase), style = MaterialTheme.typography.bodyMedium)
                if (state.phase !in setOf("saving", "submitting", "uploading", "preparing", "checking_account")) {
                    TextButton(onClick = vm::stopWaiting) { Text(t("stop_waiting")) }
                }
            }
            if (job.status == "preview") {
                Text(t("preview_note"), style = MaterialTheme.typography.bodyMedium)
                if (job.cost != null) {
                    Button(onClick = { confirm = job }, enabled = !state.busy && job.imagePath != null,
                        modifier = Modifier.fillMaxWidth().heightIn(min = 52.dp).testTag("keep"), shape = RoundedCornerShape(16.dp)) {
                        Text("${t("keep")} — ${formatCredits(job.cost)} ${t("credits")}")
                    }
                } else Text(t("price_unknown"))
            }
            if (job.imagePath != null) {
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    OutlinedButton(onClick = { vm.export(job) }, enabled = !state.busy, modifier = Modifier.weight(1f)) { Text(t("download"), fontSize = 12.sp) }
                    OutlinedButton(onClick = {
                        runCatching {
                            val file = vm.repository.safeImage(job)
                            val uri = FileProvider.getUriForFile(context, "${context.packageName}.files", file)
                            context.startActivity(Intent.createChooser(Intent(Intent.ACTION_SEND).apply {
                                type = "image/png"; putExtra(Intent.EXTRA_STREAM, uri); addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                            }, t("share")))
                        }.onFailure { vm.error("storage") }
                    }, enabled = !state.busy, modifier = Modifier.weight(1f)) { Text(t("share"), fontSize = 12.sp) }
                }
            }
            OutlinedButton(onClick = { vm.refresh(job.id) }, enabled = !state.busy, modifier = Modifier.fillMaxWidth()) { Text(t("check")) }
            if (job.error != null) Text(Copy.error(job.error, state.language), color = Danger)
            if (job.description.isNotBlank()) Text(job.description, style = MaterialTheme.typography.bodyMedium)
            Text(job.id, style = MaterialTheme.typography.bodySmall)
        }
    }
    confirm?.let { snapshot ->
        AlertDialog(onDismissRequest = { confirm = null }, title = { Text(t("confirm_title")) },
            text = { Text(t("confirm_body").replace("{cost}", formatCredits(snapshot.cost!!))) },
            confirmButton = { Button(onClick = { confirm = null; vm.keep(snapshot) }, enabled = !state.busy) { Text(t("confirm")) } },
            dismissButton = { TextButton(onClick = { confirm = null }) { Text(t("cancel")) } })
    }
}

@Composable
private fun HistoryScreen(state: ScreenState, vm: StudioViewModel, t: (String) -> String) {
    var id by rememberSaveable { mutableStateOf("") }
    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(20.dp), verticalArrangement = Arrangement.spacedBy(14.dp)) {
        Text(t("history"), fontFamily = Kawaii, fontSize = 30.sp)
        Text(t("history_help"), style = MaterialTheme.typography.bodyMedium)
        if (state.jobs.isEmpty()) Text(t("empty_history"), Modifier.padding(vertical = 24.dp))
        state.jobs.forEach { job ->
            Surface(onClick = { vm.select(job) }, color = Paper, border = BorderStroke(1.dp, Border), shape = RoundedCornerShape(16.dp)) {
                Row(Modifier.fillMaxWidth().padding(12.dp), horizontalArrangement = Arrangement.spacedBy(12.dp), verticalAlignment = Alignment.CenterVertically) {
                    if (job.imagePath != null) LocalImage(job.imagePath, null, Modifier.size(74.dp).clip(RoundedCornerShape(10.dp)), crop = true, failureText = t("invalid_image"))
                    else AssetImage("chiikawa.png", null, Modifier.size(74.dp))
                    Column(Modifier.weight(1f)) {
                        Text(t(job.status), fontWeight = FontWeight.Medium)
                        Text(job.description.ifBlank { "Nano Banana 2.1" }, maxLines = 2, overflow = TextOverflow.Ellipsis, style = MaterialTheme.typography.bodySmall)
                        Text(job.createdAt.take(16).replace('T', ' '), style = MaterialTheme.typography.labelSmall)
                    }
                }
            }
        }
        HorizontalDivider(Modifier.padding(vertical = 12.dp), color = Border)
        Text(t("restore"), style = MaterialTheme.typography.titleMedium)
        OutlinedTextField(id, { id = it.trim().take(122) }, label = { Text(t("job_id")) }, placeholder = { Text("g_…") }, singleLine = true, modifier = Modifier.fillMaxWidth().testTag("job_id"))
        OutlinedButton(onClick = { vm.refresh(id) }, enabled = !state.busy && id.isNotBlank(), modifier = Modifier.fillMaxWidth()) { Text(t("check")) }
    }
}

@Composable
private fun SettingsScreen(state: ScreenState, vm: StudioViewModel, t: (String) -> String) {
    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(20.dp), verticalArrangement = Arrangement.spacedBy(18.dp)) {
        Text(t("settings"), fontFamily = Kawaii, fontSize = 30.sp)
        Text(t("connection"), style = MaterialTheme.typography.titleMedium)
        Text("Elyum MCP / Nano Banana 2.1", style = MaterialTheme.typography.bodyMedium)
        OutlinedButton(onClick = vm::account, enabled = !state.busy, modifier = Modifier.testTag("account")) { Text(t("account")) }
        if (state.account != null) Text(if (state.account == "connected") t("connected") else "${t("balance")}: ${state.account} ${t("credits")}")
        if (state.busy) { LinearProgressIndicator(Modifier.fillMaxWidth()); Text(t(state.phase)) }
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(t("credit_mode"), Modifier.weight(1f))
            Switch(state.creditMode, vm::creditMode)
        }
        Text(t("credit_help"), style = MaterialTheme.typography.bodySmall)
        HorizontalDivider(color = Border)
        Text(t("about"), style = MaterialTheme.typography.titleMedium)
        Text(t("about_body"), style = MaterialTheme.typography.bodyMedium)
        Text(t("about_art"), style = MaterialTheme.typography.bodySmall)
        Text("${t("app")} ${BuildConfig.VERSION_NAME}", style = MaterialTheme.typography.bodySmall)
    }
}

@Composable
private fun Choice(label: String, value: String, options: List<String>, render: (String) -> String, selected: (String) -> Unit) {
    var open by remember { mutableStateOf(false) }
    Column {
        Text(label, style = MaterialTheme.typography.labelLarge)
        Box {
            OutlinedButton(onClick = { open = true }, modifier = Modifier.fillMaxWidth(), shape = RoundedCornerShape(12.dp)) { Text(render(value)) }
            DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
                options.forEach { key -> DropdownMenuItem(text = { Text(render(key)) }, onClick = { selected(key); open = false }) }
            }
        }
    }
}

@Composable
private fun AssetImage(name: String, description: String?, modifier: Modifier) {
    val context = LocalContext.current
    val bitmap = remember(name) { runCatching { context.assets.open(name).use { BitmapFactory.decodeStream(it) } }.getOrNull() }
    bitmap?.let { Image(it.asImageBitmap(), description, modifier, contentScale = ContentScale.Fit) }
}

@Composable
private fun LocalImage(path: String, description: String?, modifier: Modifier, crop: Boolean, failureText: String) {
    val context = LocalContext.current
    val decoded by produceState<Pair<Bitmap?, Boolean>>(null to false, path) {
        val bitmap = withContext(Dispatchers.IO) {
            runCatching {
                val source = if (path.startsWith("content:")) ImageDecoder.createSource(context.contentResolver, Uri.parse(path)) else ImageDecoder.createSource(File(path))
                ImageDecoder.decodeBitmap(source) { decoder, info, _ ->
                    decoder.allocator = ImageDecoder.ALLOCATOR_SOFTWARE
                    val maxSize = if (crop) 256 else 1600
                    val scale = minOf(1.0, maxSize.toDouble() / max(info.size.width, info.size.height))
                    decoder.setTargetSize(max(1, (info.size.width * scale).toInt()), max(1, (info.size.height * scale).toInt()))
                }
            }.getOrNull()
        }
        value = bitmap to true
    }
    if (decoded.first != null) Image(decoded.first!!.asImageBitmap(), description, modifier, contentScale = if (crop) ContentScale.Crop else ContentScale.Fit)
    else Box(modifier.background(Blue), contentAlignment = Alignment.Center) {
        if (decoded.second) Text(failureText, Modifier.padding(8.dp), fontSize = 11.sp)
        else CircularProgressIndicator(Modifier.size(24.dp))
    }
}
