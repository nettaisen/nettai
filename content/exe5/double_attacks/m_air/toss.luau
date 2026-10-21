--!strict
-- A magnet bomb MagnetMan sends up to GyroMan in M-AIR (the DS's effect object 0xA3, 0x020A1B74): seen only, it goes
-- in a straight line from 30 pixels before MagnetMan's spot, 10 above and 56 up, to 10 before it, 10 below and 80 up,
-- a tenth of the way a tick, and is gone as it would arrive. Its sound as it starts.

local toss = {}

local SPRITE = asset.sprite("magnet-bomb")
local SOUND = asset.sound("magnet-bomb-toss")
-- The way, in tenths (16.16 offsets from the panel's center: x toward its front).
local STEPS = 0xA
local FROM_X, FROM_Y, FROM_Z = 0x1E_0000, -0xA_0000, 0x38_0000
local TO_X, TO_Y, TO_Z = 0xA_0000, 0xA_0000, 0x50_0000

-- 0x020A1C88: `t` tenths from `a` to `b` (truncating).
local function lerp(a: number, b: number, t: number): number
    return int.tdiv(int.mul32(b - a, t), STEPS) + a
end

-- 0x020A1C04: its tick count up; under 10, there; else gone.
local function tick(me: Object)
    if me.phase_init == 0 then
        me.timer = 0
        me.phase_init = 4
    end
    local t = me.timer + 1
    me.timer = t
    if t >= STEPS then
        me:set_lifecycle("destroy")
        return
    end
    -- (Its ends, the DS's +0x60 to +0x74, from its panel's center and its facing, which stay.)
    local d = me.facing
    local cx, cy = field.center(me.panel_x, me.panel_y)
    me.pos = Vec3.new(
        lerp(cx + FROM_X * d, cx + TO_X * d, t),
        lerp(cy + FROM_Y, cy + TO_Y, t),
        lerp(FROM_Z, TO_Z, t)
    )
end

-- 0x020A1B88: its sprite (10-2E, no shadow, animation 0), shown; the sound; then
-- its first tick at once.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0xFF
    me.visible = true
    battle.play_sound(SOUND)
    me:set_lifecycle("update")
    tick(me)
end

toss.kind = new.kind {
    id = "m-air/toss",
    pool = "effect",
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
            return
        end
        me:update_sprite()
    end,
}

-- 0x020A1C2C: one from `magnetman`'s spot, of his side and facing.
function toss.send(magnetman: Object): Object?
    local o = battle.spawn(toss.kind)
    if o then
        o.panel_x, o.panel_y = magnetman.panel_x, magnetman.panel_y
        o.alliance, o.flip = magnetman.alliance, magnetman.flip
        o.run_while_dimmed = true
    end
    return o
end

return toss
