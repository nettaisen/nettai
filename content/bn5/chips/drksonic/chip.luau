--!strict
-- DrkSonic (BN5 chip 0x0c6): a dark chip: a cut-in chip that sets an instrument in front of the user; when it plays,
-- each of the other side's combatants is paralyzed (status 0x11: 120 ticks) where it stands (objects/instrument:
-- the shared @common/instruments with BN5's look; its row 4 of 0x080D5158: it plays 0x3C × 2 ticks, its effect every 120
-- (so once a playing), in its palette 0xC, and pauses 0x78; its tune 0x14A; its effect 0x080D5502). Its use costs a
-- step bug (rules/light-dark).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local bn5_instrument = require("../../objects/instrument/instrument")
local collision = require("../../rules/collision")
local instrument = require("@common/instruments/instrument")
local regions = require("@common/regions")
local statuses = require("../../rules/status")

-- The sixth element: the hit's damage is 0.
local ELEMENT_NONE = 5

-- 0x080D5502: a hit on each of the other side's combatants' panels that paralyzes it.
local function stun(me: Object)
    for _, actor in battle.alive_actors(bit32.bxor(me.alliance, 1)) do
        battle.hitbox(me, {
            panel_x = actor.panel_x,
            panel_y = actor.panel_y,
            element = ELEMENT_NONE,
            z = 0,
            region = regions.single,
            -- None.
            hit_effect = nil,
            target = collision.hits_navis,
            self_type = collision.breaking,
            damage = 0,
            stamina = 0,
            hit_mod = 0,
            status = statuses.paralyze_120,
        })
    end
end

return define.chip {
    id = "bn5:drksonic",
    codes = { "E", "*" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 4,
    mb = 99,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "dark" },
    slot_in_limit = 1,
    -- BN5's row 4; its identity the actor record 0xE1 (0x08014C94: version 2, AI index 0x21).
    dimming = bn5_instrument.hook(instrument.instrument {
        identity = define.identity {
            id = "bn5:drksonic",
            class = "field_object",
            version = 2,
            ai_index = 0x21,
            look = { sprite = bn5_instrument.sprite, anim = 0, palette = 0xC, shadow = true },
        },
        palette = 0xC,
        play_ticks = 0x3C * 2,
        pause_ticks = 0x78,
        effect_period = 0x78,
        tune = asset.sound("bn5:dark-sonic"),
        effect = stun,
    }),
}
