load("@rules_python//python:defs.bzl", "py_library")

py_library(
    name = "client",
    srcs = [
        "aeron_cache/client.py",
        "aeron_cache/embedded_cache.py",
    ],
    visibility = ["//visibility:public"],
    deps = [
        # In a real bazel setup, we would need to provide these dependencies via pip_parse/requirement
        # For now, assuming they are available in the python environment
    ],
)
