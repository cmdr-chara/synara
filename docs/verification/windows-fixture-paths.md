# Native Windows registry fixture path corrections

The backend acceptance matrix exposed two independent fixture assumptions. These
are test portability corrections, not a relaxation of workspace containment.

## Evidence

Candidate `177d37315892d1bc13019579b3d95e9840effcd1` failed the registry/controller
journey on Windows when comparing the fixture's current-directory spelling to a
canonical installation path. Candidate `7b6cd3a6ec3f9a69ff4220b7bf8cf5becef31291`
compares absolute canonical filesystem identities instead and separately asserts
that the launch directory is not the project directory. That assertion passed.

Windows backend run `35373261139`, job `105692214671`, then reached the next check:
reading `scope-proof.txt` returned `callback denied` instead of `project scope`.
All 53 ACP unit tests passed in the same job. The fixture formed callback paths
by concatenating a forward slash onto the session's directory. Canonical Windows
paths can use a verbatim prefix, where that separator construction is invalid.

The fixture now uses `Path::join` for both read-scope and create/read callbacks.
A native-path regression checks that the constructed file's parent and filename
remain exactly the intended components, including a verbatim Windows root and
spaces/Unicode. The full integration journey still proves project read scope,
installation launch scope, installation in-use protection and durable history.
Production filesystem validation and its symlink/traversal guards are unchanged.

State at publication: correction awaiting native Windows CI. Linux and macOS
backend formatting, compilation, strict lints and tests passed for `7b6cd3a`.
Native macOS/Windows application compilation also passed, which is not evidence
of interactive desktop acceptance on those systems.
