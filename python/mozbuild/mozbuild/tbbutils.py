import re
from urllib.request import Request, urlopen


def list_files_http(url):
    try:
        req = Request(url, method="GET")
        with urlopen(req) as response:
            if response.status != 200:
                return []
            html = response.read().decode()
    except Exception:
        return []

    links = []
    for href in re.findall(r'<a href="([^"]+)"', html):
        if href == "../" or href.startswith(("?", "/")):
            continue

        if "tor-expert-bundle-aar" in href:
            href = f"{href.rstrip('/')}/tor-expert-bundle.aar"
        elif "tor-expert-bundle" in href:
            href = f"{href.rstrip('/')}/tor-expert-bundle.tar.gz"

        links.append(href)

    return links


TOR_BROWSER_BUILD_ARTIFACTS = [
    # Tor Browser Build-only artifacts, these artifacts are not common with Firefox.
    "noscript",
    "fonts",
    "tor-expert-bundle",
    "tor-expert-bundle-aar",
    "application-services",
]

# Mapping of artifacts from taskcluster to tor-browser-build.
ARTIFACT_NAME_MAP = {
    "cbindgen": "cbindgen",
    # FIXME (tor-browser-build#41471): nasm is more or less ready to go, but it needs to have the
    # executable in the root of the artifact folder instead of nasm/bin.
    # "nasm": "nasm",
    # FIXME (tor-browser-build#41421): the clang project as is, is not ready to use. It needs
    # to be repackaged with a bunch of things that differ per platform. Fun stuff.
    # "clang": "clang",
    "node": "node",
}


def get_artifact_index(artifact_path, artifact):
    """
    Return a unique identifier for the given artifact based on its path.

    In most cases, artifacts built by tor-browser-build include part of their
    SHA sum or version in the filename, so the file name itself serves as a unique
    identifier. However, some artifacts are stored within subfolders where the file
    name alone is not unique — in those cases, the name of the parent directory
    provides the unique identifier instead.
    """
    if artifact in ["tor-expert-bundle"]:
        return artifact_path.rsplit("/", 2)[-2]

    return artifact_path.rsplit("/", 1)[-1]


def get_artifact_name(original_artifact_name, host):
    # These are not build artifacts, they are pre-built artifacts to be added to the final build,
    # therefore this check can come before the host check.
    if original_artifact_name in TOR_BROWSER_BUILD_ARTIFACTS:
        return original_artifact_name

    if host != "linux64":
        # Tor browser build only has development artifacts for linux64 host systems.
        return None

    return ARTIFACT_NAME_MAP.get(original_artifact_name)


def get_artifact_path(url, artifact, target, prefix="", log=lambda *args, **kwargs: {}):
    if prefix:
        path = prefix
    else:
        path = artifact

    # The `?C=M;O=D` parameters make it so links are ordered by
    # the last modified date. This here to make us get the latest
    # version of file in the case there are multiple and we just
    # grab the first one.
    files = list_files_http(f"{url}/{path}?C=M;O=D")

    if not files:
        log(f"No files found in {url} for {artifact}.")
        return None

    def filter_files(files, keyword):
        return [file for file in files if keyword in file]

    artifact_files = [file for file in files if file.startswith(artifact)]

    if len(artifact_files) == 0:
        log(f"No files found in {url} for {artifact}.")
        return None

    if len(artifact_files) == 1:
        return f"{url}/{path}/{artifact_files[0]}"

    files_per_os = filter_files(artifact_files, target.tor_browser_build_alias)

    # If there are files in the folder, but they don't have the OS in the name
    # it probably means we can get any of them because they can be used to build
    # for any OS. So let's just get the first one.
    #
    # Note: It could be the case that the artifact _is_ OS dependant, but there
    # just are no files for the OS we are looking for. In that case, this will
    # return an incorrect artifact. This should not happen often though and is
    # something we cannot address until artifact names are standardized on tbb.
    if len(files_per_os) == 0:
        return f"{url}/{artifact}/{artifact_files[0]}"

    elif len(files_per_os) == 1:
        return f"{url}/{artifact}/{files_per_os[0]}"

    matches = filter_files(files_per_os, target.cpu)

    return f"{url}/{artifact}/{matches[0]}" if matches else None


TOR_PROJECT_TOOLCHAINS_URL = (
    "https://nightlies.tbb.torproject.org/nightly-builds/toolchains"
)
TOR_PROJECT_BROWSER_PREFIX = "tb"

# Mozilla job name prefix -> Tor Project toolchain target. Order matters: more
# specific prefixes must come before the prefixes they start with.
TOR_PROJECT_TOOLCHAIN_TARGETS = (
    ("android-arm", "android-armeabi-v7a"),
    ("android-aarch64", "android-arm64-v8a"),
    ("android-x86_64", "android-x86_64"),
    ("linux64-aarch64", "linux-aarch64"),
    ("linux64", "linux-x86_64"),
    ("macosx64-aarch64", "macos-aarch64"),
    ("macosx64", "macos-x86_64"),
    ("win32", "windows-i686"),
    ("win64", "windows-x86_64"),
)


def get_artifacts_dir_url(job, version):
    for prefix, target in TOR_PROJECT_TOOLCHAIN_TARGETS:
        if job.startswith(prefix):
            return f"{TOR_PROJECT_TOOLCHAINS_URL}/{TOR_PROJECT_BROWSER_PREFIX}-{version}/{target}"
    raise ValueError(f"No Tor Project toolchain artifacts for {version}, {job}")


def find_artifact_urls(job, version, artifact_job):
    """
    Return the URLs of the artifacts available for ``job`` that
    ``artifact_job`` (a mozbuild.artifacts.ArtifactJob) wants to install.
    """
    url = get_artifacts_dir_url(job, version)
    artifacts = [
        {"name": f"public/build/{name}"} for name in list_files_http(f"{url}/")
    ]
    return [
        f"{url}/{artifact.rsplit('/', 1)[-1]}"
        for artifact in artifact_job.find_candidate_artifacts(artifacts)
    ]
