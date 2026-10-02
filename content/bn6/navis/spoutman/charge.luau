--!strict
-- SpoutMan's charged attack (`spoutman/charge`): the original's weapon
-- routine 0x41 (`sub_80121F8`) and his action table's entry 9
-- (`sub_80F153C`). He winds up 8 ticks, then throws a water ball from his
-- own panel (chips/spoutman/ball, the kind whose splash cracks its panel
-- and that stands still while dimmed), with a splash where he stands; 30
-- ticks later he idles.

local ball = require("../../chips/spoutman/ball")
local link = require("../../lib/link_chips")
local weapon = require("../../lib/weapon")

export type State = { timer: number }

local charge = {}

local WIND_UP, RECOVER = 0x0, 0x4
local ANIM_WIND_UP, ANIM_THROW = 0x13, 0x14
local WIND_UP_TICKS, RECOVER_TICKS = 8, 0x1E

local SOUND = asset.sound("bn6:bubble")
local SPLASH = define.effect { sprite = asset.sprite("bn6:spout-man-effect"), anim = 0, palette = 0 }

-- The hits' counter byte, and the ball's element: Aqua (the attack's
-- element byte).
local HIT_PARAM = 0x8A
local AQUA = 2

-- `sub_80121F8`: 20 damage and 20 more per point of buster Attack (up to
-- 5).
local function setup(navi: Object): Action
    navi.attack_damage = weapon.charge_damage(navi, 0x14, 0x14)
    navi.hit_param = HIT_PARAM
    navi.attack_element = AQUA
    return charge.action
end

-- `sub_80F1558`: he winds up; 8 ticks, then the ball and the splash.
local function wind_up(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = ANIM_WIND_UP
        s.timer = WIND_UP_TICKS
    elseif link.run_out(s) then
        me.anim = ANIM_THROW
        ball.spawn(me, me.panel_x, me.panel_y, me.attack_element, { cracks = true }, link.damage_word(me))
        battle.effect(Vec3.new(me.pos.x, me.pos.y, 0), SPLASH, link.flip(me))
        battle.play_sound(SOUND)
        link.go(me, RECOVER)
    end
end

-- `sub_80F15AA`: 30 ticks, then idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        s.timer = RECOVER_TICKS
    elseif link.run_out(s) then
        me.anim = 0
        me:exit_attack()
    end
end

local PHASES: { [number]: (me: Object, s: State) -> () } = { [WIND_UP] = wind_up, [RECOVER] = recover }

charge.action = define.action {
    id = "bn6:spoutman/charge/action",
    state = { timer = "u16" },
    -- `sub_80F153C` (`off_80F1550`).
    update = function(me: Object, s: State)
        link.run(me, s, PHASES, "SpoutMan's charge (sub_80F153C)")
    end,
}

charge.weapon = define.weapon {
    id = "bn6:spoutman/charge",
    -- The charge table's row 0x41, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 40, 35, 30, 25, 20, 60, 55, 50 },
    setup = setup,
}

return charge
