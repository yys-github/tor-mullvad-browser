#!/bin/bash

if [ $# -eq 0 ]; then
    echo "Usage: ./tba-fetch-deps.sh --\$MODE"
    echo "  modes:"
    echo "    --nightly     Downloads the needed assets from the nightlies build server. Use when local version matches nightly build server version."
    echo "    --tbb PATH    Harvest most recently built assets from PATH assuming it points to a tor-browser-build dir. Use when local version does NOT match nightly build server version."
    exit -1
fi

TBB_BUILD_06="https://tb-build-06.torproject.org/~tb-builder/tor-browser-build/out"

if [[ $1 == "--tbb" && -z $2 ]]; then
  echo "--tbb needs path to tor-browser-build dir"
  exit -1
fi
TBB_PATH=$2

cd "$(dirname $(realpath "$0"))/.."

if [ -z "$TOR_BROWSER_BUILD" ]; then
	TOR_BROWSER_BUILD=../../../../tor-browser-build
fi

echo "Fetching tor-expert-bundle.aar..."

if [[ $1 == "--tbb" ]]; then
  tor_expert_bundle_aar="$(ls -1td "$TOR_BROWSER_BUILD/out/tor-expert-bundle-aar/"tor-expert-bundle-aar-* | head -1)"
  cp "$tor_expert_bundle_aar"/* app/
else
  tor_expert_bundle_aar_dirname="$(curl -s $TBB_BUILD_06/tor-expert-bundle-aar/ | sed -nE 's/.*href=\"(tor-expert-bundle-aar-[0-9a-z\.\-]*).*/\1/p')"
  curl -o app/tor-expert-bundle.aar $TBB_BUILD_06/tor-expert-bundle-aar/$tor_expert_bundle_aar_dirname/tor-expert-bundle.aar
fi

if [ -z app/tor_expert_bundle.aar ]; then
	echo "Cannot find Tor Expert Bundle arr artifacts!"
	exit 2
fi
echo ""

echo "Fetching noscript..."

mkdir -p "app/src/main/assets/extensions"

if [[ $1 == "--tbb" ]]; then
  noscript="$(find "$TOR_BROWSER_BUILD/out/browser" -name 'noscript*.xpi' -print | sort | tail -1)"
  cp "$noscript" "app/src/main/assets/extensions/{73a6fe31-595d-460b-a920-fcc0f8843232}.xpi"
else
  noscript_fname="$(curl -s $TBB_BUILD_06/browser/ | sed -nE 's/.*href=\"(noscript-[0-9a-z\.\-]*).*/\1/p')"
  curl -o "app/src/main/assets/extensions/{73a6fe31-595d-460b-a920-fcc0f8843232}.xpi" $TBB_BUILD_06/browser/$noscript_fname
fi
echo ""
