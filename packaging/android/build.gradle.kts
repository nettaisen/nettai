// nettai's APK, which package.sh builds after cargo has built the native library and the rest it hands over, each
// a Gradle property:
//
//   nettai.out          target/package/android: jniLibs/arm64-v8a/ (the library and libc++_shared.so), assets/ (the
//                       notices, the license), res/ (the launcher icon); this build's own files go in its gradle/
//   nettai.version      the version, and nettai.versionCode its number (0.17.0: 17000)
//   nettai.verifier     rustls-platform-verifier's AAR (its Kotlin, which the certificate checks call through JNI,
//                       and its network security config), pinned by package.sh
//   nettai.keystore     the keystore to sign with, nettai.keystorePassword its password and nettai.keyAlias the key's
//   nettai.debuggable   1: the APK says it is debuggable (`adb shell run-as` reads and writes its files)
//
// The Kotlin is every crate's android/ folder's (the activity, nettai-dialogs' NettaiFiles, nettai-lifecycle's
// NettaiAwake, nettai-gamepad's NettaiGamepads), compiled by AGP's own Kotlin; R8 shrinks it with the libraries' and
// takes out what nothing uses (the Kotlin standard library's most of all), keeping what the native code calls by name
// (proguard-rules.pro).
plugins {
    id("com.android.application") version "9.5.0"
}

fun setting(name: String): String = providers.gradleProperty(name).orNull ?: error("package.sh gives -P$name")

val out = file(setting("nettai.out"))
val crates = rootDir.resolve("../../crates").canonicalFile

layout.buildDirectory.set(out.resolve("gradle"))

android {
    namespace = "net.n1gp.nettai"
    compileSdk = 36

    defaultConfig {
        applicationId = "net.n1gp.nettai"
        // Android 8 (the oldest whose AAudio cpal plays through).
        minSdk = 26
        targetSdk = 34
        versionName = setting("nettai.version")
        versionCode = setting("nettai.versionCode").toInt()
    }

    sourceSets["main"].apply {
        manifest.srcFile("AndroidManifest.xml")
        kotlin.srcDirs(crates.listFiles()!!.map { it.resolve("android") }.filter { it.isDirectory }.sorted())
        jniLibs.srcDir(out.resolve("jniLibs"))
        assets.srcDir(out.resolve("assets"))
        res.srcDir(out.resolve("res"))
    }

    signingConfigs {
        create("nettai") {
            storeFile = file(setting("nettai.keystore"))
            storePassword = setting("nettai.keystorePassword")
            keyAlias = setting("nettai.keyAlias")
            keyPassword = setting("nettai.keystorePassword")
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            // (And the resources nothing uses: most of AppCompat's.)
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
            signingConfig = signingConfigs["nettai"]
            isDebuggable = providers.gradleProperty("nettai.debuggable").orNull == "1"
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    // (The library is stripped already, by package.sh: no NDK needed here.)
    packaging {
        jniLibs {
            keepDebugSymbols += "**/*.so"
        }
    }

    lint {
        checkReleaseBuilds = false
    }
}

dependencies {
    // The activity (android-activity 0.6's C is GameActivity 4.4.0's, exactly) and what it is built on.
    implementation("androidx.games:games-activity:4.4.0")
    implementation("androidx.appcompat:appcompat:1.7.1")
    implementation(files(setting("nettai.verifier")))
}
