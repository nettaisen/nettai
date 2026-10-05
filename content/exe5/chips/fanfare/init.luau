--!strict
-- Fanfare (EXE5 chip 0x064): a cut-in chip that sets an instrument in front of the user; while it plays, the user's
-- side can't be hurt (lib/instruments/instrument: the shared @exelib/instruments with EXE5's look; its row 0 of 0x080D5158, its
-- effect 0x080D542E: EXE6's but for Beast Over, which EXE5 hasn't).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local exe5_instrument = require("../lib/instruments/instrument")
local instrument = require("@exelib/instruments/instrument")

-- How long its every-third-tick invulnerability lasts.
local INVULNERABLE_TICKS = 4

-- 0x080D542E: every combatant of its side is invulnerable for a few ticks (`object_setInvulnerableTime`).
local function cheer(me: Object)
    for _, actor in battle.alive_actors(me.alliance) do
        actor:set_status_timer("invulnerable", INVULNERABLE_TICKS)
        actor:set_status("invulnerable", true)
    end
end

return define.chip {
    id = "fanfare",
    codes = { "F", "P", "Z", "*" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 1,
    mb = 20,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    slot_in_limit = 3,
    -- EXE5's row 0; its identity the actor record 0xDD (0x08014C94: version 4, AI index 0x21).
    dimming = exe5_instrument.hook(instrument.instrument {
        identity = define.identity {
            id = "fanfare",
            class = "field_object",
            version = 4,
            ai_index = 0x21,
            look = { sprite = exe5_instrument.sprite, anim = 0, palette = 0, shadow = true },
        },
        palette = 0,
        play_ticks = 0x55 * 2,
        pause_ticks = 0x3C,
        tune = asset.sound("fanfare"),
        effect = cheer,
    }),
}
