--!strict
-- What VarSwrd (the original's action 0x53, `sub_80EF6FC`) and NeoVari
-- (0x54, `sub_80EF87C`) share: the navi stands holding A and the chip
-- becomes another sword by the direction sequence entered (with A held,
-- each step the whole joypad word); letting go of A or 48 ticks without a
-- sequence gives a plain Sword. In a Beast Over form, or as a special (the
-- attack's special source), the sword is random. The chosen chip's action
-- starts in place (`object_setAttack0`) as that chip's, with the original
-- damage; charged (SlashCross's A-charge), it starts as the charged slash
-- of the pick instead (navis/megaman/forms/slashcross/charge). Each
-- chip composes its own (`vari.action { ... }`, a `VariSwordSpec`): its
-- sequences, and the actions they give.

-- The state (`VariSwordState`, types.d.luau): the ticks left to enter a
-- sequence (AIAttackVars+0x12), the navi's direction flip (which set of
-- sequences: AIAttackVars+0x28), and how far into each sequence the navi
-- is, in bytes (AIAttackVars+0x30 up; the game keeps the first four in the
-- attack's marker word, which every later reader sets before reading).
type State = VariSwordState
type Spec = VariSwordSpec

local slash_charge = require("../../navis/megaman/forms/slashcross/charge")
local slashcross = require("../../navis/megaman/forms/slashcross/form")
local slashcross_beast = require("../../navis/megaman/forms/slashcross-beast/form")
local protoman = require("../../navis/protoman/navi")

local vari = {}

-- One state for both actions (the original's two handlers share the
-- attack variables' layout).
vari.STATE = { countdown = "u16", flip = "u8", progress = "u8[5]" }

-- Ticks to enter a sequence.
local COUNTDOWN = 0x30
-- The phases, on the attack's step.
local START, PICK = 0, 4
local SOUND = asset.sound("ok-8b")
-- The forms that wait while A is up: SlashCross and its Beast (the Beast
-- Overs pick at random, and the Beast forms give the pick's Beast
-- lock-on).
local WAITING_FORMS: { FormDef } = { slashcross, slashcross_beast }
-- The navi that waits while A is up and never gets the charged sword or
-- the lock-on.
local WAITING_NAVI: NaviDef = protoman
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

-- `sub_8012792`: advance every sequence whose next step is `held`, last
-- first; the first one already complete is the pick. (After a step the
-- game also tests the word at the new position for zero, reading it as an
-- unaligned word; in these tables it never is, so a sequence shows
-- complete on the tick after its last step.)
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

-- `loc_80EF788`: the attack becomes pick `i` (0: Sword). Its action is the
-- chip's, and the attack's chip the pick (the Beast rush reads the
-- attack's chip for its lock-on mode, and whether it rushes at all: the
-- chip's `beast`). A charged pick starts as SlashCross's charged slash of
-- it (unless it is the no-charge pick), which the game does by writing the
-- action over the one it just started. (The game also copies the pick's
-- subtype and parameters into the attack, which its charged slash and the
-- Beast rush read there: here the pick's slash names what they need.)
local function become(me: Object, spec: Spec, i: number)
    local chip = if i == 0 then spec.sword else spec.picks[i]
    local action = chip.action
    if action == nil then
        error(string.format("the variable sword's pick %s has no action", chip.id))
    end
    me.attack_chip = chip
    me:set_attack(action, 0)
    local lockon = if chip.beast ~= nil and chip.beast.rush ~= false then 1 else 0
    if me.special_source ~= 0 then
        me.beast_lockon = lockon
        return
    end
    local stats = battle.navi(me.alliance)
    if stats.navi == WAITING_NAVI then
        return
    end
    if chip ~= spec.no_charged and me.charged ~= 0 then
        me:set_attack(slash_charge.of_action(me, action), 0)
    end
    if stats.beast then
        me.beast_lockon = lockon
    end
end

-- `sub_80EF6FC` / `sub_80EF87C`: a tick of waiting for the pick.
local function pick(me: Object, s: State, spec: Spec)
    local stats = battle.navi(me.alliance)
    if stats.beast_over or me.special_source ~= 0 then
        local r = battle.rng_positive() % spec.random
        -- (Past the picks, the random draw is a Sword.)
        return become(me, spec, if spec.picks[r + 1] then r + 1 else 0)
    end
    local left = s.countdown - 1
    s.countdown = left
    if left == 0 then
        return become(me, spec, 0)
    end
    local held = held_word(me)
    if not bit32.btest(held, 0x001) then
        if stats.navi == WAITING_NAVI or table.find(WAITING_FORMS, stats.form) then
            return
        end
        return become(me, spec, 0)
    end
    local i = advance(s, spec.sequences[s.flip], held)
    if i == nil then
        return
    end
    battle.play_sound(SOUND)
    become(me, spec, i)
end

-- `sub_80EF648` / `sub_80EF7FC`: 48 ticks, the sequences for the navi's
-- facing, none begun; then the first pick.
local function start(me: Object, s: State, spec: Spec)
    s.countdown = COUNTDOWN
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

-- A variable sword: its sequences and what they give.
function vari.action(spec: Spec): Action
    -- `sub_80EF62E` / `sub_80EF7E2`: set up, then wait for the pick.
    local function update(me: Object, s: State)
        local step = me.step
        if step == START then
            start(me, s, spec)
        elseif step == PICK then
            pick(me, s, spec)
        else
            error(string.format("variable sword phase %#x reads past its phase table", step))
        end
    end

    return define.action { id = spec.id, state = vari.STATE, args = spec, update = update }
end

return vari
