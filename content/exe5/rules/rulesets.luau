--!strict
-- EXE5's rulesets: what a ruleset may hold back of a build, and the rulesets the game offers ready-made, as data
-- (nettai-match's `ruleset`, for nettai's lobby and build screens; docs/design/rules-in-luau.md, "Rulesets";
-- docs/design/versions.md §2.3). The simulation reads none of it: a ruleset says which builds may play a match, never
-- how its rounds play. A category is a fact of the setup (../init.luau's `setup`) that names definitions, by the
-- fact's name; what it ranges over is the fact's type's. The groups a ruleset offers as quick choices and holds a build
-- to are the teams (./builds), each with what it states that the other doesn't (its six souls and six Giga chips;
-- ChaosLrd is both's). Names are the locales' ([rulesets], `category.<fact>`, `preset.<id>`).

return {
    -- In the order a ruleset's page lists them.
    categories = {
        { fact = "navi" },
        -- The team navis of a party battle's party (../init.luau's `party`, ./party): the navis it may hold, after the
        -- navi (the user: "party tab should be next to navi tab, in ruleset editor as well"). Under one version per
        -- build, its navis are its team's (the groups: each navi's own `version`), as the build's souls and Giga chips
        -- are; mixed, any of the twelve.
        { fact = "party" },
        { fact = "folder" },
        { fact = "navicust_programs" },
        { fact = "patch_cards" },
        { fact = "souls" },
        -- The auto battle data's places (./auto_battle/block), a page of their own (the user: "you should instead have a
        -- separate abd rules page"): the chips a folder lists, and the Program Advances besides (`classes`), which the
        -- program advance list holds and no folder does. What the folder's category holds back is allowed here.
        { fact = "auto_battle_places", classes = { "program_advance" } },
    },
    -- The rulesets ready-made, the first a player's and a new lobby's until they pick another. `versions`: whether each
    -- build must be one team's (`one_per_build`: what it holds of the teams' own things all one's) or `mixed`; `only`:
    -- what a category allows, by the items' names (a category it doesn't name allows everything). Masters class is the
    -- cartridges' own rules, one team per build with patch cards; Senior class the same without patch cards;
    -- Unrestricted lets a build mix the teams (the user: "no patch card rules is known as "senior class". patch card
    -- rules is known as "masters class". mixed rules is known as "unrestricted". get rid of version only rules").
    presets = {
        { id = "masters", versions = "one_per_build" },
        { id = "senior", versions = "one_per_build", only = { patch_cards = {} } },
        { id = "unrestricted", versions = "mixed" },
    },
}
