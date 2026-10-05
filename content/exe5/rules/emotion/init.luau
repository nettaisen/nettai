--!strict
-- EXE5's emotions (docs/design/exe5-map.md §15.10). The emotion itself is the framework's by EXE5's rules (the status
-- section's `emotions`: 0x08012740's order, 0x080127D6's setter, the anger tick); the starting mood and the palette
-- go by the light/dark value (rules/light-dark). What EXE5 decides here:
--
-- - **Full Synchro on a counter** (0x08016FDC, EXE6's `sub_801A200`): the counterer's side, in no soul (NaviStats
--   +0x2C 0), to mood 0xFF through EXE5's setter, which leaves a mood of 0 (a dark MegaMan's) as it is.
-- - **The emotion-swing bug** (0x080113F8, EXE6's `sub_8013DA0`, from the tick of every navi of the player's kind,
--   0x080EAD6A; an auto-battling navi's tick, 0x080F224C, has none): while the side's NaviStats +0x24 is set, out of battle
--   flag 0x40's mode and out of a soul, every 60 ticks the anger ends and the mood goes back to 0x80 (0x08011A94), then
--   the emotion swings to one of the table's (0x0801147C: seven normal, seven worried, one angry, one Full Synchro) but
--   the one it last swung to (every entry of it taken out), drawn from RNG2: angry asks for anger (0x08011ABA: the
--   request, the mood not consulted), any other sets its mood (0x0801148C) through EXE5's setter. A curse (BugCurse) can
--   set the bug mid-round, so every navi of the player's kind is ticked.

local operation_battle = require("../lib/operation_battle")

local emotion = {}

-- AIData +0x3A (ticks toward the next swing) and +0x0B (the emotion last swung to) of each navi of the player's kind,
-- by its slot on its side's list of alive actors (the player's the first).
type State = {
    swing_ticks: { number },
    swung: { number },
}

-- 0x0801147C: the emotions the swing draws from (0 normal, 1 worried, 2 angry, 3 Full Synchro).
local SWINGS = { 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 2, 3 }
-- 0x0801148C: the mood each sets (angry's unused).
local SWING_MOODS = { [0] = 0x99, [1] = 0x3F, [2] = 0x00, [3] = 0xFF }
local ANGRY = 2

-- The navi's slot on its side's list (1 to 4).
local function slot(side: number, navi: Object): number
    for i = 1, 4 do
        if battle.alive_actor_slot(side, i) == navi then
            return i
        end
    end
    error("a navi of the player's kind on no slot of its side's list")
end

-- 0x080113F8 (the framework skips it while paused). An auto-battling navi's tick reaches the side's systems too: it has no
-- swing.
local function swing(side: number, navi: Object)
    if navi.actor_type ~= "player" then
        return
    end
    if operation_battle() then
        return
    end
    local stats = battle.navi(side)
    if not stats.form.base or stats.emotion_bug == 0 then
        return
    end
    local s = system.state() :: State
    local i = slot(side, navi)
    local ticks = (s.swing_ticks[i] + 1) % 0x10000
    s.swing_ticks[i] = ticks
    if ticks < 60 then
        return
    end
    s.swing_ticks[i] = 0
    navi:end_anger()
    local current = s.swung[i]
    local others = {}
    for _, e in SWINGS do
        if e ~= current then
            table.insert(others, e)
        end
    end
    local swung = others[battle.rng_positive() % #others + 1]
    s.swung[i] = swung
    if swung == ANGRY then
        navi:set_request("anger", true)
    else
        battle.set_mood(side, SWING_MOODS[swung])
    end
end

emotion.system = define.system {
    id = "emotion",
    state = { swing_ticks = "u16[4]", swung = "u8[4]" },
    hooks = {
        -- The swing runs from the navi's tick, whose bug a curse may set at any time.
        round_start = function(side: number)
            local navi = battle.player(side)
            if navi ~= nil then
                navi.ticked = true
            end
        end,
        countered = function(side: number, _victim: Object)
            if battle.navi(side).form.base then
                battle.set_mood(side, 0xFF)
            end
        end,
        navi_tick = swing,
    },
}

return emotion
