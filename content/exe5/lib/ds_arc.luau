--!strict
-- The DS's `sub_8001330` (0x0208DB5C, the DS's party battle's Double Attacks: K-THROW's KnightMan, K-BOMB's bombs):
-- the velocity that takes `pos` to (x, y, z) in `ticks` ticks, falling by `gravity` a tick. The GBA's (@exelib/
-- trajectory's `in_ticks`) relinked, with two of its helpers rewritten for the DS's hardware:
--
-- - the angle (0x0208D980, the GBA's `calcAngle_800117C`): the SDK's FX_Atan2Idx (0x020B15BC) of (-dx / 16, dy / 16),
--   its table (0x0213D144) read at the hardware divider's rounded quotient (FX_Div, 0x020B1318), less 0xBF80, in 256
--   steps a turn;
-- - the distance: the deltas over 1024 (the GBA's over 256), the hardware square root (0x020B12AC: of the sum in its
--   high word, rounded to 6 fraction bits; 0 for a sum that isn't positive), its whole part times 1024.
--
-- The rest (the speed, the sine table's step, the fall) is the GBA's, the DS's sine table EXE5's (rules/math).

local trajectory = require("./trajectory")

local ds_arc = {}

-- FX_AtanIdxTable_ (0x0213D144): atan(i / 128) in 65536 steps a turn; past its 128 entries the read takes the next
-- word's halfword, 0 (an angle within a step of a diagonal reads it).
local ATAN: { number } = {
    0, 81, 163, 244, 326, 407, 489, 570, 651, 732, 813, 894, 975, 1056, 1136, 1217, 1297, 1377, 1457, 1537, 1617, 1696,
    1775, 1854, 1933, 2012, 2090, 2168, 2246, 2324, 2401, 2478, 2555, 2632, 2708, 2784, 2860, 2935, 3010, 3085, 3159,
    3233, 3307, 3380, 3453, 3526, 3599, 3670, 3742, 3813, 3884, 3955, 4025, 4095, 4164, 4233, 4302, 4370, 4438, 4505,
    4572, 4639, 4705, 4771, 4836, 4901, 4966, 5030, 5094, 5157, 5220, 5282, 5344, 5406, 5467, 5528, 5589, 5649, 5708,
    5768, 5826, 5885, 5943, 6000, 6058, 6114, 6171, 6227, 6282, 6337, 6392, 6446, 6500, 6554, 6607, 6660, 6712, 6764,
    6815, 6867, 6917, 6968, 7018, 7068, 7117, 7166, 7214, 7262, 7310, 7358, 7405, 7451, 7498, 7544, 7589, 7635, 7679,
    7724, 7768, 7812, 7856, 7899, 7942, 7984, 8026, 8068, 8110, 8151, 0,
}

-- FX_Div (0x020B1318) of 0 <= a <= b: (a << 32) / b by the hardware divider, rounded to 12 fraction bits.
local function fx_div(a: number, b: number): number
    return (2 * a * 0x1000 + b) // (2 * b)
end

-- 0x020B171C: `base` plus (or less) the table's angle of `small` / `large`.
local function from_table(base: number, small: number, large: number, adds: boolean): number
    if large == 0 then
        return 0
    end
    local v = ATAN[int.asr(fx_div(small, large), 5) + 1]
    return bit32.band(if adds then base + v else base - v, 0xFFFF)
end

-- FX_Atan2Idx (0x020B15BC): the angle of (x, y), 65536 steps a turn, by octant.
local function atan2_idx(y: number, x: number): number
    if y > 0 then
        if x > 0 then
            if x > y then
                return from_table(0, y, x, true)
            elseif x < y then
                return from_table(0x4000, x, y, false)
            end
            return 0x2000
        elseif x < 0 then
            x = -x
            if x < y then
                return from_table(0x4000, x, y, true)
            elseif x > y then
                return from_table(0x8000, y, x, false)
            end
            return 0x6000
        end
        return 0x4000
    elseif y < 0 then
        local ny = -y
        if x < 0 then
            x = -x
            if x > ny then
                return from_table(-0x8000, ny, x, true)
            elseif x < ny then
                return from_table(-0x4000, x, ny, false)
            end
            return 0xA000
        elseif x > 0 then
            if x < ny then
                return from_table(-0x4000, x, ny, true)
            elseif x > ny then
                return from_table(0, ny, x, false)
            end
            return 0xE000
        end
        return 0xC000
    end
    return if x >= 0 then 0 else 0x8000
end

-- 0x0208D980: the angle toward (dx, dy), 256 steps a turn.
local function angle_of(dx: number, dy: number): number
    local a = atan2_idx(int.i32(-int.asr(dx, 4)), int.asr(dy, 4))
    return bit32.rshift(bit32.band(a - 0xBF80, 0xFFFF), 8)
end

-- 0x020B12AC: the hardware square root of `v` << 32, rounded to 6 fraction bits (0 for a `v` that isn't positive).
local function sqrt(v: number): number
    if v <= 0 then
        return 0
    end
    -- floor(sqrt(v) * 65536): the whole root, then the fraction's 16 bits one at a time.
    local root = trajectory.sqrt(v)
    local left = (v - root * root) * 0x1_0000_0000
    local k = 0
    for bit = 15, 0, -1 do
        local t = k + bit32.lshift(1, bit)
        if 2 * root * t * 0x10000 + t * t <= left then
            k = t
        end
    end
    return (root * 0x10000 + k + 0x200) // 0x400
end

-- The canonical side's computation.
local function in_ticks(pos: Vec3, x: number, y: number, z: number, ticks: number, gravity: number): Vec3
    local dx, dy = int.i32(x - pos.x), int.i32(y - pos.y)
    local angle = angle_of(dx, dy)
    local ax, ay = int.asr(dx, 10), int.asr(dy, 10)
    local root = sqrt(int.i32(int.mul32(ax, ax) + int.mul32(ay, ay)))
    local distance = int.u32(bit32.lshift(bit32.rshift(root, 6), 10))
    local speed = trajectory.div(distance, ticks)
    local vx, vy = trajectory.toward(angle, speed)
    local drop = int.asr(int.mul32(int.mul32(ticks, ticks), gravity), 1)
    local vz = trajectory.div(int.i32(z - pos.z - drop), ticks)
    return Vec3.new(vx, vy, vz)
end

-- 0x0208DB5C for an object of `side`: the canonical side's as the DS computes it; the other side's on the mirror
-- image, its X velocity mirrored back (docs/engine/side-symmetry.md, as @exelib/trajectory's).
function ds_arc.in_ticks(pos: Vec3, x: number, y: number, z: number, ticks: number, gravity: number, side: number): Vec3
    if field.side_sign(side) < 0 then
        local v = in_ticks(Vec3.new(int.i32(-pos.x), pos.y, pos.z), int.i32(-x), y, z, ticks, gravity)
        return Vec3.new(int.i32(-v.x), v.y, v.z)
    end
    return in_ticks(pos, x, y, z, ticks, gravity)
end

return ds_arc
