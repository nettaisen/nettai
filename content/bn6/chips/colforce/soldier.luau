--!strict
-- One of ColForce's soldiers (the original's attack object #0xCF,
-- `sub_80DF328`): it drops in on a panel behind one of the other side's,
-- waits its turn, lands, and fires three shots (objects/bullet, the
-- vulcan's hit, at a random height) at the panel in front of it, 10 ticks
-- apart; then it salutes and leaves. It stands still while dimmed.

local bullet = require("../../objects/bullet/bullet")

local soldier = {}

export type State = {
    -- Ticks it waits for its turn (its Param1).
    delay: number,
}

local SPRITE = asset.sprite("col-army")
local SHOT_SOUND = asset.sound("col-army-2")
-- Where it drops from, 16.16.
local DROP_Z = 0xFF_0000
-- The shots: bullets (row 0x12), three of them 10 ticks apart, at one of
-- four heights (`dword_80DF484`, in pixels).
local SHOT = bullet.variants.col_force
local SHOTS, SHOT_TICKS = 3, 0xA
local HEIGHTS = { [0] = 0x08, 0x10, 0x18, 0x20 }

-- The actions.
local WAIT, LAND, FIRE, AIM_DOWN, LEAVE = 0, 4, 8, 0xC, 0x10

-- `sub_80C6ADA` and `sub_801155A`: a shot at (x, y), `z` up.
local function shot(me: Object, x: number, y: number, z: number)
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    bullet.fire(me, x, y, z, SHOT, me.element, damage)
end

-- Count the timer down; true when it runs out.
local function count(me: Object): boolean
    local left = me.timer - 1
    me.timer = left
    return left <= 0
end

-- To the next action, from its first phase.
local function next(me: Object, action: number)
    me.action = action
    me.phase, me.phase_init = 0, 0
end

-- `sub_80DF3CC`: its turn.
local function wait(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.timer = (me.state :: State).delay
        return
    end
    if count(me) then
        next(me, LAND)
    end
end

-- `sub_80DF3F0`: on the ground, 20 ticks.
local function land(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 0
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y, 0)
        me.timer = 0x14
        return
    end
    if count(me) then
        next(me, FIRE)
    end
end

-- `sub_80DF41C`: three shots.
local function fire(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 1
        me.timer2 = SHOTS
        me.timer = 0
    end
    if not count(me) then
        return
    end
    me.timer = SHOT_TICKS
    local z = bit32.lshift(HEIGHTS[bit32.band(battle.rng_positive(), 3)], 16)
    shot(me, me.panel_x + me.facing, me.panel_y, z)
    battle.play_sound(SHOT_SOUND)
    local left = me.timer2 - 1
    me.timer2 = left
    if left <= 0 then
        next(me, AIM_DOWN)
    end
end

-- `sub_80DF488`: 10 ticks.
local function aim_down(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 3
        me.timer = 0xA
        return
    end
    if count(me) then
        next(me, LEAVE)
    end
end

-- `sub_80DF4B0`: 13 ticks, then gone.
local function leave(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 2
        me.timer = 0xD
        return
    end
    if count(me) then
        me:set_lifecycle("destroy")
    end
end

-- `sub_80DF34C`: high above its panel.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, DROP_Z)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = 3, 3
    sprite:set_animation(3)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
end

-- `loc_80DF390`.
local function tick(me: Object)
    if battle.over() then
        me:set_lifecycle("destroy")
        return
    end
    if battle.dimmed() then
        return
    end
    local a = me.action
    if a == WAIT then
        wait(me)
    elseif a == LAND then
        land(me)
    elseif a == FIRE then
        fire(me)
    elseif a == AIM_DOWN then
        aim_down(me)
    elseif a == LEAVE then
        leave(me)
    else
        error(string.format("ColForce soldier action %#x reads past its table", a))
    end
end

soldier.kind = define.kind {
    id = "colforce/soldier",
    pool = "attack",
    state = { delay = "u8" },
    -- `sub_80DF328`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        me:update_sprite_while_paused()
    end,
}

-- `sub_80DF4D4`: a soldier of `owner`'s (the force) on (x, y), waiting
-- `delay` ticks, with the damage word `damage`; it runs while dimmed. (It
-- spawns at the registers' values, which its init replaces.)
function soldier.spawn(owner: Object, x: number, y: number, element: number, delay: number, damage: number): Object?
    local o = battle.spawn(soldier.kind, Vec3.new(y, element, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true;
        (o.state :: State).delay = delay
    end
    return o
end

return soldier
