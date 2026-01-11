workspace(name = "aeron_cache_embedded")

load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")

# Rules for Java
http_archive(
    name = "rules_java",
    url = "https://github.com/bazelbuild/rules_java/releases/download/6.4.0/rules_java-6.4.0.tar.gz",
    sha256 = "451bdce684e030f55de5b8b988f58c7041a7732a3fc267c7689a9f5d34559c90",
)
load("@rules_java//java:repositories.bzl", "rules_java_dependencies", "rules_java_toolchains")
rules_java_dependencies()
rules_java_toolchains()

# Rules for Python
http_archive(
    name = "rules_python",
    sha256 = "c68bdc4f98fa2a800d984950e3a991823528b6d85a676b7012574e87019d67b2",
    strip_prefix = "rules_python-0.31.0",
    url = "https://github.com/bazelbuild/rules_python/releases/download/0.31.0/rules_python-0.31.0.tar.gz",
)
load("@rules_python//python:repositories.bzl", "py_repositories")
py_repositories()

# Rules for Rust
http_archive(
    name = "rules_rust",
    sha256 = "2f974eb12e87902d131498e5e670415a776c5b966601b633008892120e5c9287",
    urls = ["https://github.com/bazelbuild/rules_rust/releases/download/0.38.0/rules_rust-v0.38.0.tar.gz"],
)
load("@rules_rust//rust:repositories.bzl", "rules_rust_dependencies", "rust_register_toolchains")
rules_rust_dependencies()
rust_register_toolchains(
    edition = "2021",
)

# Rules for JavaScript/TypeScript (using aspect_rules_js or rules_nodejs)
http_archive(
    name = "aspect_rules_js",
    sha256 = "40e1bf40ad159239401777d11f6760df763f055da3597c2763c332997d623223",
    strip_prefix = "rules_js-1.34.0",
    url = "https://github.com/aspect-build/rules_js/releases/download/v1.34.0/rules_js-v1.34.0.tar.gz",
)
load("@aspect_rules_js//js:repositories.bzl", "rules_js_dependencies")
rules_js_dependencies()

http_archive(
    name = "aspect_rules_ts",
    sha256 = "0d20dffcc85bb0579e0bf80613dc03c8004f98145e6912389d0407a505b263df",
    strip_prefix = "rules_ts-2.1.0",
    url = "https://github.com/aspect-build/rules_ts/releases/download/v2.1.0/rules_ts-v2.1.0.tar.gz",
)
load("@aspect_rules_ts//ts:repositories.bzl", "rules_ts_dependencies")
rules_ts_dependencies()
