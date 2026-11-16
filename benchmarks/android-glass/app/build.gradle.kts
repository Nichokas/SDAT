plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
}

android {
    namespace = "com.nichokas.sdat.bench"
    compileSdk = 37
    defaultConfig {
        applicationId = "com.nichokas.sdat.bench"
        minSdk = 33
        targetSdk = 37
        versionCode = 1
        versionName = "1"
        buildConfigField("String", "GLASMIO", "\"${libs.versions.glasmio.get()}\"")
    }
    buildTypes {
        // Measured as a user runs SDAT: R8 and no debuggable runtime. The debug key only makes the APK installable.
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"))
            signingConfig = signingConfigs.getByName("debug")
        }
    }
    buildFeatures {
        compose = true
        buildConfig = true
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_21
        targetCompatibility = JavaVersion.VERSION_21
    }
}

androidComponents {
    onVariants { variant ->
        // SDAT's own PlaneGeometry, so the Discover scenes lay out the plane exactly as the app does.
        variant.sources.kotlin?.addStaticSourceDirectory(rootDir.resolve("../../sdat-android/app/src/shared/kotlin").canonicalPath)
    }
}

dependencies {
    implementation(platform(libs.compose.bom))
    implementation(libs.compose.material3)
    implementation(libs.activity.compose)
    implementation(libs.glasmio.scaffold)
    implementation(libs.coil.compose)
    testImplementation(libs.junit)
}
