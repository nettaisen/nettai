--!strict
-- Fanfare: a cut-in chip that sets a trumpet in front of the user; while
-- it plays, the user's side can't be hurt (lib/instruments).

local instrument = require("../../lib/instruments/instrument")
local instruments = require("../../lib/instruments/controller")

-- How long its every-third-tick invulnerability lasts.
local INVULNERABLE_TICKS = 4

-- `sub_80D435C`: every combatant of its side is invulnerable for a few
-- ticks (`object_setInvulnerableTime`), but in Beast Over.
local function cheer(me: Object)
    for _, actor in battle.alive_actors(me.alliance) do
        if not battle.navi(actor.alliance).beast_over then
            actor:set_status_timer("invulnerable", INVULNERABLE_TICKS)
            actor:set_status("invulnerable", true)
        end
    end
end

return define.chip {
    id = "fanfare",
    name = "Fanfare",
    codes = { "P", "S", "Z", "*" },
    element = "null", family = "summon", class = "standard",
    rarity = 1, mb = 20, damage = 0, hit_param = 0, lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    library = { number = 149, index = 149, sort = 202 },
    slot_in_limit = 3,
    -- `byte_80D4078` row 0.
    dimming = instruments.hook(instrument.instrument {
        identity = 0xDD,
        palette = 0,
        play_ticks = 0x55 * 2,
        pause_ticks = 0xAA,
        tune = asset.sound("fanfare"),
        effect = cheer,
    }),
}
