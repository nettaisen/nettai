--!strict
-- Rubble (effect object #9, `sub_80E0FA0`): a piece of a broken rock that
-- flies off (the rock's sprite, animation 1), falls under gravity, then
-- blinks for 10 ticks and is gone. Purely visual, but it holds an effect
-- slot. GroundMan's rocks throw two where they land (objects/falling-rock);
-- other rocks' spawners (`sub_80C7EC8`) use it too.

export type State = {
    -- Its sprite's priority against the background layers (object
    -- +0x0C, which the spawner sets and the init applies).
    priority: number,
}

local rubble = {
    state = { priority = "u8" },
}

local SPRITE = "10-05"

-- Gravity, per tick (16.16).
local GRAVITY = -0x3000

-- `sub_80E1084`: rubble at `pos` flying at (vx, 0, vz), with `params`
-- (its spawner's register) and sprite priority `priority`.
function rubble.spawn(pos: Vec3, vx: number, vz: number, params: { number }, priority: number): Object?
    local o = battle.spawn_kind("rubble", pos, params)
    if o then
        (o.state :: State).priority = priority
        o.vel = Vec3.new(vx, 0, vz)
    end
    return o
end

-- `sub_80E1028`: fly and fall; once below the ground, blink.
local function fly(me: Object)
    local p, v = me.pos, me.vel
    local vz = v.z + GRAVITY
    me.vel = Vec3.new(v.x, v.y, vz)
    local z = int.i32(p.z + vz)
    me.pos = Vec3.new(p.x + v.x, p.y, z)
    if z < 0 then
        me:set_action(4)
    end
end

-- `sub_80E1054`: blink (shown on ticks whose timer has bit 1) until the
-- timer runs out.
local function blink(me: Object)
    me.visible = bit32.btest(me.timer, 2)
    local left = me.timer - 1
    me.timer = left
    if left == 0 then
        me:free()
    end
end

-- `sub_80E100A`.
local function tick(me: Object)
    local a = me.action
    if a == 0 then
        fly(me)
    elseif a == 4 then
        blink(me)
    else
        error(string.format("rubble action %#x reads past its table (off_80E1020)", a))
    end
    me:update_sprite()
end

-- `sub_80E0FC0`: the rock's sprite at animation 1, a tick in, with its
-- priority; then straight on to its first tick.
local function init(me: Object)
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim = 1
    sprite:set_animation(1)
    sprite:step()
    sprite.priority = (me.state :: State).priority
    me.timer = 0xA
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80E0FA0`.
function rubble.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:free()
    end
end

return rubble
