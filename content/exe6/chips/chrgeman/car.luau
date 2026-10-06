--!strict
-- A train car (the original's attack object #0xAC, `sub_80DAE94`): it
-- waits (the navi chip's; another sets off at once), then rolls ahead at
-- its speed to its side's far edge, hitting each new panel it reaches; it
-- explodes where the floor ends, and when its owner leaves the action it
-- is tied to (if any) or the battle is over. Each car sits its place + 1
-- pixels higher and further back than the one before, so they stack. The
-- navi chip's acts, steps its sprite and hits while dimmed; another waits
-- a dimming out. What it is spawned with is a `ChargeCar` (types.d.luau).
-- See docs/engine/chips.md §3.6.17.

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local car = {}

export type State = {
    -- Its speed forward, 16.16 (ExtraVars[0]).
    speed: number,
    -- It waits before it rolls, and acts while the battle is dimmed
    -- (Param1 set: the navi chip's).
    while_dimmed: boolean,
    -- Its place in the train (Param2) and the ticks it waits (Param3).
    place: number,
    wait: number,
    -- The action of its owner's it is tied to (Param4; 0xFF: none).
    owner_action: number,
}

local SPRITE = asset.sprite("charge-car")

-- The floor it needs (`byte_80DB004`, both rows alike): require, forbid.
local FLOOR = { 0x10, 0 }

-- Its hit's modifier (`dword_80DB0B4`): plain, and the navi chip's.
local HIT_MOD, CHIP_HIT_MOD = 1, 3
-- No owner's action to watch.
local UNTIED = 0xFF

-- `sub_80DB0D0`: a burst of smoke where it is, 16 pixels up.
local function explode(me: Object)
    local p = me.pos
    battle.effect(Vec3.new(p.x, p.y, 0x10_0000), effects.smoke)
end

-- `sub_80DAEC4`: on its panel, raised and set back by its place in the
-- train (`sub_80DB0BC`: halfword stores over Z's and Y's whole parts);
-- one that isn't the navi chip's skips the wait.
local function init(me: Object)
    local s = me.state :: State
    me:set_coordinates_from_panel()
    local p = me.pos
    local lift = s.place + 1
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
    me:set_action(if s.while_dimmed then 0 else 4)
end

-- `sub_80DAF74`: its wait.
local function wait(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.timer = (me.state :: State).wait
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_action(4)
    end
end

-- `sub_80DB078`: the hit on its panel (the navi chip's: running while
-- dimmed, `sub_80C53A6`).
local function hit(me: Object)
    local chip = (me.state :: State).while_dimmed
    local box = battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.element,
        z = 0,
        region = regions.single,
        hit_effect = sparks.breaking,
        target = collision.hits_navis,
        self_type = collision.breaking,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = if chip then CHIP_HIT_MOD else HIT_MOD,
    })
    if box and chip then
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
        if not field.check(x, y, FLOOR[1], FLOOR[2]) then
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
-- leaves the action it is tied to; outside a dimming unless it is the
-- navi chip's.
local function tick(me: Object)
    if battle.over() then
        explode(me)
        me.lifecycle = "destroy"
        return
    end
    me:update_visibility()
    local s = me.state :: State
    local action = s.owner_action
    if action ~= UNTIED and (me.related1 :: Object).action ~= action then
        explode(me)
        me.lifecycle = "destroy"
        return
    end
    if not s.while_dimmed and battle.dimmed() then
        return
    end
    local f = ACTIONS[me.action]
    if not f then
        error(string.format("train car action %#x reads past its table (off_80DAF6C)", me.action))
    end
    f(me)
end

car.kind = new.kind {
    id = "chrgeman/car",
    pool = "attack",
    state = {
        speed = "u32",
        while_dimmed = "bool",
        place = "u8",
        wait = "u8",
        owner_action = "u8",
    },
    -- `sub_80DAE94`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        if (me.state :: State).while_dimmed then
            me:update_sprite_while_dimmed()
        else
            me:update_sprite_while_paused()
        end
    end,
}

-- `sub_80DB014`: a car on (x, y) for `owner` with `element`, the damage
-- word, what it is and `speed`. `z` is the spawner's register that
-- becomes its Z, whose fraction it keeps.
function car.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    what: ChargeCar,
    damage: number,
    speed: number,
    z: number
): Object?
    local o = battle.spawn(car.kind, Vec3.new(y, element, z))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.related1 = owner
        o:set_damage_word(damage)
        local s = o.state :: State
        s.speed = speed
        s.while_dimmed = what.while_dimmed == true
        s.place = what.place
        s.wait = what.wait
        s.owner_action = what.owner_action or UNTIED
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
    end
    return o
end

-- `sub_80DB03E`: its owner bursts it: the smoke, and it ends (a byte
-- store: its action and phase stay).
function car.burst(o: Object)
    explode(o)
    o.lifecycle = "destroy"
end

return car
