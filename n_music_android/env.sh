#!/usr/bin/env bash
# Source this before a local Android build. CI supplies ANDROID_HOME and JAVA_HOME.
export ANDROID_HOME="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}}"
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$ANDROID_HOME/ndk/28.2.13676358}"
export ANDROID_NDK="$ANDROID_NDK_HOME"
