--!strict
-- Fanfare (EXE4 chip 0x064): a cut-in chip that sets an instrument in front of the user; while it plays, the user's
-- navi can't be hurt (lib/instruments: the shared @exelib/instruments with EXE4's look; its row 0 of 0x080DC32C, 50
-- HP, its tune 0x12A, its effect 0x080DC5C4).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local instrument = require("@exelib/instruments/instrument")
local instruments = require("../lib/instruments")

-- How long its every-third-tick invulnerability lasts.
local INVULNERABLE_TICKS = 4

-- 0x080DC5C4: its side's navi (BattleState +0x44 by the side) is invulnerable for a few ticks
-- (`object_setInvulnerableTime`, 0x0800B018). (EXE5's and EXE6's make every combatant of the side so.)
local function cheer(me: Object)
    local navi = battle.player(me.alliance)
    if navi then
        navi:set_status_timer("invulnerable", INVULNERABLE_TICKS)
        navi:set_status("invulnerable", true)
    end
end

local chip: Chip = {
    codes = { "E", "G", "N", "*" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 1,
    mb = 20,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = instruments.hook(instrument.instrument {
        palette = 0,
        hp = 50,
        play_ticks = 0x55 * 2,
        pause_ticks = 0x3C,
        tune = asset.sound("fanfare"),
        effect = cheer,
    }),
}

return { fanfare = chip }
