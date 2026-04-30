
• The good news is: it already looks set up to be publishable.

  ../lingua/bindings/typescript/README.md already documents npm install @braintrust/lingua, and the package layout is basically ready:

  - @braintrust/lingua-types
  - @braintrust/lingua-wasm
  - @braintrust/lingua

  The main thing is that you need to publish all packages, with the wasm package before the runtime package.

  What to do

  1. Make sure the generated TS is current.

  cd ../lingua
  make generate-types

  2. Build the packages.

  cd ../lingua
  make typescript

  That should build:

  - bindings/typescript-types/dist
  - bindings/lingua-wasm/nodejs and bindings/lingua-wasm/web
  - bindings/typescript/dist

  3. Sanity-check the install surface.

  cd ../lingua/bindings/typescript
  pnpm test:run

  4. Log into the registry you want to use.
     For npm:

  npm login

  You’ll need publish rights for the @braintrust scope.

  5. Publish @braintrust/lingua-types.

  cd ../lingua
  pnpm --filter @braintrust/lingua-types publish --access public

  6. Publish @braintrust/lingua-wasm.

  cd ../lingua
  pnpm --filter @braintrust/lingua-wasm publish --access public

  7. Publish @braintrust/lingua last.

  cd ../lingua
  pnpm --filter @braintrust/lingua publish --access public

  Why this order matters

  - @braintrust/lingua depends on @braintrust/lingua-wasm
  - so the wasm package has to exist first
  - @braintrust/lingua-types is independent, but publishing it first makes the types-only install path available immediately

  About workspace:*
  You likely do not need to manually rewrite it if you publish with pnpm from the workspace. pnpm publish will resolve workspace:* deps to the published version during packing/publishing.

  Likely one-time gotchas

  - If this is the first publish for a scoped public package on npm, you need --access public.
  - If @braintrust is not set up on npm yet, you need the org/scope configured there first.
  - If you changed Rust-side universal types, run make generate-types before publishing.
  - @braintrust/lingua-wasm has a prepublishOnly build script now, but running make typescript first is still the safer release flow.

  What I’d test after publishing
  From a temp dir:

  mkdir /tmp/lingua-smoke && cd /tmp/lingua-smoke
  pnpm init
  pnpm add @braintrust/lingua

  Then verify:

  import type { Message } from "@braintrust/lingua";

  Also test the types-only package:

  mkdir /tmp/lingua-types-smoke && cd /tmp/lingua-types-smoke
  pnpm init
  pnpm add -D @braintrust/lingua-types

  Then verify:

  import type { Message } from "@braintrust/lingua-types";

  If you want, I can next look at the Lingua workspace and tell you the exact commands I’d use for a safe first release, including version bumps and a smoke-test script.
