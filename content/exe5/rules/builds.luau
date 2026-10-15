--!strict
-- EXE5's builds: what a tool that makes and edits a player's side as a build reads of the game, as data (nettai-match's
-- `build`, for nettai's build screens). The simulation reads none of it. A definition is named by its id, a fact by
-- its setup field's name (../init.luau's `setup`).

return {
    -- The versions, as groups (docs/design/versions.md §2.2): what a save of each team can own that the other's can't,
    -- in the order every list shows them, Team ProtoMan then Team Colonel. A build is of no version: it states what it
    -- has (its souls, of either team, its folder, its cards), and a ruleset says whether it must be one version's. A
    -- group's members are the definitions whose own `version` names it: its six souls (the soul button's table,
    -- 0x08024BF0: Team ProtoMan's souls 1 to 6, Team Colonel's 7 to 12, by compat's numbers; exe5-compat's
    -- `Version::soul_flag`) and its six Giga chips (./folder: those its ROM's chip records give the library flag;
    -- ChaosLrd, which both versions' Japanese games gave out, is neither's), and the Bass-Cross MegaMan card's variant of
    -- its name (../patch_cards/bass-cross-megaman: card 111's effects are each team's own). A new build starts from a
    -- group: the souls of its version. A group's `color` (0xRRGGBB) is the color a tool draws it in: Team ProtoMan's
    -- red, Team Colonel's green (the user: "team colonel should be green instead of blue"). Its name is the locales'
    -- ([builds], `version.<id>`).
    groups = {
        { id = "protoman", color = 0xFF5D50 },
        { id = "colonel", color = 0x45E0A0 },
    },
    -- A build's other choices, each of a few presets that set their facts together, the first a new build's. EXE5's
    -- one is MegaMan light or dark, the two kinds of save Tango's EXE5 netplay saves are (tango-gamesupport-bn5's
    -- src/saves/*.raw, light and dark of each team and region alike; MegaMan's NaviStats at the image's 0x52A8): the
    -- light/dark value (+0x44) 1000 with the base HP (+0x3E) 1000, or 0 with 997. A build is on a preset by the fact
    -- that tells them apart (`fact`), on the game's line (`from`, the least value that is the preset's): a value of 470
    -- or more is a light MegaMan's, under it a dark one's (./light_dark's `LIGHT_CHIPS`: the chips' check, 0x08010118;
    -- the starting mood, 0x0801283A, a dark MegaMan's under it). The choice is MegaMan's alone (`navi`): a team navi's
    -- NaviStats block has a light/dark value of its own, which MegaMan's being dark doesn't touch (the team navis of
    -- Tango's dark saves hold 500 to 608), and its HP is its story's, so a build of another navi keeps the rules'
    -- defaults for both.
    choices = {
        {
            id = "light_dark",
            navi = "megaman",
            fact = "karma",
            presets = {
                { id = "light", from = 470, set = { karma = 1000, hp = 1000 } },
                { id = "dark", from = 0, set = { karma = 0, hp = 997 } },
            },
        },
    },
    -- The facts a build never states: every build plays at the most the game allows, so these are always the rules'
    -- defaults (the most Regular memory, the sun, a team navi's last level, every SP navi deleted in no time, the auto
    -- battle data's pattern records as nothing learned, Chaos Unison).
    at_defaults = { "reg_up", "sun", "level", "sp_times", "auto_battle_records", "chaos_unison" },
    -- A new build is its group's souls and each choice's first preset (a light MegaMan), with this.
    new = {
        -- The folder a new save starts with: the first folder of the ROM's folder table, which the new game routine
        -- copies into the save's first folder (Team ProtoMan US's table 0x0801D818, copied by 0x0801DF20 from the new
        -- game routine 0x08004A88, which the title screen's NEW GAME runs; the Japanese ROMs' tables are at 0x0801D7D4
        -- and, Team Colonel's, 0x0801D7D0). The four ROMs' tables are the same, so a new save's folder is one for both
        -- versions.
        folder = {
            { chip = "cannon", code = "A" },
            { chip = "cannon", code = "A" },
            { chip = "cannon", code = "B" },
            { chip = "cannon", code = "B" },
            { chip = "airshot", code = "*" },
            { chip = "airshot", code = "*" },
            { chip = "airshot", code = "*" },
            { chip = "vulcan1", code = "D" },
            { chip = "vulcan1", code = "D" },
            { chip = "vulcan1", code = "D" },
            { chip = "minibomb", code = "B" },
            { chip = "minibomb", code = "B" },
            { chip = "minibomb", code = "L" },
            { chip = "minibomb", code = "L" },
            { chip = "sword", code = "S" },
            { chip = "sword", code = "S" },
            { chip = "sword", code = "S" },
            { chip = "sword", code = "S" },
            { chip = "wideswrd", code = "S" },
            { chip = "wideswrd", code = "S" },
            { chip = "busterup", code = "*" },
            { chip = "crakout", code = "*" },
            { chip = "crakout", code = "*" },
            { chip = "recov10", code = "A" },
            { chip = "recov10", code = "A" },
            { chip = "recov10", code = "L" },
            { chip = "recov10", code = "L" },
            { chip = "areagrab", code = "S" },
            { chip = "attck-10", code = "*" },
            { chip = "attck-10", code = "*" },
        },
    },
}
