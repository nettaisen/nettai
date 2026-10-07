--!strict
-- EXE4's SP navi chips' damage (the damage formulas 1 to 22, 0x0801949E: EXE6's `sub_8010AE4`, EXE5's 0x0800E8DE,
-- with EXE4's own reading of the time): lower the slower the side's player deleted the chip's SP navi, a step per two
-- seconds past ten. A chip's damage by step is its `sp` (the rules extend chips with it, ./init: the formula's row of
-- 0x08018B48, 11 a navi); the step is how many of the steps' times (0x080194F0: BCD hours, minutes, seconds,
-- hundredths) the deletion time passes, which the side's setup states for the chip (`sp_times`: the battle's copy at
-- 0x02037C80, 0x2C a side, a halfword a navi in the formulas' order; none stated, deleted in no time). Under event
-- flag 0x1187 (0x080406A4) the step is 5, whatever the time: the main subsystem sets it, no netbattle (rules/flow's
-- `navi_win_banner`), so a netbattle's step is always its time's.

local sp_chips = {}

-- The deletion times at which an SP chip's damage steps down (0x080194F0).
sp_chips.DELETION_TIMES = new.record("sp-deletion-times", {
    0x00001000,
    0x00001200,
    0x00001400,
    0x00001600,
    0x00001800,
    0x00002000,
    0x00002200,
    0x00002400,
    0x00002600,
    0x00002800,
})

-- The frames in each of the time's first six digits (0x08000C00): tens of hours, hours, tens of minutes, minutes,
-- tens of seconds, seconds.
local DIGITS = { 2160000, 216000, 36000, 3600, 600, 60 }
-- Past the most it can say (0x08000BBE), the time is 99:59:59.99.
local MOST, MOST_TIME = 0x01499700, 0x99595999

-- 0x08000A38: a number as BCD, eight digits (to 99999999).
local function bcd(n: number): number
    local out = 0
    local divisor = 10000000
    for _ = 1, 8 do
        local digit = n // divisor
        n -= digit * divisor
        out = bit32.bor(bit32.lshift(out, 4), digit)
        divisor //= 10
    end
    return out
end

-- 0x08000BBC: frames as a BCD time, hours:minutes:seconds.hundredths (a byte each), each of the first six digits by
-- repeated subtraction and the hundredths the frames left times 0x1AAAA, >> 16 (EXE5's and EXE6's `sub_8000D84`
-- divide by 60, which can say one more).
local function time_bcd(frames: number): number
    if frames >= MOST then
        return MOST_TIME
    end
    local out = 0
    for _, d in DIGITS do
        local digit = frames // d
        frames -= digit * d
        out = bit32.bor(bit32.lshift(out, 4), digit)
    end
    return bit32.bor(bit32.lshift(out, 8), bcd(bit32.rshift(frames * 0x1AAAA, 16)))
end

-- The frames side `side`'s player took to delete `chip`'s SP navi: the setup's `sp_times` entry for it, none 0.
local function frames(side: number, chip: Chip): number
    local times = rules.setup_of(side).sp_times
    for i = 1, #times do
        local t = times[i]
        if t.chip == chip then
            return t.frames
        end
    end
    return 0
end

-- An SP chip's damage on side `side` (a chip's `damage`: a function of the side and the chip).
function sp_chips.damage(side: number, chip: Chip): number
    local by_time = (chip :: any).sp :: { number }
    local time = time_bcd(frames(side, chip))
    local step = 0
    for _, t in sp_chips.DELETION_TIMES :: { number } do
        if time <= t then
            break
        end
        step += 1
    end
    local damage = by_time[step + 1]
    if damage == nil then
        error(string.format("SP chip %s has no damage for deletion-time step %d (its `sp`)", tostring(chip.id), step))
    end
    return damage
end

return sp_chips
