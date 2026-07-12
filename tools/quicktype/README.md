# Quicktype toolchain

This private workspace package pins the quicktype CLI used by Lingua's provider type generator.

`quicktype-core@24.0.2` is temporarily patched with the fix from [quicktype PR #2914](https://github.com/glideapps/quicktype/pull/2914). Remove `patches/quicktype-core@24.0.2.patch` and the `patchedDependencies` entry after upgrading to a quicktype release that contains that fix.
