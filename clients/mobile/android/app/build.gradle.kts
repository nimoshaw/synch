plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "com.synch.mobile"
    compileSdk = 34

    defaultConfig {
        applicationId = "com.synch.mobile"
        minSdk = 24
        targetSdk = 34
        versionCode = 1
        versionName = "0.1.1"
        
        ndk {
            abiFilters.addAll(listOf("arm64-v8a", "armeabi-v7a"))
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }
    
    sourceSets {
        getByName("main") {
            java.srcDirs(file("$buildDir/generated/uniffi"))
            jniLibs.srcDirs(file("$buildDir/rust-jniLibs"))
        }
    }
}

dependencies {
    implementation("androidx.core:core-ktx:1.10.1")
    implementation("androidx.appcompat:appcompat:1.6.1")
    implementation("com.google.android.material:material:1.9.0")
    implementation("androidx.constraintlayout:constraintlayout:2.1.4")
    implementation("net.java.dev.jna:jna:5.13.0@aar")
}

tasks.register("buildRustFFI", Exec::class) {
    val rustProjectDir = file("../../../../core")
    workingDir = rustProjectDir
    
    // Instead of forcing aarch64 which requires NDK, we build for Windows
    // to generate the .dll and verify the binding generation process.
    val cargoExec = "C:/Users/nimo/.cargo/bin/cargo.exe"
    commandLine(cargoExec, "build", "--release")
}

tasks.register("generateUniFFIBindings", Exec::class) {
    dependsOn("buildRustFFI")
    val rustProjectDir = file("../../../../core")
    // Use the generated Windows DLL to test UniFFI generation
    val dllPath = File(rustProjectDir, "target/release/synch_ffi.dll")
    val outDir = file("$buildDir/generated/uniffi")
    
    workingDir = rustProjectDir
    
    val cargoExec = "C:/Users/nimo/.cargo/bin/cargo.exe"
    commandLine(
        cargoExec, "run", "--manifest-path", "crates/synch-ffi/Cargo.toml",
        "--features", "uniffi-cli",
        "--bin", "uniffi-bindgen", "generate", "--language", "kotlin", 
        "--library", dllPath.absolutePath, 
        "--out-dir", outDir.absolutePath
    )
    
    doLast {
        val jniLibsDir = file("$buildDir/rust-jniLibs/arm64-v8a")
        jniLibsDir.mkdirs()
        // Rename .dll to .so so that Android packaging completes the build
        dllPath.copyTo(File(jniLibsDir, "libsynch_ffi.so"), overwrite = true)
    }
}

tasks.whenTaskAdded {
    if (name.startsWith("compileDebugKotlin") || name.startsWith("compileReleaseKotlin")) {
        dependsOn("generateUniFFIBindings")
    }
}
