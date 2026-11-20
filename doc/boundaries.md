## Style
* prefer expect over unwrap to document invariant
* rust edition is 2024, mod.rs forbidden

## Tests
* Only feature tests allowed
* Flow for adding feature is to add requirements, write user scenarios for this requirements, then add feature tests for these scenarios
* Every feature test corresponds to one user scenario.
* Name every feature test after its user scenario in snake case, omitting articles. `A user creates a chain` becomes `user_creates_chain`.
