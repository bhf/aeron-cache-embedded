"""Python client soak test — a faithful port of the Java client soak.

Drives a weighted, seeded-random mix of key-value and counter operations through one client transport
(``http`` or ``bidi``) against a running backend, reconciling against an in-memory oracle and sampling
client memory throughout. Correctness drift, a stalled op, or memory that trends up with time are soak
findings.

Run via the ``Soak Python`` workflow (``poetry run python -m soak``).
"""
