#!/bin/bash
set -e
DEV_ROOT=$1
ANDROID_ARCH=$2
VARIANT=$3

cd $DEV_ROOT
OBJ_DIR=$(MOZCONFIG=mozconfig-android-$ARCH ./mach environment --format json --verbose | jq -r .topobjdir)
adb install "$OBJ_DIR/gradle/build/mobile/android/fenix/app/outputs/apk/fenix/$VARIANT/app-fenix-$ANDROID_ARCH-nightly-signed.apk"
