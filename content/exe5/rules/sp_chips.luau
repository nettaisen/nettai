--!strict
-- EXE5's SP navi chips' damage (0x0800E8DE, EXE6's `sub_8010AE4`): lower the slower the side's player deleted the
-- chip's SP navi, a step per two seconds past ten. A chip's damage by step is its `sp.by_time` (the rules extend
-- chips with it, ./init); the step is how many of the steps' times (0x0800E93C, EXE6's: BCD hours, minutes, seconds,
-- hundredths) the deletion time passes, which the side's setup states for the chip (`sp_times`: the save's
-- 0x0203E560, one a chip; none stated, deleted in no time). In the own-gauges mode (operation battle, battle flag
-- 0x40) it is the chip's `sp.operation_battle` instead. Roll's SP chip counts holy panels and Django's turns (their
-- damage is a count): neither goes by a time. An SP chip's `damage = sp_chips.damage`, which the round's setup asks
-- once for each side.

local sp_chips = {}

-- What an SP chip says of its damage (the rules' extension, ./init).
type Sp = { by_time: { number }, operation_battle: number? }

-- The deletion times at which an SP chip's damage steps down.
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

local function bcd(v: number): number
    return bit32.bor(bit32.lshift(v // 10, 4), v % 10)
end

-- `sub_8000D84`: frames as a BCD time, hours:minutes:seconds.hundredths (a byte each). (A deletion time is at most
-- 65535 frames, under the routine's cap of 99:59:59.99.)
local function time_bcd(frames: number): number
    local hours, rest = frames // 216000, frames % 216000
    local minutes = rest // 3600
    rest %= 3600
    local seconds, past = rest // 60, rest % 60
    return bit32.bor(
        bit32.lshift(bcd(hours), 24),
        bit32.lshift(bcd(minutes), 16),
        bit32.lshift(bcd(seconds), 8),
        bcd(past * 100 // 60)
    )
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
    local sp = (chip :: any).sp :: Sp
    if battle.own_gauges() and sp.operation_battle ~= nil then
        return sp.operation_battle
    end
    local time = time_bcd(frames(side, chip))
    local step = 0
    for _, t in sp_chips.DELETION_TIMES :: { number } do
        if time <= t then
            break
        end
        step += 1
    end
    local damage = sp.by_time[step + 1]
    if damage == nil then
        error(string.format("SP chip %s has no damage for deletion-time step %d (its `sp.by_time`)", tostring(chip.id), step))
    end
    return damage
end

return sp_chips
