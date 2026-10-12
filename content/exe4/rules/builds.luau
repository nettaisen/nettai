--!strict
-- EXE4's builds: what a tool that makes and edits a player's side as a build reads of the game, as data (nettai-match's
-- `build`, for nettai's build screens). The simulation reads none of it. A definition is named by its id, a fact by
-- its setup field's name (../init.luau's `setup`).

return {
    -- The versions a build is one of, in the order every list shows them: Red Sun, then Blue Moon. EXE4's rules have no
    -- version (it is the save's, and the two play alike), so a version is what its saves can have, which a build's
    -- version sets as it is made: its six souls (0x08020018, the UNITE button's table: Red Sun's souls 1 to 6, Blue
    -- Moon's 7 to 12, by compat's numbers; exe4-compat's `Version::soul_flag`), and its Giga chips (./folder: its
    -- game's five, and the e-Reader's Duo, which either version's save can have).
    versions = {
        {
            id = "redsun",
            set = {
                souls = { "rollsoul", "gutssoul", "windsoul", "searchsoul", "firesoul", "thundersoul" },
                giga_chips = { "bass", "redsun", "holydrem", "blakbarr", "bugcharg", "duo" },
            },
        },
        {
            id = "bluemoon",
            set = {
                souls = { "protosoul", "numbersoul", "metalsoul", "junksoul", "aquasoul", "woodsoul" },
                giga_chips = { "deltaray", "bugcurse", "signlred", "bassanly", "bluemoon", "duo" },
            },
        },
    },
    -- A build's other choices, each of a few presets that set their facts together, the first a new build's. EXE4's
    -- one is MegaMan light or dark, as Tango's EXE4 netplay saves have him (tango-gamesupport-bn4's src/saves/*.raw:
    -- the light_hp_1000 and dark_hp_997 ones, every version and region alike; the base HP at the image's 0x21CA, the
    -- light/dark value at MegaMan's NaviStats +0x36, 0x4E96): 1000 with 1000 HP, or 460 with 997. A build is on a preset
    -- by the fact that tells them apart (`fact`), on the game's line (`from`, the least value that is the preset's): a
    -- value of 500 or more is a light MegaMan's, 499 and under a dark one's (./light_dark's `DARK_AT_MOST`, the tests
    -- at 0x08013306 and 0x08013348). The choice is MegaMan's (`navi`).
    choices = {
        {
            id = "light_dark",
            navi = "megaman",
            fact = "karma",
            presets = {
                { id = "light", from = 500, set = { karma = 1000, hp = 1000 } },
                { id = "dark", from = 0, set = { karma = 460, hp = 997 } },
            },
        },
    },
    -- The build's list whose entries go in slots: the patch cards (the setup's `patch_cards`, each a `card` in a
    -- `slot`). A card belongs in one of the six catalog slots (`own`, the field of its definition that says which:
    -- ./patch_cards/cards' `SLOT_0A` to `SLOT_0F`, 0 to 5), and a build keeps each card in its own (./patch_cards'
    -- `build_check`); `names` are the labels the game shows the slots under, by their numbers.
    slots = {
        field = "patch_cards",
        entry = "card",
        slot = "slot",
        own = "slot",
        names = { "0A", "0B", "0C", "0D", "0E", "0F" },
    },
    -- The facts a build never states: every build plays at the most the game allows, so these are always the rules'
    -- defaults (the most Regular memory, the sun, Double Soul, the rival wins at the 100 RedSun's and BlueMoon's damage
    -- counts).
    at_defaults = { "reg_up", "sun", "double_soul", "rival_wins" },
    -- A new build is its version and each choice's first preset (a light MegaMan), with this.
    new = {
        -- The folder a new save starts with: the first folder of the ROM's folder table, which the new game routine
        -- copies into the save's first folder (Red Sun US's table 0x08017CC8, copied by 0x080190E0 from the new game
        -- routine 0x080049A8, which the title screen's NEW GAME runs; the Japanese ROMs' table is at 0x08017C08). The
        -- four ROMs' tables are the same, so a new save's folder is one for both versions.
        folder = {
            { chip = "cannon", code = "A" },
            { chip = "cannon", code = "A" },
            { chip = "cannon", code = "B" },
            { chip = "cannon", code = "B" },
            { chip = "airshot", code = "A" },
            { chip = "airshot", code = "A" },
            { chip = "airshot", code = "A" },
            { chip = "vulcan1", code = "V" },
            { chip = "vulcan1", code = "V" },
            { chip = "vulcan1", code = "V" },
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
            { chip = "crakout", code = "*" },
            { chip = "crakout", code = "*" },
            { chip = "crakout", code = "*" },
            { chip = "recov10", code = "A" },
            { chip = "recov10", code = "A" },
            { chip = "recov10", code = "L" },
            { chip = "recov10", code = "L" },
            { chip = "areagrab", code = "S" },
            { chip = "atk-10", code = "*" },
            { chip = "atk-10", code = "*" },
        },
    },
}
