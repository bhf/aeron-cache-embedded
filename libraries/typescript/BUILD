load("@aspect_rules_ts//ts:defs.bzl", "ts_project")
load("@aspect_rules_js//js:defs.bzl", "js_library")

ts_project(
    name = "client_ts",
    srcs = [
        "src/index.ts",
        "src/embedded_cache.ts",
    ],
    declaration = True,
    tsconfig = {},
    visibility = ["//visibility:public"],
)

js_library(
    name = "client",
    srcs = [":client_ts"],
    visibility = ["//visibility:public"],
)
