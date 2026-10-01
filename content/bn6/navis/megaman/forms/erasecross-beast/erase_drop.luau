--!strict
-- EraseCross Beast's drop (the original's attack object #0xA1,
-- `sub_80D96A4`), its charged chip's (./drop): it falls onto its panel
-- from high up behind in its ticks, and hits it; on solid ground it shakes
-- the screen and blinks for 10 ticks, over a hole it bursts at once. It
-- stands still while dimmed.

local collision = require("../../../../rules/collision")
local effects = require("../../../../lib/effects")
local regions = require("../../../../lib/regions")
local sparks = require("../../../../lib/sparks")

local erase_drop = {}

export type State = {
    -- How long it falls, in ticks, and the panel it falls on (the
    -- original's first, third and fourth parameters).
    ticks: number,
    x: number,
    y: number,
}

local SPRITE = asset.sprite("erase-drop")

-- Where it starts, relative to its panel: this much back and up, 16.16;
-- the fall covers the first part of each.
local FALL = 0x3C_0000
local BACK, UP = 0x14_0000, 0x19_0000

-- The actions, on its action byte.
local FALLING, BLINKING = 0, 4

-- A panel's "solid" bit (the flags word).
local SOLID = 0x10

-- Where it appears, and its burst (a puff).
local APPEAR = define.effect { sprite = asset.sprite("puff"), anim = 3, palette = 0 }
local BURST = effects.puff

-- Its element: Cursor, whatever the attack's.
local CURSOR = 0x40

-- Its hit: piercing, at navis, with the plain spark and a bug (code 0x18,
-- argument 1).
local HIT_Z, HIT_MOD, BUG, BUG_ARG = 0x10_0000, 3, 0x18, 1

-- The camera's shake when it lands.
local SHAKE, SHAKE_TICKS = 2, 0x18

-- `sub_80D9788`: fall; on landing, the hit, and blink or burst.
local function fall(me: Object)
    local p, v = me.pos, me.vel
    me.pos = Vec3.new(p.x + v.x, p.y, p.z - v.z)
    local left = me.timer - 1
    me.timer = left
    if left >= 0 then
        return
    end
    me.timer = 0xA
    me.action = BLINKING
    me.phase, me.phase_init = 0, 0
    if not bit32.btest(field.flags(me.panel_x, me.panel_y), SOLID) then
        me:set_lifecycle("destroy")
        battle.effect(me.pos, BURST)
    else
        battle.shake_camera(SHAKE, SHAKE_TICKS)
    end
    battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.element,
        z = HIT_Z,
        region = regions.single,
        hit_effect = sparks.plain,
        target = collision.hits_navis,
        self_type = collision.piercing,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = HIT_MOD,
        status = 0,
        bug = BUG,
        bug_arg = BUG_ARG,
    })
end

-- `sub_80D97EC`: blink, then go.
local function blink(me: Object)
    me.visible = bit32.btest(me.timer, 2)
    local left = me.timer - 1
    me.timer = left
    if left >= 0 then
        return
    end
    me.visible = false
    me:set_lifecycle("destroy")
end

-- `sub_80D96C8`: on its panel, high up behind it, falling toward it.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.panel_x, me.panel_y = s.x, s.y
    me:set_coordinates_from_panel()
    local ticks = s.ticks
    if ticks == 0 then
        error("an erase drop with no fall time divides by zero (sub_80D96C8)")
    end
    me.timer = ticks
    local front = me.facing
    local p = me.pos
    me.pos = Vec3.new(p.x - front * FALL - front * BACK, p.y, FALL + UP)
    me.vel = Vec3.new(int.tdiv(front * FALL, ticks), me.vel.y, int.tdiv(FALL, ticks))
    battle.effect(me.pos, APPEAR)
    me:set_lifecycle("update")
    fall(me)
end

-- `sub_80D9746`: gone once the battle is over; still while dimmed.
local function tick(me: Object)
    if battle.over() then
        battle.effect(me.pos, BURST)
        me.visible = false
        me:set_lifecycle("destroy")
        return
    end
    if battle.dimmed() then
        return
    end
    local a = me.action
    if a == FALLING then
        fall(me)
    elseif a == BLINKING then
        blink(me)
    else
        error(string.format("erase drop action %#x reads past its table (sub_80D9746)", a))
    end
end

erase_drop.kind = define.kind {
    id = "erasecross-beast/drop",
    pool = "attack",
    state = { ticks = "u8", x = "u8", y = "u8" },
    -- `sub_80D96A4`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        me:update_sprite()
    end,
}

-- `sub_80D981C`: a drop for `owner` that falls on (x, y) in `ticks` ticks,
-- with the damage word `damage`; Cursor whatever the attack's element. The
-- panel goes through the game's bytes (a panel off the field wraps). (Its
-- position is the spawner's registers until its init places it.)
function erase_drop.spawn(owner: Object, x: number, y: number, ticks: number, damage: number): Object?
    local o = battle.spawn(erase_drop.kind, Vec3.zero)
    if o then
        local s = o.state :: State
        s.ticks, s.x, s.y = ticks, int.u8(x), int.u8(y)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.element = CURSOR
        o:set_damage_word(damage)
    end
    return o
end

return erase_drop
