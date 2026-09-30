--!strict
-- What VarSwrd (action 0x53, `sub_80EF6FC`) and NeoVari (0x54,
-- `sub_80EF87C`) share: the navi stands holding A and the chip becomes
-- another sword by the direction sequence entered (with A held, each step
-- the whole joypad word); letting go of A or 48 ticks without a sequence
-- gives a plain Sword. In a Beast Over form, or as a special (the attack's
-- special source), the sword is random. The chosen chip's action starts
-- in place (`object_setAttack0`) with its subtype and parameters but the
-- original damage.

-- The state (`VariSwordState`, types.d.luau): the ticks left to enter a
-- sequence (AIAttackVars+0x12), the navi's direction flip (which set of
-- sequences: AIAttackVars+0x28), and how far into each sequence the navi
-- is, in bytes (AIAttackVars+0x30 up; the game keeps the first four in the
-- attack's marker word, which every later reader sets before reading).
-- What differs between the two actions is a `VariSwordSpec`.
type State = VariSwordState
type Spec = VariSwordSpec

-- The state schema both actions declare.
local STATE = { countdown = "u16", flip = "u8", progress = "u8[5]" }

local vari_sword = { STATE = STATE }

local SWORD = 0x47
-- Ticks to enter a sequence.
local COUNTDOWN = 0x30
-- The charged sword a charged pick turns into (unless it is the no-charge
-- chip).
local CHARGED_SWORD = 0x41
-- Forms: SlashCross and its Beast (wait while A is up), the Beast Overs
-- (random), and the Beast forms (0xB to 0x18: the pick's Beast lock-on).
local SLASH_CROSS, SLASH_CROSS_BEAST = 0x03, 0x0F
local GREGAR_BEAST_OVER, FALZAR_BEAST_OVER = 0x17, 0x18
local FIRST_BEAST, LAST_BEAST = 0x0B, 0x18
-- The navi (NaviStats+0x29) that waits while A is up and never gets the
-- charged sword or the lock-on.
local WAITING_NAVI = 0x0B
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

-- `loc_80EF788`: the attack becomes `chip`.
local function become(me: Object, chip: number, spec: Spec)
    me.chip = chip
    local c = data.chips[chip]
    if c == nil then
        error(string.format("the variable sword's chip %#x is not in the content", chip))
    end
    me.variant = c.subtype
    for n = 1, 4 do
        me:set_attack_param(n, c.params[n])
    end
    me:set_attack(c.action, 0)
    local lockon = if c.beast_lockon then 1 else 0
    if me.special_source ~= 0 then
        me.beast_lockon = lockon
        return
    end
    local stats = battle.navi(me.alliance)
    if stats.navi == WAITING_NAVI then
        return
    end
    if chip ~= spec.no_charged and me.charged ~= 0 then
        me.action = CHARGED_SWORD
    end
    if stats.form >= FIRST_BEAST and stats.form <= LAST_BEAST then
        me.beast_lockon = lockon
    end
end

-- `sub_80EF648` / `sub_80EF7FC`: 48 ticks, the sequences for the navi's
-- facing, none begun; then the first pick.
function vari_sword.start(me: Object, s: State, spec: Spec)
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
    me.step = 4
    me.step_init = 0
    vari_sword.pick(me, s, spec)
end

-- `sub_80EF6FC` / `sub_80EF87C`: a tick of waiting for the pick.
function vari_sword.pick(me: Object, s: State, spec: Spec)
    local stats = battle.navi(me.alliance)
    local form = stats.form
    if form == GREGAR_BEAST_OVER or form == FALZAR_BEAST_OVER or me.special_source ~= 0 then
        local r = battle.rng_positive() % spec.random
        return become(me, spec.choices[r + 1] or SWORD, spec)
    end
    local left = s.countdown - 1
    s.countdown = left
    if left == 0 then
        return become(me, SWORD, spec)
    end
    local held = held_word(me)
    if not bit32.btest(held, 0x001) then
        if stats.navi == WAITING_NAVI or form == SLASH_CROSS or form == SLASH_CROSS_BEAST then
            return
        end
        return become(me, SWORD, spec)
    end
    local i = advance(s, spec.sequences[s.flip], held)
    if i == nil then
        return
    end
    battle.play_sound(0x8B)
    become(me, spec.choices[i], spec)
end

return vari_sword
