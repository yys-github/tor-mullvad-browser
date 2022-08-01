export JAVA_HOME=/opt/homebrew/opt/openjdk@17/libexec/openjdk.jdk/Contents/Home # for arm64. Or JAVA_HOME=/usr/local/opt/openjdk@17/libexec/openjdk.jdk/Contents/Home for x86_64.
export ANDROID_HOME=$HOME/.mozbuild/android-sdk-macosx
export ANDROID_NDK_HOME=$HOME/.mozbuild/android-ndk-r28b # for ESR140
export GRADLE_HOME=$HOME/.mozbuild/gradle-8.14.3 # not included by default, need to download from https://gradle.org/releases/ and put the extracted directory "gradle-8.14.3" into ~/.mozbuild/
export LOCAL_DEV_BUILD=1
export PATH=$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64/bin:$PATH # prepend mozbuilds NDK to the PATH so it's clang gets used to build geckoview
