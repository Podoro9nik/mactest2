package app.chiikawizator

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import org.json.JSONObject
import java.util.Locale

data class ScreenState(
    val language: String = "en", val tab: Int = 0, val jobs: List<StudioJob> = emptyList(),
    val selected: StudioJob? = null, val busy: Boolean = false, val phase: String = "",
    val error: String? = null, val notice: String? = null, val creditMode: Boolean = false,
    val account: String? = null
)

class StudioViewModel(application: Application) : AndroidViewModel(application) {
    val repository = StudioRepository(application)
    private val preferences = application.getSharedPreferences("preferences", 0)
    private val mutable = MutableStateFlow(ScreenState(
        language = preferences.getString("language", null) ?: Locale.getDefault().language.takeIf { it in setOf("ru", "en", "ja") } ?: "en",
        creditMode = preferences.getBoolean("creditMode", false)))
    val state = mutable.asStateFlow()
    private var operation: Job? = null
    init { viewModelScope.launch { updateJobs() } }

    fun language(language: String) {
        if (language !in setOf("ru", "en", "ja")) return
        preferences.edit().putString("language", language).apply()
        mutable.update { it.copy(language = language) }
    }
    fun creditMode(value: Boolean) {
        preferences.edit().putBoolean("creditMode", value).apply()
        mutable.update { it.copy(creditMode = value) }
    }
    fun tab(tab: Int) { mutable.update { it.copy(tab = tab) } }
    fun clearMessage() { mutable.update { it.copy(error = null, notice = null) } }
    fun error(code: String) { mutable.update { it.copy(error = code) } }
    fun select(job: StudioJob) { mutable.update { it.copy(selected = job, tab = 1, error = null, notice = null) } }

    private suspend fun updateJobs() {
        val jobs = repository.listJobs()
        mutable.update { state -> state.copy(jobs = jobs, selected = jobs.find { it.id == state.selected?.id } ?: state.selected) }
    }
    private fun run(phase: String, block: suspend () -> Unit) {
        if (mutable.value.busy) return
        mutable.update { it.copy(busy = true, phase = phase, error = null, notice = null) }
        operation = viewModelScope.launch {
            try { block() }
            catch (e: CancellationException) { throw e }
            catch (e: Exception) { mutable.update { it.copy(error = (e as? StudioException)?.code ?: "provider_error") } }
            finally {
                kotlinx.coroutines.withContext(kotlinx.coroutines.NonCancellable) { updateJobs() }
                mutable.update { it.copy(busy = false, phase = "") }
            }
        }
    }
    fun generate(request: GenerationRequest) = run("preparing") {
        val job = repository.generate(request) { phase -> mutable.update { it.copy(phase = phase) } }
        mutable.update { it.copy(selected = job, tab = 1, phase = "running") }
        updateJobs()
        poll(job.id)
    }
    private suspend fun poll(id: String) {
        val deadline = System.currentTimeMillis() + 10 * 60 * 1000
        while (System.currentTimeMillis() < deadline) {
            val job = repository.refresh(id)
            mutable.update { it.copy(selected = if (it.selected?.id == id) job else it.selected, phase = job.status) }
            updateJobs()
            if (job.status in setOf("preview", "completed", "failed")) return
            delay(1200)
        }
        throw StudioException("timeout")
    }
    fun refresh(id: String) = run("checking") {
        if (!isJobId(id)) throw StudioException("invalid_job")
        val job = repository.refresh(id)
        mutable.update { it.copy(selected = job, tab = 1) }
        if (job.status in setOf("queued", "running")) poll(job.id)
    }
    fun keep(confirmed: StudioJob) = run("saving") {
        val job = repository.keep(confirmed)
        mutable.update { it.copy(selected = job, tab = 1) }
    }
    fun stopWaiting() {
        operation?.cancel()
        mutable.update { it.copy(notice = "stopped") }
    }
    fun export(job: StudioJob) = run("saving") {
        repository.export(job)
        mutable.update { it.copy(notice = "saved") }
    }
    fun account() = run("checking_account") {
        val value = repository.account()
        val balance = number(value, "credits") ?: number(value, "balance") ?: value.optJSONObject("balance")?.let { number(it, "credits") }
        mutable.update { it.copy(account = balance?.let { value -> "${formatCredits(value)}" } ?: "connected") }
    }
}

fun formatCredits(value: Double): String = if (value % 1.0 == 0.0) value.toLong().toString() else String.format(Locale.ROOT, "%.2f", value)
