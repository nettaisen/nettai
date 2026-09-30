--!strict
-- EraseCross Beast's drop (attack object #0xA1, `sub_80D96A4`): it falls
-- onto its panel (Param3, Param4) from high up behind, in Param1 ticks,
-- and hits it; on solid ground it shakes the screen and blinks for 10
-- ticks, over a hole it bursts at once. It stands still while dimmed.

local erase_drop = {}

local SPRITE = "10-4d"

-- Where it starts, relative to its panel: this much back and up, 16.16;
-- the fall covers the first part of each.
local FALL = 0x3C_0000
local BACK, UP = 0x14_0000, 0x19_0000

-- The actions, on its action byte.
local FALLING, BLINKING = 0, 4

-- A panel's "solid" bit (the flags word).
local SOLID = 0x10

-- The effects: where it appears (0x15) and where it bursts (0x14).
local APPEAR, BURST = 0x15, 0x14

-- `sub_80D981C`: a drop for `owner` with `params` (the fall's ticks, 0,
-- and the panel) and the damage word `damage`; Cursor (0x40) whatever the
-- attack's element. (Its position is the spawner's registers until its
-- init places it.)
function erase_drop.spawn(owner: Object, params: { number }, damage: number): Object?
    local o = battle.spawn_kind("erase-drop", Vec3.zero, params)
    if o then
        o.alliance, o.flip = owner.alliance, owner.flip
        o.element = 0x40
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
    end
    return o
end

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
    end
    -- (Otherwise a camera shake: presentation only.)
    battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.element,
        z = 0x10_0000,
        region = 1,
        hit_effect = 0,
        target = 5,
        self_type = 0x0B,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 3,
        status = 0,
        bug = 0x18,
        bug_arg = 1,
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
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.panel_x, me.panel_y = me:param(3), me:param(4)
    me:set_coordinates_from_panel()
    local ticks = me:param(1)
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

-- `sub_80D96A4`.
function erase_drop.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
    me:update_sprite()
end

return erase_drop
