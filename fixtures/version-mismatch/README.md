This fixture tests what happens when there's versioning mismatches between the
bindings and scaffolding code.  The crate has scripts which trigger version
mismatches and run bindings scripts in order to verify the output.

Ideally this wouldn't be a binary, but a trybuild-style test that checks the
output.  However, this is tricky for a variety of reasons so we just have the
hacky scripts.
