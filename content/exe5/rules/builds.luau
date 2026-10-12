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
    versions = {
        {
            id = "protoman",
            set = {
                souls = { "protosoul", "gyrosoul", "searchsoul", "napalmsoul", "magnetsoul", "meddysoul" },
                giga_chips = { "bass", "deltaray", "bugcurse", "holydrem", "bighook", "dethphnx", "chaoslrd" },
            },
        },
        {
            id = "colonel",
            set = {
                souls = { "colonelsoul", "shadowsoul", "numbersoul", "tomahawksoul", "knightsoul", "toadsoul" },
                giga_chips = { "metrknuk", "omegarkt", "bassanly", "crossdiv", "bugcharg", "phoenix", "chaoslrd" },
            },
        },
    },
    -- The facts a build never states: every build plays at the most the game allows, so these are always the rules'
    -- defaults (the most Regular memory, the sun, a team navi's last level, every SP navi deleted in no time, the auto
    -- battle data's pattern records as nothing learned, Chaos Unison).
    at_defaults = { "reg_up", "sun", "level", "sp_times", "auto_battle_records", "chaos_unison" },
    new = {
        -- What a new build states besides its version: a light MegaMan at his most, as Tango's EXE5 netplay saves have
        -- him (the light/dark value 1000 at MegaMan's NaviStats +0x44; the base HP 1000 is the rules' default).
        set = { karma = 1000 },
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
