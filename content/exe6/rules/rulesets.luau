--!strict
-- EXE6's rulesets: what a ruleset may hold back of a build, and the rulesets the game offers ready-made, as data
-- (nettai-match's `ruleset`, for nettai's lobby and build screens; docs/design/rules-in-luau.md, "Rulesets";
-- docs/design/versions.md §2.3). The simulation reads none of it: a ruleset says which builds may play a match, never
-- how its rounds play. A category is a fact of the setup (../init.luau's `setup`) that names definitions, by the
-- fact's name; what it ranges over is the fact's type's. The groups a ruleset offers as quick choices and holds a build
-- to are the versions (./builds' `groups`), each with the chips and forms whose own `version` names it (each version's
-- Giga chips, Crosses and Beast forms) and the Bass-Cross MegaMan card's variant of its name. Names are the locales'
-- ([rulesets], `category.<fact>`, `preset.<id>`).

return {
    -- In the order a ruleset's page lists them.
    categories = {
        { fact = "navi" },
        { fact = "folder" },
        { fact = "navicust_programs" },
        { fact = "patch_cards" },
        { fact = "cybeast" },
        { fact = "crosses" },
    },
    -- The rulesets ready-made, the first a player's and a new lobby's until they pick another. `versions`: whether
    -- each build must be one version's (`one_per_build`: what it holds of the versions' own things all one's) or
    -- `mixed`; `groups`: the versions whose own things it allows (none said: both); `only`: what a category allows,
    -- by the items' names (a category it doesn't name allows everything). Standard is the cartridges' own rules (the
    -- user: "you should have a standard ruleset for each game that enforces version-specific stuff"); Mixed lets a
    -- build mix the versions; then each version alone, and no patch cards.
    presets = {
        { id = "standard", versions = "one_per_build" },
        { id = "mixed", versions = "mixed" },
        { id = "gregar", versions = "one_per_build", groups = { "gregar" } },
        { id = "falzar", versions = "one_per_build", groups = { "falzar" } },
        { id = "no_patch_cards", versions = "one_per_build", only = { patch_cards = {} } },
    },
}
