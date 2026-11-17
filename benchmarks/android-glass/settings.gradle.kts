pluginManagement {
    repositories { google(); mavenCentral(); gradlePluginPortal() }
}
dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories { google(); mavenCentral() }
    // The app's catalog, so the benchmark always measures the Glasmio, Compose and Coil versions SDAT ships.
    versionCatalogs { create("libs") { from(files("../../sdat-android/gradle/libs.versions.toml")) } }
}
rootProject.name = "SDAT glass benchmark"
include(":app")
