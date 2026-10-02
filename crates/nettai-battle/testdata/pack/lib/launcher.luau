--!strict
-- A kind that uses the v2 API once: it spawns a ticker by definition and
-- fills its references, shows an effect and plays a sound by name, and
-- registers its collision with defined collision types and a defined
-- region.

local ticker = require("./ticker")

local SHORT = ticker.variant { lifetime = 5 }
local BURST = define.effect { sprite = asset.sprite("test:test-burst"), anim = 2 }
local WIDE = define.region { panels = { { 1, -1 }, { 1, 0 }, { 1, 1 } } }
local THROWN = define.collision { id = "test:test/thrown", side0 = 0x80000088, side1 = 0x40000088 }
local HITS = define.collision { id = "test:test/hits-navis", side0 = 0x15800000, side1 = 0x2a800000 }
local TICK = asset.sound("test:test-tick")

local launcher = {}

launcher.kind = define.kind {
    id = "test:test/launcher",
    pool = "attack",
    state = { launched = "bool" },
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

-- A kind that stores a record of the wrong type in a ticker's variant.
local OTHER = define.record("other-variant", { lifetime = 1 })

launcher.misuse = define.kind {
    id = "test:test/misuse",
    pool = "effect",
    update = function(me: Object)
        local o = battle.spawn(ticker.kind, me.pos)
        if o then
            (o.state :: any).variant = OTHER
        end
    end,
}

return launcher
