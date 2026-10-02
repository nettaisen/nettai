--!strict
-- The charged shot of the patch card count (`patch-cards/count/charge`):
-- Count's rain. The original's weapon routine 0x6D (`sub_801244A`) sets up
-- action 0x5E (`sub_80F020E`): the navi raises a dust storm
-- (lib/instant/dust_storm) over the middle of the other side's area, 20
-- damage a hit, for 60 ticks or until the navi leaves the action, and
-- shakes in place while it blows, with the rain's sound every 17 ticks.

local dust_storm = require("../../lib/instant/dust_storm")

local charge = {}

export type State = {
    -- Ticks left of the storm (AIAttackVars+0x10), and toward the next
    -- sound (+0x12).
    timer: number,
    sound: number,
}

-- The phases, on the attack's step.
local RAISE, BLOW = 0x0, 0x4

local ANIM = 0x0F
local SOUND = asset.sound("drip-shower")
-- The attack: 20 damage, hit parameter 0x8A; the storm's ticks (the
-- parameter word's second byte, 0x3C).
local DAMAGE, HIT_PARAM, TICKS = 0x14, 0x8A, 0x3C
-- The storm's Z: the register the counter window's setup leaves, the
-- navi's collision data's address, of which its init keeps the fraction.
local STORM_Z = 0

-- `sub_80F0228`: the storm on the middle panel of the other side's area,
-- tied to this action.
local function raise(me: Object, s: State)
    me:set_animation(ANIM)
    me:open_counter_window()
    me:set_status("using_action", true)
    s.timer = TICKS
    s.sound = 0x10
    local x = bit32.bxor(me.alliance, 1) * 3 + 2
    local damage = me.attack_damage + me.hit_param * 0x10000
    dust_storm.spawn(me, x, 2, STORM_Z, 0, damage, { ticks = TICKS, tied = charge.action })
    me.step = BLOW
    me.step_init = 0
end

-- `sub_80F0262`: the sound every 17 ticks; on the ticks whose bit 1 is
-- clear, the navi back on its panel's spot and its X jittered by up to a
-- pixel either way (an RNG2 draw); when the storm's ticks are out, back on
-- its spot and idle.
local function blow(me: Object, s: State)
    local n = s.sound + 1
    s.sound = n
    if n > 0x10 then
        s.sound = 0
        battle.play_sound(SOUND)
    end
    if bit32.band(s.timer, 2) == 0 then
        me:set_coordinates_from_panel()
        me.pos = Vec3.new(battle.jitter(3, me.pos).x, me.pos.y, me.pos.z)
    end
    local t = s.timer - 1
    s.timer = t
    if t < 0 then
        me:set_coordinates_from_panel()
        me:exit_attack()
    end
end

-- `sub_80F020E`.
local function update(me: Object, s: State)
    local step = me.step
    if step == RAISE then
        raise(me, s)
    elseif step == BLOW then
        blow(me, s)
    else
        error(string.format("the rain's phase %#x reads past its phase table", step))
    end
end

charge.action = define.action {
    id = "patch-cards/count/charge/action",
    state = { timer = "u16", sound = "u16" },
    update = update,
}

-- `sub_801244A`.
local function setup(navi: Object): Action
    navi.attack_damage = DAMAGE
    navi.hit_param = HIT_PARAM
    return charge.action
end

charge.weapon = define.weapon {
    id = "patch-cards/count/charge",
    name = "B.Rain",
    -- The charge table's row 0x6D, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 210, 200, 190, 185, 180, 160, 150, 140 },
    setup = setup,
}

return charge.weapon
