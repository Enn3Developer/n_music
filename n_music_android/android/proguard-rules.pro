# Add project specific ProGuard rules here.
# You can control the set of applied configuration files using the
# proguardFiles setting in build.gradle.
#
# For more details, see
#   http://developer.android.com/guide/developing/tools/proguard.html

# If your project uses WebView with JS, uncomment the following
# and specify the fully qualified class name to the JavaScript interface
# class:
#-keepclassmembers class fqcn.of.javascript.interface.for.webview {
#   public *;
#}

# Uncomment this to preserve the line number information for
# debugging stack traces.
#-keepattributes SourceFile,LineNumberTable

# If you keep the line number information, uncomment this to
# hide the original source file name.
#-renamesourcefileattribute SourceFile

# UniFFI's Kotlin bindings call the native core through JNA, which finds their classes and
# methods by name.
-keep class com.sun.jna.** { *; }
-keep class * extends com.sun.jna.** { *; }
-keep class com.enn3developer.n_music.core.** { *; }
# JNA's desktop code refers to AWT, which Android lacks.
-dontwarn java.awt.**

# Shrink, but keep class and method names and line numbers, so crash logs read as they are
# without a mapping file. Renaming would save about 0.4 MB more.
-dontobfuscate
-keepattributes SourceFile,LineNumberTable
