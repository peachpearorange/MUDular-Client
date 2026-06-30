#!/bin/bash
set -e

# Build the Rust cdylib for Android
# Requires: cargo-ndk, Android NDK (set ANDROID_NDK_HOME)
#   cargo install cargo-ndk
#   rustup target add aarch64-linux-android

cd "$(dirname "$0")/.."

cargo ndk -t arm64-v8a -o android/app/src/main/jniLibs build --lib --release

cd android
gradle assembleDebug

echo ""
echo "APK: android/app/build/outputs/apk/debug/app-debug.apk"
