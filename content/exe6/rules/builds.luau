--!strict
-- EXE6's builds: what a tool that makes and edits a player's side as a build reads of the game, as data (nettai-match's
-- `build`, for nettai's build screens). The simulation reads none of it. A definition is named by its id, a fact by
-- its setup field's name (../init.luau's `setup`).

return {
    -- The versions, as groups (docs/design/versions.md §2.2): what a save of each can own that the other's can't,
    -- in the order every list shows them, Gregar then Falzar. A build is of no version: it states what it has (its
    -- Crosses, of either version, its Cybeast, its folder), and a ruleset says whether it must be one version's. A
    -- group's members are the definitions whose own `version` names it (its Crosses and Beast forms, its Giga chips:
    -- ./folder) and the Bass-Cross MegaMan card's variant of its name (../patch_cards/bass-cross-megaman). A new
    -- build starts from a group: the forms MegaMan's facts offer of its version (its five Crosses, its Cybeast).
    -- A group's `color` (0xRRGGBB) is the color a tool draws it in: Gregar's green, Falzar's red. Its name is the
    -- locales' ([builds], `version.<id>`).
    groups = {
        { id = "gregar", color = 0x45E0A0 },
        { id = "falzar", color = 0xFF5D50 },
    },
    -- The facts a build never states: every build plays at the most the game allows, so these are always the rules'
    -- defaults (the most HP and Regular memory, the sun, a link navi's last level, every SP navi deleted in no time).
    at_defaults = { "hp", "reg_up", "sun", "level", "sp_times" },
    -- A new build is its group's forms, with this. (EXE6's builds have no other choice of presets: its MegaMan is
    -- neither light nor dark.)
    new = {
        -- What a new build states besides its group's forms: the most BugFrags a save holds, which the dark chips'
        -- parts spend (Tango's four EXE6 netplay saves hold 9999, the word at their image's 0x1BE0).
        set = { bug_frags = 9999 },
        -- The folder a new save starts with: the first folder of the ROM's folder table, which the new game routine
        -- copies into the save's first folder (US Falzar's table `byte_80213AC`, copied by `sub_8021AB4` from
        -- `reqBBS_init_8004DF0`, which the title screen's NEW GAME runs; the Japanese ROMs' table is at 0x080217C0).
        -- The four ROMs' tables are the same, so a new save's folder is one for both versions.
        folder = {
            { chip = "cannon", code = "A" },
            { chip = "cannon", code = "A" },
            { chip = "cannon", code = "B" },
            { chip = "cannon", code = "B" },
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
            { chip = "crakshot", code = "*" },
            { chip = "crakshot", code = "*" },
            { chip = "recov10", code = "A" },
            { chip = "recov10", code = "A" },
            { chip = "recov10", code = "L" },
            { chip = "recov10", code = "L" },
            { chip = "areagrab", code = "S" },
            { chip = "atk-10", code = "*" },
            { chip = "atk-10", code = "*" },
            { chip = "whicapsl", code = "*" },
        },
    },
}
