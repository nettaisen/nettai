--!strict
-- EXE5's builds: what a tool that makes and edits a player's side as a build reads of the game, as data (nettai-match's
-- `build`, for nettai's build screens). The simulation reads none of it. A definition is named by its id, a fact by
-- its setup field's name (../init.luau's `setup`).

return {
    -- The versions a build is one of, in the order every list shows them: Team ProtoMan, then Team Colonel. EXE5's
    -- rules have no version (its two play alike), so a version is what its saves can have, which a build's version
    -- sets as it is made: its six souls (0x08024BF0, the soul button's table: Team ProtoMan's souls 1 to 6, Team
    -- Colonel's 7 to 12, by compat's numbers; exe5-compat's `Version::soul_flag`), and its Giga chips (./folder: the
    -- six its ROM's chip records give the library flag, and ChaosLrd, which both versions' Japanese games gave out).
    -- A version's `color` (0xRRGGBB) is the color a tool draws its tag in: Team ProtoMan's red, Team Colonel's green
    -- (the user: "team colonel should be green instead of blue").
    -- Its name is the locales' ([builds], `version.<id>`).
    versions = {
        {
            id = "protoman",
            color = 0xFF5D50,
            set = {
                souls = { "protosoul", "gyrosoul", "searchsoul", "napalmsoul", "magnetsoul", "meddysoul" },
                giga_chips = { "bass", "deltaray", "bugcurse", "holydrem", "bighook", "dethphnx", "chaoslrd" },
            },
        },
        {
            id = "colonel",
            color = 0x45E0A0,
            set = {
                souls = { "colonelsoul", "shadowsoul", "numbersoul", "tomahawksoul", "knightsoul", "toadsoul" },
                giga_chips = { "metrknuk", "omegarkt", "bassanly", "crossdiv", "bugcharg", "phoenix", "chaoslrd" },
            },
        },
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
    -- A new build is its version and each choice's first preset (a light MegaMan), with this.
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
