.DEFAULT_GOAL := all

# one of armv7 aarch64 x86 x86_64
ARCH := aarch64
ANDROID_ARCH := $(ARCH)
ifeq ($(ANDROID_ARCH),aarch64)
  ANDROID_ARCH := arm64-v8a
endif
ifeq ($(ANDROID_ARCH),armv7)
  ANDROID_ARCH := armeabi-v7a
endif

OS="${shell uname}"

# https://stackoverflow.com/questions/18136918/how-to-get-current-relative-directory-of-your-makefile
mkfile_path := "$(shell dirname $(realpath $(firstword $(MAKEFILE_LIST))))"

DEV_ROOT = "$(mkfile_path)/../.."

clobber: env
	./clobber.sh $(DEV_ROOT) $(ARCH)

config: env
	./config.sh $(DEV_ROOT) $(ARCH)

geckoview: env
	./build-geckoview.sh $(DEV_ROOT) $(ARCH)

# These targets do not depend on GeckoView so that you can build only Fenix if
# you are not changing GV code.
fenix-release: env
	./build-fenix.sh $(DEV_ROOT) $(ARCH) Release
fenix-beta: env
	./build-fenix.sh $(DEV_ROOT) $(ARCH) Beta
fenix-nightly: env
	./build-fenix.sh $(DEV_ROOT) $(ARCH) Nightly
fenix-debug: env
	./build-fenix.sh $(DEV_ROOT) $(ARCH) Debug

env:
	test -e android-env.sh || { echo "copy android-env-...-template.sh to android-env.sh and edit appropriatly"; exit 1; }

install-release:
	./install-fenix.sh $(DEV_ROOT) $(ARCH) $(ANDROID_ARCH) release
install-beta:
	./install-fenix.sh $(DEV_ROOT) $(ARCH) $(ANDROID_ARCH) beta
install-nightly:
	./install-fenix.sh $(DEV_ROOT) $(ARCH) $(ANDROID_ARCH) nightly
install-debug:
	./install-fenix.sh $(DEV_ROOT) $(ARCH) $(ANDROID_ARCH) debug

all: env geckoview fenix-nightly install-nightly
all-release: env geckoview fenix-release install-release
all-beta: env geckoview fenix-beta install-beta

jslint:
	./jslint.sh $(DEV_ROOT) $(JS)

clean:
	rm -rf $(BUILD_OUTPUT)
