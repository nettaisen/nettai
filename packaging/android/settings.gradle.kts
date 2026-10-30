// nettai's APK (package.sh runs this build; build.gradle.kts says what it is made of). One project, the app: its
// Kotlin is the crates', its native library cargo's.
pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "nettai"
