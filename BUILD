# Bazel Sample Runner
# Runs all language samples via a single command: bazel run //:run-all

genrule(
    name = "run-all-script",
    srcs = ["run-all-samples.sh"],
    outs = ["run-all.sh"],
    cmd = "cp $(location run-all-samples.sh) $@",
    executable = True,
)

sh_binary(
    name = "run-all",
    srcs = [":run-all-script"],
    data = glob([
        "libraries/**/*",
        "samples/**/*",
    ]),
)
