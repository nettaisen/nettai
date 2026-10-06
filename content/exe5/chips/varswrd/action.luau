--!strict
-- VarSwrd's action (EXE5's action 0x2A, 0x080ED4A2: EXE6's `sub_80EF62E`'s waiting, EXE5's own picks): the navi stands
-- holding A and the chip becomes another sword by the direction sequence entered (with A held, each step the whole
-- joypad word, EXE6's sequences): LongSwrd, FtrSword, WideSwrd, LifeSrd or SonicBom (0x080ED618). Letting go of A, or
-- 48 ticks without a sequence, gives a Sword (chip 0x30). The pick's action starts in place (`object_setAttack0`) as
-- that chip's, with its subtype and parameters and the original damage. In ProtoSoul or ShadowSoul (the forms with
-- `var_sword_waits`) the navi waits while A is up.
--
-- A navi in auto battle (NaviStats +0x2A, never set in a Team Battle, or AIData +0xF0) takes the side's sword
-- pick instead (0x0802D4E2's +0x12, which its AI draws: the picks by their place, one past the last a Sword). +0xF0
-- is set by 0x0802C110 alone: the story navis' routines and DarkInvs (0x080E231A, its user in auto battle for
-- 600 ticks), not Chaos Unison's Dark MegaMan's (AI index 0x16: 0x08104234), who takes the joypad path. NeoVari's
-- action is this one with its own sequences and picks.

local action = {}

export type State = {
    -- The navi's direction flip (which set of sequences: AIAttackVars+0x28), and how far into each sequence the navi
    -- is, in bytes (+0x30 up).
    flip: number,
    progress: { number },
}

action.STATE = { flip = "u8", progress = "u8[5]" }

-- Ticks to enter a sequence.
local COUNTDOWN = 0x30
-- The phases, on the attack's step.
local START, PICK = 0, 4
local SOUND = asset.sound("ok-8b")
-- The joypad bits, as the game's JoypadHeld word has them.
local KEYS: { { key: Key, bit: number } } = {
    { key = "a", bit = 0x001 },
    { key = "b", bit = 0x002 },
    { key = "select", bit = 0x004 },
    { key = "start", bit = 0x008 },
    { key = "right", bit = 0x010 },
    { key = "left", bit = 0x020 },
    { key = "up", bit = 0x040 },
    { key = "down", bit = 0x080 },
    { key = "r", bit = 0x100 },
    { key = "l", bit = 0x200 },
}

export type Spec = {
    id: string?,
    -- The sequences by direction flip (0x080ED4E4: facing right, then mirrored), as joypad words with A held, each
    -- ending in 0.
    sequences: { [number]: { { number } } },
    -- What the sequences give, in order, and what the countdown or A let go gives.
    picks: { Chip },
    sword: Chip,
}

-- The held buttons as a joypad word (JoypadHeld & 0x3FF).
local function held_word(me: Object): number
    local w = 0
    for _, k in KEYS do
        if me:held(k.key) then
            w += k.bit
        end
    end
    return w
end

-- `sub_8012792`'s counterpart: advance every sequence whose next step is `held`, last first; the first one already
-- complete is the pick.
local function advance(s: State, sequences: { { number } }, held: number): number?
    for i = #sequences, 1, -1 do
        local at = s.progress[i]
        local want = sequences[i][at // 2 + 1]
        if want == nil then
            error(string.format("sequence %d reads past its table at %d", i - 1, at))
        end
        if want == 0 then
            return i
        end
        if held == want then
            s.progress[i] = at + 2
        end
    end
    return nil
end

-- 0x080ED5D4 / 0x080ED5EA: the attack becomes `chip` (its action, subtype and parameters; the damage stays).
local function become(me: Object, chip: Chip)
    local a = chip.action
    if a == nil then
        error(string.format("VarSwrd's pick %s has no action", chip.id :: string))
    end
    me.attack_chip = chip
    me:set_attack(a, 0)
end

-- `sub_802C106`: AIData +0xF0 (the navi's `in_auto_battle`), which only DarkInvs's drive sets in a battle
-- (chips/darkinvs, rules/auto_battle). (NaviStats +0x2A, which the pick reads first, no Team Battle sets.)
local function in_auto_battle(me: Object): boolean
    return me.in_auto_battle
end

-- 0x080ED570 (NeoVari's 0x080EDF7C): a tick of waiting for the pick.
local function pick(me: Object, s: State, spec: Spec)
    if in_auto_battle(me) then
        local chosen = battle.sword_pick(me.alliance)
        if chosen == #spec.picks then
            return become(me, spec.sword)
        end
        local chip = spec.picks[chosen + 1]
        if chip == nil then
            error(string.format("the side's sword pick %d reads past the picks' table", chosen))
        end
        battle.play_sound(SOUND)
        return become(me, chip)
    end
    local left = me.attack_count - 1
    me.attack_count = left
    if left == 0 then
        return become(me, spec.sword)
    end
    local held = held_word(me)
    if not bit32.btest(held, 0x001) then
        -- (The souls that wait while A is up, by NaviStats +0x2C: ProtoSoul and ShadowSoul, their forms'
        -- `var_sword_waits`.)
        if battle.navi(me.alliance).form.var_sword_waits == true then
            return
        end
        return become(me, spec.sword)
    end
    local i = advance(s, spec.sequences[s.flip], held)
    if i == nil then
        return
    end
    battle.play_sound(SOUND)
    become(me, spec.picks[i])
end

-- 0x080ED4BC: 48 ticks, the sequences for the navi's flip, none begun; then the first pick.
local function start(me: Object, s: State, spec: Spec)
    me.attack_count = COUNTDOWN
    local flip = me.flip
    if spec.sequences[flip] == nil then
        error(string.format("direction flip %d reads past the sequence tables", flip))
    end
    s.flip = flip
    for i = 1, 5 do
        s.progress[i] = 0
    end
    me.marker = 0
    me.step = PICK
    me.step_init = 0
    pick(me, s, spec)
end

-- VarSwrd's action: its sequences and what they give.
function action.action(spec: Spec): Action
    -- 0x080ED4A2.
    local function update(me: Object, s: State)
        local step = me.step
        if step == START then
            start(me, s, spec)
        elseif step == PICK then
            pick(me, s, spec)
        else
            error(string.format("VarSwrd phase %#x reads past its phase table", step))
        end
    end

    return new.action { id = spec.id, state = action.STATE, args = spec, update = update }
end

return action
