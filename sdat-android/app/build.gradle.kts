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
    }
    // One APK per ABI (the only two the Rust core is built for): a phone installs just its own copy of the core.
    splits {
        abi {
            isEnable = true
            reset()
            include("arm64-v8a", "x86_64")
            isUniversalApk = false
        }
    }
    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
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
        repo.file("Cargo.toml"),
        repo.file("Cargo.lock"),
        repo.dir("sdat-core").asFileTree.matching { exclude("**/target/**") },
    )
    jniLibs.set(layout.buildDirectory.dir("generated/rust/jniLibs"))
    bindings.set(layout.buildDirectory.dir("generated/rust/kotlin"))
}

androidComponents {
    onVariants { variant ->
        variant.sources.jniLibs?.addGeneratedSourceDirectory(buildRustCore, BuildRustCore::jniLibs)
        variant.sources.kotlin?.addGeneratedSourceDirectory(buildRustCore, BuildRustCore::bindings)
    }
}

dependencies {
    implementation(platform(libs.compose.bom))
    implementation(libs.compose.material3)
    implementation(libs.activity.compose)
    implementation(libs.glasmio.scaffold)
    implementation(libs.coroutines.android)
    implementation(libs.coil.compose)
    implementation(libs.coil.network)
    implementation(libs.media3.exoplayer)
    implementation(libs.media3.hls)
    implementation(libs.media3.session)
    implementation(libs.jna) { artifact { type = "aar" } }
    testImplementation(libs.junit)
    // android.jar only stubs org.json; the APK uses the platform copy.
    testImplementation(libs.org.json)
}
