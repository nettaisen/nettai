--!strict
-- A chip of broken rock (effect object #9, `sub_80E0FA0`): spawned with a
-- sideways and an upward speed (`sub_80E1084`), it flies and falls; below
-- the ground it blinks for 10 ticks and is gone.

export type State = {
    -- ObjectData+0x0C: the sprite's priority.
    priority: number,
}

local rock_chip = {
    state = { priority = "u8" },
}

local SPRITE = "10-05"

-- The fall's acceleration (16.16, down).
local GRAVITY = 0x3000

-- `sub_80E1084`: a chip at `pos` with speeds `vx` and `vz` (16.16), drawn
-- at priority `priority`, with Param1 `n`.
function rock_chip.spawn(pos: Vec3, vx: number, vz: number, priority: number, n: number): Object?
    local o = battle.spawn_kind("rock-chip", pos, { n, 0, 0, 0 })
    if o then
        local s = o.state :: State
        s.priority = priority
        o.vel = Vec3.new(vx, 0, vz)
    end
    return o
end

-- `sub_80E0FC0`: visible, its sprite in animation 1, 10 ticks to blink.
local function init(me: Object, s: State)
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim = 1
    sprite:set_animation(1)
    sprite:step()
    sprite.priority = s.priority
    me.timer = 10
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
end

-- `sub_80E1028`: fly and fall; under the ground, on to blinking.
local function fly(me: Object)
    local v = me.vel
    local vz = v.z - GRAVITY
    me.vel = Vec3.new(v.x, v.y, vz)
    local p = me.pos
    local z = p.z + vz
    me.pos = Vec3.new(p.x + v.x, p.y, z)
    if z < 0 then
        me.action = 4
        me.phase = 0
        me.phase_init = 0
    end
end

-- `sub_80E1054`: blink (shown on the ticks the timer's bit 1 is set), then
-- gone.
local function blink(me: Object): boolean
    me.visible = bit32.btest(me.timer, 2)
    me.timer -= 1
    if me.timer == 0 then
        me:free()
        return false
    end
    return true
end

-- `sub_80E100A`.
local function tick(me: Object)
    if me.action == 0 then
        fly(me)
    elseif me.action == 4 then
        if not blink(me) then
            return
        end
    else
        error("rock chip action " .. me.action .. " runs off its table (sub_80E100A)")
    end
    me:update_sprite()
end

function rock_chip.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me, me.state :: State)
        tick(me)
    elseif l == "update" then
        tick(me)
    else
        me:free()
    end
end

return rock_chip
