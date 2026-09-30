--!strict
-- A train car (attack object #0xAC, `sub_80DAE94`): it waits Param3 ticks
-- (or, Param1 0, sets off at once), then rolls ahead at ExtraVars[0]'s
-- speed to its side's far edge, hitting each new panel it reaches (region
-- 1); it explodes where the floor ends, and when its owner leaves action
-- Param4 (0xFF: never checked) or the battle is over. Each car sits
-- Param2 + 1 pixels higher and further back than the one before, so they
-- stack. With Param1 set (ChargeMan's) it acts, steps its sprite and hits
-- while dimmed; Param1 0 waits a dimming out. See docs/engine/chips.md
-- §3.6.17.

local charge_car = {}

export type State = {
    -- Its speed forward, 16.16 (ExtraVars[0]).
    speed: number,
}

charge_car.state = { speed = "u32" }

local SPRITE = "10-54"

-- The floor it needs, by Param1 (`byte_80DB004`): require, forbid.
local FLOOR = { [0] = { 0x10, 0 }, { 0x10, 0 } }

-- Its hit's modifier by Param1 (`dword_80DB0B4`).
local HIT_MODS = { [0] = 1, 3 }

-- `sub_80DB014`: a car on (x, y) for `owner` with `element`, the damage
-- word, `params` {Param1, place, wait, owner's action} and `speed`. `z`
-- is the spawner's register that becomes its Z, whose fraction it keeps.
function charge_car.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    params: { number },
    damage: number,
    speed: number,
    z: number
): Object?
    local o = battle.spawn_kind("charge-car", Vec3.new(y, element, z), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.related1 = owner
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        local s = o.state :: State
        s.speed = speed
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
    end
    return o
end

-- `sub_80DB0D0`: an explosion (effect 0x12) where it is, 16 pixels up.
local function explode(me: Object)
    local p = me.pos
    battle.effect(Vec3.new(p.x, p.y, 0x10_0000), 0x12)
end

-- `sub_80DAEC4`: on its panel, raised and set back by its place in the
-- train (`sub_80DB0BC`: halfword stores over Z's and Y's whole parts);
-- Param1 0 skips the wait.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    local lift = me:param(2) + 1
    local z = bit32.bor(bit32.band(p.z, 0xFFFF), bit32.lshift(bit32.band(-lift, 0xFFFF), 16))
    local y = bit32.bor(bit32.band(p.y, 0xFFFF), bit32.lshift(bit32.band(bit32.rshift(p.y, 16) - lift, 0xFFFF), 16))
    me.pos = Vec3.new(p.x, int.i32(y), int.i32(z))
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:update_visibility()
    me.lifecycle = "update"
    me:set_action(if me:param(1) == 0 then 4 else 0)
end

-- `sub_80DAF74`: Param3 ticks.
local function wait(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.timer = me:param(3)
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_action(4)
    end
end

-- `sub_80DB078`: the hit on its panel (Param1 set: running while dimmed,
-- `sub_80C53A6`).
local function hit(me: Object)
    local p1 = me:param(1)
    local box = battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.element,
        z = 0,
        region = 1,
        hit_effect = 0xA,
        target = 5,
        self_type = 6,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = HIT_MODS[if p1 == 0 then 0 else 1],
    })
    if box and p1 ~= 0 then
        box.run_while_dimmed = true
    end
end

-- The panel under a position (`sub_800E258`: two BIOS divisions).
local function panel_under(p: Vec3): (number, number)
    return int.tdiv(int.asr(p.x, 16) + 0xA0, 0x28), int.tdiv(int.asr(p.y, 16) + 0x20, 0x18)
end

-- `sub_80DAF98`: roll to the far edge (`sub_80DB04C`: |edge - x| panels of
-- 40 pixels at its speed, a BIOS division), hitting each new panel;
-- explode where the floor ends.
local function roll(me: Object)
    if me.phase == 0 then
        me.phase = 4
        local speed = (me.state :: State).speed
        me.vel = Vec3.new(int.mul32(me.facing, speed), me.vel.y, me.vel.z)
        local edge = if me.alliance == 0 then 7 else 0
        me.timer = int.tdiv(int.mul32(math.abs(edge - me.panel_x), 0x28_0000), speed)
        return
    end
    local old_x = me.panel_x
    local p = me.pos
    me.pos = Vec3.new(p.x + me.vel.x, p.y, p.z)
    local x, y = panel_under(me.pos)
    if field.valid(x, y) then
        local floor = FLOOR[me:param(1)]
        if not floor then
            error(string.format("train car %d reads past byte_80DB004", me:param(1)))
        end
        if not field.check(x, y, floor[1], floor[2]) then
            explode(me)
            me:set_lifecycle("destroy")
            return
        end
    end
    me:set_panel_from_coordinates()
    if me.panel_x ~= old_x then
        hit(me)
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_lifecycle("destroy")
    end
end

local ACTIONS = { [0] = wait, [4] = roll }

-- `sub_80DAF2A`: gone (exploding) once the battle is over or its owner
-- leaves action Param4; outside a dimming unless Param1 is set.
local function tick(me: Object)
    if battle.over() then
        explode(me)
        me.lifecycle = "destroy"
        return
    end
    me:update_visibility()
    local action = me:param(4)
    if action ~= 0xFF and (me.related1 :: Object).action ~= action then
        explode(me)
        me.lifecycle = "destroy"
        return
    end
    if me:param(1) == 0 and battle.dimmed() then
        return
    end
    local f = ACTIONS[me.action]
    if not f then
        error(string.format("train car action %#x reads past its table (off_80DAF6C)", me.action))
    end
    f(me)
end

-- `sub_80DAE94`.
function charge_car.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
    if me:param(1) == 0 then
        me:update_sprite_while_paused()
    else
        me:update_sprite_while_dimmed()
    end
end

return charge_car
