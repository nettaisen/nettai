--!strict
-- Falzar's whirlwind (the original's attack object #0xCE, `sub_80DF188`;
-- its routines are in the US ROMs too, but only the Japanese ROMs' Falzar
-- chip spawns it; docs/engine/beast-chips.md): a whirlwind forty pixels
-- ahead of the panel in front of Falzar's summon that blows three times,
-- at its start and every third of its time after: the first blow hits the
-- panel it is on, the next one and the column two ahead; the second the
-- last two; the third the column alone. Each hit resolves while dimmed.
-- Falzar's leading piece sends it (chips/falzar/summon).

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local tornado = {}

export type State = {
    -- How long it blows (its Param1), the ticks between blows (a third of
    -- that, ExtraVars), and the blows so far (ExtraVars+4).
    ticks: number,
    every: number,
    blows: number,
}

local SPRITE = asset.sprite("whirlwind")
-- Forty pixels ahead of its panel.
local AHEAD = 0x28_0000
-- Its hits' modifier.
local HIT_MOD = 3

-- A blow's hits (`off_80DF2CC`, three entries of eight bytes each, an
-- entry's second byte zero when it has none): panels ahead, and the
-- region (0x04050601 and 0x04050604: a heavy spark, an attack on navis).
type Blow = { ahead: number, region: Region }
local BLOWS: { { Blow } } = {
    { { ahead = 0, region = regions.single }, { ahead = 1, region = regions.single }, { ahead = 2, region = regions.wide } },
    { { ahead = 1, region = regions.single }, { ahead = 2, region = regions.wide } },
    { { ahead = 2, region = regions.wide } },
}

-- `sub_80DF288` and `sub_80DF2A0`: blow `n` (from 0): its hits, resolving
-- while dimmed (`sub_80C53A6`).
local function blow(me: Object, n: number)
    for _, h in BLOWS[n + 1] do
        local o = battle.hitbox(me, {
            panel_x = me.panel_x + me.facing * h.ahead,
            panel_y = me.panel_y,
            element = me.element,
            z = 0,
            region = h.region,
            hit_effect = sparks.heavy,
            target = collision.hits_navis,
            self_type = collision.attack,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = HIT_MOD,
        })
        if o then
            o.run_while_dimmed = true
        end
    end
end

-- `sub_80DF218`: from its first tick a blow, and one every third of its
-- time, three in all; at the end of its time it goes.
local function tick(me: Object)
    local s = me.state :: State
    local a = me.action
    if a ~= 0 then
        error(string.format("Falzar's whirlwind: action %#x reads past its table (off_80DF214)", a))
    end
    if me.phase_init == 0 then
        me.phase_init = 4
        me.timer = s.ticks
        -- (svc 6: the BIOS's division.)
        s.every = s.ticks // 3
        s.blows = 0
        me.timer2 = 0
    end
    local t2 = me.timer2 - 1
    me.timer2 = t2
    if t2 <= 0 and s.blows < #BLOWS then
        blow(me, s.blows)
        me.timer2 = s.every
        s.blows += 1
    end
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        me:set_lifecycle("destroy")
    end
end

-- `sub_80DF1AC`: forty pixels ahead of its panel, on the ground, drawn
-- without a shadow, facing its way.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x + me.facing * AHEAD, p.y, 0)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
end

tornado.kind = new.kind {
    id = "falzar/tornado",
    pool = "attack",
    state = { ticks = "u8", every = "u32", blows = "u32" },
    -- `sub_80DF188`: by state (its end `object_genericDestroy`), then the
    -- sprite, stepping while dimmed.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- `sub_80DF262`: a whirlwind of `owner`'s on (x, y) with `element` and the
-- damage word, blowing `ticks` ticks, acting while dimmed (flag 0x10).
-- (Spawned at the registers: the row, the element and whatever the third
-- holds; its init places it.)
function tornado.spawn(owner: Object, x: number, y: number, element: number, damage: number, ticks: number): Object?
    local o = battle.spawn(tornado.kind, Vec3.new(y, element, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
        ;(o.state :: State).ticks = ticks
    end
    return o
end

return tornado
