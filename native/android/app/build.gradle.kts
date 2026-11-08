plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

val embeddedKey = providers.environmentVariable("CHIIKAWA_EMBEDDED_MCP_KEY").orElse("")
val signingPath = providers.environmentVariable("CHIIKAWA_ANDROID_KEYSTORE").orNull
fun quoted(value: String) = "\"" + value.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "") + "\""

android {
    namespace = "app.chiikawizator"
    compileSdk = 36
    testBuildType = providers.gradleProperty("chiikawaTestBuildType").getOrElse("debug")
    defaultConfig {
        applicationId = "app.chiikawizator"
        minSdk = 29
        targetSdk = 36
        versionCode = 30001
        versionName = "3.0.1"
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        buildConfigField("String", "ELYUM_MCP_KEY", quoted(embeddedKey.get()))
    }
    signingConfigs {
        if (signingPath != null) create("distribution") {
            storeFile = file(signingPath)
            storePassword = System.getenv("CHIIKAWA_ANDROID_STORE_PASSWORD")
            keyAlias = System.getenv("CHIIKAWA_ANDROID_KEY_ALIAS") ?: "chiikawizator"
            keyPassword = System.getenv("CHIIKAWA_ANDROID_KEY_PASSWORD") ?: storePassword
        }
    }
    buildTypes {
        release {
            isMinifyEnabled = false
            if (signingPath != null) signingConfig = signingConfigs.getByName("distribution")
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
    }
    sourceSets["main"].assets.srcDir("../../shared/assets")
    buildFeatures { compose = true; buildConfig = true }
    compileOptions { sourceCompatibility = JavaVersion.VERSION_17; targetCompatibility = JavaVersion.VERSION_17 }
    kotlinOptions { jvmTarget = "17" }
    packaging { resources.excludes += "/META-INF/{AL2.0,LGPL2.1}" }
}
dependencies {
    implementation(platform("androidx.compose:compose-bom:2025.06.01"))
    implementation("androidx.activity:activity-compose:1.10.1")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.9.2")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.9.2")
    implementation("androidx.core:core-ktx:1.16.0")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.10.2")
    implementation("com.squareup.okhttp3:okhttp:4.12.0")
    debugImplementation("androidx.compose.ui:ui-tooling")
    debugImplementation("androidx.compose.ui:ui-test-manifest")
    testImplementation("junit:junit:4.13.2")
    testImplementation("org.json:json:20240303")
    testImplementation("com.squareup.okhttp3:mockwebserver:4.12.0")
    androidTestImplementation(platform("androidx.compose:compose-bom:2025.06.01"))
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
    androidTestImplementation("androidx.test:runner:1.6.2")
    androidTestImplementation("androidx.compose.ui:ui-test-junit4")
}
