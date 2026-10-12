--!strict
-- EXE4's dark soul (soul 15, navis/megaman/forms/darksoul; docs/design/exe4-map.md §18 item 74): a dark MegaMan whose
-- HP a hit empties holds at 1 HP, once a battle, and changes into it (the souls' change, ./change, with DarkMega's
-- image); for 960 ticks no hit reaches him and the auto battle AI drives him on his own player's auto battle data
-- (../auto_battle), then he reverts (the souls' revert).
--
-- - **The trigger** (0x08013AB0 in the player's damage, the rules' `hp_emptied`: 0x0800EBC8): not yet this battle
--   (AIData +0x1B), a navi with forms (NaviStats +0x23 0: MegaMan) whose emotion is the worn-out one (0x0800F49C's 5:
--   a mood of 0, a dark MegaMan's, out of a soul and unangry). Then the dark soul is asked for (AIData +0x44's
--   0x1000) and his HP is 1. (Also tested: NaviStats +0x26, a story navi's, and an object of the battle's enemy list
--   whose +0x0F is 224 to 227, 0x08007CE6: none in a netbattle. And the side's statistic 1 is set, 0x08008020, which
--   the karma after a story's battle reads, 0x0800F5E2: a battle type under 70.)
-- - **The change** starts as any soul's, from the status routine, the request taken for soul 15 whatever the hand's
--   next chip (0x0800B8B0). Its own: the hand stays as it is (0x080EBC8E), 960 ticks to run (0x0800B93C: 0x08018088's
--   row 15), and its status reset (0x0800E17E: the form's `status_reset`, untouchable; the mood's setter with 0,
--   which finds it 0; the AI's block zeroed).
-- - **In it**: each tick of the intake its ticks run down, and at 0 the revert is asked for (0x0800B6A4); he is
--   invisible and flashing, three ticks at a time (0x0800C43C: the flash's timer and the flag); his last 90 ticks he
--   pulses blue (0x0800BB18, the rules' `navi_shader`); the B button does nothing and no charge counts (the form's
--   `ignores_b`), but A uses the hand's chips still, each use zeroing the AI's block (0x080EED38); where his idle
--   would read the pad for a step the AI decides (0x080EEC78, the rules' `controller`), its counts taken each tick
--   (0x080EAED2) and the charge it shows leveled as the input levels any (0x0800BB50; the rules' `navi_tick`).

local ai = require("../auto_battle/ai")
local change = require("./change")

local dark = {}

-- What a side keeps of it through a round.
export type State = {
    -- AIData +0x44's 0x1000: the dark soul asked for; +0x1B: it was, this battle; +0x2C: its ticks left.
    dark_soul_asked: boolean,
    dark_soul_spent: boolean,
    dark_soul_ticks: number,
    -- The AI's (../auto_battle/ai).
    auto_battle: any,
}

dark.state = {
    dark_soul_asked = "bool",
    dark_soul_spent = "bool",
    dark_soul_ticks = "u16",
    auto_battle = ai.STATE,
} :: StateSchema

-- Its ticks (0x08018088's row 15), the last of them it pulses through (0x0800BB24), and the pulse's blue by the
-- battle's ticks (0x0800BB40).
local TICKS, PULSING = 960, 90
local PULSE: { number } = { 0x04, 0x08, 0x0C, 0x10, 0x14, 0x18, 0x1C, 0x1F, 0x1F, 0x1C, 0x18, 0x14, 0x10, 0x0C, 0x08, 0x04 }
-- How long each tick's flash lasts (0x0800C44A), and a charge's levels by its count (0x0800BB66: charging from 10,
-- full at the AI's 40).
local FLASH_TICKS = 3
local CHARGING_AT = 10

local function state(): State
    return rules.state() :: State
end

-- The side's navi's dark soul (its `forms.souls.dark_soul`): none for a navi with no forms.
local function form_of(side: number): FormDef?
    local forms: { [string]: any }? = battle.navi(side).navi.forms
    local souls = if forms ~= nil then forms.souls else nil
    return if souls ~= nil then souls.dark_soul else nil
end

-- The side's navi is in it.
function dark.united(side: number): boolean
    local form = form_of(side)
    return form ~= nil and battle.navi(side).form == form
end

-- The dark soul's change, its form's `change` (the souls' change with DarkMega's image): the request taken as it
-- starts (0x080EBA8A), its ticks set as he emerges (0x080EBBC6) and the AI's counts taken from then on, the AI's
-- block zeroed at the status reset (0x0800E18E), the hand left as it is (0x080EBC8E).
dark.change = change.make {
    id = "darksoul",
    keeps_hand = true,
    prepare = function(_navi: Object)
        state().dark_soul_asked = false
    end,
    emerged = function(navi: Object)
        state().dark_soul_ticks = TICKS
        navi.ticked = true
    end,
    reset = function(_navi: Object)
        ai.reset(state().auto_battle)
    end,
}

-- 0x0800EBC8 (the rules' `hp_emptied`): whether the navi holds.
function dark.hp_emptied(side: number, navi: Object): boolean
    local s = state()
    local form = form_of(side)
    if s.dark_soul_spent or form == nil or battle.player(side) ~= navi or battle.emotion(side) ~= "worn_out" then
        return false
    end
    s.dark_soul_spent = true
    s.dark_soul_asked = true
    navi.hp = 1
    navi:ask_form_change(form)
    return true
end

-- 0x0800B8B0's part, each tick of the intake that asks for a change (./init's `navi_intake`): asked for, the dark soul
-- is the change asked, whatever the hand's next chip.
function dark.asked(side: number, navi: Object)
    local form = form_of(side)
    if state().dark_soul_asked and form ~= nil then
        navi:ask_form_change(form)
    end
end

-- 0x0800B6A4, the intake's part in the dark soul: its ticks run down, and their end asks for the revert.
function dark.countdown(navi: Object)
    local s = state()
    if s.dark_soul_ticks == 0 then
        return
    end
    s.dark_soul_ticks -= 1
    if s.dark_soul_ticks == 0 then
        navi:set_request("revert_form", true)
    end
end

-- 0x0800C43C, each tick of the intake after the souls' part: in the dark soul no hit reaches him for three ticks
-- more.
function dark.navi_intake(side: number, navi: Object)
    if dark.united(side) and battle.player(side) == navi then
        navi:set_status_timer("flash", FLASH_TICKS)
        navi:set_status("invisible", true)
    end
end

-- 0x0800BB18 (the rules' `navi_shader`): his last ticks' pulse.
function dark.navi_shader(side: number, navi: Object): number?
    if battle.player(side) ~= navi then
        return nil
    end
    local ticks = state().dark_soul_ticks
    if ticks < 1 or ticks > PULSING then
        return nil
    end
    return bit32.lshift(PULSE[bit32.band(battle.battle_time(), 15) + 1], 10)
end

-- Each tick after the navi's input (the rules' `navi_tick`, which the change asks for as he emerges): the charge the
-- AI shows takes its level from its count (0x0800BB50, which levels every navi's each tick; a navi out of the dark
-- soul has its level from the engine's count), then the AI's counts (0x0800E9E0). Out of the dark soul it asks no
-- more.
function dark.navi_tick(side: number, navi: Object)
    if not dark.united(side) then
        navi.ticked = false
        return
    end
    local count = navi.charge_counter
    navi.charge_level = if count >= ai.CHARGE_FULL then 2 elseif count >= CHARGING_AT then 1 else 0
    ai.tick(navi, state().auto_battle)
end

-- 0x080EEC78 (the rules' `controller`, where the idle would read the pad for a step): in the dark soul the AI
-- decides.
function dark.controller(side: number, navi: Object): "nothing"?
    if not dark.united(side) or battle.player(side) ~= navi then
        return nil
    end
    ai.run(navi, state().auto_battle)
    return "nothing"
end

-- 0x080EED38 (the rules' `chip_used`): a chip's use from idle, the hand's, zeroes the AI's block, whatever the form.
function dark.chip_used(_side: number)
    ai.reset(state().auto_battle)
end

return dark
