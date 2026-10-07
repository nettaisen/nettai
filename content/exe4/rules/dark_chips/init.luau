--!strict
-- EXE4's dark chips (docs/design/exe4-map.md §5): never in a folder, offered on the custom screen to a dark MegaMan,
-- each costing its user as his hand takes it.
--
-- - **The offer** (0x0801DE90, from the screen's opening, 0x0801DD02, after its hand size and the round's count of
--   screens, before it deals): in a netbattle (battle type 0x46 and on: every EXE4 round of nettai's) to a MegaMan
--   worn out (emotion 5: a mood of 0, a dark MegaMan's from his start) alone. (Its other branches are the story's:
--   a worried MegaMan offered them too in a battle of type under 0x46, by its settings and the story's event flags,
--   0x0416; the story's dark chips, event flag 0x74, which a dark MegaMan's save has: no other way leads to a mood of
--   0 in a netbattle.) Two draws of the console's RNG1 pick two different chips of a list of 64 (0x0801DF38: the
--   table 0x0801DF9C's fifth, 0x0801ECB0, or with the navi's HP at half its maximum or less the sixth, 0x0801ECF0),
--   which the screen shows in slots 8 and 9 in the code outside the alphabet (27), flagged 0x21: the second only
--   while the side has sent fewer than nine this battle, neither from ten (0x0203F6D0, which the battle's start
--   zeroes, 0x0801592C), the draws drawn all the same. The cursor starts on slot 8 (0x0801E110). An offered dark chip
--   picked goes in the hand as any; OK counts it sent (0x0801F0FA: 0x0801EF7C, its flag 0x20).
-- - **The costs** (0x0801EA1E, as each side's hand is installed, 0x0801E9AC and 0x0801E9CC; each chip of the hand once,
--   its hand entry's flag 0x04): by the table 0x0801EAF4, a byte of the user's NaviStats for each of chips 0xBB to
--   0xC2, through 0x0801EA9A's rules for three of them; DrkVulcn's (0x40) a status on its user (0x080134C4: 0x21).
-- - **The mood**: none. The offer asks a mood of 0, which no setter changes (0x0800F4DE and the others leave it), and
--   neither the pick nor the use touches it.

local drkcanon = require("../chips/drkcanon").drkcanon
local drksword = require("../chips/drksword").drksword
local darkbomb = require("../chips/darkbomb").darkbomb
local drkvulcn = require("../chips/drkvulcn").drkvulcn
local drklance = require("../chips/drklance").drklance
local drkspred = require("../chips/drkspred").drkspred
local drkstage = require("../chips/drkstage").drkstage
local drkrecov = require("../chips/drkrecov").drkrecov
local status = require("./status")
local taunt = require("../navis/megaman/weapons/taunt")

local dark_chips = {}

-- What a side keeps of them through a round.
export type State = {
    -- Dark chips sent this battle (0x0203F6D0).
    dark_chips_sent: number,
    -- This screen's picks so far, and which of them were offered (a bit each, by pick).
    picks: number,
    offered_picks: number,
    -- OK sent a hand (a pick or more): it is installed as the screen closes, and its chips cost.
    hand_sent: boolean,
}
dark_chips.state = {
    dark_chips_sent = "u8",
    picks = "u8",
    offered_picks = "u16",
    hand_sent = "bool",
}

-- The slots the offer fills (0x0801DF84, 0x0801DF88: the block's +6 and +8, slots 8 and 9).
local SLOTS: { number } = { 8, 9 }
-- The dark chips a side may be sent in a battle (0x0801DF7A), and the count past which the second slot stays empty.
local MAX_SENT, ONE_LEFT = 10, 9

-- The lists of 64 a netbattle draws from (0x0801ECB0, 0x0801ECF0), as runs of each chip.
type Run = { chip: Chip, n: number }
local function list(runs: { Run }): { Chip }
    local out: { Chip } = {}
    for _, r in runs do
        for _ = 1, r.n do
            table.insert(out, r.chip)
        end
    end
    assert(#out == 64, "a dark chip list of 64")
    return out
end
local HEALTHY = list {
    { chip = drksword, n = 12 },
    { chip = darkbomb, n = 12 },
    { chip = drkvulcn, n = 12 },
    { chip = drklance, n = 12 },
    { chip = drkspred, n = 8 },
    { chip = drkstage, n = 8 },
}
local HURT = list {
    { chip = drkcanon, n = 24 },
    { chip = drksword, n = 3 },
    { chip = darkbomb, n = 3 },
    { chip = drkvulcn, n = 3 },
    { chip = drklance, n = 3 },
    { chip = drkspred, n = 2 },
    { chip = drkstage, n = 2 },
    { chip = drkrecov, n = 24 },
}

-- 0x0801DF5E: a draw of the side's console's RNG1, its low six bits an entry of the list.
local function draw(side: number, chips: { Chip }): Chip
    return chips[bit32.band(battle.console_rng_positive(side), 63) + 1]
end

-- 0x0801DE90: the offer, as the screen deals.
function dark_chips.deal(side: number)
    local s = rules.state() :: State
    s.picks, s.offered_picks, s.hand_sent = 0, 0, false
    if not battle.link() or custom.player(side).emotion ~= "worn_out" then
        return
    end
    local navi = battle.player(side)
    if navi == nil then
        return
    end
    -- 0x0801DF4C: the list by the navi's HP against half its maximum.
    local chips = if navi.hp * 2 <= navi.max_hp then HURT else HEALTHY
    local first = draw(side, chips)
    local second = draw(side, chips)
    while second == first do
        second = draw(side, chips)
    end
    if s.dark_chips_sent >= MAX_SENT then
        return
    end
    custom.offer(side, SLOTS[1], first)
    if s.dark_chips_sent ~= ONE_LEFT then
        custom.offer(side, SLOTS[2], second)
    end
end

-- A chip picked: whether it was offered (its slot one of the offer's).
function dark_chips.chip_picked(side: number, _chip: Chip)
    local s = rules.state() :: State
    local pick = custom.last_pick(side)
    if pick and table.find(SLOTS, pick.slot) then
        s.offered_picks = bit32.bor(s.offered_picks, bit32.lshift(1, s.picks))
    end
    s.picks += 1
end

-- The last pick taken back.
function dark_chips.chip_taken_back(_side: number, _chip: Chip)
    local s = rules.state() :: State
    s.picks -= 1
    s.offered_picks = bit32.band(s.offered_picks, bit32.bnot(bit32.lshift(1, s.picks)))
end

-- 0x0801F0FA: OK sends the hand; each offered dark chip in it counts (0x0801EF92).
function dark_chips.confirmed(_side: number)
    local s = rules.state() :: State
    for i = 0, s.picks - 1 do
        if bit32.btest(s.offered_picks, bit32.lshift(1, i)) then
            s.dark_chips_sent += 1
        end
    end
    s.hand_sent = s.picks > 0
end

-- NaviStats as the costs write them.
type Stats = Navi

-- 0x0801EA9A: a drain's period by what it was: none, `first`; up to `first`, `second`; else 3.
local function worsened(was: number, first: number, second: number): number
    if was == 0 then
        return first
    elseif was <= first then
        return second
    end
    return 3
end

-- 0x0801EAF4: each chip's cost (its NaviStats byte and value, 0x0801EA9A's rules), by chip.
local COSTS: { [Chip]: (side: number, stats: Stats) -> () } = {
    -- +0x0A: the charged shot becomes weapon routine 0x21, the taunt, which angers the other side's navi.
    [drkcanon] = function(_side: number, stats: Stats)
        stats.charge_shot_weapon = taunt.weapon
    end,
    -- +0x0D: the move bug, the keys the navi steps by with none held (0x0800B4D8: effects.steps.bug): right, left.
    [drksword] = function(_side: number, stats: Stats)
        stats.move_bug = 0x10
    end,
    [darkbomb] = function(_side: number, stats: Stats)
        stats.move_bug = 0x20
    end,
    -- 0x40: a status on its user (0x080134C4's 0x21: 720 ticks of confusion).
    [drkvulcn] = function(side: number, _stats: Stats)
        local navi = battle.player(side)
        if navi then
            navi:take_status(status.confuse_720)
        end
    end,
    -- +0x0F: the custom screen's drain, 6, else 4 up to 6, else 3.
    [drklance] = function(_side: number, stats: Stats)
        stats.custom_drain = worsened(stats.custom_drain, 6, 4)
    end,
    -- +0x1B: a step leaves a poison panel (the panel trail, 4: effects.panel_trail).
    [drkspred] = function(_side: number, stats: Stats)
        stats.panel_trail = 4
    end,
    -- +0x12: the custom level one less, down to 2.
    [drkstage] = function(_side: number, stats: Stats)
        if stats.custom_level > 2 then
            stats.custom_level -= 1
        end
    end,
    -- +0x0F as DrkLance's, then +0x0E (0x0801EB04): the HP drain, 10, else 6 up to 10, else 3.
    [drkrecov] = function(_side: number, stats: Stats)
        stats.custom_drain = worsened(stats.custom_drain, 6, 4)
        stats.hp_drain = worsened(stats.hp_drain, 10, 6)
    end,
}

-- 0x0801EA1E: as the side's hand is installed, each of its chips costs its user once.
function dark_chips.custom_result(side: number)
    local s = rules.state() :: State
    if not s.hand_sent then
        return
    end
    s.hand_sent = false
    local stats = battle.navi(side)
    local i = 0
    local chip = battle.hand_chip(side, i)
    while chip do
        local cost = COSTS[chip]
        if cost then
            cost(side, stats)
        end
        i += 1
        chip = battle.hand_chip(side, i)
    end
end

return dark_chips
