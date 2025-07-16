#!/bin/bash
set -e
DEV_ROOT=$1
ARCH=$2
ANDROID_ARCH=$3
VARIANT=$4

cd $DEV_ROOT
OBJ_DIR=$(MOZCONFIG=mozconfig-android-$ARCH ./mach environment --format json --verbose | jq -r .topobjdir)

if [ $VARIANT == "debug" ]
then
  adb install "$OBJ_DIR/gradle/build/mobile/android/fenix/app/outputs/apk/fenix/$VARIANT/app-fenix-$ANDROID_ARCH-$VARIANT.apk"
else
  adb install "$OBJ_DIR/gradle/build/mobile/android/fenix/app/outputs/apk/fenix/$VARIANT/app-fenix-$ANDROID_ARCH-$VARIANT-signed.apk"
fi
