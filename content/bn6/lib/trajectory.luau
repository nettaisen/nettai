--!strict
-- Aiming thrown things: the game's trajectory helpers and the GBA BIOS
-- routines they call (division, square root, arctangent), as the BIOS
-- computes them, over the pack's sine table (data.rules.sine).
--
-- All values are 32-bit integers as the game keeps them: positions and
-- velocities 16.16, angles 256 steps a turn.

local trajectory = {}

-- ---- The BIOS ----------------------------------------------------------------

-- `SWI_Div` (BIOS call 6): the quotient truncated toward zero, the
-- remainder, and the quotient's magnitude.
function trajectory.div(num: number, denom: number): (number, number, number)
    num, denom = int.i32(num), int.i32(denom)
    if denom == 0 then
        -- The BIOS loops forever on n / 0 (it returns for n = 0, 1, -1).
        error(string.format("the BIOS division %d / 0", num))
    end
    local q = int.i32(int.tdiv(num, denom))
    local r = int.i32(num - int.tdiv(num, denom) * denom)
    return q, r, math.abs(q)
end

-- `SWI_Sqrt` (BIOS call 8): the whole square root of an unsigned word.
function trajectory.sqrt(x: number): number
    x = int.u32(x)
    local lo, hi = 0, 0x10000
    -- The largest r with r * r <= x (products stay under 2^33: exact).
    while hi - lo > 1 do
        local mid = (lo + hi) // 2
        if mid * mid <= x then
            lo = mid
        else
            hi = mid
        end
    end
    return lo
end

-- The BIOS arctangent's polynomial, for |i| <= 1.0 (1 << 14).
local function arctan(i: number): number
    local a = -int.asr(int.mul32(i, i), 14)
    local b = int.asr(int.mul32(0xA9, a), 14) + 0x390
    b = int.asr(int.mul32(b, a), 14) + 0x91C
    b = int.asr(int.mul32(b, a), 14) + 0xFB6
    b = int.asr(int.mul32(b, a), 14) + 0x16AA
    b = int.asr(int.mul32(b, a), 14) + 0x2081
    b = int.asr(int.mul32(b, a), 14) + 0x3651
    b = int.asr(int.mul32(b, a), 14) + 0xA2F9
    return int.i16(int.asr(int.mul32(i, b), 16))
end

-- `SWI_ArcTan2` (BIOS call 10): the angle of (x, y), 0x10000 a turn.
function trajectory.arctan2(x: number, y: number): number
    x, y = int.i32(x), int.i32(y)
    local r
    if y == 0 then
        r = if x >= 0 then 0 else 0x8000
    elseif x == 0 then
        r = if y >= 0 then 0x4000 else 0xC000
    elseif y >= 0 then
        if x >= 0 and x >= y then
            r = arctan(int.tdiv(int.shl(y, 14), x))
        elseif x < 0 and -x >= y then
            r = arctan(int.tdiv(int.shl(y, 14), x)) + 0x8000
        else
            r = 0x4000 - arctan(int.tdiv(int.shl(x, 14), y))
        end
    else
        if x <= 0 and -x > -y then
            r = arctan(int.tdiv(int.shl(y, 14), x)) + 0x8000
        elseif x > 0 and x >= -y then
            r = arctan(int.tdiv(int.shl(y, 14), x)) + 0x10000
        else
            r = 0xC000 - arctan(int.tdiv(int.shl(x, 14), y))
        end
    end
    return int.u16(r)
end

-- ---- The game's helpers ---------------------------------------------------------

-- `math_sinTable`: the sine of step `i` (256 steps a turn, 1.0 = 0x100;
-- the table runs over a turn and a half, so the cosine of step a is entry
-- a + 64).
function trajectory.sine(i: number): number
    local v = data.rules.sine[i]
    if v == nil then
        error(string.format("sine table entry %d is not in the content", i))
    end
    return v
end
local sine = trajectory.sine

-- `calcAngle_800117C`: the angle toward (dx, dy), 256 steps a turn.
function trajectory.angle(dx: number, dy: number): number
    return bit32.rshift(trajectory.arctan2(int.asr(dx, 16), int.asr(dy, 16)), 8)
end

-- `sub_80011A0`: a velocity of `speed` toward `angle` (x and y).
function trajectory.toward(angle: number, speed: number): (number, number)
    local cos, sin = sine(angle + 64), -sine(angle + 128)
    return int.asr(int.mul32(cos, speed), 8), int.asr(int.mul32(sin, speed), 8)
end

-- `sub_8001330`: the velocity that takes `pos` to (x, y, z) in `ticks`
-- ticks, falling by `gravity` a tick.
function trajectory.in_ticks(pos: Vec3, x: number, y: number, z: number, ticks: number, gravity: number): Vec3
    local dx, dy = int.i32(x - pos.x), int.i32(y - pos.y)
    local angle = trajectory.angle(dx, dy)
    local ax, ay = int.asr(dy, 8), int.asr(dx, 8)
    local distance = trajectory.sqrt(int.i32(int.mul32(ax, ax) + int.mul32(ay, ay)))
    local speed = trajectory.div(int.shl(distance, 8), ticks)
    local vx, vy = trajectory.toward(angle, speed)
    local drop = int.asr(int.mul32(int.mul32(ticks, ticks), gravity), 1)
    local vz = trajectory.div(int.i32(z - pos.z - drop), ticks)
    return Vec3.new(vx, vy, vz)
end

-- `sub_800120E`: at `speed` along the ground from `pos` to (x, y), how
-- many ticks, and the rise that brings it to height `z` then, falling by
-- `gravity` a tick: (rise, ticks). `vel_y` is what the game reads as the
-- height when the distance rounds to no ticks (the word after the
-- position's Z: the object's Y velocity).
function trajectory.at_speed(pos: Vec3, vel_y: number, x: number, y: number, z: number, speed: number, gravity: number): (number, number)
    local height = pos.z
    if speed ~= 0 then
        local ax, ay = int.asr(int.i32(x - pos.x), 8), int.asr(int.i32(y - pos.y), 8)
        local distance = int.shl(trajectory.sqrt(int.i32(int.mul32(ax, ax) + int.mul32(ay, ay))), 8)
        local ticks, rem = trajectory.div(distance, speed)
        speed = bit32.rshift(speed, 1)
        if int.i32(rem - speed) >= 0 then
            ticks += 1
        end
        if ticks ~= 0 then
            local rise = trajectory.div(int.i32(z - pos.z), ticks)
            local fall = bit32.rshift(int.mul32(int.i32(-gravity), ticks), 1)
            return int.i32(rise + fall), ticks
        end
        height = vel_y
    end
    local climb = int.i32(z - height)
    if climb >= 0 then
        return speed, 0
    end
    local q = trajectory.div(climb, bit32.rshift(int.u32(gravity), 8))
    local root = trajectory.sqrt(q)
    local up = if bit32.band(root, 0xFF) ~= 0 then 1 else 0
    return 0, bit32.rshift(root, 8) + up
end

return trajectory
