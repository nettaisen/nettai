--!strict
-- EXE6's emotions (docs/design/rules-in-luau.md §3.1, S5): what in EXE6
-- makes a navi tired, Full Synchro on a counter, and the NaviCust
-- emotion-swing bug. The emotion's state is the framework's, which EXE5's
-- rules share (the mood, held tired or exhausted, anger and its timer, and
-- the order `sub_8015B54` reads them in); a game's rules decide when.

local emotion = {}

type State = {
    -- AIData+0x3A: ticks toward the next swing (every 60).
    swing_ticks: number,
    -- AIData+0x0B: the emotion the swing last rolled: 0 normal (mood 0x99),
    -- 1 tired, 2 angry, 3 Full Synchro (mood 0xFF).
    swung: number,
}

-- `byte_8013E44`: the emotions the swing rolls from (six normal, eight
-- tired, one angry, one Full Synchro), before the current one is swapped
-- out.
local SWINGS = { 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 2, 3 }
-- `dword_8013E54`: the mood a normal (0) or Full Synchro (3) swing sets.
local SWING_MOODS = { [0] = 0x99, [1] = 0x3C, [2] = 0x00, [3] = 0xFF }

-- `sub_8013892`'s part: a navi whose Beast Out counter is spent starts the
-- round tired (`sub_801443C`); the NaviCust's swing bug ticks with it.
local function round_start(side: number)
    local navi = battle.player(side)
    if navi == nil then
        return
    end
    local stats = battle.navi(side)
    if stats.beast_out_counter == 0 then
        navi.tired = true
    end
    if stats.emotion_bug ~= 0 then
        navi.ticked = true
    end
end

-- `sub_801A200`'s counter: the counterer gets Full Synchro in base form or
-- a plain Beast Out, unless the countered navi's mood is held (and the
-- counterer's own mood isn't: `battle.set_mood`).
local function countered(side: number, victim: Object)
    local form = battle.navi(side).form
    if (form.base or form.kind == "beast") and not victim.mood_held then
        battle.set_mood(side, 0xFF)
    end
end

-- `sub_8013DA0`: the NaviCust emotion-swing bug (stat 0x24), while the
-- Beast Out counter (stat 0x21) lasts: in a form it only keeps the navi
-- calm and untired; in base form, every 60 ticks the emotion swings to a
-- random other one (a roll of 16 from `SWINGS`, where the current emotion
-- counts as normal, or as tired when it is normal).
local function navi_tick(side: number, navi: Object)
    local stats = battle.navi(side)
    if stats.emotion_bug == 0 or stats.beast_out_counter == 0 then
        return
    end
    if not stats.form.base then
        navi:end_anger()
        navi.tired = false
        return
    end
    local s = system.state() :: State
    s.swing_ticks = (s.swing_ticks + 1) % 0x10000
    if s.swing_ticks < 0x3C then
        return
    end
    s.swing_ticks = 0
    navi:end_anger()
    -- sub_8014446
    navi.tired = false
    local current = s.swung
    local roll = SWINGS[battle.rng_positive() % #SWINGS + 1]
    local swung = if roll ~= current then roll elseif current == 0 then 1 else 0
    s.swung = swung
    if swung == 2 then
        -- sub_80143CE: anger, unless the mood is held.
        if not navi.mood_held then
            navi:set_request("anger", true)
        end
    elseif swung == 1 then
        -- sub_801443C
        navi.tired = true
    else
        battle.set_mood(side, SWING_MOODS[swung])
    end
end

emotion.system = define.system {
    id = "emotion",
    state = { swing_ticks = "u16", swung = "u8" },
    hooks = { round_start = round_start, countered = countered, navi_tick = navi_tick },
}

return emotion
