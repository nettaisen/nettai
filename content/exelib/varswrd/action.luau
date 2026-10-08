--!strict
-- VarSwrd's and NeoVari's action (EXE5's actions 0x2A, 0x080ED4A2, and 0x31; EXE6's `sub_80EF62E`'s waiting, EXE5's
-- own picks; EXE4's actions 0x36, 0x080ED6BE, and 0x5A, 0x080EE1A2, from which EXE5's grew), made for a game
-- (`action.make`): the navi stands holding A and the chip becomes another sword by the direction sequence entered
-- (with A held, each step the whole joypad word). Letting go of A, or 48 ticks without a sequence, gives a Sword (chip
-- 0x30). The pick's action starts in place (`object_setAttack0`) as that chip's, with its subtype and parameters and
-- the original damage; a sequence's pick plays the game's sound. In the souls the game names (`waits`: EXE5's
-- ProtoSoul and ShadowSoul, EXE4's ProtoSoul) the navi waits while A is up. A navi in auto battle takes the side's
-- sword pick instead, where the game has one (`auto_pick`: EXE5's).
--
-- The sequences are by the navi's direction flip (EXE5's facing right, then mirrored; EXE4 has no flip: its one set),
-- each ending in 0. The match (EXE5's `sub_8012792` counterpart, EXE4's 0x0800D486) advances every sequence whose
-- next step is the held word, last first; the first one already complete is the pick, so a sequence is picked the
-- tick after its last step. (EXE4's also tests, as a step matches, the word at the sequence's next step, 0x0800D4A6,
-- and picks at once at 0: an unaligned read with the steps before it, or an aligned one with the step after it,
-- never 0 in VarSwrd's and NeoVari's tables.)

local action = {}

export type State = {
    -- The navi's direction flip (which set of sequences: EXE5's AIAttackVars+0x28, EXE4's +0x1C), and how far into
    -- each sequence the navi is, in bytes (EXE5's +0x30 up, EXE4's +0x20 up: the marker word and the next).
    flip: number,
    progress: { number },
}

action.STATE = { flip = "u8", progress = "u8[5]" }

-- Ticks to enter a sequence.
local COUNTDOWN = 0x30
-- The phases, on the attack's step.
local START, PICK = 0, 4
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

-- Advance every sequence whose next step is `held`, last first; the first one already complete is the pick.
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

-- The attack becomes `chip` (its action, subtype and parameters; the damage stays).
local function become(me: Object, chip: Chip)
    local a = chip.action
    if a == nil then
        error(string.format("VarSwrd's pick %s has no action", chip.id :: string))
    end
    me.attack_chip = chip
    me:set_attack(a, 0)
end

-- A game's VarSwrd-like action: `spec`'s sequences and picks, `game`'s sound and souls.
function action.make(spec: VarSwordSpec, game: VarSwordGame): Action
    -- EXE5's 0x080ED570 (NeoVari's 0x080EDF7C), EXE4's 0x080ED742 (NeoVari's 0x080EE20C): a tick of waiting for the
    -- pick.
    local function pick(me: Object, s: State)
        local auto_pick = game.auto_pick
        local chosen = if auto_pick then auto_pick(me) else nil
        if chosen ~= nil then
            if chosen == #spec.picks then
                return become(me, spec.sword)
            end
            local chip = spec.picks[chosen + 1]
            if chip == nil then
                error(string.format("the side's sword pick %d reads past the picks' table", chosen))
            end
            battle.play_sound(game.sound)
            return become(me, chip)
        end
        local left = me.attack_count - 1
        me.attack_count = left
        if left == 0 then
            return become(me, spec.sword)
        end
        local held = held_word(me)
        if not bit32.btest(held, 0x001) then
            if game.waits(me) then
                return
            end
            return become(me, spec.sword)
        end
        local i = advance(s, spec.sequences[s.flip], held)
        if i == nil then
            return
        end
        battle.play_sound(game.sound)
        become(me, spec.picks[i])
    end

    -- EXE5's 0x080ED4BC, EXE4's 0x080ED6DC: 48 ticks, the sequences for the navi's flip, none begun; then the first
    -- pick.
    local function start(me: Object, s: State)
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
        pick(me, s)
    end

    -- EXE5's 0x080ED4A2, EXE4's 0x080ED6BE.
    local function update(me: Object, s: State)
        local step = me.step
        if step == START then
            start(me, s)
        elseif step == PICK then
            pick(me, s)
        else
            error(string.format("VarSwrd phase %#x reads past its phase table", step))
        end
    end

    return new.action { id = spec.id, state = action.STATE, args = spec, update = update }
end

return action
