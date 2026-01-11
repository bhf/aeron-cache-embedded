load("@rules_rust//rust:defs.bzl", "rust_library")

rust_library(
    name = "client",
    srcs = ["src/lib.rs"],
    edition = "2021",
    deps = [
        # These would theoretically come from crate_universe or similar
        # "//external:reqwest",
        # "//external:serde",
        # "//external:serde_json", 
        # "//external:tungstenite",
        # "//external:url",
    ],
    visibility = ["//visibility:public"],
)
