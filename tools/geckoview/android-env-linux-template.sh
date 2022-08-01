export JAVA_HOME=/usr/lib/jvm/java-17-openjdk-amd64
export ANDROID_HOME=$HOME/.mozbuild/android-sdk-linux/ # or $HOME/Android/Sdk/ # Or .../android-toolchain/android-sdk-linux if you extract android-toolchain from tor-browser-build
export ANDROID_NDK_HOME=$HOME/.mozbuild/android-ndk-r28b/ # for 140esr
export GRADLE_HOME=$HOME/.mozbuild/gradle-8.14.3 # not included by default, need to download from https://gradle.org/releases/ and put the extracted directory "gradle-8.14.3" into ~/.mozbuild/
export LOCAL_DEV_BUILD=1
export PATH=/FULL/PATH/TO/tor-browser-build/out/clang/clang-16.x.y-arm/bin/:$PATH # prepend our newly built and assembled clang to the path so it gets used to build geckoview
