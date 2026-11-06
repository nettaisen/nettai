--!strict
-- A kind that uses the v2 API once: it spawns a ticker by definition and
-- fills its references, shows an effect and plays a sound by name, and
-- registers its collision with defined collision types and a defined
-- region.

local ticker = require("./ticker")
local sprites = require("../../sprites")

local SHORT = ticker.variant { lifetime = 5 }
local BURST = new.effect { sprite = sprites["test-burst"], anim = 2, steps = 1 }
local WIDE = new.region { panels = { { 1, -1 }, { 1, 0 }, { 1, 1 } } }
local THROWN = new.collision {
    id = "test/thrown",
    flags = { own_attack = true, reaches_float_shoe = true, reaches_submerged = true },
}
local HITS = new.collision {
    id = "test/hits-navis",
    flags = { enemy_object = true, enemy_body = true, enemy_other_body = true, neutral = true },
}
local TICK = asset.sound("test-tick")

local launcher = {}

launcher.kind = new.kind {
    id = "test/launcher",
    pool = "attack",
    state = { launched = schema.bool },
    update = function(me: Object)
        local s = me.state :: { launched: boolean }
        if s.launched then
            return
        end
        s.launched = true
        local o = battle.spawn(ticker.kind, me.pos)
        if o then
            local t = o.state :: any
            t.variant = SHORT
            t.parent = me.kind
        end
        battle.effect(me.pos, BURST)
        battle.play_sound(TICK)
        me:create_collision()
        me:setup_collision(THROWN, HITS, 0)
        me.collision:set_region(WIDE)
    end,
}

local OTHER_VARIANT = schema.record_type("other-variant")

-- A kind that stores a record of the wrong type in a ticker's variant.
local OTHER = new.record(OTHER_VARIANT, { lifetime = 1 })

launcher.misuse = new.kind {
    id = "test/misuse",
    pool = "effect",
    update = function(me: Object)
        local o = battle.spawn(ticker.kind, me.pos)
        if o then
            (o.state :: any).variant = OTHER
        end
    end,
}

return launcher
