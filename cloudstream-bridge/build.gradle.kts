// cloudstream-bridge/build.gradle.kts
//
// Builds the long-running JVM bridge that Tauri spawns as a subprocess.
//
// Build commands:
//   ./gradlew shadowJar          — builds cloudstream-bridge.jar
//   ./gradlew deployToTauri      — builds + copies jar to src-tauri/resources/
//   ./gradlew build              — runs both automatically

plugins {
    kotlin("jvm")                version "2.0.21"
    kotlin("plugin.serialization") version "2.0.21"
    id("com.github.johnrengelman.shadow") version "8.1.1"
    application
}

group   = "com.cloudstream"
version = "1.0.0"

// Force compilation to target Java 21 (Java 25 breaks older Kotlin plugin parsers)
java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(21))
    }
}

application {
    mainClass.set("com.cloudstream.bridge.MainKt")
}

repositories {
    mavenCentral()
    google()
    maven("https://jitpack.io")
}

dependencies {
    // Kotlin + coroutines
    implementation(kotlin("stdlib"))
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.9.0")

    // JSON  (kotlinx.serialization — fast, no reflection)
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.7.3")

    // HTTP  (OkHttp — same as CloudStream uses in every extension)
    implementation("com.squareup.okhttp3:okhttp:4.12.0")

    // HTML parsing  (jsoup — same as CloudStream uses)
    implementation("org.jsoup:jsoup:1.17.2")

    // Logging
    implementation("org.slf4j:slf4j-simple:2.0.9")
}

// ── Fat JAR config ────────────────────────────────────────────────────────────

tasks.shadowJar {
    archiveBaseName.set("cloudstream-bridge")
    archiveClassifier.set("")
    archiveVersion.set("")
    mergeServiceFiles()
    manifest {
        attributes["Main-Class"] = "com.cloudstream.bridge.MainKt"
    }
}

// ── Auto-deploy to Tauri resources after every build ─────────────────────────

val tauriResourcesDir = file("../src-tauri/resources")

tasks.register<Copy>("deployToTauri") {
    dependsOn(tasks.shadowJar)
    doFirst { tauriResourcesDir.mkdirs() }
    from(tasks.shadowJar.get().archiveFile)
    into(tauriResourcesDir)
    rename { "cloudstream-bridge.jar" }
    doLast {
        println("[deploy] Copied cloudstream-bridge.jar → src-tauri/resources/")
    }
}

tasks.build {
    finalizedBy("deployToTauri")
}
