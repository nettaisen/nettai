--!strict
-- TomahawkMan's charged attack (`tomahawkman/charge`): the original's
-- weapon routine 0x45 (`sub_8012258`) and his action table's entry 9
-- (`sub_80F17C4`). He raises his axe (30 ticks) and brings it down (30
-- ticks): on the swing's 10th tick a slash over the two columns ahead,
-- with its effect; then 30 ticks. (Routine 0x46, `sub_8012278`, which no
-- navi's row names, starts the same action with 10 damage and 6-tick
-- waits.)

local collision = require("../../rules/collision")
local link = require("../../lib/link_chips")
local regions = require("@exelib/regions")
local weapon = require("../../lib/weapon")

export type State = {
    timer: number,
    -- The ticks he raises the axe for and recovers for (the original's
    -- first and second attack parameters).
    raise: number,
    recover: number,
}

local charge = {}

local RAISE, SWING, RECOVER = 0x0, 0x4, 0x8
local ANIM_RAISE, ANIM_SWING, ANIM_RECOVER = 0xF, 0x10, 7
local SWING_TICKS = 0x1E
-- The swing's timer when the slash lands.
local SLASH_AT = 0x14
-- The waits routine 0x45 gives.
local RAISE_TICKS, RECOVER_TICKS = 0x1E, 0x1E

local SOUND = asset.sound("tomahawk-man")

-- The hits' counter byte, and the slash's element: Wood (the attack's
-- element byte).
local HIT_PARAM = 0x8A
local WOOD = 4

-- The slash's hit modifier, and its effect 16 pixels up.
local HIT_MOD = 1
local SLASH = define.effect { sprite = asset.sprite("big-slash"), anim = 1, palette = 0 }
local SLASH_Z = 0x10_0000

-- `sub_8012258`: 100 damage and 20 more per point of buster Attack (up to
-- 5).
local function setup(navi: Object): Action
    navi.attack_damage = weapon.charge_damage(navi, 0x64, 0x14)
    navi.hit_param = HIT_PARAM
    navi.attack_element = WOOD
    local s = navi:action_state(charge.action) :: State
    s.raise, s.recover = RAISE_TICKS, RECOVER_TICKS
    return charge.action
end

-- The slash's hit over the two columns ahead (its damage word plus the
-- Atk+ bonus), and its effect on the panel ahead.
local function slash(me: Object)
    local x, y = me.panel_x + me.facing, me.panel_y
    local word = int.u32(link.damage_word(me) + me.extra)
    battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = me.attack_element,
        z = 0,
        region = regions.wide_long,
        hit_effect = nil,
        target = collision.nothing,
        self_type = collision.slash,
        damage = bit32.band(word, 0xFFFF),
        stamina = bit32.rshift(word, 16),
        hit_mod = HIT_MOD,
    })
    local cx, cy = field.center(x, y)
    battle.effect(Vec3.new(cx, cy, SLASH_Z), SLASH, link.flip(me))
end

-- `sub_80F17E4`: the axe goes up.
local function raise(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = ANIM_RAISE
        s.timer = s.raise
    elseif link.run_out(s) then
        link.go(me, SWING)
    end
end

-- `sub_80F1808`: the swing; 30 ticks, the slash at 20 left.
local function swing(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = ANIM_SWING
        battle.play_sound(SOUND)
        s.timer = SWING_TICKS
        return
    end
    local t = s.timer - 1
    s.timer = t
    if t == SLASH_AT then
        slash(me)
    elseif t <= 0 then
        link.go(me, RECOVER)
    end
end

-- `sub_80F1888`: the axe comes back; then idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = ANIM_RECOVER
        s.timer = s.recover
    elseif link.run_out(s) then
        me:exit_attack()
    end
end

local PHASES: { [number]: (me: Object, s: State) -> () } = { [RAISE] = raise, [SWING] = swing, [RECOVER] = recover }

charge.action = define.action {
    id = "tomahawkman/charge/action",
    state = { timer = "u16", raise = "u8", recover = "u8" },
    -- `sub_80F17C4` (`off_80F17D8`).
    update = function(me: Object, s: State)
        link.run(me, s, PHASES, "TomahawkMan's charge (sub_80F17C4)")
    end,
}

charge.weapon = define.weapon {
    id = "tomahawkman/charge",
    -- The charge table's row 0x45, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 140, 130, 120, 115, 110, 0, 0, 0 },
    setup = setup,
}

return charge
