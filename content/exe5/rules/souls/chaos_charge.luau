--!strict
-- Chaos Unison's charge (EXE5's): its soul change arms it (./change, 0x0801216C) as the rules' own B charge (the
-- engine's `b_charge_time`, `b_charge_glow` and `b_charge_anim`, which a weapons' load drops: 0x0800DCD8). While it
-- is armed, a full B charge cycles through a window (0x080105F8, by the cycle's row: the chaos level's, at most 2,
-- or the soul's own), which the glow shows (0x080E0E10: animation 2 + the window); its time is the soul's chaos
-- weapon's by the chaos level (0x0801067E). Released in the window (0x0801086E: window 2), the chaos weapon fires
-- (0x080F034E: the chaos level up, at most 4; the side's statistic 0; `set_attack` slot 5); out of it, the failure
-- (0x080F0382: uninterruptible, ./chaos, slot 6). The soul's weapon and its own cycle are its form's `chaos`
-- (`Exe5FormFields`).

local chaos = require("./chaos")

local chaos_charge = {}

-- The rules' state fields the charge keeps (../init.luau declares them).
type State = {
    -- AIData+0x6C: the releases that succeeded, at most 4 (the change zeroes it).
    chaos_level: number,
    -- AIData+0x1C: ticks into the cycle while the B charge is full.
    chaos_counter: number,
    -- AIData+0x1F: where in the cycle the charge is: 2 a release now succeeds; 0 or 1 it fails.
    chaos_window: number,
    -- The release's outcome as it was released (0x0801086E's request: 0x8000 in the window, 0x10000 out of it).
    chaos_success: boolean,
}

-- The cycle (0x08010650), a row by the chaos level (at most 2): the period, then the bounds of its windows: a release
-- succeeds under the first (21, 12, 9 ticks into a full charge), then the window is 1 under the second, 0 under the
-- third, 1 past it.
local CYCLE: { { number } } = {
    { 42, 21, 12, 42 },
    { 36, 12, 9, 36 },
    { 27, 9, 9, 27 },
}

-- The armed charge's glow (0x080E0EA4).
local GLOW = asset.sprite("chaos-charge-glow")

-- The navi's soul's Chaos Unison: its weapon, and its own cycle (none: the level's row).
local function soul_chaos(side: number): { weapon: Weapon, cycle: { number }? }?
    return (battle.navi(side).form :: any).chaos
end

-- 0x0801067E: the time to a full charge, the soul's chaos weapon's by the chaos level (at most 4); no weapon, 0xFF.
local function push_time(navi: Object, s: State)
    local soul = soul_chaos(navi.alliance)
    if soul == nil then
        navi.b_charge_time = 0xFF
        return
    end
    local column = math.min(s.chaos_level, 4)
    local ticks = soul.weapon.charge_ticks[column + 1]
    if ticks == nil then
        error(string.format("content error: the chaos weapon has no charge time at chaos level %d", column))
    end
    navi.b_charge_time = ticks
end

-- 0x0801216C's end: the charge armed, at the chaos level the change left (0).
function chaos_charge.arm(navi: Object)
    local s = rules.state() :: State
    push_time(navi, s)
    navi.b_charge_glow = GLOW
    navi.b_charge_anim = 2 + s.chaos_window
end

-- The change's start: the chaos level back to 0.
function chaos_charge.reset_level()
    (rules.state() :: State).chaos_level = 0
end

-- 0x080105F8, the input's last step (the rules' `navi_tick` runs next to it): while the charge is armed, a full B
-- charge's counter runs through its row's period, and where it stands sets the window; otherwise it rests at 0.
-- (The input's steps run while the battle is on.)
function chaos_charge.navi_tick(side: number, navi: Object)
    if navi.b_charge_time == nil or battle.over() then
        return
    end
    local s = rules.state() :: State
    if navi.charge_source ~= 2 or navi.charge_level ~= 2 then
        s.chaos_counter = 0
        return
    end
    -- 0x080106BC: the level's row, at most 2, unless the soul has its own (MeddySoul's: the table's row 2).
    local soul = soul_chaos(side)
    local cycle = (soul and soul.cycle) or CYCLE[math.min(s.chaos_level, 2) + 1]
    local counter = (s.chaos_counter + 1) % 0x100
    if counter >= cycle[1] then
        counter = 0
    end
    local window = if counter < cycle[2] then 2 elseif counter < cycle[3] then 1 elseif counter < cycle[4] then 0 else 1
    s.chaos_counter = counter
    s.chaos_window = window
    navi.b_charge_anim = 2 + window
end

-- The rules' `charge_released`: 0x0801086E, a full armed charge released: in the window or out of it.
function chaos_charge.charge_released(_side: number, _navi: Object)
    local s = rules.state() :: State
    s.chaos_success = s.chaos_window == 2
end

-- The rules' `release_taken`: the navi out of idle, 0x080F034E (the chaos weapon) or 0x080F0382 (the failure).
function chaos_charge.release_taken(side: number, navi: Object)
    local s = rules.state() :: State
    if not s.chaos_success then
        navi:set_navi_state("uninterruptible", true)
        navi:set_attack(chaos.failure, 6)
        return
    end
    s.chaos_level = math.min(s.chaos_level + 1, 4)
    push_time(navi, s)
    -- (0x0800F338: the side's statistic 0 counts it.)
    battle.bump_side_stat(side, 0, 1)
    local soul = soul_chaos(side)
    if soul == nil then
        error("the soul has no chaos weapon: its routine 0xFF reads past the weapon routines' table")
    end
    navi:start_weapon(soul.weapon, 5)
end

return chaos_charge
