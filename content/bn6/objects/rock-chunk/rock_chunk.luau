--!strict
-- A rock chunk (effect object #9, `sub_80E0FA0`): a piece of a broken
-- falling rock, thrown up and falling back (a Z speed dropping 0x3000 a
-- tick); once below the ground it blinks for 10 ticks and goes.

export type State = {
    -- Its sprite's priority against the background (object +0x0C).
    priority: number,
}

local rock_chunk = {
    state = { priority = "u8" },
}

local SPRITE, ANIM = "10-05", 1
local GRAVITY = 0x3000

-- The actions, on its action byte.
local FLY, BLINK = 0, 4

-- `sub_80E1084`: a chunk at `pos` with Param1 `param`, the sprite
-- priority `priority`, thrown at X speed `vx` and Z speed `vz`.
function rock_chunk.spawn(pos: Vec3, param: number, priority: number, vx: number, vz: number): Object?
    local o = battle.spawn_kind("rock-chunk", pos, { param })
    if o then
        (o.state :: State).priority = priority
        o.vel = Vec3.new(vx, 0, vz)
    end
    return o
end

-- `sub_80E1028`: fly; below the ground, on to the blinking.
local function fly(me: Object)
    local v = me.vel
    local vz = int.i32(v.z - GRAVITY)
    me.vel = Vec3.new(v.x, v.y, vz)
    local p = me.pos
    local z = int.i32(p.z + vz)
    me.pos = Vec3.new(p.x + v.x, p.y, z)
    if z < 0 then
        me.action = BLINK
        me.phase, me.phase_init = 0, 0
    end
end

-- `sub_80E1054`: shown every other two ticks; gone when the timer runs
-- out. True if it went.
local function blink(me: Object): boolean
    me.visible = bit32.btest(me.timer, 2)
    local left = me.timer - 1
    me.timer = left
    if left == 0 then
        -- sub_80E107C
        me:free()
        return true
    end
    return false
end

-- `sub_80E100A`: the action, then the sprite.
local function tick(me: Object)
    local a = me.action
    if a == FLY then
        fly(me)
    elseif a == BLINK then
        if blink(me) then
            -- (The game steps the freed slot's sprite; nothing reads it.)
            return
        end
    else
        error(string.format("rock chunk action %#x reads past sub_80E100A's table", a))
    end
    me:update_sprite()
end

-- `sub_80E0FC0`.
local function init(me: Object)
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim = ANIM
    sprite:set_animation(ANIM)
    sprite:step()
    sprite.priority = (me.state :: State).priority
    me.timer = 10
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80E0FA0`.
function rock_chunk.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:free()
    end
end

return rock_chunk
