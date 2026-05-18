// Synthetic Gradle build script. Exercises the closure-as-DSL
// pattern (`plugins`, `repositories`, `dependencies`, `tasks`),
// command-chain method invocation (`id 'java'`,
// `implementation '…'`), and `register` / configuration DSL.

plugins {
    id 'java'
    id 'application'
    id 'groovy'
}

group = 'com.example'
version = '0.1.0'

repositories {
    mavenCentral()
    gradlePluginPortal()
}

dependencies {
    implementation 'com.google.guava:guava:30.1.1-jre'
    implementation 'org.codehaus.groovy:groovy-all:3.0.10'
    testImplementation 'org.spockframework:spock-core:2.0-groovy-3.0'
    testImplementation 'junit:junit:4.13.2'
}

application {
    mainClass = 'com.example.Main'
}

tasks.withType(JavaCompile) {
    options.encoding = 'UTF-8'
}

tasks.named('test') {
    useJUnitPlatform()
    testLogging {
        events 'passed', 'failed', 'skipped'
    }
}
