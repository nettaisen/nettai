--!strict
-- EXE4's top module (docs/design/content-model-v2.md §4.0): it returns the game's root, what a match names, by id, in
-- its sections, and its rules. The loader walks from it: what it reaches is what the game has. Each section is the
-- module that returns it, from the modules that define it. Written by tools/content/index.py (the verification
-- workspace) from the modules that are there: run it again rather than edit this, also to settle a merge conflict in
-- it.

return {
    chips = require("@self/chips"),
    navis = require("@self/navis"),
    forms = require("@self/navis/megaman/forms/base"),
    stages = require("@self/stages"),
    rules = require("@self/rules"),
}
