import javax.inject.Inject
import org.gradle.process.ExecOperations

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
}

android {
    namespace = "com.nichokas.sdat"
    compileSdk = 37
    ndkVersion = "27.1.12297006"
    defaultConfig {
        applicationId = "com.nichokas.sdat"
        minSdk = 33
        targetSdk = 37
        versionCode = 1
        versionName = "0.1.0"
        ndk { abiFilters += listOf("arm64-v8a", "x86_64") }
    }
    buildFeatures { compose = true }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_21
        targetCompatibility = JavaVersion.VERSION_21
    }
}

/** Builds libtonearm_ffi.so and its UniFFI Kotlin bindings; neither is committed. */
abstract class BuildRustCore : DefaultTask() {
    @get:Inject abstract val exec: ExecOperations
    @get:InputFiles @get:PathSensitive(PathSensitivity.RELATIVE) abstract val sources: ConfigurableFileCollection
    @get:Internal abstract val script: RegularFileProperty
    @get:OutputDirectory abstract val jniLibs: DirectoryProperty
    @get:OutputDirectory abstract val bindings: DirectoryProperty

    @TaskAction
    fun run() {
        exec.exec {
            commandLine(script.get().asFile.absolutePath)
            environment("ANDROID_JNI_DIR", jniLibs.get().asFile.absolutePath)
            environment("ANDROID_KOTLIN_DIR", bindings.get().asFile.absolutePath)
        }
    }
}

val repo = rootProject.layout.projectDirectory.dir("..")
val buildRustCore = tasks.register<BuildRustCore>("buildRustCore") {
    script.set(repo.file("scripts/build-rust-android.sh"))
    sources.from(
        repo.file("scripts/build-rust-android.sh"),
        repo.file("Cargo.lock"),
        repo.dir("sdat-core").asFileTree.matching { exclude("**/target/**") },
    )
    jniLibs.set(layout.buildDirectory.dir("generated/rust/jniLibs"))
    bindings.set(layout.buildDirectory.dir("generated/rust/kotlin"))
}

val localAssets = rootProject.file("local-assets")
androidComponents {
    onVariants { variant ->
        variant.sources.jniLibs?.addGeneratedSourceDirectory(buildRustCore, BuildRustCore::jniLibs)
        variant.sources.kotlin?.addGeneratedSourceDirectory(buildRustCore, BuildRustCore::bindings)
        if (localAssets.isDirectory) variant.sources.assets?.addStaticSourceDirectory(localAssets.path)
    }
}

dependencies {
    implementation(platform(libs.compose.bom))
    implementation(libs.compose.material3)
    implementation(libs.activity.compose)
    implementation(libs.glasmio.scaffold)
    implementation(libs.coroutines.android)
    implementation(libs.jna) { artifact { type = "aar" } }
    testImplementation(libs.junit)
}
